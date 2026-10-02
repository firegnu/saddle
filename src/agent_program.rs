//! Select the product's bottom-layer executable without a PATH fallback.
use std::{os::unix::fs::PermissionsExt, path::PathBuf};

pub fn bundled() -> std::io::Result<PathBuf> {
    Ok(std::fs::canonicalize(std::env::current_exe()?)?.with_file_name("corral"))
}

pub fn resolve(configured: &str) -> std::io::Result<PathBuf> {
    if configured == "corral" {
        return bundled();
    }
    let path = crate::config::expand_home(configured);
    if path.components().count() == 1
        && !configured.contains('/')
        && let Some(found) = std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p).map(|d| d.join(&path)).find(|p| {
                p.metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
        })
    {
        return std::path::absolute(found);
    }
    std::path::absolute(path)
}

pub fn configured() -> anyhow::Result<PathBuf> {
    let config = crate::config::Config::load(&crate::config::default_path())?;
    Ok(resolve(&config.corral)?)
}
