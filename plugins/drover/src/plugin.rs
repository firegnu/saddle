//! Drover owns task storage, workers and UI. The host provides generic plugin capabilities.
use crate::{
    attention::{Target, project_name},
    config::expand_home,
    drover, notify, queue,
    theme::Theme,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};
use saddle_plugin_sdk::{Context, Event, Plugin, protocol::AttentionItem};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};

const REFRESH: Duration = Duration::from_secs(2);
type ProjectState = Option<Result<Box<drover::Snapshot>, String>>;
pub struct Drover {
    commands: crate::api::Worker,
    setup: Option<crate::project_setup::Setup>,
    system_notifier: notify::Notifier,
    system_notify: crate::system_notify::Worker,
    pub panel: queue::Panel,
    corral: String,
    refresh: Duration,
    worker: drover::Worker,
    survey: drover::Surveyor,
    channel: drover::ChannelWorker,
    notifier: notify::Notifier,
    projects: Vec<(String, ProjectState)>,
    registry_error: Option<String>,
    attention_ids: BTreeMap<String, String>,
    attention_serial: u64,
    published: Option<Vec<AttentionItem>>,
    targets: BTreeMap<String, Target>,
    seen: HashSet<Target>,
    attention_dirty: bool,
    lookup: Option<(u64, drover::RepoTasks)>,
    input_revision: u64,
    detail: Option<(queue::DetailKey, drover::DetailWorker)>,
    confirmation: Option<(queue::DetailKey, drover::DetailWorker)>,
    pending: Option<drover::PendingLoad>,
    rows: Vec<(u16, usize)>,
    cursor: Option<[u16; 2]>,
    pointer: crate::buttons::Pointer,
    visible: bool,
    preferences: bool,
    close: bool,
    preference: Option<Result<drover::Preference, String>>,
    preference_choice: Option<bool>,
    preference_saving: bool,
    preference_message: String,
    // Release ownership only after all worker threads have stopped.
    lease: Result<crate::core::PluginLease>,
}
impl Default for Drover {
    fn default() -> Self {
        Self::new()
    }
}
impl Drover {
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string();
        let projects =
            drover::registered_projects(&expand_home("~/.drover/projects")).unwrap_or_default();
        let cwd = if projects.contains(&cwd) {
            cwd
        } else {
            projects.first().cloned().unwrap_or(cwd)
        };
        Self::with_commands("corral".into(), cwd)
    }
    pub fn with_commands(corral: String, cwd: String) -> Self {
        Self::with_refresh(corral, cwd, REFRESH)
    }
    pub fn with_refresh(corral: String, cwd: String, refresh: Duration) -> Self {
        let client = drover::Client {
            corral: corral.clone(),
            cwd: cwd.clone().into(),
        };
        let mut this = Self {
            lease: crate::core::PluginLease::acquire(),
            commands: crate::api::Worker::start(corral.clone()),
            setup: None,
            system_notifier: Default::default(),
            system_notify: Default::default(),
            panel: queue::Panel {
                project: cwd,
                ..Default::default()
            },
            worker: drover::Worker::start(client.clone(), refresh),
            survey: drover::Surveyor::start(
                corral.clone(),
                expand_home("~/.drover/projects"),
                refresh.saturating_mul(5),
            ),
            channel: drover::ChannelWorker::start(client, refresh.saturating_mul(5)),
            notifier: Default::default(),
            projects: vec![],
            registry_error: None,
            attention_ids: BTreeMap::new(),
            attention_serial: 0,
            published: None,
            targets: BTreeMap::new(),
            seen: HashSet::new(),
            attention_dirty: false,
            lookup: None,
            input_revision: 0,
            corral,
            refresh,
            detail: None,
            confirmation: None,
            pending: None,
            rows: vec![],
            cursor: None,
            pointer: Default::default(),
            visible: false,
            preferences: false,
            close: false,
            preference: None,
            preference_choice: None,
            preference_saving: false,
            preference_message: String::new(),
        };
        this.reload_projects();
        this
    }
    fn client(&self, project: &str) -> drover::Client {
        drover::Client {
            corral: self.corral.clone(),
            cwd: project.into(),
        }
    }
    fn reload_projects(&mut self) -> bool {
        match drover::registered_projects(&expand_home("~/.drover/projects")) {
            Ok(projects) => {
                self.panel.project_status = projects
                    .iter()
                    .map(|p| {
                        let status = match crate::core::project::inspect(std::path::Path::new(p)) {
                            Ok(v) if v["state"] == "registered" => format!(
                                "Added · {}",
                                v["main_agent"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or("No receiver (manual)")
                            ),
                            Ok(v) => format!(
                                "Unavailable: {}",
                                v["error"].as_str().unwrap_or("Invalid configuration")
                            ),
                            Err(e) => format!("Unavailable: {e:#}"),
                        };
                        (p.clone(), status)
                    })
                    .collect();
                self.panel.projects = projects;
                self.panel.registry_error = None;
                self.panel.project_selected = self
                    .panel
                    .projects
                    .iter()
                    .position(|p| p == &self.panel.project)
                    .unwrap_or(0);
                true
            }
            Err(e) => {
                self.panel.registry_error = Some(format!("{e:#}"));
                false
            }
        }
    }
    pub fn request(&mut self, request: drover::Request) {
        match request {
            drover::Request::ProjectSetup(path, settings) => {
                if !self.panel.busy {
                    self.setup = Some(crate::project_setup::Setup::new(
                        self.corral.clone(),
                        path,
                        settings,
                    ));
                }
            }
            drover::Request::Projects => {
                self.reload_projects();
            }
            drover::Request::AllPending => {
                let projects = if self.reload_projects() {
                    self.panel.projects.clone()
                } else {
                    vec![]
                };
                self.pending = Some(drover::PendingLoad::start(&self.corral, &projects));
                self.panel.all_pending = projects.into_iter().map(|p| (p, None)).collect();
            }
            drover::Request::Project(path) => {
                if self.panel.busy {
                    return;
                }
                let target = expand_home(&path);
                if !crate::core::project::inspect(&target).is_ok_and(|v| v["state"] == "registered")
                {
                    self.setup = Some(crate::project_setup::Setup::new(
                        self.corral.clone(),
                        path,
                        false,
                    ));
                    return;
                }
                let cwd = target;
                let cwd = cwd.canonicalize().unwrap_or(cwd).display().to_string();
                self.detail = None;
                self.confirmation = None;
                self.pending = None;
                self.worker = drover::Worker::start(self.client(&cwd), self.refresh);
                self.panel = queue::Panel {
                    project: cwd,
                    projects: std::mem::take(&mut self.panel.projects),
                    project_status: std::mem::take(&mut self.panel.project_status),
                    registry_error: self.panel.registry_error.take(),
                    view: self.panel.view,
                    ..Default::default()
                };
            }
            drover::Request::Refresh => {
                self.worker.request(drover::Request::Refresh);
                self.survey.refresh();
            }
            other => self.worker.request(other),
        }
    }
    fn observe(&mut self, context: &mut Context) -> Result<bool> {
        let mut changed = false;
        for update in self.channel.updates.try_iter() {
            if matches!(update.asker, drover::Asker::Save(_))
                && let Err(e) = &update.result
            {
                self.preference_saving = false;
                self.preference_message = format!("Not saved: {e}");
                changed = true;
                continue;
            }
            let preference = update.result.as_ref().ok().copied();
            self.notifier.preference(preference);
            self.system_notifier.preference(preference.map(|mut p| {
                p.system_enabled = !p.system_enabled;
                p
            }));
            if matches!(update.asker, drover::Asker::Save(_)) {
                self.preference_saving = false;
                self.preference_message = match &update.result {
                    Ok(_) => {
                        "Saved. The plugin applies this preference at its next notification check."
                            .into()
                    }
                    Err(e) => format!("Not saved: {e}"),
                };
            }
            if self.preference_choice.is_none() || !self.preferences {
                self.preference_choice = update.result.as_ref().ok().map(|p| p.system_enabled);
            }
            self.preference = Some(update.result);
            changed = true;
        }
        for update in self.survey.updates.try_iter() {
            changed = true;
            match update {
                drover::Survey::Projects(Ok(list)) => {
                    self.notifier.registry(&list);
                    self.system_notifier.registry(&list);
                    let mut old = std::mem::take(&mut self.projects);
                    self.projects = list
                        .into_iter()
                        .map(|p| {
                            let state = old
                                .iter_mut()
                                .find(|(o, _)| *o == p)
                                .and_then(|(_, s)| s.take());
                            (p, state)
                        })
                        .collect();
                    self.registry_error = None;
                }
                drover::Survey::Projects(Err(e)) => {
                    self.projects.clear();
                    self.registry_error = Some(e);
                }
                drover::Survey::Snapshot(project, result) => {
                    if let Ok(snapshot) = &result {
                        self.notifier.snapshot(&project, snapshot, Instant::now());
                        self.system_notifier
                            .snapshot(&project, snapshot, Instant::now());
                    }
                    if let Some((_, state)) = self.projects.iter_mut().find(|(p, _)| p == &project)
                    {
                        *state = Some(result);
                    }
                }
            }
        }
        if changed {
            self.publish(context)?;
        }
        if let Some(toast) = self.system_notifier.dismiss() {
            self.system_notify.send(toast.text());
        }
        if let Some(toast) = self.notifier.dismiss() {
            let item = toast.targets.first().and_then(|(target, _)| {
                self.targets
                    .iter()
                    .find(|(_, t)| *t == target)
                    .map(|(id, _)| id.clone())
            });
            context.notify_target(
                bounded_text(&toast.text(), 1024),
                Some(saddle_plugin_sdk::protocol::OpenTarget {
                    action: "open".into(),
                    target: if toast.targets.len() == 1 && item.is_some() {
                        serde_json::json!({"id":item})
                    } else {
                        serde_json::json!({"projects":true})
                    },
                }),
            )?;
        }
        Ok(changed)
    }
    fn publish(&mut self, context: &mut Context) -> Result<()> {
        let mut entries = vec![];
        if let Some(error) = &self.registry_error {
            entries.push((
                Target::Source("registry".into()),
                "Project registry unavailable".into(),
                error.clone(),
            ));
        }
        for (project, state) in &self.projects {
            match state {
                Some(Ok(s)) => {
                    for task in s.awaiting.iter().chain(
                        s.history
                            .iter()
                            .rev()
                            .filter(|t| t.status.as_deref() == Some("failed")),
                    ) {
                        let target = Target::task(project, task);
                        if task.status.as_deref() == Some("failed") && self.seen.contains(&target) {
                            continue;
                        }
                        entries.push((
                            target,
                            format!(
                                "{} · {}",
                                project_name(project),
                                task.id.as_deref().unwrap_or("—")
                            ),
                            format!(
                                "{} · {}",
                                if task.status.as_deref() == Some("failed") {
                                    "Failed"
                                } else {
                                    "Awaiting release"
                                },
                                task.title
                            ),
                        ));
                    }
                }
                Some(Err(e)) => entries.push((
                    Target::Project(project.clone()),
                    project_name(project).into(),
                    e.clone(),
                )),
                None => {}
            }
        }
        let mut items = vec![];
        self.targets.clear();
        if entries.len() > 64 {
            let more = entries.len() - 63;
            entries.truncate(63);
            entries.push((
                Target::Source("more".into()),
                format!("{more} more items"),
                "Open Drover projects to inspect remaining tasks.".into(),
            ));
        }
        for (target, title, note) in entries {
            let key = serde_json::to_string(&target)?;
            let id = self
                .attention_ids
                .entry(key)
                .or_insert_with(|| {
                    self.attention_serial += 1;
                    format!("task-{}", self.attention_serial)
                })
                .clone();
            self.targets.insert(id.clone(), target);
            items.push(AttentionItem {
                target: serde_json::json!({"id":id}),
                id,
                title: bounded_text(if title.is_empty() { "Drover" } else { &title }, 128),
                note: bounded_text(&note, 512),
                action: "open".into(),
            });
        }
        self.attention_ids
            .retain(|_, id| self.targets.contains_key(id));
        if self.published.as_ref() != Some(&items) {
            context.attention(items.clone())?;
            self.published = Some(items);
        }
        Ok(())
    }
    fn open_target(&mut self, target: Target) {
        if self.setup.is_some()
            || self.preferences
            || self.panel.busy
            || !matches!(
                self.panel.page,
                queue::Page::List | queue::Page::Help | queue::Page::Feedback(_)
            )
        {
            self.panel.message =
                "Finish or cancel the current action, then open this item again.".into();
            return;
        }
        let (project, task) = match target {
            Target::Task {
                project,
                id,
                title,
                body,
            } => (
                project,
                Some(drover::Task {
                    id,
                    title,
                    body,
                    ..Default::default()
                }),
            ),
            Target::Project(project) => (project, None),
            Target::Source(_) => {
                self.panel.page = queue::Page::Projects;
                self.reload_projects();
                return;
            }
        };
        if project != self.panel.project {
            self.request(drover::Request::Project(project));
        }
        if let Some(task) = task {
            self.panel.locate(task);
        } else {
            self.panel.page = queue::Page::List;
        }
    }
    fn tick(&mut self, context: &mut Context) -> Result<()> {
        for (id, result) in self.commands.results.try_iter() {
            context.command_result(id, result)?;
            self.worker.request(drover::Request::Refresh);
            self.survey.refresh();
        }

        let mut changed = self.observe(context)?;
        if let Some(setup) = &mut self.setup {
            changed |= setup.poll();
            if let Some((project, message)) = setup.opened.take() {
                changed = true;
                self.setup = None;
                self.reload_projects();
                self.request(drover::Request::Project(project));
                if message.starts_with("Project added, but") {
                    self.panel.page = queue::Page::Feedback(message.clone());
                }
                self.panel.message = message;
                self.survey.refresh();
            }
        }
        if let Some(project) = self
            .lookup
            .as_ref()
            .and_then(|(_, job)| job.result.try_recv().ok())
        {
            let (revision, _) = self.lookup.take().unwrap();
            if revision == self.input_revision
                && self.setup.is_none()
                && !self.panel.busy
                && matches!(self.panel.page, queue::Page::List)
                && let Some(project) = project
                && project != self.panel.project
            {
                self.request(drover::Request::Project(project));
                changed = true;
            }
        }
        for update in self.worker.updates.try_iter() {
            changed = true;
            match update {
                drover::Update::Snapshot(Ok(s)) => self.panel.absorb(*s),
                drover::Update::Snapshot(Err(e)) => self.panel.read_error = Some(format!("{e:#}")),
                drover::Update::Feedback(op, result) => {
                    self.panel.complete(&op, result);
                    self.survey.refresh();
                }
            }
        }
        let wanted = self.panel.detail_key().filter(|_| self.visible);
        if self.detail.as_ref().map(|(k, _)| k) != wanted.as_ref() {
            self.detail = None;
            self.detail = wanted.map(|key| {
                let worker = drover::DetailWorker::start(
                    self.client(&key.project),
                    key.id.clone(),
                    Duration::from_secs(5),
                    true,
                );
                (key, worker)
            });
        }
        if let Some((key, worker)) = &self.detail {
            for result in worker.updates.try_iter() {
                self.panel.absorb_detail(key, result);
                changed = true;
            }
        }
        let wanted = self.panel.confirmation_key().filter(|_| self.visible);
        if self.confirmation.as_ref().map(|(k, _)| k) != wanted.as_ref() {
            self.confirmation = None;
            self.confirmation = wanted.map(|key| {
                let worker = drover::DetailWorker::start(
                    self.client(&key.project),
                    key.id.clone(),
                    Duration::MAX,
                    false,
                );
                (key, worker)
            });
        }
        if let Some((key, worker)) = &self.confirmation {
            for result in worker.updates.try_iter() {
                self.panel.absorb_confirmation(key, result);
                changed = true;
            }
        }
        if !matches!(self.panel.page, queue::Page::AllPending) {
            self.pending = None;
        }
        if let Some(load) = &self.pending {
            for (index, result) in load.updates.try_iter() {
                if let Some((_, state)) = self.panel.all_pending.get_mut(index) {
                    *state = Some(result.map_err(|e| format!("{e:#}")));
                    changed = true;
                }
            }
        }
        if self.visible {
            self.panel.tick_links();
        }
        if changed {
            context.redraw();
        }
        Ok(())
    }
    fn preference_key(&mut self, key: KeyEvent) {
        if self.preference_saving {
            return;
        }
        if key.code == KeyCode::Esc {
            self.preferences = false;
            return;
        }
        if !matches!(self.preference, Some(Ok(_))) {
            return;
        }
        match key.code {
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(system) = self.preference_choice {
                    self.channel.set(drover::Asker::Save(1), system);
                    self.preference_saving = true;
                    self.preference_message = "Saving…".into();
                }
            }
            KeyCode::Char('s') => self.preference_choice = Some(true),
            KeyCode::Char('i') => self.preference_choice = Some(false),
            _ => {}
        }
    }
    fn panel_key(&mut self, key: KeyEvent) -> Option<drover::Request> {
        if let Some(setup) = &mut self.setup {
            setup.key(key);
            if setup.closed {
                self.setup = None;
            }
            return None;
        }
        if self.preferences {
            self.preference_key(key);
            return None;
        }
        if key.code == KeyCode::Char('m')
            && matches!(self.panel.page, queue::Page::List)
            && !self.panel.busy
            && let Some((_, task)) = self.panel.tasks().get(self.panel.selected)
            && task.status.as_deref() == Some("failed")
        {
            self.seen.insert(Target::task(&self.panel.project, task));
            self.attention_dirty = true;
            self.panel.message = "Marked seen in Attention for this plugin session.".into();
            return None;
        }
        if key.code == KeyCode::Char('N')
            && matches!(self.panel.page, queue::Page::List)
            && !self.panel.busy
        {
            self.preferences = true;
            self.preference_choice = self
                .preference
                .as_ref()
                .and_then(|p| p.as_ref().ok())
                .map(|p| p.system_enabled);
            self.preference_message.clear();
            self.channel.read(drover::Asker::Open(1));
            return None;
        }
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) && !self.escape_input() {
            self.close = true;
            return None;
        }
        self.panel.key(key)
    }
    fn input(&mut self, value: &Value) {
        self.input_revision += 1;
        self.lookup = None;
        let event = &value["event"];
        let request = match event["type"].as_str() {
            Some("key") => {
                self.pointer.cancel();
                key(event).and_then(|key| self.panel_key(key))
            }
            Some("paste") => {
                if self.preferences {
                    return;
                }
                if let Some(text) = event["text"].as_str() {
                    if let Some(setup) = &mut self.setup {
                        setup.paste(text);
                    } else {
                        self.panel.paste(text);
                    }
                }
                None
            }
            Some("mouse") => {
                let x = event["x"].as_u64().unwrap_or(0) as u16;
                let y = event["y"].as_u64().unwrap_or(0) as u16;
                use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
                let kind = match event["action"].as_str() {
                    Some("down") if event["button"] == "left" => {
                        MouseEventKind::Down(MouseButton::Left)
                    }
                    Some("up") if event["button"] == "left" => {
                        MouseEventKind::Up(MouseButton::Left)
                    }
                    _ => MouseEventKind::Moved,
                };
                let hits: Vec<_> = self
                    .panel
                    .buttons
                    .iter()
                    .cloned()
                    .map(|h| (crate::buttons::Focus::View, h))
                    .collect();
                let captured = self.pointer.captured();
                if let Some((_, key)) = self.pointer.event(
                    MouseEvent {
                        kind,
                        column: x,
                        row: y,
                        modifiers: KeyModifiers::NONE,
                    },
                    &hits,
                ) {
                    self.panel_key(key)
                } else if captured || self.pointer.captured() {
                    None
                } else if event["action"] == "down" && event["button"] == "left" {
                    if self.preferences {
                        return;
                    }
                    if let Some(setup) = &mut self.setup {
                        setup.click(x, y);
                        return;
                    }
                    let request = self.panel.click(x, y);
                    if request.is_none()
                        && self.panel.list_area.contains((x, y).into())
                        && let Some((_, index)) = self.rows.iter().find(|(row, _)| *row == y)
                    {
                        self.panel.select(*index);
                    }
                    request
                } else if event["action"] == "scroll" {
                    let delta = event["dy"].as_i64().unwrap_or(0).signum() as isize;
                    if self.setup.is_some() {
                        self.panel_key(KeyEvent::new(
                            if delta < 0 {
                                KeyCode::Up
                            } else {
                                KeyCode::Down
                            },
                            KeyModifiers::NONE,
                        ))
                    } else if self.panel.overlay_open() {
                        self.panel.key(KeyEvent::new(
                            if delta < 0 {
                                KeyCode::Up
                            } else {
                                KeyCode::Down
                            },
                            KeyModifiers::NONE,
                        ))
                    } else {
                        self.panel.wheel(x, y, delta);
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(request) = request {
            self.request(request);
        }
    }
}
impl Plugin for Drover {
    fn id(&self) -> &str {
        "drover"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn event(&mut self, event: Event, context: &mut Context) -> Result<()> {
        if let Err(e) = &self.lease {
            anyhow::bail!(
                "Drover is already owned by another Saddle, or its data is unavailable: {e:#}"
            );
        }
        match event {
            Event::Command { id, method, params } => {
                if let Err(e) = self.commands.request(id.clone(), method, params) {
                    context.command_result(id,serde_json::json!({"ok":false,"error":{"code":"busy","message":e.to_string()}}))?;
                }
            }

            Event::Tick => self.tick(context)?,
            Event::Input(value) => {
                self.input(&value);
                if std::mem::take(&mut self.attention_dirty) {
                    self.publish(context)?;
                }
                if std::mem::take(&mut self.close) {
                    context.close_view()?;
                }
                if let Some(request) = self.panel.links.agent_request.take()
                    && let Err(e) = context.open_agent(&request.name, &request.instance)
                {
                    self.panel.links.checking_agent = None;
                    self.panel.links.message = e.to_string();
                }
                if let Some(filter) = self.panel.telemetry_request.take()
                    && let Err(e) = context.open_telemetry(filter)
                {
                    self.panel.message_failed = true;
                    self.panel.message = format!("Telemetry did not open: {e}");
                }
                context.redraw();
            }
            Event::AgentOpened {
                status, message, ..
            } => {
                self.panel.links.checking_agent = None;
                self.panel.links.message = if status == "opened" {
                    String::new()
                } else {
                    message
                };
                context.redraw();
            }
            Event::TelemetryOpened {
                status, message, ..
            } => {
                if status != "opened" {
                    self.panel.message_failed = true;
                    self.panel.message = format!("Telemetry did not open ({status}): {message}");
                }
                context.redraw();
            }
            Event::Focus(focused) => {
                if !focused {
                    self.pointer.cancel();
                }
                self.visible |= focused;
            }
            Event::Closed => {
                self.pointer.cancel();
                self.lookup = None;
                self.visible = false;
                self.detail = None;
                self.confirmation = None;
            }
            Event::NotificationOpen(open) => {
                self.lookup = None;
                if open.target["projects"] == true {
                    self.open_target(Target::Source("projects".into()));
                } else if let Some(target) = open.target["id"]
                    .as_str()
                    .and_then(|id| self.targets.get(id))
                    .cloned()
                {
                    self.open_target(target);
                }
                context.redraw();
            }
            Event::AttentionOpen(open) => {
                self.lookup = None;
                if let Some(target) = self.targets.get(&open.item_id).cloned() {
                    self.open_target(target);
                    context.redraw();
                }
            }
            Event::Opened(value) => {
                if self.setup.is_none()
                    && !self.panel.busy
                    && matches!(self.panel.page, queue::Page::List)
                    && let Some(cwd) = value["cwd"].as_str()
                {
                    self.lookup = Some((
                        self.input_revision,
                        drover::RepoTasks::start(
                            self.corral.clone(),
                            cwd.into(),
                            self.panel.projects.clone(),
                        ),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn render(&mut self, area: Rect, colors: &Value) -> Result<Buffer> {
        self.visible = true;
        let mut t = Theme::default();
        for (name, field) in [
            ("text", &mut t.text),
            ("muted", &mut t.muted),
            ("background", &mut t.bg),
            ("accent", &mut t.focus),
            ("error", &mut t.agent_error),
        ] {
            if let Ok(color) = serde_json::from_value(colors[name].clone()) {
                *field = saddle_plugin_sdk::terminal_color(&color);
            }
        }
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))?;
        terminal.draw(|frame| {
            if let Some(setup) = &mut self.setup {
                self.panel.buttons = setup.draw(&t, frame, area);
                self.rows.clear();
            } else if self.preferences {
                use crate::buttons::Button as B;
                let ready = matches!(self.preference, Some(Ok(_))) && !self.preference_saving;
                let body = crate::ui::dialog(&t, frame, area, "Task notifications", 76, 16);
                let controls = [B::control("Save ^s", KeyCode::Char('s'), ready).primary(), B::new("Cancel Esc", KeyCode::Esc, !self.preference_saving)];
                let (body, hits) = crate::buttons::draw_compact(&t, frame, body, &controls);
                self.panel.buttons = hits;
                self.rows.clear();
                let state = match &self.preference {
                    None => "Reading Drover's notification preference…".into(),
                    Some(Err(e)) => format!("Unavailable: {e}"),
                    Some(Ok(_)) => format!("Selected: {}", if self.preference_choice == Some(true) { "System" } else { "In Saddle" }),
                };
                use ratatui::{style::{Modifier, Style}, widgets::{Paragraph, Wrap}};
                if body.height >= 9 {
                    frame.render_widget(Paragraph::new("Choose where task notifications appear.").style(Style::default().fg(t.muted)), Rect::new(body.x, body.y, body.width, 1));
                    for (offset, label, key, choice) in [(2, "System s", 's', true), (4, "In Saddle i", 'i', false)] {
                        let row = Rect::new(body.x, body.y + offset, body.width, 1);
                        let selected = matches!(self.preference, Some(Ok(_))) && self.preference_choice == Some(choice);
                        frame.render_widget(Paragraph::new(format!(" {}  {label}", if selected { "●" } else { "○" })).style(
                            if selected { Style::default().fg(t.focus).bg(t.agent_selected).add_modifier(Modifier::BOLD) }
                            else { Style::default().fg(if ready { t.text } else { t.muted }) }
                        ), row);
                        if ready {
                            self.panel.buttons.push(crate::buttons::Hit { area: row, danger: false, key: KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE) });
                        }
                    }
                    frame.render_widget(Paragraph::new(format!("{state}\n{}", self.preference_message)).wrap(Wrap { trim: false }), Rect::new(body.x, body.y + 6, body.width, 3));
                    frame.render_widget(Paragraph::new("Applies across projects for this user. Changes take effect after Save.").style(Style::default().fg(t.muted)).wrap(Wrap { trim: false }), Rect::new(body.x, body.y + 9, body.width, body.height.saturating_sub(9)));
                } else {
                    let choices = [B::new("System s", KeyCode::Char('s'), ready), B::new("In Saddle i", KeyCode::Char('i'), ready)];
                    let (body, hits) = crate::buttons::draw_compact_top(&t, frame, body, &choices);
                    self.panel.buttons.extend(hits);
                    frame.render_widget(Paragraph::new(format!("{state}\n{}", self.preference_message)).wrap(Wrap { trim: false }), body);
                }
            } else {
                let content = Rect { height: area.height.saturating_sub(1), ..area };
                self.rows = self.panel.draw(&t, frame, content);
                if area.height > 0 {
                    let status = Rect::new(area.x, area.bottom()-1, area.width, 1);
                    frame.render_widget(ratatui::widgets::Paragraph::new(self.panel.message.as_str()).style(ratatui::style::Style::default().fg(if self.panel.message_failed { t.agent_error } else if self.panel.busy { t.agent_working } else { t.text })), status);
                }
            }
            let hits: Vec<_> = self.panel.buttons.iter().cloned().map(|h| (crate::buttons::Focus::View, h)).collect();
            self.pointer.paint(&t, frame, &hits);
        })?;
        let backend = terminal.backend();
        self.cursor = backend.cursor_visible().then(|| {
            let p = backend.cursor_position();
            [p.x, p.y]
        });
        Ok(backend.buffer().clone())
    }
    fn escape_input(&self) -> bool {
        self.setup.is_some()
            || self.preferences
            || !matches!(self.panel.page, queue::Page::List)
            || self.panel.reading_link()
    }
    fn cursor(&self) -> Option<[u16; 2]> {
        self.cursor
    }
}
fn bounded_text(text: &str, max: usize) -> String {
    let mut s = String::new();
    for c in text.chars().filter(|c| !c.is_control()) {
        if s.len() + c.len_utf8() > max {
            break;
        }
        s.push(c);
    }
    s
}
fn key(v: &Value) -> Option<KeyEvent> {
    if v["phase"] == "release" {
        return None;
    }
    let code = if let Some(c) = v["code"]["char"].as_str().and_then(|s| s.chars().next()) {
        KeyCode::Char(c)
    } else {
        match v["code"]["name"].as_str()? {
            "enter" => KeyCode::Enter,
            "esc" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "page_up" => KeyCode::PageUp,
            "page_down" => KeyCode::PageDown,
            "tab" => KeyCode::Tab,
            "back_tab" => KeyCode::BackTab,
            "delete" => KeyCode::Delete,
            "insert" => KeyCode::Insert,
            "function" => KeyCode::F(v["code"]["number"].as_u64()?.try_into().ok()?),
            _ => return None,
        }
    };
    let mut modifiers = KeyModifiers::NONE;
    for (name, flag) in [
        ("shift", KeyModifiers::SHIFT),
        ("control", KeyModifiers::CONTROL),
        ("alt", KeyModifiers::ALT),
        ("super", KeyModifiers::SUPER),
    ] {
        if v["modifiers"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == name))
        {
            modifiers |= flag;
        }
    }
    Some(KeyEvent::new(code, modifiers))
}
