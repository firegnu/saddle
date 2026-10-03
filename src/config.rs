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
    pub mascot_enabled: bool,
    /// Which pet the mascot is.
    pub mascot: crate::mascot::Pet,
    /// Pictures where the terminal shows them, or always block glyphs.
    pub mascot_display: crate::mascot::Display,
    /// Preserved for old config files; all task settings now belong to the Drover plugin.
    pub queue: Option<toml::Value>,
    pub theme: crate::theme::Preset,
    /// The `[colors]` keys as written, over `theme`.
    #[serde(
        rename = "colors",
        deserialize_with = "crate::theme::deserialize_overrides"
    )]
    pub overrides: crate::theme::Overrides,
    /// The colors in effect: `theme` with `overrides`.
    #[serde(skip)]
    pub colors: crate::theme::Theme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            corral: "corral".into(),
            left_width: 52,
            left_split: 0.5,
            refresh_ms: 1000,
            mascot_enabled: true,
            mascot: Default::default(),
            mascot_display: Default::default(),
            queue: None,
            theme: Default::default(),
            overrides: Default::default(),
            colors: crate::theme::Theme::default(),
        }
    }
}
impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(text)?;
        config.colors = config.theme.theme().with(&config.overrides);
        ensure!(
            config.left_split.is_finite() && config.left_split > 0.0 && config.left_split < 1.0,
            "left_split must be between 0 and 1 (exclusive)"
        );
        ensure!(config.left_width > 0, "left_width must be positive");
        ensure!(config.refresh_ms > 0, "refresh_ms must be positive");
        ensure!(!config.corral.trim().is_empty(), "corral cannot be empty");
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

/// XDG requires an absolute base directory; invalid values use the existing fallback.
pub fn default_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join("saddle/config.toml"))
        .unwrap_or_else(|| expand_home("~/.config/saddle/config.toml"))
}
