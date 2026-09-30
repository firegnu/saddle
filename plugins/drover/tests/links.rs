use saddle_drover_plugin::links::{self, Key, State, Target};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn wait(state: &mut State, ready: impl Fn(&State) -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !ready(state) {
        state.tick();
        assert!(Instant::now() < end, "{}", state.message);
        std::thread::yield_now();
    }
}
fn key(root: &Path, body: &str) -> Key {
    Key {
        project: root.display().to_string(),
        seq: 1,
        body: body.into(),
        range: None,
    }
}
#[test]
fn explicit_references_use_one_task_file_layer_and_the_documented_bases() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join("docs")).unwrap();
    for file in [
        "review.md",
        "docs/product.txt",
        "docs/second.md",
        "ignored.md",
    ] {
        fs::write(root.join(file), "plain").unwrap();
    }
    fs::write(root.join("docs/task.md"), "Review file：`review.md`\n[product](product.txt)\nTask file: docs/second.md\nAgent: team/impl | instance=012345abcdef\nCommit: abcdef123\n").unwrap();
    fs::write(root.join("docs/second.md"), "Artifact: ignored.md").unwrap();
    let entries = links::collect(
        root,
        "- Task file: `docs/task.md`\n[duplicate](docs/task.md)\n```text\nArtifact: ignored.md\n```\nordinary team/guess abcdef124\n代理：team/unknown",
    );
    assert_eq!(entries.len(), 7);
    assert!(entries.iter().any(
        |l| matches!(&l.target, Target::File(p) if p.ends_with("docs/product.txt"))
            && l.sources == ["docs/task.md"]
    ));
    assert!(!entries.iter().any(|l| l.label == "ignored.md"));
    assert!(
        entries
            .iter()
            .any(|l| l.label == "team/unknown" && l.note.as_deref() == Some("Identity unknown"))
    );
}
#[test]
fn file_reading_rejects_escapes_internal_data_special_files_and_non_text() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(dir.path().join("secret"), "outside").unwrap();
    symlink(dir.path().join("secret"), root.join("escape")).unwrap();
    fs::write(root.join(".drover.conf"), "private").unwrap();
    symlink(root.join(".drover.conf"), root.join("alias")).unwrap();
    fs::write(root.join("large"), vec![b'x'; links::LIMIT + 1]).unwrap();
    fs::write(root.join("binary"), b"utf8\0binary").unwrap();
    fs::write(root.join("invalid"), [255]).unwrap();
    fs::write(root.join("valid"), "UTF-8 中文").unwrap();
    symlink(root.join("valid"), root.join("inside")).unwrap();
    for (path, expected) in [
        ("escape", "Outside project"),
        ("alias", "Upstream internal"),
        (".drover.conf", "Upstream internal"),
        ("large", "too large"),
        ("binary", "Binary"),
        ("invalid", "Not UTF-8"),
        (".", "Not a regular"),
        ("missing", "unavailable"),
    ] {
        let error = links::read_file(&root, &root.join(path))
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{path}: {error}");
    }
    assert_eq!(
        links::read_file(&root, &root.join("inside")).unwrap(),
        "UTF-8 中文"
    );
    let entries = links::collect(
        &root,
        "Artifact: https://example.test/file\nTask file: .drover.conf",
    );
    assert!(entries.iter().all(|l| l.note.is_some()));
}
#[test]
fn replacing_a_task_or_project_discards_old_file_results_and_reading_position() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("old"), "OLD CONTENT").unwrap();
    let mut state = State::default();
    state.sync(key(dir.path(), "Artifact: old"));
    wait(&mut state, |s| !s.entries.is_empty());
    state.open();
    let mut next = key(dir.path(), "Agent: new/unknown");
    next.seq = 2;
    state.sync(next);
    wait(&mut state, |s| !s.entries.is_empty());
    assert!(state.reading.is_none());
    assert_eq!(state.entries[0].label, "new/unknown");
    let other = tempfile::tempdir().unwrap();
    state.sync(key(other.path(), ""));
    wait(&mut state, |s| s.message.is_empty());
    assert!(state.entries.is_empty());
    assert!(state.reading.is_none());
}
#[test]
fn recorded_range_is_listed_as_a_range_and_opens_real_commit_diffs() {
    fn git(root: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().into()
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Synthetic"]);
    git(root, &["config", "user.email", "synthetic@example.invalid"]);
    fs::write(root.join("file"), "before\n").unwrap();
    git(root, &["add", "file"]);
    git(root, &["commit", "-qm", "base"]);
    let start = git(root, &["rev-parse", "HEAD"]);
    fs::write(root.join("file"), "after\n").unwrap();
    git(root, &["add", "file"]);
    git(root, &["commit", "-qm", "SYNTHETIC COMMIT"]);
    let end = git(root, &["rev-parse", "HEAD"]);
    let mut input = key(root, "");
    input.range = Some((start, end));
    let mut state = State::default();
    state.sync(input);
    wait(&mut state, |s| s.message.is_empty());
    assert_eq!(
        state.entries.len(),
        1,
        "public start/end must yield Recorded range"
    );
    state.open();
    wait(&mut state, |s| {
        s.reading
            .as_ref()
            .is_some_and(|r| r.text.contains("SYNTHETIC COMMIT"))
    });
    state.open();
    wait(&mut state, |s| {
        s.reading
            .as_ref()
            .is_some_and(|r| r.text.contains("+after"))
    });
    assert!(state.reading.as_ref().unwrap().text.contains("-before"));
    state.back();
    assert!(
        state.reading.is_none(),
        "Back returns to the original Links position"
    );
    assert_eq!(state.selected, 0);
    // Repository-configured viewers must never run for committed object previews.
    git(
        root,
        &["config", "diff.synthetic.command", "touch external-ran"],
    );
    git(
        root,
        &["config", "diff.synthetic.textconv", "touch textconv-ran"],
    );
    git(root, &["config", "core.pager", "touch pager-ran"]);
    fs::write(root.join(".gitattributes"), "file diff=synthetic\n").unwrap();
    state.open();
    wait(&mut state, |s| {
        s.reading.as_ref().is_some_and(|r| !r.commits.is_empty())
    });
    state.open();
    wait(&mut state, |s| {
        s.reading
            .as_ref()
            .is_some_and(|r| r.text.contains("+after"))
    });
    for marker in ["external-ran", "textconv-ran", "pager-ran"] {
        assert!(!root.join(marker).exists());
    }
    fs::write(root.join("file"), "large line\n".repeat(links::LIMIT / 5)).unwrap();
    git(root, &["add", "file"]);
    git(root, &["commit", "-qm", "large patch"]);
    let large = git(root, &["rev-parse", "HEAD"]);
    state.sync(key(root, &format!("Commit: {large}")));
    wait(&mut state, |s| !s.entries.is_empty());
    state.open();
    wait(&mut state, |s| {
        s.reading.as_ref().is_some_and(|r| r.text != "Loading…")
    });
    assert!(
        state
            .reading
            .as_ref()
            .unwrap()
            .text
            .contains("Output truncated")
    );
    state.sync(key(
        root,
        "Commit: 0000000000000000000000000000000000000000",
    ));
    wait(&mut state, |s| !s.entries.is_empty());
    state.open();
    wait(&mut state, |s| {
        s.reading.as_ref().is_some_and(|r| r.text != "Loading…")
    });
    assert!(
        state
            .reading
            .as_ref()
            .unwrap()
            .text
            .contains("missing or ambiguous")
    );
}

#[test]
fn arriving_git_endpoints_preserve_an_open_file_and_link_selection() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file"), "KEPT READING").unwrap();
    let mut state = State::default();
    let mut input = key(dir.path(), "Artifact: file");
    state.sync(input.clone());
    wait(&mut state, |s| !s.entries.is_empty());
    state.open();
    wait(&mut state, |s| {
        s.reading.as_ref().is_some_and(|r| r.text == "KEPT READING")
    });
    input.range = Some(("abcdef1".into(), "abcdef2".into()));
    state.sync(input);
    assert_eq!(
        state.reading.as_ref().map(|r| r.text.as_str()),
        Some("KEPT READING")
    );
}

#[test]
fn links_report_public_detail_read_failures_instead_of_only_an_empty_state() {
    use saddle_drover_plugin::{
        drover::{Snapshot, Task},
        queue::{Panel, View},
    };
    let dir = tempfile::tempdir().unwrap();
    let mut panel = Panel::default();
    panel.project = dir.path().display().to_string();
    panel.view = View::Links;
    panel.absorb(Snapshot {
        current: Some(Task {
            id: Some("T1".into()),
            title: "Current task".into(),
            ..Default::default()
        }),
        ..Default::default()
    });
    let target = panel.detail_key().unwrap();
    panel.absorb_detail(
        &target,
        Err(anyhow::anyhow!("synthetic public show failed")),
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 36)).unwrap();
    terminal
        .draw(|frame| {
            panel.draw(&Default::default(), frame, frame.area());
        })
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("synthetic public show failed"), "{text}");
}
