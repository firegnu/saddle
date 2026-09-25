use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub corral: String,
    pub left_width: u16,
    pub left_split: f64,
    pub refresh_ms: u64,
    pub queue: Queue,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Queue {
    pub command: Vec<String>,
    pub cwd: Option<String>,
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            command: vec!["drover".into(), "board".into()],
            cwd: None,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            corral: "corral".into(),
            left_width: 52,
            left_split: 0.5,
            refresh_ms: 1000,
            queue: Queue::default(),
        }
    }
}
impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text)?;
        ensure!(
            config.left_split.is_finite() && config.left_split > 0.0 && config.left_split < 1.0,
            "left_split must be between 0 and 1 (exclusive)"
        );
        ensure!(config.left_width > 0, "left_width must be positive");
        ensure!(config.refresh_ms > 0, "refresh_ms must be positive");
        ensure!(!config.corral.trim().is_empty(), "corral cannot be empty");
        ensure!(
            config
                .queue
                .command
                .first()
                .is_some_and(|s| !s.trim().is_empty()),
            "queue.command needs a program"
        );
        Ok(config)
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                Self::parse(&text).with_context(|| format!("invalid config {}", path.display()))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
        }
    }
}

pub fn expand_home(path: &str) -> PathBuf {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(path.strip_prefix("~/").unwrap_or(""));
    }
    PathBuf::from(path)
}
