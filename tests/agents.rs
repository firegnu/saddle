use saddle::{agents::Panel, corral::Agent};
fn agent(name: &str, state: &str) -> Agent {
    Agent {
        name: name.into(),
        instance: Some("123".into()),
        state: Some(state.into()),
        ..Default::default()
    }
}
#[test]
fn refresh_keeps_selection_and_marks_unseen_completed_turns() {
    let mut panel = Panel::default();
    panel.absorb(
        vec![agent("p/a", "working"), agent("p/b", "idle")],
        None,
        100.0,
    );
    panel.selected = Some("p/b".into());
    panel.absorb(
        vec![
            agent("new/a", "idle"),
            agent("p/a", "idle"),
            agent("p/b", "idle"),
        ],
        None,
        101.0,
    );
    assert_eq!(panel.selected.as_deref(), Some("p/b"));
    assert!(panel.unread.contains("p/a"));
    panel.absorb(vec![agent("p/a", "idle")], Some("p/a"), 102.0);
    assert_eq!(panel.selected.as_deref(), Some("p/a"));
    assert!(panel.unread.is_empty());
    assert!(panel.message.contains("p/b exited"));
}

#[test]
fn state_sort_stays_within_projects_and_selection_follows_display_order() {
    let mut panel = Panel {
        by_state: true,
        ..Default::default()
    };
    panel.absorb(
        vec![
            agent("a/idle", "idle"),
            agent("b/blocked", "blocked"),
            agent("a/blocked", "blocked"),
            agent("a/working", "working"),
        ],
        None,
        100.0,
    );
    let names: Vec<_> = panel
        .ordered(100.0)
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, ["a/blocked", "a/working", "a/idle", "b/blocked"]);
    panel.move_selection(1, 100.0);
    assert_eq!(panel.selected.as_deref(), Some("a/working"));
    panel.absorb(
        vec![Agent {
            instance: Some("456".into()),
            ..agent("a/working", "idle")
        }],
        None,
        101.0,
    );
    assert!(panel.unread.is_empty());
}
