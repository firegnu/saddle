//! Generic installer for built-in plugin resources and its ownership record. Only the
//! declared kind, names and bytes are interpreted; plugin business never enters here.
use saddle_core_plugin::{Manifest, Resource, ResourceFile, ResourceKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

struct Agent {
    id: &'static str,
    label: &'static str,
    home: &'static str,
}
/// User-level skill locations. Agent homes are never created on the user's behalf.
const AGENTS: [Agent; 2] = [
    Agent {
        id: "claude-code",
        label: "Claude Code",
        home: ".claude",
    },
    Agent {
        id: "codex",
        label: "Codex",
        home: ".agents",
    },
];
const SKILL: &str = "SKILL.md";
const RECORD: &str = "plugin-resources.json";
const LOCK: &str = "plugin-resources.lock";
const MAX_FILE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Absent,
    Missing,
    Foreign,
    OwnedCurrent,
    OwnedOutdated,
    Newer,
    Incomplete,
    Modified,
    Unreadable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Installed,
    Updated,
    Unchanged,
    Removed,
    Conflict,
    Skipped,
    Busy,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Enable,
    Sync,
    Startup,
    Remove,
}

#[derive(Clone, Debug, Serialize)]
pub struct TargetStatus {
    pub agent: &'static str,
    #[serde(skip)]
    pub label: &'static str,
    pub path: PathBuf,
    pub state: Class,
    pub revision: Option<u32>,
    pub reason: Option<String>,
    pub detail: Option<String>,
    /// Files verified as this plugin's own, unmodified install.
    #[serde(skip)]
    files: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResourceStatus {
    pub kind: &'static str,
    pub name: String,
    pub revision: u32,
    pub fingerprint: Option<String>,
    pub error: Option<String>,
    pub targets: Vec<TargetStatus>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SetupFileStatus {
    pub label: String,
    pub resource: String,
    pub file: String,
    /// Absolute paths in targets this plugin installed; empty means not installed.
    pub paths: Vec<PathBuf>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub resources: Vec<ResourceStatus>,
    pub setup_files: Vec<SetupFileStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TargetReceipt {
    pub agent: &'static str,
    #[serde(skip)]
    pub label: &'static str,
    pub path: PathBuf,
    pub before: Class,
    pub result: Outcome,
    pub reason: Option<String>,
    pub detail: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResourceReceipt {
    pub name: String,
    pub revision: u32,
    pub error: Option<String>,
    pub targets: Vec<TargetReceipt>,
}
/// One structure for management messages and tests (design §6.5).
#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub plugin: String,
    pub enabled: bool,
    pub trigger: Trigger,
    pub error: Option<String>,
    pub resources: Vec<ResourceReceipt>,
}
impl Receipt {
    /// Startup only reports what actually changed or failed; states stay in the status view.
    pub fn notable(&self) -> bool {
        self.error.is_some()
            || self.resources.iter().any(|r| {
                r.error.is_some()
                    || r.targets.iter().any(|t| {
                        matches!(t.result, Outcome::Updated | Outcome::Busy | Outcome::Failed)
                    })
            })
    }
    /// The switch result and the resource result are separate sentences.
    pub fn summary(&self) -> String {
        let mut out = match self.trigger {
            Trigger::Enable => "Enabled.",
            Trigger::Sync => "Synced resources.",
            Trigger::Startup => "Startup resource check.",
            Trigger::Remove => "Remove resources.",
        }
        .to_owned();
        if let Some(e) = &self.error {
            out += &format!(" Resources unavailable ({e}); nothing was written.");
        }
        for r in &self.resources {
            if let Some(e) = &r.error {
                out += &format!(" {} not installed: {e}.", r.name);
                continue;
            }
            let targets: Vec<_> = r
                .targets
                .iter()
                .map(|t| format!("{} {}", t.label, t.text()))
                .collect();
            out += &format!(" {} r{}: {}.", r.name, r.revision, targets.join(" · "));
        }
        let results = || self.resources.iter().flat_map(|r| &r.targets);
        if results().any(|t| t.result == Outcome::Conflict) {
            out += " Nothing was overwritten.";
        }
        if results().any(|t| matches!(t.result, Outcome::Failed | Outcome::Busy)) {
            out += if self.trigger == Trigger::Remove {
                " Retry with Remove resources."
            } else {
                " Retry with Sync resources."
            };
        }
        out
    }
}
impl TargetReceipt {
    fn text(&self) -> String {
        let reason = self.reason.as_deref().unwrap_or_default();
        match self.result {
            Outcome::Installed => "installed".into(),
            Outcome::Updated => "updated".into(),
            Outcome::Unchanged if self.before == Class::Newer => "newer version kept".into(),
            Outcome::Unchanged if self.before == Class::Missing => {
                "missing (Sync resources reinstalls)".into()
            }
            Outcome::Unchanged => "unchanged".into(),
            Outcome::Removed if self.detail.is_some() => "removed (your files kept)".into(),
            Outcome::Removed => "removed".into(),
            Outcome::Conflict => match reason {
                "symlink" => "conflict (existing link kept)".into(),
                "directory" | "exists" => "conflict (existing directory kept)".into(),
                "file" => "conflict (existing file kept)".into(),
                "incomplete" => "incomplete (not repaired)".into(),
                _ => "modified (not overwritten)".into(),
            },
            Outcome::Skipped => "skipped (agent not found)".into(),
            Outcome::Busy => "busy (another Saddle is updating resources)".into(),
            Outcome::Failed => format!("failed ({})", self.detail.as_deref().unwrap_or(reason)),
        }
    }
}
impl TargetStatus {
    /// The ownership record has an entry for this target.
    pub fn recorded(&self) -> bool {
        !matches!(
            self.state,
            Class::Absent | Class::Foreign | Class::Unreadable
        )
    }
    pub fn text(&self) -> String {
        match self.state {
            Class::Absent if self.reason.as_deref() == Some("agent_absent") => {
                "Not installed (agent not found)".into()
            }
            Class::Absent => "Not installed".into(),
            Class::Missing => "Missing — Sync resources reinstalls".into(),
            Class::Foreign => match self.reason.as_deref() {
                Some("symlink") => "Conflict — existing link kept".into(),
                Some("directory") => "Conflict — existing directory kept".into(),
                _ => "Conflict — existing file kept".into(),
            },
            Class::OwnedCurrent => "Installed".into(),
            Class::OwnedOutdated => format!(
                "Installed r{} — update pending",
                self.revision.unwrap_or_default()
            ),
            Class::Newer => format!(
                "Newer r{} kept — not downgraded",
                self.revision.unwrap_or_default()
            ),
            Class::Incomplete => "Incomplete — check, delete the directory, then Sync".into(),
            Class::Modified => "Modified — not overwritten".into(),
            Class::Unreadable => "Unreadable — see saddle plugin status".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Version {
    revision: u32,
    files: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entry {
    plugin: String,
    resource: String,
    agent: String,
    path: PathBuf,
    revision: u32,
    files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending: Option<Version>,
    updated_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct Record {
    version: u32,
    targets: Vec<Entry>,
}
impl Default for Record {
    fn default() -> Self {
        Self {
            version: 1,
            targets: vec![],
        }
    }
}

pub struct Resources {
    home: Option<PathBuf>,
    state: Option<PathBuf>,
}
impl Resources {
    /// `state` is the directory holding the ownership record and its lock.
    pub fn new(home: PathBuf, state: PathBuf) -> Self {
        Self {
            home: Some(home),
            state: Some(state),
        }
    }
    /// Same state directory rule as telemetry; per user, independent of `--config`.
    pub fn from_environment() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let state = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.as_ref().map(|h| h.join(".local/state")))
            .map(|s| s.join("saddle"));
        Self {
            home: home.filter(|h| h.is_absolute()),
            state,
        }
    }
    /// Read-only: never creates the state directory, a lock, or anything under HOME.
    pub fn status(&self, m: &Manifest) -> Result<Status, String> {
        let record = if m.resources.is_empty() {
            Record::default()
        } else {
            self.read()?
        };
        let resources: Vec<_> = m
            .resources
            .iter()
            .map(|r| {
                let mut status = ResourceStatus {
                    kind: kind(r),
                    name: r.name.into(),
                    revision: r.revision,
                    fingerprint: None,
                    error: None,
                    targets: vec![],
                };
                match validate(m, r) {
                    Err(e) => status.error = Some(e),
                    Ok(shipped) => {
                        status.fingerprint = Some(fingerprint(&shipped.files));
                        match self.plan(m, r, &shipped, &record) {
                            Err(e) => status.error = Some(e),
                            Ok(plan) => {
                                status.targets = plan
                                    .into_iter()
                                    .map(|p| TargetStatus {
                                        agent: p.agent.id,
                                        label: p.agent.label,
                                        revision: p.seen.revision,
                                        files: match p.seen.class {
                                            Class::OwnedCurrent
                                            | Class::OwnedOutdated
                                            | Class::Newer => p
                                                .seen
                                                .version
                                                .map(|v| v.files.into_keys().collect())
                                                .unwrap_or_default(),
                                            _ => vec![],
                                        },
                                        reason: if p.seen.class == Class::Absent && !p.present {
                                            Some("agent_absent".into())
                                        } else {
                                            p.seen.reason
                                        },
                                        state: p.seen.class,
                                        detail: p.seen.detail,
                                        path: p.path,
                                    })
                                    .collect()
                            }
                        }
                    }
                }
                status
            })
            .collect();
        let setup_files = m
            .setup_files
            .iter()
            .map(|f| {
                let declared = m
                    .resources
                    .iter()
                    .any(|r| r.name == f.resource && r.files.iter().any(|x| x.path == f.path));
                let resource = resources.iter().find(|r| r.name == f.resource);
                SetupFileStatus {
                    label: f.label.into(),
                    resource: f.resource.into(),
                    file: f.path.into(),
                    paths: resource
                        .filter(|_| declared)
                        .map(|r| {
                            r.targets
                                .iter()
                                .filter(|t| t.files.iter().any(|x| x == f.path))
                                .map(|t| t.path.join(f.path))
                                .collect()
                        })
                        .unwrap_or_default(),
                    error: (!declared || resource.is_none_or(|r| r.error.is_some()))
                        .then(|| "invalid_setup_file".into()),
                }
            })
            .collect();
        Ok(Status {
            resources,
            setup_files,
        })
    }
    /// Classify every target of one validated resource against the given record.
    fn plan(
        &self,
        m: &Manifest,
        r: &Resource,
        shipped: &Version,
        record: &Record,
    ) -> Result<Vec<Planned>, String> {
        let home = self.home.as_ref().ok_or("home_unavailable")?;
        Ok(AGENTS
            .iter()
            .map(|agent| {
                let agent_home = home.join(agent.home);
                let path = agent_home.join("skills").join(r.name);
                let key = Key {
                    plugin: m.id,
                    resource: r.name,
                    agent: agent.id,
                };
                let entry = record.find(&key, &path);
                Planned {
                    agent,
                    // The agent's own home may be a link (dotfile managers); targets never are.
                    present: fs::metadata(&agent_home).is_ok_and(|m| m.is_dir()),
                    seen: classify(&path, entry, shipped),
                    key,
                    path,
                }
            })
            .collect())
    }
    pub fn apply(&self, m: &Manifest, trigger: Trigger, enabled: bool) -> Receipt {
        let mut receipt = Receipt {
            plugin: m.id.into(),
            enabled,
            trigger,
            error: None,
            resources: vec![],
        };
        if m.resources.is_empty() {
            return receipt;
        }
        let shipped: Vec<_> = m.resources.iter().map(|r| validate(m, r)).collect();
        let plans = |record: &Record| -> Result<Vec<_>, String> {
            m.resources
                .iter()
                .zip(&shipped)
                .map(|(r, s)| match s {
                    Ok(s) => self.plan(m, r, s, record).map(Some),
                    Err(_) => Ok(None),
                })
                .collect()
        };
        let mut record = match self.read() {
            Ok(record) => record,
            Err(e) => {
                receipt.error = Some(format!("record unreadable: {e}"));
                return receipt;
            }
        };
        let mut planned = match plans(&record) {
            Ok(p) => p,
            Err(e) => {
                receipt.error = Some(e);
                return receipt;
            }
        };
        // Only a write takes the lock (and creates the state directory); then everything is
        // classified again under the lock, so another Saddle cannot interleave.
        let writes = planned
            .iter()
            .flatten()
            .flatten()
            .any(|p| act(trigger, p.seen.class, p.present).writes());
        let mut busy = false;
        let _lock = if writes {
            match self.lock() {
                Ok(Some(lock)) => {
                    match self.read().and_then(|r| {
                        let p = plans(&r)?;
                        Ok((r, p))
                    }) {
                        Ok((r, p)) => (record, planned) = (r, p),
                        Err(e) => {
                            receipt.error = Some(format!("record unreadable: {e}"));
                            return receipt;
                        }
                    }
                    Some(lock)
                }
                Ok(None) => {
                    busy = true;
                    None
                }
                Err(e) => {
                    receipt.error = Some(format!("lock unavailable: {e}"));
                    return receipt;
                }
            }
        } else {
            None
        };
        for ((r, shipped), plan) in m.resources.iter().zip(&shipped).zip(planned) {
            let mut out = ResourceReceipt {
                name: r.name.into(),
                revision: r.revision,
                error: shipped.as_ref().err().cloned(),
                targets: vec![],
            };
            let (Ok(shipped), Some(plan)) = (shipped, plan) else {
                receipt.resources.push(out);
                continue;
            };
            for p in plan {
                let mut t = TargetReceipt {
                    agent: p.agent.id,
                    label: p.agent.label,
                    path: p.path.clone(),
                    before: p.seen.class,
                    result: Outcome::Unchanged,
                    reason: p.seen.reason.clone(),
                    detail: p.seen.detail.clone(),
                };
                let done = if busy {
                    Done::new(Outcome::Busy, Some("busy"), None)
                } else {
                    match act(trigger, p.seen.class, p.present) {
                        Act::Report(outcome) => Done::new(outcome, None, None),
                        Act::Skip => Done::new(Outcome::Skipped, Some("agent_absent"), None),
                        Act::Install => self.install(&mut record, &p, shipped, r.files),
                        Act::Upgrade => self.upgrade(&mut record, &p, shipped, r.files),
                        Act::Remove => self.remove(&mut record, &p),
                        Act::Forget => {
                            record.set(&p.key, &p.path, None);
                            match self.save(&record) {
                                Ok(()) => Done::new(Outcome::Removed, None, None),
                                Err(e) => Done::failed("record_write", e),
                            }
                        }
                    }
                };
                t.result = done.outcome;
                if let Some(reason) = done.reason {
                    t.reason = Some(reason.into());
                }
                if done.detail.is_some() {
                    t.detail = done.detail;
                }
                out.targets.push(t);
            }
            receipt.resources.push(out);
        }
        receipt
    }
    fn read(&self) -> Result<Record, String> {
        let state = self.state.as_ref().ok_or("state directory unavailable")?;
        let file = match fs::File::open(state.join(RECORD)) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Record::default()),
            Err(e) => return Err(e.to_string()),
        };
        let mut bytes = Vec::new();
        file.take(MAX_FILE + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE {
            return Err("record too large".into());
        }
        let record: Record = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if record.version != 1 {
            return Err("unsupported record version".into());
        }
        Ok(record)
    }
    fn save(&self, record: &Record) -> io::Result<()> {
        step("record")?;
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| io::Error::other("no state"))?;
        let mut tmp = tempfile::NamedTempFile::new_in(state)?;
        tmp.write_all(&serde_json::to_vec_pretty(record)?)?;
        tmp.as_file().sync_all()?;
        tmp.persist(state.join(RECORD)).map_err(|e| e.error)?;
        Ok(())
    }
    /// Non-blocking; `None` means another Saddle holds it.
    fn lock(&self) -> io::Result<Option<fs::File>> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| io::Error::other("no state"))?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(state)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(state.join(LOCK))?;
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(Some(file));
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

#[derive(Clone, Copy)]
struct Key<'a> {
    plugin: &'a str,
    resource: &'a str,
    agent: &'a str,
}
struct Planned {
    agent: &'static Agent,
    present: bool,
    key: Key<'static>,
    path: PathBuf,
    seen: Seen,
}
struct Seen {
    class: Class,
    /// The complete version on disk for owned classes (pending already resolved).
    version: Option<Version>,
    revision: Option<u32>,
    reason: Option<String>,
    detail: Option<String>,
}
impl Seen {
    fn new(class: Class, reason: Option<&str>, detail: Option<String>) -> Self {
        Self {
            class,
            version: None,
            revision: None,
            reason: reason.map(Into::into),
            detail,
        }
    }
}
struct Done {
    outcome: Outcome,
    reason: Option<&'static str>,
    detail: Option<String>,
}
impl Done {
    fn new(outcome: Outcome, reason: Option<&'static str>, detail: Option<String>) -> Self {
        Self {
            outcome,
            reason,
            detail,
        }
    }
    fn failed(reason: &'static str, detail: impl std::fmt::Display) -> Self {
        Self::new(Outcome::Failed, Some(reason), Some(detail.to_string()))
    }
}
enum Act {
    Report(Outcome),
    Skip,
    Install,
    Upgrade,
    Remove,
    Forget,
}
impl Act {
    fn writes(&self) -> bool {
        matches!(
            self,
            Act::Install | Act::Upgrade | Act::Remove | Act::Forget
        )
    }
}
/// The action table of design §6.4.
fn act(trigger: Trigger, class: Class, present: bool) -> Act {
    use Class::*;
    match (trigger, class) {
        (_, Unreadable) => Act::Report(Outcome::Failed),
        (_, Foreign | Modified | Incomplete) => Act::Report(Outcome::Conflict),
        (Trigger::Remove, Absent) => Act::Report(Outcome::Unchanged),
        (Trigger::Remove, Missing) => Act::Forget,
        (Trigger::Remove, OwnedCurrent | OwnedOutdated | Newer) => Act::Remove,
        (_, OwnedOutdated) => Act::Upgrade,
        (Trigger::Startup, _) => Act::Report(Outcome::Unchanged),
        (_, Absent | Missing) if !present => Act::Skip,
        (_, Absent | Missing) => Act::Install,
        (_, OwnedCurrent | Newer) => Act::Report(Outcome::Unchanged),
    }
}

impl Record {
    fn find(&self, key: &Key, path: &Path) -> Option<&Entry> {
        self.targets.iter().find(|e| {
            e.plugin == key.plugin
                && e.resource == key.resource
                && e.agent == key.agent
                && e.path == path
        })
    }
    fn set(&mut self, key: &Key, path: &Path, entry: Option<Entry>) {
        self.targets.retain(|e| {
            !(e.plugin == key.plugin
                && e.resource == key.resource
                && e.agent == key.agent
                && e.path == path)
        });
        self.targets.extend(entry);
    }
}
impl Entry {
    fn new(key: &Key, path: &Path, installed: &Version, pending: Option<&Version>) -> Self {
        Self {
            plugin: key.plugin.into(),
            resource: key.resource.into(),
            agent: key.agent.into(),
            path: path.into(),
            revision: installed.revision,
            files: installed.files.clone(),
            pending: pending.cloned(),
            updated_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}
impl Version {
    /// No files: the state before an install and after a removal.
    fn none() -> Self {
        Self {
            revision: 0,
            files: BTreeMap::new(),
        }
    }
}

impl Resources {
    /// Parent check → pending record → mkdir → files (SKILL.md last) → final record.
    fn install(
        &self,
        record: &mut Record,
        p: &Planned,
        new: &Version,
        files: &[ResourceFile],
    ) -> Done {
        let skills = p.path.parent().expect("target has a parent");
        let created_skills = match fs::metadata(skills) {
            Ok(m) if m.is_dir() => false,
            Ok(_) => return Done::failed("skills_not_directory", skills.display()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if let Err(e) = step("skills").and_then(|()| fs::create_dir(skills)) {
                    return Done::failed("mkdir", e);
                }
                true
            }
            Err(e) => return Done::failed("unreadable", e),
        };
        let tidy = || {
            if created_skills {
                let _ = fs::remove_dir(skills);
            }
        };
        let previous = record.find(&p.key, &p.path).cloned();
        record.set(
            &p.key,
            &p.path,
            Some(Entry::new(&p.key, &p.path, &Version::none(), Some(new))),
        );
        if let Err(e) = self.save(record) {
            record.set(&p.key, &p.path, previous);
            tidy();
            return Done::failed("record_write", e);
        }
        if let Err(e) = step("mkdir").and_then(|()| fs::create_dir(&p.path)) {
            record.set(&p.key, &p.path, previous);
            let _ = self.save(record);
            tidy();
            return if e.kind() == io::ErrorKind::AlreadyExists {
                Done::new(Outcome::Conflict, Some("exists"), None)
            } else {
                Done::failed("mkdir", e)
            };
        }
        let mut written = vec![];
        for f in ordered(files) {
            if let Err(e) = step(&format!("write:{}", f.path))
                .and_then(|()| write_atomic(&p.path.join(f.path), f.bytes))
            {
                let kept = rollback(&p.path, &written, new, &BTreeMap::new());
                if kept.is_empty() {
                    let _ = fs::remove_dir(&p.path);
                    record.set(&p.key, &p.path, previous);
                }
                // Otherwise the pending entry stays: the target reads as incomplete.
                let _ = self.save(record);
                tidy();
                return Done::failed("write", failure(f.path, e, &kept));
            }
            written.push(f.path);
        }
        self.finish(record, p, new, Outcome::Installed)
    }
    /// Verify the old set under the lock, then pending → changed files → old-only deletes →
    /// SKILL.md → final record. Old bytes stay in memory for this round's rollback.
    fn upgrade(
        &self,
        record: &mut Record,
        p: &Planned,
        new: &Version,
        files: &[ResourceFile],
    ) -> Done {
        let old = p.seen.version.as_ref().expect("owned version");
        let mut bytes = BTreeMap::new();
        for (name, hash) in &old.files {
            match read_file(&p.path.join(name)) {
                Ok(Some(b)) if sha256(&b) == *hash => {
                    bytes.insert(name.as_str(), b);
                }
                _ => return Done::new(Outcome::Conflict, Some("modified"), Some(name.clone())),
            }
        }
        record.set(
            &p.key,
            &p.path,
            Some(Entry::new(&p.key, &p.path, old, Some(new))),
        );
        if let Err(e) = self.save(record) {
            record.set(
                &p.key,
                &p.path,
                Some(Entry::new(&p.key, &p.path, old, None)),
            );
            return Done::failed("record_write", e);
        }
        let changed: Vec<_> = ordered(files)
            .filter(|f| old.files.get(f.path) != new.files.get(f.path))
            .collect();
        let (mut written, mut deleted) = (vec![], vec![]);
        let mut current = "";
        let result = (|| -> io::Result<()> {
            for f in changed.iter().filter(|f| f.path != SKILL) {
                current = f.path;
                unchanged(&p.path, f.path, old)?;
                step(&format!("write:{}", f.path))?;
                write_atomic(&p.path.join(f.path), f.bytes)?;
                written.push(f.path);
            }
            for name in old.files.keys().filter(|n| !new.files.contains_key(*n)) {
                current = name;
                unchanged(&p.path, name, old)?;
                step(&format!("delete:{name}"))?;
                fs::remove_file(p.path.join(name))?;
                deleted.push(name.as_str());
            }
            for f in changed.iter().filter(|f| f.path == SKILL) {
                current = f.path;
                unchanged(&p.path, f.path, old)?;
                step(&format!("write:{}", f.path))?;
                write_atomic(&p.path.join(f.path), f.bytes)?;
                written.push(f.path);
            }
            Ok(())
        })();
        if let Err(e) = result {
            let mut kept = rollback(&p.path, &written, new, &bytes);
            kept.extend(restore(&p.path, &deleted, &bytes));
            if kept.is_empty() {
                record.set(
                    &p.key,
                    &p.path,
                    Some(Entry::new(&p.key, &p.path, old, None)),
                );
            }
            let _ = self.save(record);
            return Done::failed("write", failure(current, e, &kept));
        }
        self.finish(record, p, new, Outcome::Updated)
    }
    /// SKILL.md first, so the directory stops being a skill at once; only unmodified own
    /// files; the directory only when empty. User files and foreign targets stay.
    fn remove(&self, record: &mut Record, p: &Planned) -> Done {
        let version = p.seen.version.as_ref().expect("owned version");
        let mut bytes = BTreeMap::new();
        for (name, hash) in &version.files {
            match read_file(&p.path.join(name)) {
                Ok(Some(b)) if sha256(&b) == *hash => {
                    bytes.insert(name.as_str(), b);
                }
                _ => return Done::new(Outcome::Conflict, Some("modified"), Some(name.clone())),
            }
        }
        record.set(
            &p.key,
            &p.path,
            Some(Entry::new(&p.key, &p.path, version, Some(&Version::none()))),
        );
        if let Err(e) = self.save(record) {
            record.set(
                &p.key,
                &p.path,
                Some(Entry::new(&p.key, &p.path, version, None)),
            );
            return Done::failed("record_write", e);
        }
        let order: Vec<&str> = std::iter::once(SKILL)
            .chain(
                version
                    .files
                    .keys()
                    .map(String::as_str)
                    .filter(|n| *n != SKILL),
            )
            .collect();
        let mut deleted = vec![];
        let mut current = "";
        let result = (|| -> io::Result<()> {
            for name in &order {
                current = name;
                unchanged(&p.path, name, version)?;
                step(&format!("delete:{name}"))?;
                fs::remove_file(p.path.join(name))?;
                deleted.push(*name);
            }
            Ok(())
        })();
        if let Err(e) = result {
            let kept = restore(&p.path, &deleted, &bytes);
            if kept.is_empty() {
                record.set(
                    &p.key,
                    &p.path,
                    Some(Entry::new(&p.key, &p.path, version, None)),
                );
            }
            let _ = self.save(record);
            return Done::failed("delete", failure(current, e, &kept));
        }
        let left = match fs::remove_dir(&p.path) {
            Ok(()) => None,
            Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {
                Some("directory kept with files that are not Saddle's".to_owned())
            }
            Err(e) => Some(format!("directory kept: {e}")),
        };
        record.set(&p.key, &p.path, None);
        if let Err(e) = self.save(record) {
            return Done::failed("record_write", e);
        }
        Done::new(Outcome::Removed, None, left)
    }
    fn finish(&self, record: &mut Record, p: &Planned, new: &Version, outcome: Outcome) -> Done {
        record.set(
            &p.key,
            &p.path,
            Some(Entry::new(&p.key, &p.path, new, None)),
        );
        match self.save(record) {
            Ok(()) => Done::new(outcome, None, None),
            // The pending entry already describes the complete new set.
            Err(e) => Done::failed(
                "record_write",
                format!("files written; record left pending: {e}"),
            ),
        }
    }
}

/// Undo files this round wrote, but only those still holding this round's bytes.
fn rollback(
    dir: &Path,
    written: &[&str],
    new: &Version,
    old: &BTreeMap<&str, Vec<u8>>,
) -> Vec<String> {
    let mut kept = vec![];
    for name in written.iter().rev() {
        let path = dir.join(name);
        let ours = hash_file(&path).ok().flatten().as_ref() == new.files.get(*name);
        let undone = ours
            && match old.get(name) {
                Some(bytes) => write_atomic(&path, bytes).is_ok(),
                None => fs::remove_file(&path).is_ok(),
            };
        if !undone {
            kept.push((*name).to_owned());
        }
    }
    kept
}
/// Recreate files this round deleted, unless something else now occupies the path.
fn restore(dir: &Path, deleted: &[&str], old: &BTreeMap<&str, Vec<u8>>) -> Vec<String> {
    deleted
        .iter()
        .rev()
        .filter(|name| {
            exists(&dir.join(name)) || write_atomic(&dir.join(name), &old[*name]).is_err()
        })
        .map(|name| (*name).to_owned())
        .collect()
}
fn failure(name: &str, e: io::Error, kept: &[String]) -> String {
    if kept.is_empty() {
        format!("{name}: {e}")
    } else {
        format!("{name}: {e}; left as found: {}", kept.join(", "))
    }
}
/// The file at `name` is still exactly the recorded one (or still absent).
fn unchanged(dir: &Path, name: &str, version: &Version) -> io::Result<()> {
    if hash_file(&dir.join(name))?.as_ref() == version.files.get(name) {
        Ok(())
    } else {
        Err(io::Error::other("changed during this operation"))
    }
}

/// Design §6.3, lstat/O_NOFOLLOW only. Pending entries resolve to whichever version is
/// complete on disk; anything else is incomplete and never repaired automatically.
fn classify(path: &Path, entry: Option<&Entry>, shipped: &Version) -> Seen {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut seen = Seen::new(
                if entry.is_some() {
                    Class::Missing
                } else {
                    Class::Absent
                },
                None,
                None,
            );
            seen.revision = entry.map(|e| e.revision);
            return seen;
        }
        Err(e) => return Seen::new(Class::Unreadable, Some("unreadable"), Some(e.to_string())),
    };
    let (kind, link) = if meta.file_type().is_symlink() {
        let target = fs::read_link(path)
            .map(|t| format!("-> {}", t.display()))
            .unwrap_or_else(|e| e.to_string());
        ("symlink", Some(target))
    } else if meta.is_dir() {
        ("directory", None)
    } else {
        ("file", None)
    };
    let Some(entry) = entry else {
        return Seen::new(Class::Foreign, Some(kind), link);
    };
    if kind != "directory" {
        return Seen::new(Class::Modified, Some(kind), link);
    }
    let installed = Version {
        revision: entry.revision,
        files: entry.files.clone(),
    };
    let version = match &entry.pending {
        None => {
            let problems = check(path, &installed, None);
            if !problems.is_empty() {
                return Seen::new(Class::Modified, Some("modified"), Some(problems.join("; ")));
            }
            installed
        }
        Some(next) => {
            let to_next = check(path, next, Some(&installed));
            if to_next.is_empty() {
                next.clone()
            } else if check(path, &installed, Some(next)).is_empty() {
                installed
            } else {
                return Seen::new(
                    Class::Incomplete,
                    Some("incomplete"),
                    Some(to_next.join("; ")),
                );
            }
        }
    };
    if version.files.is_empty() {
        return Seen::new(
            Class::Incomplete,
            Some("incomplete"),
            Some("no complete version on disk".into()),
        );
    }
    let class = match version.revision.cmp(&shipped.revision) {
        std::cmp::Ordering::Less => {
            let taken: Vec<_> = shipped
                .files
                .keys()
                .filter(|n| !version.files.contains_key(*n) && exists(&path.join(n)))
                .cloned()
                .collect();
            if !taken.is_empty() {
                return Seen::new(
                    Class::Modified,
                    Some("user_file"),
                    Some(format!("would overwrite {}", taken.join(", "))),
                );
            }
            Class::OwnedOutdated
        }
        std::cmp::Ordering::Greater => Class::Newer,
        std::cmp::Ordering::Equal if version.files == shipped.files => Class::OwnedCurrent,
        std::cmp::Ordering::Equal => {
            return Seen::new(
                Class::Modified,
                Some("content_differs"),
                Some("same revision with different content".into()),
            );
        }
    };
    Seen {
        class,
        revision: Some(version.revision),
        version: Some(version),
        reason: None,
        detail: None,
    }
}
/// Whole-set comparison: every file of `version` is a regular file with its hash, and the
/// paths only `other` has are absent. Returns the differences.
fn check(dir: &Path, version: &Version, other: Option<&Version>) -> Vec<String> {
    let mut problems = vec![];
    for (name, hash) in &version.files {
        match hash_file(&dir.join(name)) {
            Ok(Some(h)) if h == *hash => {}
            Ok(Some(_)) => problems.push(format!("{name} changed")),
            Ok(None) => problems.push(format!("{name} missing")),
            Err(e) => problems.push(format!("{name} {e}")),
        }
    }
    for name in other
        .into_iter()
        .flat_map(|o| o.files.keys())
        .filter(|n| !version.files.contains_key(*n))
    {
        if exists(&dir.join(name)) {
            problems.push(format!("{name} present"));
        }
    }
    problems
}

/// Plain names only: no separators, no leading dot (temporary names), no `..`.
fn plain(name: &str) -> bool {
    !name.is_empty() && name.len() <= 255 && !name.starts_with('.') && !name.contains(['/', '\0'])
}
fn validate(m: &Manifest, r: &Resource) -> Result<Version, String> {
    match r.kind {
        ResourceKind::AgentSkill => {}
        _ => return Err("invalid_resource: unsupported kind".into()),
    }
    if !plain(r.name) || m.resources.iter().filter(|o| o.name == r.name).count() != 1 {
        return Err("invalid_resource: name".into());
    }
    if r.revision == 0 {
        return Err("invalid_resource: revision".into());
    }
    let mut files = BTreeMap::new();
    for f in r.files {
        if !plain(f.path)
            || f.bytes.len() as u64 > MAX_FILE
            || files.insert(f.path.to_owned(), sha256(f.bytes)).is_some()
        {
            return Err(format!("invalid_resource: file {}", f.path));
        }
    }
    if !files.contains_key(SKILL) {
        return Err("invalid_resource: SKILL.md missing".into());
    }
    Ok(Version {
        revision: r.revision,
        files,
    })
}
/// SHA-256 of the canonical `[[path, sha256], ...]` JSON list sorted by path.
fn fingerprint(files: &BTreeMap<String, String>) -> String {
    let list: Vec<[&str; 2]> = files
        .iter()
        .map(|(p, h)| [p.as_str(), h.as_str()])
        .collect();
    format!(
        "sha256:{}",
        sha256(&serde_json::to_vec(&list).expect("plain strings"))
    )
}

fn kind(r: &Resource) -> &'static str {
    match r.kind {
        ResourceKind::AgentSkill => "agent_skill",
        _ => "unknown",
    }
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_file(path: &Path) -> io::Result<Option<String>> {
    Ok(read_file(path)?.map(|b| sha256(&b)))
}
/// Conservative: anything but a clean "not found" counts as occupied.
fn exists(path: &Path) -> bool {
    !matches!(fs::symlink_metadata(path), Err(e) if e.kind() == io::ErrorKind::NotFound)
}
/// SKILL.md last when writing, so an agent only sees a complete skill directory.
fn ordered(files: &[ResourceFile]) -> impl Iterator<Item = &ResourceFile> {
    let skill = |f: &&ResourceFile| f.path == SKILL;
    files
        .iter()
        .filter(move |f| !skill(f))
        .chain(files.iter().filter(skill))
}
/// Regular files only; links, FIFOs and other types are reported, never followed.
fn read_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
        Ok(m) if m.file_type().is_symlink() => return Err(io::Error::other("is a link")),
        Ok(m) if !m.is_file() => return Err(io::Error::other("is not a regular file")),
        Ok(_) => {}
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(io::Error::other("is too large"));
    }
    Ok(Some(bytes))
}
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| io::Error::other("no parent"))?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    // Dot-prefixed and never named SKILL.md, so a half-written file is not a skill.
    let mut tmp = tempfile::Builder::new()
        .prefix(&format!(".{name}."))
        .suffix(".saddle-tmp")
        .tempfile_in(dir)?;
    tmp.as_file()
        .set_permissions(fs::Permissions::from_mode(0o644))?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(not(test))]
fn step(_: &str) -> io::Result<()> {
    Ok(())
}
#[cfg(test)]
type Fault = Box<dyn FnMut(&str) -> io::Result<()>>;
#[cfg(test)]
thread_local! {
    static FAULT: std::cell::RefCell<Option<Fault>> = const { std::cell::RefCell::new(None) };
}
/// Test-only failure injection at named write/delete steps.
#[cfg(test)]
fn step(name: &str) -> io::Result<()> {
    FAULT.with_borrow_mut(|f| f.as_mut().map_or(Ok(()), |f| f(name)))
}

#[cfg(test)]
mod tests;
