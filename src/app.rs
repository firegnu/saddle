use crate::{
    agents::Panel,
    config::{Config, expand_home},
    corral::{Client, Poller},
    git,
    input::{Focus, Route, encode_key, encode_mouse, encode_paste},
    layout::Panes,
    placement::{self, Placement},
    pty::Session,
    terminals::{Control, Place, Terminals, Ticket},
    ui::{self, Hits, View},
};
use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags, MouseButton,
        MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[path = "app_control.rs"]
mod control_impl;
#[path = "app_links.rs"]
mod links_impl;
#[path = "app_plugins.rs"]
mod plugins_impl;
use control_impl::{Closing, Record, Replacement};

#[derive(Clone)]
enum Action {
    Attach(String, Ticket, Option<u64>),
    PluginAgent(crate::plugins::runtime::Navigation),
    PluginAgentReady(crate::plugins::runtime::Navigation, Ticket),
    Reply(String),
    Stop(String),
    Start(Vec<String>, Ticket, Option<u64>),
}
struct ActionResult {
    action: Action,
    result: Result<serde_json::Value>,
}
struct Actions {
    client: Client,
    sender: Sender<ActionResult>,
    receiver: Receiver<ActionResult>,
    cancel: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
}
impl Actions {
    fn new(client: Client) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            client,
            sender,
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
        }
    }
    fn start(&mut self, action: Action) {
        self.workers.retain(|worker| !worker.is_finished());
        let client = self.client.clone();
        let sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.workers.push(thread::spawn(move || {
            let result = match &action {
                Action::Start(args, _, _) => client.json(
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                    Duration::from_secs(120),
                    &cancel,
                ),
                _ => {
                    let (verb, name, timeout) = match &action {
                        Action::Attach(name, _, _) => ("status", name, 15),
                        Action::PluginAgent(request) | Action::PluginAgentReady(request, _) => {
                            ("status", &request.name, 15)
                        }
                        Action::Reply(name) => ("reply", name, 15),
                        Action::Stop(name) => ("stop", name, 120),
                        Action::Start(..) => unreachable!(),
                    };
                    client.json(&[verb, name], Duration::from_secs(timeout), &cancel)
                }
            };
            let _ = sender.send(ActionResult { action, result });
        }));
    }
}
impl Drop for Actions {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let guard = Self;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
            EnableMouseCapture,
            EnableBracketedPaste
        )?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            DisableBracketedPaste,
            DisableMouseCapture,
            PopKeyboardEnhancementFlags,
            LeaveAlternateScreen,
            Show
        );
        let _ = disable_raw_mode();
    }
}

/// Runs saddle with `config`, loaded from `path`, which Settings edits.
pub fn run(
    mut config: Config,
    path: std::path::PathBuf,
    core_catalog: crate::plugins::core::Catalog,
) -> Result<()> {
    config.colors = config.colors.for_terminal(truecolor());
    let mut app = App::new(config, path, core_catalog)?;
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    let result = app.run(&mut terminal);
    drop(terminal);
    drop(_guard);
    if !app.layout_store.notice.is_empty() {
        eprintln!("{}", app.layout_store.notice);
    }
    result
}
fn truecolor() -> bool {
    crate::theme::truecolor(std::env::var("COLORTERM").ok().as_deref())
}
struct App {
    plugins: crate::plugins::Manager,
    plugin_page: Option<crate::plugins::ui::Page>,
    plugin_palette: Option<crate::plugins::palette::Palette>,
    plugin_entry_press: Option<Rect>,
    plugin_overlay: Option<plugins_impl::Overlay>,
    parked_settings: Option<crate::settings::Settings>,
    plugin_toast: Option<(Rect, Rect)>,
    plugin_toast_shown: Option<crate::plugins::runtime::Toast>,

    control: crate::control::Server,
    records: Vec<Record>,
    closing: Option<Closing>,
    quit: bool,
    config: Config,
    panel: Panel,
    focus: Focus,
    poller: Poller,
    git: git::Poller,
    actions: Actions,
    viewer: Terminals,
    mascot: crate::mascot::Mascot,
    layout_store: crate::layout_state::Store,
    cwd: String,
    projects: Vec<String>,
    reply: Option<(String, String)>,
    reply_busy: bool,
    reply_due: Instant,
    placement: Option<Placement>,
    search: Option<crate::search::Search>,
    board: crate::attention::Board,
    attention: Option<crate::attention::Popup>,
    native_mouse: bool,
    new_agent: Option<crate::launch::Form>,
    /// Agents New draft parked while a location-bound form is in use.
    agent_draft: Option<crate::launch::Form>,
    /// Later deliberate input supersedes an asynchronous automatic focus transfer.
    input_revision: u64,
    viewer_area: Rect,
    hits: Hits,
    pointer: crate::buttons::Pointer,
    link_attach: Option<links_impl::LinkAttach>,
    navigation: Option<crate::plugins::runtime::Navigation>,
    navigation_revision: u64,
    config_path: std::path::PathBuf,
    settings: Option<crate::settings::Settings>,
    /// The input target to return to when Settings closes.
    settings_return: Focus,
    /// For Diagnostics: the latest agent and task reads, where the config came from, and the
    /// check running for the open page.
    agents_read: crate::diagnostics::Last,
    config_from_file: bool,
    checker: Option<crate::diagnostics::Checker>,
}
impl App {
    fn new(
        config: Config,
        config_path: std::path::PathBuf,
        core_catalog: crate::plugins::core::Catalog,
    ) -> Result<Self> {
        // The config loaded without error, so an existing file is where it came from.
        let config_from_file = config_path.exists();
        let client = Client {
            program: expand_home(&config.corral).to_string_lossy().into_owned(),
        };
        let cwd = std::env::current_dir()?.display().to_string();
        let (layout_store, saved) =
            crate::layout_state::Store::open(crate::layout_state::default_path());
        let mut viewer = match saved {
            Some(layout) => Terminals::restore(client.program.clone(), layout)?,
            None => Terminals::new(client.program.clone()),
        };
        let mut actions = Actions::new(client.clone());
        let reconnect: Vec<_> = viewer
            .tabs
            .iter()
            .flat_map(|t| &t.panes)
            .filter_map(|p| match &p.viewer.remembered {
                crate::layout_state::Content::Agent {
                    name,
                    cwd,
                    instance: Some(instance),
                } => Some((p.id, name.clone(), cwd.clone(), instance.clone())),
                _ => None,
            })
            .collect();
        let mut connecting = std::collections::HashSet::new();
        for (id, name, cwd, instance) in reconnect {
            if !connecting.insert(name.clone()) {
                continue;
            }
            let ticket = viewer.reserve_at(id, Place::Current, Some(name.clone()));
            viewer.get_mut(id).unwrap().pending_agent = crate::viewer::AgentMetadata {
                cwd,
                instance: Some(instance),
            };
            actions.start(Action::Attach(name, ticket, None));
        }
        let plugins = crate::plugins::Manager::with_core(
            config_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("plugins.toml"),
            core_catalog,
        );
        Ok(Self {
            plugins,
            plugin_page: None,
            plugin_palette: None,
            plugin_entry_press: None,
            plugin_overlay: None,
            parked_settings: None,
            plugin_toast: None,
            plugin_toast_shown: None,
            layout_store,
            control: crate::control::Server::start()?,
            records: Vec::new(),
            closing: None,
            quit: false,
            poller: Poller::start(client.clone(), Duration::from_millis(config.refresh_ms)),
            git: git::Poller::start("git".into(), Duration::from_secs(5)),
            actions,
            viewer,
            mascot: crate::mascot::Mascot::new(truecolor()),
            config,
            panel: Panel {
                follow: true,
                ..Default::default()
            },
            focus: Focus::Agents,
            cwd: cwd.clone(),
            projects: vec![cwd],
            reply: None,
            reply_busy: false,
            reply_due: Instant::now(),
            placement: None,
            search: None,
            board: Default::default(),
            attention: None,
            native_mouse: false,
            new_agent: None,
            agent_draft: None,
            input_revision: 0,
            viewer_area: Rect::default(),
            hits: Hits::default(),
            pointer: Default::default(),
            link_attach: None,
            navigation: None,
            navigation_revision: 0,
            config_path,
            settings: None,
            settings_return: Focus::Agents,
            agents_read: None,
            config_from_file,
            checker: None,
        })
    }
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            let size = terminal.size()?;
            let panes = Panes::new(Rect::new(0, 0, size.width, size.height), &self.config);
            self.tick(panes)?;
            self.layout_store.save(&self.viewer, false);
            if self.quit {
                break;
            }
            let reply = self
                .reply
                .as_ref()
                .filter(|(name, _)| Some(name) == self.panel.selected.as_ref())
                .map(|(_, text)| text.as_str())
                .unwrap_or("loading…")
                .to_owned();
            let local: Vec<String> = self
                .viewer
                .tabs
                .iter()
                .flat_map(|tab| &tab.panes)
                .filter(|pane| pane.viewer.session.is_some())
                .filter_map(|pane| pane.viewer.showing.clone())
                .collect();
            let items = self.attention_items();
            let loading = self.board.loading();
            terminal.draw(|frame| {
                self.hits = ui::draw_workspace(
                    frame,
                    &mut self.panel,
                    View {
                        colors: &self.config.colors,
                        panes,
                        focus: self.focus,
                        showing: self.viewer.active_pane().viewer.showing.as_deref(),
                        local: &local,
                        viewer: self.viewer.active_pane().viewer.session.as_ref(),
                        projects: &self.projects,
                        viewer_note: &self.viewer.active_pane().viewer.note,
                        reply: &reply,
                        now: now(),
                        pointer: &self.pointer,
                    },
                    Some(ui::Workspace {
                        terminals: &self.viewer,
                        mascot: &mut self.mascot,
                        mascot_enabled: self.config.mascot_enabled,
                        placement: self.placement.as_ref(),
                        search: self.search.as_mut(),
                        form: self.new_agent.as_mut().filter(|f| f.visible),
                        program: &self.actions.client.program,
                        modal: self.closing.is_some() || self.plugin_page.is_some(),
                        attention: ui::Attention {
                            items: &items,
                            loading,
                            popup: self.attention.as_mut(),
                        },
                        // Keep the draft, but do not draw its input cursor through plugin management.
                        settings: self
                            .settings
                            .as_mut()
                            .filter(|_| self.plugin_page.is_none()),
                    }),
                );
                self.draw_plugin_overlay(frame, panes);
                self.draw_closing(frame);
                if let (Some(page), Some(settings)) = (&mut self.plugin_page, &self.settings) {
                    page.draw(&self.config.colors, frame, &self.plugins, settings);
                    frame.render_widget(ratatui::widgets::Clear, panes.status);
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(" Input ▸ Plugin settings · Esc Back")
                            .style(self.config.colors.base()),
                        panes.status,
                    );
                }
                self.plugin_toast = if self.plugin_page.is_none() {
                    self.draw_plugin_toast(frame, panes.viewer)
                } else {
                    None
                };
                if !self.layout_store.notice.is_empty() {
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(self.layout_store.notice.as_str())
                            .style(self.config.colors.base()),
                        panes.status,
                    );
                }
                if let Some(palette) = &mut self.plugin_palette {
                    palette.draw(frame, &self.config.colors);
                    frame.render_widget(ratatui::widgets::Clear, panes.status);
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(" Input ▸ Plugins · Esc Close")
                            .style(self.config.colors.base()),
                        panes.status,
                    );
                }
            })?;
            if event::poll(Duration::from_millis(30))? && self.event(event::read()?, panes)? {
                break;
            }
        }
        self.layout_store.save(&self.viewer, true);
        Ok(())
    }
    fn tick(&mut self, panes: Panes) -> Result<()> {
        self.viewer_area = panes.viewer;
        let focused = self.focus == Focus::Viewer
            && self.settings.is_none()
            && self.plugin_palette.is_none()
            && self.plugin_page.is_none()
            && self.closing.is_none()
            && self.search.is_none()
            && self.placement.is_none()
            && self.attention.is_none()
            && self.new_agent.as_ref().is_none_or(|f| !f.visible);
        self.plugins.theme(&self.config.colors);
        self.sync_plugins(panes, focused);
        self.update_plugin_palette();
        for update in self.poller.updates.try_iter() {
            match update {
                Ok(agents) => {
                    self.agents_read = Some((SystemTime::now(), Ok(())));
                    self.board.agents_loaded = true;
                    self.board.corral_error = None;
                    self.viewer.disappeared(
                        &agents
                            .iter()
                            .filter(|a| a.state.as_deref() != Some("exited"))
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>(),
                    )?;
                    self.panel.absorb(
                        agents,
                        self.viewer.active_pane().viewer.showing.as_deref(),
                        now(),
                    );
                    let mut cwds: Vec<_> = self
                        .panel
                        .agents
                        .iter()
                        .filter_map(|a| a.cwd.clone())
                        .collect();
                    cwds.sort();
                    cwds.dedup();
                    self.git.watch(cwds);
                }
                Err(error) => {
                    self.agents_read = Some((SystemTime::now(), Err(format!("{error:#}"))));
                    self.board.corral_error = Some(format!("{error:#}"));
                    self.panel.message = format!("corral: {error:#}");
                }
            }
        }
        if let Some(checks) = self
            .checker
            .as_ref()
            .and_then(|c| c.updates.try_recv().ok())
            && let Some(settings) = &mut self.settings
        {
            settings.checked(checks);
        }
        for batch in self.git.updates.try_iter() {
            self.panel.absorb_git(batch);
        }
        let results: Vec<_> = self.actions.receiver.try_iter().collect();
        for result in results {
            match result.action {
                Action::PluginAgent(request) => self.navigation_result(request, result.result),
                Action::PluginAgentReady(request, ticket) => {
                    self.navigation_ready(request, ticket, result.result)
                }
                Action::Attach(name, ticket, focus_intent) if self.viewer.valid(ticket) => {
                    let expected = self
                        .viewer
                        .get(ticket.pane)
                        .and_then(|p| p.pending_agent.instance.clone());
                    let checked = result.result.and_then(|status| {
                        anyhow::ensure!(
                            status["state"].as_str() != Some("exited"),
                            "{name} has exited"
                        );
                        anyhow::ensure!(
                            status["attached"].as_u64().unwrap_or(0) == 0,
                            "{name} is attached elsewhere; Ctrl-] there first"
                        );
                        if let Some(expected) = expected {
                            anyhow::ensure!(
                                status["instance"].as_str() == Some(&expected),
                                "agent identity changed before attach"
                            );
                        }
                        Ok(status)
                    });
                    match checked {
                        Ok(status) => {
                            self.viewer
                                .get_mut(ticket.pane)
                                .unwrap()
                                .pending_agent
                                .instance = status["instance"].as_str().map(str::to_owned);
                            self.viewer.complete(ticket, Some(name))?;
                            self.operation_update(ticket, "attaching", None, None);
                            self.panel.message.clear();
                            if focus_intent == Some(self.input_revision)
                                && self.focus == Focus::Agents
                                && self.panel.confirm.is_none()
                                && self.placement.is_none()
                                && self.closing.is_none()
                                && self.settings.is_none()
                                && !self.new_agent.as_ref().is_some_and(|f| f.visible)
                                && self.viewer.active_pane().id == ticket.pane
                            {
                                self.focus = Focus::Viewer;
                            }
                        }
                        Err(error) => {
                            self.viewer.complete(ticket, None)?;
                            self.panel.message = format!("{error:#}");
                            self.viewer
                                .get_mut(ticket.pane)
                                .unwrap()
                                .viewer
                                .note
                                .clone_from(&self.panel.message);
                            self.operation_update(
                                ticket,
                                "failed",
                                None,
                                Some(&format!("{error:#}")),
                            );
                        }
                    }
                }
                Action::Start(_, ticket, focus_intent) => {
                    let owns_form = self
                        .new_agent
                        .as_ref()
                        .is_some_and(|f| f.busy == Some(ticket));
                    let result = result.result.and_then(|value| {
                        value["name"]
                            .as_str()
                            .filter(|n| !n.is_empty())
                            .map(|name| {
                                (
                                    name.to_owned(),
                                    value["instance"].as_str().map(str::to_owned),
                                )
                            })
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "corral start returned no name; external result uncertain"
                                )
                            })
                    });
                    match result {
                        Ok((name, instance)) => {
                            if owns_form {
                                self.new_agent = self.agent_draft.take();
                            } else if self
                                .agent_draft
                                .as_ref()
                                .is_some_and(|f| f.busy == Some(ticket))
                            {
                                self.agent_draft = None;
                            }
                            self.panel.message = format!("Started {name}");
                            self.poller.refresh();
                            if self.viewer.valid(ticket) {
                                self.viewer
                                    .get_mut(ticket.pane)
                                    .unwrap()
                                    .pending_agent
                                    .instance = instance;
                                self.viewer.expect(ticket, name.clone());
                                self.operation_update(ticket, "attaching", Some(&name), None);
                                self.actions
                                    .start(Action::Attach(name, ticket, focus_intent));
                            } else {
                                self.operation_update(ticket, "target_invalid", Some(&name), None);
                                self.panel
                                    .message
                                    .push_str("; target closed or replaced. Open it from Agents.");
                            }
                        }
                        Err(error) => {
                            self.viewer.complete(ticket, None)?;
                            self.panel.message = format!("{error:#}");
                            let uncertain = self.panel.message.contains("timed out")
                                || self.panel.message.contains("timeout")
                                || self.panel.message.contains("uncertain")
                                || self.panel.message.contains("invalid JSON");
                            self.operation_update(
                                ticket,
                                if uncertain { "uncertain" } else { "failed" },
                                None,
                                Some(&format!("{error:#}")),
                            );
                            for form in [&mut self.new_agent, &mut self.agent_draft]
                                .into_iter()
                                .flatten()
                            {
                                if form.busy == Some(ticket) {
                                    form.busy = None;
                                    form.error = format!("{error:#}");
                                }
                            }
                        }
                    }
                }
                Action::Reply(name) => {
                    self.reply_busy = false;
                    self.reply = Some((
                        name,
                        match result.result {
                            Ok(value) => value["text"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .unwrap_or("(no reply yet)")
                                .into(),
                            Err(error) => format!("{error:#}"),
                        },
                    ));
                }
                Action::Stop(name) => {
                    self.panel.stopping = false;
                    self.panel.message = match result.result {
                        Ok(_) => format!("stopped {name}"),
                        Err(error) => format!("{error:#}"),
                    };
                    self.poller.refresh();
                }
                _ => {}
            }
        }
        self.viewer.tick(panes.viewer)?;
        self.control_tick();
        self.navigation_tick()?;
        if self.panel.show_reply
            && !self.reply_busy
            && let Some(name) = &self.panel.selected
        {
            let changed = self.reply.as_ref().is_none_or(|(old, _)| old != name);
            if changed || Instant::now() >= self.reply_due {
                self.actions.start(Action::Reply(name.clone()));
                self.reply_busy = true;
                self.reply_due = Instant::now() + Duration::from_millis(self.config.refresh_ms);
            }
        }
        Ok(())
    }
    fn attach(&mut self) {
        if let Some(name) = self.panel.selected.clone() {
            match self.viewer.activate_existing(&name) {
                Ok(true) => {
                    self.panel.message.clear();
                    self.focus = Focus::Viewer;
                    return;
                }
                Err(error) => {
                    self.panel.message = format!("{error:#}");
                    return;
                }
                Ok(false) => {}
            }
            let id = self.viewer.active_pane().id;
            if self.viewer.active_pane().viewer.shell_live() {
                if let Err(error) = self.request_replace(id, Replacement::Attach(name)) {
                    self.panel.message = error.to_string();
                }
                return;
            }
            self.viewer.get_mut(id).unwrap().viewer.shell = None;
            self.attach_at(id, name);
        }
    }
    fn attach_at(&mut self, id: u64, name: String) {
        let ticket = self
            .viewer
            .reserve_at(id, Place::Current, Some(name.clone()));
        if let Some(agent) = self.panel.agents.iter().find(|a| a.name == name) {
            let pane = self.viewer.get_mut(ticket.pane).unwrap();
            pane.pending_agent.cwd = agent.cwd.clone();
            pane.pending_agent.instance = agent.instance.clone();
        }
        self.actions.start(Action::Attach(
            name.clone(),
            ticket,
            Some(self.input_revision),
        ));
        self.panel.message = format!("attaching {name}…");
    }
    /// Opens the candidate the pick is bound to at the placement's location, only if row
    /// `index` still shows it; otherwise the pick is cancelled and the list stays open. The
    /// popup's input is modal, so its originating pane is still open here.
    fn pick(&mut self, index: usize) {
        let Some(placement) = &mut self.placement else {
            return;
        };
        let Some(place) = placement.place else {
            return;
        };
        let bound = placement.pressed.take();
        let shown = placement::candidates(&self.panel.agents, &self.viewer, placement)
            .into_iter()
            .nth(index)
            .map(|(name, _)| name);
        let Some(name) = bound.filter(|name| shown.as_ref() == Some(name)) else {
            return;
        };
        let anchor = placement.pane;
        self.placement = None;
        let content = match name {
            placement::Choice::Terminal => crate::control::Content::Shell { cwd: None },
            placement::Choice::Plugin => {
                self.plugin_palette = Some(crate::plugins::palette::Palette::for_placement(
                    anchor, place,
                ));
                self.update_plugin_palette();
                self.native_mouse = false;
                return;
            }
            placement::Choice::NewAgent => {
                let project = self
                    .viewer
                    .get(anchor)
                    .and_then(|p| p.source_cwd().map(str::to_owned))
                    .unwrap_or_else(|| self.cwd.clone());
                let mut form = crate::launch::Form::new(project);
                form.anchor = self.viewer.get(anchor).map(|p| p.ticket());
                form.place = Place::ALL.iter().position(|p| *p == place).unwrap();
                if self.new_agent.as_ref().is_some_and(|f| f.anchor.is_none()) {
                    self.agent_draft = self.new_agent.take();
                }
                self.new_agent = Some(form);
                return;
            }
            placement::Choice::Agent(name) => crate::control::Content::Agent { name },
        };
        match self.open_content(anchor, place, content, true) {
            Ok((ticket, _, _)) => {
                self.panel.message = self
                    .viewer
                    .get(ticket.pane)
                    .and_then(|p| p.requested())
                    .map(|name| format!("attaching {name}…"))
                    .unwrap_or_default();
            }
            Err(error) => self.panel.message = format!("{error:#}"),
        }
    }

    fn placement_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(placement) = &mut self.placement else {
            return Ok(());
        };
        if key
            .modifiers
            .contains(crossterm::event::KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char(']' | '5'))
        {
            self.placement = None;
            self.focus = Focus::Agents;
            return Ok(());
        }
        let candidates = placement::candidates(&self.panel.agents, &self.viewer, placement);
        let last = candidates.len().saturating_sub(1);
        let control = match (placement.place, key.code) {
            (_, KeyCode::Esc) => Some(Control::Cancel),
            (None, code) => placement::side(code).map(Control::Side),
            (Some(_), KeyCode::Up | KeyCode::Char('k')) => {
                placement.selected = placement.selected.min(last).saturating_sub(1);
                None
            }
            (Some(_), KeyCode::Down | KeyCode::Char('j')) => {
                placement.selected = (placement.selected + 1).min(last);
                None
            }
            (Some(_), KeyCode::Enter) => {
                let index = placement.selected.min(last);
                placement.pressed = candidates.get(index).map(|(name, _)| name.clone());
                Some(Control::Pick(index))
            }
            _ => None,
        };
        if let Some(control) = control {
            self.terminal_control(control)?;
        }
        Ok(())
    }
    fn terminal_control(&mut self, control: Control) -> Result<()> {
        match control {
            Control::Plugins => {
                self.open_settings(self.focus);
                self.plugin_page = Some(Default::default());
            }
            Control::RestartPlugin(pane) => {
                if let Some(id) = self
                    .viewer
                    .get(pane)
                    .and_then(|p| p.plugin_id())
                    .map(str::to_owned)
                    && let Err(e) = self.plugins.restart(&id)
                {
                    self.panel.message = e.to_string();
                }
            }
            // Choosing a place first; the layout changes only when an agent is picked.
            Control::NewTab => {
                self.placement = Some(Placement {
                    pane: self.viewer.active_pane().id,
                    place: Some(Place::Tab),
                    selected: 0,
                    pressed: None,
                });
            }
            Control::Split(id) => {
                self.viewer.focus(id);
                self.placement = Some(Placement {
                    pane: id,
                    place: None,
                    selected: 0,
                    pressed: None,
                });
            }
            Control::CreateAgent(id) => {
                self.viewer.focus(id);
                let pane = self.viewer.get(id).unwrap();
                if !pane.placeholder() {
                    return Ok(());
                }
                if let Some(name) = pane.viewer.remembered.name() {
                    let mut form = crate::launch::Form::for_previous(
                        name,
                        pane.viewer.remembered.cwd().unwrap_or(&self.cwd).into(),
                    );
                    form.anchor = Some(pane.ticket());
                    if self.new_agent.as_ref().is_some_and(|f| f.anchor.is_none()) {
                        self.agent_draft = self.new_agent.take();
                    }
                    self.new_agent = Some(form);
                }
            }
            Control::ChooseAgent(id) => {
                if self.viewer.get(id).is_some_and(|p| p.placeholder()) {
                    self.viewer.focus(id);
                    self.placement = Some(Placement {
                        pane: id,
                        place: Some(Place::Current),
                        selected: 0,
                        pressed: None,
                    });
                }
            }
            Control::OpenTerminal(id) => {
                if self.viewer.get(id).is_some_and(|p| p.placeholder())
                    && let Err(error) = self.open_content(
                        id,
                        Place::Current,
                        crate::control::Content::Shell { cwd: None },
                        true,
                    )
                {
                    self.panel.message = format!("{error:#}");
                }
            }
            Control::Side(place) => {
                if let Some(placement) = &mut self.placement {
                    placement.place = Some(place);
                }
            }
            Control::Pick(index) => self.pick(index),
            Control::Cancel => self.placement = None,
            Control::Tab(id) => self.viewer.active = id,
            Control::Pane(id) => self.viewer.focus(id),
            Control::Zoom(id) => self.viewer.toggle_zoom(id),
            Control::History(id) | Control::Live(id) => {
                self.viewer.focus(id);
                if let Some(session) = self.viewer.get(id).and_then(|p| p.viewer.session.as_ref()) {
                    let mut screen = session.screen.lock().unwrap();
                    if matches!(control, Control::History(_)) {
                        screen.enter_history();
                    } else {
                        screen.exit_history();
                    }
                }
            }
            Control::Copy(id) => {
                self.viewer.focus(id);
                if let Some(session) = self.viewer.get(id).and_then(|p| p.viewer.session.as_ref()) {
                    // The terminal keeps receiving output while the clipboard is written.
                    let text = session.screen.lock().unwrap().selected_text();
                    let status = match text {
                        None => "select text first".into(),
                        Some(text) => match copy_to_clipboard(&text) {
                            Ok(()) => format!("copied {} lines", text.lines().count()),
                            Err(error) => format!("copy failed: {error:#}"),
                        },
                    };
                    session.screen.lock().unwrap().history_status(status);
                }
            }
            Control::CloseTab(id) => self.request_close(crate::control::CloseTarget::Tab(id))?,
            Control::ClosePane(id) => self.request_close(crate::control::CloseTarget::Pane(id))?,
            Control::Previous | Control::Next => {
                let index = self
                    .viewer
                    .tabs
                    .iter()
                    .position(|t| t.id == self.viewer.active)
                    .unwrap();
                let count = self.viewer.tabs.len();
                let next = if matches!(control, Control::Previous) {
                    (index + count - 1) % count
                } else {
                    (index + 1) % count
                };
                self.viewer.active = self.viewer.tabs[next].id;
            }
        }
        self.focus = Focus::Viewer;
        Ok(())
    }
    fn event(&mut self, event: Event, panes: Panes) -> Result<bool> {
        if self.native_mouse
            && let Event::Mouse(mouse) = &event
        {
            if matches!(mouse.kind, MouseEventKind::Up(_)) {
                self.native_mouse = false;
            }
            return Ok(false);
        }
        if matches!(&event, Event::Key(key) if key.kind != KeyEventKind::Release)
            || matches!(&event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down(_) | MouseEventKind::ScrollUp | MouseEventKind::ScrollDown))
        {
            self.input_revision += 1;
        }
        if let Some(page) = &mut self.plugin_page {
            let outcome = page.event(event, &mut self.plugins);
            match outcome {
                crate::plugins::ui::Outcome::Stay => {}
                crate::plugins::ui::Outcome::Back => self.plugin_page = None,
                crate::plugins::ui::Outcome::Page(key) => {
                    if key.code != KeyCode::F(5) {
                        self.plugin_page = None;
                        if let Some(settings) = &mut self.settings {
                            let outcome = settings.key(key);
                            self.settings_outcome(outcome);
                        }
                    }
                }
                crate::plugins::ui::Outcome::Open(id) => {
                    self.plugin_page = None;
                    self.parked_settings = self.settings.take();
                    self.focus = self.settings_return;
                    self.open_plugin_view(&id);
                }
            }
            return Ok(false);
        }
        self.update_plugin_palette();
        if let Some(palette) = &mut self.plugin_palette {
            let outcome = palette.event(&event);
            self.plugin_palette_outcome(outcome);
            return Ok(false);
        }
        if self.plugin_launcher_event(&event) {
            return Ok(false);
        }
        if self.plugin_overlay.is_some() {
            self.plugin_overlay_event(&event);
            return Ok(false);
        }
        match event {
            Event::Key(key) => {
                self.pointer.cancel();
                if key.kind == KeyEventKind::Release {
                    if self.focus == Focus::Viewer
                        && self.settings.is_none()
                        && self.closing.is_none()
                        && self.search.is_none()
                        && self.placement.is_none()
                        && self.attention.is_none()
                        && self.new_agent.as_ref().is_none_or(|f| !f.visible)
                    {
                        self.plugin_input(crate::plugins::key(key));
                    }
                    return Ok(false);
                }
                if self.closing.is_some() {
                    match key.code {
                        KeyCode::Char('y' | 'Y') => self.confirm_close()?,
                        KeyCode::Up | KeyCode::PageUp => {
                            let c = self.closing.as_mut().unwrap();
                            c.scroll = c.scroll.saturating_sub(3);
                        }
                        KeyCode::Down | KeyCode::PageDown => {
                            let c = self.closing.as_mut().unwrap();
                            c.scroll = c
                                .scroll
                                .saturating_add(3)
                                .min(c.snapshot.as_array().unwrap().len() as u16 * 3);
                        }
                        _ => {
                            self.closing = None;
                        }
                    }
                    return Ok(self.quit);
                }
                if let Some(settings) = &mut self.settings {
                    let outcome = settings.key(key);
                    self.settings_outcome(outcome);
                    return Ok(false);
                }
                if let Some(form) = self.new_agent.as_mut().filter(|f| f.visible) {
                    let submit = form.key(key, &self.projects);
                    let bound = form.anchor;
                    if !form.visible && bound.is_some() {
                        if form.busy.is_none() {
                            self.new_agent = self.agent_draft.take();
                        }
                        self.focus = if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                        {
                            Focus::Agents
                        } else {
                            Focus::Viewer
                        };
                        return Ok(false);
                    }
                    if submit {
                        let args = form.args();
                        let place = Place::ALL[form.place];
                        let anchor = bound.map_or(self.viewer.active_pane().id, |t| t.pane);
                        let result = args.and_then(|args| {
                            anyhow::ensure!(
                                bound.is_none_or(|t| self.viewer.valid(t)),
                                "originating pane changed"
                            );
                            if place == Place::Current
                                && self
                                    .viewer
                                    .get(anchor)
                                    .is_some_and(|p| p.viewer.shell_live())
                            {
                                self.request_replace(anchor, Replacement::Start(args))?;
                                return Ok(None);
                            }
                            if place == Place::Current {
                                self.viewer.get_mut(anchor).unwrap().viewer.shell = None;
                            }
                            self.begin_start(anchor, place, args, Some(self.input_revision))
                                .map(Some)
                        });
                        let form = self.new_agent.as_mut().unwrap();
                        match result {
                            Ok(ticket) => {
                                form.busy = ticket;
                                form.error.clear();
                            }
                            Err(error) => {
                                form.error = format!("{error:#}");
                                form.reveal_invalid();
                            }
                        }
                    }
                    return Ok(false);
                }
                if self.placement.is_some() {
                    self.placement_key(key)?;
                    return Ok(false);
                }
                if let Some(search) = &mut self.search {
                    let outcome = search.key(key, &self.panel.agents);
                    self.search_outcome(outcome);
                    return Ok(false);
                }
                if let Some(popup) = &mut self.attention {
                    let mut items = self.board.items(&self.panel, now());
                    items.extend(self.plugins.attention_items());
                    let outcome = popup.key(key, &items);
                    self.attention_outcome(outcome);
                    return Ok(false);
                }
                if self.focus == Focus::Agents
                    && let Some(name) = self.panel.confirm.take()
                {
                    if matches!(key.code, KeyCode::Char('y' | 'Y')) {
                        self.panel.stopping = true;
                        self.actions.start(Action::Stop(name.clone()));
                        self.panel.message = format!("stopping {name}…");
                    } else {
                        self.panel.message = "cancelled".into();
                    }
                    return Ok(false);
                }
                let route = self.focus.route(key);
                match route {
                    Route::Quit => {
                        self.request_close(crate::control::CloseTarget::All)?;
                        return Ok(self.quit);
                    }
                    Route::Panel => self.panel_key(key),
                    Route::Terminal => {
                        if self.plugin_input(crate::plugins::key(key)) {
                            return Ok(false);
                        }
                        if let Some(screen) = self.history_screen() {
                            screen.lock().unwrap().history_key(key);
                        } else if let Some(session) = self.focused_session() {
                            let mode = *session.screen.lock().unwrap().term.mode();
                            if let Err(error) = session.send(encode_key(
                                key,
                                mode.contains(alacritty_terminal::term::TermMode::APP_CURSOR),
                            )) {
                                self.panel.message = error.to_string();
                            }
                        }
                    }
                    Route::Ignore => {}
                }
            }
            Event::Paste(text) => {
                if self.closing.is_some() {
                    return Ok(false);
                }
                if let Some(settings) = &mut self.settings {
                    settings.paste(&text);
                    return Ok(false);
                }
                if let Some(form) = self.new_agent.as_mut().filter(|f| f.visible) {
                    form.paste(&text);
                    return Ok(false);
                }
                if self.placement.is_some() {
                    return Ok(false);
                }
                if let Some(search) = &mut self.search {
                    search.paste(&text);
                    return Ok(false);
                }
                if self.attention.is_some() {
                    return Ok(false);
                }
                if self.viewer.active_pane().plugin_id().is_some() && self.focus == Focus::Viewer {
                    if text.len() > saddle_plugin_protocol::MAX_PASTE {
                        self.panel.message = "Paste too large for plugin".into();
                    } else {
                        self.plugin_input(serde_json::json!({"type":"paste","text":text}));
                    }
                } else if let Some(screen) = self.history_screen() {
                    screen.lock().unwrap().history_paste(&text);
                } else if let Some(session) = self.focused_session() {
                    let bracketed = session
                        .screen
                        .lock()
                        .unwrap()
                        .term
                        .mode()
                        .contains(alacritty_terminal::term::TermMode::BRACKETED_PASTE);
                    if let Err(error) = session.send(encode_paste(&text, bracketed)) {
                        self.panel.message = error.to_string();
                    }
                }
            }
            Event::Mouse(mouse) => {
                let point = (mouse.column, mouse.row).into();
                if self.plugin_toast_event(&mouse) {
                    if matches!(mouse.kind, MouseEventKind::Down(_)) {
                        self.native_mouse = true;
                    }
                    return Ok(false);
                }
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                    && let Some(placement) = &mut self.placement
                {
                    // Bind the press to the name its row shows in the list just drawn; the
                    // release opens only that name.
                    placement.pressed = match self
                        .hits
                        .terminal
                        .iter()
                        .find(|(hit, _)| hit.area.contains(point))
                    {
                        Some((_, Control::Pick(index))) => {
                            placement::candidates(&self.panel.agents, &self.viewer, placement)
                                .into_iter()
                                .nth(*index)
                                .map(|(name, _)| name)
                        }
                        _ => None,
                    };
                }
                let controls: Vec<_> = self
                    .hits
                    .buttons
                    .iter()
                    .cloned()
                    .map(|h| (Focus::Agents, h))
                    .chain(
                        self.hits
                            .terminal
                            .iter()
                            .map(|(hit, _)| (Focus::Viewer, hit.clone())),
                    )
                    .collect();
                let captured = self.pointer.captured();
                if let Some((focus, key)) = self.pointer.event(mouse, &controls) {
                    if focus == Focus::Agents
                        && key.code == KeyCode::Char(',')
                        && self.settings.is_none()
                        && self.closing.is_none()
                    {
                        // The Settings entry: open, remembering where input was.
                        self.open_settings(self.focus);
                        return Ok(false);
                    }
                    if focus == Focus::Viewer {
                        if let Some((_, action)) = self
                            .hits
                            .terminal
                            .iter()
                            .find(|(hit, _)| hit.area.contains(point))
                        {
                            self.terminal_control(*action)?;
                        }
                        return Ok(false);
                    }
                    if self.closing.is_none() {
                        self.focus = focus;
                    }
                    return self.event(Event::Key(key), panes);
                }
                if captured || self.pointer.captured() {
                    return Ok(false);
                }
                // The search popup takes all mouse input: a row opens the agent drawn on it.
                if let Some(search) = &mut self.search {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            if let Some(name) = search.click(point) {
                                // The popup closes on the press; the rest of the gesture
                                // must not reach the terminal that opening focuses.
                                self.native_mouse = true;
                                self.search_outcome(crate::search::Outcome::Open(name));
                            }
                        }
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => search
                            .scroll(mouse.kind == MouseEventKind::ScrollDown, &self.panel.agents),
                        _ => {}
                    }
                    return Ok(false);
                }
                // Settings takes all mouse input the same way.
                if let Some(settings) = &mut self.settings {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => settings.click(point),
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                            settings.scroll(mouse.kind == MouseEventKind::ScrollDown)
                        }
                        _ => {}
                    }
                    return Ok(false);
                }
                // Attention takes all mouse input the same way.
                if let Some(popup) = &mut self.attention {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            if let Some(target) = popup.click(point) {
                                // Only an agent focuses a terminal the gesture could reach;
                                // A plugin overlay consumes the remaining gesture.
                                self.native_mouse =
                                    matches!(target, crate::attention::Target::Agent(_));
                                self.attention_outcome(crate::attention::Outcome::Open(target));
                            }
                        }
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                            let mut items = self.board.items(&self.panel, now());
                            items.extend(self.plugins.attention_items());
                            popup.scroll(mouse.kind == MouseEventKind::ScrollDown, &items);
                        }
                        _ => {}
                    }
                    return Ok(false);
                }
                if let Some(form) = self.new_agent.as_mut().filter(|f| f.visible) {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        form.click(point, &self.projects);
                    } else if mouse.kind == MouseEventKind::ScrollDown {
                        form.scroll(true);
                    } else if mouse.kind == MouseEventKind::ScrollUp {
                        form.scroll(false);
                    }
                    return Ok(false);
                }
                if self.closing.is_some()
                    || self.panel.confirm.is_some()
                    || self.placement.is_some()
                {
                    if self.closing.is_some()
                        && matches!(
                            mouse.kind,
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                        )
                    {
                        return self.event(
                            Event::Key(KeyEvent::new(
                                if mouse.kind == MouseEventKind::ScrollUp {
                                    KeyCode::Up
                                } else {
                                    KeyCode::Down
                                },
                                crossterm::event::KeyModifiers::NONE,
                            )),
                            panes,
                        );
                    }
                    // The wheel moves through a long candidate list; sides take no wheel.
                    if self.placement.as_ref().is_some_and(|p| p.place.is_some())
                        && matches!(
                            mouse.kind,
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                        )
                    {
                        self.placement_key(KeyEvent::new(
                            if mouse.kind == MouseEventKind::ScrollUp {
                                KeyCode::Up
                            } else {
                                KeyCode::Down
                            },
                            crossterm::event::KeyModifiers::NONE,
                        ))?;
                    }
                    return Ok(false);
                }
                if matches!(mouse.kind, MouseEventKind::Down(_))
                    && self.hits.mascot.iter().any(|area| area.contains(point))
                {
                    self.native_mouse = true;
                    return Ok(false);
                }
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    self.panel.confirm = None;
                    if panes.agents.contains(point) {
                        self.focus = Focus::Agents;
                        if let Some((_, name)) =
                            self.hits.agents.iter().find(|(row, _)| *row == mouse.row)
                        {
                            self.panel.select(Some(name.clone()));
                            self.attach();
                            return Ok(false);
                        }
                    } else if panes.viewer.contains(point) {
                        self.focus = Focus::Viewer;
                        if let Some((id, rect)) = self
                            .viewer
                            .rects(panes.viewer)
                            .into_iter()
                            .find(|(_, r)| r.contains(point))
                        {
                            self.viewer.focus(id);
                            if !ui::inner(rect).contains(point) {
                                self.native_mouse = true;
                                return Ok(false);
                            }
                        } else {
                            self.native_mouse = true;
                            return Ok(false);
                        }
                    }
                }
                if panes.agents.contains(point)
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    )
                {
                    let delta = if mouse.kind == MouseEventKind::ScrollUp {
                        -1
                    } else {
                        1
                    };
                    if (Rect {
                        width: self.hits.reply.width.saturating_add(1),
                        ..self.hits.reply
                    })
                    .contains(point)
                    {
                        self.panel.reply_top = self.panel.reply_top.saturating_add_signed(delta);
                    } else if (Rect {
                        width: self.hits.list.width.saturating_add(1),
                        ..self.hits.list
                    })
                    .contains(point)
                    {
                        self.panel.top = self.panel.top.saturating_add_signed(delta);
                        self.panel.follow = false;
                    }
                } else if self.viewer.active_pane().plugin_id().is_some()
                    && self.focus == Focus::Viewer
                {
                    let area = self.active_inner(panes.viewer);
                    if area.contains(point) {
                        self.plugin_input(crate::plugins::mouse(mouse, area));
                    }
                } else if let Some(screen) = self.history_screen() {
                    // History takes the pane's mouse: the wheel scrolls, a drag selects.
                    let area = self.active_inner(panes.viewer);
                    let cell = (
                        mouse.column.saturating_sub(area.x),
                        mouse.row.saturating_sub(area.y),
                    );
                    let mut screen = screen.lock().unwrap();
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            screen.history_select(cell, true)
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            screen.history_select(cell, false)
                        }
                        MouseEventKind::ScrollUp => screen.history_scroll(3),
                        MouseEventKind::ScrollDown => screen.history_scroll(-3),
                        _ => {}
                    }
                } else if let Some(session) = self.focused_session() {
                    let area = self
                        .viewer
                        .rects(panes.viewer)
                        .into_iter()
                        .find(|(id, _)| *id == self.viewer.active_pane().id)
                        .map(|(_, r)| ui::inner(r))
                        .unwrap_or_default();
                    let mode = *session.screen.lock().unwrap().term.mode();
                    if let Err(error) = session.send(encode_mouse(mouse, area, mode)) {
                        self.panel.message = error.to_string();
                    }
                }
            }
            Event::Resize(_, _) => self.pointer.cancel(),
            _ => {}
        }
        Ok(false)
    }
    /// Closing the search keeps Agents as the input target; opening uses the normal attach,
    /// which jumps to a pane already showing the agent.
    fn search_outcome(&mut self, outcome: crate::search::Outcome) {
        match outcome {
            crate::search::Outcome::Stay => {}
            crate::search::Outcome::Cancel => {
                self.search = None;
                self.focus = Focus::Agents;
            }
            crate::search::Outcome::Open(name) => {
                self.search = None;
                self.focus = Focus::Agents;
                self.panel.select(Some(name));
                self.attach();
            }
        }
    }
    fn open_settings(&mut self, back: Focus) {
        if let Some(mut parked) = self.parked_settings.take() {
            parked.refresh_recording();
            self.settings = Some(parked);
            self.settings_return = back;
            return;
        }
        self.settings = Some(
            crate::settings::Settings::open(self.config_path.clone(), truecolor())
                .with_telemetry(crate::telemetry::Store::from_environment()),
        );
        self.settings_return = back;
        self.focus = Focus::Agents;
    }
    /// Closing returns input to where it was. A save applies colors and the sidebar width now;
    /// the other settings wait for the next start.
    fn settings_outcome(&mut self, outcome: crate::settings::Outcome) {
        use crate::settings::Outcome;
        match outcome {
            Outcome::Plugins => {
                self.plugin_page = Some(Default::default());
                return;
            }
            Outcome::Stay => return,
            Outcome::Cancel => {}
            // The config part was written; Settings stays open on what was not saved.
            Outcome::Applied(saved, _) => {
                self.apply_settings(&saved);
                return;
            }
            Outcome::Recorded => {
                let note = self.settings.as_ref().map_or("", |s| s.message());
                self.panel.message = format!("Settings saved. {note}");
            }
            Outcome::Diagnose => {
                let report = self.diagnostics();
                // Replacing the checker cancels the previous check and drops its answer.
                self.checker = Some(crate::diagnostics::Checker::start(
                    report.corral.clone(),
                    report.config_path.clone(),
                ));
                if let Some(settings) = &mut self.settings {
                    settings.diagnose(report);
                }
                return;
            }
            Outcome::Copy(summary) => {
                let result = copy_to_clipboard(&summary).map_err(|error| format!("{error:#}"));
                if let Some(settings) = &mut self.settings {
                    settings.copied(result);
                }
                return;
            }
            Outcome::Saved(saved, restart) => {
                self.apply_settings(&saved);
                // Telemetry recording, when it was saved too.
                let note = self.settings.as_ref().map_or("", |s| s.message());
                self.panel.message = if restart.is_empty() {
                    format!("Settings saved. {note}")
                } else {
                    format!(
                        "Settings saved; restart saddle to apply: {}. {note}",
                        restart.join(", ")
                    )
                }
                .trim_end()
                .into();
            }
        }
        self.settings = None;
        self.checker = None;
        self.focus = self.settings_return;
    }
    fn apply_settings(&mut self, saved: &crate::config::Config) {
        self.config.colors = saved.colors.clone().for_terminal(truecolor());
        self.config.left_width = saved.left_width;
        self.config.mascot_enabled = saved.mascot_enabled;
    }
    /// What saddle knows now; the command and config checks run in the background.
    fn diagnostics(&self) -> crate::diagnostics::Report {
        crate::diagnostics::Report {
            corral: self.actions.client.program.clone(),
            agents: self.agents_read.clone(),
            config_path: self.config_path.clone(),
            config_from_file: self.config_from_file,
            layout_path: self.layout_store.path().map(Into::into),
            restore: self.layout_store.restore.clone(),
            save: self.layout_store.saved.clone(),
            save_off: self.layout_store.protected(),
            checked: SystemTime::now(),
            checks: None,
        }
    }
    fn attention_items(&self) -> Vec<crate::attention::Item> {
        let mut items = self.board.items(&self.panel, now());
        items.extend(self.plugins.attention_items());
        items
    }
    /// Only navigate to a target; never answer, accept or advance its business state.
    fn attention_outcome(&mut self, outcome: crate::attention::Outcome) {
        use crate::attention::{Outcome, Target};
        match outcome {
            Outcome::Stay => {}
            Outcome::Cancel => {
                self.attention = None;
                self.focus = Focus::Agents;
            }
            Outcome::Open(target) => {
                self.attention = None;
                self.focus = Focus::Agents;
                match target {
                    target @ Target::Plugin { .. } => self.open_plugin_attention(&target),
                    Target::Agent(name) => {
                        self.panel.select(Some(name));
                        self.attach();
                    }
                    Target::Source(_) => {}
                }
            }
        }
    }
    /// The public cwd of the agent input was on: the one selected in Agents, or the one in the
    /// active pane in Viewer, never the sidebar's selection there.
    fn focused_agent_cwd(&self, focus: Focus) -> Option<String> {
        let (name, cwd) = match focus {
            Focus::Agents => (self.panel.selected.clone()?, None),
            Focus::Viewer => {
                let viewer = &self.viewer.active_pane().viewer;
                if viewer.shell.is_some() {
                    return None;
                }
                (
                    viewer.target()?.to_owned(),
                    viewer.target_metadata().cwd.clone(),
                )
            }
        };
        cwd.or_else(|| {
            self.panel
                .agents
                .iter()
                .find(|a| a.name == name)?
                .cwd
                .clone()
        })
    }
    fn panel_key(&mut self, key: KeyEvent) {
        self.panel.message.clear();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.panel.move_selection(-1, now()),
            KeyCode::Down | KeyCode::Char('j') => self.panel.move_selection(1, now()),
            KeyCode::Enter => self.attach(),
            KeyCode::Char('/') => self.search = Some(Default::default()),
            KeyCode::Char(',') => self.open_settings(Focus::Agents),
            KeyCode::Char('a') => {
                self.attention = Some(Default::default());
            }
            KeyCode::Char('n') => {
                self.reload_projects();
                if self.new_agent.as_ref().is_some_and(|f| f.anchor.is_some()) {
                    self.new_agent = self.agent_draft.take();
                }
                self.new_agent
                    .get_or_insert_with(|| crate::launch::Form::new(self.cwd.clone()))
                    .visible = true;
            }
            KeyCode::Char('s') => {
                self.panel.by_name = !self.panel.by_name;
                self.panel.follow = true;
            }
            KeyCode::Char('z') => self.panel.toggle_fold(),
            KeyCode::PageUp | KeyCode::PageDown => {
                let down = key.code == KeyCode::PageDown;
                let (offset, height) = if self.panel.show_reply {
                    (&mut self.panel.reply_top, self.hits.reply.height)
                } else {
                    self.panel.follow = false;
                    (&mut self.panel.top, self.hits.list.height)
                };
                *offset = if down {
                    offset.saturating_add(usize::from(height.max(1)))
                } else {
                    offset.saturating_sub(usize::from(height.max(1)))
                };
            }
            KeyCode::Char('x') if !self.panel.stopping => {
                if let Some(name) = &self.panel.selected {
                    self.panel.confirm = Some(name.clone());
                    self.panel.message = format!("stop {name}? y confirms; any other key cancels");
                }
            }
            _ => {}
        }
    }
    fn reload_projects(&mut self) {
        self.projects = std::iter::once(self.cwd.clone())
            .chain(self.panel.agents.iter().filter_map(|a| a.cwd.clone()))
            .collect();
        self.projects.sort();
        self.projects.dedup();
    }
    fn active_inner(&self, viewer: Rect) -> Rect {
        self.viewer
            .rects(viewer)
            .into_iter()
            .find(|(id, _)| *id == self.viewer.active_pane().id)
            .map(|(_, r)| ui::inner(r))
            .unwrap_or_default()
    }
    /// The focused pane's screen while it shows history; its keys and mouse stay in saddle.
    fn history_screen(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::terminal::Screen>>> {
        if self.focus != Focus::Viewer {
            return None;
        }
        let screen = &self.viewer.active_pane().viewer.session.as_ref()?.screen;
        screen.lock().unwrap().in_history().then(|| screen.clone())
    }
    fn focused_session(&self) -> Option<&Session> {
        match self.focus {
            Focus::Agents => None,
            Focus::Viewer => {
                let id = self.viewer.active_pane().id;
                if !self
                    .viewer
                    .rects(self.viewer_area)
                    .iter()
                    .any(|(pane, rect)| *pane == id && !ui::inner(*rect).is_empty())
                {
                    return None;
                }
                self.viewer.active_pane().input_session()
            }
        }
    }
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn copy_to_clipboard(text: &str) -> Result<()> {
    arboard::Clipboard::new()?.set_text(text)?;
    Ok(())
}
