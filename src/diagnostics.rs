//! Diagnostics: a read-only look at the commands saddle uses and the latest outcomes of its
//! agent and task reads, config load and layout restore / save. Checked when the page opens or
//! Refresh is pressed; only the latest outcome of each is kept, in memory.
use crate::config::Config;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// The latest outcome of a repeated operation: when, and why it failed. None: none yet.
pub type Last = Option<(SystemTime, Result<(), String>)>;

/// What saddle knows at the moment Diagnostics is opened or refreshed.
pub struct Report {
    /// The commands as saddle runs them.
    pub corral: String,
    pub drover: String,
    /// The latest `corral ls` read.
    pub agents: Last,
    /// The Tasks project and its latest `drover` read.
    pub project: String,
    pub tasks: Last,
    pub config_path: PathBuf,
    /// Whether the config came from the file at startup, rather than defaults for a missing one.
    pub config_from_file: bool,
    /// None: no state path (HOME unavailable); the restore error says why.
    pub layout_path: Option<PathBuf>,
    /// The startup restore: whether a saved layout was restored, or why it could not be.
    pub restore: Option<(SystemTime, Result<bool, String>)>,
    pub save: Last,
    /// Saving is off because a layout file that could not be restored is kept.
    pub save_off: bool,
    pub checked: SystemTime,
    /// None: still checking.
    pub checks: Option<Checks>,
}

/// The checks that run outside the interface: they may wait on the file system or a command.
#[derive(Debug)]
pub struct Checks {
    pub corral: Probe,
    pub drover: Probe,
    /// The config file as it reads now: whether it exists, or why it cannot be used.
    pub config: Result<bool, String>,
}
#[derive(Debug)]
pub struct Probe {
    /// Where the command is found, as starting it would.
    pub path: Result<PathBuf, String>,
    /// None: the command offers no public version query.
    pub version: Option<Result<String, String>>,
}

pub fn check(
    corral: &str,
    drover: &str,
    config: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Checks {
    let version = crate::command::run(corral, &["--version"], None, timeout, cancel)
        .and_then(|output| {
            anyhow::ensure!(
                output.status.success(),
                "exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            let version = value["version"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("no version in the answer"))?;
            Ok(match value["contract"].as_str() {
                Some(contract) => format!("{version} (contract {contract})"),
                None => version.to_owned(),
            })
        })
        .map_err(|error| format!("{error:#}"));
    let config = match std::fs::read_to_string(config) {
        Ok(text) => Config::parse(&text)
            .map(|_| true)
            .map_err(|e| without_values(&line(&format!("{e:#}")))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    };
    Checks {
        corral: Probe {
            path: find(corral),
            version: Some(version),
        },
        drover: Probe {
            path: find(drover),
            version: None,
        },
        config,
    }
}

/// The executable a command name or path starts: a name is looked up on PATH.
fn find(program: &str) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let runnable = |path: &Path| {
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if program.contains('/') {
        let path = std::path::absolute(program).map_err(|e| e.to_string())?;
        return if runnable(&path) {
            Ok(path)
        } else {
            Err(format!("{} is not an executable file", path.display()))
        };
    }
    std::env::var_os("PATH")
        .iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join(program))
        .find(|path| runnable(path))
        .ok_or_else(|| format!("{program} not found on PATH"))
}

/// Runs `check` in the background; dropping it cancels a command still running.
pub struct Checker {
    pub updates: Receiver<Checks>,
    cancel: Arc<AtomicBool>,
}
impl Checker {
    pub fn start(corral: String, drover: String, config: PathBuf) -> Self {
        let (send, updates) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        thread::spawn(move || {
            let _ = send.send(check(
                &corral,
                &drover,
                &config,
                Duration::from_secs(5),
                &stop,
            ));
        });
        Self { updates, cancel }
    }
}
impl Drop for Checker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Bad,
    /// Not known: not recorded, not checked yet or not offered.
    Unknown,
}
pub enum Row {
    Heading(&'static str),
    Item(&'static str, String, Tone),
}

impl Report {
    /// The page, group by group, with the home folder shown as `~`.
    pub fn rows(&self) -> Vec<Row> {
        use Row::{Heading, Item};
        use Tone::{Bad, Good, Unknown};
        let checking = || ("checking…".to_owned(), Unknown);
        let probe = |probe: Option<&Probe>| match probe {
            None => [checking(), checking()],
            Some(probe) => [
                match &probe.path {
                    Ok(path) => (private(&path.display().to_string()), Good),
                    Err(error) => (private(error), Bad),
                },
                match &probe.version {
                    None => ("not offered by its public commands".into(), Unknown),
                    Some(Ok(version)) => (version.clone(), Good),
                    Some(Err(error)) => (format!("unavailable: {}", private(error)), Bad),
                },
            ],
        };
        let checks = self.checks.as_ref();
        let [corral_path, corral_version] = probe(checks.map(|c| &c.corral));
        let [drover_path, drover_version] = probe(checks.map(|c| &c.drover));
        let mut rows = vec![
            Heading("Program / commands"),
            Item("saddle version", env!("CARGO_PKG_VERSION").into(), Good),
            Item("corral command", private(&self.corral), Unknown),
            Item("corral path", corral_path.0, corral_path.1),
            Item("corral version", corral_version.0, corral_version.1),
            Item("drover command", private(&self.drover), Unknown),
            Item("drover path", drover_path.0, drover_path.1),
            Item("drover version", drover_version.0, drover_version.1),
            Heading("Agents"),
        ];
        let (text, tone) = last(&self.agents, "no read yet");
        rows.push(Item("Last read", text, tone));
        rows.push(Heading("Tasks"));
        rows.push(Item("Project", private(&self.project), Unknown));
        let (text, tone) = last(&self.tasks, "no read yet");
        rows.push(Item("Last read", text, tone));
        rows.push(Heading("Configuration"));
        rows.push(Item(
            "Path",
            private(&self.config_path.display().to_string()),
            Unknown,
        ));
        rows.push(Item(
            "At startup",
            if self.config_from_file {
                "loaded from the file".into()
            } else {
                "defaults (no file)".into()
            },
            Good,
        ));
        let (text, tone) = match checks.map(|c| &c.config) {
            None => checking(),
            Some(Ok(true)) => ("OK: the file is valid".into(), Good),
            Some(Ok(false)) => ("no file; defaults apply".into(), Good),
            Some(Err(error)) => (format!("failed: {}", private(&line(error))), Bad),
        };
        rows.push(Item("Load check now", text, tone));
        rows.push(Heading("Layout"));
        rows.push(Item(
            "Path",
            self.layout_path.as_ref().map_or_else(
                || "unavailable".into(),
                |path| private(&path.display().to_string()),
            ),
            if self.layout_path.is_some() {
                Unknown
            } else {
                Bad
            },
        ));
        let (text, tone) = match &self.restore {
            None => ("not attempted".into(), Unknown),
            Some((at, Ok(true))) => (format!("OK at {}: restored", clock(*at)), Good),
            Some((at, Ok(false))) => (
                format!("OK at {}: no saved layout, started fresh", clock(*at)),
                Good,
            ),
            Some((at, Err(error))) => (
                format!(
                    "failed at {}: {}; the file is kept",
                    clock(*at),
                    private(&line(error))
                ),
                Bad,
            ),
        };
        rows.push(Item("Restore", text, tone));
        let (text, tone) = if self.save_off {
            (
                "off: the file that could not be restored is kept".into(),
                Bad,
            )
        } else {
            last(&self.save, "not saved yet")
        };
        rows.push(Item("Last save", text, tone));
        rows
    }

    /// The page as plain text for the clipboard.
    pub fn summary(&self) -> String {
        let mut text = format!("saddle diagnostics, checked {}\n", clock(self.checked));
        for row in self.rows() {
            match row {
                Row::Heading(heading) => text.push_str(&format!("\n{heading}\n")),
                Row::Item(label, value, _) => text.push_str(&format!("  {label}: {value}\n")),
            }
        }
        text
    }
}

fn last(last: &Last, none: &str) -> (String, Tone) {
    match last {
        None => (none.into(), Tone::Unknown),
        Some((at, Ok(()))) => (format!("OK at {}", clock(*at)), Tone::Good),
        Some((at, Err(error))) => (
            format!("failed at {}: {}", clock(*at), private(&line(error))),
            Tone::Bad,
        ),
    }
}

/// Local date and time to the second.
fn clock(at: SystemTime) -> String {
    let unix = at
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    crate::detail::local(unix)
        .map(|tm| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                tm.tm_year + 1900,
                tm.tm_mon + 1,
                tm.tm_mday,
                tm.tm_hour,
                tm.tm_min,
                tm.tm_sec
            )
        })
        .unwrap_or_else(|| format!("{unix:.0}"))
}

/// An error on one line; a TOML error's quoted source lines are left out.
fn line(error: &str) -> String {
    let lines: Vec<_> = error
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('|') && !l.starts_with('^'))
        .filter(|l| {
            l.split_once(" |")
                .is_none_or(|(n, _)| n.parse::<u32>().is_err())
        })
        .collect();
    lines.join(" ")
}

/// A config error without what is quoted in it, the configured values; the place and kind of
/// the problem stay.
fn without_values(error: &str) -> String {
    let mut text = String::new();
    let mut chars = error.chars();
    while let Some(c) = chars.next() {
        if c != '"' && c != '`' {
            text.push(c);
            continue;
        }
        let mut escaped = false;
        for inner in chars.by_ref() {
            if inner == c && !escaped {
                break;
            }
            escaped = c == '"' && inner == '\\' && !escaped;
        }
        text.extend([c, '…', c]);
    }
    text
}

/// The home folder shown as `~`, so the summary can be shared.
fn private(text: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if home.trim_end_matches('/').len() > 1 => {
            let home = home.trim_end_matches('/');
            if text == home {
                "~".into()
            } else {
                text.replace(&format!("{home}/"), "~/")
            }
        }
        _ => text.to_owned(),
    }
}
