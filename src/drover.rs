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
        }
    }
}
impl Client {
    pub fn execute(&self, operation: &Operation, cancel: &AtomicBool) -> Result<String> {
        if let Operation::Edit { pending, index, .. } | Operation::Move { pending, index, .. } =
            operation
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
                    "Pending tasks changed; action not sent. Reopen Edit or retry Move using the refreshed queue."
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
