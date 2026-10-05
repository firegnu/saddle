//! Updates: whether the installed saddle and Corral are the ones in use. Checks only read, in
//! the background; only the user's Upgrade all writes, through the public `corral upgrade --all`
//! of the installed Corral, once per click and never on its own.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

/// Where the check looks.
#[derive(Clone, Debug)]
pub struct Sources {
    /// The saddle running now, resolved when it started: the command entry may since point
    /// elsewhere.
    pub running: Result<PathBuf, String>,
    /// The `saddle` command: a name looked up on PATH, or a path. What it points to is the
    /// installed saddle.
    pub command: String,
    /// The configured corral; the default `corral` is resolved independently on PATH.
    pub corral: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saddle {
    Current,
    /// Another saddle is installed: close and reopen the interface.
    Reopen(PathBuf),
    Unknown(String),
}

/// One agent's Corral, from its public status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pen {
    Current,
    /// Runs another program; the error of its failed last upgrade, if any.
    Outdated(Option<String>),
    /// Started by a Corral without in-place upgrade.
    NoProtocol,
    /// An upgrade is under way: its state.
    Pending(String),
    /// Frozen after a failed switch; `corral recover` continues it.
    Held {
        state: String,
        error: Option<String>,
    },
    Unknown(String),
}

#[derive(Clone, Debug)]
pub struct Check {
    pub started: Instant,
    pub at: SystemTime,
    pub saddle: Saddle,
    /// The installed saddle, when found.
    pub installed: Option<PathBuf>,
    /// The installed Corral, or why it is not known.
    pub corral: Result<PathBuf, String>,
    /// The agents the installed Corral lists, or why they could not be read.
    pub agents: Result<Vec<(String, Pen)>, String>,
    /// The running saddle, as resolved when it started.
    pub running: Result<PathBuf, String>,
    /// The builds of the running and installed saddle, from their packages' build records.
    pub running_build: Result<Build, String>,
    pub installed_build: Result<Build, String>,
    /// The Saddle source the installed build names, and what it has since.
    pub source: Result<Source, String>,
    verified_receipt: Option<Instant>,
}

/// A program's build, from the BUILD.txt that package.sh writes beside it, and only when the
/// record's checksum matches the program: neither a directory name nor a repository HEAD proves
/// what a binary was built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    pub revision: String,
    /// Built from a working tree with uncommitted changes, or one the record does not call clean.
    pub modified: bool,
    /// The Saddle source checkout and branch it was packaged from; older records do not say.
    pub source: Option<(PathBuf, String)>,
    pub record: PathBuf,
}

/// The Saddle source checkout a build names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub checkout: PathBuf,
    pub branch: String,
    /// The branch's latest commit now.
    pub head: String,
    /// Commits on the branch that the installed build does not have, or why that is unknown.
    pub newer: Result<u64, String>,
}

/// File checksums kept while the file does not change, so checks do not reread binaries.
#[derive(Default)]
pub struct Files(HashMap<PathBuf, (Stamp, [u8; 32])>);
type Stamp = (u64, u64, u64, i64, i64, i64, i64);

fn stamp(path: &Path) -> Result<Stamp, String> {
    let m = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !m.is_file() {
        return Err(format!("{} is not a file", path.display()));
    }
    Ok((
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    ))
}
impl Files {
    fn digest(&mut self, path: &Path) -> Result<[u8; 32], String> {
        let before = stamp(path)?;
        if let Some((old, digest)) = self.0.get(path)
            && *old == before
        {
            return Ok(*digest);
        }
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        if stamp(path)? != before {
            return Err("Program changed during check; Refresh again".into());
        }
        let digest = hash.finalize().into();
        self.0.insert(path.to_owned(), (before, digest));
        Ok(digest)
    }
    fn same(&mut self, a: &Path, b: &Path) -> Result<bool, String> {
        let (before_a, before_b) = (stamp(a)?, stamp(b)?);
        let same = self.digest(a)? == self.digest(b)?;
        if stamp(a)? != before_a || stamp(b)? != before_b {
            return Err("Program changed during check; Refresh again".into());
        }
        Ok(same)
    }
}

/// The build record of a program at `<package>/bin/<name>`, verified against its content.
fn build(program: &Path, files: &mut Files) -> Result<Build, String> {
    let (Some(bin), Some(name)) = (program.parent(), program.file_name()) else {
        return Err("not a program file".into());
    };
    let package = bin
        .parent()
        .filter(|_| bin.file_name().is_some_and(|n| n == "bin"))
        .ok_or("not in a package, so no BUILD.txt")?;
    let record = package.join("BUILD.txt");
    let text = fs::read_to_string(&record).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => "no BUILD.txt in its package".to_owned(),
        _ => format!("BUILD.txt: {e}"),
    })?;
    let entry = format!("bin/{}", name.to_string_lossy());
    let mut fields = HashMap::new();
    let mut checksum = None;
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(": ") {
            fields.insert(key, value.trim());
        } else if let Some((hash, file)) = line.split_once("  ")
            && file == entry
        {
            checksum = Some(hash);
        }
    }
    let checksum = checksum.ok_or_else(|| format!("BUILD.txt has no checksum of {entry}"))?;
    let actual: String = files
        .digest(program)?
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !checksum.eq_ignore_ascii_case(&actual) {
        return Err("BUILD.txt does not match this program".into());
    }
    let revision = fields
        .get("revision")
        .filter(|r| r.len() >= 40 && r.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("BUILD.txt has no valid revision")?;
    let source = match (fields.get("source"), fields.get("branch")) {
        (Some(path), Some(branch)) if !path.is_empty() => {
            Some((PathBuf::from(path), (*branch).to_owned()))
        }
        _ => None,
    };
    Ok(Build {
        revision: revision.to_ascii_lowercase(),
        modified: fields.get("working-tree") != Some(&"clean"),
        source,
        record,
    })
}

/// The branch a build was packaged from, as its source checkout has it now, compared with the
/// installed build when that is known. Only reads, with Git's bounded runner.
fn source(named: &Build, installed: Option<&Build>, cancel: &AtomicBool) -> Result<Source, String> {
    let (checkout, branch) = named
        .source
        .clone()
        .ok_or("the build record names no Saddle source (older package)")?;
    if !checkout.is_absolute() {
        return Err("the build record has no absolute Saddle source path".into());
    }
    if branch.is_empty() {
        return Err("packaged from a detached HEAD; no branch to compare".into());
    }
    if !checkout.is_dir() {
        return Err(format!("source {} is no longer there", checkout.display()));
    }
    let git = |args: &[&str]| {
        crate::git::git("git", &checkout, args, cancel)
            .ok_or_else(|| format!("Git cannot read {}", checkout.display()))
    };
    let output = git(&["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if !output.status.success()
        || !root.is_absolute()
        || fs::canonicalize(root)
            .ok()
            .zip(fs::canonicalize(&checkout).ok())
            .is_none_or(|(root, checkout)| root != checkout)
    {
        return Err("the recorded Saddle source is not a worktree root".into());
    }
    let output = git(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("refs/heads/{branch}^{{commit}}"),
    ])?;
    if !output.status.success() {
        return Err(format!(
            "branch {branch} not found in {}",
            checkout.display()
        ));
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let newer = installed
        .ok_or_else(|| "the installed build is unknown".to_owned())
        .and_then(|build| {
            let ancestor = git(&["merge-base", "--is-ancestor", &build.revision, &head])?;
            match ancestor.status.code() {
                Some(0) => {}
                Some(1) => return Err(format!("the installed revision is not on {branch}")),
                _ => return Err("the source does not have the installed revision".into()),
            }
            let count = git(&[
                "rev-list",
                "--count",
                &format!("{}..{head}", build.revision),
            ])?;
            String::from_utf8_lossy(&count.stdout)
                .trim()
                .parse()
                .map_err(|_| "Git did not count the commits".to_owned())
        });
    Ok(Source {
        checkout,
        branch,
        head,
        newer,
    })
}

fn executable(program: &str) -> Result<PathBuf, String> {
    let path = crate::config::expand_home(program);
    let runnable = |p: &Path| {
        fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    let path = if program.contains('/') {
        path
    } else {
        std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths)
            .map(|dir| dir.join(&path))
            .find(|p| runnable(p))
            .ok_or_else(|| format!("{program} not found on PATH"))?
    };
    if !runnable(&path) {
        return Err(format!("{} is not executable", path.display()));
    }
    fs::canonicalize(path).map_err(|e| e.to_string())
}

fn reason(value: &Value) -> Option<String> {
    for key in ["message", "last_error", "error", "handover", "upgrade"] {
        let v = &value[key];
        if let Some(s) = v.as_str() {
            return Some(s.to_owned());
        }
        if v.is_object()
            && let Some(s) = reason(v)
        {
            return Some(s);
        }
    }
    None
}

fn read(
    corral: &Path,
    args: &[&str],
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let output = crate::command::run_bounded(
        &corral.to_string_lossy(),
        args,
        None,
        &[],
        timeout,
        cancel,
        4 * 1024 * 1024,
    )
    .map_err(|e| format!("{e:#}"))?;
    let value: Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid JSON: {e}"))?;
    if !output.status.success() || value["ok"] != true {
        return Err(reason(&value).unwrap_or_else(|| "Command failed".into()));
    }
    Ok(value)
}

fn pen(value: &Value, target: &Path, files: &mut Files) -> Pen {
    if value["capabilities"]["upgrade"] != 1 {
        return Pen::NoProtocol;
    }
    let state = value["upgrade"]["state"].as_str().unwrap_or("unknown");
    let error = reason(&value["upgrade"]);
    if matches!(state, "hold" | "hold_unprotected") {
        return Pen::Held {
            state: state.into(),
            error,
        };
    }
    if state != "none" {
        return Pen::Pending(state.into());
    }
    if value["custody"] != "owner" {
        return Pen::Unknown("Custody is not owner; inspect corral status".into());
    }
    let Some(exe) = value["exe"].as_str() else {
        return Pen::Unknown("No running executable reported".into());
    };
    match files.same(Path::new(exe), target) {
        Ok(true)
            if !matches!(
                value["upgrade"]["result"].as_str(),
                None | Some("complete" | "already_current")
            ) =>
        {
            Pen::Unknown(format!(
                "Upgrade {}: {}",
                value["upgrade"]["result"],
                error.unwrap_or_default()
            ))
        }
        Ok(true) => Pen::Current,
        Ok(false) => Pen::Outdated(error),
        Err(e) => Pen::Unknown(e),
    }
}

pub fn check(
    sources: &Sources,
    files: &mut Files,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Check {
    let mut result = Check {
        started: Instant::now(),
        at: SystemTime::now(),
        saddle: Saddle::Unknown("not checked".into()),
        installed: None,
        corral: Err("not checked".into()),
        agents: Err("not checked".into()),
        running: sources.running.clone(),
        running_build: Err("not checked".into()),
        installed_build: Err("not checked".into()),
        source: Err("not checked".into()),
        verified_receipt: None,
    };
    let installed = executable(&sources.command);
    result.saddle = match (&sources.running, &installed) {
        (Ok(running), Ok(target)) => match files.same(running, target) {
            Ok(true) => Saddle::Current,
            Ok(false) => Saddle::Reopen(target.clone()),
            Err(e) => Saddle::Unknown(e),
        },
        (Err(e), _) | (_, Err(e)) => Saddle::Unknown(e.clone()),
    };
    result.running_build = sources
        .running
        .clone()
        .and_then(|running| build(&running, files));
    result.installed_build = installed
        .clone()
        .and_then(|installed| build(&installed, files));
    let named = [&result.installed_build, &result.running_build]
        .into_iter()
        .flatten()
        .find(|b| b.source.is_some());
    result.source = match named {
        Some(named) => source(named, result.installed_build.as_ref().ok(), cancel),
        None => Err("no build record names a Saddle source (older package or none)".into()),
    };
    result.corral = executable(&sources.corral);
    result.installed = installed.ok();
    result.agents = result
        .corral
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|corral| {
            let listing = read(corral, &["ls"], timeout, cancel)?;
            let agents = listing["agents"].as_array().ok_or("No agents in listing")?;
            agents
                .iter()
                .map(|agent| {
                    let name = agent["name"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or("Agent without name")?;
                    let state = if agent["starting"] == true {
                        Pen::Unknown("Starting; Refresh later".into())
                    } else {
                        match read(corral, &["status", name], timeout, cancel) {
                            Ok(status) => pen(&status, corral, files),
                            Err(e) => Pen::Unknown(e),
                        }
                    };
                    Ok((name.to_owned(), state))
                })
                .collect()
        });
    result
}

/// What `corral upgrade --all` answered.
#[derive(Clone, Debug)]
pub struct Receipt {
    raw: Value,
    pub at: SystemTime,
    pub finished: Instant,
    /// Err: the whole run's outcome is unknown, or the command refused it.
    pub outcome: Result<(), Failure>,
    pub items: Vec<Item>,
    /// What the run says about reminders it cannot find.
    pub legacy: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// No usable answer: nothing is known about what it did.
    Unknown(String),
    /// Refused before upgrading anything.
    Failed(String),
}
/// One agent in the receipt, with its reminders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    /// The pen's own result.
    pub pen: String,
    pub reason: Option<String>,
    pub after: Vec<After>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct After {
    pub request_id: Option<String>,
    pub result: String,
    pub reason: Option<String>,
}

/// Runs the installed Corral's `upgrade --all` with itself as the target.
pub fn upgrade(corral: &Path, timeout: Duration) -> Receipt {
    let mut receipt = Receipt {
        raw: Value::Null,
        at: SystemTime::now(),
        finished: Instant::now(),
        outcome: Err(Failure::Unknown("not run".into())),
        items: Vec::new(),
        legacy: None,
    };
    let run = || -> Result<(Value, bool), Failure> {
        let program = corral.to_string_lossy();
        let output = crate::command::run_bounded(
            &program,
            &["upgrade", "--all", "--exe", &program],
            None,
            &[],
            timeout,
            &AtomicBool::new(false),
            4 * 1024 * 1024,
        )
        .map_err(|e| Failure::Unknown(format!("{e:#}")))?;
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| Failure::Unknown(format!("Invalid upgrade receipt: {e}")))?;
        if !value["results"].is_array() {
            return Err(if value["ok"] == false && value["error"].is_string() {
                Failure::Failed(reason(&value).unwrap_or_else(|| "Upgrade refused".into()))
            } else {
                Failure::Unknown("Missing per-agent upgrade results".into())
            });
        }
        Ok((value, output.status.success()))
    };
    match run() {
        Err(e) => receipt.outcome = Err(e),
        Ok((value, success)) => {
            let results = value["results"].as_array().unwrap();
            let valid = value["ok"].is_boolean()
                && !(value["ok"] == false && results.is_empty())
                && results.iter().all(|item| {
                    item["name"].as_str().is_some_and(|name| !name.is_empty())
                        && item["result"].is_string()
                        && item.get("after").is_none_or(|after| {
                            after
                                .as_array()
                                .is_some_and(|items| items.iter().all(|a| a["result"].is_string()))
                        })
                });
            receipt.outcome = if !success || !valid {
                Err(Failure::Unknown(
                    "Incomplete upgrade receipt or nonzero command exit; inspect retained results"
                        .into(),
                ))
            } else if value["helper"]["result"] == "complete" {
                Ok(())
            } else {
                Err(Failure::Unknown("Helper update not confirmed".into()))
            };
            receipt.legacy = value["legacy_after"].as_str().map(str::to_owned);
            receipt.items = value["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| Item {
                    name: v["name"].as_str().unwrap_or("unknown agent").into(),
                    pen: v["pen_result"]
                        .as_str()
                        .or(v["result"].as_str())
                        .unwrap_or("unknown")
                        .into(),
                    reason: reason(v),
                    after: v["after"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .map(|v| After {
                                    request_id: v["request_id"].as_str().map(str::to_owned),
                                    result: v["result"].as_str().unwrap_or("unknown").into(),
                                    reason: reason(v),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect();
            receipt.raw = value;
        }
    }
    receipt.finished = Instant::now();
    receipt.at = SystemTime::now();
    receipt
}

// Reconcile a retained receipt through public status/after only. Never repeat its action.
fn verified(receipt: &Receipt, corral: &Path, files: &mut Files, cancel: &AtomicBool) -> bool {
    if receipt.outcome.is_err() {
        return false;
    }
    let Some(items) = receipt.raw["results"].as_array() else {
        return false;
    };
    for item in items {
        let Some(name) = item["name"].as_str() else {
            return false;
        };
        let result = item["pen_result"]
            .as_str()
            .or(item["result"].as_str())
            .unwrap_or("unknown");
        if !complete(result) {
            let Ok(status) = read(corral, &["status", name], Duration::from_secs(15), cancel)
            else {
                return false;
            };
            if pen(&status, corral, files) != Pen::Current
                || (!item["instance"].is_null() && item["instance"] != status["instance"])
            {
                return false;
            }
            if unresolved(result) {
                let epoch = item.get("epoch").unwrap_or(&item["upgrade"]["epoch"]);
                let attempt = item.get("attempt").unwrap_or(&item["upgrade"]["attempt"]);
                let target_matches = status["upgrade"]["target"]
                    .as_str()
                    .is_some_and(|target| files.same(Path::new(target), corral) == Ok(true));
                if epoch.is_null()
                    || attempt.is_null()
                    || *epoch != status["upgrade"]["epoch"]
                    // A user-requested recovery keeps the epoch and increases attempt.
                    || attempt.as_u64().zip(status["upgrade"]["attempt"].as_u64()).is_none_or(|(old, new)| new < old)
                    || !target_matches
                    || status["upgrade"]["result"] != "complete"
                {
                    return false;
                }
            }
        }
        if let Some(reminders) = item["after"].as_array() {
            for reminder in reminders.iter().filter(|r| r["result"] != "complete") {
                let Some(id) = reminder["request_id"].as_str() else {
                    return false;
                };
                let Ok(value) = read(
                    corral,
                    &["after", name, "--request-id", id],
                    Duration::from_secs(15),
                    cancel,
                ) else {
                    return false;
                };
                let Some(record) = value["records"]
                    .as_array()
                    .and_then(|records| records.iter().find(|r| r["request_id"] == id))
                else {
                    return false;
                };
                let terminal = matches!(
                    record["phase"].as_str(),
                    Some("confirmed" | "not_delivered" | "unknown" | "expired")
                ) || record["phase"] == "sent" && record["known"] != true;
                if terminal {
                    continue;
                }
                let Some(exe) = record["worker"]["exe"].as_str() else {
                    return false;
                };
                if record["handover"]["state"] != "complete"
                    || files.same(Path::new(exe), corral) != Ok(true)
                    || (reminder["result"] == "pending"
                        && (reminder["handover"]["token"].is_null()
                            || reminder["handover"]["token"] != record["handover"]["token"]))
                {
                    return false;
                }
            }
        }
    }
    true
}

/// The checks, the running upgrade and its last receipt.
pub struct Updates {
    results: Receiver<Check>,
    wake: Sender<Option<Receipt>>,
    cancel: Arc<AtomicBool>,
    latest: Option<Check>,
    /// A refresh asked for: checking until a check started after it arrives.
    requested: Option<Instant>,
    upgrading: Option<(SystemTime, Receiver<Receipt>)>,
    receipt: Option<Receipt>,
}
impl Updates {
    /// Checks now and every `every`.
    pub fn start(sources: Sources, every: Duration) -> Self {
        let (send, results) = mpsc::channel();
        let (wake, wait) = mpsc::channel::<Option<Receipt>>();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        thread::spawn(move || {
            let mut files = Files::default();
            let mut receipt = None;
            while !stop.load(Ordering::Relaxed) {
                let mut check = check(&sources, &mut files, Duration::from_secs(15), &stop);
                if let (Some(receipt), Ok(corral)) = (&receipt, &check.corral)
                    && verified(receipt, corral, &mut files, &stop)
                {
                    check.verified_receipt = Some(receipt.finished);
                }
                if send.send(check).is_err() {
                    break;
                }
                match wait.recv_timeout(every) {
                    Ok(Some(value)) => receipt = Some(value),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    _ => {}
                }
                while let Ok(next) = wait.try_recv() {
                    if let Some(value) = next {
                        receipt = Some(value);
                    }
                }
            }
        });
        Self {
            results,
            wake,
            cancel,
            latest: None,
            requested: None,
            upgrading: None,
            receipt: None,
        }
    }
    pub fn refresh(&mut self) {
        // Coalesce repeated clicks without moving the verification barrier indefinitely.
        if self.requested.is_none() {
            self.requested = Some(Instant::now());
            let _ = self.wake.send(None);
        }
    }
    /// Takes finished checks and upgrades.
    pub fn poll(&mut self) {
        for check in self.results.try_iter() {
            if self.requested.is_some_and(|at| check.started >= at) {
                self.requested = None;
            }
            self.latest = Some(check);
        }
        if let Some((_, receiver)) = &self.upgrading {
            let receipt = match receiver.try_recv() {
                Ok(receipt) => Some(receipt),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Receipt {
                    raw: Value::Null,
                    at: SystemTime::now(),
                    finished: Instant::now(),
                    outcome: Err(Failure::Unknown(
                        "Upgrade worker disconnected; do not retry automatically".into(),
                    )),
                    items: Vec::new(),
                    legacy: None,
                }),
            };
            if let Some(receipt) = receipt {
                let _ = self.wake.send(Some(receipt.clone()));
                self.receipt = Some(receipt);
                self.upgrading = None;
                self.requested = None;
                self.refresh();
            }
        }
    }
    pub fn latest(&self) -> Option<&Check> {
        self.latest.as_ref()
    }
    pub fn receipt(&self) -> Option<&Receipt> {
        self.receipt.as_ref()
    }
    pub fn upgrading(&self) -> bool {
        self.upgrading.is_some()
    }
    pub fn checking(&self) -> bool {
        self.requested.is_some()
    }
    fn receipt_verified(&self) -> bool {
        self.receipt.as_ref().is_some_and(|receipt| {
            self.latest
                .as_ref()
                .is_some_and(|check| check.verified_receipt == Some(receipt.finished))
        })
    }
    pub fn can_upgrade(&self) -> bool {
        if self.upgrading() || self.checking() {
            return false;
        }
        if let Some(receipt) = &self.receipt
            && !self.receipt_verified()
            && (matches!(receipt.outcome, Err(Failure::Unknown(_)))
                || receipt
                    .items
                    .iter()
                    .any(|i| unresolved(&i.pen) || i.after.iter().any(|a| unresolved(&a.result))))
        {
            return false;
        }
        self.latest.as_ref().is_some_and(|check| {
            check.corral.is_ok()
                && check.agents.as_ref().is_ok_and(|agents| {
                    agents.iter().any(|(_, p)| matches!(p, Pen::Outdated(_)))
                        || !self.receipt_verified()
                            && self.receipt.as_ref().is_some_and(|r| {
                                r.items
                                    .iter()
                                    .any(|i| i.after.iter().any(|a| a.result == "failed"))
                            })
                })
        })
    }
    /// Starts Upgrade all unless one is running or it is not offered.
    pub fn upgrade(&mut self) -> bool {
        if !self.can_upgrade() {
            return false;
        }
        let corral = self
            .latest
            .as_ref()
            .unwrap()
            .corral
            .as_ref()
            .unwrap()
            .clone();
        let (sender, receiver) = mpsc::channel();
        self.upgrading = Some((SystemTime::now(), receiver));
        thread::spawn(move || {
            let _ = sender.send(upgrade(&corral, Duration::from_secs(300)));
        });
        true
    }
    /// Something needs the user, or could not be confirmed: the dot beside Settings.
    pub fn attention(&self) -> bool {
        self.upgrading()
            || self.checking()
            || self.latest.as_ref().is_none_or(|check| {
                check.saddle != Saddle::Current
                    || check.agents.as_ref().map_or(true, |agents| {
                        agents.iter().any(|(_, p)| *p != Pen::Current)
                    })
            })
            || !self.receipt_verified()
                && self.receipt.as_ref().is_some_and(|r| {
                    r.outcome.is_err()
                        || r.items.iter().any(|i| {
                            !complete(&i.pen) || i.after.iter().any(|a| !complete(&a.result))
                        })
                })
    }
    /// Source, Installed, Running and, when the agents were read, how many use the installed
    /// Corral: each says only what its own evidence shows.
    fn versions(&self, check: &Check) -> Vec<Row> {
        let build = |build: &Result<Build, String>| match build {
            Ok(b) if b.modified => format!("{} + uncommitted changes", short(&b.revision)),
            Ok(b) => short(&b.revision).to_owned(),
            Err(e) => format!("build unknown ({e})"),
        };
        let mut rows = Vec::new();
        let (text, tone) = match &check.source {
            Err(e) => (format!("Unknown: {e}"), Tone::Unknown),
            Ok(source) => {
                let at = format!(
                    "{} at {} in {}",
                    source.branch,
                    short(&source.head),
                    crate::diagnostics::private(&source.checkout.display().to_string())
                );
                let modified = check.installed_build.as_ref().is_ok_and(|b| b.modified);
                match &source.newer {
                    Ok(0) if modified => (
                        format!("{at}; Installed was built from it with uncommitted changes"),
                        Tone::Unknown,
                    ),
                    Ok(0) => (format!("{at}; Installed has every commit"), Tone::Good),
                    Ok(n) => (
                        format!(
                            "{at}; {n} newer commit{} not in Installed; build and install to use {}",
                            if *n == 1 { "" } else { "s" },
                            if *n == 1 { "it" } else { "them" }
                        ),
                        Tone::Action,
                    ),
                    Err(e) => (
                        format!("{at}; cannot compare with Installed: {e}"),
                        Tone::Unknown,
                    ),
                }
            }
        };
        rows.push(Row::Item("Source".into(), text, tone));
        let (text, tone) = match (&check.installed, &check.installed_build) {
            (None, Err(e)) => (format!("Unknown: {e}"), Tone::Unknown),
            (_, Ok(_)) => (build(&check.installed_build), Tone::Good),
            (Some(_), Err(_)) => (build(&check.installed_build), Tone::Unknown),
        };
        rows.push(Row::Item("Installed".into(), text, tone));
        let (text, tone) = match &check.saddle {
            Saddle::Current => (
                format!("{}; same program as Installed", build(&check.running_build)),
                Tone::Good,
            ),
            Saddle::Reopen(_) => (
                format!(
                    "{}; differs from Installed. Reopen Saddle normally to apply; agents keep running",
                    build(&check.running_build)
                ),
                Tone::Action,
            ),
            Saddle::Unknown(e) => (format!("Cannot confirm: {e}"), Tone::Unknown),
        };
        rows.push(Row::Item("Running".into(), text, tone));
        if let Ok(agents) = &check.agents {
            let count = |f: fn(&Pen) -> bool| agents.iter().filter(|(_, p)| f(p)).count();
            let current = count(|p| *p == Pen::Current);
            let older = count(|p| matches!(p, Pen::Outdated(_)));
            let restart = count(|p| *p == Pen::NoProtocol);
            let other = agents.len() - current - older - restart;
            let (text, tone) = if agents.is_empty() {
                ("No agents".into(), Tone::Good)
            } else if current == agents.len() {
                (
                    format!("All {} use the installed Corral", agents.len()),
                    Tone::Good,
                )
            } else {
                let mut text = format!("{current} of {} use the installed Corral", agents.len());
                if older > 0 {
                    text += &format!(
                        "; {older} still on an older Corral{}",
                        if self.can_upgrade() {
                            " (Upgrade all)"
                        } else {
                            ""
                        }
                    );
                }
                if restart > 0 {
                    text += &format!("; {restart} need a restart");
                }
                if other > 0 {
                    text += &format!("; {other} pending or unconfirmed");
                }
                (
                    text,
                    if older + restart > 0 {
                        Tone::Action
                    } else {
                        Tone::Unknown
                    },
                )
            };
            rows.push(Row::Item("Agents".into(), text, tone));
        }
        rows
    }
    pub fn page(&self) -> Page {
        let mut rows = Vec::new();
        if self.upgrading() {
            rows.push(Row::Heading(
                "Upgrading — waiting for per-agent results".into(),
            ));
        } else if self.checking() || self.latest.is_none() {
            rows.push(Row::Heading("Checking installed updates…".into()));
        } else {
            rows.push(Row::Heading(
                if self.attention() {
                    "Updates need attention"
                } else {
                    "All installed updates are active"
                }
                .into(),
            ));
        }
        if let Some(check) = &self.latest {
            rows.extend(self.versions(check));
            match &check.agents {
                Err(e) => rows.push(Row::Item(
                    "Agents".into(),
                    format!("Cannot confirm: {e}"),
                    Tone::Unknown,
                )),
                Ok(agents) => {
                    for (name, pen) in agents {
                        let (text, tone) = match pen {
                            Pen::Current => ("Current".into(), Tone::Good),
                            Pen::Outdated(error) => (
                                format!(
                                    "Upgrade available{}",
                                    error
                                        .as_ref()
                                        .map(|e| format!("; last failure: {e}"))
                                        .unwrap_or_default()
                                ),
                                Tone::Action,
                            ),
                            Pen::NoProtocol => (
                                "needs_restart: no upgrade protocol; arrange a restart separately"
                                    .into(),
                                Tone::Action,
                            ),
                            Pen::Pending(state) => (
                                format!("Processing: {state}; Refresh to check"),
                                Tone::Action,
                            ),
                            Pen::Held { state, error } => (
                                format!(
                                    "{state}: {}; use corral recover {name} --exe {}",
                                    error.as_deref().unwrap_or("stream held"),
                                    check
                                        .corral
                                        .as_ref()
                                        .map(|p| shell_words::quote(&p.to_string_lossy())
                                            .into_owned())
                                        .unwrap_or_default()
                                ),
                                Tone::Bad,
                            ),
                            Pen::Unknown(e) => (
                                format!("Cannot confirm: {e}; inspect corral status {name}"),
                                Tone::Unknown,
                            ),
                        };
                        rows.push(Row::Item(name.clone(), text, tone));
                    }
                }
            }
        }
        if let Some(receipt) = &self.receipt {
            if self.receipt_verified() {
                rows.push(Row::Heading(
                    "Public verification complete; original receipt below".into(),
                ));
            }
            rows.push(Row::Heading("Last upgrade receipt".into()));
            if let Err(error) = &receipt.outcome {
                let (kind, error) = match error {
                    Failure::Unknown(e) => ("unknown — inspect public status before retrying", e),
                    Failure::Failed(e) => ("failed", e),
                };
                rows.push(Row::Item(kind.into(), error.clone(), Tone::Bad));
            }
            for item in &receipt.items {
                rows.push(Row::Item(
                    item.name.clone(),
                    format!(
                        "{}{}",
                        item.pen,
                        item.reason
                            .as_ref()
                            .map(|e| format!(": {e}"))
                            .unwrap_or_default()
                    ),
                    result_tone(&item.pen),
                ));
                for after in &item.after {
                    rows.push(Row::Item(
                        format!("after {}", after.request_id.as_deref().unwrap_or("unknown")),
                        format!(
                            "{}{}",
                            after.result,
                            after
                                .reason
                                .as_ref()
                                .map(|e| format!(": {e}"))
                                .unwrap_or_default()
                        ),
                        result_tone(&after.result),
                    ));
                }
            }
            if let Some(legacy) = &receipt.legacy {
                rows.push(Row::Item(
                    "Legacy reminders".into(),
                    legacy.clone(),
                    Tone::Unknown,
                ));
            }
        }
        Page {
            rows,
            upgrade: self.can_upgrade(),
        }
    }
}
fn complete(result: &str) -> bool {
    matches!(result, "complete" | "already_current")
}
fn unresolved(result: &str) -> bool {
    !matches!(
        result,
        "complete" | "already_current" | "failed" | "needs_restart"
    )
}
fn result_tone(result: &str) -> Tone {
    if complete(result) {
        Tone::Good
    } else {
        Tone::Bad
    }
}
impl Drop for Updates {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.wake.send(None);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Good,
    /// Something for the user to do.
    Action,
    Bad,
    Unknown,
}
#[derive(Clone, Debug)]
pub enum Row {
    Heading(String),
    Item(String, String, Tone),
}
/// What the Updates page shows.
#[derive(Clone, Debug, Default)]
pub struct Page {
    pub rows: Vec<Row>,
    /// Upgrade all is offered.
    pub upgrade: bool,
}
/// A revision as Git abbreviates it by default.
fn short(revision: &str) -> &str {
    revision.get(..7).unwrap_or(revision)
}
