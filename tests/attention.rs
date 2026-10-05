use saddle::{
    agents::Panel,
    attention::{Board, Kind},
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
