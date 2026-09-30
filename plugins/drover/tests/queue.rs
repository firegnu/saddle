use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use saddle_drover_plugin::{
    drover::{Operation, Request, Snapshot},
    queue::{Page, Panel, View},
};
fn key(code: K) -> KeyEvent {
    KeyEvent::new(code, M::NONE)
}

#[test]
fn selected_pending_shows_beside_the_list_and_edits_return_to_the_same_view() {
    let mut panel = Panel::default();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "current": null, "awaiting": null, "history": [],
            "pending": [{"id":"T2", "title":"Second", "body":"second body"}]
        }))
        .unwrap(),
    );
    assert_eq!(panel.view, View::Text, "the task text shows first");
    assert!(
        panel.content.is_some(),
        "the selected task shows beside the list"
    );
    panel.key(key(K::Char('e')));
    assert!(matches!(&panel.page, Page::Edit { title, body, .. }
        if title.text == "Second" && body.text == "second body"));
    panel.paste(" discarded");
    panel.key(key(K::Esc));
    assert!(matches!(panel.page, Page::List) && panel.view == View::Text);

    panel.key(key(K::Enter));
    assert!(matches!(panel.page, Page::List) && panel.view == View::Details);
    panel.key(key(K::Char('e')));
    assert!(
        matches!(panel.page, Page::Edit { .. }),
        "Run details keep Edit"
    );
    panel.paste(" revised");
    let Some(Request::Run(op)) = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL)) else {
        panic!("Edit must submit a public operation");
    };
    assert!(
        matches!(&op, Operation::Edit { index:0, title, body, .. } if title=="Second revised" && body=="second body")
    );
    panel.complete(&op, Ok("saved".into()));
    let mut fresh = panel.snapshot.clone().unwrap();
    fresh.pending[0].title = "Second revised".into();
    panel.absorb(fresh);
    assert!(
        matches!(panel.page, Page::List) && panel.view == View::Details,
        "Save returns to the same task and view"
    );
    panel.key(key(K::Char('e')));
    assert!(matches!(&panel.page, Page::Edit { title, .. } if title.text == "Second revised"));
}

#[test]
fn edit_follows_the_selected_task_and_preserves_the_form_on_failure() {
    let mut panel = Panel::default();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "history": [],
            "pending": [{"id":"T1", "title":"First"}, {"id":"T2", "title":"Second"}]
        }))
        .unwrap(),
    );
    panel.select(1);
    let mut fresh = panel.snapshot.clone().unwrap();
    fresh.pending.swap(0, 1);
    panel.absorb(fresh.clone());
    assert_eq!(panel.selected, 0, "the selection follows T2");
    panel.key(key(K::Char('e')));
    panel.paste(" changed");
    let Some(Request::Run(op)) = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL)) else {
        panic!("missing edit");
    };
    assert!(
        matches!(&op, Operation::Edit { index:0, title, body, .. } if title=="Second changed" && body.is_empty())
    );
    panel.key(key(K::Esc));
    assert!(
        matches!(panel.page, Page::Edit { .. }),
        "Busy form stays open"
    );
    panel.complete(&op, Err(anyhow::anyhow!("pending changed")));
    assert!(matches!(&panel.page, Page::Edit { title, .. } if title.text == "Second changed"));
    fresh.current = Some(fresh.pending.remove(0));
    panel.absorb(fresh);
    panel.key(key(K::Esc));
    assert!(matches!(panel.page, Page::List));
    panel.key(key(K::Enter));
    let target = panel.detail_key().unwrap();
    assert_eq!(target.id, "T2");
    panel.absorb_detail(&target, Ok(show("T2", "current", "doing")));
    assert_eq!(
        opened(&panel).data.as_ref().unwrap().task.id.as_deref(),
        Some("T2")
    );
    // A running task is read-only.
    panel.key(key(K::Char('e')));
    assert!(matches!(panel.page, Page::List));
}

#[test]
fn saving_an_unnumbered_edit_keeps_it_selected() {
    let mut panel = Panel::default();
    panel.absorb(Snapshot {
        pending: vec![saddle_drover_plugin::drover::Task {
            title: "Original".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    panel.key(key(K::Char('e')));
    panel.paste(" revised");
    let Some(Request::Run(op)) = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL)) else {
        panic!("missing edit");
    };
    panel.complete(&op, Ok("saved".into()));
    assert!(matches!(panel.page, Page::List));
    let mut fresh = panel.snapshot.clone().unwrap();
    fresh.pending[0].title = "Original revised".into();
    panel.absorb(fresh);
    assert_eq!(panel.tasks()[panel.selected].1.title, "Original revised");
    panel.key(key(K::Char('e')));
    assert!(matches!(&panel.page, Page::Edit { title, .. } if title.text == "Original revised"));
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
    assert!(
        matches!(&op, Operation::Edit { index:1, title, body, .. } if title=="Second revised" && body=="second body\nextra line")
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
    assert!(matches!(
        &op,
        Operation::Move {
            index: 1,
            to: 0,
            ..
        }
    ));
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
    assert!(matches!(
        &op,
        Operation::Move {
            index: 0,
            to: 1,
            ..
        }
    ));
}

#[test]
fn moving_identical_unnumbered_tasks_keeps_the_destination_selected() {
    let mut panel = Panel::default();
    let task = saddle_drover_plugin::drover::Task {
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
    assert!(matches!(panel.page, Page::List) && panel.view == View::Details);
    assert_eq!(
        panel.selected, 1,
        "details show the selection, not a new page"
    );
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
    fresh.current = Some(saddle_drover_plugin::drover::Task {
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

#[test]
fn selected_pending_delete_confirms_a_fixed_target_and_can_be_cancelled() {
    let mut panel = Panel::default();
    let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
        "mode": {}, "paused": false, "history": [{"title":"Old", "status":"done"}],
        "current": {"id":"T0", "title":"Running"},
        "awaiting": {"id":"T9", "title":"Waiting"},
        "pending": [
            {"id":"T1", "title":"First"},
            {"id":null, "title":"Unnumbered", "body":"second body"},
            {"id":"T3", "title":"Third"}
        ]
    }))
    .unwrap();
    panel.absorb(snapshot.clone());
    for index in [0, 1, 5] {
        panel.select(index);
        assert!(panel.key(key(K::Char('x'))).is_none());
        assert!(
            matches!(panel.page, Page::List),
            "x must ignore row {index}"
        );
    }
    panel.select(3);
    assert!(panel.key(key(K::Char('x'))).is_none());
    assert!(matches!(panel.page, Page::Delete { .. }));
    panel.key(key(K::Esc));
    assert!(
        matches!(panel.page, Page::List),
        "Esc must cancel the delete"
    );
    assert!(panel.key(key(K::Char('x'))).is_none());
    // A refresh and a selection change must not retarget the open confirmation.
    let mut fresh = snapshot.clone();
    fresh.pending.remove(0);
    panel.absorb(fresh);
    panel.select(0);
    for c in ['g', 'n', 'e', 'u', 'd', 'a', 'x'] {
        assert!(panel.key(key(K::Char(c))).is_none(), "{c}");
    }
    assert!(matches!(panel.page, Page::Delete { .. }));
    let Some(Request::Run(op)) = panel.key(key(K::Char('y'))) else {
        panic!("y must confirm the delete through a public operation");
    };
    assert!(matches!(&op, Operation::Delete { index: 1, .. }));
    let Operation::Delete { pending, index } = &op else {
        panic!("unexpected operation {op:?}");
    };
    assert_eq!(
        (pending.as_slice(), *index),
        (snapshot.pending.as_slice(), 1)
    );
    assert!(panel.busy);
    assert!(
        panel.key(key(K::Char('y'))).is_none(),
        "busy must block repeats"
    );
    panel.complete(&op, Ok("dropped".into()));
    assert!(matches!(panel.page, Page::List));
    assert!(!panel.busy);
    let mut fresh = snapshot;
    let dropped = fresh.pending.remove(1);
    fresh.history.push(dropped);
    panel.absorb(fresh);
    assert_eq!(
        panel.tasks()[panel.selected].1.title,
        "Third",
        "selection must follow the next pending task, not the dropped history entry"
    );
}

fn show(id: &str, location: &str, status: &str) -> saddle_drover_plugin::drover::Detail {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    value["task"]["id"] = id.into();
    value["task"]["run_id"] = serde_json::Value::Null;
    value["task"]["status"] = match location {
        "current" => "running",
        "awaiting" => "awaiting_release",
        _ => status,
    }
    .into();
    serde_json::from_value(value).unwrap()
}
fn opened(panel: &Panel) -> &saddle_drover_plugin::detail::TaskDetail {
    panel.content.as_deref().expect("a task is selected")
}

#[test]
fn details_follow_the_task_id_through_completion_and_ignore_older_targets() {
    let mut panel = Panel::default();
    panel.project = "/tmp/project-a".into();
    let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
        "mode": {}, "paused": false, "awaiting": null,
        "current": {"id":"T4", "title":"Running"},
        "pending": [{"id":"T5", "title":"Next"}],
        "history": [{"id":"T3", "title":"Old", "status":"done"}]
    }))
    .unwrap();
    panel.absorb(snapshot);
    assert_eq!(panel.detail_key(), None);
    assert!(panel.key(key(K::Enter)).is_none());
    let first = panel.detail_key().expect("current task must be queried");
    assert_eq!(
        (first.project.as_str(), first.id.as_str()),
        ("/tmp/project-a", "T4")
    );
    // T4 finishes and T5 starts: the list index 0 is now another task.
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "pending": [],
            "current": {"id":"T5", "title":"Next"},
            "awaiting": {"id":"T4", "title":"Running", "status":"done"},
            "history": [{"id":"T3", "title":"Old", "status":"done"}]
        }))
        .unwrap(),
    );
    let awaiting = panel.detail_key().unwrap();
    assert_ne!(awaiting, first, "group changes invalidate old reads");
    panel.absorb_detail(&awaiting, Ok(show("T4", "awaiting", "done")));
    assert_eq!(
        opened(&panel).data.as_ref().unwrap().task.status.as_deref(),
        Some("awaiting_release")
    );
    // Switching views keeps the same target; only Run details are queried.
    panel.key(key(K::Char('t')));
    assert_eq!(panel.detail_key(), None);
    panel.key(key(K::Enter));
    assert_eq!(panel.detail_key(), Some(awaiting.clone()));
    // Selecting it again after another task is a new target; old results are dropped.
    panel.select(0);
    assert_eq!(panel.detail_key().unwrap().id, "T5");
    panel.select(1);
    let second = panel.detail_key().unwrap();
    assert_eq!(second.id, "T4");
    assert_ne!(second, first);
    panel.absorb_detail(&first, Ok(show("T4", "awaiting", "done")));
    panel.absorb_detail(&first, Err(anyhow::anyhow!("old failure")));
    assert!(opened(&panel).data.is_none() && opened(&panel).error.is_none());
    let other_project = saddle_drover_plugin::queue::DetailKey {
        project: "/tmp/project-b".into(),
        ..second.clone()
    };
    panel.absorb_detail(&other_project, Ok(show("T4", "awaiting", "done")));
    assert!(opened(&panel).data.is_none());
    panel.absorb_detail(&second, Ok(show("T4", "awaiting", "done")));
    assert!(opened(&panel).data.is_some());
}

#[test]
fn numbered_pending_supports_show_while_unnumbered_tasks_use_list_data() {
    let mut panel = Panel::default();
    let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
        "mode": {}, "paused": false, "current": null, "awaiting": null,
        "pending": [{"id":"T2", "title":"Queued"}, {"title":"Unnumbered pending"}],
        "history": [{"title":"Unnumbered old", "status":"dropped", "reason":"gone"}, {"id":"X9", "title":"Odd id", "status":"done"}]
    }))
    .unwrap();
    panel.absorb(snapshot.clone());
    panel.key(key(K::Enter));
    for index in 0..4 {
        panel.select(index);
        assert_eq!(panel.view, View::Details, "row {index}");
        assert_eq!(
            panel.detail_key().is_some(),
            index == 0,
            "only numbered Pending is queryable; row {index}"
        );
    }
    panel.select(0);
    let mut started = snapshot;
    started.current = Some(started.pending.remove(0));
    panel.absorb(started);
    assert_eq!(panel.detail_key().unwrap().id, "T2");
}

#[test]
fn content_keys_scroll_the_text_without_moving_the_list() {
    let mut panel = Panel::default();
    let history: Vec<_> = (0..30)
        .map(|i| serde_json::json!({"id": format!("T{i}"), "title": format!("Done {i}"), "status":"done"}))
        .collect();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "current": {"id":"T99", "title":"Now"}, "awaiting": null,
            "pending": [{"id":"T100", "title":"Queued"}], "history": history
        }))
        .unwrap(),
    );
    panel.select(7);
    panel.top = 5;
    let selected_title = panel.tasks()[panel.selected].1.title.clone();
    panel.key(key(K::Enter));
    for code in [K::PageDown, K::PageUp, K::Char('t'), K::Enter] {
        assert!(panel.key(key(code)).is_none());
        assert!(matches!(panel.page, Page::List));
        assert_eq!(panel.tasks()[panel.selected].1.title, selected_title);
        assert_eq!(panel.top, 5);
    }
    assert_eq!(panel.view, View::Details);
    panel.key(key(K::Down));
    assert_eq!(panel.selected, 8);
    assert_eq!(
        panel.view,
        View::Details,
        "the chosen view stays across tasks"
    );
}

#[test]
fn task_fields_edit_at_the_cursor() {
    let mut panel = Panel::default();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "current": null, "awaiting": null, "history": [],
            "pending": [{"id":"T2", "title":"ab", "body":"ab\n中d"}]
        }))
        .unwrap(),
    );
    panel.key(key(K::Char('e')));
    for code in [
        K::Left,
        K::Char('X'),
        K::Home,
        K::Char('Y'),
        K::Delete,
        K::End,
    ] {
        panel.key(key(code));
    }
    panel.key(key(K::Left));
    panel.key(key(K::Backspace));
    panel.paste("P");
    panel.key(key(K::Tab));
    // Body cursor starts at the end: after 中d on the second line.
    for code in [K::Up, K::Home, K::Right, K::Char('1'), K::Down, K::Enter] {
        panel.key(key(code));
    }
    panel.paste("中");
    let request = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL));
    let Some(Request::Run(Operation::Edit { title, body, .. })) = request else {
        panic!("Ctrl-S saves the edit");
    };
    assert_eq!((title.as_str(), body.as_str()), ("YPb", "a1b\n中\n中d"));
}

#[test]
fn dispatch_is_the_third_view_and_queries_only_explicit_task_numbers() {
    let mut panel = Panel::default();
    panel.project = "/tmp/project-a".into();
    panel.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "current": {"id":"T38", "title":"Now"}, "awaiting": null,
            "pending": [{"title":"Unnumbered"}], "history": []
        }))
        .unwrap(),
    );
    let mut seen = Vec::new();
    for _ in 0..4 {
        panel.key(key(K::Tab));
        seen.push(panel.view);
    }
    assert_eq!(
        seen,
        [View::Details, View::Dispatch, View::Links, View::Text]
    );
    panel.key(key(K::BackTab));
    panel.key(key(K::BackTab));
    assert_eq!(panel.view, View::Dispatch);
    // Dispatch reads its own records, not `drover show`.
    assert_eq!(panel.detail_key(), None);
    let dispatch = panel.dispatch_key("dlog").unwrap();
    assert_eq!(
        (dispatch.project.as_str(), dispatch.task.as_str()),
        ("/tmp/project-a", "T38")
    );
    panel.select(1);
    assert_eq!(
        panel.dispatch_key("dlog"),
        None,
        "no guessing without a task number"
    );
    assert!(panel.key(key(K::Enter)).is_none());
    assert_eq!(
        panel.view,
        View::Dispatch,
        "Enter opens an entry, not Run details"
    );
}

fn three_pending(panel: &mut Panel, state: serde_json::Value) {
    panel.project = "/tmp/project-a".into();
    let mut value = serde_json::json!({
        "mode": {"loop": false, "gate": true}, "paused": false, "current": null,
        "awaiting": null, "history": [{"id":"T0", "title":"Old", "status":"done"}],
        "pending": [
            {"id":"T1", "title":"First", "actions":{"dispatch-pending":{"pos":1, "target_token":"d1:one", "unavailable_reason":null}}},
            {"id":"T2", "title":"Second", "actions":{"dispatch-pending":{"pos":2, "target_token":"d1:two", "unavailable_reason":null}}},
            {"id":"T3", "title":"Third", "actions":{"dispatch-pending":{"pos":3, "target_token":"d1:three", "unavailable_reason":null}}}
        ]
    });
    for (field, v) in state.as_object().unwrap() {
        value[field] = v.clone();
    }
    panel.absorb(serde_json::from_value(value).unwrap());
}

#[test]
fn dispatch_selected_sends_the_selected_pending_target_not_the_first() {
    let mut panel = Panel::default();
    three_pending(&mut panel, serde_json::json!({}));
    panel.select(1);
    assert_eq!(panel.tasks()[panel.selected].1.title, "Second");
    let Some(Request::Run(op)) = panel.key(saddle_drover_plugin::queue::dispatch_selected_click())
    else {
        panic!("Dispatch selected sends one public write");
    };
    assert_eq!(
        op,
        Operation::DispatchPending {
            project: "/tmp/project-a".into(),
            pos: 2,
            token: "d1:two".into(),
        }
    );
    assert!(panel.busy);
    assert!(
        panel
            .key(saddle_drover_plugin::queue::dispatch_selected_click())
            .is_none(),
        "no second dispatch while busy"
    );
    assert!(panel.key(key(K::Char('n'))).is_none(), "nor Next");
    panel.complete(&op, Ok("T2 dispatched".into()));
    assert!(!panel.busy);
    assert!(matches!(&panel.page, Page::Feedback(text) if text == "T2 dispatched"));

    panel.key(key(K::Esc));
    panel.complete(&op, Err(anyhow::anyhow!("Delivery result unknown")));
    assert!(matches!(&panel.page, Page::Feedback(text) if text.contains("unknown")));
    panel.key(key(K::Esc));
    assert!(panel.key(key(K::Char('n'))).is_none(), "Next is retired");
}

#[test]
fn dispatch_selected_is_refused_without_a_usable_target_or_when_the_queue_cannot_start_it() {
    let unusable = [
        serde_json::json!({"paused": true}),
        serde_json::json!({"current": {"id":"T9", "title":"Running"}}),
        serde_json::json!({"awaiting": {"id":"T9", "title":"Awaiting", "status":"done"}}),
        serde_json::json!({"pending": [{"id":"T1", "title":"First"}, {"id":"T2", "title":"Second"}]}),
        serde_json::json!({"pending": [{"id":"T1", "title":"First"}, {"id":"T2", "title":"Second",
            "actions":{"dispatch-pending":{"pos":2, "target_token":null, "unavailable_reason":"target_ambiguous"}}}]}),
        serde_json::json!({"pending": [{"id":"T1", "title":"First"}, {"id":"T2", "title":"Second",
            "actions":{"dispatch-pending":{"pos":2, "target_token":"d1:two", "unavailable_reason":"state_invalid"}}}]}),
        serde_json::json!({"pending": [{"id":"T1", "title":"First"}, {"id":"T2", "title":"Second",
            "actions":{"dispatch-pending":{"pos":0, "target_token":"d1:two", "unavailable_reason":null}}}]}),
        serde_json::json!({"pending": [{"id":"T1", "title":"First"}, {"id":"T2", "title":"Second",
            "actions":{"dispatch-pending":{"target_token":"d1:two", "unavailable_reason":null}}}]}),
    ];
    for state in unusable {
        let mut panel = Panel::default();
        three_pending(&mut panel, state.clone());
        let second = panel
            .tasks()
            .iter()
            .position(|(group, t)| *group == "Pending" && t.title == "Second")
            .unwrap();
        panel.select(second);
        assert!(
            panel
                .key(saddle_drover_plugin::queue::dispatch_selected_click())
                .is_none(),
            "{state}"
        );
        assert!(!panel.busy && matches!(panel.page, Page::List), "{state}");
    }
    // Only a selected Pending task, on the list, with fresh data.
    let mut panel = Panel::default();
    three_pending(&mut panel, serde_json::json!({}));
    let history = panel.tasks().len() - 1;
    panel.select(history);
    assert!(
        panel
            .key(saddle_drover_plugin::queue::dispatch_selected_click())
            .is_none()
    );
    panel.select(1);
    panel.read_error = Some("list failed".into());
    assert!(
        panel
            .key(saddle_drover_plugin::queue::dispatch_selected_click())
            .is_none()
    );
    panel.read_error = None;
    panel.key(key(K::Char('?')));
    assert!(
        panel
            .key(saddle_drover_plugin::queue::dispatch_selected_click())
            .is_none()
    );
    assert!(!panel.busy);
}
