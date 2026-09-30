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
fn palette_filters_preserves_selection_and_gates_every_runtime_state() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use saddle::plugins::palette::{Item, Outcome, Palette};
    let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    let mut item = Item {
        id: "a".into(),
        title: "Counter".into(),
        state: "Running".into(),
        note: String::new(),
        has_view: true,
        opened: false,
        pid: Some(7),
    };
    let mut p = Palette::default();
    let draw = |p: &mut Palette| {
        let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        t.draw(|f| p.draw(f, &Default::default())).unwrap();
    };
    p.update(vec![item.clone()]);
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Open(item.clone()));
    let mut starting = item.clone();
    starting.state = "Starting".into();
    p.update(vec![starting]);
    draw(&mut p);
    p.update(vec![item.clone()]);
    assert_eq!(
        p.event(&key(KeyCode::Enter)),
        Outcome::Stay,
        "cannot execute a state not yet displayed"
    );
    draw(&mut p);
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Open(item.clone()));
    for state in [
        "Starting",
        "Stopping",
        "Restarting",
        "Disabled",
        "Failed",
        "Unresponsive",
        "Unavailable",
    ] {
        let mut blocked = item.clone();
        blocked.state = state.into();
        assert!(blocked.action().is_none());
        assert!(!blocked.explanation().is_empty());
        p.update(vec![blocked]);
        draw(&mut p);
        assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Stay);
    }
    let mut background = item.clone();
    background.has_view = false;
    p.update(vec![background]);
    draw(&mut p);
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Stay);
    let mut other = item.clone();
    other.id = "b".into();
    other.title = "Tasks".into();
    other.opened = true;
    assert_eq!(other.action(), Some("Switch"));
    p.update(vec![item.clone(), other.clone()]);
    p.event(&key(KeyCode::Down));
    draw(&mut p);
    item.state = "Failed".into();
    p.update(vec![item, other.clone()]);
    assert_eq!(p.selected_id(), Some("b"));
    draw(&mut p);
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Open(other.clone()));
    p.event(&Event::Paste("no match".into()));
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Stay);
    p.event(&Event::Key(KeyEvent::new(
        KeyCode::Char('u'),
        KeyModifiers::CONTROL,
    )));
    p.event(&Event::Paste("TASK".into()));
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Open(other.clone()));
    p.event(&Event::Key(KeyEvent::new(
        KeyCode::Char('u'),
        KeyModifiers::CONTROL,
    )));
    p.event(&Event::Paste(format!("{}x", " ".repeat(4095))));
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Stay);
    p.event(&key(KeyCode::Backspace));
    p.event(&key(KeyCode::Char('s')));
    assert_eq!(p.event(&key(KeyCode::Enter)), Outcome::Open(other));
}

#[test]
fn palette_mouse_actions_cancel_when_state_changes_and_tiny_layouts_fit() {
    use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use saddle::plugins::palette::{Item, Outcome, Palette};
    let mut p = Palette::default();
    let mut item = Item {
        id: "a".into(),
        title: "计数器".into(),
        state: "Running".into(),
        note: String::new(),
        has_view: true,
        opened: false,
        pid: Some(7),
    };
    p.update(vec![item.clone()]);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| p.draw(f, &Default::default())).unwrap();
    let buffer = terminal.backend().buffer();
    let (x, y) = (0..24)
        .flat_map(|y| (0..76).map(move |x| (x, y)))
        .find(|&(x, y)| {
            (0..4)
                .map(|d| buffer[(x + d, y)].symbol())
                .collect::<String>()
                == "Open"
        })
        .unwrap();
    let mouse = |kind| {
        Event::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        })
    };
    assert_eq!(
        p.event(&mouse(MouseEventKind::Down(MouseButton::Left))),
        Outcome::Stay
    );
    item.state = "Failed".into();
    p.update(vec![item.clone()]);
    terminal.draw(|f| p.draw(f, &Default::default())).unwrap();
    assert_eq!(
        p.event(&mouse(MouseEventKind::Up(MouseButton::Left))),
        Outcome::Stay
    );
    item.state = "Running".into();
    p.update(vec![item.clone()]);
    terminal.draw(|f| p.draw(f, &Default::default())).unwrap();
    p.event(&mouse(MouseEventKind::Down(MouseButton::Left)));
    assert_eq!(
        p.event(&mouse(MouseEventKind::Up(MouseButton::Left))),
        Outcome::Open(item)
    );
    for (w, h) in [(1, 1), (8, 3), (20, 8), (40, 12), (80, 24)] {
        let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        t.draw(|f| p.draw(f, &Default::default())).unwrap();
    }
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
            .sync_with_overlay(
                &mut terminals,
                area,
                false,
                Some(("test.entry", area, true)),
            )
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
        Some(("test.entry", Rect::new(0, 0, 40, 12), true)),
    );
    assert!(!manager.input(
        &old,
        serde_json::json!({"type":"key","code":{"name":"enter"},"phase":"press","modifiers":[]})
    ));
    manager.disable("test.entry").unwrap();
}

fn attention_peer() -> (tempfile::TempDir, Manifest) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("peer");
    std::fs::write(&path, include_str!("fixtures/attention.py")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let m = Manifest {
        manifest_version: 1,
        id: "test.attention".into(),
        name: "Attention demo".into(),
        version: "1".into(),
        protocol_major: 1,
        executable: "peer".into(),
        args: vec![],
        required_capabilities: vec![
            "panel.v1".into(),
            "ui.entry.v1".into(),
            "attention.v1".into(),
        ],
        view: Some(saddle_plugin_protocol::ViewDeclaration {
            id: "main".into(),
            placement: saddle_plugin_protocol::Placement::Workspace,
        }),
        action: Some(saddle_plugin_protocol::OpenAction {
            id: "open".into(),
            title: "Attention demo".into(),
            view: "main".into(),
        }),
        entry: Some("open".into()),
    };
    std::fs::write(dir.path().join("plugin.toml"), toml::to_string(&m).unwrap()).unwrap();
    (dir, m)
}
fn wait_until(mut check: impl FnMut() -> bool) {
    let start = std::time::Instant::now();
    while !check() {
        assert!(start.elapsed() < Duration::from_secs(4), "timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn attention_capability_accepts_a_snapshot_without_sending_a_notification() {
    let (dir, m) = attention_peer();
    assert!(
        Manifest::read(dir.path()).is_ok(),
        "attention.v1 must be supported"
    );
    let r = Runtime::start(dir.path(), m);
    wait_until(|| dir.path().join("response.json").exists());
    let response: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("response.json")).unwrap()).unwrap();
    assert_eq!(response["result"]["status"], "accepted");
    r.stop();
}

#[test]
fn attention_replacement_is_atomic_bounded_and_rejects_stale_targets() {
    use saddle_plugin_protocol::Message;
    use serde_json::json;
    let (dir, m) = attention_peer();
    let notices = saddle::plugins::runtime::Notices::default();
    let r = Runtime::with_notices(dir.path(), m, notices.clone());
    wait_until(|| r.snapshot().attention.is_some());
    let old = r.snapshot();
    assert!(notices.lock().unwrap().items.is_empty());
    assert!(r.open_attention(old.session, 1, "one"));
    wait_until(|| dir.path().join("opened.json").exists());
    let opened: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("opened.json")).unwrap()).unwrap();
    assert_eq!(opened["target"], json!({"document":"one"}));
    let valid = json!({"items":[{"id":"two","title":"Changed","note":"","action":"open","target":{"document":"two"}}]});
    let mut duplicate = valid.clone();
    duplicate["items"]
        .as_array_mut()
        .unwrap()
        .push(valid["items"][0].clone());
    let mut bad_action = valid.clone();
    bad_action["items"][0]["action"] = json!("missing");
    let mut huge = valid.clone();
    huge["items"][0]["target"] = json!("x".repeat(4097));
    let mut control = valid.clone();
    control["items"][0]["note"] = json!("oops\n");
    let mut id = 2;
    for bad in [duplicate, bad_action, huge, control] {
        r.send(Message::event(
            "test.replace",
            json!({"id":id,"snapshot":bad}),
        ));
        wait_until(|| {
            std::fs::read(dir.path().join("response.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .is_some_and(|v| v["id"] == id && v["error"]["code"] == "invalid_attention")
        });
        assert_eq!(
            r.snapshot().attention.unwrap().0,
            1,
            "invalid update must preserve previous snapshot"
        );
        id += 1;
    }
    r.send(Message::event(
        "test.replace",
        json!({"id":id,"snapshot":valid}),
    ));
    wait_until(|| r.snapshot().attention.is_some_and(|(r, _)| r == 2));
    assert!(!r.open_attention(old.session, 1, "one"));
    assert!(!r.open_attention(old.session + 1000, 2, "two"));
    assert!(r.open_attention(old.session, 2, "two"));
    r.send(Message::event(
        "test.replace",
        json!({"id":id+1,"snapshot":{"items":[]}}),
    ));
    wait_until(|| {
        r.snapshot()
            .attention
            .is_some_and(|(r, s)| r == 3 && s.items.is_empty())
    });
    assert!(!r.open_attention(old.session, 2, "two"));
    r.stop();
}

#[test]
fn attention_sources_are_isolated_and_lifecycle_invalidates_open() {
    use saddle::attention::{Kind, Target};
    let (dir, m) = attention_peer();
    let (other, mut n) = attention_peer();
    n.id = "test.other".into();
    std::fs::write(
        other.path().join("peer"),
        include_str!("fixtures/attention.py").replace("test.attention", "test.other"),
    )
    .unwrap();
    std::fs::write(
        other.path().join("plugin.toml"),
        toml::to_string(&n).unwrap(),
    )
    .unwrap();
    let mut manager = saddle::plugins::Manager::open(dir.path().join("plugins.toml"));
    for (path, manifest) in [(dir.path(), &m), (other.path(), &n)] {
        manager.add(path, manifest).unwrap();
        manager.enable(&manifest.id).unwrap();
    }
    wait_until(|| {
        manager
            .attention_items()
            .iter()
            .filter(|i| i.kind == Kind::Plugin)
            .count()
            == 2
    });
    let target = manager
        .attention_items()
        .into_iter()
        .find(|i| matches!(&i.target, Target::Plugin {plugin, ..} if plugin == &m.id))
        .unwrap()
        .target;
    assert!(manager.open_attention(&target));
    manager.restart(&m.id).unwrap();
    assert!(!manager.open_attention(&target));
    assert_eq!(
        manager
            .attention_items()
            .iter()
            .filter(|i| i.kind == Kind::Unavailable)
            .count(),
        1
    );
    wait_until(|| {
        manager.tick();
        manager
            .attention_items()
            .iter()
            .filter(|i| i.kind == Kind::Plugin)
            .count()
            == 2
    });
    assert!(
        !manager.open_attention(&target),
        "old session must not become valid after restart"
    );
    let pid = manager.snapshot(&m.id).unwrap().pid.unwrap();
    // This is our isolated test child, never a real agent.
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGKILL) }, 0);
    wait_until(|| manager.state(&m.id) == "Failed");
    assert!(
        manager
            .attention_items()
            .iter()
            .any(|i| i.kind == Kind::Unavailable && matches!(i.target, Target::Source(_)))
    );
    manager.disable(&m.id).unwrap();
    assert_eq!(manager.attention_items().len(), 1);
    assert!(
        matches!(&manager.attention_items()[0].target, Target::Plugin {plugin, ..} if plugin == &n.id)
    );
    wait_until(|| manager.stopped(&m.id));
    manager.remove(&m.id).unwrap();
    assert_eq!(manager.attention_items().len(), 1);
    manager.disable(&n.id).unwrap();
    assert!(manager.attention_items().is_empty());
    assert!(!manager.open_attention(&target));
}

#[test]
fn attention_requests_require_the_declared_capability() {
    let (dir, mut m) = attention_peer();
    m.required_capabilities.retain(|c| c != "attention.v1");
    std::fs::write(dir.path().join("plugin.toml"), toml::to_string(&m).unwrap()).unwrap();
    let r = Runtime::start(dir.path(), m);
    wait_until(|| dir.path().join("response.json").exists());
    let response: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("response.json")).unwrap()).unwrap();
    assert_eq!(response["error"]["code"], "unsupported");
    assert!(r.snapshot().attention.is_none());
    r.stop();
}
