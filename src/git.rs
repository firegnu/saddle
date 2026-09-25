use crate::command::{Output, run_with_env};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(5);
// Never fetch missing objects of a partial clone just to count lines.
const ENV: &[(&str, &str)] = &[("GIT_NO_LAZY_FETCH", "1")];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    Branch(String),
    Detached,
    Unknown,
}
/// Uncommitted changes of the working tree against HEAD, staged and unstaged together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    pub added: u64,
    pub deleted: u64,
    /// Files without line counts, such as binaries; not folded into the line totals.
    pub binary: u64,
}
/// Read-only Git state of one worktree; `None` fields could not be determined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub head: Head,
    /// Commits on HEAD beyond the base: local main, or the upstream when on main.
    pub ahead: Option<(u64, String)>,
    pub changes: Option<Changes>,
    pub untracked: Option<u64>,
}
/// Results by the cwd they were computed for; `None` means Git was unavailable there.
pub type Batch = Vec<(String, Option<Summary>)>;

struct Worktree<'a> {
    program: &'a str,
    top: PathBuf,
    attr_source: String,
    cancel: &'a AtomicBool,
}
impl Worktree<'_> {
    fn run(&self, args: &[&str]) -> Option<Output> {
        git(
            self.program,
            &self.top,
            Some(self.attr_source.as_str()),
            args,
            self.cancel,
        )
    }
    fn text(&self, args: &[&str]) -> Option<String> {
        self.run(args)
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
    }
    fn summary(&self) -> Summary {
        let head = match self.run(&["symbolic-ref", "--quiet", "--short", "HEAD"]) {
            Some(o) if o.status.success() => {
                Head::Branch(String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
            }
            Some(o) if o.status.code() == Some(1) => Head::Detached,
            _ => Head::Unknown,
        };
        let base = match &head {
            Head::Branch(branch) if branch == "main" => self
                .text(&[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream}",
                ])
                .map(|name| ("@{upstream}", name)),
            Head::Branch(_) => self
                .text(&[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    "refs/heads/main^{commit}",
                ])
                .map(|_| ("refs/heads/main", "main".to_owned())),
            _ => None,
        };
        let ahead = base.and_then(|(rev, name)| {
            let count = self.text(&["rev-list", "--count", &format!("{rev}..HEAD")])?;
            Some((count.parse().ok()?, name))
        });
        let changes = self
            .run(&[
                "diff-index",
                "--numstat",
                "-z",
                "--no-ext-diff",
                "--no-textconv",
                "HEAD",
                "--",
            ])
            .filter(|o| o.status.success())
            .map(|o| {
                let mut changes = Changes {
                    added: 0,
                    deleted: 0,
                    binary: 0,
                };
                for record in o.stdout.split(|b| *b == 0) {
                    let record = String::from_utf8_lossy(record);
                    let mut fields = record.splitn(3, '\t');
                    match (fields.next(), fields.next()) {
                        (Some("-"), Some("-")) => changes.binary += 1,
                        (Some(added), Some(deleted)) => {
                            changes.added += added.parse::<u64>().unwrap_or(0);
                            changes.deleted += deleted.parse::<u64>().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
                changes
            });
        let untracked = self
            .run(&["ls-files", "--others", "--exclude-standard", "-z"])
            .filter(|o| o.status.success())
            .map(|o| o.stdout.iter().filter(|b| **b == 0).count() as u64);
        Summary {
            head,
            ahead,
            changes,
            untracked,
        }
    }
}

// Every call skips optional index writes and fsmonitor hooks; once the object format is known,
// attributes come from the empty tree so no diff driver, textconv or filter from the repo runs.
fn git(
    program: &str,
    dir: &Path,
    attr_source: Option<&str>,
    args: &[&str],
    cancel: &AtomicBool,
) -> Option<Output> {
    let mut full = vec!["--no-optional-locks", "-c", "core.fsmonitor=false"];
    full.extend(attr_source);
    full.extend(args);
    run_with_env(program, &full, Some(dir), ENV, TIMEOUT, cancel).ok()
}

/// Summaries for each cwd; directories in the same worktree share one query per call.
/// A cancelled round returns nothing, so a partial round never reports directories as unavailable.
pub fn collect(program: &str, cwds: &[String], cancel: &AtomicBool) -> Batch {
    let mut shared: HashMap<PathBuf, Summary> = HashMap::new();
    let batch = cwds
        .iter()
        .map(|cwd| {
            let summary = locate(program, cwd, cancel).map(|(top, attr_source)| {
                shared
                    .entry(top.clone())
                    .or_insert_with(|| {
                        Worktree {
                            program,
                            top,
                            attr_source,
                            cancel,
                        }
                        .summary()
                    })
                    .clone()
            });
            (cwd.clone(), summary)
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        Vec::new()
    } else {
        batch
    }
}

fn locate(program: &str, cwd: &str, cancel: &AtomicBool) -> Option<(PathBuf, String)> {
    if !Path::new(cwd).is_absolute() {
        return None;
    }
    let output = git(
        program,
        Path::new(cwd),
        None,
        &["rev-parse", "--show-toplevel", "--show-object-format"],
        cancel,
    )
    .filter(|o| o.status.success())?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines();
    let top = lines.next()?.into();
    let empty_tree = match lines.next()? {
        "sha1" => "4b825dc642cb6eb9a060e54bf8d69288fbee4904",
        "sha256" => "6ef19b41225c5369f1c104d45d8d85efa9b057b53b14b4b9b939dd74decc5321",
        _ => return None,
    };
    Some((top, format!("--attr-source={empty_tree}")))
}

/// Re-reads the watched directories in the background every `every`, and at once when they change.
pub struct Poller {
    pub updates: mpsc::Receiver<Batch>,
    watched: Vec<String>,
    wake: mpsc::Sender<Vec<String>>,
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Poller {
    pub fn start(program: String, every: Duration) -> Self {
        let (tx, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::spawn(move || {
            let mut cwds = Vec::new();
            loop {
                match wait.recv_timeout(every) {
                    Ok(next) => cwds = next,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                if let Some(latest) = wait.try_iter().last() {
                    cwds = latest;
                }
                if quitting.load(Ordering::Relaxed) {
                    break;
                }
                if cwds.is_empty() {
                    continue;
                }
                let batch = collect(&program, &cwds, &quitting);
                if quitting.load(Ordering::Relaxed) || tx.send(batch).is_err() {
                    break;
                }
            }
        });
        Self {
            updates,
            watched: Vec::new(),
            wake,
            cancel,
            worker: Some(worker),
        }
    }
    pub fn watch(&mut self, cwds: Vec<String>) {
        if cwds != self.watched {
            self.watched.clone_from(&cwds);
            let _ = self.wake.send(cwds);
        }
    }
}
impl Drop for Poller {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.wake.send(Vec::new());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
