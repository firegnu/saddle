use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use saddle::{
    drover::{Operation, Request, Snapshot},
    queue::{Page, Panel},
};
fn key(code: K) -> KeyEvent {
    KeyEvent::new(code, M::NONE)
}

#[test]
fn selected_pending_edit_prefills_and_saves_the_native_form() {
    let mut panel = Panel::default();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "awaiting": null, "history": [],
            "current": {"id":"T0", "title":"Running"},
            "pending": [
                {"id":"T1", "title":"First", "body":"first body"},
                {"id":"T2", "title":"Second", "body":"second body"}
            ]
        }))
        .unwrap(),
    );
    panel.select(2);
    panel.key(key(K::Char('e')));
    assert!(
        panel.overlay_open(),
        "Edit must open the selected pending form"
    );
    panel.paste(" revised");
    panel.key(key(K::Tab));
    panel.key(key(K::Enter));
    panel.paste("extra line");
    let Some(Request::Run(op)) = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL)) else {
        panic!("Edit must submit a public operation");
    };
    assert_eq!(
        op.args(),
        ["edit", "2", "Second revised", "second body\nextra line"]
    );
}

#[test]
fn selected_pending_moves_use_pending_positions_and_stop_at_the_ends() {
    let mut panel = Panel::default();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "history": [],
            "current": {"id":"T0", "title":"Running"},
            "awaiting": {"id":"T9", "title":"Waiting"},
            "pending": [
                {"id":"T1", "title":"First"},
                {"id":"T2", "title":"Second"}
            ]
        }))
        .unwrap(),
    );
    panel.select(3);
    assert!(panel.key(key(K::Char('d'))).is_none());
    let Some(Request::Run(op)) = panel.key(key(K::Char('u'))) else {
        panic!("Move up must submit a public operation for the selected pending task");
    };
    assert_eq!(op.args(), ["move", "2", "1"]);
    assert!(panel.key(key(K::Char('u'))).is_none());
    let mut fresh = panel.snapshot.clone().unwrap();
    fresh.pending.swap(0, 1);
    // Even if the user changes selection while the command runs, success follows its target.
    panel.select(0);
    panel.complete(&op, Ok("moved".into()));
    panel.absorb(fresh);
    assert_eq!(panel.selected, 2);
    assert!(panel.key(key(K::Char('u'))).is_none());
    let Some(Request::Run(op)) = panel.key(key(K::Char('d'))) else {
        panic!("Move down must submit a public operation");
    };
    assert_eq!(op.args(), ["move", "1", "2"]);
}

#[test]
fn moving_identical_unnumbered_tasks_keeps_the_destination_selected() {
    let mut panel = Panel::default();
    let task = saddle::drover::Task {
        title: "Unnumbered".into(),
        ..Default::default()
    };
    let snapshot = Snapshot {
        pending: vec![task.clone(), task],
        ..Default::default()
    };
    panel.absorb(snapshot.clone());
    let Some(Request::Run(op)) = panel.key(key(K::Char('d'))) else {
        panic!("missing move")
    };
    panel.complete(&op, Ok("moved".into()));
    panel.absorb(snapshot.clone());
    assert_eq!(panel.selected, 1);
    panel.absorb(snapshot);
    assert_eq!(
        panel.selected, 1,
        "periodic refresh must retain selection too"
    );
}
#[test]
fn native_queue_selects_details_and_creates_tasks_without_an_external_editor() {
    let mut panel = Panel::default();
    panel.absorb(serde_json::from_str::<Snapshot>(r#"{"mode":{"loop":false,"gate":true},"paused":false,"current":null,"awaiting":null,"history":[],"pending":[{"id":"T1","title":"First","body":"first body"},{"id":"T2","title":"Second","body":"second body"}]}"#).unwrap());
    panel.key(key(K::Down));
    assert_eq!(panel.selected, 1);
    panel.key(key(K::Enter));
    assert!(matches!(panel.page, Page::Detail));
    panel.key(key(K::Esc));
    panel.key(key(K::Char('a')));
    for c in "中文新增".chars() {
        panel.key(key(K::Char(c)));
    }
    panel.key(key(K::Tab));
    for c in "正文".chars() {
        panel.key(key(K::Char(c)));
    }
    panel.key(key(K::Enter));
    panel.key(key(K::Char('b')));
    let request = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL));
    assert!(
        matches!(request,Some(Request::Run(Operation::Add{title,body})) if title=="中文新增" && body=="正文\nb")
    );
    assert!(panel.busy);
    assert!(matches!(panel.page, Page::Add { .. }));
    panel.complete(
        &Operation::Add {
            title: "中文新增".into(),
            body: "正文\nb".into(),
        },
        Ok("added".into()),
    );
    assert!(matches!(panel.page, Page::List));
}

#[test]
fn failed_add_retains_the_native_draft_and_prevents_duplicate_submission() {
    let mut panel = Panel::default();
    panel.absorb(Snapshot::default());
    panel.key(key(K::Char('a')));
    panel.paste("Task with 中文");
    panel.key(key(K::Tab));
    panel.paste("first\nsecond");
    let save = KeyEvent::new(K::Char('s'), M::CONTROL);
    let Some(Request::Run(op)) = panel.key(save) else {
        panic!("missing add request");
    };
    assert!(panel.key(save).is_none());
    panel.complete(&op, Err(anyhow::anyhow!("write failed")));
    assert!(panel.message.contains("write failed"));
    assert!(
        matches!(panel.key(save),Some(Request::Run(Operation::Add{title,body})) if title=="Task with 中文" && body=="first\nsecond")
    );
}

#[test]
fn failed_refresh_blocks_stale_actions_and_busy_operations_block_project_switches() {
    let mut panel = Panel::default();
    panel.absorb(Snapshot::default());
    panel.read_error = Some("unavailable".into());
    for c in ['g', 'n', 'p', 'l', 'a'] {
        assert!(panel.key(key(K::Char(c))).is_none());
        assert!(matches!(panel.page, Page::List));
    }
    panel.absorb(Snapshot::default());
    assert!(panel.read_error.is_none());
    assert!(matches!(
        panel.key(key(K::Char('p'))),
        Some(Request::Run(Operation::Pause(true)))
    ));
    assert!(panel.key(key(K::Char('c'))).is_none());
    assert!(matches!(panel.page, Page::List));
    panel.complete(&Operation::Pause(true), Ok("paused".into()));
    assert!(matches!(
        panel.key(key(K::Char('c'))),
        Some(Request::Projects)
    ));
    assert!(matches!(panel.page, Page::Projects));
}

#[test]
fn edit_draft_survives_refresh_errors_and_busy_input_then_follows_renamed_task() {
    let mut panel = Panel::default();
    let snapshot: Snapshot = serde_json::from_str(r#"{"mode":{},"paused":false,"current":null,"awaiting":null,"pending":[{"id":null,"title":"Original","body":"Body"},{"id":"T2","title":"Other"}],"history":[]}"#).unwrap();
    panel.absorb(snapshot.clone());
    panel.key(key(K::Char('e')));
    panel.paste(" revised");
    panel.key(key(K::Tab));
    panel.paste("\nsecond line");
    panel.absorb(snapshot.clone());
    let save = KeyEvent::new(K::Char('s'), M::CONTROL);
    panel.read_error = Some("read failed".into());
    assert!(panel.key(save).is_none());
    panel.absorb(snapshot.clone());
    let Some(Request::Run(op)) = panel.key(save) else {
        panic!("missing edit")
    };
    assert!(panel.key(save).is_none());
    panel.paste("must not append while busy");
    panel.key(key(K::Esc));
    assert!(matches!(panel.page, Page::Edit { .. }));
    panel.complete(&op, Err(anyhow::anyhow!("actual CLI write error")));
    assert!(panel.message.contains("actual CLI write error"));
    let mut fresh = snapshot;
    fresh.pending[0].title = "Concurrent edit".into();
    panel.absorb(fresh.clone());
    let Some(Request::Run(retry)) = panel.key(save) else {
        panic!("missing retry")
    };
    assert_eq!(
        retry, op,
        "refresh must preserve both the draft and its original baseline"
    );
    panel.complete(&retry, Ok("saved".into()));
    fresh.pending[0].title = "Original revised".into();
    fresh.pending[0].body = "Body\nsecond line".into();
    fresh.current = Some(saddle::drover::Task {
        title: "Current".into(),
        ..Default::default()
    });
    panel.absorb(fresh);
    assert_eq!(panel.selected, 1);
    assert!(matches!(panel.page, Page::List));
    panel.key(key(K::Char('e')));
    panel.key(key(K::Esc));
    assert!(matches!(panel.page, Page::List));
}

#[test]
fn pending_actions_ignore_other_states_overlays_busy_and_read_errors() {
    let mut panel = Panel::default();
    panel.absorb(serde_json::from_str(r#"{"mode":{},"paused":false,"current":{"title":"Current"},"awaiting":{"title":"Awaiting"},"pending":[{"title":"Pending"}],"history":[{"title":"History"}]}"#).unwrap());
    for index in [0, 1, 3] {
        panel.select(index);
        for c in ['e', 'u', 'd'] {
            assert!(panel.key(key(K::Char(c))).is_none());
            assert!(matches!(panel.page, Page::List));
        }
    }
    panel.select(2);
    panel.busy = true;
    assert!(panel.key(key(K::Char('e'))).is_none());
    assert!(matches!(panel.page, Page::List));
    panel.busy = false;
    panel.read_error = Some("offline".into());
    assert!(panel.key(key(K::Char('e'))).is_none());
    assert!(matches!(panel.page, Page::List));
    panel.read_error = None;
    panel.key(key(K::Char('?')));
    for c in ['e', 'u', 'd'] {
        assert!(panel.key(key(K::Char(c))).is_none());
    }
    assert!(matches!(panel.page, Page::Help));
}

#[test]
fn all_pending_opens_a_read_only_overlay_even_when_the_current_project_failed() {
    let mut panel = Panel::default();
    panel.absorb(Snapshot::default());
    panel.read_error = Some("current project unavailable".into());
    assert!(matches!(
        panel.key(key(K::Char('A'))),
        Some(Request::AllPending)
    ));
    assert!(matches!(panel.page, Page::AllPending));
    for c in ['g', 'n', 'p', 'l', 'a', 'e', 'u', 'd', 'c', '?'] {
        assert!(panel.key(key(K::Char(c))).is_none(), "{c}");
        assert!(matches!(panel.page, Page::AllPending), "{c}");
    }
    assert!(matches!(
        panel.key(key(K::Char('r'))),
        Some(Request::AllPending)
    ));
    panel.key(key(K::Esc));
    assert!(matches!(panel.page, Page::List));
    panel.read_error = None;
    panel.busy = true;
    assert!(panel.key(key(K::Char('A'))).is_none());
    assert!(matches!(panel.page, Page::List));
}
