//! Plugin-owned task storage and explicit transitions. No scheduler or completion gate.
use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn fail(code: &str, why: &str) -> anyhow::Error {
    crate::drover::TransitionError {
        code: code.into(),
        why: why.into(),
    }
    .into()
}
fn require(ok: bool, code: &str, why: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(fail(code, why)) }
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn random() -> Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn valid_id(s: &str) -> bool {
    s.starts_with('T') && s.len() > 1 && s[1..].bytes().all(|b| b.is_ascii_digit())
}
fn header() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^##\s+(?:(T\d+)\s+)?(.+?)\s*$").unwrap())
}
fn read(path: &Path, missing: bool) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if missing && e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("Cannot read {}", path.display())),
    }
}
fn signature(path: &Path) -> Result<Value> {
    match fs::metadata(path) {
        Ok(m) => Ok(json!([
            m.dev(),
            m.ino(),
            m.size(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec()
        ])),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Null),
        Err(e) => Err(e.into()),
    }
}
pub fn atomic_write(path: &Path, text: &str) -> Result<()> {
    let parent = path.parent().context("missing parent")?;
    fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(text.as_bytes())?;
    f.as_file().sync_all()?;
    f.persist(path).map_err(|e| e.error)?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}
struct Lock(fs::File);
impl Lock {
    fn new(path: &Path) -> Result<Self> {
        fs::create_dir_all(path.parent().context("missing lock parent")?)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("Cannot open task lock {}", path.display()))?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(fail("state_busy", "Another task write is in progress"));
        }
        Ok(Self(file))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

// Python's historical JSON representation is part of the legacy run identity, not a new format.
fn ascii_json(value: &Value, spaced: bool) -> String {
    fn string(s: &str) -> String {
        let encoded = serde_json::to_string(s).unwrap();
        let mut out = String::new();
        for c in encoded.chars() {
            if c.is_ascii() {
                out.push(c)
            } else {
                let mut units = [0; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
        }
        out
    }
    let comma = if spaced { ", " } else { "," };
    let colon = if spaced { ": " } else { ":" };
    match value {
        Value::String(s) => string(s),
        Value::Array(a) => format!(
            "[{}]",
            a.iter()
                .map(|v| ascii_json(v, spaced))
                .collect::<Vec<_>>()
                .join(comma)
        ),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}{colon}{}", string(k), ascii_json(v, spaced)))
                .collect::<Vec<_>>()
                .join(comma)
        ),
        _ => value.to_string(),
    }
}
fn blocks(text: &str) -> Vec<Value> {
    let mut out = vec![];
    let mut current: Option<Value> = None;
    let mut body = vec![];
    for line in text.lines() {
        if let Some(c) = header().captures(line) {
            if let Some(mut t) = current.take() {
                t["body"] = json!(body.join("\n").trim_matches('\n'));
                out.push(t);
            }
            body.clear();
            current = Some(json!({"id":c.get(1).map(|m|m.as_str()),"title":c[2].to_owned()}));
        } else if current.is_some() {
            body.push(line);
        }
    }
    if let Some(mut t) = current {
        t["body"] = json!(body.join("\n").trim_matches('\n'));
        out.push(t);
    }
    out
}
fn split_blocks(text: &str) -> (String, Vec<String>) {
    let mut pre = String::new();
    let mut out: Vec<String> = vec![];
    for line in text.split_inclusive('\n') {
        if header().is_match(line.trim_end_matches(['\r', '\n'])) {
            out.push(line.into());
        } else if let Some(last) = out.last_mut() {
            last.push_str(line)
        } else {
            pre.push_str(line)
        }
    }
    (pre, out)
}
fn join_blocks(pre: &str, raw: &[String]) -> String {
    let mut out = pre.to_owned();
    for (i, b) in raw.iter().enumerate() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n')
        }
        if i > 0 && !out.ends_with("\n\n") {
            out.push('\n')
        }
        out.push_str(b)
    }
    if !out.ends_with('\n') {
        out.push('\n')
    }
    out
}
fn text(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or_default().into()
}
fn fold(events: &[Value]) -> Result<Vec<Value>> {
    let mut tasks: Vec<Value> = vec![];
    for (index, e) in events.iter().enumerate() {
        require(
            e.is_object() && e["t"].as_f64().is_some_and(f64::is_finite),
            "state_invalid",
            "Invalid event time",
        )?;
        let ev = e["ev"].as_str().unwrap_or_default();
        if ev == "hold" {
            require(
                e["key"].as_str().is_some_and(|s| !s.is_empty()) && e["on"].is_boolean(),
                "state_invalid",
                "Invalid legacy hold",
            )?;
            continue;
        }
        let id = e["id"].as_str().unwrap_or_default();
        require(valid_id(id), "state_invalid", "Invalid task ID")?;
        for k in ["title", "body", "key", "sha", "main"] {
            require(
                e.get(k).is_none_or(Value::is_string),
                "state_invalid",
                "Invalid event text",
            )?;
        }
        let pos = tasks.iter().position(|t| t["id"] == id);
        if ev == "start" {
            require(
                !tasks
                    .iter()
                    .any(|t| matches!(t["status"].as_str(), Some("running" | "awaiting_release")))
                    && pos.is_none_or(|i| tasks[i]["status"] == "pending"),
                "state_invalid",
                "Invalid start",
            )?;
            let run = e
                .get("run_id")
                .filter(|v| !v.is_null() && **v != json!(""))
                .cloned()
                .unwrap_or_else(|| {
                    json!(format!(
                        "legacy:{}",
                        hash(ascii_json(
                            &json!([index, id, e["t"], e["sha"], e["main"]]),
                            true
                        ))
                    ))
                });
            require(run.is_string(), "state_invalid", "Invalid run identity")?;
            let mut previous = pos
                .and_then(|i| tasks[i]["previous_runs"].as_array().cloned())
                .unwrap_or_default();
            if let Some(i) = pos {
                let mut old = tasks[i].clone();
                old.as_object_mut().unwrap().remove("previous_runs");
                previous.push(old);
            }
            let task = json!({"id":id,"title":text(e,"title"),"body":text(e,"body"),"body_known":e.get("body").is_some()&&e.get("title").is_some(),"key":text(e,"key"),"start":text(e,"sha"),"main":text(e,"main"),"t0":e["t"],"status":"running","run_id":run,"previous_runs":previous});
            if let Some(i) = pos {
                tasks[i] = task
            } else {
                tasks.push(task)
            }
        } else if ev == "drop" {
            require(
                pos.is_none_or(|i| {
                    matches!(tasks[i]["status"].as_str(), Some("running" | "pending"))
                }),
                "state_invalid",
                "Invalid historical drop",
            )?;
            let i=pos.unwrap_or_else(||{tasks.push(json!({"id":id,"title":text(e,"title"),"body":"","key":text(e,"key"),"start":"","run_id":null}));tasks.len()-1});
            let t = &mut tasks[i];
            t["status"] = json!("dropped");
            t["reason"] = e.get("reason").cloned().unwrap_or(json!(""));
            t["t1"] = e["t"].clone();
        } else {
            let i = pos.ok_or_else(|| fail("state_invalid", "Event has no original start"))?;
            let t = &mut tasks[i];
            if matches!(ev, "submitted" | "accepted" | "returned") {
                require(
                    e["run_id"] == t["run_id"],
                    "state_invalid",
                    "Run identity mismatch",
                )?;
            }
            match ev {
                "submitted" | "done" => {
                    require(
                        t["status"] == "running" && (ev != "done" || e["gate"].is_boolean()),
                        "state_invalid",
                        "Invalid submission",
                    )?;
                    t["status"] = json!(if ev == "submitted" || e["gate"] == true {
                        "awaiting_release"
                    } else {
                        "done"
                    });
                    t["end"] = json!(text(e, "sha"));
                    t["t1"] = e["t"].clone();
                    t["submission"] = e.clone();
                    if let Some(r) = e.get("completion_record") {
                        t["completion_record"] = r.clone();
                    }
                }
                "accepted" | "go" => {
                    let legacy = ev == "go"
                        && t["status"] == "done"
                        && t.get("t2").is_none()
                        && t["submission"]["ev"] == "done"
                        && t["submission"]["gate"] == false;
                    require(
                        t["status"] == "awaiting_release" || legacy,
                        "state_invalid",
                        "Invalid acceptance",
                    )?;
                    t["status"] = json!("done");
                    t["t2"] = e["t"].clone();
                }
                "returned" | "return" => {
                    let r = &e["return_record"];
                    require(
                        matches!(t["status"].as_str(), Some("running" | "awaiting_release"))
                            && r["work_stopped"] == true
                            && r["reason"].as_str().is_some_and(|s| !s.trim().is_empty()),
                        "state_invalid",
                        "Invalid return",
                    )?;
                    t["status"] = json!("pending");
                    t["returned_at"] = e["t"].clone();
                    if t.get("return_history").is_none() {
                        t["return_history"] = json!([])
                    }
                    t["return_history"].as_array_mut().unwrap().push(r.clone());
                }
                _ => return Err(fail("state_invalid", "Unknown event")),
            }
        }
    }
    Ok(tasks)
}
fn pending(blocks: Vec<Value>, tasks: &[Value]) -> Vec<Value> {
    blocks
        .into_iter()
        .filter_map(|mut b| {
            if tasks.iter().any(|t| {
                t["status"] != "pending"
                    && if b["id"].is_null() {
                        !text(t, "key").is_empty() && t["key"] == b["title"]
                    } else {
                        t["id"] == b["id"]
                    }
            }) {
                return None;
            }
            if let Some(t) = tasks.iter().find(|t| t["id"] == b["id"]) {
                for (k, v) in t.as_object().unwrap() {
                    if !["id", "title", "body", "status"].contains(&k.as_str()) {
                        b[k] = v.clone();
                    }
                }
            }
            b["status"] = json!("pending");
            Some(b)
        })
        .collect()
}

struct Snapshot {
    repo: PathBuf,
    directory: PathBuf,
    config: String,
    conf: BTreeMap<String, String>,
    state: String,
    queue: String,
    signatures: BTreeMap<PathBuf, Value>,
    tasks: Vec<Value>,
    pending: Vec<Value>,
    paused: bool,
}
impl Snapshot {
    fn load(repo: &Path) -> Result<Self> {
        let repo = repo.canonicalize()?;
        let cp = repo.join(".drover.conf");
        let mut signatures = BTreeMap::new();
        signatures.insert(cp.clone(), signature(&cp)?);
        let config = read(&cp, false)?;
        let conf: BTreeMap<_, _> = config
            .lines()
            .filter_map(|l| {
                let (k, v) = l.trim().split_once('=')?;
                let k = k.trim();
                (!k.is_empty() && k.bytes().all(|b| b.is_ascii_uppercase() || b == b'_'))
                    .then(|| (k.to_owned(), v.trim().trim_matches(['\'', '"']).to_owned()))
            })
            .collect();
        let dir = conf
            .get("HANDOFF_DIR")
            .filter(|p| !p.is_empty() && !p.contains('\0'))
            .ok_or_else(|| fail("not_configured", "HANDOFF_DIR is missing"))?;
        let dir = crate::config::expand_home(dir);
        let directory = if dir.is_absolute() {
            dir
        } else {
            repo.join(dir)
        };
        let directory = directory
            .canonicalize()
            .with_context(|| format!("Task directory unavailable: {}", directory.display()))?;
        for name in ["tasks.state", "queue.md", "paused"] {
            let p = directory.join(name);
            signatures.insert(p.clone(), signature(&p)?);
        }
        let state = read(&directory.join("tasks.state"), true)?;
        let queue = read(&directory.join("queue.md"), true)?;
        let events: Vec<Value> = state
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| fail("state_invalid", "Invalid task event JSON"))?;
        let tasks = fold(&events)?;
        let pending = pending(blocks(&queue), &tasks);
        let paused = !signatures[&directory.join("paused")].is_null();
        let this = Self {
            repo,
            directory,
            config,
            conf,
            state,
            queue,
            signatures,
            tasks,
            pending,
            paused,
        };
        this.fresh()?;
        Ok(this)
    }
    fn fresh(&self) -> Result<()> {
        for (p, s) in &self.signatures {
            require(
                signature(p)? == *s,
                "target_changed",
                "State changed; refresh and confirm again",
            )?;
        }
        Ok(())
    }
    fn token(&self, action: &str, target: &Value) -> String {
        format!(
            "v2:{}",
            hash(
                json!([
                    self.repo,
                    self.directory,
                    self.config,
                    self.state,
                    self.queue,
                    self.signatures,
                    action,
                    target
                ])
                .to_string()
            )
        )
    }
    fn actions(&self, t: &Value, pos: Option<usize>) -> Value {
        let mut a = json!({});
        for (action, statuses) in [
            ("done", vec!["running"]),
            ("go", vec!["awaiting_release"]),
            ("return-to-pending", vec!["running", "awaiting_release"]),
        ] {
            if statuses.contains(&t["status"].as_str().unwrap_or_default()) {
                let reason = if action == "return-to-pending" && t["body_known"] != true {
                    Some("body_unavailable")
                } else {
                    None
                };
                a[action] = json!({"target_token":if reason.is_some(){None}else{Some(self.token(action,&t["id"]))},"unavailable_reason":reason});
            }
        }
        if let Some(pos) = pos {
            let target = &self.pending[pos - 1];
            let ambiguous = self.pending.iter().enumerate().any(|(i, t)| {
                i != pos - 1
                    && if t["id"].is_null() {
                        t["title"] == target["title"]
                    } else {
                        t["id"] == target["id"]
                    }
            });
            let reason = if ambiguous {
                Some("target_ambiguous")
            } else if self.tasks.iter().any(|t| t["status"] == "running") {
                Some("current_exists")
            } else if self.tasks.iter().any(|t| t["status"] == "awaiting_release") {
                Some("awaiting_release")
            } else if self.paused {
                Some("paused")
            } else {
                None
            };
            a["dispatch-pending"] = json!({"pos":pos,"target_token":if reason.is_some(){None}else{Some(self.token("dispatch-pending",&json!(pos)))},"unavailable_reason":reason});
        }
        a
    }
    fn public(&self, t: &Value, pos: Option<usize>) -> Value {
        let mut t = t.clone();
        t["actions"] = self.actions(&t, pos);
        t["notification_key"] = if t["status"] == "awaiting_release" {
            json!(ascii_json(
                &json!([self.repo, t["id"], t["run_id"], "awaiting_release"]),
                false
            ))
        } else {
            Value::Null
        };
        t
    }
    /// Whether this project's dispatches are recorded unless one says otherwise; off by default.
    fn record_default(&self) -> bool {
        self.conf.get("TELEMETRY_RECORD").is_some_and(|v| v == "on")
    }
    fn list(&self) -> Value {
        json!({"schema_version":2,"ok":true,"project":self.repo,"record_default":self.record_default(),"queue_token":self.token("queue",&Value::Null),"paused":self.paused,"current":self.tasks.iter().find(|t|t["status"]=="running").map(|t|self.public(t,None)),"awaiting":self.tasks.iter().find(|t|t["status"]=="awaiting_release").map(|t|self.public(t,None)),"pending":self.pending.iter().enumerate().map(|(i,t)|self.public(t,Some(i+1))).collect::<Vec<_>>(),"history":self.tasks.iter().rev().filter(|t|matches!(t["status"].as_str(),Some("done"|"dropped"))).map(|t|self.public(t,None)).collect::<Vec<_>>()})
    }
    fn append(&self, event: Value) -> Result<()> {
        self.fresh()?;
        let sep = if self.state.is_empty() || self.state.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        atomic_write(
            &self.directory.join("tasks.state"),
            &format!("{}{sep}{event}\n", self.state),
        )
    }
    fn next_id(&self) -> String {
        format!(
            "T{}",
            self.tasks
                .iter()
                .chain(blocks(&self.queue).iter())
                .filter_map(|t| t["id"].as_str()?.strip_prefix('T')?.parse::<u64>().ok())
                .max()
                .unwrap_or(0)
                + 1
        )
    }
}
pub fn list(repo: &Path) -> Result<Value> {
    Ok(Snapshot::load(repo)?.list())
}
fn git(repo: &Path, args: &[&str], cancel: &AtomicBool) -> Option<String> {
    let r = crate::command::run("git", args, Some(repo), Duration::from_secs(3), cancel).ok()?;
    r.status.success().then(|| {
        String::from_utf8_lossy(&r.stdout)
            .trim_end_matches('\n')
            .into()
    })
}
pub fn show(repo: &Path, id: &str, corral: &str, cancel: &AtomicBool) -> Result<Value> {
    let s = Snapshot::load(repo)?;
    let list = s.list();
    let task = [list["current"].clone(), list["awaiting"].clone()]
        .into_iter()
        .chain(list["pending"].as_array().unwrap().clone())
        .chain(list["history"].as_array().unwrap().clone())
        .find(|t| t["id"] == id)
        .ok_or_else(|| fail("task_not_found", "Task not found"))?;
    let before = git(repo, &["show-ref", "--head"], cancel);
    let dirty = git(
        repo,
        &["status", "--porcelain", "--untracked-files=no"],
        cancel,
    );
    let branches = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "--no-merged=main",
            "refs/heads/",
        ],
        cancel,
    );
    let head = git(repo, &["rev-parse", "--verify", "HEAD"], cancel);
    let main = git(repo, &["rev-parse", "--verify", "main"], cancel);
    let after = git(repo, &["show-ref", "--head"], cancel);
    let stale = before != after;
    let state = if stale {
        "stale"
    } else if [
        before.as_ref(),
        dirty.as_ref(),
        branches.as_ref(),
        head.as_ref(),
        main.as_ref(),
    ]
    .iter()
    .all(|x| x.is_some())
    {
        "available"
    } else {
        "unavailable"
    };
    let check = last_check(&s, &task, main.as_deref(), stale);
    let agent = s
        .conf
        .get("MAIN_AGENT")
        .filter(|s| !s.is_empty())
        .and_then(|a| {
            crate::command::run(
                corral,
                &["status", a],
                Some(repo),
                Duration::from_secs(10),
                cancel,
            )
            .ok()
        })
        .and_then(|r| serde_json::from_slice::<Value>(&r.stdout).ok());
    s.fresh()?;
    Ok(
        json!({"schema_version":2,"ok":true,"project":s.repo,"task":task,"evidence":{"scope":"repository_reference","controls_transition":false,"observed_at":now(),"git":{"state":state,"head_sha":head,"main_sha":main,"tracked_changes":dirty.unwrap_or_default().lines().collect::<Vec<_>>(),"unmerged_local_branches":branches.unwrap_or_default().lines().collect::<Vec<_>>()},"last_check":check},"agent_status":agent}),
    )
}
fn last_check(s: &Snapshot, t: &Value, main: Option<&str>, stale: bool) -> Value {
    let path = s.directory.join(".check-result");
    if !path.exists() {
        return json!({"state":"unknown","record":null,"reason":"no_record"});
    }
    let load = || -> Result<Value> {
        let raw = read(&path, false)?;
        ensure!(raw.len() <= 65536, "oversized record");
        let v: Value = serde_json::from_str(&raw)?;
        ensure!(
            v["t"]
                .as_f64()
                .is_some_and(|x| x.is_finite() && x <= now() + 5.),
            "invalid time"
        );
        for k in ["task", "cmd", "main", "why"] {
            ensure!(
                v[k].as_str().is_some_and(|s| !s.contains('\0')),
                "invalid text"
            )
        }
        ensure!(
            text(&v, "why").len() <= 4096
                && v.get("ok").is_some_and(|v| v.is_null() || v.is_boolean()),
            "invalid result"
        );
        Ok(v)
    };
    match load() {
        Ok(v) => {
            let current = v["run_id"] == t["run_id"]
                && v["main"].as_str() == main
                && v["task"] == t["id"]
                && v["cmd"].as_str() == Some(s.conf.get("CHECK_CMD").map_or("", String::as_str))
                && !stale;
            json!({"state":if !current{"stale"}else if v["ok"]==true{"passed"}else if v["ok"]==false{"failed"}else{"unknown"},"reason":if current{None}else{Some("reference_only_or_changed")},"record":v})
        }
        Err(_) => {
            json!({"state":"unavailable","record":null,"reason":"invalid_or_unreadable_record"})
        }
    }
}

pub fn execute(
    repo: &Path,
    operation: &crate::drover::Operation,
    corral: &str,
    cancel: &AtomicBool,
) -> Result<Value> {
    execute_expected(repo, operation, corral, cancel, None)
}
pub fn execute_expected(
    repo: &Path,
    operation: &crate::drover::Operation,
    corral: &str,
    cancel: &AtomicBool,
    expected_queue: Option<&str>,
) -> Result<Value> {
    let host = crate::telemetry::host();
    execute_with(
        repo,
        operation,
        corral,
        host.as_deref(),
        cancel,
        expected_queue,
    )
}
/// `host` is the Saddle executable for optional telemetry; without it nothing is recorded.
pub fn execute_with(
    repo: &Path,
    operation: &crate::drover::Operation,
    corral: &str,
    host: Option<&Path>,
    cancel: &AtomicBool,
    expected_queue: Option<&str>,
) -> Result<Value> {
    use crate::drover::{Operation as O, Transition};
    require(
        !cancel.load(Ordering::Relaxed),
        "cancelled",
        "Operation cancelled before execution",
    )?;
    let first = Snapshot::load(repo)?;
    let _lock = Lock::new(&first.directory.join(".tasks.lock"))?;
    let mut s = Snapshot::load(repo)?;
    require(
        first.directory == s.directory,
        "target_changed",
        "Project configuration changed",
    )?;
    if let Some(token) = expected_queue {
        require(
            token == s.token("queue", &Value::Null),
            "target_changed",
            "Queue changed; refresh and confirm again",
        )?;
    }
    match operation {
        O::Transition {
            project,
            id,
            run_id,
            token,
            action,
            reason,
        } => {
            require(
                Path::new(project).canonicalize()? == s.repo
                    && *token == s.token(action.command(), &json!(id)),
                "target_changed",
                "Task target changed",
            )?;
            let t = s
                .tasks
                .iter()
                .find(|t| t["id"] == *id)
                .cloned()
                .ok_or_else(|| fail("invalid_state", "No task run"))?;
            require(
                t["run_id"] == *run_id && action.allows(t["status"].as_str().unwrap_or_default()),
                "invalid_state",
                "This task cannot perform that transition",
            )?;
            let at = now();
            let mut event = json!({"ev":match action{Transition::Submit=>"submitted",Transition::Accept=>"accepted",Transition::Return=>"returned"},"id":id,"run_id":run_id,"t":at});
            if *action == Transition::Return {
                require(
                    !reason.trim().is_empty(),
                    "invalid_arguments",
                    "Return needs a reason and confirmation that work stopped",
                )?;
                require(
                    t["body_known"] == true,
                    "body_unavailable",
                    "Original task body unavailable",
                )?;
                let (pre, raw) = split_blocks(&s.queue);
                let kept: Vec<_> = raw
                    .into_iter()
                    .filter(|r| {
                        let b = &blocks(r)[0];
                        b["id"] != *id
                            && !(b["id"].is_null()
                                && b["title"]
                                    == if text(&t, "key").is_empty() {
                                        t["title"].clone()
                                    } else {
                                        t["key"].clone()
                                    })
                    })
                    .collect();
                let mut restored = vec![format!(
                    "## {id} {}\n{}\n\n",
                    text(&t, "title"),
                    text(&t, "body")
                )];
                restored.extend(kept);
                s.fresh()?;
                let path = s.directory.join("queue.md");
                atomic_write(&path, &join_blocks(&pre, &restored))?;
                s.signatures.insert(path.clone(), signature(&path)?);
                event["return_record"] = json!({"dispatched_at":t["t0"],"returned_at":at,"reason":reason,"work_stopped":true});
            }
            s.append(event)?;
            let to = match action {
                Transition::Submit => "awaiting_release",
                Transition::Accept => "done",
                Transition::Return => "pending",
            };
            // After Drover saved it: declared on this run's trace, if the run was recorded.
            let telemetry = crate::telemetry::transition(
                host,
                &crate::telemetry::binding(s.repo.to_str().unwrap_or_default(), id, run_id),
                None,
                t["status"].as_str().unwrap_or_default(),
                to,
                crate::telemetry::record_time(at),
                cancel,
            );
            Ok(
                json!({"schema_version":2,"ok":true,"task_id":id,"run_id":run_id,"state":to,"record":{"status":"recorded"},"telemetry":telemetry}),
            )
        }
        O::DispatchPending {
            project,
            pos,
            token,
            record,
        } => {
            require(
                Path::new(project).canonicalize()? == s.repo
                    && *token == s.token("dispatch-pending", &json!(pos)),
                "target_changed",
                "Dispatch target changed",
            )?;
            let t = s
                .pending
                .get(pos.checked_sub(1).unwrap_or(u64::MAX) as usize)
                .ok_or_else(|| fail("target_changed", "Pending task not found"))?;
            let a = s.actions(t, Some(*pos as usize));
            if let Some(why) = a["dispatch-pending"]["unavailable_reason"].as_str() {
                return Err(fail(why, "Cannot dispatch this task"));
            }
            let tid = t["id"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| s.next_id());
            let run = random()?;
            let body = text(t, "body");
            let message = format!(
                "TASK {tid}: {}{}",
                text(t, "title"),
                if body.is_empty() {
                    String::new()
                } else {
                    format!("\n\n{body}")
                }
            );
            let agent = s.conf.get("MAIN_AGENT").filter(|s| !s.is_empty());
            let sha = git(repo, &["rev-parse", "--verify", "HEAD"], cancel).unwrap_or_default();
            let main = git(repo, &["rev-parse", "--verify", "main"], cancel).unwrap_or_default();
            s.fresh()?;
            require(
                !cancel.load(Ordering::Relaxed),
                "cancelled",
                "Cancelled before dispatch",
            )?;
            // Recording identity comes first. Failing to get one here, before anything is sent,
            // is the only case where a recorded dispatch goes the plain way.
            let binding =
                crate::telemetry::binding(s.repo.to_str().unwrap_or_default(), &tid, &run);
            let mut telemetry = json!({"status":"not_requested","reason":null,"trace_id":null,"dispatch_id":null,"send":null,"transition":null});
            let context = if record.unwrap_or_else(|| s.record_default()) {
                let made = match (host, s.repo.to_str()) {
                    (None, _) => Err("host_unavailable".to_owned()),
                    (_, None) => Err("unavailable".to_owned()),
                    (Some(host), Some(_)) => crate::telemetry::prepare(
                        host,
                        &binding,
                        &format!("{tid} {}", text(t, "title")),
                        cancel,
                    )
                    .and_then(|identity| {
                        crate::telemetry::context_file(&identity)
                            .map(|file| (identity, file))
                            .map_err(|_| "unavailable".to_owned())
                    }),
                };
                match made {
                    Ok(made) => {
                        telemetry["status"] = json!("context");
                        telemetry["trace_id"] = json!(made.0.trace_id);
                        telemetry["dispatch_id"] = json!(made.0.dispatch_id);
                        Some(made)
                    }
                    Err(reason) => {
                        telemetry["status"] = json!("no_context");
                        telemetry["reason"] = json!(reason);
                        None
                    }
                }
            } else {
                None
            };
            require(
                !cancel.load(Ordering::Relaxed),
                "cancelled",
                "Cancelled before dispatch",
            )?;
            let message = match &context {
                Some((identity, _)) => format!(
                    "{message}{}",
                    crate::telemetry::appendix(identity, &tid, &run)
                ),
                None => message,
            };
            // `receipt` is None on the plain path; Some(None) when the agent entry's own
            // receipt is missing or does not pair, which leaves the delivery unknown.
            let (code, reply, receipt) = match (agent, &context, host) {
                (Some(agent), Some((_, (_, path))), Some(host)) => {
                    let sent = crate::telemetry::send(
                        host,
                        corral,
                        repo,
                        agent,
                        &message,
                        path,
                        Duration::from_secs(60),
                        cancel,
                    );
                    let reply =
                        serde_json::from_slice::<Value>(&sent.stdout).unwrap_or(Value::Null);
                    (sent.code, reply, Some(sent.receipt))
                }
                (Some(agent), _, _) => match crate::command::run_without_env(
                    corral,
                    &["send", agent, &message],
                    Some(repo),
                    &[crate::telemetry::HOST_VARIABLE.into()],
                    Duration::from_secs(60),
                    cancel,
                ) {
                    Ok(r) => (
                        r.status.code(),
                        serde_json::from_slice::<Value>(&r.stdout).unwrap_or(Value::Null),
                        None,
                    ),
                    Err(_) => (None, Value::Null, None),
                },
                (None, _, _) => (None, json!({}), None),
            };
            let executed = match &receipt {
                Some(Some(r)) => r["executed"].as_bool(),
                _ => None,
            };
            let confirmed = reply["confirmed"].as_bool();
            let merged = reply["merged_with_draft"].as_bool();
            // Only a paired executed=false says nothing was sent; exit codes alone never do.
            let status = if agent.is_none() {
                "not_sent"
            } else if matches!(receipt, Some(None)) {
                "unknown"
            } else if executed == Some(false) {
                "not_executed"
            } else if code == Some(0) && reply["ok"] == true && confirmed == Some(true) {
                "confirmed"
            } else if matches!(code, Some(0 | 3)) {
                "unconfirmed"
            } else if matches!(code, Some(2 | 6 | 7 | 8)) {
                "rejected"
            } else {
                "unknown"
            };
            telemetry["send"] = match &receipt {
                Some(Some(r)) => {
                    json!({"receipt":"matched","executed":r["executed"],"outcome":r["outcome"],"operation_id":r["operation_id"],"begin":r["begin"],"end":r["end"],"error":r.get("error"),"gaps":r["gaps"]})
                }
                Some(None) => json!({"receipt":"missing","executed":null}),
                None => Value::Null,
            };
            let ok = matches!(status, "not_sent" | "confirmed");
            let mut result = json!({"schema_version":2,"ok":ok,"task_id":tid,"run_id":null,"state":"pending","record":{"status":"not_attempted"},"manual_text":if agent.is_none(){Some(&message)}else{None},"delivery":{"status":status,"attempted":agent.is_some(),"corral_exit_code":code,"confirmed":confirmed,"merged_with_draft":merged}});
            if status == "not_executed" {
                result["error"] = json!({"code":"delivery_not_executed","why":format!("Saddle did not start the delivery ({}); nothing was sent. Check the cause before dispatching again", telemetry["send"]["error"].as_str().or(telemetry["send"]["outcome"]["kind"].as_str()).unwrap_or("no reason given"))});
            } else if !ok {
                result["error"] = json!({"code":match status{"unconfirmed"=>"delivery_unconfirmed","rejected"=>"send_rejected",_=>"delivery_unknown"},"why":"Check the controller and task record; do not automatically resend"})
            }
            if context.is_some() {
                telemetry["transition"] = json!({"status":"not_attempted","trace_id":null});
            }
            if agent.is_none() || matches!(status, "confirmed" | "unconfirmed") {
                let at = now();
                match s.append(json!({"ev":"start","id":tid,"run_id":run,"t":at,"title":t["title"],"body":body,"key":t["title"],"sha":sha,"main":main})){Ok(())=>{result["run_id"]=json!(run);result["state"]=json!("running");result["record"]=json!({"status":"recorded"});
                    // Saved first, then declared: the transition only follows Drover's own record.
                    if let Some((identity, _)) = &context {
                        telemetry["transition"] = crate::telemetry::transition(host, &binding, Some(&identity.trace_id), "pending", "running", crate::telemetry::record_time(at), cancel);
                    }
                },Err(e)=>{result["ok"]=json!(false);result["state"]=json!("unknown");result["record"]=json!({"status":"unknown"});result["error"]=json!({"code":"write_failed","why":format!("{e:#}; delivery may already have happened")});}}
            }
            result["telemetry"] = telemetry;
            Ok(result)
        }
        O::RecordDefault(on) => {
            let line = format!("TELEMETRY_RECORD={}\n", if *on { "on" } else { "off" });
            let mut found = false;
            let mut config: String = s
                .config
                .split_inclusive('\n')
                .map(|l| {
                    if l.trim().split_once('=').map(|(k, _)| k.trim()) == Some("TELEMETRY_RECORD") {
                        found = true;
                        line.clone()
                    } else {
                        l.to_owned()
                    }
                })
                .collect();
            if !found {
                if !config.is_empty() && !config.ends_with('\n') {
                    config.push('\n');
                }
                config.push_str(&line);
            }
            s.fresh()?;
            atomic_write(&s.repo.join(".drover.conf"), &config)?;
            Ok(
                json!({"ok":true,"record_default":on,"message":format!("Dispatches of this project are {} by default. Saddle's Telemetry recording switch still decides whether anything is recorded.", if *on {"recorded"} else {"not recorded"})}),
            )
        }
        O::Pause(on) => {
            s.fresh()?;
            let path = s.directory.join("paused");
            if *on {
                atomic_write(&path, "")?
            } else if path.exists() {
                fs::remove_file(&path)?;
            }
            Ok(
                json!({"ok":true,"message":if *on{"Paused: explicit dispatch disabled."}else{"Resumed: explicit dispatch enabled. Nothing automatically dispatched."}}),
            )
        }
        O::Add { title, body } => {
            validate_text(title, body)?;
            let id = s.next_id();
            let sep = if s.queue.is_empty() || s.queue.ends_with("\n\n") {
                ""
            } else if s.queue.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            s.fresh()?;
            atomic_write(
                &s.directory.join("queue.md"),
                &format!("{}{sep}## {id} {}\n{body}\n", s.queue, title.trim()),
            )?;
            Ok(json!({"ok":true,"task_id":id,"message":format!("Added {id}: {}",title.trim())}))
        }
        O::Edit { pending, index, .. }
        | O::Move { pending, index, .. }
        | O::Delete { pending, index } => {
            let actual: Vec<crate::drover::Task> = serde_json::from_value(json!(s.pending))?;
            require(
                *index < pending.len()
                    && actual
                        .iter()
                        .map(|t| (&t.id, &t.title, &t.body))
                        .eq(pending.iter().map(|t| (&t.id, &t.title, &t.body))),
                "target_changed",
                "Pending tasks changed; refresh before editing",
            )?;
            let (pre, mut raw) = split_blocks(&s.queue);
            let slots: Vec<_> = raw
                .iter()
                .enumerate()
                .filter(|(_, r)| !crate::core::pending(blocks(r), &s.tasks).is_empty())
                .map(|(i, _)| i)
                .collect();
            let slot = *slots
                .get(*index)
                .ok_or_else(|| fail("target_changed", "Pending position changed"))?;
            match operation {
                O::Edit { title, body, .. } => {
                    validate_text(title, body)?;
                    let b = &blocks(&raw[slot])[0];
                    let prefix = b["id"]
                        .as_str()
                        .map(|s| format!("{s} "))
                        .unwrap_or_default();
                    raw[slot] = format!("## {prefix}{}\n{body}\n", title.trim());
                }
                O::Move { to, .. } => {
                    require(
                        *to < slots.len(),
                        "invalid_arguments",
                        "Move position out of range",
                    )?;
                    let mut selected: Vec<_> = slots
                        .iter()
                        .map(|i| raw[*i].trim_end_matches(['\r', '\n']).to_owned() + "\n")
                        .collect();
                    let b = selected.remove(*index);
                    selected.insert(*to, b);
                    for (i, b) in slots.iter().zip(selected) {
                        raw[*i] = b;
                    }
                }
                O::Delete { .. } => {
                    let b = &blocks(&raw[slot])[0];
                    let id = b["id"]
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| s.next_id());
                    s.append(json!({"ev":"drop","id":id,"title":b["title"],"key":b["title"],"reason":"Deleted in saddle","t":now()}))?;
                    return Ok(json!({"ok":true,"message":format!("Dropped {id}")}));
                }
                _ => unreachable!(),
            };
            s.fresh()?;
            atomic_write(&s.directory.join("queue.md"), &join_blocks(&pre, &raw))?;
            Ok(json!({"ok":true,"message":"Pending queue updated"}))
        }
    }
}
fn validate_text(title: &str, body: &str) -> Result<()> {
    require(
        !title.trim().is_empty()
            && !title.contains(['\n', '\r', '\0'])
            && !body.contains('\0')
            && !body.lines().any(|l| header().is_match(l)),
        "invalid_arguments",
        "Use a nonempty single-line title; body headings must use ###, not ##",
    )
}
pub fn preference(set: Option<bool>) -> Result<crate::drover::Preference> {
    let path = crate::config::expand_home("~/.drover/notifications.json");
    let _lock = if set.is_some() {
        Some(Lock::new(&path.with_file_name("notifications.json.lock"))?)
    } else {
        None
    };
    let raw = read(&path, true)?;
    let v: Value = if !path.exists() {
        json!({"schema_version":1,"system_enabled":true,"revision":0})
    } else {
        serde_json::from_str(&raw)
            .map_err(|_| fail("preferences_invalid", "Invalid notification preference"))?
    };
    require(
        v["schema_version"] == 1
            && v["system_enabled"].is_boolean()
            && v["revision"].as_u64().is_some(),
        "preferences_invalid",
        "Invalid notification preference",
    )?;
    let mut p = crate::drover::Preference {
        system_enabled: v["system_enabled"].as_bool().unwrap(),
        revision: v["revision"].as_u64().unwrap(),
    };
    if let Some(enabled) = set {
        if p.system_enabled != enabled {
            p.system_enabled = enabled;
            p.revision = p
                .revision
                .checked_add(1)
                .context("preference revision overflow")?;
        }
        atomic_write(
            &path,
            &json!({"schema_version":1,"system_enabled":p.system_enabled,"revision":p.revision})
                .to_string(),
        )?;
    }
    Ok(p)
}

/// A single plugin owns observation for this user's data, including across Saddle instances.
pub struct PluginLease {
    _lock: Lock,
}
impl PluginLease {
    pub fn acquire() -> Result<Self> {
        Ok(Self {
            _lock: Lock::new(&crate::config::expand_home("~/.drover/.plugin-owner.lock"))?,
        })
    }
}
pub fn register(project: &Path, name: &str, agent: &str) -> Result<Value> {
    require(
        !name.is_empty()
            && name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
        "invalid_arguments",
        "Project name allows lowercase letters, digits and hyphens",
    )?;
    require(
        !agent.chars().any(char::is_control),
        "invalid_arguments",
        "Invalid main agent name",
    )?;
    let project = project.canonicalize()?;
    require(
        project.is_dir(),
        "invalid_arguments",
        "Project must be a directory",
    )?;
    let home = crate::config::expand_home("~/.drover");
    let _lock = Lock::new(&home.join(".projects.lock"))?;
    let cp = project.join(".drover.conf");
    let directory = home.join(name);
    if !cp.exists() {
        require(
            !directory.exists(),
            "project_exists",
            "This data directory already exists; use the existing project configuration",
        )?;
        fs::create_dir(&directory)?;
        atomic_write(
            &cp,
            &format!(
                "# Drover plugin data; lifecycle follows Saddle.\nHANDOFF_DIR={}\nMAIN_AGENT={agent}\nTASK_FILE_DIR=\n",
                directory.display()
            ),
        )?;
    }
    Snapshot::load(&project)?;
    let registry = home.join("projects");
    let raw = read(&registry, true)?;
    let path = project.display().to_string();
    if !raw.lines().any(|l| l.trim() == path) {
        let sep = if raw.is_empty() || raw.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        atomic_write(&registry, &format!("{raw}{sep}{path}\n"))?;
    }
    Ok(json!({"ok":true,"project":project,"message":"Project registered; no task dispatched"}))
}

#[path = "project.rs"]
pub mod project;
