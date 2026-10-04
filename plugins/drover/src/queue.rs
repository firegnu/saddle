use crate::drover::{Operation, Request, Snapshot, Task, Transition};
use crate::theme::Theme;
#[path = "queue_links.rs"]
mod links_impl;
pub use crate::launch_edit::Input;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What the Tasks popup shows. `List` is the list with the selected task beside it; every
/// other page replaces both inside the same popup.
#[derive(Default)]
pub enum Page {
    #[default]
    List,
    Help,
    Feedback(String),
    Projects,
    Project(String),
    AllPending,
    Edit {
        pending: Vec<Task>,
        index: usize,
        title: Input,
        body: Input,
        body_focus: bool,
    },
    Add {
        title: Input,
        body: Input,
        body_focus: bool,
    },
    Delete {
        pending: Vec<Task>,
        index: usize,
    },
    /// Confirming one explicit task transition.
    Confirm(Box<Confirmation>),
}
/// The selected transition confirmation, bound to the run its own task detail read read.
pub struct Confirmation {
    pub action: Transition,
    pub run_id: String,
    pub work_stopped: bool,
    /// The project and task it opened on; `seq` is new for each reading of the target.
    pub key: DetailKey,
    pub title: String,
    /// That reading: `None` while it runs.
    pub target: Option<Result<Box<crate::drover::Detail>, String>>,
    pub reason: String,
    /// Drover said the run changed; only a new reading and a new confirmation go on.
    pub expired: bool,
}
/// The Submit for review / Accept button key. No key press produces it: terminals never send
/// Null, and the list ignores Alt combinations.
pub fn transition_click() -> KeyEvent {
    KeyEvent::new(KeyCode::Null, KeyModifiers::ALT)
}
/// The Return to pending button has no terminal shortcut.
pub fn return_click() -> KeyEvent {
    KeyEvent::new(KeyCode::Null, KeyModifiers::CONTROL)
}
/// The Dispatch selected button has no terminal shortcut either.
pub fn dispatch_selected_click() -> KeyEvent {
    KeyEvent::new(KeyCode::Null, KeyModifiers::SUPER)
}
/// The Record toggle beside Dispatch selected: click only, for that one dispatch.
pub fn record_click() -> KeyEvent {
    KeyEvent::new(KeyCode::Null, KeyModifiers::META)
}
/// The Telemetry ↗ button of a numbered task: click only.
pub fn telemetry_click() -> KeyEvent {
    KeyEvent::new(KeyCode::Null, KeyModifiers::HYPER)
}
/// Why the page cannot confirm its target now, or `None` when it can.
pub fn confirmation_problem(confirmation: &Confirmation) -> Option<String> {
    let id = &confirmation.key.id;
    let detail = match &confirmation.target {
        None => return Some(format!("Reading {id}'s current run…")),
        Some(Err(error)) => {
            return Some(format!(
                "Could not read {id}'s current run: {error}. Refresh to retry."
            ));
        }
        Some(Ok(detail)) => detail,
    };
    if detail.task.id.as_ref() != Some(id)
        || detail.task.run_id.as_deref() != Some(&confirmation.run_id)
    {
        return Some("The task run changed; cancel and select it again.".into());
    }
    if !confirmation
        .action
        .allows(detail.task.status.as_deref().unwrap_or_default())
    {
        return Some(format!(
            "{id} is no longer in the state required for {}.",
            confirmation.action.label()
        ));
    }
    let Some(target) = detail.task.actions.get(confirmation.action.command()) else {
        return Some("This action is unavailable for the shown task.".into());
    };
    if confirmation.expired {
        return Some("This run's confirmation expired. Refresh, then confirm again.".into());
    }
    match &target.target_token {
        Some(token) if !token.is_empty() && target.unavailable_reason.is_none() => None,
        _ => Some(format!(
            "Confirmation unavailable now: {}. Refresh to try again.",
            target
                .unavailable_reason
                .as_deref()
                .map(crate::detail::phrase)
                .unwrap_or_else(|| "no reason given".into())
        )),
    }
}
/// Read-only views of the selected task beside the list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Text,
    Details,
    Links,
}
#[derive(Default)]
pub struct Panel {
    pub(crate) buttons: Vec<crate::buttons::Hit>,
    pub(crate) fields: Vec<(ratatui::layout::Rect, bool)>,
    pub projects: Vec<String>,
    pub project_selected: usize,
    pub project_status: std::collections::BTreeMap<String, String>,
    pub registry_error: Option<String>,
    pub project_rows: Vec<(ratatui::layout::Rect, usize)>,
    pub project: String,
    pub read_error: Option<String>,
    pub snapshot: Option<Snapshot>,
    pub selected: usize,
    pub top: usize,
    pub(crate) manual_scroll: bool,
    pub(crate) list_area: ratatui::layout::Rect,
    pub scroll: usize,
    pub page: Page,
    /// The selected task as shown beside the list; replaced when the selection moves to
    /// another task, so each selection gets its own task detail read target.
    pub content: Option<Box<crate::detail::TaskDetail>>,
    pub view: View,
    pub links: crate::links::State,
    pub(crate) text_scroll: usize,
    pub(crate) content_area: ratatui::layout::Rect,
    pub message: String,
    pub(crate) message_failed: bool,
    pub busy: bool,
    pub(crate) selection_after_write: Option<(usize, Task)>,
    pub all_pending: Vec<ProjectPending>,
    /// A task to select once the list has it, found by identity rather than position.
    pub(crate) locate: Option<Task>,
    /// A cancelled confirmation reason, by task id and action, for the next opening.
    pub(crate) return_draft: Option<(String, Transition, String)>,
    /// This dispatch's recording choice when the user changed it from the project default.
    pub record: Option<bool>,
    /// A Telemetry ↗ press for the plugin to hand to the host, taken from the input callback.
    pub telemetry_request: Option<saddle_plugin_sdk::protocol::TelemetryFilter>,
    /// Where the view was last opened from and the project it shows, set by the plugin.
    pub source: Option<String>,
}
/// Which task a detail result belongs to; a reopened page gets a new `seq`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailKey {
    pub project: String,
    pub id: String,
    pub seq: u64,
}
/// A registered project and its pending tasks: `None` while loading, `Err` with the read error.
pub type ProjectPending = (String, Option<Result<Vec<Task>, String>>);
impl Panel {
    pub fn click(&mut self, column: u16, row: u16) -> Option<Request> {
        if self.view == View::Links
            && matches!(self.page, Page::List)
            && let Some((_, index)) = self
                .links
                .rows
                .iter()
                .find(|(area, _)| area.contains((column, row).into()))
        {
            if let Some(reading) = &mut self.links.reading {
                reading.selected = *index;
            } else {
                self.links.selected = *index;
            }
            self.links.open();
            return None;
        }
        let point = (column, row).into();
        if let Some(hit) = self.buttons.iter().find(|hit| hit.area.contains(point)) {
            if hit.key == transition_click()
                || hit.key == return_click()
                || hit.key == dispatch_selected_click()
                || hit.key == record_click()
                || hit.key == telemetry_click()
            {
                return self.key(hit.key);
            }
            if hit.key.code == KeyCode::Null {
                self.view = View::Links;
                return None;
            }
            if hit.key.code == KeyCode::Enter
                && matches!(self.page, Page::List)
                && self.view == View::Links
            {
                self.view = View::Details;
                return None;
            }
            return self.key(hit.key);
        }
        if self.busy {
            return None;
        }
        if let Some((_, index)) = self
            .project_rows
            .iter()
            .find(|(area, _)| area.contains(point))
        {
            return self.projects.get(*index).cloned().map(Request::Project);
        }
        if let Page::Add {
            title,
            body,
            body_focus,
        }
        | Page::Edit {
            title,
            body,
            body_focus,
            ..
        } = &mut self.page
            && let Some((_, on_body)) = self.fields.iter().find(|(area, _)| area.contains(point))
        {
            *body_focus = *on_body;
            if *on_body { body } else { title }.click(point);
        }
        None
    }

    fn controls(&self) -> Vec<crate::buttons::Button<'static>> {
        use crate::buttons::Button as B;
        use KeyCode as K;
        match self.page {
            Page::Add { .. } | Page::Edit { .. } => vec![
                B::control(
                    "Save ^s",
                    K::Char('s'),
                    !self.busy && self.read_error.is_none(),
                )
                .primary(),
                B::new("Cancel Esc", K::Esc, !self.busy),
            ],
            Page::Delete { .. } => vec![
                B::new(
                    "Delete y",
                    K::Char('y'),
                    !self.busy && self.read_error.is_none(),
                )
                .danger(),
                B::new("Cancel Esc", K::Esc, !self.busy),
            ],
            Page::Confirm(ref confirmation) => {
                let mut buttons = Vec::new();
                if confirmation.action == Transition::Return {
                    buttons.push(B::control(
                        if confirmation.work_stopped {
                            "[x] Work has stopped ^w"
                        } else {
                            "[ ] Work has stopped ^w"
                        },
                        K::Char('w'),
                        !self.busy,
                    ));
                }
                buttons.extend([
                    B::new(
                        match confirmation.action {
                            Transition::Submit => "Submit for review ↵",
                            Transition::Accept => "Accept ↵",
                            Transition::Return => "Return to pending ↵",
                        },
                        K::Enter,
                        !self.busy
                            && confirmation_problem(confirmation).is_none()
                            && (confirmation.action != Transition::Return
                                || (confirmation.work_stopped
                                    && !confirmation.reason.trim().is_empty())),
                    )
                    .primary(),
                    B::control(
                        "Refresh ^r",
                        K::Char('r'),
                        !self.busy && confirmation.target.is_some(),
                    ),
                    B::new("Cancel Esc", K::Esc, !self.busy),
                ]);
                buttons
            }
            Page::Project(_) => vec![
                B::new("Apply ↵", K::Enter, !self.busy),
                B::new("Cancel Esc", K::Esc, true),
            ],
            Page::AllPending => vec![
                B::new("Refresh r", K::Char('r'), true),
                B::new("Back Esc", K::Esc, true),
            ],
            Page::Projects => vec![
                B::new("Open ↵", K::Enter, !self.projects.is_empty()),
                B::new("Refresh r", K::Char('r'), true),
                B::new("Add project a", K::Char('a'), true),
                B::new("Settings s", K::Char('s'), !self.projects.is_empty()),
                B::new("Path e", K::Char('e'), true),
                B::new("Back Esc", K::Esc, true),
            ],
            _ => vec![B::new("Back Esc", K::Esc, true)],
        }
    }
    pub fn tasks(&self) -> Vec<(&'static str, &Task)> {
        let Some(s) = &self.snapshot else {
            return Vec::new();
        };
        s.current
            .iter()
            .map(|t| ("Current", t))
            .chain(s.awaiting.iter().map(|t| ("Awaiting", t)))
            .chain(s.pending.iter().map(|t| ("Pending", t)))
            .chain(s.history.iter().map(|t| ("History", t)))
            .collect()
    }
    fn pending_index(&self) -> Option<usize> {
        let s = self.snapshot.as_ref()?;
        let index = self
            .selected
            .checked_sub(usize::from(s.current.is_some()) + usize::from(s.awaiting.is_some()))?;
        (index < s.pending.len()).then_some(index)
    }
    /// The selected Pending task's shown position and token, while the queue could start it:
    /// fresh data, nothing running, awaiting or paused, and a target drover offered.
    fn dispatch_target(&self) -> Option<(u64, String)> {
        let s = self.snapshot.as_ref()?;
        if self.busy
            || self.read_error.is_some()
            || !matches!(self.page, Page::List)
            || s.paused
            || s.current.is_some()
            || s.awaiting.is_some()
        {
            return None;
        }
        let target = s
            .pending
            .get(self.pending_index()?)?
            .actions
            .get("dispatch-pending")?;
        match (
            &target.pos,
            &target.target_token,
            &target.unavailable_reason,
        ) {
            (Some(pos), Some(token), None) if *pos > 0 && !token.is_empty() => {
                Some((*pos, token.clone()))
            }
            _ => None,
        }
    }
    /// Where the list has the shown task now: by id, or by content if unnumbered.
    fn live(&self, detail: &crate::detail::TaskDetail) -> Option<(&'static str, &Task)> {
        self.tasks()
            .into_iter()
            .find(|(_, t)| same_task(t, &detail.task))
    }
    /// The task detail read target while Run details or Links is chosen; only numbered tasks are covered.
    pub fn detail_key(&self) -> Option<DetailKey> {
        if !matches!(self.view, View::Details | View::Links) {
            return None;
        }
        let detail = self.content.as_ref()?;
        let id = detail.task.id.as_ref().filter(|id| {
            id.strip_prefix('T')
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })?;
        Some(DetailKey {
            project: self.project.clone(),
            id: id.clone(),
            seq: detail.seq,
        })
    }
    /// Takes a show result only if it belongs to the page open now.
    pub fn absorb_detail(
        &mut self,
        key: &DetailKey,
        result: anyhow::Result<crate::drover::Detail>,
    ) {
        if self.detail_key().as_ref() != Some(key) {
            return;
        }
        if let Some(detail) = &mut self.content {
            match result {
                Ok(data)
                    if data.task.id == detail.task.id && data.task.run_id == detail.task.run_id =>
                {
                    detail.data = Some(Box::new(data));
                    detail.error = None;
                }
                Ok(_) => detail.error = Some("Task run changed; refresh the task list.".into()),
                Err(error) => detail.error = Some(format!("{error:#}")),
            }
        }
    }
    /// The target reading the confirmation page waits for, if any.
    pub fn confirmation_key(&self) -> Option<DetailKey> {
        match &self.page {
            Page::Confirm(confirmation) if confirmation.target.is_none() => {
                Some(confirmation.key.clone())
            }
            _ => None,
        }
    }
    /// Takes a target reading only if it is the one the open page waits for.
    pub fn absorb_confirmation(
        &mut self,
        key: &DetailKey,
        result: anyhow::Result<crate::drover::Detail>,
    ) {
        if let Page::Confirm(confirmation) = &mut self.page
            && confirmation.key == *key
            && confirmation.target.is_none()
        {
            if let Ok(detail) = &result
                && detail.task.id.as_ref() == Some(&confirmation.key.id)
            {
                confirmation.title.clone_from(&detail.task.title);
            }
            confirmation.target = Some(result.map(Box::new).map_err(|e| format!("{e:#}")));
        }
    }
    /// Opens the confirmation page on the selected Running or Awaiting task.
    fn open_confirmation(&mut self, returning: bool) {
        if self.busy || self.read_error.is_some() {
            return;
        }
        let Some((group, task)) = self.tasks().get(self.selected).copied() else {
            return;
        };
        let action = match (returning, group) {
            (true, "Current" | "Awaiting") => Transition::Return,
            (false, "Current") => Transition::Submit,
            (false, "Awaiting") => Transition::Accept,
            _ => return,
        };
        let (Some(id), Some(run_id)) = (task.id.clone(), task.run_id.clone()) else {
            return;
        };
        let title = task.title.clone();
        let reason = match self.return_draft.take() {
            Some((draft, kind, reason)) if draft == id && kind == action => reason,
            _ => String::new(),
        };
        self.page = Page::Confirm(Box::new(Confirmation {
            action,
            run_id,
            work_stopped: false,
            key: DetailKey {
                project: self.project.clone(),
                id,
                seq: crate::detail::opening(),
            },
            title,
            target: None,
            reason,
            expired: false,
        }));
        self.scroll = 0;
        self.message.clear();
    }
    fn confirmation_key_press(&mut self, key: KeyEvent) -> Option<Request> {
        let Page::Confirm(confirmation) = &mut self.page else {
            return None;
        };
        if self.busy {
            return None;
        }
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                let reason = std::mem::take(&mut confirmation.reason);
                self.return_draft =
                    Some((confirmation.key.id.clone(), confirmation.action, reason));
                self.page = Page::List;
                self.scroll = 0;
                self.message.clear();
            }
            KeyCode::Char('r') if control => {
                // A new reading; the old one's answer no longer matches the key.
                confirmation.key.seq = crate::detail::opening();
                confirmation.target = None;
                confirmation.expired = false;
                confirmation.work_stopped = false;
                self.message.clear();
            }
            KeyCode::Char('w') if control && (confirmation.action == Transition::Return) => {
                confirmation.work_stopped = !confirmation.work_stopped;
            }
            KeyCode::Char('u') if control => confirmation.reason.clear(),
            KeyCode::Enter => {
                if let Some(problem) = confirmation_problem(confirmation) {
                    self.message_failed = true;
                    self.message = problem;
                    return None;
                }
                if confirmation.action == Transition::Return && !confirmation.work_stopped {
                    self.message_failed = true;
                    self.message =
                        "Confirm that work has stopped before returning this task".into();
                    return None;
                }
                if confirmation.action == Transition::Return
                    && confirmation.reason.trim().is_empty()
                {
                    self.message_failed = true;
                    self.message = "Reason is required".into();
                    return None;
                }
                let Some(Ok(detail)) = &confirmation.target else {
                    return None;
                };
                let token = detail
                    .task
                    .actions
                    .get(confirmation.action.command())?
                    .target_token
                    .clone()?;
                self.busy = true;
                self.message_failed = false;
                self.message = format!("{}…", confirmation.action.label());
                return Some(Request::Run(Operation::Transition {
                    project: confirmation.key.project.clone(),
                    id: confirmation.key.id.clone(),
                    run_id: confirmation.run_id.clone(),
                    token,
                    action: confirmation.action,
                    reason: confirmation.reason.trim().into(),
                }));
            }
            KeyCode::Backspace => {
                confirmation.reason.pop();
            }
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                confirmation.reason.push(c)
            }
            _ => {}
        }
        None
    }
    /// Keeps the content beside the list on the selected task; a different task starts
    /// afresh at the top.
    fn sync_content(&mut self) {
        let selected = self
            .tasks()
            .get(self.selected)
            .map(|(group, task)| (*group, (*task).clone()));
        let Some((group, task)) = selected else {
            self.content = None;
            self.links = Default::default();
            self.record = None;
            return;
        };
        if !self.content.as_ref().is_some_and(|content| {
            same_task(&content.task, &task)
                && content.group == group
                && content.task.run_id == task.run_id
        }) {
            self.content = Some(Box::new(crate::detail::TaskDetail::new(group, task)));
            self.text_scroll = 0;
            self.links = Default::default();
            self.record = None;
        }
    }
    fn edit(&mut self) {
        if self.busy || self.read_error.is_some() {
            return;
        }
        if let Some(index) = self.pending_index() {
            let pending = self.snapshot.as_ref().unwrap().pending.clone();
            let task = &pending[index];
            let title = Input::new(task.title.clone());
            let body = Input::new(task.body.clone());
            self.page = Page::Edit {
                pending,
                index,
                title,
                body,
                body_focus: false,
            };
            self.message.clear();
        }
    }
    fn finish_edit(&mut self) {
        self.page = Page::List;
    }
    pub fn absorb(&mut self, snapshot: Snapshot) {
        self.read_error = None;
        let old = if let Some((index, task)) = self.selection_after_write.take() {
            let offset =
                usize::from(snapshot.current.is_some()) + usize::from(snapshot.awaiting.is_some());
            Some((index + offset, task))
        } else {
            self.tasks()
                .get(self.selected)
                .map(|(_, t)| (self.selected, (*t).clone()))
        };
        self.snapshot = Some(snapshot);
        let tasks = self.tasks();
        self.selected = old
            .and_then(|(index, old)| {
                let matches = |(_, t): &(&str, &Task)| same_task(t, &old);
                if tasks.get(index).is_some_and(matches) {
                    Some(index)
                } else {
                    tasks.iter().position(matches)
                }
            })
            .unwrap_or(self.selected)
            .min(tasks.len().saturating_sub(1));
        self.sync_content();
        self.apply_locate();
    }
    /// Shows the list with `task` selected now, or as soon as a snapshot arrives.
    pub fn locate(&mut self, task: Task) {
        self.page = Page::List;
        self.locate = Some(task);
        self.apply_locate();
    }
    fn apply_locate(&mut self) {
        if self.snapshot.is_none() {
            return;
        }
        let Some(task) = self.locate.take() else {
            return;
        };
        match self.tasks().iter().position(|(_, t)| same_task(t, &task)) {
            Some(index) => self.select(index),
            None => {
                self.message_failed = true;
                self.message = format!(
                    "{} is no longer in this project's queue.",
                    task.id.as_deref().unwrap_or(&task.title)
                );
            }
        }
    }
    /// Selects a task row; the content beside the list follows it.
    pub fn select(&mut self, index: usize) {
        self.selected = index.min(self.tasks().len().saturating_sub(1));
        self.scroll = 0;
        self.manual_scroll = false;
        self.sync_content();
    }
    fn scroll_content(&mut self, delta: isize) {
        if self.view == View::Links {
            self.links.scroll(delta);
            return;
        }
        match (self.view, &mut self.content) {
            (View::Details, Some(detail)) => detail.scroll_by(delta),
            // Clamped when drawn, like the other plain-text pages.
            _ => self.text_scroll = self.text_scroll.saturating_add_signed(delta),
        }
    }
    fn content_page(&self) -> isize {
        match (self.view, &self.content) {
            (View::Details, Some(detail)) => detail.page(),
            _ => self.content_area.height.saturating_sub(3).max(1) as isize,
        }
    }
    pub fn wheel(&mut self, column: u16, row: u16, delta: isize) {
        if self.overlay_open() {
            return;
        }
        if self.content_area.contains((column, row).into()) {
            self.scroll_content(delta);
        } else if self.list_area.contains((column, row).into()) {
            if self.read_error.is_some() {
                self.scroll = self.scroll.saturating_add_signed(delta);
            } else {
                self.top = self.top.saturating_add_signed(delta);
                self.manual_scroll = true;
            }
        }
    }
    pub fn paste(&mut self, text: &str) {
        if self.busy {
            return;
        }
        if let Page::Project(path) = &mut self.page {
            path.extend(text.chars().filter(|c| !c.is_control()));
            return;
        }
        if let Page::Confirm(confirmation) = &mut self.page {
            confirmation
                .reason
                .extend(text.chars().map(|c| if c.is_control() { ' ' } else { c }));
            return;
        }
        if let Page::Add {
            title,
            body,
            body_focus,
        }
        | Page::Edit {
            title,
            body,
            body_focus,
            ..
        } = &mut self.page
        {
            let field = if *body_focus { body } else { title };
            field.insert(&text.replace('\t', ""), *body_focus);
        }
    }
    pub fn complete(&mut self, operation: &Operation, result: anyhow::Result<String>) {
        self.busy = false;
        match result {
            Ok(text) => {
                self.message_failed = false;
                self.message = text.clone();
                if matches!(
                    operation,
                    Operation::Transition { .. } | Operation::DispatchPending { .. }
                ) {
                    if let Operation::Transition {
                        id,
                        action: Transition::Return,
                        ..
                    } = operation
                        && let Some(task) = self
                            .tasks()
                            .into_iter()
                            .map(|(_, t)| t)
                            .find(|t| t.id.as_ref() == Some(id))
                    {
                        self.selection_after_write = Some((0, task.clone()));
                    }
                    self.page = Page::Feedback(text);
                    self.scroll = 0;
                    self.view = View::Details;
                    self.content = None;
                } else if let Operation::Edit {
                    pending,
                    index,
                    title,
                    body,
                } = operation
                {
                    self.selection_after_write = pending.get(*index).cloned().map(|mut task| {
                        task.title = title.clone();
                        task.body = body.clone();
                        (*index, task)
                    });
                    if let Some((_, task)) = &self.selection_after_write
                        && let Some(snapshot) = &mut self.snapshot
                        && let Some(old) = snapshot.pending.get_mut(*index)
                        && Some(&*old) == pending.get(*index)
                    {
                        *old = task.clone();
                    }
                    self.finish_edit();
                    self.manual_scroll = false;
                } else if matches!(operation, Operation::Add { .. }) {
                    self.page = Page::List;
                } else if let Operation::Move { pending, index, to } = operation {
                    self.selection_after_write =
                        pending.get(*index).cloned().map(|task| (*to, task));
                    self.page = Page::List;
                    self.manual_scroll = false;
                } else if let Operation::Delete { pending, index } = operation {
                    // Follow the neighbour; the dropped task itself reappears in History.
                    self.selection_after_write = pending
                        .get(index + 1)
                        .map(|task| (*index, task.clone()))
                        .or_else(|| {
                            let before = index.checked_sub(1)?;
                            pending.get(before).map(|task| (before, task.clone()))
                        });
                    self.page = Page::List;
                    self.manual_scroll = false;
                }
            }
            Err(error) => {
                self.message_failed = true;
                self.message = format!("{error:#}");
                if let Page::Confirm(confirmation) = &mut self.page {
                    // Never retried here: an expired target needs a new reading and consent.
                    confirmation.expired = true;
                    confirmation.work_stopped = false;
                    return;
                }
                if !matches!(self.page, Page::Add { .. } | Page::Edit { .. }) {
                    self.page = Page::Feedback(self.message.clone());
                    self.scroll = 0;
                }
            }
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> Option<Request> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if matches!(self.page, Page::Confirm(_)) {
            return self.confirmation_key_press(key);
        }
        if key == transition_click() || key == return_click() {
            if matches!(self.page, Page::List) {
                self.open_confirmation(key == return_click());
            }
            return None;
        }
        if key == dispatch_selected_click() {
            let (pos, token) = self.dispatch_target()?;
            self.busy = true;
            self.message_failed = false;
            self.message = "Dispatching selected task…".into();
            return Some(Request::Run(Operation::DispatchPending {
                project: self.project.clone(),
                pos,
                token,
                // Kept if this fails; a new task selection starts from the default again.
                record: self.record,
            }));
        }
        if key == record_click() {
            if self.dispatch_target().is_some() {
                self.record = Some(!self.recording());
            }
            return None;
        }
        if key == telemetry_click() {
            if let (Page::List, Some(id)) = (
                &self.page,
                self.content.as_ref().and_then(|c| c.task.id.clone()),
            ) {
                let scope = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.project.clone())
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| self.project.clone());
                // Every run of the task: no run, so the page lists them all.
                self.telemetry_request = Some(saddle_plugin_sdk::protocol::TelemetryFilter {
                    kind: crate::telemetry::KIND.into(),
                    scope,
                    key: id,
                    run: None,
                });
            }
            return None;
        }
        if matches!(self.page, Page::Projects) {
            match key.code {
                KeyCode::Esc => self.page = Page::List,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.project_selected = self.project_selected.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.project_selected =
                        (self.project_selected + 1).min(self.projects.len().saturating_sub(1))
                }
                KeyCode::Enter => {
                    return self
                        .projects
                        .get(self.project_selected)
                        .cloned()
                        .map(Request::Project);
                }
                KeyCode::Char('a') => {
                    return Some(Request::ProjectSetup(self.project.clone(), false));
                }
                KeyCode::Char('s') => {
                    return self
                        .projects
                        .get(self.project_selected)
                        .cloned()
                        .map(|p| Request::ProjectSetup(p, true));
                }
                KeyCode::Char('e') => self.page = Page::Project(self.project.clone()),
                KeyCode::Char('r') => return Some(Request::Projects),
                _ => {}
            }
            return None;
        }
        if matches!(self.page, Page::AllPending) {
            match key.code {
                KeyCode::Esc => {
                    self.page = Page::List;
                    self.scroll = 0;
                }
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
                KeyCode::Char('r') => return Some(Request::AllPending),
                _ => {}
            }
            return None;
        }
        if let Page::Delete { pending, index } = &self.page {
            match key.code {
                KeyCode::Esc if !self.busy => {
                    self.page = Page::List;
                    self.scroll = 0;
                }
                KeyCode::Char('y' | 'Y') if !self.busy && self.read_error.is_none() => {
                    let operation = Operation::Delete {
                        pending: pending.clone(),
                        index: *index,
                    };
                    self.busy = true;
                    self.message_failed = false;
                    self.message = "Deleting task…".into();
                    return Some(Request::Run(operation));
                }
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
                _ => {}
            }
            return None;
        }
        if let Page::Project(path) = &mut self.page {
            match key.code {
                KeyCode::Esc => self.page = Page::List,
                KeyCode::Enter => {
                    if !path.trim().is_empty() {
                        return Some(Request::Project(path.clone()));
                    }
                }
                KeyCode::Backspace => {
                    path.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => path.clear(),
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    path.push(c)
                }
                _ => {}
            }
            return None;
        }
        if let Page::Add {
            title,
            body,
            body_focus,
        }
        | Page::Edit {
            title,
            body,
            body_focus,
            ..
        } = &mut self.page
        {
            if self.busy {
                return None;
            }
            match key.code {
                KeyCode::Esc => self.finish_edit(),
                KeyCode::Tab | KeyCode::BackTab => *body_focus = !*body_focus,
                KeyCode::Enter if !*body_focus => *body_focus = true,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.read_error.is_some() {
                        return None;
                    }
                    if title.text.trim().is_empty() {
                        self.message_failed = true;
                        self.message = "Title is required".into();
                        return None;
                    }
                    self.busy = true;
                    self.message_failed = false;
                    let title = title.text.clone();
                    let body = body.text.clone();
                    let operation = if let Page::Edit { pending, index, .. } = &self.page {
                        self.message = "Saving task…".into();
                        Operation::Edit {
                            pending: pending.clone(),
                            index: *index,
                            title,
                            body,
                        }
                    } else {
                        self.message = "Adding task…".into();
                        Operation::Add { title, body }
                    };
                    return Some(Request::Run(operation));
                }
                KeyCode::Char(_)
                    if key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {}
                code => {
                    let field = if *body_focus { body } else { title };
                    field.key(code, *body_focus);
                }
            }
            return None;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        if matches!(self.page, Page::List) {
            match key.code {
                KeyCode::Null => {
                    self.view = View::Links;
                    return None;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    self.view = match (self.view, key.code == KeyCode::BackTab) {
                        (View::Text, false) | (View::Links, true) => View::Details,
                        (View::Details, false) | (View::Text, true) => View::Links,
                        _ => View::Text,
                    };
                    return None;
                }
                _ => {}
            }
            if self.view == View::Links {
                match key.code {
                    KeyCode::Esc if self.links.reading.is_some() => self.links.back(),
                    KeyCode::Up | KeyCode::Char('k') => self.links.scroll(-1),
                    KeyCode::Down | KeyCode::Char('j') => self.links.scroll(1),
                    KeyCode::PageUp => self.links.scroll(-self.content_page()),
                    KeyCode::PageDown => self.links.scroll(self.content_page()),
                    KeyCode::Enter => self.links.open(),
                    KeyCode::Char('t' | 'c') => {}
                    _ if self.links.reading.is_some() => return None,
                    _ => {}
                }
                if matches!(
                    key.code,
                    KeyCode::Esc
                        | KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Char('j' | 'k')
                        | KeyCode::PageUp
                        | KeyCode::PageDown
                        | KeyCode::Enter
                ) {
                    return None;
                }
            }
        }
        match key.code {
            KeyCode::Esc => {
                self.message.clear();
                self.page = Page::List;
                self.scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if matches!(self.page, Page::List) && self.read_error.is_none() {
                    self.select(self.selected.saturating_sub(1));
                } else {
                    self.scroll = self.scroll.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if matches!(self.page, Page::List) && self.read_error.is_none() {
                    self.select(self.selected + 1);
                } else {
                    self.scroll = self.scroll.saturating_add(1);
                }
            }
            KeyCode::PageUp | KeyCode::PageDown
                if matches!(self.page, Page::List) && self.read_error.is_none() =>
            {
                let page = self.content_page();
                self.scroll_content(if key.code == KeyCode::PageUp {
                    -page
                } else {
                    page
                });
            }
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
            KeyCode::Enter if matches!(self.page, Page::List) => self.view = View::Details,
            KeyCode::Char('t') if matches!(self.page, Page::List) => self.view = View::Text,
            KeyCode::Char('?' | 'h') => {
                self.page = Page::Help;
                self.scroll = 0;
            }
            KeyCode::Char('A') if matches!(self.page, Page::List) && !self.busy => {
                self.page = Page::AllPending;
                self.scroll = 0;
                self.message.clear();
                return Some(Request::AllPending);
            }
            KeyCode::Char('c') if !self.busy => {
                self.page = Page::Projects;
                self.scroll = 0;
                return Some(Request::Projects);
            }
            KeyCode::Char('a')
                if !self.busy && self.read_error.is_none() && self.snapshot.is_some() =>
            {
                self.page = Page::Add {
                    title: Input::new(String::new()),
                    body: Input::new(String::new()),
                    body_focus: false,
                };
                self.message.clear();
            }
            KeyCode::Char('e')
                if matches!(self.page, Page::List) && !self.busy && self.read_error.is_none() =>
            {
                self.edit();
            }
            KeyCode::Char('x')
                if matches!(self.page, Page::List) && !self.busy && self.read_error.is_none() =>
            {
                if let Some(index) = self.pending_index() {
                    self.page = Page::Delete {
                        pending: self.snapshot.as_ref().unwrap().pending.clone(),
                        index,
                    };
                    self.scroll = 0;
                    self.message.clear();
                }
            }
            KeyCode::Char('r') => return Some(Request::Refresh),
            KeyCode::Char(c @ ('u' | 'd'))
                if matches!(self.page, Page::List) && !self.busy && self.read_error.is_none() =>
            {
                let index = self.pending_index()?;
                let pending = &self.snapshot.as_ref().unwrap().pending;
                let to = if c == 'u' {
                    index.checked_sub(1)?
                } else {
                    index + 1
                };
                if to >= pending.len() {
                    return None;
                }
                self.busy = true;
                self.message_failed = false;
                self.message = "Moving task…".into();
                return Some(Request::Run(Operation::Move {
                    pending: pending.clone(),
                    index,
                    to,
                }));
            }
            KeyCode::Char('R') if !self.busy && matches!(self.page, Page::List) => {
                if self.read_error.is_some() {
                    return None;
                }
                let Some(snapshot) = &self.snapshot else {
                    self.message_failed = true;
                    self.message = "Waiting for queue data; action not sent".into();
                    return None;
                };
                let operation = Operation::RecordDefault(!snapshot.record_default);
                self.busy = true;
                self.message_failed = false;
                self.message = "Saving the recording default…".into();
                return Some(Request::Run(operation));
            }
            KeyCode::Char('p') if !self.busy && matches!(self.page, Page::List) => {
                if self.read_error.is_some() {
                    return None;
                }
                let Some(snapshot) = &self.snapshot else {
                    self.message_failed = true;
                    self.message = "Waiting for queue data; action not sent".into();
                    return None;
                };
                let operation = Operation::Pause(!snapshot.paused);
                self.busy = true;
                self.message_failed = false;
                self.message = "Running action…".into();
                return Some(Request::Run(operation));
            }
            _ => {}
        }
        None
    }
}

impl Panel {
    fn message_color(&self, t: &Theme) -> ratatui::style::Color {
        if self.busy {
            t.agent_working
        } else if self.message_failed {
            t.agent_error
        } else {
            t.agent_idle
        }
    }
    /// An action result: the headline carries its outcome color; the details stay plain, so a
    /// failure report with recorded parts does not read as all error.
    fn result_lines(&self, t: &Theme, text: &str, width: u16) -> Vec<ratatui::text::Line<'static>> {
        use ratatui::style::{Modifier, Style};
        let (head, rest) = text.split_once('\n').unwrap_or((text, ""));
        let mut lines: Vec<_> = wrap_text(head, width)
            .into_iter()
            .map(|l| {
                l.style(
                    Style::default()
                        .fg(self.message_color(t))
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();
        if text.contains('\n') {
            lines.extend(
                wrap_text(rest, width)
                    .into_iter()
                    .map(|l| l.style(Style::default().fg(t.text))),
            );
        }
        lines
    }
    /// Whether the selected Pending task's dispatch will be recorded: its own choice, or the
    /// project default.
    fn recording(&self) -> bool {
        self.record
            .unwrap_or_else(|| self.snapshot.as_ref().is_some_and(|s| s.record_default))
    }
    /// A sub-page has replaced the list and content inside the popup.
    pub fn overlay_open(&self) -> bool {
        !matches!(self.page, Page::List)
    }
    fn mode_line(&self, t: &Theme) -> ratatui::text::Line<'static> {
        use ratatui::{
            style::{Modifier, Style},
            text::{Line, Span},
        };
        let emphasis = |color| Style::default().fg(color).add_modifier(Modifier::BOLD);
        if self.read_error.is_some() {
            return Line::styled("Read failed", emphasis(t.agent_error));
        }
        let Some(s) = &self.snapshot else {
            return Line::styled("Loading tasks…", emphasis(t.agent_starting));
        };
        let (state, color) = if s.paused {
            ("Paused", t.agent_blocked)
        } else if s.current.is_some() {
            ("Running", t.agent_working)
        } else if s.awaiting.is_some() {
            ("Awaiting", t.agent_blocked)
        } else if !s.pending.is_empty() {
            ("Ready", t.agent_idle)
        } else {
            ("Idle", t.agent_idle)
        };
        Line::from(vec![
            Span::styled("Queue: ", Style::default().fg(t.muted)),
            Span::styled(state, emphasis(color)),
        ])
    }
    /// Draws the Tasks popup into `area`: project and queue actions on top, then either the
    /// list beside the selected task with task actions below, or the open sub-page.
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
    ) -> Vec<(u16, usize)> {
        use crate::{
            buttons::{self, Button as B},
            ui,
        };
        use KeyCode as K;
        use ratatui::{
            layout::Rect,
            style::Style,
            text::Line,
            widgets::{Clear, Paragraph},
        };
        use unicode_width::UnicodeWidthStr;
        self.buttons.clear();
        self.fields.clear();
        self.project_rows.clear();
        self.links.rows.clear();
        self.list_area = Rect::default();
        self.content_area = Rect::default();
        if area.is_empty() {
            return Vec::new();
        }
        self.sync_content();
        frame.render_widget(Clear, area);
        // Saddle supplies the outer frame and title. Keep content padding,
        // while reserving borders for fields, buttons and internal sections.
        frame.buffer_mut().set_style(area, t.base().bg(t.overlay));
        if matches!(self.page, Page::Projects) {
            let height = (self.projects.len().min(14) as u16 + 12).max(16);
            let body = ui::dialog(t, frame, area, "Projects", 104, height);
            let (mut body, hits) = buttons::draw_compact(t, frame, body, &self.controls());
            self.buttons = hits;
            if let Some(source) = self.source.as_deref().filter(|_| body.height > 1) {
                frame.render_widget(
                    Paragraph::new(source).style(Style::default().fg(t.muted)),
                    Rect::new(body.x, body.y, body.width, 1),
                );
                body.y += 1;
                body.height -= 1;
            }
            if !body.is_empty() {
                let name_width = (usize::from(body.width) / 3).clamp(12, 32);
                frame.render_widget(
                    Paragraph::new(format!(
                        "  {} Status / receiver",
                        ui::pad("Project", name_width)
                    ))
                    .style(Style::default().fg(t.muted)),
                    Rect::new(body.x, body.y, body.width, 1),
                );
                body.y += 1;
                body.height -= 1;
            }
            self.draw_page(t, frame, body, true, false);
            return Vec::new();
        }
        if !matches!(self.page, Page::List) {
            let (title, width, height) = match &self.page {
                Page::Add { .. } => ("Add task", 104, 32),
                Page::Edit { .. } => ("Edit task", 104, 32),
                Page::Confirm(c) => (c.action.label(), 96, 24),
                Page::Delete { .. } => ("Delete task", 96, 24),
                Page::Project(_) => ("Project path", 96, 12),
                Page::Help => ("Tasks help", 96, 30),
                Page::Feedback(_) => ("Action result", 96, 24),
                Page::AllPending => ("All pending", 104, 32),
                _ => unreachable!(),
            };
            let body = ui::dialog(t, frame, area, title, width, height);
            let (mut body, hits) = buttons::draw_compact(t, frame, body, &self.controls());
            self.buttons = hits;
            if !body.is_empty() {
                frame.render_widget(
                    Paragraph::new(match (&self.source, self.busy) {
                        (Some(source), true) => format!("Running action… · {source}"),
                        (Some(source), false) => source.clone(),
                        (None, true) => "Running action…".into(),
                        (None, false) if matches!(self.page, Page::AllPending) => {
                            "All registered projects".into()
                        }
                        (None, false) => ui::clip(&self.project, body.width as usize),
                    })
                    .style(Style::default().fg(if self.busy {
                        t.agent_working
                    } else {
                        t.muted
                    })),
                    Rect::new(body.x, body.y, body.width, 1),
                );
                body.y += 1;
                body.height -= 1;
            }
            if !self.message.is_empty()
                && !matches!(&self.page, Page::Feedback(text) if text == &self.message)
                && body.height > 3
            {
                let lines = self.result_lines(t, &self.message, body.width);
                let height = (lines.len() as u16).min(body.height / 3).max(1);
                frame.render_widget(
                    Paragraph::new(lines),
                    Rect::new(body.x, body.bottom() - height, body.width, height),
                );
                body.height -= height;
            }
            if !matches!(
                self.page,
                Page::Add { .. } | Page::Edit { .. } | Page::Project(_)
            ) && body.height > 1
            {
                frame.render_widget(
                    Paragraph::new("PgUp / PgDn Scroll").style(Style::default().fg(t.muted)),
                    Rect::new(body.x, body.bottom() - 1, body.width, 1),
                );
                body.height -= 1;
            }
            self.draw_page(t, frame, body, true, false);
            return Vec::new();
        }
        let inside = area.inner(ratatui::layout::Margin::new(1, 0));
        if inside.height < 4 || inside.width < 12 {
            return Vec::new();
        }
        let on_list = matches!(self.page, Page::List);
        let ready = on_list
            && !self.reading_link()
            && !self.busy
            && self.snapshot.is_some()
            && self.read_error.is_none();
        // Keep the compact toolbars when outlines would crowd out the task content.
        let outlined = inside.width >= 76 && inside.height >= 28;
        let draw_toolbar = if outlined {
            buttons::draw_outlined_top
        } else {
            buttons::draw_compact_top
        };
        let row_height = if outlined { 3 } else { 1 };
        let padding = if outlined { 4 } else { 2 };
        let text_y = inside.y + row_height / 2;
        // Project row: picker and directory on the left, queue state on the right.
        let name = std::path::Path::new(&self.project)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let project = format!("{} ▾ c", ui::clip(&name, 24));
        let mode = self.mode_line(t);
        let mode_width = (mode.width() as u16).min(inside.width);
        frame.render_widget(
            Paragraph::new(mode).right_aligned(),
            Rect::new(inside.right() - mode_width, text_y, mode_width, 1),
        );
        let label = Rect::new(inside.x, text_y, 8.min(inside.width), 1);
        frame.render_widget(
            Paragraph::new("Project").style(Style::default().fg(t.muted)),
            label,
        );
        let picker_width = (project.width() as u16 + padding).min(inside.width - label.width);
        let (_, hits) = draw_toolbar(
            t,
            frame,
            Rect::new(label.right(), inside.y, picker_width, row_height),
            &[B::new(&project, K::Char('c'), !self.busy && on_list)],
        );
        self.buttons.extend(hits);
        let path_x = label.right() + picker_width + 1;
        let path_end = inside.right().saturating_sub(mode_width + 2);
        if path_end > path_x {
            frame.render_widget(
                Paragraph::new(ui::clip(&self.project, usize::from(path_end - path_x)))
                    .style(Style::default().fg(t.dim)),
                Rect::new(path_x, text_y, path_end - path_x, 1),
            );
        }
        // Queue actions for the whole project; Refresh sits apart on the right.
        let refresh = "Refresh r";
        let refresh_width = refresh.width() as u16 + padding;
        let actions = Rect::new(
            inside.x,
            inside.y + row_height,
            inside.width.saturating_sub(refresh_width + 1),
            inside.height - row_height,
        );
        let (_, hits) = draw_toolbar(
            t,
            frame,
            Rect::new(
                inside.right().saturating_sub(refresh_width),
                actions.y,
                refresh_width.min(inside.width),
                row_height,
            ),
            &[B::new(refresh, K::Char('r'), !self.busy && on_list)],
        );
        self.buttons.extend(hits);
        let (below, hits) = draw_toolbar(
            t,
            frame,
            actions,
            &[
                B::new(
                    if self.snapshot.as_ref().is_some_and(|s| s.paused) {
                        "Resume p"
                    } else {
                        "Pause p"
                    },
                    K::Char('p'),
                    ready,
                ),
                B::new(
                    if self.snapshot.as_ref().is_some_and(|s| s.record_default) {
                        "Record default: On R"
                    } else {
                        "Record default: Off R"
                    },
                    K::Char('R'),
                    ready,
                ),
            ],
        );
        self.buttons.extend(hits);
        if below.height < 2 {
            return Vec::new();
        }
        let rule = Rect::new(inside.x, below.y, inside.width, 1);
        let body = Rect::new(inside.x, below.y + 1, inside.width, below.height - 1);
        let heading = match self.page {
            Page::List => "",
            Page::Help => " Help ",
            Page::Feedback(_) => " Action result ",
            Page::Projects => " Projects ",
            Page::Project(_) => " Project path ",
            Page::AllPending => " All pending ",
            Page::Add { .. } => " Add task ",
            Page::Edit { .. } => " Edit task ",
            Page::Delete { .. } => " Delete task ",
            Page::Confirm(ref confirmation) => confirmation.action.label(),
        };
        let mut line = vec![ratatui::text::Span::styled(
            "─".repeat(usize::from(rule.width)),
            Style::default().fg(t.border),
        )];
        if self.busy {
            line.insert(
                0,
                ratatui::text::Span::styled(
                    "─ Running action… ",
                    Style::default().fg(t.agent_working),
                ),
            );
        } else if !heading.is_empty() {
            line.insert(
                0,
                ratatui::text::Span::styled(format!("─{heading}"), Style::default().fg(t.bright)),
            );
        }
        if let Some(source) = self.source.as_deref().filter(|_| heading.is_empty()) {
            line.insert(
                usize::from(self.busy),
                ratatui::text::Span::styled(format!("─ {source} "), Style::default().fg(t.muted)),
            );
        }
        frame.render_widget(Paragraph::new(Line::from(line)), rule);
        // Task actions below the list and content; Close sits apart on the right.
        let close = if self.reading_link() {
            "Back Esc"
        } else {
            "Close Esc"
        };
        let close_width = close.width() as u16 + 2;
        let (_, hits) = buttons::draw_compact(
            t,
            frame,
            Rect::new(
                body.right().saturating_sub(close_width),
                body.bottom() - 1,
                close_width.min(body.width),
                1,
            ),
            &[B::new(close, K::Esc, true)],
        );
        self.buttons.extend(hits);
        // Queue-wide actions, then the selected task's, then other views; same order as before,
        // with a divider between groups.
        let queue_controls = vec![
            B::new("Add task a", K::Char('a'), ready),
            B::new("Notifications N", K::Char('N'), !self.busy),
        ];
        let mut controls = Vec::new();
        if self
            .tasks()
            .get(self.selected)
            .is_some_and(|(_, task)| task.status.as_deref() == Some("failed"))
        {
            controls.push(B::new("Mark seen m", K::Char('m'), !self.busy));
        }
        let active = self
            .tasks()
            .get(self.selected)
            .is_some_and(|(group, task)| {
                matches!(*group, "Current" | "Awaiting")
                    && task.id.is_some()
                    && task.run_id.is_some()
            });
        if let Some(index) = self.pending_index() {
            controls.extend([
                B::new("Edit e", K::Char('e'), ready),
                B::new("Move up u", K::Char('u'), ready && index > 0),
                B::new(
                    "Move down d",
                    K::Char('d'),
                    ready && index + 1 < self.snapshot.as_ref().unwrap().pending.len(),
                ),
                B::new("Delete x", K::Char('x'), ready).danger(),
            ]);
            let dispatchable = ready && self.dispatch_target().is_some();
            let mut button = B::new(
                if self.recording() {
                    "[x] Record"
                } else {
                    "[ ] Record"
                },
                K::Null,
                dispatchable,
            );
            button.key = record_click();
            controls.push(button);
            let mut button = B::new("Dispatch selected", K::Null, dispatchable).primary();
            button.key = dispatch_selected_click();
            controls.push(button);
        }
        if active {
            let label = if self.tasks()[self.selected].0 == "Current" {
                "Submit for review"
            } else {
                "Accept"
            };
            let mut button = B::new(label, K::Null, ready).primary();
            button.key = transition_click();
            controls.push(button);
            let mut button = B::new("Return to pending…", K::Null, ready);
            button.key = return_click();
            controls.push(button);
        }
        let view_controls = vec![
            B::new("All pending A", K::Char('A'), !self.busy),
            B::new("Help ?", K::Char('?'), true),
        ];
        let (middle, hits) = draw_groups(
            t,
            frame,
            Rect {
                width: body.width.saturating_sub(close_width + 1),
                ..body
            },
            &[queue_controls, controls, view_controls],
        );
        self.buttons.extend(hits);
        if middle.height < 2 {
            return Vec::new();
        }
        // The buttons left Close its column; the list and content use the full width.
        let middle = Rect {
            height: middle.height - 1,
            width: body.width,
            ..middle
        };
        let border = Style::default().fg(t.border);
        frame.render_widget(
            Paragraph::new("─".repeat(usize::from(body.width))).style(border),
            Rect::new(body.x, middle.bottom(), body.width, 1),
        );
        // Side by side when there is room; otherwise the list sits above the content.
        let (list, content) = if middle.width >= 60 {
            let left = middle.width / 3;
            let divider = middle.x + left;
            for y in middle.y..middle.bottom() {
                frame.render_widget(
                    Paragraph::new("│").style(border),
                    Rect::new(divider, y, 1, 1),
                );
            }
            if heading.is_empty() && !self.busy && self.source.is_none() {
                frame.render_widget(
                    Paragraph::new("┬").style(border),
                    Rect::new(divider, rule.y, 1, 1),
                );
            }
            frame.render_widget(
                Paragraph::new("┴").style(border),
                Rect::new(divider, middle.bottom(), 1, 1),
            );
            (
                Rect {
                    width: left,
                    ..middle
                },
                Rect::new(
                    divider + 2,
                    middle.y,
                    middle.right().saturating_sub(divider + 2),
                    middle.height,
                ),
            )
        } else {
            let top = (middle.height * 2 / 5).max(1);
            frame.render_widget(
                Paragraph::new("─".repeat(usize::from(middle.width))).style(border),
                Rect::new(middle.x, middle.y + top, middle.width, 1),
            );
            (
                Rect {
                    height: top,
                    ..middle
                },
                Rect::new(
                    middle.x,
                    middle.y + top + 1,
                    middle.width,
                    middle.height.saturating_sub(top + 1),
                ),
            )
        };
        let rows = self.draw_page(t, frame, list, true, true);
        self.draw_content(t, frame, content, outlined);
        rows
    }
    /// The selected task beside the list, as its text or its run details.
    fn draw_content(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        _outlined: bool,
    ) {
        use KeyCode as K;
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::{Line, Span},
            widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
        };
        use unicode_width::UnicodeWidthStr;
        if area.is_empty() {
            return;
        }
        let shown = self.read_error.is_none() && self.content.is_some();
        let numbered = self.content.as_ref().is_some_and(|c| c.task.id.is_some());
        let mut tabs = vec![
            (
                "Task text t",
                KeyEvent::from(K::Char('t')),
                Some(View::Text),
            ),
            ("Run details", KeyEvent::from(K::Enter), Some(View::Details)),
            ("Links", KeyEvent::from(K::Null), Some(View::Links)),
        ];
        if numbered {
            tabs.push(("Telemetry ↗", telemetry_click(), None));
        }
        let (mut x, mut y) = (area.x, area.y);
        for (label, key, view) in tabs {
            let width = (label.width() as u16 + 2).min(area.width);
            if x > area.x && x + width > area.right() {
                x = area.x;
                y += 1;
            }
            if view.is_none() {
                x = area.right().saturating_sub(width);
            }
            if y >= area.bottom() {
                break;
            }
            let rect = Rect::new(x, y, width, 1);
            let selected = shown && view == Some(self.view);
            let style = if !shown {
                Style::default().fg(t.dim)
            } else if selected {
                Style::default()
                    .fg(t.focus)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(if view.is_none() { t.text } else { t.muted })
            };
            frame.render_widget(Paragraph::new(format!(" {label} ")).style(style), rect);
            if shown {
                self.buttons.push(crate::buttons::Hit {
                    area: rect,
                    danger: false,
                    key,
                });
            }
            x += width + 1;
        }
        let top = (y + 2).min(area.bottom());
        let body = Rect::new(area.x, top, area.width, area.bottom() - top);
        self.content_area = body;
        if !shown {
            frame.render_widget(
                Paragraph::new(if self.read_error.is_some() {
                    "Queue data unavailable"
                } else if self.snapshot.is_some() {
                    "No task selected"
                } else {
                    "Loading tasks…"
                })
                .style(Style::default().fg(t.muted)),
                body,
            );
            return;
        }
        if self.view == View::Links {
            self.draw_links(t, frame, body);
            return;
        }
        self.links.rows.clear();
        let width = body.width.saturating_sub(1);
        let height = usize::from(body.height);
        let queried = self.detail_key().is_some();
        let detail = self.content.as_ref().unwrap();
        let live = self.live(detail);
        let (lines, top) = match self.view {
            View::Links => unreachable!(),
            View::Details => {
                let lines = detail.lines(t, live, queried, usize::from(width));
                let max = lines.len().saturating_sub(height);
                let detail = self.content.as_mut().unwrap();
                detail.view = (height, max);
                (lines, detail.scroll.min(max))
            }
            View::Text => {
                let task = live.map(|(_, task)| task).unwrap_or(&detail.task);
                let (group, status) = live
                    .map(|(group, task)| (group, task.status.as_deref()))
                    .unwrap_or((detail.group, detail.task.status.as_deref()));
                let (status, status_color) = task_status(t, group, status);
                let title_style = Style::default().fg(t.bright).add_modifier(Modifier::BOLD);
                let mut lines: Vec<Line<'static>> = wrap_text(
                    &format!(
                        "{status} {} {}",
                        task.id.as_deref().unwrap_or("·"),
                        task.title
                    ),
                    width,
                )
                .into_iter()
                .map(|line| line.style(title_style))
                .collect();
                // The status label leads the first row in the task's own status color.
                if let Some(first) = lines.first_mut() {
                    let text = first.to_string();
                    if let Some(rest) = text.strip_prefix(status) {
                        *first = Line::from(vec![
                            Span::styled(status.to_owned(), title_style.fg(status_color)),
                            Span::styled(rest.to_owned(), title_style),
                        ]);
                    }
                }
                lines.push(Line::raw(""));
                if task.body.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No body",
                        Style::default().fg(t.muted),
                    )));
                } else {
                    lines.extend(wrap_text(&task.body, width));
                }
                self.text_scroll = self.text_scroll.min(lines.len().saturating_sub(height));
                (lines, self.text_scroll)
            }
        };
        let visible: Vec<_> = lines.iter().skip(top).take(height).cloned().collect();
        frame.render_widget(
            Paragraph::new(visible),
            Rect::new(body.x, body.y, width, body.height),
        );
        if lines.len() > height && height > 0 {
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .thumb_style(Style::default().fg(t.muted))
                    .track_style(Style::default().fg(t.border)),
                body,
                &mut ScrollbarState::new(lines.len() - height + 1)
                    .viewport_content_length(height)
                    .position(top),
            );
        }
    }
    fn draw_page(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        mut body: ratatui::layout::Rect,
        focused: bool,
        list: bool,
    ) -> Vec<(u16, usize)> {
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Paragraph, Wrap},
        };
        let mut hits = Vec::new();
        let page = if list { &Page::List } else { &self.page };
        match page {
            Page::List => {
                self.list_area = body;
                if let Some(error) = &self.read_error {
                    let text = format!(
                        "{error}\n\nCheck plugin arguments or choose Project.\nScroll to read the full error.\n\nDirectory: {}",
                        self.project
                    );
                    let lines = wrap_text(&text, body.width);
                    self.scroll = self
                        .scroll
                        .min(lines.len().saturating_sub(usize::from(body.height)));
                    frame.render_widget(
                        Paragraph::new(lines)
                            .scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                        body,
                    );
                    return hits;
                }
                let tasks = self.tasks();
                if tasks.is_empty() {
                    let muted = Style::default().fg(t.muted);
                    let lines = if self.snapshot.is_some() {
                        vec![
                            Line::styled("No active tasks · Add task a", muted),
                            Line::raw(""),
                            Line::styled("History 0", muted.add_modifier(Modifier::BOLD)),
                            Line::styled("No history yet", muted),
                        ]
                    } else {
                        vec![Line::styled("Loading tasks…", muted)]
                    };
                    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
                    return hits;
                }
                let text_width = body.width.saturating_sub(1);
                let mut rows = Vec::new();
                if let Some(s) = &self.snapshot
                    && s.current.is_none()
                    && s.awaiting.is_none()
                    && s.pending.is_empty()
                {
                    rows.push((
                        None,
                        Line::styled("No active tasks · Add task a", Style::default().fg(t.muted)),
                    ));
                    rows.push((None, Line::raw("")));
                }
                let mut section = "";
                for (index, (group, task)) in tasks.iter().enumerate() {
                    if section != *group {
                        // History mixes outcomes, so only its rows carry status colors.
                        let color = match *group {
                            "Current" => t.agent_working,
                            "Awaiting" => t.agent_blocked,
                            "Pending" => t.agent_starting,
                            _ => t.muted,
                        };
                        rows.push((
                            None,
                            Line::styled(
                                format!(
                                    "{group} {}",
                                    tasks.iter().filter(|(g, _)| g == group).count()
                                ),
                                Style::default().fg(color).add_modifier(Modifier::BOLD),
                            ),
                        ));
                        section = group;
                    }
                    let style = if index == self.selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let (status, color) = task_status(t, group, task.status.as_deref());
                    let status_width = unicode_width::UnicodeWidthStr::width(status) + 1;
                    let id = task.id.as_deref().unwrap_or("·");
                    let id_width = unicode_width::UnicodeWidthStr::width(id).min(8);
                    let title_width =
                        usize::from(text_width).saturating_sub(id_width + status_width + 3);
                    let spans = vec![
                        Span::styled(
                            if index == self.selected { "▎" } else { " " },
                            Style::default().fg(if focused { t.focus } else { t.muted }),
                        ),
                        Span::styled(crate::ui::clip(id, id_width), Style::default().fg(t.muted)),
                        Span::raw(" "),
                        Span::raw(crate::ui::pad(
                            &crate::ui::clip(&task.title, title_width),
                            title_width,
                        )),
                        Span::styled(
                            format!(" {status}"),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                    ];
                    rows.push((Some(index), Line::from(spans).style(style)));
                }
                let history_start = tasks.iter().position(|(group, _)| *group == "History");
                let history_total = tasks.len() - history_start.unwrap_or(tasks.len());
                let history_footer = history_start.filter(|_| body.height > 1).map(|_| {
                    body.height -= 1;
                    Rect::new(body.x, body.bottom(), body.width, 1)
                });
                let selected_row = rows
                    .iter()
                    .position(|(i, _)| *i == Some(self.selected))
                    .unwrap_or(0);
                let height = usize::from(body.height);
                if !self.manual_scroll {
                    self.top = self
                        .top
                        .max((selected_row + 1).saturating_sub(height))
                        .min(selected_row);
                }
                self.top = self.top.min(rows.len().saturating_sub(height));
                if let Some(footer) = history_footer {
                    let start = history_start.unwrap();
                    let total = history_total;
                    let visible: Vec<_> = rows
                        .iter()
                        .skip(self.top)
                        .take(height)
                        .filter_map(|(index, _)| {
                            index.filter(|i| *i >= start).map(|i| i - start + 1)
                        })
                        .collect();
                    let label = match (visible.first(), visible.last()) {
                        (Some(first), Some(last)) => format!(
                            " History {first}–{last}/{total}{} ",
                            if *last == total { " · End" } else { "" }
                        ),
                        _ => format!(" History {total} below "),
                    };
                    frame.render_widget(
                        Block::new()
                            .borders(ratatui::widgets::Borders::TOP)
                            .border_style(Style::default().fg(t.border))
                            .title(Line::styled(label, Style::default().fg(t.muted))),
                        footer,
                    );
                }
                for (i, (index, line)) in rows.iter().skip(self.top).take(height).enumerate() {
                    let y = body.y + i as u16;
                    frame.render_widget(
                        Paragraph::new(line.clone()).style(line.style),
                        Rect::new(body.x, y, text_width, 1),
                    );
                    if let Some(index) = index {
                        hits.push((y, *index));
                    }
                }
                if rows.len() > height && height > 0 {
                    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
                    frame.render_stateful_widget(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(None)
                            .end_symbol(None)
                            .thumb_style(Style::default().fg(t.muted))
                            .track_style(Style::default().fg(t.border)),
                        body,
                        &mut ScrollbarState::new(rows.len().saturating_sub(height) + 1)
                            .viewport_content_length(height)
                            .position(self.top),
                    );
                }
            }
            Page::Projects => {
                let mut lines = Vec::new();
                if let Some(error) = &self.registry_error {
                    lines.extend(
                        wrap_text(error, body.width)
                            .into_iter()
                            .map(|l| (None, l.to_string())),
                    );
                }
                for (index, path) in self.projects.iter().enumerate() {
                    let name = std::path::Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let name_width = (usize::from(body.width) / 3).clamp(12, 32);
                    let name = crate::ui::clip(&name, name_width);
                    let label = format!(
                        "{} {} {}",
                        if index == self.project_selected {
                            "›"
                        } else {
                            " "
                        },
                        crate::ui::pad(&name, name_width),
                        crate::ui::clip(
                            &crate::project_setup::safe(
                                self.project_status
                                    .get(path)
                                    .map(String::as_str)
                                    .unwrap_or("Checking…")
                            ),
                            usize::from(body.width).saturating_sub(name_width + 3)
                        )
                    );
                    lines.push((Some(index), label));
                }
                if self.projects.is_empty() {
                    lines.push((None, "No projects added · Add project a".into()));
                }
                let footer_height = body.height.min(5);
                let height = usize::from(body.height - footer_height);
                let selected = lines
                    .iter()
                    .position(|(i, _)| *i == Some(self.project_selected))
                    .unwrap_or(0);
                // Registry error rows lead the list.
                let errors = self
                    .registry_error
                    .as_ref()
                    .map_or(0, |error| wrap_text(error, body.width).len());
                let top = selected.saturating_sub(height.saturating_sub(1));
                for (offset, (index, line)) in lines.iter().skip(top).take(height).enumerate() {
                    let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
                    frame.render_widget(
                        Paragraph::new(line.as_str()).style(
                            if *index == Some(self.project_selected) {
                                Style::default().bg(t.selected).add_modifier(Modifier::BOLD)
                            } else if index.is_none() {
                                Style::default().fg(if top + offset < errors {
                                    t.agent_error
                                } else {
                                    t.muted
                                })
                            } else {
                                Style::default()
                            },
                        ),
                        row,
                    );
                    if let Some(index) = index {
                        self.project_rows.push((row, *index));
                    }
                }
                let path = self
                    .projects
                    .get(self.project_selected)
                    .map(String::as_str)
                    .unwrap_or(&self.project);
                frame.render_widget(
                    Paragraph::new(crate::project_setup::safe(&format!(
                        "{}\n{}",
                        path,
                        self.project_status
                            .get(path)
                            .map(String::as_str)
                            .unwrap_or("Checking project…")
                    )))
                    .wrap(Wrap { trim: false })
                    .style(Style::default().fg(t.muted)),
                    Rect::new(
                        body.x,
                        body.bottom() - footer_height,
                        body.width,
                        footer_height,
                    ),
                );
            }
            Page::Project(path) => {
                let field = t.block(" Directory ", focused);
                let field_area = Rect::new(body.x, body.y, body.width, body.height.min(3));
                let inner = field.inner(field_area);
                frame.render_widget(field, field_area);
                let width = unicode_width::UnicodeWidthStr::width(path.as_str()) as u16;
                frame.render_widget(
                    Paragraph::new(path.as_str())
                        .scroll((0, width.saturating_sub(inner.width.saturating_sub(1)))),
                    inner,
                );
                if focused && !inner.is_empty() {
                    frame.set_cursor_position((inner.x + width.min(inner.width - 1), inner.y));
                }
                if body.height > 3 {
                    frame.render_widget(
                        Paragraph::new(
                            "Type a registered project directory. Ctrl-U clears.\nApplies to this session only.",
                        )
                        .style(Style::default().fg(t.muted))
                        .wrap(Wrap { trim: false }),
                        Rect::new(body.x, body.y + 3, body.width, body.height - 3),
                    );
                }
            }
            Page::Add { .. } | Page::Edit { .. } => {
                let (Page::Add {
                    title,
                    body: text,
                    body_focus,
                }
                | Page::Edit {
                    title,
                    body: text,
                    body_focus,
                    ..
                }) = &mut self.page
                else {
                    return hits;
                };
                if body.height < 5 {
                    frame.render_widget(Paragraph::new("Enlarge the window to edit a task"), body);
                    return hits;
                }
                // Field titles name the field; the key hint for switching fields sits below, only
                // when the Body field still keeps a text row (title 3 + body borders 2 + hint 1).
                let hint = u16::from(body.height >= 7);
                let title_area = Rect::new(body.x, body.y, body.width, 3);
                let text_area = Rect::new(body.x, body.y + 3, body.width, body.height - 3 - hint);
                self.fields = vec![(title_area, false), (text_area, true)];
                let title_block = t.block(" Title ", !*body_focus);
                let text_block = t.block(" Body ", *body_focus);
                if hint > 0 {
                    frame.render_widget(
                        Paragraph::new("Tab Switch field · Ctrl-S Save")
                            .style(Style::default().fg(t.muted)),
                        Rect::new(body.x, text_area.bottom(), body.width, 1),
                    );
                }
                let title_inner = title_block.inner(title_area);
                let text_inner = text_block.inner(text_area);
                frame.render_widget(title_block, title_area);
                frame.render_widget(text_block, text_area);
                let typing = focused && !self.busy;
                title.draw(frame, title_inner, typing && !*body_focus, "", t);
                text.draw(frame, text_inner, typing && *body_focus, "", t);
            }
            Page::Confirm(_) => self.draw_confirmation(t, frame, body, focused),
            Page::AllPending => {
                let lines = self.all_pending_lines(t, body.width.saturating_sub(1));
                let height = usize::from(body.height);
                self.scroll = self.scroll.min(lines.len().saturating_sub(height));
                frame.render_widget(
                    Paragraph::new(lines.clone())
                        .scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                    Rect::new(body.x, body.y, body.width.saturating_sub(1), body.height),
                );
                if lines.len() > height && height > 0 {
                    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
                    frame.render_stateful_widget(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(None)
                            .end_symbol(None)
                            .thumb_style(Style::default().fg(t.muted))
                            .track_style(Style::default().fg(t.border)),
                        body,
                        &mut ScrollbarState::new(lines.len() - height + 1)
                            .viewport_content_length(height)
                            .position(self.scroll),
                    );
                }
            }
            _ => {
                let width = body.width.saturating_sub(1);
                let lines = match &self.page {
                    Page::Help => help_lines(t, width),
                    Page::Delete { pending, index } => {
                        let task = &pending[*index];
                        let mut lines: Vec<Line<'static>> =
                            wrap_text(&format!("Delete pending task {}?", index + 1), width)
                                .into_iter()
                                .map(|l| {
                                    l.style(
                                        Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
                                    )
                                })
                                .collect();
                        lines.extend(
                            wrap_text(
                                &format!("{} · {}", task.id.as_deref().unwrap_or("·"), task.title),
                                width,
                            )
                            .into_iter()
                            .map(|l| {
                                l.style(Style::default().fg(t.bright).add_modifier(Modifier::BOLD))
                            }),
                        );
                        lines.push(Line::raw(""));
                        lines.extend(wrap_text(
                            "Removes it from Pending; it stays in History as Dropped.",
                            width,
                        ));
                        lines.push(Line::raw(""));
                        lines.push(Line::styled("Body", Style::default().fg(t.muted)));
                        if task.body.is_empty() {
                            lines.push(Line::styled("No body", Style::default().fg(t.muted)));
                        } else {
                            lines.extend(wrap_text(&task.body, width));
                        }
                        lines
                    }
                    Page::Feedback(text) => self.result_lines(t, text, width),
                    _ => unreachable!(),
                };
                let total = lines.len();
                self.scroll = self
                    .scroll
                    .min(lines.len().saturating_sub(usize::from(body.height)));
                frame.render_widget(
                    Paragraph::new(lines).scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                    Rect { width, ..body },
                );
                crate::ui::scrollbar(t, frame, body, total, self.scroll);
            }
        }
        hits
    }
}

impl Panel {
    /// The confirmation page: what is confirmed and what it does not do, the checks as drover
    /// reported them, then the one-line reason.
    fn draw_confirmation(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        body: ratatui::layout::Rect,
        focused: bool,
    ) {
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::Line,
            widgets::Paragraph,
        };
        let Page::Confirm(confirmation) = &self.page else {
            return;
        };
        if body.height < 6 {
            frame.render_widget(Paragraph::new("Enlarge the window to confirm"), body);
            return;
        }
        let width = body.width.saturating_sub(1);
        let name = std::path::Path::new(&confirmation.key.project)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut lines = Vec::new();
        for (label, value, color) in [
            (
                "Project",
                format!("{name} · {}", confirmation.key.project),
                t.muted,
            ),
            (
                "Task",
                format!("{} · {}", confirmation.key.id, confirmation.title),
                t.bright,
            ),
            ("Run", confirmation.run_id.clone(), t.muted),
        ] {
            lines.extend(
                wrap_text(&format!("{}{value}", crate::ui::pad(label, 9)), width)
                    .into_iter()
                    .map(|line| {
                        line.style(Style::default().fg(color).add_modifier(Modifier::BOLD))
                    }),
            );
        }
        lines.push(Line::raw(""));
        lines.extend(
            wrap_text(
                if confirmation.action == Transition::Return {
                    "Confirm that this run's work has stopped, including any implementers. Returning the task does not stop agents. Its ID and text stay the same, it moves to the front of Pending, and the dispatch pause setting stays unchanged. This run and your reason are kept; the task is not completed or dropped. No next task is sent."
                } else if confirmation.action == Transition::Submit { "Submit this run for review. It will wait for acceptance. Repository evidence is reference only." } else { "Accept this run's delivery and record it as Done. The next task requires explicit dispatch." },
                width,
            )
            .into_iter()
            .map(|l| l.style(Style::default().fg(t.text))),
        );
        if let Some(problem) = confirmation_problem(confirmation) {
            lines.push(Line::raw(""));
            lines.extend(wrap_text(&problem, width).into_iter().map(|l| {
                l.style(Style::default().fg(if confirmation.target.is_none() {
                    t.agent_starting
                } else {
                    t.agent_blocked
                }))
            }));
        }
        let reason_height = if confirmation.action == Transition::Return {
            3
        } else {
            0
        };
        let info = Rect::new(body.x, body.y, body.width, body.height - reason_height);
        let height = usize::from(info.height);
        self.scroll = self.scroll.min(lines.len().saturating_sub(height));
        let total = lines.len();
        frame.render_widget(
            Paragraph::new(lines).scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
            Rect::new(info.x, info.y, width, info.height),
        );
        crate::ui::scrollbar(t, frame, info, total, self.scroll);
        let Page::Confirm(confirmation) = &self.page else {
            return;
        };
        if confirmation.action != Transition::Return {
            return;
        }
        let field_area = Rect::new(body.x, body.bottom() - 3, body.width, 3);
        let field = t.block(" Reason (required) ", true);
        let inner = field.inner(field_area);
        frame.render_widget(field, field_area);
        let reason_width =
            unicode_width::UnicodeWidthStr::width(confirmation.reason.as_str()) as u16;
        frame.render_widget(
            Paragraph::new(confirmation.reason.as_str()).scroll((
                0,
                reason_width.saturating_sub(inner.width.saturating_sub(1)),
            )),
            inner,
        );
        if focused && !self.busy && !inner.is_empty() {
            frame.set_cursor_position((inner.x + reason_width.min(inner.width - 1), inner.y));
        }
    }
    /// Pending tasks grouped by registered project, including loading and read-failure states.
    fn all_pending_lines(&self, t: &Theme, width: u16) -> Vec<ratatui::text::Line<'static>> {
        use ratatui::{
            style::{Modifier, Style},
            text::{Line, Span},
        };
        let bold = |color| Style::default().fg(color).add_modifier(Modifier::BOLD);
        let mut lines = Vec::new();
        if let Some(error) = &self.registry_error {
            lines.extend(
                wrap_text(error, width)
                    .into_iter()
                    .map(|l| l.style(Style::default().fg(t.agent_error))),
            );
            return lines;
        }
        if self.all_pending.is_empty() {
            lines.push(Line::styled(
                "No registered projects",
                Style::default().fg(t.muted),
            ));
            return lines;
        }
        let loaded: Vec<_> = self
            .all_pending
            .iter()
            .filter_map(|(_, r)| r.as_ref())
            .collect();
        let total: usize = loaded
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .map(Vec::len)
            .sum();
        let failed = loaded.iter().filter(|r| r.is_err()).count();
        let loading = self.all_pending.len() - loaded.len();
        let mut summary = format!("{} projects · {total} pending", self.all_pending.len());
        if loading > 0 {
            summary += &format!(" · {loading} loading");
        }
        if failed > 0 {
            summary += &format!(" · {failed} failed");
        }
        lines.push(Line::styled(summary, Style::default().fg(t.muted)));
        for (project, result) in &self.all_pending {
            lines.push(Line::raw(""));
            let name = std::path::Path::new(project)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| project.clone());
            let state = match result {
                None => Span::styled("Loading…", Style::default().fg(t.muted)),
                Some(Err(_)) => Span::styled("Read failed", bold(t.agent_error)),
                Some(Ok(tasks)) if tasks.is_empty() => {
                    Span::styled("No pending", Style::default().fg(t.muted))
                }
                Some(Ok(tasks)) => {
                    Span::styled(format!("{} pending", tasks.len()), bold(t.agent_starting))
                }
            };
            lines.push(Line::from(vec![
                Span::styled(name, bold(t.connected)),
                Span::raw("  "),
                state,
            ]));
            lines.extend(
                wrap_text(project, width)
                    .into_iter()
                    .map(|l| l.style(Style::default().fg(t.dim))),
            );
            match result {
                Some(Err(error)) => lines.extend(
                    wrap_text(error, width)
                        .into_iter()
                        .map(|l| l.style(Style::default().fg(t.agent_error))),
                ),
                Some(Ok(tasks)) => {
                    for (index, task) in tasks.iter().enumerate() {
                        let prefix =
                            format!("{:>3} {} ", index + 1, task.id.as_deref().unwrap_or("·"));
                        let indent = unicode_width::UnicodeWidthStr::width(prefix.as_str());
                        let title =
                            wrap_text(&task.title, width.saturating_sub(indent as u16).max(1));
                        for (row, line) in title.into_iter().enumerate() {
                            let lead = if row == 0 {
                                Span::styled(prefix.clone(), Style::default().fg(t.muted))
                            } else {
                                Span::raw(" ".repeat(indent))
                            };
                            let mut spans = vec![lead];
                            spans.extend(line.spans);
                            lines.push(Line::from(spans));
                        }
                    }
                }
                None => {}
            }
        }
        lines
    }
}

/// Tasks help: grouped like the popup, with keys or button names in one column.
fn help_lines(t: &Theme, width: u16) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
    };
    const KEY: usize = 21;
    let groups: [(&str, &[(&str, &str)]); 4] = [
        (
            "Top · project and dispatch",
            &[
                (
                    "c",
                    "Projects; inside: a Add project, s Project settings, e Set path",
                ),
                ("r", "Refresh"),
                ("p", "Pause / Resume explicit dispatch"),
                ("R", "Record default for this project"),
            ],
        ),
        (
            "Bottom · tasks",
            &[
                ("a", "Add task"),
                ("N", "Notifications"),
                ("e", "Edit pending"),
                ("u / d", "Move pending"),
                ("x", "Delete pending"),
                ("m", "Mark a failed task seen in Attention"),
                (
                    "Dispatch selected",
                    "Explicitly send the selected Pending task",
                ),
                (
                    "[ ] Record",
                    "Beside Dispatch selected; changes the Record default for that dispatch only. Saddle's Telemetry recording switch still decides.",
                ),
                ("Submit for review", "Selected Running to Awaiting"),
                ("Accept", "Selected Awaiting to Done"),
                (
                    "Return to pending",
                    "Running or Awaiting; reason and work stopped confirmation required. Pause setting is unchanged.",
                ),
                ("A", "All pending"),
            ],
        ),
        (
            "List and views",
            &[
                ("Up/Down / j k", "Select task or project"),
                ("t", "Task text"),
                ("Enter", "Run details"),
                ("PgUp / PgDn", "Scroll text or details"),
                (
                    "Telemetry ↗",
                    "Saddle's Telemetry page on every run of the selected numbered task",
                ),
            ],
        ),
        (
            "Forms and pages",
            &[
                ("Tab", "Switch field"),
                ("Ctrl-S", "Save"),
                ("Esc", "Back or close Tasks"),
                ("Ctrl-]", "Agents"),
            ],
        ),
    ];
    let width = usize::from(width);
    // Narrow windows put each description under its key.
    let column = if width >= KEY + 20 { KEY } else { 2 };
    let mut lines = Vec::new();
    for (index, (heading, rows)) in groups.iter().enumerate() {
        if index > 0 {
            lines.push(Line::raw(""));
        }
        lines.push(Line::styled(
            *heading,
            Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
        ));
        for (key, text) in rows.iter() {
            let key_span = Span::styled(
                crate::ui::pad(&format!("  {key}"), column),
                Style::default().fg(t.muted),
            );
            let wrapped = wrap_words(text, width.saturating_sub(column).max(1));
            if column == KEY {
                for (row, line) in wrapped.into_iter().enumerate() {
                    let lead = if row == 0 {
                        key_span.clone()
                    } else {
                        Span::raw(" ".repeat(column))
                    };
                    let mut spans = vec![lead];
                    spans.extend(line.spans);
                    lines.push(Line::from(spans));
                }
            } else {
                lines.push(Line::from(key_span));
                for line in wrapped {
                    let mut spans = vec![Span::raw(" ".repeat(column))];
                    spans.extend(line.spans);
                    lines.push(Line::from(spans));
                }
            }
        }
    }
    lines
}

/// Wraps at spaces where it can; a word longer than the width is split like `wrap_text`.
fn wrap_words(text: &str, width: usize) -> Vec<ratatui::text::Line<'static>> {
    use unicode_width::UnicodeWidthStr;
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split(' ') {
        if !row.is_empty() && row.width() + 1 + word.width() > width {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    rows.push(row);
    rows.iter()
        .flat_map(|row| wrap_text(row, width as u16))
        .collect()
}

/// A compact button bar at the bottom of `area`, like `buttons::draw_compact`, with a divider
/// between non-empty groups that share a row. Returns the area left above the bar.
fn draw_groups(
    t: &Theme,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    groups: &[Vec<crate::buttons::Button<'_>>],
) -> (ratatui::layout::Rect, Vec<crate::buttons::Hit>) {
    use ratatui::{layout::Rect, style::Style, widgets::Paragraph};
    use unicode_width::UnicodeWidthStr;
    const GAP: u16 = 3;
    if area.height == 0 || area.width < 3 {
        return (area, Vec::new());
    }
    // Relative placements: buttons and the dividers drawn between groups.
    let mut placed = Vec::new();
    let mut dividers = Vec::new();
    let (mut x, mut y) = (0u16, 0u16);
    for group in groups.iter().filter(|g| !g.is_empty()) {
        let mut first = true;
        for button in group {
            let width = (button.label.width() as u16 + 2).min(area.width);
            let gap = match (x, first) {
                (0, _) => 0,
                (_, true) => GAP,
                _ => 1,
            };
            if x > 0 && x + gap + width > area.width {
                x = 0;
                y += 1;
            } else if gap == GAP {
                dividers.push((x + 1, y));
                x += gap;
            } else {
                x += gap;
            }
            placed.push((Rect::new(x, y, width, 1), button));
            x += width;
            first = false;
        }
    }
    let height = if placed.is_empty() {
        0
    } else {
        (y + 1).min(area.height)
    };
    let start = area.bottom() - height;
    let mut hits = Vec::new();
    for (relative, button) in placed {
        if relative.y >= height {
            break;
        }
        let rect = Rect::new(area.x + relative.x, start + relative.y, relative.width, 1);
        let (_, button_hits) =
            crate::buttons::draw_compact(t, frame, rect, std::slice::from_ref(button));
        hits.extend(button_hits);
    }
    for (dx, dy) in dividers {
        if dy < height {
            frame.render_widget(
                Paragraph::new("│").style(Style::default().fg(t.border)),
                Rect::new(area.x + dx, start + dy, 1, 1),
            );
        }
    }
    (
        Rect {
            height: area.height - height,
            ..area
        },
        hits,
    )
}

/// The same task across refreshes: by id, or by title and body if unnumbered.
fn same_task(a: &Task, b: &Task) -> bool {
    match &b.id {
        Some(id) => a.id.as_ref() == Some(id),
        None => a.id.is_none() && a.title == b.title && a.body == b.body,
    }
}

/// A task's state as the list words and colors it; unknown values keep their raw text.
pub(crate) fn task_status<'a>(
    t: &Theme,
    group: &str,
    status: Option<&'a str>,
) -> (&'a str, ratatui::style::Color) {
    match group {
        "Current" => ("Running", t.agent_working),
        "Awaiting" => ("Awaiting", t.agent_blocked),
        "Pending" => ("Pending", t.agent_starting),
        _ => match status {
            Some("done") => ("Done", t.agent_idle),
            Some("failed") => ("Failed", t.agent_error),
            Some("dropped" | "drop") => ("Dropped", t.agent_stalled),
            Some(s) => (s, t.muted),
            None => ("—", t.dim),
        },
    }
}

pub(crate) fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

pub(crate) fn wrap_text(text: &str, width: u16) -> Vec<ratatui::text::Line<'static>> {
    use unicode_width::UnicodeWidthChar;
    if width == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for c in clean(text).replace('\t', "    ").chars() {
        if c == '\n' {
            rows.push(ratatui::text::Line::raw(std::mem::take(&mut line)));
            used = 0;
            continue;
        }
        let w = c.width().unwrap_or(0);
        if used + w > usize::from(width) && !line.is_empty() {
            rows.push(ratatui::text::Line::raw(std::mem::take(&mut line)));
            used = 0;
        }
        line.push(c);
        used += w;
    }
    rows.push(ratatui::text::Line::raw(line));
    rows
}
