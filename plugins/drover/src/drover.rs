use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;
use std::{sync::atomic::AtomicBool, time::Duration};

/// Plugin-owned project registry, retained at its existing data path.
pub fn registered_projects(path: &std::path::Path) -> Result<Vec<String>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("Read {}", path.display())),
    };
    let mut projects = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let path = crate::config::expand_home(line);
        let path = path.canonicalize().unwrap_or(path).display().to_string();
        if !projects.contains(&path) {
            projects.push(path);
        }
    }
    Ok(projects)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Task {
    pub id: Option<String>,
    pub title: String,
    pub body: String,
    pub status: Option<String>,
    pub reason: Option<String>,
    pub run_id: Option<String>,
    pub notification_key: Option<String>,
    pub t0: Option<serde_json::Value>,
    pub t1: Option<serde_json::Value>,
    pub t2: Option<serde_json::Value>,
    pub start: Option<String>,
    pub main: Option<String>,
    pub submission: Option<serde_json::Value>,
    pub completion_record: Option<serde_json::Value>,
    pub return_history: Vec<serde_json::Value>,
    pub previous_runs: Vec<Task>,
    pub actions: std::collections::BTreeMap<String, ActionTarget>,
}
/// Opaque targets from the displayed public response, never reconstructed by Saddle.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ActionTarget {
    pub pos: Option<u64>,
    pub target_token: Option<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Snapshot {
    /// The canonical project root drover answered for.
    #[serde(default)]
    pub project: String,
    pub paused: bool,
    /// Whether this project's dispatches are recorded unless one says otherwise.
    #[serde(default)]
    pub record_default: bool,
    pub current: Option<Task>,
    pub awaiting: Option<Task>,
    pub pending: Vec<Task>,
    pub history: Vec<Task>,
}
#[derive(Clone)]
pub struct Client {
    pub corral: String,
    pub cwd: PathBuf,
}
impl Client {
    pub fn snapshot(&self) -> Result<Snapshot> {
        self.read(&AtomicBool::new(false))
    }
    fn read(&self, cancel: &AtomicBool) -> Result<Snapshot> {
        anyhow::ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "Read cancelled"
        );
        serde_json::from_value(crate::core::list(&self.cwd)?)
            .context("Invalid native task projection")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Pause(bool),
    Add {
        title: String,
        body: String,
    },
    Edit {
        pending: Vec<Task>,
        index: usize,
        title: String,
        body: String,
    },
    Move {
        pending: Vec<Task>,
        index: usize,
        to: usize,
    },
    Delete {
        pending: Vec<Task>,
        index: usize,
    },
    /// One explicitly confirmed transition of a specific run.
    Transition {
        project: String,
        id: String,
        run_id: String,
        token: String,
        action: Transition,
        reason: String,
    },
    /// Dispatch the Pending task the list showed at `pos`, bound by its listing token.
    /// `record` overrides the project's recording default for this one dispatch.
    DispatchPending {
        project: String,
        pos: u64,
        token: String,
        record: Option<bool>,
    },
    /// Save whether this project's dispatches are recorded by default.
    RecordDefault(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Submit,
    Accept,
    Return,
}
impl Transition {
    pub fn command(self) -> &'static str {
        match self {
            Self::Submit => "done",
            Self::Accept => "go",
            Self::Return => "return-to-pending",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Submit => "Submit for review",
            Self::Accept => "Accept",
            Self::Return => "Return to pending",
        }
    }
    pub fn allows(self, state: &str) -> bool {
        match self {
            Self::Submit => state == "running",
            Self::Accept => state == "awaiting_release",
            Self::Return => matches!(state, "running" | "awaiting_release"),
        }
    }
}
impl Client {
    pub fn execute(&self, operation: &Operation, cancel: &AtomicBool) -> Result<String> {
        let value = crate::core::execute(&self.cwd, operation, &self.corral, cancel)?;
        if let Operation::DispatchPending { .. } = operation {
            let (clean, report) = dispatch_report(&value, value["ok"] == true);
            return if clean { Ok(report) } else { bail!(report) };
        }
        if let Operation::Transition {
            id,
            run_id,
            action,
            reason,
            ..
        } = operation
        {
            let telemetry: String = match value["telemetry"]["status"].as_str() {
                Some("stored" | "duplicate") => {
                    "Telemetry: transition recorded on the run's trace".into()
                }
                Some("not_recorded") => "Telemetry: this run has no trace".into(),
                Some(other) => format!("Telemetry: transition not recorded ({other})"),
                None => "Telemetry: unknown".into(),
            };
            let done = match action {
                Transition::Submit => format!(
                    "{id} submitted for review · Awaiting release\nRecord: recorded · Run: {run_id}"
                ),
                Transition::Accept => {
                    format!("{id} accepted · Done\nRecord: recorded · Run: {run_id}")
                }
                Transition::Return => format!(
                    "{id} returned to Pending\nRecord: recorded · Run: {run_id}\nReason: {reason}\nDispatch pause setting is unchanged. Work stopped was confirmed by the user."
                ),
            };
            return Ok(format!("{done}\n{telemetry}"));
        }
        Ok(value["message"].as_str().unwrap_or("Done").into())
    }
}

pub enum Request {
    Refresh,
    AllPending,
    Projects,
    Project(String),
    Run(Operation),
}
pub enum Update {
    Snapshot(Result<Box<Snapshot>>),
    Feedback(Operation, Result<String>),
}
pub struct Worker {
    pub updates: std::sync::mpsc::Receiver<Update>,
    requests: std::sync::mpsc::Sender<Request>,
    cancel: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Worker {
    pub fn start(client: Client, every: Duration) -> Self {
        use std::sync::{Arc, atomic::Ordering, mpsc};
        let (send, updates) = mpsc::channel();
        let (requests, receive) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let thread = std::thread::spawn(move || {
            while !quitting.load(Ordering::Relaxed) {
                if send
                    .send(Update::Snapshot(client.read(&quitting).map(Box::new)))
                    .is_err()
                {
                    break;
                }
                match receive.recv_timeout(every) {
                    Ok(Request::Run(op)) => {
                        if send
                            .send(Update::Feedback(op.clone(), client.execute(&op, &quitting)))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    _ => {}
                }
            }
        });
        Self {
            updates,
            requests,
            cancel,
            thread: Some(thread),
        }
    }
    pub fn request(&self, request: Request) {
        let _ = self.requests.send(request);
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.request(Request::Refresh);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Reads the pending tasks of every registered project in parallel; results arrive per project.
pub struct PendingLoad {
    pub updates: std::sync::mpsc::Receiver<(usize, Result<Vec<Task>>)>,
    cancel: std::sync::Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}
impl PendingLoad {
    pub fn start(corral: &str, projects: &[String]) -> Self {
        let (send, updates) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let threads = projects
            .iter()
            .enumerate()
            .map(|(index, project)| {
                let client = Client {
                    corral: corral.into(),
                    cwd: project.into(),
                };
                let (send, cancel) = (send.clone(), cancel.clone());
                std::thread::spawn(move || {
                    let _ = send.send((index, client.read(&cancel).map(|s| s.pending)));
                })
            })
            .collect();
        Self {
            updates,
            cancel,
            threads,
        }
    }
}
impl Drop for PendingLoad {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

/// Finds the registered project of the repository holding `cwd`, answering it only when its
/// queue has any task; `None` whenever that cannot be established. Worktrees and subdirectories
/// belong to their repository; a project at the same worktree top is preferred.
pub struct RepoTasks {
    pub result: std::sync::mpsc::Receiver<Option<String>>,
    cancel: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl RepoTasks {
    pub fn start(corral: String, cwd: String, projects: Vec<String>) -> Self {
        let (send, result) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let thread = std::thread::spawn(move || {
            let find = || {
                let (top, common) = crate::git::repository("git", &cwd, &quitting)?;
                let matching: Vec<_> = projects
                    .iter()
                    .filter_map(|project| {
                        let (project_top, project_common) =
                            crate::git::repository("git", project, &quitting)?;
                        (project_common == common).then_some((project, project_top == top))
                    })
                    .collect();
                let (project, _) = matching
                    .iter()
                    .find(|(_, same_top)| *same_top)
                    .or(matching.first())?;
                let snapshot = Client {
                    corral,
                    cwd: project.into(),
                }
                .read(&quitting)
                .ok()?;
                let any = snapshot.current.is_some()
                    || snapshot.awaiting.is_some()
                    || !snapshot.pending.is_empty()
                    || !snapshot.history.is_empty();
                any.then(|| project.to_string())
            };
            let _ = send.send(find());
        });
        Self {
            result,
            cancel,
            thread: Some(thread),
        }
    }
}
impl Drop for RepoTasks {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Schema 2 task plus repository evidence; no completion gates are consumed.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Detail {
    pub project: String,
    pub task: Task,
    pub evidence: Evidence,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Evidence {
    pub scope: String,
    pub controls_transition: bool,
    pub observed_at: f64,
    pub git: Reference,
    pub last_check: Reference,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Reference {
    pub state: String,
    #[serde(flatten)]
    pub fields: std::collections::BTreeMap<String, serde_json::Value>,
}
impl Client {
    pub fn show(&self, id: &str, cancel: &AtomicBool) -> Result<Detail> {
        serde_json::from_value(crate::core::show(&self.cwd, id, &self.corral, cancel)?)
            .context("Invalid native task detail")
    }
}
/// A public transition rejection; its code is not inferred from prose.
#[derive(Debug)]
pub struct TransitionError {
    pub code: String,
    pub why: String,
}
impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Task action failed ({}): {}. Refresh and confirm again; Saddle has not retried.",
            self.code, self.why
        )
    }
}
impl std::error::Error for TransitionError {}
/// Words `dispatch-pending`'s answer for the user, delivery and record apart, and whether it is
/// a success: exit 0, `ok`, and either a confirmed delivery without a draft or manual mode, each
/// recorded.
fn dispatch_report(value: &serde_json::Value, exited_zero: bool) -> (bool, String) {
    let task = value["task_id"].as_str();
    let delivery = &value["delivery"];
    let status = delivery["status"].as_str().unwrap_or("missing");
    let record = value["record"]["status"].as_str().unwrap_or("missing");
    let exit = delivery["corral_exit_code"].as_i64();
    let exit_text = exit.map_or("no exit code".to_owned(), |code| {
        format!("corral exit {code}")
    });
    let confirmed = status == "confirmed"
        && delivery["confirmed"] == true
        && delivery["attempted"] == true
        && exit == Some(0);
    let recorded = record == "recorded"
        && value["state"] == "running"
        && value["run_id"].as_str().is_some_and(|id| !id.is_empty());
    let draft = delivery["merged_with_draft"] == true;
    let manual = value["manual_text"]
        .as_str()
        .filter(|text| !text.is_empty());
    let mut lines = vec![match task {
        Some(id) => format!("{id} · dispatch of the selected task"),
        None => "Selected task · not dispatched".to_owned(),
    }];
    lines.push(match status {
        "confirmed" if confirmed => "Delivered to the main agent (confirmed by corral).".to_owned(),
        "confirmed" => {
            "Delivery result inconsistent; confirmation is unknown. Check the main agent."
                .to_owned()
        }
        "unconfirmed" => {
            format!("Delivery not confirmed ({exit_text}); check the main agent. Do not resend.")
        }
        "rejected" => {
            let why = match exit {
                Some(2) => "no such agent",
                Some(6) => "sandbox refused",
                Some(7) => "agent not idle",
                Some(8) => "a person is typing there",
                _ => "refused",
            };
            format!("Delivery rejected by corral ({exit_text}: {why}); nothing was sent.")
        }
        "unknown" => format!(
            "Delivery result unknown ({exit_text}); the text may or may not have been sent."
        ),
        "not_sent" if manual.is_some() => {
            "Delivery: not sent; paste the text below to the main agent yourself.".to_owned()
        }
        "not_sent" => "Delivery: not sent.".to_owned(),
        "not_executed" => {
            "Delivery not executed: Saddle did not start Corral; nothing was sent.".to_owned()
        }
        "not_attempted" => "Delivery: not attempted; nothing was sent.".to_owned(),
        other => format!("Delivery status unrecognized ({other}); check the main agent."),
    });
    if draft {
        lines.push(
            "Corral merged it with a draft typed at the main agent; check what it received. Do not resend."
                .into(),
        );
    }
    lines.push(match record {
        "recorded" if recorded => "Record: recorded as Running.".to_owned(),
        "recorded" => {
            "Record: reported recorded, but the run/state is inconsistent; check the queue."
                .to_owned()
        }
        "not_attempted" => "Record: not recorded; drover saved no start.".to_owned(),
        "unknown" => "Record: unknown; the start may or may not have been saved.".to_owned(),
        other => format!("Record: unrecognized ({other})."),
    });
    lines.push(telemetry_report(&value["telemetry"]));
    if let Some(code) = value["error"]["code"].as_str() {
        let meaning = match code {
            "target_changed" => "the queue changed since it was shown; Refresh and choose again",
            "state_busy" => "another queue write was running",
            "current_exists" => "a task is already running",
            "paused" => "the queue is paused",
            "awaiting_release" => "a task awaits release",
            "send_rejected" => "corral refused the delivery",
            "delivery_unconfirmed" => "delivery was not confirmed",
            "delivery_unknown" => "the delivery result is unknown",
            "target_ambiguous" => "another pending task shares its number or title",
            "write_failed" => "drover could not save its record",
            "delivery_not_executed" => "Saddle did not start the delivery",
            _ => "drover refused the request",
        };
        lines.push(format!(
            "Drover: {meaning} ({code}): {}",
            value["error"]["why"].as_str().unwrap_or_default()
        ));
    }
    let clean = exited_zero
        && value["ok"] == true
        && task.is_some_and(|id| !id.is_empty())
        && recorded
        && ((confirmed && !draft)
            || (status == "not_sent" && delivery["attempted"] == false && manual.is_some()));
    if value["ok"] == true && !clean {
        lines.push(format!(
            "Unexpected answer ({}); check the queue and main agent.",
            if exited_zero {
                "exit 0"
            } else {
                "non-zero exit"
            }
        ));
    }
    if let Some(text) = manual {
        lines.push(String::new());
        lines.push("Text for the main agent:".into());
        lines.push(text.to_owned());
    }
    if !clean {
        lines.push(String::new());
        lines.push(
            "Saddle has not retried. The queue refreshes now; check it and the main agent before dispatching again."
                .into(),
        );
    }
    (clean, lines.join("\n"))
}
/// What a dispatch recorded, apart from its delivery and task record.
fn telemetry_report(telemetry: &serde_json::Value) -> String {
    let text = |v: &serde_json::Value| v.as_str().unwrap_or("unknown").to_owned();
    match telemetry["status"].as_str() {
        Some("context") => {
            let trace: String = text(&telemetry["trace_id"]).chars().take(8).collect();
            let send = match telemetry["send"]["receipt"].as_str() {
                Some("matched") => format!(
                    "send begin {}, end {}",
                    text(&telemetry["send"]["begin"]),
                    text(&telemetry["send"]["end"])
                ),
                Some(_) => "send receipt missing".into(),
                None => "nothing sent".into(),
            };
            format!(
                "Telemetry: recorded context, trace {trace}… · {send} · transition {}",
                text(&telemetry["transition"]["status"])
            )
        }
        Some("no_context") => format!(
            "Telemetry: no record context ({}); sent the plain way.",
            text(&telemetry["reason"])
        ),
        _ => "Telemetry: not requested for this dispatch.".into(),
    }
}
/// Queries one task's details until dropped; the next query starts only after the previous one
/// returned. Dropping it cancels a running query and discards its results.
pub struct DetailWorker {
    pub updates: std::sync::mpsc::Receiver<Result<Detail>>,
    stop: Option<std::sync::mpsc::Sender<()>>,
    cancel: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl DetailWorker {
    pub fn start(client: Client, id: String, every: Duration) -> Self {
        use std::sync::{Arc, mpsc};
        let (send, updates) = mpsc::channel();
        let (stop, stopped) = mpsc::channel::<()>();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let thread = std::thread::spawn(move || {
            while send.send(client.show(&id, &quitting)).is_ok()
                && stopped.recv_timeout(every) == Err(mpsc::RecvTimeoutError::Timeout)
            {}
        });
        Self {
            updates,
            stop: Some(stop),
            cancel,
            thread: Some(thread),
        }
    }
}
impl Drop for DetailWorker {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.stop.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One Attention reading: the registry, then each listed project's snapshot as its read returns.
pub enum Survey {
    Projects(Result<Vec<String>, String>),
    Snapshot(String, Result<Box<Snapshot>, String>),
}
/// Rereads the registry and every registered project's public snapshot, a round at a time,
/// until dropped. Projects are read in parallel so a slow one does not hold back the others;
/// the next round starts only after the previous one finished.
pub struct Surveyor {
    pub updates: std::sync::mpsc::Receiver<Survey>,
    wake: std::sync::mpsc::Sender<()>,
    cancel: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Surveyor {
    pub fn start(corral: String, registry: PathBuf, every: Duration) -> Self {
        use std::sync::{Arc, atomic::Ordering, mpsc};
        let (send, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel::<()>();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let thread = std::thread::spawn(move || {
            while !quitting.load(Ordering::Relaxed) {
                let projects = registered_projects(&registry).map_err(|e| format!("{e:#}"));
                let list = projects.clone().unwrap_or_default();
                if send.send(Survey::Projects(projects)).is_err() {
                    break;
                }
                std::thread::scope(|scope| {
                    for project in &list {
                        let client = Client {
                            corral: corral.clone(),
                            cwd: project.into(),
                        };
                        let (send, quitting) = (send.clone(), &quitting);
                        scope.spawn(move || {
                            let result = client.read(quitting).map(Box::new);
                            let _ = send.send(Survey::Snapshot(
                                project.clone(),
                                result.map_err(|e| format!("{e:#}")),
                            ));
                        });
                    }
                });
                if wait.recv_timeout(every) == Err(mpsc::RecvTimeoutError::Disconnected) {
                    break;
                }
            }
        });
        Self {
            updates,
            wake,
            cancel,
            thread: Some(thread),
        }
    }
    /// Starts the next round now, or right after the one in progress.
    pub fn refresh(&self) {
        let _ = self.wake.send(());
    }
}
impl Drop for Surveyor {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.refresh();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The plugin's persisted notification preference, compatible with existing settings.
/// Saving it means only that it is saved; Drover applies it at its next notification check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preference {
    pub system_enabled: bool,
    pub revision: u64,
}
impl Client {
    pub fn notifications(&self, set: Option<bool>, cancel: &AtomicBool) -> Result<Preference> {
        anyhow::ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "Preference operation cancelled"
        );
        crate::core::preference(set)
    }
}
/// Who asked for a preference reading: the periodic check, or the Settings popup with this
/// token opening or saving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asker {
    Poll,
    Open(u64),
    Save(u64),
}
pub struct ChannelUpdate {
    pub asker: Asker,
    pub result: Result<Preference, String>,
}
/// Serializes native notification preference reads and writes from
/// Settings. Serial calls keep answers in order, so the latest answer is the latest preference.
pub struct ChannelWorker {
    pub updates: std::sync::mpsc::Receiver<ChannelUpdate>,
    requests: Option<std::sync::mpsc::Sender<(Asker, Option<bool>)>>,
    cancel: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ChannelWorker {
    pub fn start(client: Client, every: Duration) -> Self {
        use std::sync::{Arc, atomic::Ordering, mpsc};
        let (send, updates) = mpsc::channel();
        let (requests, receive) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let thread = std::thread::spawn(move || {
            let mut next = (Asker::Poll, None);
            while !quitting.load(Ordering::Relaxed) {
                let (asker, set) = next;
                let result = client
                    .notifications(set, &quitting)
                    .map_err(|e| format!("{e:#}"));
                if send.send(ChannelUpdate { asker, result }).is_err() {
                    break;
                }
                next = match receive.recv_timeout(every) {
                    Ok(request) => request,
                    Err(mpsc::RecvTimeoutError::Timeout) => (Asker::Poll, None),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
            }
        });
        Self {
            updates,
            requests: Some(requests),
            cancel,
            thread: Some(thread),
        }
    }
    /// Reads the preference now, or right after the call in progress.
    pub fn read(&self, asker: Asker) {
        self.request(asker, None);
    }
    /// Turns Drover's system notifications on or off.
    pub fn set(&self, asker: Asker, system_enabled: bool) {
        self.request(asker, Some(system_enabled));
    }
    fn request(&self, asker: Asker, set: Option<bool>) {
        if let Some(requests) = &self.requests {
            let _ = requests.send((asker, set));
        }
    }
}
impl Drop for ChannelWorker {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.requests.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
