use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;
use std::{sync::atomic::AtomicBool, time::Duration};

/// The only drover file saddle reads directly, explicitly authorized by the user.
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
    /// The start event's time, commit and main: with the id, a run's identity. Kept as JSON so a
    /// malformed time makes only that identity unusable, not the whole snapshot.
    pub t0: Option<serde_json::Value>,
    pub start: Option<String>,
    pub main: Option<String>,
    pub return_history: Vec<ReturnRecord>,
    /// A pending task's target for `dispatch-pending`, as this listing showed it; absent from
    /// an older drover, and left out when malformed.
    #[serde(deserialize_with = "lenient")]
    pub dispatch_pending: Option<PendingTarget>,
}
/// The shown position and opaque token of one Pending task; handed back unchanged, never built.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PendingTarget {
    pub pos: Option<u64>,
    pub target_token: Option<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Mode {
    pub r#loop: bool,
    pub gate: bool,
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Snapshot {
    pub mode: Mode,
    pub paused: bool,
    pub current: Option<Task>,
    pub awaiting: Option<Task>,
    pub pending: Vec<Task>,
    pub history: Vec<Task>,
}
#[derive(Clone)]
pub struct Client {
    pub program: String,
    pub cwd: PathBuf,
}
impl Client {
    pub fn snapshot(&self) -> Result<Snapshot> {
        self.read(&AtomicBool::new(false))
    }
    fn read(&self, cancel: &AtomicBool) -> Result<Snapshot> {
        let output = crate::command::run(
            &self.program,
            &["list", "--json"],
            Some(&self.cwd),
            Duration::from_secs(15),
            cancel,
        )?;
        if !output.status.success() {
            bail!(
                "drover list: {}{}",
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        serde_json::from_slice(&output.stdout).context("drover list: invalid JSON")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Go,
    Next,
    Pause(bool),
    Loop(bool),
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
    /// Return a specific run after the user confirms its work has stopped.
    ReturnToPending {
        project: String,
        id: String,
        token: String,
        reason: String,
    },
    /// The user's confirmation that one run is complete, bound by `drover show`'s token.
    CompleteManually {
        project: String,
        id: String,
        token: String,
        reason: String,
    },
    /// Dispatch the Pending task the list showed at `pos`, bound by its listing token.
    DispatchPending {
        project: String,
        pos: u64,
        token: String,
    },
}
impl Operation {
    pub fn args(&self) -> Vec<String> {
        match self {
            Self::Go => vec!["go".into()],
            Self::Next => vec!["next".into()],
            Self::Pause(true) => vec!["pause".into()],
            Self::Pause(false) => vec!["resume".into()],
            Self::Loop(true) => vec!["loop".into(), "on".into()],
            Self::Loop(false) => vec!["loop".into(), "off".into()],
            Self::Add { title, body } => vec!["add".into(), title.clone(), body.clone()],
            Self::Edit {
                index, title, body, ..
            } => {
                vec![
                    "edit".into(),
                    (index + 1).to_string(),
                    title.clone(),
                    body.clone(),
                ]
            }
            Self::Move { index, to, .. } => {
                vec!["move".into(), (index + 1).to_string(), (to + 1).to_string()]
            }
            Self::Delete { index, .. } => vec![
                "drop".into(),
                "--pos".into(),
                (index + 1).to_string(),
                "Deleted in saddle".into(),
            ],
            Self::ReturnToPending {
                id, token, reason, ..
            } => vec![
                "return-to-pending".into(),
                id.clone(),
                "--target-token".into(),
                token.clone(),
                "--reason".into(),
                reason.trim().into(),
                "--work-stopped".into(),
                "--json".into(),
            ],
            Self::CompleteManually {
                id, token, reason, ..
            } => vec![
                "complete-manually".into(),
                id.clone(),
                "--target-token".into(),
                token.clone(),
                "--reason".into(),
                reason.trim().into(),
                "--json".into(),
            ],
            Self::DispatchPending { pos, token, .. } => vec![
                "dispatch-pending".into(),
                "--pos".into(),
                pos.to_string(),
                "--target-token".into(),
                token.clone(),
                "--json".into(),
            ],
        }
    }
}
impl Client {
    pub fn execute(&self, operation: &Operation, cancel: &AtomicBool) -> Result<String> {
        if let Operation::ReturnToPending { project, id, .. } = operation {
            return self.return_to_pending(project, id, &operation.args(), cancel);
        }
        if let Operation::CompleteManually { project, id, .. } = operation {
            return self.complete_manually(project, id, &operation.args(), cancel);
        }
        if let Operation::DispatchPending { project, .. } = operation {
            return self.dispatch_pending(project, &operation.args(), cancel);
        }
        if let Operation::Edit { pending, index, .. }
        | Operation::Move { pending, index, .. }
        | Operation::Delete { pending, index } = operation
        {
            let fresh = self.read(cancel)?;
            if *index >= pending.len()
                || !fresh
                    .pending
                    .iter()
                    .map(|t| (&t.id, &t.title, &t.body))
                    .eq(pending.iter().map(|t| (&t.id, &t.title, &t.body)))
            {
                bail!(
                    "Pending tasks changed; action not sent. Reopen Edit or Delete, or retry Move, using the refreshed queue."
                );
            }
        }
        let args = operation.args();
        let output = crate::command::run(
            &self.program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
            Some(&self.cwd),
            Duration::from_secs(120),
            cancel,
        )?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .trim()
        .to_string();
        if !output.status.success() {
            bail!(
                "drover {} ({}): {}",
                operation.args()[0],
                output.status,
                text
            );
        }
        Ok(if text.is_empty() { "Done".into() } else { text })
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
    pub fn start(program: &str, projects: &[String]) -> Self {
        let (send, updates) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let threads = projects
            .iter()
            .enumerate()
            .map(|(index, project)| {
                let client = Client {
                    program: program.into(),
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
    pub fn start(program: String, cwd: String, projects: Vec<String>) -> Self {
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
                    program,
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

/// One task from public `drover show Tn --json` (schema_version 1). Only structured fields
/// carry meaning; `why` texts are shown verbatim and never parsed.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Detail {
    pub observed_at: f64,
    pub task: DetailTask,
    pub timing: Timing,
    pub git: GitProgress,
    pub completion: Completion,
    pub last_check: LastCheck,
    pub routing: Option<Routing>,
    pub hold: Hold,
    pub attention: Attention,
    pub warnings: Vec<Warning>,
    /// Absent from a drover without manual completion.
    #[serde(default)]
    pub manual_completion: Option<ManualTarget>,
    #[serde(default)]
    pub return_to_pending: Option<ManualTarget>,
}
/// The run `drover show` read, as an opaque token to hand back unchanged.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ManualTarget {
    pub target_token: Option<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DetailTask {
    pub id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    /// current, awaiting or history; an awaiting task's status is still done.
    pub location: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// A manual completion's saved record; absent for every other task, never back-filled.
    #[serde(default)]
    pub completion_record: Option<CompletionRecord>,
    #[serde(default)]
    pub return_history: Vec<ReturnRecord>,
}
/// One returned run, saved by Drover; never reconstructed from current task state.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ReturnRecord {
    pub reason: String,
    pub dispatched_at: Option<serde_json::Number>,
    pub returned_at: Option<serde_json::Number>,
    pub work_stopped: bool,
}
/// What the user confirmed and what the checks said then; not a check that passed.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CompletionRecord {
    pub method: String,
    pub reason: String,
    pub confirmed_at: Option<f64>,
    /// Snapshots saved at confirmation; one that does not parse is left out, not guessed.
    #[serde(default, deserialize_with = "lenient")]
    pub completion: Option<Completion>,
    #[serde(default, deserialize_with = "lenient")]
    pub last_check: Option<LastCheck>,
    #[serde(default, deserialize_with = "lenient")]
    pub workspace: Option<Workspace>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub state: String,
    pub tracked_dirty: bool,
}
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Timing {
    pub started_at: Option<f64>,
    pub ended_at: Option<f64>,
    pub released_at: Option<f64>,
    pub elapsed_seconds: Option<f64>,
    pub release_wait_seconds: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GitProgress {
    pub start_head: Option<String>,
    pub start_main: Option<String>,
    /// HEAD recorded at done, not main at that time.
    pub end_head: Option<String>,
    pub observed_head: Option<String>,
    pub observed_main: Option<String>,
    /// A range count, not commits owned by this task.
    pub range_commits: Option<u64>,
    pub main_commits_since_start: Option<u64>,
    /// Commit time of main as observed now, even for history.
    pub main_tip_committed_at: Option<f64>,
    pub unavailable_reasons: std::collections::BTreeMap<String, String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Completion {
    pub scope: String,
    /// Recomputed now for current/awaiting; `None` for history, whose snapshot was not saved.
    pub rows: Option<Vec<CompletionRow>>,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CompletionRow {
    pub id: String,
    pub state: String,
    pub reason: String,
    pub why: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LastCheck {
    pub status: String,
    pub scope: String,
    pub record: Option<CheckRecord>,
    pub stale_reasons: Vec<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CheckRecord {
    pub cmd: String,
    pub why: String,
    pub ok: Option<bool>,
    pub t: f64,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Routing {
    pub source: String,
    pub tier: String,
    pub cross: bool,
    pub overridden: bool,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Hold {
    pub enabled: Option<bool>,
    pub scope: String,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Attention {
    pub state: String,
    pub reason: String,
    pub unmet_rows: Vec<String>,
    pub agent: Option<AttentionAgent>,
    pub inference: bool,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AttentionAgent {
    pub name: Option<String>,
    pub state: Option<String>,
    pub last_input_source: Option<String>,
    pub idle_for: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Warning {
    pub code: String,
    pub sources: Vec<String>,
}
/// Budget for one `drover show`: its Git subcommands may each take 30 s and the optional agent
/// status 10 s. A slower answer counts as a failure and is retried on the next refresh.
const SHOW_TIMEOUT: Duration = Duration::from_secs(60);
impl Client {
    /// Read-only details of a started, finished or dropped task; never runs the check command.
    pub fn show(&self, id: &str, cancel: &AtomicBool) -> Result<Detail> {
        let output = crate::command::run(
            &self.program,
            &["show", id, "--json", "--with-agent-status"],
            Some(&self.cwd),
            SHOW_TIMEOUT,
            cancel,
        )?;
        let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_owned();
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).with_context(|| {
                format!(
                    "drover show ({}): {}{}",
                    output.status,
                    text(&output.stdout),
                    text(&output.stderr)
                )
            })?;
        if value["schema_version"] != 1 {
            bail!(
                "drover show: unsupported schema_version {}",
                value["schema_version"]
            );
        }
        if value["ok"] != true {
            bail!(
                "drover show: {}: {}",
                value["error"]["code"].as_str().unwrap_or("unknown error"),
                value["error"]["why"].as_str().unwrap_or_default()
            );
        }
        if !output.status.success() {
            bail!("drover show ({}): {}", output.status, text(&output.stderr));
        }
        serde_json::from_value(value)
            .context("drover show: response does not match schema_version 1")
    }
}
/// A public `complete-manually` failure code, with Drover's text shown but never parsed.
#[derive(Debug)]
pub struct ManualError {
    pub code: String,
    pub why: String,
}
impl std::fmt::Display for ManualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let meaning = match self.code.as_str() {
            "target_changed" => {
                "The task run changed since this page read it; nothing was completed. Refresh, then confirm again"
            }
            "state_busy" => {
                "Drover is busy with another queue write; nothing was completed. Try again"
            }
            "invalid_arguments" => "Drover rejected the request",
            "repository_unavailable" => "The project repository is unavailable",
            "not_configured" => "The project is not set up for drover",
            "configuration_unreadable" => "Drover cannot read the project configuration",
            "state_unreadable" => "Drover cannot read the task state",
            "state_invalid" => "Drover cannot determine the current run",
            "snapshot_unavailable" => "Drover cannot take a reliable check snapshot",
            "write_failed" => "Drover could not save the completion",
            _ => "Manual completion failed",
        };
        write!(f, "{meaning} ({}): {}", self.code, self.why)
    }
}
impl std::error::Error for ManualError {}
impl Client {
    /// Runs `complete-manually` in this project only, and accepts nothing but this target now
    /// awaiting release. Never falls back to another write.
    fn complete_manually(
        &self,
        project: &str,
        id: &str,
        args: &[String],
        cancel: &AtomicBool,
    ) -> Result<String> {
        let real = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.into());
        if real(&self.cwd) != real(std::path::Path::new(project)) {
            bail!("The project changed; manual completion not sent.");
        }
        let output = crate::command::run(
            &self.program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
            Some(&self.cwd),
            Duration::from_secs(120),
            cancel,
        )?;
        let text = |bytes: &[u8]| crate::ui::clip(String::from_utf8_lossy(bytes).trim(), 300);
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            bail!(
                "This drover does not support manual completion ({}); nothing else was sent: {}{}",
                output.status,
                text(&output.stdout),
                text(&output.stderr)
            );
        };
        if value["schema_version"] != 1 {
            bail!(
                "drover complete-manually: unsupported schema_version {}",
                value["schema_version"]
            );
        }
        if value["ok"] == false {
            return Err(ManualError {
                code: value["error"]["code"]
                    .as_str()
                    .unwrap_or("unknown_error")
                    .into(),
                why: value["error"]["why"].as_str().unwrap_or_default().into(),
            }
            .into());
        }
        if value["ok"] != true
            || value["task_id"] != id
            || value["state"] != "awaiting_release"
            || !output.status.success()
        {
            bail!(
                "drover complete-manually ({} exit): unexpected answer for {id}; check the queue before retrying: {}",
                output.status,
                crate::ui::clip(&value.to_string(), 300)
            );
        }
        let record: Option<CompletionRecord> =
            serde_json::from_value(value["completion_record"].clone()).ok();
        let mut lines = vec![format!("{id} marked complete manually · Awaiting release")];
        if let Some(record) = record {
            lines.push(format!("Reason: {}", record.reason));
            if let Some(at) = record.confirmed_at {
                lines.push(format!("Confirmed: {}", crate::detail::clock(at)));
            }
            if let Some(rows) = record.completion.and_then(|c| c.rows) {
                let rows: Vec<_> = rows
                    .iter()
                    .map(|r| format!("{} {}", r.id.replace('_', " "), r.state.replace('_', " ")))
                    .collect();
                lines.push(format!("Checks saved at confirmation: {}", rows.join(", ")));
            }
            if let Some(workspace) = record.workspace {
                lines.push(format!("Workspace: {}", workspace.state));
            }
        }
        lines.push(String::new());
        lines.push(
            "Checks were not run and are not counted as passed. The task waits for your release (Check & release); saddle did not send go or next, change loop or pause, touch Git, or stop agents.".into(),
        );
        Ok(lines.join("\n"))
    }
}
#[derive(Debug)]
pub struct ReturnError {
    pub code: String,
    pub why: String,
}
impl std::fmt::Display for ReturnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Return to pending failed ({}): {}", self.code, self.why)
    }
}
impl std::error::Error for ReturnError {}
impl Client {
    fn return_to_pending(
        &self,
        project: &str,
        id: &str,
        args: &[String],
        cancel: &AtomicBool,
    ) -> Result<String> {
        let real = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.into());
        if real(&self.cwd) != real(std::path::Path::new(project)) {
            bail!("The project changed; return to pending not sent.");
        }
        let output = crate::command::run(
            &self.program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
            Some(&self.cwd),
            Duration::from_secs(120),
            cancel,
        )?;
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("Return to pending returned no valid JSON; check the queue before retrying. Nothing else was sent")?;
        if value["schema_version"] != 1 {
            bail!(
                "drover return-to-pending: unsupported schema_version {}",
                value["schema_version"]
            );
        }
        if value["ok"] == false {
            return Err(ReturnError {
                code: value["error"]["code"]
                    .as_str()
                    .unwrap_or("unknown_error")
                    .into(),
                why: value["error"]["why"].as_str().unwrap_or_default().into(),
            }
            .into());
        }
        if value["ok"] != true
            || value["task_id"] != id
            || value["state"] != "pending"
            || value["paused"] != true
            || !output.status.success()
        {
            bail!(
                "drover return-to-pending: unexpected answer for {id}; check the queue before retrying: {}",
                crate::ui::clip(&value.to_string(), 300)
            );
        }
        let record: ReturnRecord = serde_json::from_value(value["return_record"].clone())
            .context("Return result has no valid record; check the queue before retrying")?;
        if !record.work_stopped {
            bail!("Return result does not confirm work stopped; check the queue before retrying");
        }
        Ok(format!(
            "{id} returned to Pending · Queue paused\nReason: {}\nReturned: {}\n\nThe task keeps its ID and text at the front of Pending. This run's history is retained. No task was sent and no agent was stopped. Resume the queue when ready.",
            record.reason,
            record
                .returned_at
                .as_ref()
                .and_then(|n| n.as_f64())
                .map(crate::detail::clock)
                .unwrap_or_else(|| "not recorded".into())
        ))
    }
}
impl Client {
    /// Runs `dispatch-pending` once in this project. Only a confirmed delivery that was recorded,
    /// or manual mode's recorded start with its text, is a success; every other answer is
    /// reported from its structured fields and never retried.
    fn dispatch_pending(
        &self,
        project: &str,
        args: &[String],
        cancel: &AtomicBool,
    ) -> Result<String> {
        let real = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.into());
        if real(&self.cwd) != real(std::path::Path::new(project)) {
            bail!("The project changed; the selected task was not dispatched.");
        }
        let output = crate::command::run(
            &self.program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
            Some(&self.cwd),
            Duration::from_secs(120),
            cancel,
        )
        .map_err(|error| {
            // The run may have ended after drover already delivered; its answer is lost.
            anyhow::anyhow!(
                "Saddle cannot confirm the dispatch result ({error:#}); the task may or may not have been sent. Saddle has not retried. The queue refreshes now; check it and the main agent before dispatching again."
            )
        })?;
        let text = |bytes: &[u8]| crate::ui::clip(String::from_utf8_lossy(bytes).trim(), 300);
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            bail!(
                "This drover does not support dispatching a selected task, or its answer was lost ({}). Saddle has not retried; refresh and check the queue and main agent: {}{}",
                output.status,
                text(&output.stdout),
                text(&output.stderr)
            );
        };
        if value["schema_version"] != 1 {
            bail!(
                "drover dispatch-pending: unsupported schema_version {}. Saddle has not retried; check the queue and main agent.",
                value["schema_version"]
            );
        }
        let (clean, report) = dispatch_report(&value, output.status.success());
        if clean { Ok(report) } else { bail!(report) }
    }
}
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
    let draft = delivery["merged_with_draft"] == true;
    let manual = value["manual_text"]
        .as_str()
        .filter(|text| !text.is_empty());
    let mut lines = vec![match task {
        Some(id) => format!("{id} · dispatch of the selected task"),
        None => "Selected task · not dispatched".to_owned(),
    }];
    lines.push(match status {
        "confirmed" => "Delivered to the main agent (confirmed by corral).".to_owned(),
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
        "not_sent" => {
            "Delivery: not sent; no main agent is configured. Paste the text below to it yourself."
                .to_owned()
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
        "recorded" => "Record: recorded as started (now Current).".to_owned(),
        "not_attempted" => "Record: not recorded; drover saved no start.".to_owned(),
        "unknown" => "Record: unknown; the start may or may not have been saved.".to_owned(),
        other => format!("Record: unrecognized ({other})."),
    });
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
            _ => "drover refused the request",
        };
        lines.push(format!(
            "Drover: {meaning} ({code}): {}",
            value["error"]["why"].as_str().unwrap_or_default()
        ));
    }
    let clean = exited_zero
        && value["ok"] == true
        && task.is_some()
        && record == "recorded"
        && ((status == "confirmed" && !draft) || (status == "not_sent" && manual.is_some()));
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
    pub fn start(program: String, registry: PathBuf, every: Duration) -> Self {
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
                            program: program.clone(),
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

/// The user's Drover system-notification preference (`drover notifications`, schema_version 1).
/// Saving it means only that it is saved; Drover applies it at its next notification check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preference {
    pub system_enabled: bool,
    pub revision: u64,
}
impl Client {
    /// Reads the preference (`None`), or turns system notifications on or off. The command does
    /// not depend on the project, so it runs without one.
    pub fn notifications(&self, set: Option<bool>, cancel: &AtomicBool) -> Result<Preference> {
        let action = match set {
            None => "status",
            Some(true) => "on",
            Some(false) => "off",
        };
        let output = crate::command::run(
            &self.program,
            &["notifications", action, "--json"],
            None,
            Duration::from_secs(15),
            cancel,
        )?;
        let text = |bytes: &[u8]| crate::ui::clip(String::from_utf8_lossy(bytes).trim(), 200);
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            bail!(
                "this drover does not support task notifications ({}): {}{}",
                output.status,
                text(&output.stdout),
                text(&output.stderr)
            );
        };
        if value["schema_version"] != 1 {
            bail!(
                "drover notifications: unsupported schema_version {}",
                value["schema_version"]
            );
        }
        if value["ok"] == false {
            let code = value["error"]["code"].as_str().unwrap_or("unknown_error");
            let meaning = match code {
                "invalid_arguments" => "drover rejected the request",
                "preferences_invalid" => "Drover's notification preference is invalid",
                "preferences_unreadable" => "Drover's notification preference cannot be read",
                "preferences_write_failed" => "Drover could not save the notification preference",
                _ => "drover notifications failed",
            };
            bail!(
                "{meaning} ({code}): {}",
                value["error"]["message"].as_str().unwrap_or_default()
            );
        }
        match (
            output.status.success() && value["ok"] == true && value["scope"] == "user",
            value["system_enabled"].as_bool(),
            value["revision"].as_u64(),
        ) {
            (true, Some(system_enabled), Some(revision)) => Ok(Preference {
                system_enabled,
                revision,
            }),
            _ => bail!(
                "drover notifications ({}): unexpected answer {}",
                output.status,
                crate::ui::clip(&value.to_string(), 200)
            ),
        }
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
/// Runs `drover notifications` one call at a time: a periodic status read, and the requests of
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
