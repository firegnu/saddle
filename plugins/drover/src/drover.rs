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
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("drover list: invalid JSON")?;
        validate_read(&value, "list")?;
        serde_json::from_value(value)
            .context("drover list: response does not match schema_version 2")
    }
}

fn validate_read(value: &serde_json::Value, command: &str) -> Result<()> {
    if value["schema_version"] != 2 {
        bail!(
            "drover {command}: unsupported schema_version {}",
            value["schema_version"]
        );
    }
    if value["ok"] != true {
        bail!(
            "drover {command}: {}: {}",
            value["error"]["code"].as_str().unwrap_or("unknown error"),
            value["error"]["why"].as_str().unwrap_or_default()
        );
    }
    Ok(())
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
    DispatchPending {
        project: String,
        pos: u64,
        token: String,
    },
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
    fn result_state(self) -> &'static str {
        match self {
            Self::Submit => "awaiting_release",
            Self::Accept => "done",
            Self::Return => "pending",
        }
    }
}
impl Operation {
    pub fn args(&self) -> Vec<String> {
        match self {
            Self::Pause(true) => vec!["pause".into()],
            Self::Pause(false) => vec!["resume".into()],
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
            Self::Transition {
                id,
                token,
                action,
                reason,
                ..
            } => {
                let mut args = vec![
                    action.command().into(),
                    id.clone(),
                    "--target-token".into(),
                    token.clone(),
                ];
                if *action == Transition::Return {
                    args.extend([
                        "--reason".into(),
                        reason.trim().into(),
                        "--work-stopped".into(),
                    ]);
                }
                args.push("--json".into());
                args
            }
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
        if let Operation::Transition {
            project,
            id,
            run_id,
            action,
            ..
        } = operation
        {
            return self.transition(project, id, run_id, *action, &operation.args(), cancel);
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
/// Budget for one `drover show`: its Git subcommands may each take 30 s and the optional agent
/// status 10 s. A slower answer counts as a failure and is retried on the next refresh.
const SHOW_TIMEOUT: Duration = Duration::from_secs(60);
impl Client {
    /// Read-only details of a numbered task; never runs the check command.
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
        validate_read(&value, "show")?;
        if value["task"]["id"] != id {
            bail!("drover show: answered for a different task than {id}");
        }
        if !output.status.success() {
            bail!("drover show ({}): {}", output.status, text(&output.stderr));
        }
        let detail: Detail = serde_json::from_value(value)
            .context("drover show: response does not match schema_version 2")?;
        if detail.evidence.scope != "repository_reference" || detail.evidence.controls_transition {
            bail!("drover show: unsupported evidence scope or transition control");
        }
        Ok(detail)
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
impl Client {
    fn transition(
        &self,
        project: &str,
        id: &str,
        run_id: &str,
        action: Transition,
        args: &[String],
        cancel: &AtomicBool,
    ) -> Result<String> {
        let real = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.into());
        if real(&self.cwd) != real(std::path::Path::new(project)) {
            bail!("The project changed; task action not sent.");
        }
        let output = crate::command::run(&self.program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(), Some(&self.cwd),
            Duration::from_secs(120), cancel)
            .context("Task action result unknown; refresh and check the task before confirming again. Saddle has not retried")?;
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("Task action returned no valid JSON; result unknown. Refresh and check the task. Saddle has not retried")?;
        if value["schema_version"] != 2 {
            bail!(
                "drover {}: unsupported schema_version {}; check the task before confirming again",
                action.command(),
                value["schema_version"]
            );
        }
        if value["ok"] == false {
            return Err(TransitionError {
                code: value["error"]["code"]
                    .as_str()
                    .unwrap_or("unknown_error")
                    .into(),
                why: value["error"]["why"].as_str().unwrap_or_default().into(),
            }
            .into());
        }
        if !output.status.success()
            || value["ok"] != true
            || value["task_id"] != id
            || value["run_id"] != run_id
            || value["state"] != action.result_state()
            || value["record"]["status"] != "recorded"
        {
            bail!(
                "drover {}: unexpected result for {id} / {run_id}; check the task before confirming again: {}",
                action.command(),
                crate::ui::clip(&value.to_string(), 300)
            );
        }
        Ok(match action {
            Transition::Submit => format!(
                "{id} submitted for review · Awaiting release\nRecord: recorded · Run: {run_id}"
            ),
            Transition::Accept => format!("{id} accepted · Done\nRecord: recorded · Run: {run_id}"),
            Transition::Return => format!(
                "{id} returned to Pending\nRecord: recorded · Run: {run_id}\nReason: {}\nDispatch pause setting is unchanged. Work stopped was confirmed by the user.",
                args[5]
            ),
        })
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
        if value["schema_version"] != 2 {
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
