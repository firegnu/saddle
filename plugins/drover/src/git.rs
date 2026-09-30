use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
pub fn repository(program: &str, cwd: &str, cancel: &AtomicBool) -> Option<(PathBuf, PathBuf)> {
    if !Path::new(cwd).is_absolute() {
        return None;
    }
    let output = crate::command::run(
        program,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-common-dir",
        ],
        Some(Path::new(cwd)),
        std::time::Duration::from_secs(5),
        cancel,
    )
    .ok()
    .filter(|o| o.status.success())?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines().map(PathBuf::from);
    let (top, common) = (lines.next()?, lines.next()?);
    Some((
        top.canonicalize().unwrap_or(top),
        common.canonicalize().unwrap_or(common),
    ))
}
