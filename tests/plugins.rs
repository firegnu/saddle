use saddle::plugins::{registry::Manifest, runtime::Runtime};
use std::time::Duration;
#[test]
fn process_handshake_then_disable_reaps_the_child() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plugin");
    std::fs::write(&path,r#"#!/usr/bin/env python3
import sys,json
for line in sys.stdin:
 m=json.loads(line)
 if m['method']=='initialize':
  print(json.dumps({'kind':'response','id':m['id'],'result':{'id':'test.plugin','version':'0.1.0','protocol_major':1,'capabilities':['panel.v1','notify.v1'],'width_profile':'saddle-grapheme-v1'}}),flush=True)
 elif m['method']=='shutdown': break
"#).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        view: None,
        action: None,
        entry: None,
        id: "test.plugin".into(),
        name: "Test".into(),
        version: "0.1.0".into(),
        protocol_major: 1,
        executable: "plugin".into(),
        args: vec![],
        required_capabilities: vec!["panel.v1".into()],
    };
    std::fs::write(
        dir.path().join("plugin.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    let runtime = Runtime::start(dir.path(), manifest);
    assert!(
        runtime.wait_for("Running", Duration::from_secs(3)),
        "state: {}",
        runtime.snapshot().note
    );
    runtime.stop();
    assert!(runtime.wait_for("Disabled", Duration::from_secs(3)));
}

#[test]
fn plugin_layout_is_a_distinct_content_kind_without_a_pty() {
    let saved = serde_json::json!({"version":2,"active":1,"tabs":[{"id":1,"active":1,"tree":{"kind":"leaf","value":1},"panes":[{"id":1,"content":{"kind":"plugin","id":"demo.counter"}}]}]});
    let layout: saddle::layout_state::Layout =
        serde_json::from_value(saved).expect("plugin layout should decode");
    let terminals = saddle::terminals::Terminals::restore("unused-corral".into(), layout).unwrap();
    assert!(terminals.active_pane().input_session().is_none());
    assert_eq!(
        serde_json::to_value(terminals.snapshot()).unwrap()["tabs"][0]["panes"][0]["content"]["kind"],
        "plugin"
    );
}

#[test]
fn registration_defaults_disabled_and_concurrent_edit_is_not_overwritten() {
    use saddle::plugins::registry::Registry;
    let dir = tempfile::tempdir().unwrap();
    let plugin = dir.path().join("plugin");
    std::fs::create_dir(&plugin).unwrap();
    std::os::unix::fs::symlink("/bin/echo", plugin.join("entry")).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        view: None,
        action: None,
        entry: None,
        id: "demo.other".into(),
        name: "Other".into(),
        version: "1".into(),
        protocol_major: 1,
        executable: "entry".into(),
        args: vec![],
        required_capabilities: vec![],
    };
    std::fs::write(
        plugin.join("plugin.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    let path = dir.path().join("plugins.toml");
    let mut first = Registry::open(path.clone());
    let mut stale = Registry::open(path.clone());
    first.add(&plugin, &manifest).unwrap();
    assert!(!first.entries[0].enabled);
    assert!(stale.add(&plugin, &manifest).is_err());
    stale.refresh().unwrap();
    stale.enabled("demo.other", true).unwrap();
    assert!(first.remove("demo.other").is_err());
    assert!(Registry::open(path).entries[0].enabled);
    assert!(plugin.join("entry").exists());
}
#[test]
fn unknown_registry_version_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plugins.toml");
    let bytes = "version = 900\nplugins = []\n";
    std::fs::write(&path, bytes).unwrap();
    let r = saddle::plugins::registry::Registry::open(path.clone());
    assert!(r.error.is_some());
    assert_eq!(std::fs::read_to_string(path).unwrap(), bytes);
}
#[test]
fn closing_last_plugin_does_not_downgrade_the_layout_format() {
    let mut terminals = saddle::terminals::Terminals::new("unused".into());
    let pane = terminals.open_plugin("demo.counter");
    assert_eq!(terminals.snapshot().version, 2);
    terminals.close_pane(pane).unwrap();
    assert_eq!(terminals.snapshot().version, 2);
}

fn peer(mode: &str) -> (tempfile::TempDir, Manifest) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("peer");
    std::fs::write(&path, include_str!("fixtures/plugin.py")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let m = Manifest {
        manifest_version: 1,
        view: None,
        action: None,
        entry: None,
        id: "test.peer".into(),
        name: "Peer".into(),
        version: "1".into(),
        protocol_major: 1,
        executable: "peer".into(),
        args: vec![mode.into()],
        required_capabilities: vec!["panel.v1".into()],
    };
    std::fs::write(dir.path().join("plugin.toml"), toml::to_string(&m).unwrap()).unwrap();
    (dir, m)
}
#[test]
fn invalid_handshake_and_unterminated_oversized_line_fail_and_reap() {
    for mode in ["wrong", "partial"] {
        let (dir, m) = peer(mode);
        let r = Runtime::start(dir.path(), m);
        assert!(
            r.wait_for("Failed", Duration::from_secs(4)),
            "{}",
            r.snapshot().note
        );
        assert!(r.snapshot().pid.is_none());
    }
}
#[test]
fn restart_waits_for_a_stubborn_child_to_be_reaped() {
    let (dir, m) = peer("stubborn");
    let mut manager = saddle::plugins::Manager::open(dir.path().join("plugins.toml"));
    manager.add(dir.path(), &m).unwrap();
    manager.enable(&m.id).unwrap();
    fn wait(manager: &mut saddle::plugins::Manager, state: &str) {
        let start = std::time::Instant::now();
        while manager.state("test.peer") != state {
            manager.tick();
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    wait(&mut manager, "Running");
    let old = manager.snapshot(&m.id).unwrap().pid.unwrap();
    manager.restart(&m.id).unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        manager.tick();
        let s = manager.snapshot(&m.id).unwrap();
        if s.state == "Running" && s.pid != Some(old) {
            break;
        }
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        unsafe { libc::kill(old as i32, 0) },
        -1,
        "old child must be reaped before replacement runs"
    );
    manager.disable(&m.id).unwrap();
    wait(&mut manager, "Disabled");
    assert!(manager.snapshot(&m.id).unwrap().pid.is_none());
}
#[test]
fn layout_upgrade_keeps_the_original_v1_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    let mut t = saddle::terminals::Terminals::new("unused".into());
    let bytes = serde_json::to_vec(&t.snapshot()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let (mut store, _) = saddle::layout_state::Store::open(Ok(path.clone()));
    t.open_plugin("demo.counter");
    store.save(&t, true);
    assert!(store.notice.is_empty(), "{}", store.notice);
    assert_eq!(
        std::fs::read(path.with_extension("v1.json")).unwrap(),
        bytes
    );
    let (_, layout) = saddle::layout_state::Store::open(Ok(path));
    assert_eq!(layout.unwrap().version, 2);
}
#[test]
#[ignore = "requires separately built SDK stdio probe in SADDLE_TEST_PROBE"]
fn sdk_business_stdout_and_child_stdin_cannot_corrupt_protocol() {
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        std::env::var("SADDLE_TEST_PROBE").unwrap(),
        dir.path().join("probe"),
    )
    .unwrap();
    let m = Manifest {
        manifest_version: 1,
        view: None,
        action: None,
        entry: None,
        id: "test.stdio".into(),
        name: "Probe".into(),
        version: "1".into(),
        protocol_major: 1,
        executable: "probe".into(),
        args: vec![],
        required_capabilities: vec!["panel.v1".into()],
    };
    std::fs::write(dir.path().join("plugin.toml"), toml::to_string(&m).unwrap()).unwrap();
    let runtime = Runtime::start(dir.path(), m);
    assert!(
        runtime.wait_for("Running", Duration::from_secs(3)),
        "{}",
        runtime.snapshot().note
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !runtime.snapshot().log.contains("child stdout became a log") {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        runtime
            .snapshot()
            .log
            .contains("business stdout became a log")
    );
    runtime.stop();
    assert!(runtime.wait_for("Disabled", Duration::from_secs(3)));
}
#[test]
fn plugin_management_and_add_dialog_fit_tiny_terminals() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    let dir = tempfile::tempdir().unwrap();
    let mut manager = saddle::plugins::Manager::open(dir.path().join("plugins.toml"));
    let mut page = saddle::plugins::ui::Page::default();
    for adding in [false, true] {
        if adding {
            for _ in 0..4 {
                page.event(
                    Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
                    &mut manager,
                );
            }
            page.event(
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                &mut manager,
            );
        }
        for (cols, rows) in [(1, 1), (10, 3), (20, 5), (40, 12), (100, 40)] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(cols, rows)).unwrap();
            terminal
                .draw(|frame| page.draw(&saddle::theme::Theme::default(), frame, &manager))
                .unwrap();
        }
    }
}

#[test]
fn entry_manifest_requires_explicit_capabilities_and_valid_references() {
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/bin/echo", dir.path().join("entry")).unwrap();
    let base = r#"
manifest_version = 1
id = "test.entry"
name = "Entry fixture"
version = "1"
protocol_major = 1
executable = "entry"
required_capabilities = ["panel.v1", "ui.entry.v1", "panel.overlay.v1"]
entry = "open"
[view]
id = "main"
placement = "overlay"
[action]
id = "open"
title = "Counter"
view = "main"
"#;
    std::fs::write(dir.path().join("plugin.toml"), base).unwrap();
    assert!(Manifest::read(dir.path()).is_ok());
    for bad in [
        base.replace("\"ui.entry.v1\", ", ""),
        base.replace(", \"panel.overlay.v1\"", ""),
        base.replace("entry = \"open\"", "entry = \"missing\""),
        base.replace("view = \"main\"", "view = \"other\""),
        base.replace("placement = \"overlay\"", "placement = \"unknown\""),
        base.replace("title = \"Counter\"", "title = \"\""),
    ] {
        std::fs::write(dir.path().join("plugin.toml"), bad).unwrap();
        assert!(Manifest::read(dir.path()).is_err());
    }
}
#[test]
fn entry_bar_scrolls_and_never_activates_a_replaced_or_disabled_entry() {
    use crossterm::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use saddle::plugins::entry::{Bar, Entry};
    use saddle_plugin_protocol::Placement;
    let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    let mouse = |kind| {
        Event::Mouse(MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })
    };
    let mut bar = Bar::default();
    let mut entries: Vec<_> = (0..8)
        .map(|i| Entry {
            plugin: format!("p{i}"),
            action: "open".into(),
            title: format!("View {i}"),
            placement: Placement::Overlay,
            state: "Running".into(),
        })
        .collect();
    bar.update(entries.clone());
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(20, 3)).unwrap();
    let draw = |bar: &mut Bar, t: &mut ratatui::Terminal<_>| {
        t.draw(|f| bar.draw(f, f.area(), &Default::default()))
            .unwrap();
    };
    draw(&mut bar, &mut terminal);
    bar.event(&mouse(MouseEventKind::Down(MouseButton::Left)), true);
    entries[0].plugin = "replacement".into();
    bar.update(entries.clone());
    draw(&mut bar, &mut terminal);
    assert!(
        bar.event(&mouse(MouseEventKind::Up(MouseButton::Left)), true)
            .1
            .is_none()
    );
    bar.event(&key(KeyCode::F(6)), true);
    for _ in 0..7 {
        bar.event(&key(KeyCode::Down), true);
    }
    draw(&mut bar, &mut terminal);
    assert_eq!(
        bar.event(&key(KeyCode::Enter), true).1.unwrap().plugin,
        "p7"
    );
    entries[7].state = "Failed".into();
    bar.update(entries);
    assert!(bar.event(&key(KeyCode::Enter), true).1.is_none());
    assert!(!bar.event(&key(KeyCode::F(6)), false).0);
}

#[test]
fn overlay_resize_rejects_the_previously_displayed_frame() {
    use ratatui::layout::Rect;
    use saddle::plugins::{Manager, registry::Registry};
    let dir = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(
        dir.path().join("counter"),
        include_str!("fixtures/plugin_counter.py"),
    )
    .unwrap();
    std::fs::set_permissions(
        dir.path().join("counter"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let manifest:Manifest=serde_json::from_value(serde_json::json!({"manifest_version":1,"id":"test.entry","name":"Fixture","version":"1","protocol_major":1,"executable":"counter","required_capabilities":["panel.v1"]})).unwrap();
    std::fs::write(
        dir.path().join("plugin.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    let path = dir.path().join("plugins.toml");
    let mut registry = Registry::open(path.clone());
    registry.add(dir.path(), &manifest).unwrap();
    registry.enabled("test.entry", true).unwrap();
    let mut manager = Manager::open(path);
    let mut terminals = saddle::terminals::Terminals::new("unused".into());
    let area = Rect::new(0, 0, 80, 24);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let old = loop {
        let p = manager
            .sync_with_overlay(&mut terminals, area, false, Some(("test.entry", area)))
            .unwrap();
        if p.interactive {
            break p;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    };
    manager.sync_with_overlay(
        &mut terminals,
        area,
        false,
        Some(("test.entry", Rect::new(0, 0, 40, 12))),
    );
    assert!(!manager.input(
        &old,
        serde_json::json!({"type":"key","code":{"name":"enter"},"phase":"press","modifiers":[]})
    ));
    manager.disable("test.entry").unwrap();
}
