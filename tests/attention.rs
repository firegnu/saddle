use saddle::{
    agents::Panel,
    attention::{Board, Kind, Target},
    corral::Agent,
};
fn agent(name: &str, state: &str) -> Agent {
    Agent {
        name: name.into(),
        instance: Some("1".into()),
        state: Some(state.into()),
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
}

#[test]
fn failed_sources_are_reported_not_counted_as_nothing() {
    let mut board = Board::default();
    assert!(board.loading());
    board.agents_loaded = true;
    board.corral_error = Some("ls: timed out".into());
    assert_eq!(
        kinds(&board, &Panel::default()),
        [(Kind::ReadFailed, "corral".into())]
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
