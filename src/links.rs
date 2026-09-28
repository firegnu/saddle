//! Explicit task references and bounded, project-local read-only previews.
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub const LIMIT: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    File(PathBuf),
    Commit(String),
    Range(String, String),
    Agent {
        name: String,
        instance: Option<String>,
    },
}
#[derive(Clone, Debug)]
pub struct Link {
    pub target: Target,
    pub label: String,
    pub sources: Vec<String>,
    pub note: Option<String>,
}
impl Link {
    pub fn group(&self) -> &'static str {
        match self.target {
            Target::File(_) => "Files",
            Target::Commit(_) | Target::Range(..) => "Commits",
            Target::Agent { .. } => "Agents",
        }
    }
}
fn value(text: &str) -> Option<&str> {
    let text = text.trim();
    let text = if let Some(rest) = text.strip_prefix('`') {
        rest.strip_suffix('`')?
    } else {
        text
    };
    (!text.is_empty() && !text.contains('`')).then_some(text)
}
fn field(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();
    let line = ["- ", "* ", "+ "]
        .into_iter()
        .find_map(|p| line.strip_prefix(p))
        .unwrap_or(line);
    let line = if let Some((n, rest)) = line.split_once(". ") {
        if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
            rest
        } else {
            line
        }
    } else {
        line
    };
    let (name, target) = line.split_once([':', '：'])?;
    Some((name.trim(), value(target)?))
}
fn file_link(root: &Path, base: &Path, target: &str, source: &str) -> Link {
    let path = base.join(target);
    let note = if target.contains("://") || target.starts_with("mailto:") || target.starts_with('#')
    {
        Some("Unsupported URL or fragment".into())
    } else {
        checked_path(root, &path).err().map(|e| format!("{e:#}"))
    };
    let path = if note.is_none() {
        path.canonicalize().unwrap_or(path)
    } else {
        path
    };
    Link {
        target: Target::File(path),
        label: target.into(),
        sources: vec![source.into()],
        note,
    }
}
fn parse(root: &Path, base: &Path, text: &str, source: &str) -> (Vec<Link>, Vec<PathBuf>) {
    let mut links = Vec::new();
    let mut tasks = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some((marker, size)) = fence {
            if trimmed.chars().take_while(|c| *c == marker).count() >= size
                && trimmed.trim_matches(marker).trim().is_empty()
            {
                fence = None;
            }
            continue;
        }
        for marker in ['`', '~'] {
            let n = trimmed.chars().take_while(|c| *c == marker).count();
            if n >= 3 {
                fence = Some((marker, n));
            }
        }
        if fence.is_some() {
            continue;
        }
        if let Some((name, target)) = field(line) {
            match name {
                "Task file" | "任务文件" | "Review file" | "审查文件" | "Artifact" | "产物" =>
                {
                    let link = file_link(root, root, target, source);
                    if matches!(name, "Task file" | "任务文件")
                        && link.note.is_none()
                        && let Target::File(path) = &link.target
                    {
                        tasks.push(path.clone());
                    }
                    links.push(link);
                }
                "Commit" | "提交" if is_sha(target) => links.push(Link {
                    target: Target::Commit(target.into()),
                    label: target.into(),
                    sources: vec![source.into()],
                    note: None,
                }),
                "Agent" | "代理" => {
                    let (name, instance) =
                        target.split_once('|').map_or((target, None), |(n, i)| {
                            (
                                n.trim(),
                                i.trim().strip_prefix("instance=").filter(|v| {
                                    v.len() == 12 && v.bytes().all(|b| b.is_ascii_hexdigit())
                                }),
                            )
                        });
                    if !name.is_empty()
                        && !name.chars().any(char::is_whitespace)
                        && !name.starts_with('-')
                    {
                        links.push(Link {
                            target: Target::Agent {
                                name: name.into(),
                                instance: instance.map(str::to_owned),
                            },
                            label: name.into(),
                            sources: vec![source.into()],
                            note: instance.is_none().then(|| "Identity unknown".into()),
                        });
                    }
                }
                _ => {}
            }
        }
        // The supported inline form has one destination, optionally enclosed in <...>.
        let mut rest = line;
        while let Some(start) = rest.find('[') {
            rest = &rest[start + 1..];
            let Some((_, tail)) = rest.split_once("](") else {
                break;
            };
            let Some((destination, tail)) = tail.split_once(')') else {
                break;
            };
            rest = tail;
            let destination = destination.trim();
            let destination = destination
                .strip_prefix('<')
                .and_then(|v| v.strip_suffix('>'))
                .or_else(|| (!destination.chars().any(char::is_whitespace)).then_some(destination));
            if let Some(destination) = destination.filter(|s| !s.is_empty()) {
                links.push(file_link(root, base, destination, source));
            }
        }
    }
    (links, tasks)
}
pub fn is_sha(text: &str) -> bool {
    (7..=64).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}
/// Only a Task file field in the task body grants the additional discovery layer.
pub fn collect(root: &Path, body: &str) -> Vec<Link> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    let root = canonical.as_path();
    let (mut links, tasks) = parse(root, root, body, "Task text");
    let mut seen = std::collections::HashSet::new();
    for task in tasks {
        if !seen.insert(task.clone()) {
            continue;
        }
        match read_file(root, &task) {
            Ok(text) => {
                let source = task
                    .strip_prefix(root)
                    .unwrap_or(&task)
                    .display()
                    .to_string();
                links.extend(parse(root, task.parent().unwrap_or(root), &text, &source).0);
            }
            Err(error) => {
                for link in &mut links {
                    if link.target == Target::File(task.clone()) {
                        link.note = Some(format!("{error:#}"));
                    }
                }
            }
        }
    }
    let mut merged: Vec<Link> = Vec::new();
    for link in links {
        if let Some(old) = merged.iter_mut().find(|old| old.target == link.target) {
            for source in link.sources {
                if !old.sources.contains(&source) {
                    old.sources.push(source);
                }
            }
        } else {
            merged.push(link);
        }
    }
    merged.sort_by_key(|l| match l.target {
        Target::File(_) => 0,
        Target::Commit(_) | Target::Range(..) => 1,
        Target::Agent { .. } => 2,
    });
    merged
}
fn upstream_internal(path: &Path) -> bool {
    let parts: Vec<_> = path.iter().map(|v| v.to_string_lossy()).collect();
    parts
        .iter()
        .any(|p| p.starts_with(".drover") || p.starts_with(".corral"))
        || parts.windows(2).any(|p| {
            matches!(
                p[0].as_ref(),
                ".config" | ".cache" | "share" | "state" | "Application Support"
            ) && matches!(p[1].as_ref(), "drover" | "corral")
        })
}
fn checked_path(root: &Path, path: &Path) -> Result<(PathBuf, PathBuf)> {
    ensure!(
        !upstream_internal(path),
        "Upstream internal files are not readable"
    );
    let root = root.canonicalize().context("Project root unavailable")?;
    let path = path.canonicalize().context("File unavailable")?;
    ensure!(path.starts_with(&root), "Outside project root");
    ensure!(
        !upstream_internal(&path),
        "Upstream internal files are not readable"
    );
    Ok((root, path))
}
pub fn read_file(root: &Path, path: &Path) -> Result<String> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
        os::unix::ffi::OsStrExt,
    };
    let (root, path) = checked_path(root, path)?;
    // Walk the resolved path via directory descriptors: a symlink swap cannot redirect a read.
    let root_c = CString::new(root.as_os_str().as_bytes())?;
    let fd = unsafe {
        libc::open(
            root_c.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    ensure!(
        fd >= 0,
        "Project root unavailable: {}",
        std::io::Error::last_os_error()
    );
    let mut fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let parts: Vec<_> = path.strip_prefix(&root)?.iter().collect();
    for (i, part) in parts.iter().enumerate() {
        let name = CString::new(part.as_bytes())?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | libc::O_CLOEXEC
            | if i + 1 < parts.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        let next = unsafe { libc::openat(fd.as_raw_fd(), name.as_ptr(), flags) };
        ensure!(
            next >= 0,
            "File unavailable: {}",
            std::io::Error::last_os_error()
        );
        fd = unsafe { OwnedFd::from_raw_fd(next) };
    }
    let file = File::from(fd);
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), "Not a regular file");
    ensure!(
        metadata.len() <= LIMIT as u64,
        "File too large (limit 1 MiB)"
    );
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= LIMIT, "File too large (limit 1 MiB)");
    ensure!(!bytes.contains(&0), "Binary file is not supported");
    String::from_utf8(bytes).context("Not UTF-8 text")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub project: String,
    pub seq: u64,
    pub body: String,
    pub range: Option<(String, String)>,
}
impl Key {
    pub fn same_task(&self, other: &Self) -> bool {
        self.project == other.project && self.seq == other.seq && self.body == other.body
    }
}
pub enum Output {
    Listing(Vec<Link>),
    Text(String),
    Range(Vec<(String, String)>),
}
struct Job {
    result: std::sync::mpsc::Receiver<Result<Output>>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Job {
    fn start(
        work: impl FnOnce(&std::sync::atomic::AtomicBool) -> Result<Output> + Send + 'static,
    ) -> Self {
        let (tx, result) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let quitting = cancel.clone();
        let thread = Some(std::thread::spawn(move || {
            let _ = tx.send(work(&quitting));
        }));
        Self {
            result,
            cancel,
            thread,
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub struct Reading {
    pub title: String,
    pub text: String,
    pub scroll: usize,
    pub commits: Vec<(String, String)>,
    pub selected: usize,
}
#[derive(Default)]
pub struct State {
    pub key: Option<Key>,
    pub entries: Vec<Link>,
    pub selected: usize,
    pub top: usize,
    pub reading: Option<Reading>,
    pub message: String,
    pub rows: Vec<(ratatui::layout::Rect, usize)>,
    job: Option<Job>,
    pub agent_request: Option<AgentRequest>,
    pub checking_agent: Option<AgentRequest>,
}
impl State {
    pub fn sync(&mut self, key: Key) {
        if self.key.as_ref() == Some(&key) {
            return;
        }
        if self.key.as_ref().is_some_and(|old| old.same_task(&key)) {
            self.key = Some(key);
            self.update_range();
            return;
        }
        *self = Self::default();
        self.key = Some(key.clone());
        self.message = "Loading links…".into();
        self.job = Some(Job::start(move |_| {
            Ok(Output::Listing(collect(Path::new(&key.project), &key.body)))
        }));
    }
    fn update_range(&mut self) {
        let selected = self.entries.get(self.selected).map(|l| l.target.clone());
        self.entries
            .retain(|l| !matches!(l.target, Target::Range(..)));
        if let Some((start, end)) = self.key.as_ref().and_then(|k| k.range.clone()) {
            let index = self
                .entries
                .iter()
                .position(|l| l.group() == "Agents")
                .unwrap_or(self.entries.len());
            self.entries.insert(
                index,
                Link {
                    label: format!("Recorded range {start}..{end}"),
                    target: Target::Range(start, end),
                    sources: vec!["drover show · recorded Git endpoints".into()],
                    note: None,
                },
            );
        }
        self.selected = selected
            .and_then(|target| self.entries.iter().position(|l| l.target == target))
            .unwrap_or(self.selected)
            .min(self.entries.len().saturating_sub(1));
    }
    pub fn tick(&mut self) {
        let result = self.job.as_ref().and_then(|j| j.result.try_recv().ok());
        if let Some(result) = result {
            self.job = None;
            self.message.clear();
            match result {
                Ok(Output::Listing(entries)) => {
                    self.entries = entries;
                    self.update_range();
                }
                Ok(Output::Range(commits)) => {
                    if let Some(reading) = &mut self.reading {
                        reading.text = format!(
                            "Recorded range only; commits are not necessarily owned by this task.\n{}",
                            commits
                                .iter()
                                .map(|(sha, subject)| format!("{sha} {subject}"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        );
                        if commits.is_empty() {
                            reading.text.push_str("\nNo commits in recorded range");
                        }
                        reading.commits = commits;
                    }
                }
                Ok(Output::Text(text)) => {
                    if let Some(reading) = &mut self.reading {
                        reading.text = text;
                    }
                }
                Err(error) => {
                    if let Some(reading) = &mut self.reading {
                        reading.text = format!("{error:#}");
                    } else {
                        self.message = format!("{error:#}");
                    }
                }
            }
        }
    }
    pub fn open(&mut self) {
        if self.job.is_some() || self.checking_agent.is_some() {
            return;
        }
        let link = if let Some(reading) = &self.reading {
            let Some((sha, subject)) = reading.commits.get(reading.selected) else {
                return;
            };
            Link {
                target: Target::Commit(sha.clone()),
                label: subject.clone(),
                sources: vec![],
                note: None,
            }
        } else {
            let Some(link) = self.entries.get(self.selected).cloned() else {
                return;
            };
            link
        };
        if let Some(note) = link.note {
            self.message = note;
            return;
        }
        let Some(key) = self.key.clone() else {
            return;
        };
        if let Target::Agent {
            name,
            instance: Some(instance),
        } = link.target
        {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let request = AgentRequest {
                key,
                name,
                instance,
                seq: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            };
            self.agent_request = Some(request.clone());
            self.checking_agent = Some(request);
            self.message = "Checking agent…".into();
            return;
        }
        self.reading = Some(Reading {
            title: link.label,
            text: "Loading…".into(),
            scroll: 0,
            commits: Vec::new(),
            selected: 0,
        });
        self.job = Some(Job::start(move |cancel| match link.target {
            Target::File(path) => read_file(Path::new(&key.project), &path).map(Output::Text),
            Target::Commit(sha) => {
                crate::link_git::commit(Path::new(&key.project), &sha, cancel).map(Output::Text)
            }
            Target::Range(start, end) => {
                crate::link_git::range(Path::new(&key.project), &start, &end, cancel)
                    .map(Output::Range)
            }
            _ => bail!("Target unavailable"),
        }));
    }
    pub fn back(&mut self) {
        self.job = None;
        self.reading = None;
        self.message.clear();
    }
    pub fn scroll(&mut self, delta: isize) {
        if let Some(reading) = &mut self.reading {
            if reading.commits.is_empty() {
                reading.scroll = reading.scroll.saturating_add_signed(delta);
            } else {
                reading.selected = reading
                    .selected
                    .saturating_add_signed(delta)
                    .min(reading.commits.len() - 1);
            }
        } else {
            self.selected = self
                .selected
                .saturating_add_signed(delta)
                .min(self.entries.len().saturating_sub(1));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRequest {
    pub key: Key,
    pub name: String,
    pub instance: String,
    pub seq: u64,
}
/// Public status must confirm the original instance even when a terminal is already open.
pub fn check_agent(
    status: &serde_json::Value,
    request: &AgentRequest,
    locally_attached: bool,
) -> Result<()> {
    ensure!(
        status["instance"].as_str() == Some(&request.instance),
        "agent identity changed or unavailable"
    );
    ensure!(
        status["state"].as_str().is_some_and(|s| s != "exited")
            && status["starting"] != true
            && status["incompatible"] != true
            && status.get("error").is_none_or(serde_json::Value::is_null),
        "agent has exited or is unavailable"
    );
    let attached = status["attached"]
        .as_u64()
        .context("agent attachment state unavailable")?;
    ensure!(
        attached == u64::from(locally_attached),
        "agent is attached elsewhere or local attachment changed"
    );
    Ok(())
}
