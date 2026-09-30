use crate::{
    agents::Panel,
    config::{Config, expand_home},
    corral::{Client, Poller},
    drover, git,
    input::{Focus, Route, encode_key, encode_mouse, encode_paste},
    layout::Panes,
    placement::{self, Placement},
    pty::Session,
    queue,
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
use control_impl::{Closing, Record, Replacement};

#[derive(Clone)]
enum Action {
    Attach(String, Ticket, Option<u64>),
    TaskAgent(crate::links::AgentRequest),
    TaskAgentReady(crate::links::AgentRequest, Ticket),
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
                        Action::TaskAgent(request) | Action::TaskAgentReady(request, _) => {
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
pub fn run(mut config: Config, path: std::path::PathBuf) -> Result<()> {
    config.colors = config.colors.for_terminal(truecolor());
    let mut app = App::new(config, path)?;
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
    layout_store: crate::layout_state::Store,
    queue: queue::Panel,
    queue_worker: drover::Worker,
    pending_load: Option<drover::PendingLoad>,
    /// The focused agent's repository lookup for the Tasks opening that started it, with the
    /// project and input revision at that opening.
    repo_tasks: Option<(String, u64, drover::RepoTasks)>,
    detail: Option<(queue::DetailKey, drover::DetailWorker)>,
    /// The one reading of the run a manual completion page confirms.
    manual_target: Option<(queue::DetailKey, drover::DetailWorker)>,
    reply: Option<(String, String)>,
    reply_busy: bool,
    reply_due: Instant,
    placement: Option<Placement>,
    search: Option<crate::search::Search>,
    survey: drover::Surveyor,
    board: crate::attention::Board,
    attention: Option<crate::attention::Popup>,
    /// Drover's notification preference, read periodically and set from Settings.
    channel: drover::ChannelWorker,
    notifier: crate::notify::Notifier,
    /// The task prompt as last drawn: its area and close mark.
    toast: Option<(Rect, Rect)>,
    native_mouse: bool,
    new_agent: Option<crate::launch::Form>,
    /// Agents New draft parked while a location-bound form is in use.
    agent_draft: Option<crate::launch::Form>,
    /// Later deliberate input supersedes an asynchronous automatic focus transfer.
    input_revision: u64,
    viewer_area: Rect,
    hits: Hits,
    pointer: crate::buttons::Pointer,
    /// The input target to return to when Tasks closes.
    tasks_return: Focus,
    link_attach: Option<links_impl::LinkAttach>,
    config_path: std::path::PathBuf,
    settings: Option<crate::settings::Settings>,
    /// The input target to return to when Settings closes.
    settings_return: Focus,
    /// Identifies the open Settings, so a Drover answer meant for an earlier one is not used.
    settings_token: u64,
    /// For Diagnostics: the latest agent and task reads, where the config came from, and the
    /// check running for the open page.
    agents_read: crate::diagnostics::Last,
    tasks_read: crate::diagnostics::Last,
    config_from_file: bool,
    checker: Option<crate::diagnostics::Checker>,
}
impl App {
    fn new(config: Config, config_path: std::path::PathBuf) -> Result<Self> {
        // The config loaded without error, so an existing file is where it came from.
        let config_from_file = config_path.exists();
        let client = Client {
            program: expand_home(&config.corral).to_string_lossy().into_owned(),
        };
        let registered = drover::registered_projects(&expand_home("~/.drover/projects"));
        let registry_error = registered.as_ref().err().map(|error| format!("{error:#}"));
        let projects = registered.unwrap_or_default();
        let current = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let default_project = if projects
            .iter()
            .any(|path| std::path::Path::new(path) == current)
        {
            current.clone()
        } else {
            projects
                .first()
                .map(std::path::PathBuf::from)
                .unwrap_or(current)
        };
        let queue_client = drover::Client {
            program: expand_home(&config.queue.drover)
                .to_string_lossy()
                .into_owned(),
            cwd: config
                .queue
                .cwd
                .as_deref()
                .map(expand_home)
                .unwrap_or(default_project),
        };
        let project = queue_client
            .cwd
            .canonicalize()
            .unwrap_or_else(|_| queue_client.cwd.clone())
            .display()
            .to_string();
        let queue_worker =
            drover::Worker::start(queue_client, Duration::from_millis(config.refresh_ms));
        // Across projects, reads come every five refresh periods; opening Attention rereads.
        let survey = drover::Surveyor::start(
            expand_home(&config.queue.drover)
                .to_string_lossy()
                .into_owned(),
            expand_home("~/.drover/projects"),
            Duration::from_millis(config.refresh_ms.saturating_mul(5)),
        );
        // Drover's notification preference is checked as often as the projects are read.
        let channel = drover::ChannelWorker::start(
            drover::Client {
                program: expand_home(&config.queue.drover)
                    .to_string_lossy()
                    .into_owned(),
                cwd: ".".into(),
            },
            Duration::from_millis(config.refresh_ms.saturating_mul(5)),
        );
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
        Ok(Self {
            layout_store,
            control: crate::control::Server::start()?,
            records: Vec::new(),
            closing: None,
            quit: false,
            poller: Poller::start(client.clone(), Duration::from_millis(config.refresh_ms)),
            git: git::Poller::start("git".into(), Duration::from_secs(5)),
            actions,
            viewer,
            config,
            panel: Panel {
                follow: true,
                ..Default::default()
            },
            focus: Focus::Agents,
            queue: queue::Panel {
                project,
                projects,
                registry_error,
                ..Default::default()
            },
            queue_worker,
            pending_load: None,
            repo_tasks: None,
            detail: None,
            manual_target: None,
            reply: None,
            reply_busy: false,
            reply_due: Instant::now(),
            placement: None,
            search: None,
            survey,
            board: Default::default(),
            attention: None,
            channel,
            notifier: Default::default(),
            toast: None,
            native_mouse: false,
            new_agent: None,
            agent_draft: None,
            input_revision: 0,
            viewer_area: Rect::default(),
            hits: Hits::default(),
            pointer: Default::default(),
            tasks_return: Focus::Agents,
            link_attach: None,
            config_path,
            settings: None,
            settings_return: Focus::Agents,
            settings_token: 0,
            agents_read: None,
            tasks_read: None,
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
            let items = self.board.items(&self.panel, now());
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
                        queue: &mut self.queue,
                        viewer_note: &self.viewer.active_pane().viewer.note,
                        reply: &reply,
                        now: now(),
                        pointer: &self.pointer,
                    },
                    Some(ui::Workspace {
                        terminals: &self.viewer,
                        placement: self.placement.as_ref(),
                        search: self.search.as_mut(),
                        form: self.new_agent.as_mut().filter(|f| f.visible),
                        program: &self.actions.client.program,
                        modal: self.closing.is_some(),
                        attention: ui::Attention {
                            items: &items,
                            loading,
                            popup: self.attention.as_mut(),
                        },
                        settings: self.settings.as_mut(),
                    }),
                );
                self.draw_closing(frame);
                self.toast = self.notifier.visible(Instant::now()).and_then(|toast| {
                    crate::notify::draw(&self.config.colors, frame, panes.viewer, toast)
                });
                if !self.layout_store.notice.is_empty() {
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(self.layout_store.notice.as_str())
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
        let updates: Vec<_> = self.channel.updates.try_iter().collect();
        for update in updates {
            self.channel_update(update);
        }
        if let Some(checks) = self
            .checker
            .as_ref()
            .and_then(|c| c.updates.try_recv().ok())
            && let Some(settings) = &mut self.settings
        {
            settings.checked(checks);
        }
        for update in self.survey.updates.try_iter() {
            match &update {
                drover::Survey::Projects(Ok(projects)) => self.notifier.registry(projects),
                drover::Survey::Snapshot(project, Ok(snapshot)) => {
                    self.notifier.snapshot(project, snapshot, Instant::now())
                }
                _ => {}
            }
            self.board.absorb(update);
        }
        for batch in self.git.updates.try_iter() {
            self.panel.absorb_git(batch);
        }
        let results: Vec<_> = self.actions.receiver.try_iter().collect();
        for result in results {
            match result.action {
                Action::TaskAgent(request) => self.task_agent_result(request, result.result),
                Action::TaskAgentReady(request, ticket) => {
                    self.task_agent_ready(request, ticket, result.result)
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
        if self.focus != Focus::Queue {
            self.repo_tasks = None;
        }
        if let Some(found) = self
            .repo_tasks
            .as_ref()
            .and_then(|(_, _, lookup)| lookup.result.try_recv().ok())
        {
            let (opened, revision, _) = self.repo_tasks.take().unwrap();
            // Only while that opening is untouched: no later input, project choice, page or write.
            if let Some(project) = found
                && project != self.queue.project
                && opened == self.queue.project
                && revision == self.input_revision
                && matches!(self.queue.page, queue::Page::List)
                && !self.queue.busy
            {
                self.queue_request(drover::Request::Project(project));
            }
        }
        for update in self.queue_worker.updates.try_iter() {
            match update {
                drover::Update::Snapshot(Ok(snapshot)) => {
                    self.tasks_read = Some((SystemTime::now(), Ok(())));
                    self.queue.absorb(*snapshot)
                }
                drover::Update::Snapshot(Err(error)) => {
                    self.tasks_read = Some((SystemTime::now(), Err(format!("{error:#}"))));
                    self.queue.read_error = Some(format!("{error:#}"));
                }
                drover::Update::Feedback(operation, result) => {
                    self.queue.complete(&operation, result)
                }
            }
        }
        // Run details refresh only while Tasks is open to show them.
        let wanted = self
            .queue
            .detail_key()
            .filter(|_| self.focus == Focus::Queue);
        if self.detail.as_ref().map(|(key, _)| key) != wanted.as_ref() {
            // Dropping the old worker cancels its query and discards its results before a new
            // target (another task, project or reopened page) starts.
            self.detail = None;
            self.detail = wanted.map(|key| {
                let worker = drover::DetailWorker::start(
                    drover::Client {
                        program: expand_home(&self.config.queue.drover)
                            .to_string_lossy()
                            .into_owned(),
                        cwd: key.project.clone().into(),
                    },
                    key.id.clone(),
                    Duration::from_secs(5),
                );
                (key, worker)
            });
        }
        if let Some((key, worker)) = &self.detail {
            for result in worker.updates.try_iter() {
                self.queue.absorb_detail(key, result);
            }
        }
        // Read once per opening or Refresh, never renewed behind the user's back; closing
        // Tasks drops a reading in progress, and reopening starts it again.
        let wanted = self
            .queue
            .manual_key()
            .filter(|_| self.focus == Focus::Queue);
        if self.manual_target.as_ref().map(|(key, _)| key) != wanted.as_ref() {
            self.manual_target = None;
            self.manual_target = wanted.map(|key| {
                let worker = drover::DetailWorker::start(
                    drover::Client {
                        program: expand_home(&self.config.queue.drover)
                            .to_string_lossy()
                            .into_owned(),
                        cwd: key.project.clone().into(),
                    },
                    key.id.clone(),
                    Duration::MAX,
                );
                (key, worker)
            });
        }
        if let Some((key, worker)) = &self.manual_target {
            for result in worker.updates.try_iter() {
                self.queue.absorb_manual(key, result);
            }
        }
        self.task_links_tick()?;
        if !matches!(self.queue.page, queue::Page::AllPending) {
            self.pending_load = None;
        }
        if let Some(load) = &self.pending_load {
            for (index, result) in load.updates.try_iter() {
                if let Some((_, entry)) = self.queue.all_pending.get_mut(index) {
                    *entry = Some(result.map_err(|error| format!("{error:#}")));
                }
            }
        }
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
            placement::Choice::NewAgent => {
                let project = self
                    .viewer
                    .get(anchor)
                    .and_then(|p| p.source_cwd().map(str::to_owned))
                    .unwrap_or_else(|| self.queue.project.clone());
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
                        pane.viewer
                            .remembered
                            .cwd()
                            .unwrap_or(&self.queue.project)
                            .into(),
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
        if matches!(&event, Event::Key(key) if key.kind != KeyEventKind::Release)
            || matches!(&event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down(_) | MouseEventKind::ScrollUp | MouseEventKind::ScrollDown))
        {
            self.input_revision += 1;
        }
        match event {
            Event::Key(key) => {
                self.pointer.cancel();
                if key.kind == KeyEventKind::Release {
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
                            if self.closing.as_ref().is_some_and(|c| {
                                matches!(c.replacement, Some(Replacement::TaskLink(_)))
                            }) {
                                self.queue.links.checking_agent = None;
                                self.queue.links.message = "Cancelled".into();
                            }
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
                    let submit = form.key(key, &self.queue.projects);
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
                    let items = self.board.items(&self.panel, now());
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
                let before = self.focus;
                let route = self.focus.route(key);
                if before != Focus::Queue && self.focus == Focus::Queue {
                    self.tasks_return = before;
                    self.look_up_focused_repo(before);
                }
                match route {
                    Route::Quit => {
                        self.request_close(crate::control::CloseTarget::All)?;
                        return Ok(self.quit);
                    }
                    Route::Panel => self.panel_key(key),
                    Route::Queue => {
                        let typing = matches!(
                            self.queue.page,
                            queue::Page::Add { .. }
                                | queue::Page::Edit { .. }
                                | queue::Page::Project(_)
                                | queue::Page::Manual(_)
                        );
                        // Closing keeps the page, selection and scroll for the next opening.
                        if (key.code == KeyCode::Char('q') && !typing)
                            || (key.code == KeyCode::Esc
                                && matches!(self.queue.page, queue::Page::List)
                                && !self.queue.reading_link()
                                && !self.queue.reading_dispatch())
                        {
                            self.focus = self.tasks_return;
                        } else if let Some(request) = self.queue.key(key) {
                            self.queue_request(request);
                        }
                    }
                    Route::Terminal => {
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
                if self.focus == Focus::Queue {
                    self.queue.paste(&text);
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
                // The task prompt takes every mouse action over it; none reaches a terminal.
                if let Some((area, close)) = self.toast
                    && area.contains(point)
                    && !self.pointer.captured()
                    && self.notifier.visible(Instant::now()).is_some()
                {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        self.toast_click(close.contains(point));
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
                        self.queue
                            .buttons
                            .iter()
                            .cloned()
                            .map(|h| (Focus::Queue, h)),
                    )
                    .chain(
                        self.hits
                            .terminal
                            .iter()
                            .map(|(hit, _)| (Focus::Viewer, hit.clone())),
                    )
                    .collect();
                let captured = self.pointer.captured();
                if let Some((focus, key)) = self.pointer.event(mouse, &controls) {
                    if focus == Focus::Queue && self.focus != Focus::Queue {
                        // The Tasks entry: open, remembering where input was.
                        self.tasks_return = self.focus;
                        self.look_up_focused_repo(self.focus);
                        self.focus = Focus::Queue;
                        return Ok(false);
                    }
                    if focus == Focus::Queue
                        && key.code == KeyCode::Enter
                        && matches!(self.queue.page, queue::Page::List)
                    {
                        self.queue.view = queue::View::Details;
                        return Ok(false);
                    }
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
                                // Tasks takes the rest of it by itself.
                                self.native_mouse =
                                    matches!(target, crate::attention::Target::Agent(_));
                                self.attention_outcome(crate::attention::Outcome::Open(target));
                            }
                        }
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                            let items = self.board.items(&self.panel, now());
                            popup.scroll(mouse.kind == MouseEventKind::ScrollDown, &items);
                        }
                        _ => {}
                    }
                    return Ok(false);
                }
                if let Some(form) = self.new_agent.as_mut().filter(|f| f.visible) {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        form.click(point, &self.queue.projects);
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
                // The open Tasks popup takes all mouse input; outside it nothing happens.
                if self.focus == Focus::Queue {
                    let scroll = matches!(
                        mouse.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    );
                    let delta = if mouse.kind == MouseEventKind::ScrollUp {
                        -1
                    } else {
                        1
                    };
                    if matches!(mouse.kind, MouseEventKind::Up(_)) {
                        self.native_mouse = false;
                    }
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        if let Some(request) = self.queue.click(mouse.column, mouse.row) {
                            self.queue_request(request);
                        } else if self.queue.list_area.contains(point)
                            && let Some((_, index)) = self
                                .hits
                                .queue_rows
                                .iter()
                                .find(|(row, _)| *row == mouse.row)
                        {
                            self.queue.select(*index);
                        }
                        if self.queue.links.checking_agent.is_some() {
                            self.native_mouse = true;
                        }
                    } else if scroll && self.queue.overlay_open() {
                        self.queue.key(KeyEvent::new(
                            if delta < 0 {
                                KeyCode::Up
                            } else {
                                KeyCode::Down
                            },
                            crossterm::event::KeyModifiers::NONE,
                        ));
                    } else if scroll {
                        self.queue.wheel(mouse.column, mouse.row, delta);
                    }
                    return Ok(false);
                }

                if self.native_mouse {
                    if matches!(mouse.kind, MouseEventKind::Up(_)) {
                        self.native_mouse = false;
                    }
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
        self.settings = Some(crate::settings::Settings::open(
            self.config_path.clone(),
            truecolor(),
        ));
        self.settings_token += 1;
        self.channel.read(drover::Asker::Open(self.settings_token));
        self.settings_return = back;
        self.focus = Focus::Agents;
    }
    /// Every reading calibrates the prompts; the open Settings gets only the answers it asked
    /// for. A save answered after its Settings closed is still reported.
    fn channel_update(&mut self, update: drover::ChannelUpdate) {
        use drover::Asker;
        match (&update.result, update.asker) {
            (Ok(preference), _) => self.notifier.preference(Some(*preference)),
            (Err(_), Asker::Poll | Asker::Open(_)) => self.notifier.preference(None),
            (Err(_), Asker::Save(_)) => {}
        }
        let result = update.result.map(|p| p.system_enabled);
        let open = self.settings.as_mut().filter(
            |_| matches!(update.asker, Asker::Open(t) | Asker::Save(t) if t == self.settings_token),
        );
        match (update.asker, open) {
            (Asker::Open(_), Some(settings)) => settings.channel_status(result),
            (Asker::Save(_), Some(settings)) => {
                let outcome = settings.channel_saved(result);
                self.settings_outcome(outcome);
            }
            (Asker::Save(_), None) => {
                self.panel.message = match result {
                    Ok(system) => format!(
                        "Task notifications: {}; Drover applies it at its next notification check.",
                        if system { "System" } else { "In saddle" }
                    ),
                    Err(error) => format!("Task notifications not saved: {error}"),
                }
            }
            _ => {}
        }
    }
    /// Closing returns input to where it was. A save applies colors and the sidebar width now;
    /// the other settings wait for the next start.
    fn settings_outcome(&mut self, outcome: crate::settings::Outcome) {
        use crate::settings::Outcome;
        match outcome {
            Outcome::Stay => return,
            Outcome::Cancel => {}
            Outcome::SetChannel(saved, system) => {
                if let Some(saved) = saved {
                    self.config.colors = saved.colors.for_terminal(truecolor());
                    self.config.left_width = saved.left_width;
                }
                self.channel
                    .set(drover::Asker::Save(self.settings_token), system);
                return;
            }
            Outcome::Done(message) => self.panel.message = message,
            Outcome::Diagnose => {
                let report = self.diagnostics();
                // Replacing the checker cancels the previous check and drops its answer.
                self.checker = Some(crate::diagnostics::Checker::start(
                    report.corral.clone(),
                    report.drover.clone(),
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
                self.config.colors = saved.colors.for_terminal(truecolor());
                self.config.left_width = saved.left_width;
                self.panel.message = if restart.is_empty() {
                    "Settings saved.".into()
                } else {
                    format!(
                        "Settings saved; restart saddle to apply: {}.",
                        restart.join(", ")
                    )
                };
            }
        }
        self.settings = None;
        self.checker = None;
        self.focus = self.settings_return;
    }
    /// What saddle knows now; the command and config checks run in the background.
    fn diagnostics(&self) -> crate::diagnostics::Report {
        crate::diagnostics::Report {
            corral: self.actions.client.program.clone(),
            drover: expand_home(&self.config.queue.drover)
                .to_string_lossy()
                .into_owned(),
            agents: self.agents_read.clone(),
            project: self.queue.project.clone(),
            tasks: self.tasks_read.clone(),
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
    /// The close mark only closes the prompt. Elsewhere it opens its task in Tasks, or
    /// Attention for several; while another popup has input, only the close mark works.
    fn toast_click(&mut self, close: bool) {
        let busy = self.settings.is_some()
            || self.attention.is_some()
            || self.search.is_some()
            || self.closing.is_some()
            || self.placement.is_some()
            || self.panel.confirm.is_some()
            || self.new_agent.as_ref().is_some_and(|f| f.visible);
        if !close && busy {
            return;
        }
        let Some(toast) = self.notifier.dismiss() else {
            return;
        };
        if close {
            // The release must not reach the terminal under the closed prompt.
            self.native_mouse = !busy && self.focus != Focus::Queue;
        } else if let [
            (
                crate::attention::Target::Task {
                    project,
                    id,
                    title,
                    body,
                },
                _,
            ),
        ] = &toast.targets[..]
        {
            self.open_tasks(
                project.clone(),
                Some(drover::Task {
                    id: id.clone(),
                    title: title.clone(),
                    body: body.clone(),
                    ..Default::default()
                }),
            );
        } else {
            self.attention = Some(Default::default());
            self.survey.refresh();
        }
    }
    /// Opening only shows the target: an agent's terminal, or a task selected in its project's
    /// Tasks. Nothing is answered, released or advanced.
    fn attention_outcome(&mut self, outcome: crate::attention::Outcome) {
        use crate::attention::{Outcome, Target};
        match outcome {
            Outcome::Stay => {}
            Outcome::Cancel => {
                self.attention = None;
                self.focus = Focus::Agents;
            }
            Outcome::Seen(target) => {
                self.board.seen.insert(target);
            }
            Outcome::Open(target) => {
                self.attention = None;
                self.focus = Focus::Agents;
                match target {
                    Target::Agent(name) => {
                        self.panel.select(Some(name));
                        self.attach();
                    }
                    Target::Task {
                        project,
                        id,
                        title,
                        body,
                    } => self.open_tasks(
                        project,
                        Some(drover::Task {
                            id,
                            title,
                            body,
                            ..Default::default()
                        }),
                    ),
                    Target::Project(project) => self.open_tasks(project, None),
                    Target::Source(_) => {}
                }
            }
        }
    }
    /// Opens Tasks on `project`, selecting `task` once listed. An unfinished Tasks page or write
    /// is kept rather than replaced.
    fn open_tasks(&mut self, project: String, task: Option<drover::Task>) {
        let unfinished = matches!(
            self.queue.page,
            queue::Page::Add { .. }
                | queue::Page::Edit { .. }
                | queue::Page::Delete { .. }
                | queue::Page::Project(_)
                | queue::Page::Manual(_)
        ) || (self.queue.busy && self.queue.project != project);
        if unfinished {
            self.panel.message =
                "Tasks has an unfinished action; finish or cancel it, then open this again.".into();
            return;
        }
        if self.queue.project != project {
            self.queue_request(drover::Request::Project(project));
        }
        match task {
            Some(task) => self.queue.locate(task),
            None => self.queue.page = queue::Page::List,
        }
        self.tasks_return = Focus::Agents;
        self.focus = Focus::Queue;
    }
    /// A plain Tasks opening from `from` looks for the focused agent's repository; a found
    /// project with tasks replaces the current one in `tick`. Nothing starts over an unfinished
    /// page or write.
    fn look_up_focused_repo(&mut self, from: Focus) {
        self.repo_tasks = None;
        if !matches!(self.queue.page, queue::Page::List) || self.queue.busy {
            return;
        }
        let Some(cwd) = self.focused_agent_cwd(from) else {
            return;
        };
        let lookup = drover::RepoTasks::start(
            expand_home(&self.config.queue.drover)
                .to_string_lossy()
                .into_owned(),
            cwd,
            self.queue.projects.clone(),
        );
        self.repo_tasks = Some((self.queue.project.clone(), self.input_revision, lookup));
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
            Focus::Queue => return None,
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
                self.survey.refresh();
            }
            KeyCode::Char('n') => {
                self.reload_projects();
                if self.new_agent.as_ref().is_some_and(|f| f.anchor.is_some()) {
                    self.new_agent = self.agent_draft.take();
                }
                self.new_agent
                    .get_or_insert_with(|| crate::launch::Form::new(self.queue.project.clone()))
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
    fn reload_projects(&mut self) -> bool {
        match drover::registered_projects(&expand_home("~/.drover/projects")) {
            Ok(projects) => {
                self.queue.projects = projects;
                self.queue.registry_error = None;
                self.queue.project_selected = self
                    .queue
                    .projects
                    .iter()
                    .position(|p| *p == self.queue.project)
                    .unwrap_or(0);
                true
            }
            Err(error) => {
                self.queue.registry_error = Some(format!("{error:#}"));
                false
            }
        }
    }
    fn queue_request(&mut self, request: drover::Request) {
        if matches!(request, drover::Request::Projects) {
            self.reload_projects();
        } else if matches!(request, drover::Request::AllPending) {
            let projects = if self.reload_projects() {
                self.queue.projects.clone()
            } else {
                Vec::new()
            };
            // Replacing the loader cancels the previous reads and drops their results.
            self.pending_load = Some(drover::PendingLoad::start(
                &expand_home(&self.config.queue.drover).to_string_lossy(),
                &projects,
            ));
            self.queue.all_pending = projects.into_iter().map(|p| (p, None)).collect();
        } else if let drover::Request::Project(path) = request {
            if self.queue.busy {
                return;
            }
            let cwd = expand_home(&path);
            let cwd = cwd.canonicalize().unwrap_or(cwd);
            // Drop the old reader and its result channel before showing the new state.
            self.queue_worker = drover::Worker::start(
                drover::Client {
                    program: expand_home(&self.config.queue.drover)
                        .to_string_lossy()
                        .into_owned(),
                    cwd: cwd.clone(),
                },
                Duration::from_millis(self.config.refresh_ms),
            );
            self.tasks_read = None;
            self.queue = queue::Panel {
                project: cwd.display().to_string(),
                projects: std::mem::take(&mut self.queue.projects),
                registry_error: self.queue.registry_error.take(),
                view: self.queue.view,
                ..Default::default()
            };
        } else {
            self.queue_worker.request(request);
        }
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
            Focus::Queue => None,
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
