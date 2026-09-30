use saddle_diff_plugin::git::{Mode, scan};
use std::{path::Path, process::Command, sync::atomic::AtomicBool};

fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    git(d.path(), &["config", "user.name", "Test"]);
    git(d.path(), &["config", "user.email", "test@example.invalid"]);
    std::fs::write(d.path().join("a.rs"), "fn old() {}\n").unwrap();
    git(d.path(), &["add", "."]);
    git(d.path(), &["commit", "-qm", "base"]);
    d
}
#[test]
fn all_staged_and_unstaged_show_the_correct_baselines_and_untracked_files() {
    let d = repo();
    std::fs::write(d.path().join("a.rs"), "fn staged() {}\n").unwrap();
    git(d.path(), &["add", "a.rs"]);
    std::fs::write(d.path().join("a.rs"), "fn latest() {}\n").unwrap();
    std::fs::write(d.path().join("new file.txt"), "new content\n").unwrap();
    let cancel = AtomicBool::new(false);
    let all = scan(d.path(), Mode::All, &cancel).unwrap();
    assert_eq!(all.files.len(), 2);
    assert!(all.files[0].patch.contains("-fn old() {}"));
    assert!(all.files[0].patch.contains("+fn latest() {}"));
    assert!(all.files[1].patch.contains("+new content"));
    let staged = scan(d.path(), Mode::Staged, &cancel).unwrap();
    assert_eq!(staged.files.len(), 1);
    assert!(staged.files[0].patch.contains("+fn staged() {}"));
    let unstaged = scan(d.path(), Mode::Unstaged, &cancel).unwrap();
    assert!(unstaged.files[0].patch.contains("-fn staged() {}"));
    assert!(unstaged.files[0].patch.contains("+fn latest() {}"));
    std::fs::write(d.path().join("a.rs"), "fn changed_again() {}\n").unwrap();
    assert!(
        scan(d.path(), Mode::All, &cancel).unwrap().files[0]
            .patch
            .contains("changed_again")
    );
}

#[test]
fn file_types_deletions_renames_and_modes_remain_visible() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let d = repo();
    std::fs::write(d.path().join("binary.txt"), b"old\0bytes").unwrap();
    std::fs::write(d.path().join("gone.md"), "gone\n").unwrap();
    std::fs::write(d.path().join(".gitattributes"), "*.opaque -diff\n").unwrap();
    std::fs::write(d.path().join("a.opaque"), "old\n").unwrap();
    symlink("first", d.path().join("link")).unwrap();
    git(d.path(), &["add", "."]);
    git(d.path(), &["commit", "-qm", "types"]);
    git(d.path(), &["mv", "a.rs", "renamed.rs"]);
    std::fs::write(d.path().join("binary.txt"), b"new\0bytes").unwrap();
    std::fs::write(d.path().join("a.opaque"), "changed\n").unwrap();
    std::fs::remove_file(d.path().join("gone.md")).unwrap();
    std::fs::remove_file(d.path().join("link")).unwrap();
    symlink("second", d.path().join("link")).unwrap();
    std::fs::set_permissions(
        d.path().join("renamed.rs"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let s = scan(d.path(), Mode::All, &AtomicBool::new(false)).unwrap();
    let patch = |path: &str| {
        &s.files
            .iter()
            .find(|f| f.path == Path::new(path))
            .unwrap()
            .patch
    };
    assert!(patch("binary.txt").contains("Binary"));
    assert!(patch("a.opaque").contains("Binary"));
    assert!(patch("gone.md").contains("-gone"));
    assert!(patch("renamed.rs").contains("rename from a.rs"));
    assert!(patch("renamed.rs").contains("100755"));
    assert!(patch("link").contains("-first"));
    assert!(patch("link").contains("+second"));
}

#[test]
fn unborn_repository_and_hostile_names_are_read_only_and_not_lost() {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    let name = "-new\n文件.txt";
    std::fs::write(d.path().join(name), "hello\n").unwrap();
    git(d.path(), &["add", "--", name]);
    std::fs::write(d.path().join(name), "latest\n").unwrap();
    let index = std::fs::read(d.path().join(".git/index")).unwrap();
    let s = scan(d.path(), Mode::All, &AtomicBool::new(false)).unwrap();
    assert_eq!(s.files.len(), 1);
    assert_eq!(s.files[0].path, Path::new(name));
    assert!(s.files[0].patch.contains("+latest"));
    let staged = scan(d.path(), Mode::Staged, &AtomicBool::new(false)).unwrap();
    assert!(staged.files[0].patch.contains("+hello"));
    assert_eq!(index, std::fs::read(d.path().join(".git/index")).unwrap());
}

#[test]
fn unsupported_encoding_large_files_ignored_files_and_cancellation_are_explicit() {
    let d = repo();
    std::fs::write(d.path().join(".gitignore"), "ignored\n").unwrap();
    std::fs::write(d.path().join("ignored"), "secret").unwrap();
    std::fs::write(d.path().join("encoding.txt"), b"\xffbad").unwrap();
    std::fs::write(d.path().join("large.txt"), vec![b'a'; 2 * 1024 * 1024]).unwrap();
    let s = scan(d.path(), Mode::All, &AtomicBool::new(false)).unwrap();
    assert!(!s.files.iter().any(|f| f.path == Path::new("ignored")));
    assert!(
        s.files
            .iter()
            .find(|f| f.path == Path::new("encoding.txt"))
            .unwrap()
            .patch
            .contains("encoding")
    );
    assert!(
        s.files
            .iter()
            .find(|f| f.path == Path::new("large.txt"))
            .unwrap()
            .patch
            .contains("limit")
    );
    assert!(scan(d.path(), Mode::All, &AtomicBool::new(true)).is_err());
}

#[test]
fn external_diff_textconv_and_filters_never_execute() {
    let d = repo();
    let marker = d.path().join("executed");
    let script = format!("touch {}; cat", marker.display());
    git(d.path(), &["config", "diff.external", &script]);
    git(d.path(), &["config", "diff.danger.textconv", &script]);
    std::fs::write(d.path().join(".gitattributes"), "a.rs diff=danger\n").unwrap();
    std::fs::write(d.path().join("a.rs"), "changed\n").unwrap();
    let s = scan(d.path(), Mode::All, &AtomicBool::new(false)).unwrap();
    assert!(s.files.iter().any(|f| f.patch.contains("+changed")));
    assert!(!marker.exists());
    git(d.path(), &["config", "filter.danger.clean", &script]);
    std::fs::write(d.path().join(".gitattributes"), "a.rs filter=danger\n").unwrap();
    // Refuse a filtered diff rather than execute the program or pretend the tree is clean.
    assert!(scan(d.path(), Mode::All, &AtomicBool::new(false)).is_err());
    assert!(!marker.exists());
}

#[test]
fn linked_worktree_uses_its_own_index_and_head() {
    let d = repo();
    let target = tempfile::tempdir().unwrap();
    let wt = target.path().join("linked");
    git(
        d.path(),
        &["worktree", "add", "-qb", "other", wt.to_str().unwrap()],
    );
    std::fs::write(wt.join("a.rs"), "linked_content\n").unwrap();
    assert!(
        scan(&wt, Mode::All, &AtomicBool::new(false)).unwrap().files[0]
            .patch
            .contains("linked_content")
    );
    assert!(
        scan(d.path(), Mode::All, &AtomicBool::new(false))
            .unwrap()
            .files
            .is_empty()
    );
}

#[test]
fn untracked_executable_and_empty_file_keep_git_metadata() {
    use std::os::unix::fs::PermissionsExt;
    let d = repo();
    std::fs::write(d.path().join("run.sh"), "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(
        d.path().join("run.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    std::fs::write(d.path().join("empty"), "").unwrap();
    let s = scan(d.path(), Mode::All, &AtomicBool::new(false)).unwrap();
    assert!(
        s.files
            .iter()
            .find(|f| f.path == Path::new("run.sh"))
            .unwrap()
            .patch
            .contains("100755")
    );
    assert!(
        !s.files
            .iter()
            .find(|f| f.path == Path::new("empty"))
            .unwrap()
            .patch
            .contains("@@")
    );
}

#[test]
fn gitlink_and_conflicts_are_not_reported_as_missing_blobs_or_clean() {
    let d = repo();
    let oid = String::from_utf8(
        Command::new("git")
            .arg("-C")
            .arg(d.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let missing = "0123456789012345678901234567890123456789";
    git(
        d.path(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{missing},sub"),
        ],
    );
    git(d.path(), &["commit", "-qm", "gitlink"]);
    git(
        d.path(),
        &[
            "update-index",
            "--cacheinfo",
            &format!("160000,{},sub", oid.trim()),
        ],
    );
    let s = scan(d.path(), Mode::Staged, &AtomicBool::new(false)).unwrap();
    let patch = &s
        .files
        .iter()
        .find(|f| f.path == Path::new("sub"))
        .unwrap()
        .patch;
    assert!(
        patch.contains("-Subproject commit") && patch.contains("+Subproject commit"),
        "{patch}"
    );
    git(d.path(), &["reset", "--hard", "HEAD"]);
    git(d.path(), &["checkout", "-qb", "conflicting"]);
    std::fs::write(d.path().join("a.rs"), "other\n").unwrap();
    git(d.path(), &["commit", "-qam", "other"]);
    git(d.path(), &["checkout", "-"]);
    std::fs::write(d.path().join("a.rs"), "ours\n").unwrap();
    git(d.path(), &["commit", "-qam", "ours"]);
    let result = Command::new("git")
        .arg("-C")
        .arg(d.path())
        .args(["merge", "conflicting"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let s = scan(d.path(), Mode::Unstaged, &AtomicBool::new(false)).unwrap();
    assert!(
        s.files
            .iter()
            .find(|f| f.path == Path::new("a.rs"))
            .unwrap()
            .patch
            .contains("Unresolved conflict")
    );
}
