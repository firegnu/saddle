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
    /// The configured corral; the default `corral` is the one installed beside saddle.
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
    verified_receipt: Option<Instant>,
}

/// File comparisons kept while neither file changes, so checks do not reread binaries.
#[derive(Default)]
pub struct Files(HashMap<(PathBuf, PathBuf), (Stamp, Stamp, bool)>);
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
    fn same(&mut self, a: &Path, b: &Path) -> Result<bool, String> {
        let (sa, sb) = (stamp(a)?, stamp(b)?);
        let key = (a.to_owned(), b.to_owned());
        if let Some((old_a, old_b, same)) = self.0.get(&key)
            && (*old_a, *old_b) == (sa, sb)
        {
            return Ok(*same);
        }
        let hash = |p: &Path| -> Result<_, String> {
            let mut file = fs::File::open(p).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut buffer = [0; 65536];
            loop {
                let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
            Ok(hash.finalize())
        };
        let same = sa.2 == sb.2 && hash(a)? == hash(b)?;
        if stamp(a)? != sa || stamp(b)? != sb {
            return Err("Program changed during check; Refresh again".into());
        }
        self.0.insert(key, (sa, sb, same));
        Ok(same)
    }
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
    result.corral = if sources.corral == "corral" {
        installed
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|p| executable(&p.with_file_name("corral").to_string_lossy()))
    } else {
        executable(&sources.corral)
    };
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
            let (text, tone) = match &check.saddle {
                Saddle::Current => ("Current".into(), Tone::Good),
                Saddle::Reopen(_) => (
                    "Reopen Saddle normally to apply; agents keep running".into(),
                    Tone::Action,
                ),
                Saddle::Unknown(e) => (format!("Cannot confirm: {e}"), Tone::Unknown),
            };
            rows.push(Row::Item("Saddle".into(), text, tone));
            match &check.corral {
                Ok(path) => rows.push(Row::Item(
                    "Installed Corral".into(),
                    path.display().to_string(),
                    Tone::Good,
                )),
                Err(e) => rows.push(Row::Item(
                    "Installed Corral".into(),
                    e.clone(),
                    Tone::Unknown,
                )),
            }
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
