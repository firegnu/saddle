//! Resolve the external agent runtime from PATH or an explicit configured path.
use std::{os::unix::fs::PermissionsExt, path::PathBuf};

pub fn resolve(configured: &str) -> std::io::Result<PathBuf> {
    let path = crate::config::expand_home(configured);
    if !configured.contains('/') {
        let found = std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p).map(|d| d.join(&path)).find(|p| {
                p.metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
        });
        return found.map(std::path::absolute).unwrap_or_else(|| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{configured} not found on PATH"),
            ))
        });
    }
    std::path::absolute(path)
}

pub fn configured() -> anyhow::Result<PathBuf> {
    let config = crate::config::Config::load(&crate::config::default_path())?;
    Ok(resolve(&config.corral)?)
}
