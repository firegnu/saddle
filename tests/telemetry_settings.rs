//! Settings → General `Telemetry recording`: the same Store switch that `saddle agent` obeys.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use saddle::settings::{Outcome, RECORDING, Settings};
use saddle::telemetry::{EventQuery, SettingInput, Store};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn press(settings: &mut Settings, code: KeyCode) -> Outcome {
    settings.key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn ctrl(settings: &mut Settings, c: char) -> Outcome {
    settings.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}
fn open(config: &Path, root: &Path) -> Settings {
    Settings::open(config.to_path_buf(), true).with_telemetry(Ok(Store::new(root.to_path_buf())))
}
/// The switch is the last General field; Up wraps to it from the first.
fn toggle(settings: &mut Settings) {
    press(settings, KeyCode::F(1));
    press(settings, KeyCode::Up);
    press(settings, KeyCode::Char(' '));
}
fn state(root: &Path) -> (bool, i64) {
    let settings = Store::new(root.to_path_buf()).settings().unwrap();
    (
        settings["enabled"].as_bool().unwrap(),
        settings["generation"].as_i64().unwrap(),
    )
}
fn external(root: &Path, enabled: bool) {
    Store::new(root.to_path_buf())
        .set_recording(
            None,
            SettingInput {
                schema_version: 1,
                enabled,
                actor: "synthetic cli".into(),
            },
        )
        .unwrap();
}
fn screen(settings: &mut Settings, width: u16, height: u16) -> String {
    use ratatui::{Terminal, backend::TestBackend};
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            settings.draw(&saddle::theme::Theme::default(), frame);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                + "\n"
        })
        .collect()
}

#[test]
fn switch_reads_without_initializing_and_only_save_writes_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let (config, root) = (dir.path().join("config.toml"), dir.path().join("telemetry"));
    let mut settings = open(&config, &root);
    assert_eq!(settings.value(RECORDING), Some("false"));
    let shown = screen(&mut settings, 80, 24);
    assert!(shown.contains("Telemetry recording [Disabled"), "{shown}");
    assert!(shown.contains("explicitly chosen"), "{shown}");
    assert!(shown.contains("history is kept"), "{shown}");
    assert!(!root.exists(), "opening must not initialize storage");
    // A click on the drawn row toggles the draft, as for the other switches.
    let row = shown
        .lines()
        .position(|l| l.contains("Telemetry recording"))
        .unwrap();
    settings.click(ratatui::layout::Position::new(10, row as u16));
    assert_eq!(settings.value(RECORDING), Some("true"));
    settings.click(ratatui::layout::Position::new(10, row as u16));
    assert_eq!(settings.value(RECORDING), Some("false"));
    assert!(!root.exists(), "a draft must not touch storage");
    toggle(&mut settings);
    assert_eq!(settings.value(RECORDING), Some("true"));
    assert!(!root.exists(), "a draft must not touch storage");
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    assert!(!root.exists(), "Cancel must not touch storage");

    let mut settings = open(&config, &root);
    toggle(&mut settings);
    let outcome = ctrl(&mut settings, 's');
    assert!(
        matches!(outcome, Outcome::Recorded),
        "{}",
        settings.message()
    );
    assert!(
        settings.message().contains("Enabled"),
        "{}",
        settings.message()
    );
    assert_eq!(state(&root), (true, 1));
    assert!(
        !config.exists(),
        "the switch is never stored in config.toml"
    );

    // Reopening shows the stored state; saving without edits writes nothing.
    let mut settings = open(&config, &root);
    assert_eq!(settings.value(RECORDING), Some("true"));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Cancel));
    // Turning a value back to what was read is not an edit either.
    toggle(&mut settings);
    press(&mut settings, KeyCode::Char(' '));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Cancel));
    assert_eq!(state(&root), (true, 1));
}

#[test]
fn an_external_switch_change_is_never_overwritten_by_a_stale_draft() {
    let dir = tempfile::tempdir().unwrap();
    let (config, root) = (dir.path().join("config.toml"), dir.path().join("telemetry"));
    external(&root, true);
    // Untouched, the stale display is not written back when other settings are saved.
    let mut settings = open(&config, &root);
    assert_eq!(settings.value(RECORDING), Some("true"));
    external(&root, false);
    ctrl(&mut settings, 'u');
    settings.paste("60");
    let Outcome::Saved(saved, _) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message());
    };
    assert_eq!(saved.left_width, 60);
    assert_eq!(state(&root), (false, 2));

    // Edited, the change is refused and Settings offers to reread.
    let mut settings = open(&config, &root);
    assert_eq!(settings.value(RECORDING), Some("false"));
    toggle(&mut settings);
    external(&root, true);
    external(&root, false);
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(settings.conflict(), "{}", settings.message());
    assert_eq!(state(&root), (false, 4));
    let shown = screen(&mut settings, 80, 24);
    assert!(shown.contains("Telemetry recording changed"), "{shown}");
    // Keep my edits rereads the store underneath the draft; only the next Save writes it.
    press(&mut settings, KeyCode::Char('k'));
    assert!(!settings.conflict());
    assert_eq!(settings.value(RECORDING), Some("true"));
    assert_eq!(state(&root), (false, 4));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Recorded));
    assert_eq!(state(&root), (true, 5));

    // Discard my edits takes the store as it is now.
    let mut settings = open(&config, &root);
    toggle(&mut settings);
    external(&root, false);
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    press(&mut settings, KeyCode::Char('d'));
    assert_eq!(settings.value(RECORDING), Some("false"));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Cancel));
    assert_eq!(state(&root), (false, 6));
}

#[test]
fn unreadable_or_locked_storage_is_reported_and_does_not_block_config_edits() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    // A file where the storage folder belongs: the state is unknown, not "Disabled".
    let blocked = dir.path().join("blocked");
    fs::write(&blocked, "").unwrap();
    let mut settings = open(&config, &blocked);
    assert_eq!(settings.value(RECORDING), Some(""));
    let shown = screen(&mut settings, 80, 24);
    assert!(shown.contains("[Unknown"), "{shown}");
    assert!(!shown.contains("[Disabled"), "{shown}");
    ctrl(&mut settings, 'u');
    settings.paste("60");
    let Outcome::Saved(saved, _) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message());
    };
    assert_eq!(saved.left_width, 60);
    // Choosing a state that cannot be checked keeps the draft and saves nothing there.
    let mut settings = open(&config, &blocked);
    toggle(&mut settings);
    let outcome = ctrl(&mut settings, 's');
    assert!(matches!(outcome, Outcome::Stay), "{}", settings.message());
    assert!(
        settings.message().contains("Telemetry recording not saved"),
        "{}",
        settings.message()
    );
    assert_eq!(settings.value(RECORDING), Some("true"));
    assert_eq!(fs::read(&blocked).unwrap(), b"");

    // A held write lock: the config part is saved, the switch reports failure and stays drafted.
    let root = dir.path().join("telemetry");
    external(&root, false);
    let lock = rusqlite::Connection::open(root.join("telemetry.sqlite3")).unwrap();
    let mut settings = open(&config, &root);
    toggle(&mut settings);
    press(&mut settings, KeyCode::Down);
    ctrl(&mut settings, 'u');
    settings.paste("61");
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let Outcome::Applied(saved, _) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message());
    };
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(saved.left_width, 61);
    assert!(
        settings.message().contains("Telemetry recording not saved"),
        "{}",
        settings.message()
    );
    assert_eq!(settings.value(RECORDING), Some("true"));
    assert_eq!(state(&root), (false, 0));
    // Retrying writes only what is still a draft.
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Recorded));
    assert_eq!(state(&root), (true, 1));
    assert!(
        fs::read_to_string(&config)
            .unwrap()
            .contains("left_width = 61")
    );

    // The reverse: the switch is committed, the config write fails; neither is undone or redone.
    let link = dir.path().join("dangling.toml");
    std::os::unix::fs::symlink(dir.path().join("missing/config.toml"), &link).unwrap();
    let mut settings = open(&link, &root);
    toggle(&mut settings);
    press(&mut settings, KeyCode::Down);
    ctrl(&mut settings, 'u');
    settings.paste("62");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    let message = settings.message().to_owned();
    assert!(
        message.contains("Telemetry recording saved: Disabled"),
        "{message}"
    );
    assert!(message.contains("Config not saved"), "{message}");
    assert_eq!(state(&root), (false, 2));
    assert_eq!(settings.value("left_width"), Some("62"));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert_eq!(state(&root), (false, 2));
}

/// A fake Corral that counts its calls and, for `send`, can wait on a test barrier.
fn fake_corral(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("fake corral");
    fs::write(
        &path,
        "#!/usr/bin/python3\nimport os, json, time\nwith open(os.environ['CALLS'], 'ab') as f: f.write(b'call\\n')\nopen(os.environ['READY'], 'w').close()\nwhile not os.path.exists(os.environ['RELEASE']): time.sleep(0.005)\nprint(json.dumps({'ok':True,'name':'synthetic/name','instance':'0123456789ab','confirmed':True}))\n",
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
/// The host final receipt: the last stderr line, matching the first line's call id.
fn receipt(stderr: &[u8]) -> Value {
    let text = String::from_utf8_lossy(stderr);
    let line = |l: &str| -> Value {
        serde_json::from_str(l.strip_prefix("saddle-telemetry: ").unwrap()).unwrap()
    };
    let (first, last) = (
        line(text.lines().next().unwrap()),
        line(text.lines().last().unwrap()),
    );
    assert_eq!(first["call_id"], last["call_id"]);
    assert_eq!(last["final"], true);
    last
}
/// Kills the owned process group if a test fails before the process exits.
struct Owned(Option<std::process::Child>);
impl Drop for Owned {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
            let _ = child.wait();
        }
    }
}

#[test]
fn saved_switch_controls_agent_capture_while_business_still_runs() {
    use std::os::unix::process::CommandExt;
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    // The same location `saddle agent` derives from XDG_STATE_HOME.
    let root = dir.path().join("state/saddle/telemetry");
    let corral = fake_corral(dir.path());
    let calls = || {
        fs::read_to_string(dir.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    };
    let begins = || {
        Store::new(root.clone())
            .events(&EventQuery::default())
            .unwrap()["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "agent.send.begin")
            .count()
    };
    let switch = |enabled: bool| {
        let mut settings = open(&config, &root);
        if settings.value(RECORDING) != Some(&enabled.to_string()) {
            toggle(&mut settings);
            assert!(
                matches!(ctrl(&mut settings, 's'), Outcome::Recorded),
                "{}",
                settings.message()
            );
        }
        assert_eq!(state(&root).0, enabled);
    };
    let agent = || {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_saddle"));
        cmd.arg("agent")
            .arg("--corral")
            .arg(&corral)
            .arg("--record-context")
            .arg(dir.path().join("context.json"))
            .args(["--", "send", "synthetic/name", "message"])
            .env("HOME", dir.path())
            .env("XDG_STATE_HOME", dir.path().join("state"))
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("SADDLE_RUNTIME_DIR", dir.path().join("runtime"))
            .env("CALLS", dir.path().join("calls"))
            .env("READY", dir.path().join("ready"))
            .env("RELEASE", dir.path().join("release"))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .process_group(0);
        Owned(Some(cmd.spawn().unwrap()))
    };
    let finish = |mut owned: Owned| {
        let out = owned.0.take().unwrap().wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        receipt(&out.stderr)
    };

    switch(true);
    let store = Store::new(root.clone());
    store
        .create_trace(
            serde_json::from_value(
                json!({"schema_version":1,"trace_id":"t","origin":"ad_hoc","label":"synthetic"}),
            )
            .unwrap(),
        )
        .unwrap();
    store
        .create_dispatch(
            serde_json::from_value(
                json!({"schema_version":1,"trace_id":"t","dispatch_id":"d","kind":"implementation"}),
            )
            .unwrap(),
        )
        .unwrap();
    fs::write(
        dir.path().join("context.json"),
        json!({"schema_version":1,"trace_id":"t","dispatch_id":"d","send_kind":"initial"})
            .to_string(),
    )
    .unwrap();

    // Off then on through Settings while a call is in flight: its old generation stays revoked.
    let running = agent();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !dir.path().join("ready").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "fake corral never started"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    switch(false);
    switch(true);
    fs::write(dir.path().join("release"), b"").unwrap();
    let first = finish(running);
    assert_eq!(
        (first["executed"].as_bool(), first["begin"].as_str()),
        (Some(true), Some("stored"))
    );
    assert_eq!(first["end"], "disabled");
    assert_eq!((calls(), begins()), (1, 1));

    // Off: the business call still runs once; nothing new is captured.
    switch(false);
    let off = finish(agent());
    assert_eq!(
        (off["executed"].as_bool(), off["begin"].as_str()),
        (Some(true), Some("disabled"))
    );
    assert_eq!((calls(), begins()), (2, 1));

    // On again: new calls with explicit context are captured.
    switch(true);
    let on = finish(agent());
    assert_eq!(
        (on["begin"].as_str(), on["end"].as_str()),
        (Some("stored"), Some("stored"))
    );
    assert_eq!((calls(), begins()), (3, 2));
}

/// The visible text inside the Settings box, one row per line joined by spaces, so a phrase that
/// wraps onto the next row still reads as one.
fn visible(screen: &str) -> String {
    screen
        .lines()
        .map(|l| l.trim().trim_matches('┃').trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_saved_restart_setting_and_a_failed_switch_both_show_at_normal_size() {
    let dir = tempfile::tempdir().unwrap();
    let (config, root) = (dir.path().join("config.toml"), dir.path().join("telemetry"));
    external(&root, false);
    let lock = rusqlite::Connection::open(root.join("telemetry.sqlite3")).unwrap();
    let mut settings = open(&config, &root);
    toggle(&mut settings);
    // From the switch, Down wraps to Sidebar width, then Refresh interval (restart required).
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Down);
    ctrl(&mut settings, 'u');
    settings.paste("2500");
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let Outcome::Applied(saved, restart) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message());
    };
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        (saved.refresh_ms, restart),
        (2500, vec!["Refresh interval"])
    );
    assert!(
        fs::read_to_string(&config)
            .unwrap()
            .contains("refresh_ms = 2500")
    );
    assert_eq!(state(&root), (false, 0));
    assert_eq!(settings.value(RECORDING), Some("true"));
    let shown = screen(&mut settings, 80, 24);
    let text = visible(&shown);
    for part in [
        "Config saved",
        "Telemetry recording not saved: storage_unavailable",
        "storage_unavailable: SQLite operation failed",
        "Restart saddle to apply: Refresh interval",
        // The switch's explanation keeps its place.
        "only work explicitly chosen for recording",
        "history is kept",
    ] {
        assert!(text.contains(part), "{part:?} not visible:\n{shown}");
    }
    // The switch row still shows the unsaved draft, marked as such, ready to retry.
    assert!(shown.contains("•Telemetry recording [Enabled"), "{shown}");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Recorded));
    assert_eq!(state(&root), (true, 1));
}

#[test]
fn a_failed_switch_save_does_not_claim_the_last_read_state_as_current() {
    let dir = tempfile::tempdir().unwrap();
    let (config, root) = (dir.path().join("config.toml"), dir.path().join("telemetry"));
    external(&root, false);
    let lock = rusqlite::Connection::open(root.join("telemetry.sqlite3")).unwrap();
    let mut settings = open(&config, &root);
    toggle(&mut settings);
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Down);
    ctrl(&mut settings, 'u');
    settings.paste("2500");
    // Another client turns recording on, then this Save cannot get the write lock.
    external(&root, true);
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let Outcome::Applied(saved, _) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message());
    };
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(saved.refresh_ms, 2500);
    assert_eq!(state(&root), (true, 1));
    assert_eq!(settings.value(RECORDING), Some("true"));
    let shown = screen(&mut settings, 80, 24);
    let text = visible(&shown);
    assert!(text.contains("Config saved"), "{shown}");
    assert!(text.contains("Telemetry recording not saved"), "{shown}");
    // Settings has no current reading after the failure; it must not call the old one current.
    for claim in ["still Disabled", "still Enabled"] {
        assert!(!text.contains(claim), "{claim:?} shown:\n{shown}");
    }
}
