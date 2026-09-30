//! Bounded, read-only Git snapshots. Git owns path, attribute and rename semantics.
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    io::Read,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::PermissionsExt,
    },
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

const FILE_LIMIT: usize = 512 * 1024;
const TOTAL_LIMIT: usize = 4 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    All,
    Unstaged,
    Staged,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Unstaged => "Unstaged",
            Self::Staged => "Staged",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    pub path: PathBuf,
    pub status: String,
    pub patch: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub root: PathBuf,
    pub files: Vec<FileDiff>,
}
struct Git<'a> {
    cwd: PathBuf,
    cancel: &'a AtomicBool,
    overrides: Vec<String>,
}
struct Output {
    status: ExitStatus,
    bytes: Vec<u8>,
    error: Vec<u8>,
}
impl Git<'_> {
    fn run(&self, args: &[OsString], limit: usize) -> Result<Output> {
        ensure!(!self.cancel.load(Ordering::Relaxed), "Cancelled");
        let mut c = Command::new("git");
        for (key, _) in std::env::vars_os().filter(|(k, _)| k.as_bytes().starts_with(b"GIT_")) {
            c.env_remove(key);
        }
        c.current_dir(&self.cwd)
            .args([
                "--no-lazy-fetch",
                "--no-optional-locks",
                "--literal-pathspecs",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "color.ui=false",
                "-c",
                "diff.suppressBlankEmpty=false",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for value in &self.overrides {
            c.args(["-c", value]);
        }
        let mut child = c.args(args).spawn().context("Start Git")?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let read = |pipe: Box<dyn Read + Send>, cap: usize| {
            thread::spawn(move || {
                let mut bytes = Vec::new();
                pipe.take((cap + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes)
            })
        };
        let out = read(Box::new(stdout), limit);
        let err = read(Box::new(stderr), 8192);
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None)
                    if !self.cancel.load(Ordering::Relaxed)
                        && started.elapsed() < Duration::from_secs(5) =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                other => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break match other {
                        Err(e) => Err(e.into()),
                        _ => Err(anyhow::anyhow!("Git cancelled or timed out")),
                    };
                }
            }
        };
        let bytes = out
            .join()
            .map_err(|_| anyhow::anyhow!("Git reader failed"))??;
        let error = err
            .join()
            .map_err(|_| anyhow::anyhow!("Git error reader failed"))??;
        ensure!(
            bytes.len() <= limit && error.len() <= 8192,
            "Preview limit exceeded"
        );
        Ok(Output {
            status: status?,
            bytes,
            error,
        })
    }
    fn checked(&self, args: &[OsString], limit: usize) -> Result<Vec<u8>> {
        let o = self.run(args, limit)?;
        ensure!(
            o.status.success(),
            "Git: {}",
            String::from_utf8_lossy(&o.error).trim()
        );
        Ok(o.bytes)
    }
    fn block_filters(&mut self) -> Result<()> {
        let o = self.run(
            &args(&["config", "-z", "--name-only", "--get-regexp", r"^filter\."]),
            FILE_LIMIT,
        )?;
        ensure!(
            o.status.success() || o.status.code() == Some(1),
            "Cannot inspect Git filters"
        );
        let mut drivers = BTreeSet::new();
        for name in o.bytes.split(|b| *b == 0).filter(|n| !n.is_empty()) {
            if let Some((driver, _)) = std::str::from_utf8(name)?
                .strip_prefix("filter.")
                .and_then(|n| n.rsplit_once('.'))
            {
                ensure!(!driver.contains('='), "Unsupported Git filter name");
                drivers.insert(driver.to_owned());
            }
        }
        self.overrides = drivers
            .into_iter()
            .flat_map(|d| {
                ["clean=", "smudge=", "process=", "required=true"]
                    .map(|k| format!("filter.{d}.{k}"))
            })
            .collect();
        Ok(())
    }
    fn addition(&self, path: &Path) -> Result<String> {
        let full = self.cwd.join(path);
        let meta = std::fs::symlink_metadata(&full)?;
        if meta.len() > FILE_LIMIT as u64 {
            return Ok("Preview limit: file exceeds 512 KiB".into());
        }
        let bytes = if meta.file_type().is_symlink() {
            std::fs::read_link(full)?.as_os_str().as_bytes().to_vec()
        } else if meta.is_file() {
            let mut bytes = Vec::new();
            std::fs::File::open(full)?
                .take((FILE_LIMIT + 1) as u64)
                .read_to_end(&mut bytes)?;
            bytes
        } else {
            return Ok("Special file: no text preview".into());
        };
        let mut query = args(&["check-attr", "-z", "diff", "--"]);
        query.push(path.as_os_str().into());
        let attrs = self.checked(&query, FILE_LIMIT)?;
        if attrs.split(|b| *b == 0).nth(2) == Some(b"unset".as_slice()) {
            return Ok("Binary file (Git attributes)".into());
        }
        if bytes.len() > FILE_LIMIT {
            return Ok("Preview limit: file exceeds 512 KiB".into());
        }
        if bytes.starts_with(b"\xff\xfe") || bytes.starts_with(b"\xfe\xff") {
            return Ok("Unsupported text encoding (UTF-16)".into());
        }
        if bytes.contains(&0) {
            return Ok("Binary file".into());
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            return Ok("Unsupported text encoding (not UTF-8)".into());
        };
        if text.lines().any(|l| l.len() > 16 * 1024) {
            return Ok("Preview limit: line exceeds 16 KiB".into());
        }
        let mut patch = format!(
            "new file mode {}\n@@ -0,0 +1,{} @@\n",
            if meta.file_type().is_symlink() {
                "120000"
            } else if meta.permissions().mode() & 0o111 != 0 {
                "100755"
            } else {
                "100644"
            },
            text.lines().count()
        );
        if text.is_empty() {
            patch.truncate(patch.find('\n').unwrap() + 1);
            return Ok(patch);
        }
        for line in text.split_inclusive('\n') {
            patch.push('+');
            patch.push_str(line);
        }
        if !text.is_empty() && !text.ends_with('\n') {
            patch.push_str("\n\\ No newline at end of file\n");
        }
        Ok(patch)
    }
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
fn paths(bytes: &[u8]) -> impl Iterator<Item = PathBuf> + '_ {
    bytes
        .split(|b| *b == 0)
        .filter(|b| !b.is_empty())
        .map(|b| PathBuf::from(OsString::from_vec(b.to_vec())))
}

pub fn scan(cwd: &Path, mode: Mode, cancel: &AtomicBool) -> Result<Snapshot> {
    let mut git = Git {
        cwd: cwd.into(),
        cancel,
        overrides: vec![],
    };
    let mut root = git.checked(&args(&["rev-parse", "--show-toplevel"]), FILE_LIMIT)?;
    if root.last() == Some(&b'\n') {
        root.pop();
    }
    git.cwd = PathBuf::from(OsString::from_vec(root));
    git.block_filters()?;
    let head = git
        .run(
            &args(&["rev-parse", "--verify", "--quiet", "HEAD"]),
            FILE_LIMIT,
        )?
        .status
        .success();
    let unborn_all = mode == Mode::All && !head;
    let mut diff = args(&[
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--no-relative",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--ignore-submodules=dirty",
        "--submodule=short",
        "--find-renames",
        "--diff-algorithm=myers",
        "--output-indicator-new=+",
        "--output-indicator-old=-",
        "--output-indicator-context= ",
    ]);
    match mode {
        Mode::All if head => diff.push("HEAD".into()),
        Mode::Staged | Mode::All => diff.push("--cached".into()),
        Mode::Unstaged => {}
    }
    let mut query = diff.clone();
    query.extend(args(&["--raw", "-z", "--no-abbrev", "--"]));
    let raw = git.checked(&query, TOTAL_LIMIT)?;
    let mut records = raw.split(|b| *b == 0).filter(|b| !b.is_empty());
    let mut entries = Vec::new();
    while let Some(meta) = records.next() {
        let meta = std::str::from_utf8(meta)?;
        ensure!(meta.starts_with(':'), "Invalid Git diff metadata");
        let fields: Vec<_> = meta.split_whitespace().collect();
        ensure!(fields.len() == 5, "Invalid Git diff metadata");
        let status = fields[4].to_owned();
        let first = PathBuf::from(OsString::from_vec(
            records.next().context("Missing Git path")?.to_vec(),
        ));
        let path = if status.starts_with(['R', 'C']) {
            PathBuf::from(OsString::from_vec(
                records.next().context("Missing rename path")?.to_vec(),
            ))
        } else {
            first.clone()
        };
        // Gitlink commit objects belong to the submodule, not this repository.
        let blob = if fields[0] == ":160000" {
            ""
        } else {
            fields[2]
        };
        entries.push((path, first, status, blob.to_owned()));
    }
    if mode != Mode::Staged {
        for path in paths(&git.checked(
            &args(&["ls-files", "--others", "--exclude-standard", "-z"]),
            TOTAL_LIMIT,
        )?) {
            entries.push((path.clone(), path, "?".into(), String::new()));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.dedup_by(|a, b| a.0 == b.0);
    let mut files = Vec::new();
    let mut total = 0;
    let mut remaining_lines = 20_000usize;
    for (path, old_path, status, old_oid) in entries {
        ensure!(!cancel.load(Ordering::Relaxed), "Cancelled");
        if unborn_all
            && !git.cwd.join(&path).try_exists()?
            && std::fs::symlink_metadata(git.cwd.join(&path)).is_err()
        {
            continue;
        }
        let preview = || -> Result<String> {
            if total >= TOTAL_LIMIT {
                return Ok("Preview limit: total diff exceeds 4 MiB".into());
            }
            if remaining_lines == 0 {
                return Ok("Preview limit: total diff exceeds 20000 lines".into());
            }
            if status == "U" {
                return Ok(
                    "Unresolved conflict: resolve conflicts before viewing a regular diff".into(),
                );
            }
            if status == "?" || unborn_all {
                return git.addition(&path);
            }
            if mode != Mode::Staged
                && std::fs::symlink_metadata(git.cwd.join(&path))
                    .is_ok_and(|m| m.is_file() && m.len() > FILE_LIMIT as u64)
            {
                return Ok("Preview limit: file exceeds 512 KiB".into());
            }
            if !old_oid.bytes().all(|b| b == b'0') {
                let size = git.checked(&args(&["cat-file", "-s", &old_oid]), 128)?;
                if std::str::from_utf8(&size)?.trim().parse::<u64>()? > FILE_LIMIT as u64 {
                    return Ok("Preview limit: base exceeds 512 KiB".into());
                }
            }
            let mut query = diff.clone();
            query.extend(args(&["--patch", "--unified=3", "--"]));
            query.push(old_path.into_os_string());
            if query.last() != Some(&path.as_os_str().to_owned()) {
                query.push(path.as_os_str().into());
            }
            let bytes = git.checked(&query, FILE_LIMIT)?;
            let Ok(patch) = String::from_utf8(bytes) else {
                return Ok("Unsupported text encoding (not UTF-8)".into());
            };
            if patch.lines().any(|l| l.len() > 16 * 1024) {
                return Ok("Preview limit: line exceeds 16 KiB".into());
            }
            Ok(patch)
        };
        let mut patch = preview().unwrap_or_else(|e| format!("Preview unavailable: {e}"));
        let max_lines = remaining_lines.min(5000);
        let end = patch.match_indices('\n').nth(max_lines).map(|(i, _)| i);
        if let Some(end) = end {
            patch.truncate(end);
            patch.push_str("\nPreview limit: remaining lines omitted\n");
        }
        remaining_lines = remaining_lines.saturating_sub(patch.lines().count());
        total += patch.len();
        files.push(FileDiff {
            path,
            status,
            patch,
        });
    }
    if cancel.load(Ordering::Relaxed) {
        bail!("Cancelled");
    }
    Ok(Snapshot {
        root: git.cwd,
        files,
    })
}
