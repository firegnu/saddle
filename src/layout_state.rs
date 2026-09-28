//! Versioned workspace state, separate from configuration and terminal history.
use crate::terminals::{Node, Terminals};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    #[default]
    Empty,
    Agent {
        name: String,
        cwd: Option<String>,
        instance: Option<String>,
    },
    Shell {
        cwd: String,
    },
}
impl Content {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Agent { name, .. } => Some(name),
            _ => None,
        }
    }
    pub fn cwd(&self) -> Option<&str> {
        match self {
            Self::Agent { cwd, .. } => cwd.as_deref(),
            Self::Shell { cwd } => Some(cwd),
            Self::Empty => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Layout {
    pub version: u32,
    pub active: u64,
    pub tabs: Vec<SavedTab>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedTab {
    pub id: u64,
    pub active: u64,
    pub(crate) tree: Node,
    pub panes: Vec<SavedPane>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedPane {
    pub id: u64,
    pub content: Content,
}
impl Layout {
    pub fn validate(&self) -> Result<()> {
        use std::collections::HashSet;
        ensure!(
            self.version == 1,
            "unsupported layout version {}",
            self.version
        );
        ensure!(!self.tabs.is_empty(), "layout has no tabs");
        let mut tabs = HashSet::new();
        let mut panes = HashSet::new();
        for tab in &self.tabs {
            ensure!(tabs.insert(tab.id) && tab.id < u64::MAX, "invalid tab ID");
            ensure!(!tab.panes.is_empty(), "tab has no panes");
            let mut leaves = Vec::new();
            tab.tree.leaves(&mut leaves)?;
            let ids: HashSet<_> = tab.panes.iter().map(|p| p.id).collect();
            ensure!(ids.contains(&tab.active), "active pane missing");
            ensure!(
                leaves.len() == ids.len() && leaves.iter().copied().collect::<HashSet<_>>() == ids,
                "invalid split tree"
            );
            for pane in &tab.panes {
                ensure!(
                    panes.insert(pane.id) && pane.id < u64::MAX,
                    "invalid pane ID"
                );
                if let Content::Agent { name, .. } = &pane.content {
                    ensure!(
                        !name.is_empty()
                            && !name.starts_with('-')
                            && !name.chars().any(char::is_whitespace),
                        "invalid agent name"
                    );
                }
                if let Some(cwd) = pane.content.cwd() {
                    ensure!(
                        std::path::Path::new(cwd).is_absolute() && !cwd.contains('\0'),
                        "invalid working directory"
                    );
                }
            }
        }
        ensure!(tabs.contains(&self.active), "active tab missing");
        Ok(())
    }
}

pub fn default_path() -> Result<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .context("HOME is unavailable")?;
    Ok(state.join("saddle/layout.json"))
}

pub struct Store {
    path: Option<PathBuf>,
    protected: bool,
    last_attempt: Option<Vec<u8>>,
    pub notice: String,
}
impl Store {
    pub fn open(path: Result<PathBuf>) -> (Self, Option<Layout>) {
        let mut store = Self {
            path: None,
            protected: false,
            last_attempt: None,
            notice: String::new(),
        };
        let loaded = (|| -> Result<Option<Layout>> {
            store.path = Some(path?);
            let bytes = match fs::read(store.path.as_ref().unwrap()) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            let layout: Layout = serde_json::from_slice(&bytes)?;
            layout.validate()?;
            Ok(Some(layout))
        })();
        match loaded {
            Ok(layout) => (store, layout),
            Err(error) => {
                store.protected = true;
                store.notice = format!("Cannot restore layout; original file preserved: {error:#}");
                (store, None)
            }
        }
    }
    pub fn save(&mut self, terminals: &Terminals, force: bool) {
        if self.protected {
            return;
        }
        let bytes = serde_json::to_vec_pretty(&terminals.snapshot()).expect("serializable layout");
        if !force && self.last_attempt.as_ref() == Some(&bytes) {
            return;
        }
        self.last_attempt = Some(bytes.clone());
        let result = (|| -> Result<()> {
            let path = self.path.as_ref().context("state path unavailable")?;
            let parent = path.parent().context("state directory unavailable")?;
            fs::create_dir_all(parent)?;
            let mut file = tempfile::NamedTempFile::new_in(parent)?;
            file.write_all(&bytes)?;
            file.as_file().sync_all()?;
            file.persist(path)?;
            Ok(())
        })();
        self.notice = result
            .err()
            .map(|e| format!("Layout save failed: {e:#}"))
            .unwrap_or_default();
    }
}
