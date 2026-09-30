use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub protocol_major: u32,
    pub executable: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    pub required_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<saddle_plugin_protocol::ViewDeclaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<saddle_plugin_protocol::OpenAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}
pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}
impl Manifest {
    pub fn read(dir: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        fs::File::open(dir.join("plugin.toml"))?
            .take(65537)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 65536, "manifest too large");
        let m: Self = toml::from_slice(&bytes)?;
        ensure!(
            m.manifest_version == 1 && m.protocol_major == 1,
            "unsupported plugin version"
        );
        ensure!(
            valid_id(&m.id)
                && !m.name.is_empty()
                && m.name.len() <= 128
                && !m.name.chars().any(char::is_control),
            "invalid plugin identity"
        );
        ensure!(
            !m.version.is_empty()
                && m.version.len() <= 128
                && !m.version.chars().any(char::is_control),
            "invalid version"
        );
        ensure!(
            !m.executable.as_os_str().is_empty()
                && !m.executable.is_absolute()
                && !m
                    .executable
                    .components()
                    .any(|c| matches!(c, Component::ParentDir)),
            "executable must be relative without .."
        );
        ensure!(
            m.required_capabilities
                .iter()
                .all(|c| saddle_plugin_protocol::CAPABILITIES.contains(&c.as_str())),
            "unsupported required capability"
        );
        if m.view.is_some() || m.action.is_some() || m.entry.is_some() {
            let view = m.view.as_ref().context("missing view declaration")?;
            let action = m.action.as_ref().context("missing open action")?;
            ensure!(
                view.id == "main"
                    && action.view == view.id
                    && valid_id(&action.id)
                    && m.entry.as_ref() == Some(&action.id)
                    && !action.title.is_empty()
                    && action.title.len() <= 128
                    && !action.title.chars().any(char::is_control),
                "invalid plugin entry"
            );
            ensure!(
                m.required_capabilities.iter().any(|c| c == "ui.entry.v1")
                    && m.required_capabilities.iter().any(|c| c == "panel.v1"),
                "entry requires ui.entry.v1 and panel.v1"
            );
            ensure!(
                view.placement != saddle_plugin_protocol::Placement::Overlay
                    || m.required_capabilities
                        .iter()
                        .any(|c| c == "panel.overlay.v1"),
                "overlay requires panel.overlay.v1"
            );
        }
        ensure!(
            !m.required_capabilities.iter().any(|c| c == "attention.v1") || m.action.is_some(),
            "attention requires an open action"
        );
        let exe = m.program(dir)?;
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(exe)?;
        ensure!(
            meta.is_file() && meta.permissions().mode() & 0o111 != 0,
            "entry is not executable"
        );
        Ok(m)
    }
    pub fn program(&self, dir: &Path) -> Result<PathBuf> {
        Ok(dir.join(&self.executable).canonicalize()?)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub directory: PathBuf,
    pub enabled: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct File {
    version: u32,
    plugins: Vec<Entry>,
}
pub struct Registry {
    pub entries: Vec<Entry>,
    path: PathBuf,
    baseline: Option<Vec<u8>>,
    pub error: Option<String>,
}
impl Registry {
    pub fn open(path: PathBuf) -> Self {
        let mut r = Self {
            entries: vec![],
            path,
            baseline: None,
            error: None,
        };
        if let Err(e) = r.refresh() {
            r.error = Some(format!("{e:#}"));
        }
        r
    }
    pub fn refresh(&mut self) -> Result<()> {
        let bytes = read(&self.path)?;
        let entries = if let Some(b) = &bytes {
            ensure!(b.len() <= 1024 * 1024, "registry too large");
            let file: File = toml::from_slice(b)?;
            ensure!(file.version == 1, "unsupported plugins registry version");
            let mut ids = std::collections::HashSet::new();
            for e in &file.plugins {
                ensure!(
                    valid_id(&e.id) && e.directory.is_absolute() && ids.insert(e.id.clone()),
                    "invalid plugin registry"
                );
            }
            file.plugins
        } else {
            vec![]
        };
        self.entries = entries;
        self.baseline = bytes;
        self.error = None;
        Ok(())
    }
    pub fn add(&mut self, dir: &Path, preview: &Manifest) -> Result<()> {
        let dir = dir.canonicalize()?;
        let actual = Manifest::read(&dir)?;
        ensure!(&actual == preview, "manifest changed; read it again");
        ensure!(
            !self.entries.iter().any(|e| e.id == actual.id),
            "plugin ID already registered"
        );
        let mut entries = self.entries.clone();
        entries.push(Entry {
            id: actual.id,
            directory: dir,
            enabled: false,
        });
        self.write(entries)
    }
    pub fn enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let mut entries = self.entries.clone();
        entries
            .iter_mut()
            .find(|e| e.id == id)
            .context("plugin missing")?
            .enabled = enabled;
        self.write(entries)
    }
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let mut entries = self.entries.clone();
        ensure!(
            entries.iter().any(|e| e.id == id && !e.enabled),
            "disable before removing"
        );
        entries.retain(|e| e.id != id);
        self.write(entries)
    }
    fn write(&mut self, entries: Vec<Entry>) -> Result<()> {
        ensure!(self.error.is_none(), "registry unavailable; refresh first");
        let parent = self
            .path
            .parent()
            .context("registry directory unavailable")?;
        fs::create_dir_all(parent)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.path.with_extension("lock"))?;
        use std::os::fd::AsRawFd;
        ensure!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
            "plugin registry busy; refresh and retry"
        );
        ensure!(
            read(&self.path)? == self.baseline,
            "plugin registry changed; refresh first"
        );
        let bytes = toml::to_string(&File {
            version: 1,
            plugins: entries.clone(),
        })?
        .into_bytes();
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        tmp.write_all(&bytes)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&self.path)?;
        self.entries = entries;
        self.baseline = Some(bytes);
        Ok(())
    }
}
fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::File::open(path) {
        Ok(file) => {
            let mut b = Vec::new();
            file.take(1024 * 1024 + 1).read_to_end(&mut b)?;
            ensure!(b.len() <= 1024 * 1024, "registry too large");
            Ok(Some(b))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
