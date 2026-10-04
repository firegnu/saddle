use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    os::fd::AsRawFd,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoreEntry {
    pub enabled: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct File {
    version: u32,
    plugins: Vec<Entry>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    core: std::collections::BTreeMap<String, CoreEntry>,
    /// The one plugin the user pinned to the Agents header; never set by a plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pinned: Option<String>,
}
pub struct Registry {
    pub entries: Vec<Entry>,
    pub core: std::collections::BTreeMap<String, CoreEntry>,
    pub pinned: Option<String>,
    reserved: std::collections::BTreeSet<String>,
    path: PathBuf,
    baseline: Option<Vec<u8>>,
    pub error: Option<String>,
}
struct RegistryLock<'a>(&'a fs::File);
impl Drop for RegistryLock<'_> {
    fn drop(&mut self) {
        // Closing our fd alone may leave a forked child's reference holding the
        // lock until exec. Best-effort unlock preserves the original write result
        // (including an already persisted success); closing the file still follows.
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}
fn acquire_lock(fd: std::os::fd::RawFd) -> Result<()> {
    if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let error = std::io::Error::last_os_error();
        ensure!(
            error.kind() != std::io::ErrorKind::WouldBlock,
            "plugin registry busy; refresh and retry"
        );
        return Err(error).context("could not lock plugin registry");
    }
    Ok(())
}
impl Registry {
    pub fn open(path: PathBuf) -> Self {
        Self::with_reserved(path, std::iter::empty::<&str>())
    }
    pub fn with_reserved<'a>(path: PathBuf, ids: impl IntoIterator<Item = &'a str>) -> Self {
        let mut r = Self {
            entries: vec![],
            core: Default::default(),
            pinned: None,
            reserved: ids.into_iter().map(str::to_owned).collect(),
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
        let (entries, core, pinned) = if let Some(b) = &bytes {
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
            // A pin left behind for a missing registration (an older Saddle removed it) is dropped.
            let pinned = file
                .pinned
                .filter(|id| file.plugins.iter().any(|e| &e.id == id));
            (file.plugins, file.core, pinned)
        } else {
            (vec![], Default::default(), None)
        };
        self.entries = entries;
        self.core = core;
        self.pinned = pinned;
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
        ensure!(
            !self.reserved.contains(&actual.id),
            "plugin ID reserved by a built-in plugin"
        );
        let mut entries = self.entries.clone();
        entries.push(Entry {
            id: actual.id,
            directory: dir,
            enabled: false,
        });
        self.write(entries, self.core.clone(), self.pinned.clone())
    }
    pub fn enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let mut entries = self.entries.clone();
        entries
            .iter_mut()
            .find(|e| e.id == id)
            .context("plugin missing")?
            .enabled = enabled;
        self.write(entries, self.core.clone(), self.pinned.clone())
    }
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let mut entries = self.entries.clone();
        ensure!(
            entries.iter().any(|e| e.id == id && !e.enabled),
            "disable before removing"
        );
        entries.retain(|e| e.id != id);
        self.write(entries, self.core.clone(), self.pinned.clone())
    }
    /// Management-layer switch only; headless commands never call this.
    pub fn core_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        ensure!(self.reserved.contains(id), "unknown built-in plugin");
        ensure!(
            !self.entries.iter().any(|e| e.id == id),
            "plugin ID conflicts with an external plugin"
        );
        let mut core = self.core.clone();
        core.insert(id.into(), CoreEntry { enabled });
        self.write(self.entries.clone(), core, self.pinned.clone())
    }
    /// Pins one registered plugin to the Agents header, replacing any other; `None` unpins.
    pub fn pin(&mut self, id: Option<&str>) -> Result<()> {
        if let Some(id) = id {
            ensure!(self.entries.iter().any(|e| e.id == id), "plugin missing");
        }
        self.write(
            self.entries.clone(),
            self.core.clone(),
            id.map(str::to_owned),
        )
    }
    /// Removing a registration also removes its pin.
    fn write(
        &mut self,
        entries: Vec<Entry>,
        core: std::collections::BTreeMap<String, CoreEntry>,
        pinned: Option<String>,
    ) -> Result<()> {
        let pinned = pinned.filter(|id| entries.iter().any(|e| &e.id == id));
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
        acquire_lock(lock.as_raw_fd())?;
        let _unlock = RegistryLock(&lock);
        #[cfg(test)]
        tests::after_lock(lock.as_raw_fd());
        ensure!(
            read(&self.path)? == self.baseline,
            "plugin registry changed; refresh first"
        );
        let bytes = toml::to_string(&File {
            version: 1,
            plugins: entries.clone(),
            core: core.clone(),
            pinned: pinned.clone(),
        })?
        .into_bytes();
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        tmp.write_all(&bytes)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&self.path)?;
        self.entries = entries;
        self.core = core;
        self.pinned = pinned;
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

#[cfg(test)]
mod tests;
