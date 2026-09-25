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
        }
    }
}
impl Client {
    pub fn execute(&self, operation: &Operation, cancel: &AtomicBool) -> Result<String> {
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
