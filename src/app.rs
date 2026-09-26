use crate::{
    agents::Panel,
    config::{Config, expand_home},
    corral::{Client, Poller},
    drover, git,
    input::{Focus, Route, encode_key, encode_mouse, encode_paste},
    layout::Panes,
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
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind,
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

#[derive(Clone)]
enum Action {
    Attach(String, Ticket),
    Reply(String),
    Stop(String),
    Start(Vec<String>, Ticket),
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
                Action::Start(args, _) => client.json(
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                    Duration::from_secs(120),
                    &cancel,
                ),
                _ => {
                    let (verb, name, timeout) = match &action {
                        Action::Attach(name, _) => ("status", name, 15),
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
            LeaveAlternateScreen,
            Show
        );
        let _ = disable_raw_mode();
    }
}

pub fn run(config: Config) -> Result<()> {
    let mut app = App::new(config);
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    app.run(&mut terminal)
}
struct App {
    config: Config,
    panel: Panel,
    focus: Focus,
    poller: Poller,
    git: git::Poller,
    actions: Actions,
    viewer: Terminals,
    queue: queue::Panel,
    queue_worker: drover::Worker,
    pending_load: Option<drover::PendingLoad>,
    detail: Option<(queue::DetailKey, drover::DetailWorker)>,
    reply: Option<(String, String)>,
    reply_busy: bool,
    reply_due: Instant,
    open_agent: Option<String>,
    native_mouse: bool,
    new_agent: Option<crate::launch::Form>,
    viewer_area: Rect,
    hits: Hits,
    pointer: crate::buttons::Pointer,
    left_queue: bool,
}
impl App {
    fn new(config: Config) -> Self {
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
        Self {
            poller: Poller::start(client.clone(), Duration::from_millis(config.refresh_ms)),
            git: git::Poller::start("git".into(), Duration::from_secs(5)),
            actions: Actions::new(client.clone()),
            viewer: Terminals::new(client.program),
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
            detail: None,
            reply: None,
            reply_busy: false,
            reply_due: Instant::now(),
            open_agent: None,
            native_mouse: false,
            new_agent: None,
            viewer_area: Rect::default(),
            hits: Hits::default(),
            pointer: Default::default(),
            left_queue: false,
        }
    }
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            let size = terminal.size()?;
            if self.focus != Focus::Viewer {
                self.left_queue = self.focus == Focus::Queue;
            }
            let panes = Panes::with_queue(
                Rect::new(0, 0, size.width, size.height),
                &self.config,
                self.left_queue,
            );
            self.tick(panes)?;
            let reply = self
                .reply
                .as_ref()
                .filter(|(name, _)| Some(name) == self.panel.selected.as_ref())
                .map(|(_, text)| text.as_str())
                .unwrap_or("loading…");
            terminal.draw(|frame| {
                self.hits = ui::draw_workspace(
                    frame,
                    &mut self.panel,
                    View {
                        colors: &self.config.colors,
                        panes,
                        focus: self.focus,
                        showing: self.viewer.active_pane().viewer.showing.as_deref(),
                        viewer: self.viewer.active_pane().viewer.session.as_ref(),
                        queue: &mut self.queue,
                        viewer_note: &self.viewer.active_pane().viewer.note,
                        reply,
                        now: now(),
                        pointer: &self.pointer,
                    },
                    Some(ui::Workspace {
                        terminals: &self.viewer,
                        open_agent: self.open_agent.as_deref(),
                        form: self.new_agent.as_mut().filter(|f| f.visible),
                        program: &self.actions.client.program,
                    }),
                );
            })?;
            if event::poll(Duration::from_millis(30))? && self.event(event::read()?, panes)? {
                break;
            }
        }
        Ok(())
    }
    fn tick(&mut self, panes: Panes) -> Result<()> {
        self.viewer_area = panes.viewer;
        for update in self.poller.updates.try_iter() {
            match update {
                Ok(agents) => {
                    self.viewer
                        .disappeared(&agents.iter().map(|a| a.name.as_str()).collect::<Vec<_>>())?;
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
                Err(error) => self.panel.message = format!("corral: {error:#}"),
            }
        }
        for batch in self.git.updates.try_iter() {
            self.panel.absorb_git(batch);
        }
        let results: Vec<_> = self.actions.receiver.try_iter().collect();
        for result in results {
            match result.action {
                Action::Attach(name, ticket) if self.viewer.valid(ticket) => match result.result {
                    Ok(status) if status["attached"].as_u64().unwrap_or(0) > 0 => {
                        self.viewer.complete(ticket, None)?;
                        self.panel.message =
                            format!("{name} is attached elsewhere; Ctrl-] there first")
                    }
                    Ok(_) => {
                        self.viewer.complete(ticket, Some(name))?;
                        self.panel.message.clear();
                        if self.focus == Focus::Agents
                            && self.panel.confirm.is_none()
                            && self.open_agent.is_none()
                            && !self.new_agent.as_ref().is_some_and(|f| f.visible)
                            && self.viewer.active_pane().id == ticket.pane
                        {
                            self.focus = Focus::Viewer;
                        }
                    }
                    Err(error) => {
                        self.viewer.complete(ticket, None)?;
                        self.panel.message = format!("{error:#}");
                    }
                },
                Action::Start(_, ticket) => {
                    let result = result.result.and_then(|value| {
                        value["name"]
                            .as_str()
                            .filter(|n| !n.is_empty())
                            .map(str::to_owned)
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "corral start returned no name; check Agents before retrying"
                                )
                            })
                    });
                    match result {
                        Ok(name) => {
                            let visible = self.new_agent.as_ref().is_some_and(|f| f.visible);
                            self.new_agent = None;
                            self.panel.message = format!("Started {name}");
                            self.poller.refresh();
                            if self.viewer.valid(ticket) {
                                if let Some(id) = self.viewer.find(&name) {
                                    self.viewer.complete(ticket, None)?;
                                    if visible {
                                        self.viewer.focus(id);
                                    }
                                } else {
                                    self.viewer.expect(ticket, name.clone());
                                    self.actions.start(Action::Attach(name, ticket));
                                }
                            } else {
                                self.panel
                                    .message
                                    .push_str("; target closed or replaced. Open it from Agents.");
                            }
                        }
                        Err(error) => {
                            self.viewer.complete(ticket, None)?;
                            self.panel.message = format!("{error:#}");
                            if let Some(form) = &mut self.new_agent {
                                form.busy = None;
                                form.error = format!("{error:#}");
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
        for update in self.queue_worker.updates.try_iter() {
            match update {
                drover::Update::Snapshot(Ok(snapshot)) => self.queue.absorb(*snapshot),
                drover::Update::Snapshot(Err(error)) => {
                    self.queue.read_error = Some(format!("{error:#}"));
                }
                drover::Update::Feedback(operation, result) => {
                    self.queue.complete(&operation, result)
                }
            }
        }
        let wanted = self.queue.detail_key();
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
        self.attach_at(Place::Current);
    }
    fn attach_at(&mut self, place: Place) {
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
            let ticket = self.viewer.reserve(place, Some(name.clone()));
            self.actions.start(Action::Attach(name.clone(), ticket));
            self.panel.message = format!("attaching {name}…");
        }
    }
    fn terminal_control(&mut self, control: Control) -> Result<()> {
        match control {
            Control::NewTab => {
                self.viewer.new_tab();
            }
            Control::Tab(id) => self.viewer.active = id,
            Control::Pane(id) => self.viewer.focus(id),
            Control::CloseTab(id) => self.viewer.close_tab(id)?,
            Control::ClosePane(id) => self.viewer.close_pane(id)?,
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
        match event {
            Event::Key(key) => {
                self.pointer.cancel();
                if key.kind == KeyEventKind::Release {
                    return Ok(false);
                }
                if let Some(form) = self.new_agent.as_mut().filter(|f| f.visible) {
                    if form.key(key, &self.queue.projects) {
                        match form.args() {
                            Ok(args) => {
                                let ticket = self.viewer.reserve(Place::ALL[form.place], None);
                                form.busy = Some(ticket);
                                form.error.clear();
                                self.actions.start(Action::Start(args, ticket));
                            }
                            Err(error) => {
                                form.error = format!("{error:#}");
                                form.reveal_invalid();
                            }
                        }
                    }
                    return Ok(false);
                }
                if self.open_agent.is_some() {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char(']' | '5'))
                    {
                        self.open_agent = None;
                        self.focus = Focus::Agents;
                        return Ok(false);
                    }
                    match key.code {
                        KeyCode::Esc => self.open_agent = None,
                        KeyCode::Char(c @ '1'..='6') if key.modifiers.is_empty() => {
                            self.panel.select(self.open_agent.take());
                            self.attach_at(Place::ALL[c as usize - '1' as usize]);
                        }
                        _ => {}
                    }
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
                match self.focus.route(key) {
                    Route::Quit => return Ok(true),
                    Route::Panel => self.panel_key(key),
                    Route::Queue => {
                        if key.code == KeyCode::Char('q')
                            && !matches!(
                                self.queue.page,
                                queue::Page::Add { .. }
                                    | queue::Page::Edit { .. }
                                    | queue::Page::Project(_)
                            )
                        {
                            self.queue.page = queue::Page::List;
                            self.focus = Focus::Agents;
                        } else if let Some(request) = self.queue.key(key) {
                            self.queue_request(request);
                        }
                    }
                    Route::Terminal => {
                        if let Some(session) = self.focused_session() {
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
                if let Some(form) = self.new_agent.as_mut().filter(|f| f.visible) {
                    form.paste(&text);
                    return Ok(false);
                }
                if self.open_agent.is_some() {
                    return Ok(false);
                }
                if self.focus == Focus::Queue {
                    self.queue.paste(&text);
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
                    self.focus = focus;
                    return self.event(Event::Key(key), panes);
                }
                if captured || self.pointer.captured() {
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
                if self.panel.confirm.is_some() || self.open_agent.is_some() {
                    return Ok(false);
                }
                if self.focus == Focus::Queue && self.queue.overlay_open() {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        if let Some(request) = self.queue.click(mouse.column, mouse.row) {
                            self.queue_request(request);
                        }
                    } else if matches!(
                        mouse.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    ) {
                        self.queue.key(KeyEvent::new(
                            if mouse.kind == MouseEventKind::ScrollUp {
                                KeyCode::Up
                            } else {
                                KeyCode::Down
                            },
                            crossterm::event::KeyModifiers::NONE,
                        ));
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
                    if panes.tabs.contains(point) {
                        self.focus = if mouse.column - panes.tabs.x < 9 {
                            Focus::Agents
                        } else {
                            Focus::Queue
                        };
                        self.left_queue = self.focus == Focus::Queue;
                        return Ok(false);
                    }
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
                    } else if panes.queue.contains(point) {
                        self.focus = Focus::Queue;
                        if let Some(request) = self.queue.click(mouse.column, mouse.row) {
                            self.queue_request(request);
                            return Ok(false);
                        }
                        if let Some((_, index)) = self
                            .hits
                            .queue_rows
                            .iter()
                            .find(|(row, _)| *row == mouse.row)
                        {
                            self.queue.open(*index);
                        }
                        return Ok(false);
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
                if panes.queue.contains(point)
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    )
                {
                    self.queue.wheel(
                        mouse.column,
                        mouse.row,
                        if mouse.kind == MouseEventKind::ScrollUp {
                            -1
                        } else {
                            1
                        },
                    );
                } else if panes.agents.contains(point)
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
    fn panel_key(&mut self, key: KeyEvent) {
        self.panel.message.clear();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.panel.move_selection(-1, now()),
            KeyCode::Down | KeyCode::Char('j') => self.panel.move_selection(1, now()),
            KeyCode::Enter => self.attach(),
            KeyCode::Char('o') => self.open_agent = self.panel.selected.clone(),
            KeyCode::Char('n') => {
                self.reload_projects();
                self.new_agent
                    .get_or_insert_with(|| crate::launch::Form::new(self.queue.project.clone()))
                    .visible = true;
            }
            KeyCode::Char('s') => {
                self.panel.by_state = !self.panel.by_state;
                self.panel.follow = true;
            }
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
            self.queue = queue::Panel {
                project: cwd.display().to_string(),
                projects: std::mem::take(&mut self.queue.projects),
                registry_error: self.queue.registry_error.take(),
                ..Default::default()
            };
        } else {
            self.queue_worker.request(request);
        }
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
