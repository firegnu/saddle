use saddle::{
    agents::Panel,
    attention::{Board, Kind, Target},
    corral::Agent,
    drover::{Snapshot, Survey, Task},
};
fn agent(name: &str, state: &str) -> Agent {
    Agent {
        name: name.into(),
        instance: Some("1".into()),
        state: Some(state.into()),
        ..Default::default()
    }
}
fn task(id: &str, status: Option<&str>) -> Task {
    Task {
        id: Some(id.into()),
        title: format!("title {id}"),
        status: status.map(Into::into),
        ..Default::default()
    }
}
fn kinds(board: &Board, panel: &Panel) -> Vec<(Kind, String)> {
    board
        .items(panel, 100.0)
        .into_iter()
        .map(|i| (i.kind, i.label))
        .collect()
}

#[test]
fn one_row_per_agent_with_its_need_before_its_reply() {
    let mut panel = Panel::default();
    panel.absorb(
        vec![
            agent("p/ask", "working"),
            agent("p/done", "working"),
            agent("p/busy", "working"),
            agent("p/sel", "idle"),
        ],
        None,
        100.0,
    );
    panel.select(Some("p/sel".into()));
    panel.absorb(
        vec![
            agent("p/ask", "blocked"),
            agent("p/done", "idle"),
            agent("p/busy", "working"),
            Agent {
                error: Some("status failed".into()),
                ..agent("p/sel", "idle")
            },
        ],
        None,
        101.0,
    );
    // A waiting agent with an unread reply still takes one row, under its need.
    panel.unread.insert("p/ask".into());
    let items = Board::default().items(&panel, 101.0);
    let rows: Vec<_> = items.iter().map(|i| (i.kind, i.label.as_str())).collect();
    assert_eq!(
        rows,
        [
            (Kind::Waiting, "p/ask"),
            (Kind::Error, "p/sel"),
            (Kind::Reply, "p/done")
        ]
    );
    assert_eq!(items[0].note, "new reply");
    assert_eq!(items[1].note, "status failed");
    assert!(items.iter().all(|i| !i.seeable() || i.kind == Kind::Failed));
}

#[test]
fn tasks_come_from_every_project_and_seen_is_per_project_task() {
    let mut board = Board {
        agents_loaded: true,
        ..Default::default()
    };
    board.absorb(Survey::Projects(Ok(vec![
        "/work/one".into(),
        "/work/two".into(),
        "/work/three".into(),
    ])));
    assert!(board.loading());
    board.absorb(Survey::Snapshot(
        "/work/one".into(),
        Ok(Box::new(Snapshot {
            awaiting: Some(task("T3", Some("done"))),
            history: vec![task("T1", Some("failed")), task("T2", Some("done"))],
            ..Default::default()
        })),
    ));
    board.absorb(Survey::Snapshot(
        "/work/two".into(),
        Ok(Box::new(Snapshot {
            history: vec![task("T1", Some("failed")), task("T4", Some("dropped"))],
            ..Default::default()
        })),
    ));
    board.absorb(Survey::Snapshot(
        "/work/three".into(),
        Err("drover list: boom".into()),
    ));
    // A result for a project no longer registered is ignored.
    board.absorb(Survey::Snapshot(
        "/work/gone".into(),
        Ok(Box::new(Snapshot::default())),
    ));
    assert!(!board.loading());
    let panel = Panel::default();
    assert_eq!(
        kinds(&board, &panel),
        [
            (Kind::Awaiting, "one · T3".into()),
            (Kind::Failed, "one · T1".into()),
            (Kind::Failed, "two · T1".into()),
            (Kind::ReadFailed, "three".into()),
        ]
    );
    let items = board.items(&panel, 100.0);
    assert!(items[1].seeable() && !items[0].seeable() && !items[3].seeable());
    board.seen.insert(items[1].target.clone());
    assert_eq!(
        kinds(&board, &panel),
        [
            (Kind::Awaiting, "one · T3".into()),
            (Kind::Failed, "two · T1".into()),
            (Kind::ReadFailed, "three".into()),
        ]
    );
    // A new task reusing the number is a different task.
    board.absorb(Survey::Snapshot(
        "/work/one".into(),
        Ok(Box::new(Snapshot {
            history: vec![Task {
                title: "another".into(),
                ..task("T1", Some("failed"))
            }],
            ..Default::default()
        })),
    ));
    assert_eq!(kinds(&board, &panel)[0], (Kind::Failed, "one · T1".into()));
    assert!(matches!(
        &board.items(&panel, 100.0)[0].target,
        Target::Task { project, .. } if project == "/work/one"
    ));
}

#[test]
fn failed_sources_are_reported_not_counted_as_nothing() {
    let mut board = Board::default();
    assert!(board.loading());
    board.agents_loaded = true;
    board.corral_error = Some("ls: timed out".into());
    board.absorb(Survey::Projects(Ok(vec!["/work/one".into()])));
    board.absorb(Survey::Snapshot(
        "/work/one".into(),
        Ok(Box::new(Snapshot {
            awaiting: Some(task("T3", None)),
            ..Default::default()
        })),
    ));
    board.absorb(Survey::Projects(Err("Read ~/.drover/projects".into())));
    // Without the registry the old project result is not kept as if current.
    assert_eq!(
        kinds(&board, &Panel::default()),
        [
            (Kind::ReadFailed, "corral".into()),
            (Kind::ReadFailed, "~/.drover/projects".into()),
        ]
    );
    assert!(!board.loading());
}

#[test]
fn plugin_attention_enter_does_not_follow_a_replaced_row() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use saddle::attention::{Item, Outcome, Popup};
    let item = |revision| Item {
        target: Target::Plugin {
            plugin: "test.one".into(),
            session: 1,
            revision,
            item: "one".into(),
        },
        kind: Kind::Plugin,
        label: "Example".into(),
        note: String::new(),
    };
    let mut popup = Popup::default();
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert!(matches!(popup.key(key, &[item(1)]), Outcome::Open(_)));
    assert!(matches!(popup.key(key, &[item(2)]), Outcome::Stay));
    let unavailable = Item {
        target: Target::Source("plugin:test.one".into()),
        kind: Kind::Unavailable,
        label: "Unavailable".into(),
        note: String::new(),
    };
    assert!(matches!(
        Popup::default().key(key, &[unavailable]),
        Outcome::Stay
    ));
    assert!(!item(1).seeable());
}

#[test]
fn plugin_attention_selection_survives_snapshot_reordering() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use saddle::attention::{Item, Outcome, Popup};
    let item = |id: &str, revision| Item {
        target: Target::Plugin {
            plugin: "test.one".into(),
            session: 1,
            revision,
            item: id.into(),
        },
        kind: Kind::Plugin,
        label: id.into(),
        note: String::new(),
    };
    let mut popup = Popup::default();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    let theme = saddle::theme::Theme::default();
    terminal
        .draw(|f| {
            popup.draw(&theme, f, &[item("one", 1), item("two", 1)], false);
        })
        .unwrap();
    popup.key(
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        &[item("one", 1), item("two", 1)],
    );
    terminal
        .draw(|f| {
            popup.draw(&theme, f, &[item("two", 2), item("one", 2)], false);
        })
        .unwrap();
    assert!(
        matches!(popup.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &[item("two",2),item("one",2)]), Outcome::Open(Target::Plugin {item,revision:2,..}) if item == "two")
    );
}
