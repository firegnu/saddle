use saddle::{
    layout_state::{Content, Layout, Store},
    terminals::{Place, Terminals},
};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn missing_state_creates_directories_and_round_trips_nested_layout_without_starting_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/state/saddle/layout.json");
    let (mut store, saved) = Store::open(Ok(path.clone()));
    assert!(saved.is_none());
    assert!(store.notice.is_empty());
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let first = terminals.active_pane().id;
    terminals.get_mut(first).unwrap().viewer.remembered = Content::Agent {
        name: "p/gone".into(),
        cwd: Some(dir.path().display().to_string()),
        instance: Some("original".into()),
    };
    let right = terminals.reserve(Place::Right, None);
    terminals.complete(right, None).unwrap();
    terminals.get_mut(right.pane).unwrap().viewer.remembered = Content::Shell {
        cwd: dir.path().display().to_string(),
    };
    let down = terminals.reserve(Place::Down, None);
    terminals.complete(down, None).unwrap();
    terminals.new_tab();
    terminals.focus(right.pane);
    store.save(&terminals, false);
    assert!(store.notice.is_empty(), "{}", store.notice);
    let (store, loaded) = Store::open(Ok(path.clone()));
    assert!(store.notice.is_empty());
    let restored = Terminals::restore("unused-fake-corral".into(), loaded.unwrap()).unwrap();
    let area = ratatui::layout::Rect::new(0, 0, 120, 43);
    assert_eq!(restored.rects(area), terminals.rects(area));
    assert_eq!(restored.active_pane().id, right.pane);
    assert_eq!(restored.tabs.len(), 2);
    assert_eq!(
        restored.get(first).unwrap().viewer.remembered.name(),
        Some("p/gone")
    );
    assert!(
        restored
            .tabs
            .iter()
            .flat_map(|t| &t.panes)
            .all(|p| p.viewer.closed())
    );
    assert_eq!(
        serde_json::to_value(restored.snapshot()).unwrap(),
        serde_json::to_value(terminals.snapshot()).unwrap()
    );
    assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o077, 0);
}

#[test]
fn corrupt_unsupported_and_inconsistent_files_are_preserved_through_changes_and_exit_save() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("layout.json");
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let valid = serde_json::to_value(terminals.snapshot()).unwrap();
    let mut invalid = vec![b"{broken".to_vec()];
    for (pointer, value) in [
        ("/version", serde_json::json!(99)),
        ("/active", serde_json::json!(99)),
        ("/tabs/0/active", serde_json::json!(99)),
        ("/tabs/0/tree/value", serde_json::json!(99)),
        ("/tabs/0/panes", serde_json::json!([])),
        (
            "/tabs/0/panes/0/content",
            serde_json::json!({"kind":"shell","cwd":"relative"}),
        ),
    ] {
        let mut broken = valid.clone();
        *broken.pointer_mut(pointer).unwrap() = value;
        invalid.push(serde_json::to_vec(&broken).unwrap());
    }
    for original in invalid {
        fs::write(&path, &original).unwrap();
        let (mut store, saved) = Store::open(Ok(path.clone()));
        assert!(saved.is_none());
        assert!(store.notice.contains("Cannot restore layout"));
        terminals.new_tab();
        store.save(&terminals, false);
        store.save(&terminals, true);
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}

#[test]
fn read_or_write_permission_failure_preserves_file_and_does_not_block_layout_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let original = serde_json::to_vec(&terminals.snapshot()).unwrap();
    fs::write(&path, &original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    let (mut unreadable, loaded) = Store::open(Ok(path.clone()));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(loaded.is_none());
    unreadable.save(&terminals, true);
    assert_eq!(fs::read(&path).unwrap(), original);

    let (mut store, _) = Store::open(Ok(path.clone()));
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
    terminals.new_tab();
    store.save(&terminals, false);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        store.notice.contains("Layout save failed"),
        "{}",
        store.notice
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    store.save(&terminals, true);
    assert!(store.notice.is_empty());
    let (_, loaded) = Store::open(Ok(path));
    assert_eq!(loaded.unwrap().tabs.len(), 2);
}

#[test]
fn invalid_split_ratios_and_duplicate_leaves_are_rejected() {
    let mut terminals = Terminals::new("unused-fake-corral".into());
    terminals.reserve(Place::Right, None);
    let valid = serde_json::to_value(terminals.snapshot()).unwrap();
    for (pointer, value) in [
        ("/tabs/0/tree/value/ratio", 0),
        ("/tabs/0/tree/value/ratio", 100),
        ("/tabs/0/tree/value/second/value", 1),
        ("/tabs/0/panes/1/id", 1),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = serde_json::json!(value);
        let layout: Layout = serde_json::from_value(invalid).unwrap();
        assert!(layout.validate().is_err());
    }
}

#[test]
fn state_path_probe() {
    if let Some(expected) = std::env::var_os("SADDLE_TEST_STATE_PATH") {
        assert_eq!(
            saddle::layout_state::default_path().unwrap(),
            std::path::PathBuf::from(expected)
        );
    }
}

#[test]
fn default_state_path_accepts_only_absolute_xdg_override() {
    let dir = tempfile::tempdir().unwrap();
    for value in [
        None,
        Some(""),
        Some("relative"),
        Some(dir.path().to_str().unwrap()),
    ] {
        let expected = if value.is_some_and(|s| std::path::Path::new(s).is_absolute()) {
            dir.path().join("saddle/layout.json")
        } else {
            dir.path().join(".local/state/saddle/layout.json")
        };
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "state_path_probe"])
            .env("HOME", dir.path())
            .env("SADDLE_TEST_STATE_PATH", expected)
            .env_remove("XDG_STATE_HOME");
        if let Some(value) = value {
            child.env("XDG_STATE_HOME", value);
        }
        let output = child.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn restored_ratio_controls_geometry_and_new_ids_do_not_reuse_sparse_saved_ids() {
    use serde_json::json;
    let layout: Layout = serde_json::from_value(json!({"version":1,"active":100,"tabs":[
        {"id":100,"active":80,"tree":{"kind":"split","value":{"vertical":false,"ratio":30,"first":{"kind":"leaf","value":20},"second":{"kind":"leaf","value":80}}},"panes":[
            {"id":20,"content":{"kind":"empty"}},{"id":80,"content":{"kind":"empty"}}]}
    ]})).unwrap();
    let mut restored = Terminals::restore("unused-fake-corral".into(), layout).unwrap();
    assert_eq!(
        restored.rects(ratatui::layout::Rect::new(0, 0, 100, 23)),
        [
            (20, ratatui::layout::Rect::new(0, 3, 30, 20)),
            (80, ratatui::layout::Rect::new(30, 3, 70, 20)),
        ]
    );
    assert!(restored.new_tab() > 100);
}

#[test]
fn retired_plugin_slots_restore_empty_without_losing_agents_or_split_positions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    let mut terminals = Terminals::new("unused-corral".into());
    let agent = terminals.active_pane().id;
    terminals.get_mut(agent).unwrap().viewer.remembered = Content::Agent {
        name: "p/main".into(),
        cwd: Some("/tmp".into()),
        instance: Some("original".into()),
    };
    let slot = terminals.reserve(Place::Right, None);
    terminals.complete(slot, None).unwrap();
    let mut old = serde_json::to_value(terminals.snapshot()).unwrap();
    old["version"] = serde_json::json!(2);
    old["tabs"][0]["panes"][1]["content"] = serde_json::json!({"kind":"plugin","id":"drover"});
    let bytes = serde_json::to_vec(&old).unwrap();
    fs::write(&path, &bytes).unwrap();
    let (mut store, loaded) = Store::open(Ok(path.clone()));
    assert!(store.notice.is_empty(), "{}", store.notice);
    assert_eq!(
        fs::read(&path).unwrap(),
        bytes,
        "loading does not rewrite state"
    );
    let restored = Terminals::restore("unused-corral".into(), loaded.unwrap()).unwrap();
    assert_eq!(restored.active_pane().id, slot.pane);
    assert_eq!(
        restored.get(slot.pane).unwrap().viewer.remembered,
        Content::Empty
    );
    assert_eq!(
        restored.get(agent).unwrap().viewer.remembered,
        terminals.get(agent).unwrap().viewer.remembered
    );
    let area = ratatui::layout::Rect::new(0, 0, 120, 40);
    assert_eq!(restored.rects(area), terminals.rects(area));
    store.save(&restored, true);
    assert!(store.notice.is_empty(), "{}", store.notice);
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["version"], 2);
    assert_eq!(saved["tabs"][0]["panes"][1]["content"]["kind"], "empty");
}
