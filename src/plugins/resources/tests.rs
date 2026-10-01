use super::*;
use saddle_core_plugin::SetupFile;
use std::os::unix::fs::symlink;

const R1: &[ResourceFile] = &[
    ResourceFile {
        path: "SKILL.md",
        bytes: b"skill v1",
    },
    ResourceFile {
        path: "README.md",
        bytes: b"readme v1",
    },
    ResourceFile {
        path: "OLD.md",
        bytes: b"only in r1",
    },
];
const R2: &[ResourceFile] = &[
    ResourceFile {
        path: "SKILL.md",
        bytes: b"skill v2",
    },
    ResourceFile {
        path: "README.md",
        bytes: b"readme v2",
    },
    ResourceFile {
        path: "NEW.md",
        bytes: b"only in r2",
    },
];
fn skill(name: &'static str, revision: u32, files: &'static [ResourceFile]) -> Resource {
    Resource {
        kind: ResourceKind::AgentSkill,
        name,
        revision,
        files,
    }
}
fn plugin(resources: Vec<Resource>) -> &'static Manifest {
    Box::leak(Box::new(Manifest {
        id: "test.res",
        name: "Synthetic resources",
        version: "1",
        commands: &[],
        resources: Box::leak(resources.into_boxed_slice()),
        setup_note: "Synthetic note",
        setup_files: &[
            SetupFile {
                label: "Template",
                resource: "skill-a",
                path: "README.md",
            },
            SetupFile {
                label: "Unknown",
                resource: "nope",
                path: "README.md",
            },
        ],
    }))
}
fn r1() -> &'static Manifest {
    plugin(vec![skill("skill-a", 1, R1)])
}
fn r2() -> &'static Manifest {
    plugin(vec![skill("skill-a", 2, R2)])
}
fn version(revision: u32, files: &[ResourceFile]) -> Version {
    Version {
        revision,
        files: files
            .iter()
            .map(|f| (f.path.into(), sha256(f.bytes)))
            .collect(),
    }
}

struct Env {
    dir: tempfile::TempDir,
}
impl Env {
    /// Synthetic HOME with both agent homes present.
    fn new() -> Self {
        let env = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(env.home().join(".claude")).unwrap();
        fs::create_dir_all(env.home().join(".agents")).unwrap();
        env
    }
    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }
    fn state(&self) -> PathBuf {
        self.dir.path().join("state/saddle")
    }
    fn res(&self) -> Resources {
        Resources::new(self.home(), self.state())
    }
    fn claude(&self) -> PathBuf {
        self.home().join(".claude/skills/skill-a")
    }
    fn codex(&self) -> PathBuf {
        self.home().join(".agents/skills/skill-a")
    }
    fn record(&self, path: &Path, agent: &str, installed: Version, pending: Option<Version>) {
        let res = self.res();
        let mut record = res.read().unwrap();
        record.targets.retain(|e| e.path != path);
        record.targets.push(Entry {
            plugin: "test.res".into(),
            resource: "skill-a".into(),
            agent: agent.into(),
            path: path.into(),
            revision: installed.revision,
            files: installed.files,
            pending,
            updated_at: "synthetic".into(),
        });
        fs::create_dir_all(self.state()).unwrap();
        res.save(&record).unwrap();
    }
    fn forget(&self) {
        let _ = fs::remove_file(self.state().join(RECORD));
    }
}
fn put(dir: &Path, files: &[ResourceFile]) {
    fs::create_dir_all(dir).unwrap();
    for f in files {
        fs::write(dir.join(f.path), f.bytes).unwrap();
    }
}
fn reset(path: &Path) {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path).unwrap(),
        Ok(_) => fs::remove_file(path).unwrap(),
        Err(_) => {}
    }
}
/// Every entry under `dir` (links are listed with their target, never followed).
fn tree(dir: &Path) -> BTreeMap<PathBuf, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for e in entries {
            let p = e.unwrap().path();
            let m = fs::symlink_metadata(&p).unwrap();
            let v = if m.file_type().is_symlink() {
                format!("-> {}", fs::read_link(&p).unwrap().display())
            } else if m.is_dir() {
                stack.push(p.clone());
                "dir".into()
            } else {
                String::from_utf8_lossy(&fs::read(&p).unwrap()).into_owned()
            };
            out.insert(p, v);
        }
    }
    out
}
fn target<'a>(status: &'a Status, agent: &str) -> &'a TargetStatus {
    status.resources[0]
        .targets
        .iter()
        .find(|t| t.agent == agent)
        .expect("target listed")
}
fn claude_class(env: &Env, m: &Manifest) -> (Class, Option<String>, Option<String>) {
    let status = env.res().status(m).unwrap();
    let t = target(&status, "claude-code");
    (t.state, t.reason.clone(), t.detail.clone())
}
fn result<'a>(receipt: &'a Receipt, agent: &str) -> &'a TargetReceipt {
    receipt.resources[0]
        .targets
        .iter()
        .find(|t| t.agent == agent)
        .expect("target in receipt")
}
fn on_fault(f: impl FnMut(&str) -> io::Result<()> + 'static) {
    FAULT.set(Some(Box::new(f)));
}
fn fail_once(name: &'static str) {
    let mut done = false;
    on_fault(move |s| {
        if s == name && !done {
            done = true;
            return Err(io::Error::other("injected"));
        }
        Ok(())
    });
}

/// Name, setup, expected class and reason.
type Case<'a> = (&'a str, Box<dyn Fn() + 'a>, Class, Option<&'a str>);
#[test]
fn status_classifies_every_target_state_without_writing() {
    let env = Env::new();
    fs::remove_dir(env.home().join(".agents")).unwrap();
    let m = r2();
    let status = env.res().status(m).unwrap();
    assert_eq!(status.resources[0].kind, "agent_skill");
    assert_eq!(status.resources[0].revision, 2);
    assert!(
        status.resources[0]
            .fingerprint
            .as_deref()
            .is_some_and(|f| f.starts_with("sha256:"))
    );
    assert_eq!(target(&status, "claude-code").state, Class::Absent);
    let codex = target(&status, "codex");
    assert_eq!(codex.state, Class::Absent);
    assert_eq!(codex.reason.as_deref(), Some("agent_absent"));
    assert_eq!(codex.path, env.codex());
    fs::create_dir(env.home().join(".agents")).unwrap();

    let path = env.claude();
    let v1 = version(1, R1);
    let v2 = version(2, R2);
    let cases: Vec<Case> = vec![
        (
            "record without directory",
            Box::new(|| env.record(&path, "claude-code", v2.clone(), None)),
            Class::Missing,
            None,
        ),
        (
            "existing link",
            Box::new(|| {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                symlink("/synthetic/elsewhere", &path).unwrap();
            }),
            Class::Foreign,
            Some("symlink"),
        ),
        (
            "unrecorded directory",
            Box::new(|| put(&path, R2)),
            Class::Foreign,
            Some("directory"),
        ),
        (
            "unrecorded file",
            Box::new(|| {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, "x").unwrap();
            }),
            Class::Foreign,
            Some("file"),
        ),
        (
            "current with user extra",
            Box::new(|| {
                put(&path, R2);
                fs::write(path.join("EXTRA.md"), "mine").unwrap();
                env.record(&path, "claude-code", v2.clone(), None);
            }),
            Class::OwnedCurrent,
            None,
        ),
        (
            "outdated",
            Box::new(|| {
                put(&path, R1);
                env.record(&path, "claude-code", v1.clone(), None);
            }),
            Class::OwnedOutdated,
            None,
        ),
        (
            "outdated but new path taken by a user file",
            Box::new(|| {
                put(&path, R1);
                fs::write(path.join("NEW.md"), "mine").unwrap();
                env.record(&path, "claude-code", v1.clone(), None);
            }),
            Class::Modified,
            Some("user_file"),
        ),
        (
            "newer build installed",
            Box::new(|| {
                put(&path, R2);
                let mut v3 = v2.clone();
                v3.revision = 3;
                env.record(&path, "claude-code", v3, None);
            }),
            Class::Newer,
            None,
        ),
        (
            "edited file",
            Box::new(|| {
                put(&path, R2);
                fs::write(path.join("README.md"), "edited").unwrap();
                env.record(&path, "claude-code", v2.clone(), None);
            }),
            Class::Modified,
            Some("modified"),
        ),
        (
            "file replaced by link",
            Box::new(|| {
                put(&path, R2);
                fs::remove_file(path.join("README.md")).unwrap();
                symlink("/synthetic/readme", path.join("README.md")).unwrap();
                env.record(&path, "claude-code", v2.clone(), None);
            }),
            Class::Modified,
            Some("modified"),
        ),
        (
            "file deleted",
            Box::new(|| {
                put(&path, R2);
                fs::remove_file(path.join("NEW.md")).unwrap();
                env.record(&path, "claude-code", v2.clone(), None);
            }),
            Class::Modified,
            Some("modified"),
        ),
        (
            "same revision different content",
            Box::new(|| {
                put(&path, R1);
                let mut v = v1.clone();
                v.revision = 2;
                env.record(&path, "claude-code", v, None);
            }),
            Class::Modified,
            Some("content_differs"),
        ),
        (
            "recorded target replaced by a link",
            Box::new(|| {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                symlink("/synthetic/elsewhere", &path).unwrap();
                env.record(&path, "claude-code", v2.clone(), None);
            }),
            Class::Modified,
            Some("symlink"),
        ),
        (
            "pending upgrade completed",
            Box::new(|| {
                put(&path, R2);
                env.record(&path, "claude-code", v1.clone(), Some(v2.clone()));
            }),
            Class::OwnedCurrent,
            None,
        ),
        (
            "pending upgrade never started",
            Box::new(|| {
                put(&path, R1);
                env.record(&path, "claude-code", v1.clone(), Some(v2.clone()));
            }),
            Class::OwnedOutdated,
            None,
        ),
        (
            "pending upgrade mixed",
            Box::new(|| {
                put(&path, R1);
                fs::write(path.join("README.md"), "readme v2").unwrap();
                env.record(&path, "claude-code", v1.clone(), Some(v2.clone()));
            }),
            Class::Incomplete,
            Some("incomplete"),
        ),
        (
            "pending upgrade left an old-only file",
            Box::new(|| {
                put(&path, R2);
                fs::write(path.join("OLD.md"), "only in r1").unwrap();
                env.record(&path, "claude-code", v1.clone(), Some(v2.clone()));
            }),
            Class::Incomplete,
            Some("incomplete"),
        ),
        (
            "pending install with no files",
            Box::new(|| {
                fs::create_dir_all(&path).unwrap();
                let none = Version {
                    revision: 0,
                    files: BTreeMap::new(),
                };
                env.record(&path, "claude-code", none, Some(v2.clone()));
            }),
            Class::Incomplete,
            Some("incomplete"),
        ),
    ];
    for (name, setup, class, reason) in cases {
        reset(&path);
        env.forget();
        setup();
        let before = tree(env.dir.path());
        let (got, why, detail) = claude_class(&env, m);
        assert_eq!(got, class, "{name}");
        if let Some(reason) = reason {
            assert_eq!(why.as_deref(), Some(reason), "{name}");
        }
        if name == "existing link" {
            assert_eq!(detail.as_deref(), Some("-> /synthetic/elsewhere"));
        }
        assert_eq!(tree(env.dir.path()), before, "status wrote during {name}");
    }
    assert!(!env.state().join(LOCK).exists());
}

#[test]
fn enable_and_sync_install_report_conflicts_and_never_downgrade() {
    let env = Env::new();
    fs::create_dir_all(env.codex().parent().unwrap()).unwrap();
    symlink("/synthetic/old-skill", env.codex()).unwrap();
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    assert_eq!(receipt.trigger, Trigger::Enable);
    let claude = result(&receipt, "claude-code");
    assert_eq!(
        (claude.before, claude.result),
        (Class::Absent, Outcome::Installed)
    );
    for f in R2 {
        assert_eq!(fs::read(env.claude().join(f.path)).unwrap(), f.bytes);
    }
    let codex = result(&receipt, "codex");
    assert_eq!(codex.result, Outcome::Conflict);
    assert_eq!(codex.reason.as_deref(), Some("symlink"));
    assert_eq!(
        fs::read_link(env.codex()).unwrap(),
        Path::new("/synthetic/old-skill")
    );
    let record = env.res().read().unwrap();
    assert_eq!(record.targets.len(), 1);
    assert!(record.targets[0].pending.is_none());
    assert_eq!(record.targets[0].files, version(2, R2).files);
    let again = env.res().apply(r2(), Trigger::Sync, true);
    assert_eq!(result(&again, "claude-code").result, Outcome::Unchanged);

    // Missing targets are reinstalled only by explicit Enable/Sync.
    fs::remove_dir_all(env.claude()).unwrap();
    let sync = env.res().apply(r2(), Trigger::Sync, true);
    let claude = result(&sync, "claude-code");
    assert_eq!(
        (claude.before, claude.result),
        (Class::Missing, Outcome::Installed)
    );

    // Modified and incomplete targets are reported, never overwritten or repaired.
    fs::write(env.claude().join("README.md"), "user edit").unwrap();
    let before = tree(env.dir.path());
    let sync = env.res().apply(r2(), Trigger::Sync, true);
    let claude = result(&sync, "claude-code");
    assert_eq!(
        (claude.before, claude.result),
        (Class::Modified, Outcome::Conflict)
    );
    assert_eq!(tree(env.dir.path()), before);
    reset(&env.claude());
    put(&env.claude(), R1);
    fs::write(env.claude().join("SKILL.md"), "skill v2").unwrap();
    env.record(
        &env.claude(),
        "claude-code",
        version(1, R1),
        Some(version(2, R2)),
    );
    let before = tree(env.dir.path());
    for trigger in [Trigger::Enable, Trigger::Sync, Trigger::Startup] {
        let r = env.res().apply(r2(), trigger, true);
        assert_eq!(result(&r, "claude-code").before, Class::Incomplete);
        assert_eq!(result(&r, "claude-code").result, Outcome::Conflict);
    }
    assert_eq!(tree(env.dir.path()), before);

    // Upgrade replaces own files, removes old-only files and keeps user extras.
    reset(&env.claude());
    env.forget();
    env.res().apply(r1(), Trigger::Enable, true);
    fs::write(env.claude().join("EXTRA.md"), "mine").unwrap();
    let sync = env.res().apply(r2(), Trigger::Sync, true);
    let claude = result(&sync, "claude-code");
    assert_eq!(
        (claude.before, claude.result),
        (Class::OwnedOutdated, Outcome::Updated)
    );
    for f in R2 {
        assert_eq!(fs::read(env.claude().join(f.path)).unwrap(), f.bytes);
    }
    assert!(!env.claude().join("OLD.md").exists());
    assert_eq!(fs::read(env.claude().join("EXTRA.md")).unwrap(), b"mine");
    assert_eq!(env.res().read().unwrap().targets[0].revision, 2);

    // An older build never downgrades what a newer one installed.
    let before = tree(env.dir.path());
    for trigger in [Trigger::Enable, Trigger::Sync, Trigger::Startup] {
        let r = env.res().apply(r1(), trigger, true);
        let claude = result(&r, "claude-code");
        assert_eq!(
            (claude.before, claude.result),
            (Class::Newer, Outcome::Unchanged)
        );
    }
    assert_eq!(tree(env.dir.path()), before);
}

#[test]
fn absent_agent_homes_are_skipped_and_not_created() {
    let env = Env::new();
    fs::remove_dir(env.home().join(".agents")).unwrap();
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    let codex = result(&receipt, "codex");
    assert_eq!(codex.result, Outcome::Skipped);
    assert_eq!(codex.reason.as_deref(), Some("agent_absent"));
    assert!(!env.home().join(".agents").exists());
    assert_eq!(result(&receipt, "claude-code").result, Outcome::Installed);
}

#[test]
fn startup_only_upgrades_own_outdated_targets() {
    let env = Env::new();
    // Nothing recorded: startup reads only, creating neither state nor skill directories.
    let receipt = env.res().apply(r2(), Trigger::Startup, true);
    assert!(
        receipt.resources[0]
            .targets
            .iter()
            .all(|t| t.result == Outcome::Unchanged)
    );
    assert!(!env.state().exists());
    assert!(!env.home().join(".claude/skills").exists());

    let enable = env.res().apply(r1(), Trigger::Enable, true);
    assert_eq!(result(&enable, "codex").result, Outcome::Installed);
    fs::remove_dir_all(env.codex()).unwrap();
    let startup = env.res().apply(r2(), Trigger::Startup, true);
    let claude = result(&startup, "claude-code");
    assert_eq!(
        (claude.before, claude.result),
        (Class::OwnedOutdated, Outcome::Updated)
    );
    let codex = result(&startup, "codex");
    assert_eq!(
        (codex.before, codex.result),
        (Class::Missing, Outcome::Unchanged)
    );
    assert!(!env.codex().exists());

    fs::write(env.claude().join("SKILL.md"), "user edit").unwrap();
    let before = tree(env.dir.path());
    let startup = env.res().apply(r1(), Trigger::Startup, true);
    assert_eq!(result(&startup, "claude-code").result, Outcome::Conflict);
    assert_eq!(tree(env.dir.path()), before);
}

#[test]
fn remove_deletes_only_unmodified_owned_files() {
    let env = Env::new();
    env.res().apply(r2(), Trigger::Enable, true);
    fs::write(env.claude().join("EXTRA.md"), "mine").unwrap();
    let receipt = env.res().apply(r2(), Trigger::Remove, false);
    assert!(!receipt.enabled);
    let claude = result(&receipt, "claude-code");
    assert_eq!(claude.result, Outcome::Removed);
    assert_eq!(
        fs::read_dir(env.claude())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>(),
        ["EXTRA.md"]
    );
    assert_eq!(result(&receipt, "codex").result, Outcome::Removed);
    assert!(!env.codex().exists(), "empty directory is removed");
    assert!(env.home().join(".agents/skills").exists());
    assert!(env.res().read().unwrap().targets.is_empty());

    // Modified, foreign and missing targets.
    reset(&env.claude());
    env.res().apply(r2(), Trigger::Enable, true);
    fs::write(env.claude().join("README.md"), "user edit").unwrap();
    fs::remove_dir_all(env.codex()).unwrap();
    let before = tree(&env.home());
    let receipt = env.res().apply(r2(), Trigger::Remove, false);
    assert_eq!(result(&receipt, "claude-code").result, Outcome::Conflict);
    let codex = result(&receipt, "codex");
    assert_eq!(
        (codex.before, codex.result),
        (Class::Missing, Outcome::Removed)
    );
    assert_eq!(tree(&env.home()), before);
    assert_eq!(env.res().read().unwrap().targets.len(), 1);
    symlink("/synthetic/old-skill", env.codex()).unwrap();
    let receipt = env.res().apply(r2(), Trigger::Remove, false);
    assert_eq!(result(&receipt, "codex").result, Outcome::Conflict);
    assert!(fs::symlink_metadata(env.codex()).unwrap().is_symlink());
}

#[test]
fn failed_install_rolls_back_only_files_proven_from_this_round() {
    let env = Env::new();
    fail_once("write:SKILL.md");
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    let claude = result(&receipt, "claude-code");
    assert_eq!(claude.result, Outcome::Failed);
    assert!(!env.claude().exists(), "own files and directory removed");
    assert!(
        !env.home().join(".claude/skills").exists(),
        "created skills directory removed when empty"
    );
    assert_eq!(result(&receipt, "codex").result, Outcome::Installed);
    let record = env.res().read().unwrap();
    assert_eq!(record.targets.len(), 1, "failed target has no record");

    // A file changed after this round wrote it is neither deleted nor overwritten.
    let env = Env::new();
    let claude = env.claude();
    let mut done = false;
    on_fault(move |s| {
        if s == "write:SKILL.md" && !done {
            done = true;
            fs::write(claude.join("README.md"), "user edit").unwrap();
            return Err(io::Error::other("injected"));
        }
        Ok(())
    });
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    let claude = result(&receipt, "claude-code");
    assert_eq!(claude.result, Outcome::Failed);
    assert!(claude.detail.as_deref().unwrap().contains("README.md"));
    assert_eq!(
        fs::read(env.claude().join("README.md")).unwrap(),
        b"user edit"
    );
    assert!(!env.claude().join("NEW.md").exists());
    assert!(!env.claude().join("SKILL.md").exists());
    assert_eq!(claude_class(&env, r2()).0, Class::Incomplete);

    // The record is written before any directory change.
    let env = Env::new();
    fail_once("record");
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    assert_eq!(result(&receipt, "claude-code").result, Outcome::Failed);
    assert!(!env.claude().exists());
}

#[test]
fn failed_upgrade_and_remove_restore_the_previous_version() {
    let env = Env::new();
    env.res().apply(r1(), Trigger::Enable, true);
    fail_once("write:SKILL.md");
    let receipt = env.res().apply(r2(), Trigger::Sync, true);
    assert_eq!(result(&receipt, "claude-code").result, Outcome::Failed);
    for f in R1 {
        assert_eq!(fs::read(env.claude().join(f.path)).unwrap(), f.bytes);
    }
    assert!(!env.claude().join("NEW.md").exists());
    let record = env.res().read().unwrap();
    let entry = record
        .targets
        .iter()
        .find(|e| e.path == env.claude())
        .unwrap();
    assert_eq!(entry.revision, 1);
    assert!(entry.pending.is_none());
    assert_eq!(claude_class(&env, r2()).0, Class::OwnedOutdated);
    assert_eq!(result(&receipt, "codex").result, Outcome::Updated);

    fail_once("delete:README.md");
    let receipt = env.res().apply(r1(), Trigger::Remove, false);
    assert_eq!(result(&receipt, "claude-code").result, Outcome::Failed);
    for f in R1 {
        assert_eq!(fs::read(env.claude().join(f.path)).unwrap(), f.bytes);
    }
    assert_eq!(claude_class(&env, r1()).0, Class::OwnedCurrent);
}

#[test]
fn busy_lock_and_unreadable_record_write_nothing() {
    let env = Env::new();
    fs::create_dir_all(env.state()).unwrap();
    let held = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(env.state().join(LOCK))
        .unwrap();
    use std::os::fd::AsRawFd;
    assert_eq!(
        unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    assert!(
        receipt.resources[0]
            .targets
            .iter()
            .all(|t| t.result == Outcome::Busy)
    );
    assert!(!env.home().join(".claude/skills").exists());
    drop(held);

    fs::write(env.state().join(RECORD), "{broken").unwrap();
    assert!(env.res().status(r2()).is_err());
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    assert!(receipt.error.is_some());
    assert!(!env.home().join(".claude/skills").exists());
}

#[test]
fn invalid_resources_are_reported_and_never_written() {
    const NO_SKILL: &[ResourceFile] = &[ResourceFile {
        path: "README.md",
        bytes: b"x",
    }];
    const NESTED: &[ResourceFile] = &[
        ResourceFile {
            path: "SKILL.md",
            bytes: b"x",
        },
        ResourceFile {
            path: "sub/x.md",
            bytes: b"x",
        },
    ];
    const UP: &[ResourceFile] = &[
        ResourceFile {
            path: "SKILL.md",
            bytes: b"x",
        },
        ResourceFile {
            path: "../x.md",
            bytes: b"x",
        },
    ];
    const HIDDEN: &[ResourceFile] = &[
        ResourceFile {
            path: "SKILL.md",
            bytes: b"x",
        },
        ResourceFile {
            path: ".x.md",
            bytes: b"x",
        },
    ];
    const DUPLICATE: &[ResourceFile] = &[
        ResourceFile {
            path: "SKILL.md",
            bytes: b"x",
        },
        ResourceFile {
            path: "SKILL.md",
            bytes: b"y",
        },
    ];
    let env = Env::new();
    for resources in [
        vec![skill("../escape", 1, R1)],
        vec![skill(".hidden", 1, R1)],
        vec![skill("a/b", 1, R1)],
        vec![skill("", 1, R1)],
        vec![skill("skill-a", 0, R1)],
        vec![skill("skill-a", 1, NO_SKILL)],
        vec![skill("skill-a", 1, NESTED)],
        vec![skill("skill-a", 1, UP)],
        vec![skill("skill-a", 1, HIDDEN)],
        vec![skill("skill-a", 1, DUPLICATE)],
        vec![skill("skill-a", 1, R1), skill("skill-a", 2, R2)],
    ] {
        let m = plugin(resources);
        let status = env.res().status(m).unwrap();
        assert!(status.resources.iter().all(|r| r.error.is_some()));
        let receipt = env.res().apply(m, Trigger::Enable, true);
        assert!(receipt.resources.iter().all(|r| r.error.is_some()));
        assert!(!env.home().join(".claude/skills").exists());
        assert!(!env.state().exists());
    }
}

#[test]
fn setup_files_resolve_only_to_installed_own_files() {
    let env = Env::new();
    let status = env.res().status(r2()).unwrap();
    assert_eq!(status.setup_files[0].label, "Template");
    assert!(status.setup_files[0].paths.is_empty());
    assert!(status.setup_files[1].error.is_some());
    fs::create_dir_all(env.codex().parent().unwrap()).unwrap();
    symlink("/synthetic/old-skill", env.codex()).unwrap();
    env.res().apply(r2(), Trigger::Enable, true);
    let status = env.res().status(r2()).unwrap();
    assert_eq!(
        status.setup_files[0].paths,
        [env.claude().join("README.md")]
    );
    let receipt = env.res().apply(r2(), Trigger::Enable, true);
    let summary = receipt.summary();
    assert!(summary.starts_with("Enabled."), "{summary}");
    assert!(
        summary.contains("Codex conflict (existing link kept)"),
        "{summary}"
    );
    assert!(summary.contains("Nothing was overwritten"), "{summary}");
}
