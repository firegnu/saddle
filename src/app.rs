use crate::{
    agents::Panel,
    config::{Config, expand_home},
    corral::{Client, Poller},
    drover,
    input::{Focus, Route, encode_key, encode_mouse, encode_paste},
    layout::Panes,
    pty::Session,
    queue,
    terminal::Size,
    ui::{self, Hits, View},
    viewer::Viewer,
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
    Attach(String, u64),
    Reply(String),
    Stop(String),
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
            let (verb, name, timeout) = match &action {
                Action::Attach(name, _) => ("status", name, 15),
                Action::Reply(name) => ("reply", name, 15),
                Action::Stop(name) => ("stop", name, 120),
            };
            let result = client.json(&[verb, name], Duration::from_secs(timeout), &cancel);
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
    actions: Actions,
    viewer: Viewer,
    queue: queue::Panel,
    queue_worker: drover::Worker,
    reply: Option<(String, String)>,
    reply_busy: bool,
    reply_due: Instant,
    attach_sequence: u64,
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
            actions: Actions::new(client.clone()),
            viewer: Viewer::new(client.program),
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
            reply: None,
            reply_busy: false,
            reply_due: Instant::now(),
            attach_sequence: 0,
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
                self.hits = ui::draw(
                    frame,
                    &mut self.panel,
                    View {
                        panes,
                        focus: self.focus,
                        showing: self.viewer.showing.as_deref(),
                        viewer: self.viewer.session.as_ref(),
                        queue: &mut self.queue,
                        viewer_note: &self.viewer.note,
                        reply,
                        now: now(),
                        pointer: &self.pointer,
                    },
                );
            })?;
            if event::poll(Duration::from_millis(30))? && self.event(event::read()?, panes)? {
                break;
            }
        }
        Ok(())
    }
    fn tick(&mut self, panes: Panes) -> Result<()> {
        for update in self.poller.updates.try_iter() {
            match update {
                Ok(agents) => {
                    self.viewer
                        .disappeared(&agents.iter().map(|a| a.name.as_str()).collect::<Vec<_>>())?;
                    self.panel
                        .absorb(agents, self.viewer.showing.as_deref(), now());
                }
                Err(error) => self.panel.message = format!("corral: {error:#}"),
            }
        }
        for result in self.actions.receiver.try_iter() {
            match result.action {
                Action::Attach(name, seq) if seq == self.attach_sequence => match result.result {
                    Ok(status) if status["attached"].as_u64().unwrap_or(0) > 0 => {
                        self.panel.message =
                            format!("{name} is attached elsewhere; Ctrl-] there first")
                    }
                    Ok(_) => {
                        self.viewer.select(name)?;
                        self.panel.message.clear();
                        if self.focus == Focus::Agents && self.panel.confirm.is_none() {
                            self.focus = Focus::Viewer;
                        }
                    }
                    Err(error) => self.panel.message = format!("{error:#}"),
                },
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
        self.viewer.tick(size_of(panes.viewer))?;
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
        if let Some(name) = &self.panel.selected {
            self.attach_sequence += 1;
            if self.viewer.showing.as_ref() == Some(name) {
                if let Err(error) = self.viewer.select(name.clone()) {
                    self.panel.message = error.to_string();
                }
                self.focus = Focus::Viewer;
                return;
            }
            self.actions
                .start(Action::Attach(name.clone(), self.attach_sequence));
            self.panel.message = format!("attaching {name}…");
        }
    }
    fn event(&mut self, event: Event, panes: Panes) -> Result<bool> {
        match event {
            Event::Key(key) => {
                self.pointer.cancel();
                if key.kind == KeyEventKind::Release {
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
                                queue::Page::Add { .. } | queue::Page::Project(_)
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
                    .collect();
                let captured = self.pointer.captured();
                if let Some((focus, key)) = self.pointer.event(mouse, &controls) {
                    self.focus = focus;
                    return self.event(Event::Key(key), panes);
                }
                if captured || self.pointer.captured() {
                    return Ok(false);
                }
                if self.panel.confirm.is_some() {
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
                            self.queue.select(*index);
                        }
                        return Ok(false);
                    } else if panes.viewer.contains(point) {
                        self.focus = Focus::Viewer;
                    }
                }
                if panes.queue.contains(point)
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    )
                {
                    self.queue.key(KeyEvent::new(
                        if mouse.kind == MouseEventKind::ScrollUp {
                            KeyCode::Up
                        } else {
                            KeyCode::Down
                        },
                        crossterm::event::KeyModifiers::NONE,
                    ));
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
                    let area = ui::inner(panes.viewer);
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
    fn queue_request(&mut self, request: drover::Request) {
        if matches!(request, drover::Request::Projects) {
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
                }
                Err(error) => self.queue.registry_error = Some(format!("{error:#}")),
            }
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
            Focus::Viewer => self.viewer.session.as_ref(),
        }
    }
}
fn size_of(area: Rect) -> Size {
    let area = ui::inner(area);
    Size {
        rows: area.height.max(1),
        cols: area.width.max(2),
    }
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
