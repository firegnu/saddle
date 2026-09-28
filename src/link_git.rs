use anyhow::{Context, Result, ensure};
use std::{ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Duration};

fn git(root: &Path, args: &[&str], cancel: &AtomicBool) -> Result<String> {
    let mut full = vec![
        "--no-pager",
        "--no-lazy-fetch",
        "--no-optional-locks",
        "--no-replace-objects",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "log.showSignature=false",
    ];
    full.extend(args);
    let inherited: Vec<OsString> = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| name.as_encoded_bytes().starts_with(b"GIT_"))
        .collect();
    let output = crate::command::run_bounded(
        "git",
        &full,
        Some(root),
        &inherited,
        Duration::from_secs(5),
        cancel,
        crate::links::LIMIT,
    )?;
    ensure!(
        output.status.success(),
        "Git unavailable: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout).context("Git output is not UTF-8")
}
fn resolve(root: &Path, sha: &str, cancel: &AtomicBool) -> Result<String> {
    ensure!(crate::links::is_sha(sha), "Invalid commit SHA");
    let ids = git(
        root,
        &["rev-parse", &format!("--disambiguate={sha}")],
        cancel,
    )?;
    let ids: Vec<_> = ids.lines().collect();
    ensure!(ids.len() == 1, "Commit missing or ambiguous: {sha}");
    ensure!(
        git(root, &["cat-file", "-t", ids[0]], cancel)?.trim() == "commit",
        "Object is not a commit: {sha}"
    );
    Ok(ids[0].into())
}
pub fn commit(root: &Path, sha: &str, cancel: &AtomicBool) -> Result<String> {
    let sha = resolve(root, sha, cancel)?;
    git(
        root,
        &[
            "show",
            "--format=fuller",
            "--patch",
            "--stat",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
            "--ignore-submodules=all",
            &sha,
            "--",
        ],
        cancel,
    )
}
pub fn range(
    root: &Path,
    start: &str,
    end: &str,
    cancel: &AtomicBool,
) -> Result<Vec<(String, String)>> {
    let start = resolve(root, start, cancel)?;
    let end = resolve(root, end, cancel)?;
    let output = git(
        root,
        &[
            "log",
            "--format=%H %s",
            "--no-decorate",
            "--no-color",
            &format!("{start}..{end}"),
            "--",
        ],
        cancel,
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            line.split_once(' ')
                .map(|(sha, subject)| (sha.into(), subject.into()))
        })
        .collect())
}
