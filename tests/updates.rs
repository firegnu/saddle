mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use saddle::{
    settings::{Outcome, Settings},
    updates::{self, Failure, Files, Pen, Saddle, Sources, Updates},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

/// Agents as the installed Corral's public status shows them: the same program, an identical
/// copy elsewhere, an older program, one without the protocol, one held, one that cannot be read,
/// one switching and one still starting.
const AGENTS: &str = r#"{"ok":true,"agents":[{"name":"p/new"},{"name":"p/copy"},{"name":"p/old"},{"name":"p/legacy"},{"name":"p/held"},{"name":"p/lost"},{"name":"p/busy"},{"name":"p/start","starting":true}]}"#;
/// `upgrade --all` exits 0 with failed items in its answer, as the public contract says it may.
const RECEIPT: &str = r#"{"ok":false,"helper":{"result":"complete"},"legacy_after":"unrecorded workers cannot be discovered or migrated","results":[{"ok":true,"name":"p/new","result":"already_current"},{"ok":false,"name":"p/old","result":"failed","error":{"ok":false,"error":"busy","message":"upgrade refused"},"epoch":"e1","attempt":1},{"ok":false,"name":"p/lost","result":"unknown","epoch":"e2","attempt":1},{"ok":false,"name":"p/legacy","result":"needs_restart","message":"this pen has no upgrade protocol; no restart attempted"},{"ok":false,"name":"p/copy","result":"failed","pen_result":"complete","after":[{"request_id":"r1","result":"failed","handover":{"state":"failed","error":{"ok":false,"error":"spawn","message":"worker failed"}}},{"request_id":"r2","result":"complete"}]}]}"#;

struct Install {
    _dir: tempfile::TempDir,
    root: PathBuf,
    log: PathBuf,
    /// The `saddle` command entry: a link into the installed package.
    command: PathBuf,
    running: PathBuf,
    new_corral: PathBuf,
}

fn status(exe: &str, state: &str, extra: &str) -> String {
    format!(
        r#"{{"ok":true,"proto":1,"exe":"{exe}","custody":"owner","capabilities":{{"upgrade":1,"recover":1,"snapshot":1}},"upgrade":{{"state":"{state}"{extra}}}}}"#
    )
}

/// Two immutable packages, as package.sh builds them: the running saddle in `old`, the installed
/// one in `new`, with the command entry linked to `new`. The Corral beside the running saddle
/// logs any use, which must never happen.
fn install() -> Install {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let log = root.join("calls.log");
    let (old, new, copy) = (
        root.join("old/bin"),
        root.join("new/bin"),
        root.join("copy/bin"),
    );
    for bin in [&old, &new, &copy] {
        fs::create_dir_all(bin).unwrap();
    }
    let corral = |name: &str| root.join(name).join("bin/corral").display().to_string();
    let (new_exe, copy_exe, old_exe) = (corral("new"), corral("copy"), corral("old"));
    let fake = format!(
        r#"#!/bin/sh
echo "$*" >> '{log}'
case "$1:$2" in
  ls:) echo '{AGENTS}' ;;
  status:p/new) echo '{new}' ;;
  status:p/copy) echo '{copy}' ;;
  status:p/old) echo '{old}' ;;
  status:p/legacy) echo '{{"ok":true,"name":"p/legacy","proto":1}}' ;;
  status:p/held) echo '{held}' ;;
  status:p/lost) echo '{{"ok":false,"error":"timeout","message":"no answer"}}'; exit 1 ;;
  status:p/busy) echo '{busy}' ;;
  upgrade:--all) sleep 1; echo '{RECEIPT}' ;;
  *) echo '{{"ok":false,"error":"unexpected"}}'; exit 1 ;;
esac
"#,
        log = log.display(),
        new = status(&new_exe, "none", r#","result":"complete""#),
        copy = status(&copy_exe, "none", ""),
        old = status(
            &old_exe,
            "none",
            r#","result":"failed","last_error":"probe refused""#
        ),
        held = status(
            &old_exe,
            "hold_unprotected",
            r#","last_error":"resume failed""#
        ),
        busy = status(&new_exe, "running_new_pending", ""),
    );
    common::script(&new, "corral", &fake);
    common::script(&copy, "corral", &fake);
    common::script(
        &old,
        "corral",
        &format!("#!/bin/sh\necho \"old $*\" >> '{}'\n", log.display()),
    );
    common::script(&new, "saddle", "#!/bin/sh\n# saddle new\n");
    common::script(&old, "saddle", "#!/bin/sh\n# saddle old\n");
    fs::create_dir_all(root.join("links")).unwrap();
    std::os::unix::fs::symlink(new.join("saddle"), root.join("links/saddle")).unwrap();
    Install {
        command: root.join("links/saddle"),
        running: old.join("saddle"),
        new_corral: new.join("corral"),
        log,
        root,
        _dir: dir,
    }
}

fn sources(install: &Install) -> Sources {
    Sources {
        running: Ok(install.running.clone()),
        command: install.command.display().to_string(),
        corral: "corral".into(),
    }
}

fn calls(install: &Install) -> String {
    fs::read_to_string(&install.log).unwrap_or_default()
}

fn wait(updates: &mut Updates, done: impl Fn(&Updates) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done(updates) {
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(20));
        updates.poll();
    }
}

fn screen(settings: &mut Settings) -> String {
    use ratatui::{Terminal, backend::TestBackend};
    let mut terminal = Terminal::new(TestBackend::new(120, 50)).unwrap();
    terminal
        .draw(|frame| {
            settings.draw(&saddle::theme::Theme::default(), frame);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..50)
        .map(|y| {
            (0..120)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn press(settings: &mut Settings, code: KeyCode) -> Outcome {
    settings.key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn the_check_reads_the_installed_targets_and_tells_each_agent_apart() {
    let install = install();
    let check = updates::check(
        &sources(&install),
        &mut Files::default(),
        Duration::from_secs(10),
        &AtomicBool::new(false),
    );
    assert_eq!(
        check.saddle,
        Saddle::Reopen(install.root.join("new/bin/saddle"))
    );
    // The installed package's Corral, not the one beside the running saddle.
    assert_eq!(check.corral, Ok(install.new_corral.clone()));
    let agents = check.agents.unwrap();
    let pen = |name: &str| {
        agents
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} missing: {agents:?}"))
            .1
            .clone()
    };
    assert_eq!(pen("p/new"), Pen::Current);
    // Same content elsewhere is not an update.
    assert_eq!(pen("p/copy"), Pen::Current);
    assert_eq!(pen("p/old"), Pen::Outdated(Some("probe refused".into())));
    assert_eq!(pen("p/legacy"), Pen::NoProtocol);
    assert_eq!(
        pen("p/held"),
        Pen::Held {
            state: "hold_unprotected".into(),
            error: Some("resume failed".into())
        }
    );
    assert!(matches!(pen("p/lost"), Pen::Unknown(e) if e.contains("no answer")));
    assert_eq!(pen("p/busy"), Pen::Pending("running_new_pending".into()));
    assert!(matches!(pen("p/start"), Pen::Unknown(_)));
    let calls = calls(&install);
    assert!(
        !calls.lines().any(|line| line.starts_with("old ")),
        "the running package's corral ran:\n{calls}"
    );
    assert!(!calls.contains("upgrade"), "a check upgraded:\n{calls}");
}

#[test]
fn an_identical_saddle_elsewhere_is_current_and_explicit_corral_config_is_kept() {
    let install = install();
    let elsewhere = install.root.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    fs::copy(
        install.root.join("new/bin/saddle"),
        elsewhere.join("saddle"),
    )
    .unwrap();
    let custom = install.root.join("custom");
    fs::create_dir_all(&custom).unwrap();
    fs::copy(&install.new_corral, custom.join("corral")).unwrap();
    let check = updates::check(
        &Sources {
            running: Ok(elsewhere.join("saddle")),
            corral: custom.join("corral").display().to_string(),
            ..sources(&install)
        },
        &mut Files::default(),
        Duration::from_secs(10),
        &AtomicBool::new(false),
    );
    assert_eq!(check.saddle, Saddle::Current);
    assert_eq!(check.corral, Ok(custom.join("corral")));
    // Without an installed saddle nothing is called up to date.
    let check = updates::check(
        &Sources {
            command: install.root.join("missing/saddle").display().to_string(),
            ..sources(&install)
        },
        &mut Files::default(),
        Duration::from_secs(10),
        &AtomicBool::new(false),
    );
    assert!(
        matches!(check.saddle, Saddle::Unknown(_)),
        "{:?}",
        check.saddle
    );
    assert!(check.corral.is_err());
}

#[test]
fn upgrade_all_runs_once_per_click_and_keeps_every_item_of_its_receipt() {
    let install = install();
    let mut updates = Updates::start(sources(&install), Duration::from_secs(3600));
    wait(&mut updates, |u| u.latest().is_some());
    assert!(updates.attention());
    assert!(updates.can_upgrade());
    let before = calls(&install);
    assert!(!before.contains("upgrade"), "{before}");
    assert!(updates.upgrade());
    // Repeated clicks, refreshes and the page reopening while it runs do not upgrade again.
    assert!(!updates.upgrade());
    updates.refresh();
    let mut settings = Settings::open(install.root.join("config.toml"), true);
    settings.open_updates();
    settings.set_updates(updates.page());
    assert!(screen(&mut settings).contains("Upgrading"));
    assert!(!updates.upgrade());
    wait(&mut updates, |u| u.receipt().is_some());
    // Until a check made after it arrives, the receipt is not taken as the current state.
    assert!(updates.checking());
    assert!(!updates.can_upgrade());
    let receipt = updates.receipt().unwrap().clone();
    let upgrades: Vec<_> = calls(&install)
        .lines()
        .filter(|l| l.starts_with("upgrade"))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        upgrades,
        [format!(
            "upgrade --all --exe {}",
            install.new_corral.display()
        )]
    );
    assert_eq!(receipt.outcome, Ok(()));
    assert_eq!(receipt.items.len(), 5);
    let item = |name: &str| receipt.items.iter().find(|i| i.name == name).unwrap();
    assert_eq!(item("p/new").pen, "already_current");
    assert_eq!(item("p/old").pen, "failed");
    assert!(
        item("p/old")
            .reason
            .as_deref()
            .unwrap()
            .contains("upgrade refused")
    );
    assert_eq!(item("p/lost").pen, "unknown");
    assert_eq!(item("p/legacy").pen, "needs_restart");
    let copy = item("p/copy");
    assert_eq!(copy.pen, "complete");
    assert_eq!(copy.after.len(), 2);
    assert_eq!(copy.after[0].request_id.as_deref(), Some("r1"));
    assert_eq!(copy.after[0].result, "failed");
    assert!(
        copy.after[0]
            .reason
            .as_deref()
            .unwrap()
            .contains("worker failed")
    );
    assert!(receipt.legacy.is_some());
    // The status check after it still finds agents to look after; the failed reminder handover
    // keeps the dot whatever the statuses say.
    wait(&mut updates, |u| !u.checking());
    assert!(updates.attention());
    settings.set_updates(updates.page());
    let shown = screen(&mut settings);
    for text in [
        "Updates",
        "Reopen",
        "Upgrade all",
        "p/old",
        "upgrade refused",
        "unknown",
        "r1",
        "worker failed",
        "corral recover p/held",
    ] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
}

#[test]
fn a_lost_or_refused_receipt_is_unknown_or_failed_never_an_empty_success() {
    let dir = tempfile::tempdir().unwrap();
    let slow = common::script(dir.path(), "slow", "#!/bin/sh\nsleep 5\n");
    let receipt = updates::upgrade(Path::new(&slow), Duration::from_millis(200));
    assert!(matches!(receipt.outcome, Err(Failure::Unknown(_))));
    let garbage = common::script(dir.path(), "garbage", "#!/bin/sh\necho done\n");
    let receipt = updates::upgrade(Path::new(&garbage), Duration::from_secs(5));
    assert!(matches!(receipt.outcome, Err(Failure::Unknown(_))));
    let refused = common::script(
        dir.path(),
        "refused",
        "#!/bin/sh\necho '{\"ok\":false,\"error\":\"usage\",\"message\":\"bad target\"}'\nexit 2\n",
    );
    let receipt = updates::upgrade(Path::new(&refused), Duration::from_secs(5));
    assert!(matches!(&receipt.outcome, Err(Failure::Failed(e)) if e.contains("bad target")));
}

#[test]
fn the_updates_page_offers_upgrade_all_only_when_the_check_does() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::open(dir.path().join("config.toml"), true);
    assert!(matches!(
        press(&mut settings, KeyCode::F(6)),
        Outcome::CheckUpdates
    ));
    settings.set_updates(updates::Page::default());
    assert!(matches!(
        press(&mut settings, KeyCode::Char('u')),
        Outcome::Stay
    ));
    settings.set_updates(updates::Page {
        rows: vec![updates::Row::Item(
            "p/old".into(),
            "upgrade available".into(),
            updates::Tone::Action,
        )],
        upgrade: true,
    });
    let shown = screen(&mut settings);
    assert!(
        shown.contains("Upgrade all") && shown.contains("p/old"),
        "{shown}"
    );
    assert!(matches!(
        press(&mut settings, KeyCode::Char('u')),
        Outcome::Upgrade
    ));
    assert!(matches!(
        settings.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
        Outcome::Stay
    ));
    assert!(matches!(
        press(&mut settings, KeyCode::Char('r')),
        Outcome::CheckUpdates
    ));
    // Typing on the page edits no setting.
    settings.paste("99");
    press(&mut settings, KeyCode::F(1));
    assert_eq!(settings.value("left_width"), Some("52"));
}

#[test]
fn malformed_or_nonzero_receipts_keep_partial_results_without_claiming_success() {
    let dir = tempfile::tempdir().unwrap();
    for (value, code) in [
        (
            r#"{"ok":false,"helper":{"result":"complete"},"results":[]}"#,
            0,
        ),
        (
            r#"{"ok":true,"helper":{"result":"complete"},"results":[{"result":"complete"}]}"#,
            0,
        ),
        (
            r#"{"ok":true,"helper":{"result":"complete"},"results":[{"name":"p/one","result":"complete","after":"broken"}]}"#,
            0,
        ),
        (
            r#"{"ok":true,"helper":{"result":"complete"},"results":[{"name":"p/one","result":"complete"}]}"#,
            1,
        ),
    ] {
        let script = common::script(
            dir.path(),
            "receipt",
            &format!("#!/bin/sh\necho '{value}'\nexit {code}\n"),
        );
        let receipt = updates::upgrade(Path::new(&script), Duration::from_secs(5));
        assert!(
            matches!(receipt.outcome, Err(Failure::Unknown(_))),
            "{receipt:?}"
        );
        assert_eq!(
            receipt.items.len(),
            serde_json::from_str::<serde_json::Value>(value).unwrap()["results"]
                .as_array()
                .unwrap()
                .len()
        );
    }
}

#[test]
fn lost_receipt_stays_unknown_across_refresh_and_page_reopening_without_replay() {
    let install = install();
    let script = fs::read_to_string(&install.new_corral)
        .unwrap()
        .replace("sleep 1; echo", "echo done; exit 0; echo");
    common::script(install.new_corral.parent().unwrap(), "corral", &script);
    let mut updates = Updates::start(sources(&install), Duration::from_secs(3600));
    wait(&mut updates, |u| u.latest().is_some());
    assert!(updates.upgrade());
    wait(&mut updates, |u| u.receipt().is_some());
    wait(&mut updates, |u| !u.checking());
    updates.refresh();
    wait(&mut updates, |u| !u.checking());
    assert!(updates.attention());
    assert!(!updates.upgrade());
    let mut settings = Settings::open(install.root.join("config.toml"), true);
    settings.open_updates();
    settings.set_updates(updates.page());
    assert!(matches!(
        press(&mut settings, KeyCode::Char('u')),
        Outcome::Stay
    ));
    assert_eq!(
        calls(&install)
            .lines()
            .filter(|l| l.starts_with("upgrade"))
            .count(),
        1
    );
}

#[test]
fn updates_tab_is_reachable_from_plugins_by_keyboard() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = saddle::plugins::Manager::open(dir.path().join("plugins.toml"));
    let mut page = saddle::plugins::ui::Page::default();
    let outcome = page.event(
        crossterm::event::Event::Key(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE)),
        &mut manager,
    );
    assert!(matches!(outcome, saddle::plugins::ui::Outcome::Page(_)));
}

#[test]
fn cached_checks_notice_replaced_programs_and_mouse_actions_match_keys() {
    let install = install();
    let running = install.root.join("running-saddle");
    let installed = install.root.join("new/bin/saddle");
    fs::copy(&installed, &running).unwrap();
    let sources = Sources {
        running: Ok(running),
        ..sources(&install)
    };
    let mut files = Files::default();
    let check = updates::check(
        &sources,
        &mut files,
        Duration::from_secs(5),
        &AtomicBool::new(false),
    );
    assert_eq!(check.saddle, Saddle::Current);
    let text = fs::read_to_string(&installed)
        .unwrap()
        .replace("new", "NEW");
    fs::write(&installed, text).unwrap();
    let check = updates::check(
        &sources,
        &mut files,
        Duration::from_secs(5),
        &AtomicBool::new(false),
    );
    assert_eq!(check.saddle, Saddle::Reopen(installed));

    use ratatui::{Terminal, backend::TestBackend};
    let mut settings = Settings::open(install.root.join("config.toml"), true);
    settings.open_updates();
    settings.set_updates(updates::Page {
        rows: vec![],
        upgrade: true,
    });
    for (width, height) in [(120, 50), (32, 14), (1, 1)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut hits = vec![];
        terminal
            .draw(|frame| hits = settings.draw(&saddle::theme::Theme::default(), frame))
            .unwrap();
        if width == 120 {
            let upgrade = hits
                .iter()
                .find(|h| h.key.code == KeyCode::Char('u'))
                .unwrap();
            let mut pointer = saddle::buttons::Pointer::default();
            let targets = vec![(saddle::input::Focus::Agents, upgrade.clone())];
            let mouse = |kind| crossterm::event::MouseEvent {
                kind,
                column: upgrade.area.x,
                row: upgrade.area.y,
                modifiers: KeyModifiers::NONE,
            };
            assert!(
                pointer
                    .event(
                        mouse(crossterm::event::MouseEventKind::Down(
                            crossterm::event::MouseButton::Left
                        )),
                        &targets
                    )
                    .is_none()
            );
            let (_, key) = pointer
                .event(
                    mouse(crossterm::event::MouseEventKind::Up(
                        crossterm::event::MouseButton::Left,
                    )),
                    &targets,
                )
                .unwrap();
            assert!(matches!(settings.key(key), Outcome::Upgrade));
        }
    }
}

#[test]
fn pending_pen_and_reminder_clear_only_after_public_verification() {
    let install = install();
    let status_file = install.root.join("status.json");
    let after_file = install.root.join("after.json");
    let script = format!(
        r#"#!/bin/sh
echo "$*" >> '{}'
case "$1" in
ls) echo '{{"ok":true,"agents":[{{"name":"p/one"}}]}}' ;;
status) cat '{}' ;;
after) cat '{}' ;;
upgrade) echo '{{"ok":false,"helper":{{"result":"complete"}},"results":[{{"name":"p/one","instance":"i1","result":"pending","epoch":"e1","attempt":1,"after":[{{"request_id":"r1","result":"pending","handover":{{"token":"h1"}}}}]}}]}}' ;;
esac
"#,
        install.log.display(),
        status_file.display(),
        after_file.display()
    );
    common::script(install.new_corral.parent().unwrap(), "corral", &script);
    let write_status = |exe: &Path, result: &str, epoch: &str| {
        fs::write(&status_file, serde_json::json!({"ok":true,"instance":"i1","exe":exe,"custody":"owner","capabilities":{"upgrade":1},"upgrade":{"state":"none","result":result,"epoch":epoch,"attempt":1,"target":install.new_corral}}).to_string()).unwrap();
    };
    write_status(&install.root.join("old/bin/corral"), "complete", "e0");
    fs::write(&after_file, r#"{"ok":true,"records":[]}"#).unwrap();
    let mut updates = Updates::start(
        Sources {
            running: Ok(install.root.join("new/bin/saddle")),
            ..sources(&install)
        },
        Duration::from_secs(3600),
    );
    wait(&mut updates, |u| u.latest().is_some());
    assert!(updates.upgrade());
    wait(&mut updates, |u| u.receipt().is_some());
    wait(&mut updates, |u| !u.checking());
    assert!(updates.attention());
    assert!(!updates.can_upgrade());
    // Exe and even complete from a different epoch cannot settle this request.
    write_status(&install.new_corral, "complete", "e0");
    updates.refresh();
    wait(&mut updates, |u| !u.checking());
    assert!(updates.attention());
    write_status(&install.new_corral, "complete", "e1");
    updates.refresh();
    wait(&mut updates, |u| !u.checking());
    assert!(updates.attention(), "the reminder is still unverified");
    fs::write(&after_file, serde_json::json!({"ok":true,"records":[{"request_id":"r1","phase":"waiting","worker":{"exe":install.new_corral},"handover":{"state":"complete","token":"h1","target":install.new_corral}}]}).to_string()).unwrap();
    let mut wrong_target: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&status_file).unwrap()).unwrap();
    wrong_target["upgrade"]["target"] = serde_json::json!(install.root.join("old/bin/corral"));
    fs::write(&status_file, wrong_target.to_string()).unwrap();
    updates.refresh();
    wait(&mut updates, |u| !u.checking());
    assert!(
        updates.attention(),
        "completion must match the upgrade target too"
    );
    write_status(&install.new_corral, "complete", "e1");
    let mut recovered: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&status_file).unwrap()).unwrap();
    recovered["upgrade"]["attempt"] = serde_json::json!(2);
    fs::write(&status_file, recovered.to_string()).unwrap();
    updates.refresh();
    wait(&mut updates, |u| !u.checking());
    assert!(
        !updates.attention(),
        "verified pen and reminder must clear the dot"
    );
    assert_eq!(
        calls(&install)
            .lines()
            .filter(|l| l.starts_with("upgrade"))
            .count(),
        1
    );
}
