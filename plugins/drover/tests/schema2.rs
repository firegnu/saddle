use serde_json::json;

fn detail_value(status: &str, run: &str) -> serde_json::Value {
    json!({"schema_version":2,"ok":true,"project":"/synthetic", "task":{
        "id":"T7","title":"Research","body":"No branch required","status":status,"run_id":run,
        "t0":10,"t1":null,"t2":null,"start":null,"main":null,
        "actions":{"done":{"target_token":"v2:shown-token","unavailable_reason":null},
            "return-to-pending":{"target_token":"v2:return-token","unavailable_reason":null}},
        "return_history":[],"previous_runs":[]},
        "evidence":{"scope":"repository_reference","controls_transition":false,"observed_at":20,
            "git":{"state":"unavailable","head_sha":null,"main_sha":null,"tracked_changes":null,"unmerged_local_branches":[]},
            "last_check":{"state":"stale","reason":"legacy cache","record":{"ok":false,"why":"failed earlier","cmd":"check","t":1}}}})
}

#[test]
fn retired_queue_shortcuts_cannot_write_or_advance_another_task() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut panel = saddle_drover_plugin::queue::Panel::default();
    panel.absorb(saddle_drover_plugin::drover::Snapshot::default());
    for key in ['g', 'n', 'l'] {
        assert!(
            panel
                .key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
                .is_none(),
            "retired {key}"
        );
    }
}

#[test]
fn notification_identity_is_the_public_key_without_git_endpoints() {
    use saddle_drover_plugin::{
        drover::{Preference, Snapshot, Task},
        notify::Notifier,
    };
    let now = std::time::Instant::now();
    let mut notifier = Notifier::default();
    notifier.preference(Some(Preference {
        system_enabled: false,
        revision: 1,
    }));
    notifier.snapshot("/synthetic", &Snapshot::default(), now);
    let mut task: Task = serde_json::from_value(json!({"id":"T7","status":"awaiting_release","run_id":"run-a","notification_key":"opaque-public-key"})).unwrap();
    notifier.snapshot(
        "/synthetic",
        &Snapshot {
            awaiting: Some(task.clone()),
            ..Default::default()
        },
        now,
    );
    assert!(
        notifier.visible(now).is_some(),
        "Git endpoints must not be required for the public identity"
    );
    notifier.dismiss();
    task.start = Some("changed-repository-ref".into());
    notifier.snapshot(
        "/synthetic",
        &Snapshot {
            awaiting: Some(task.clone()),
            ..Default::default()
        },
        now,
    );
    assert!(
        notifier.visible(now).is_none(),
        "same public key must deduplicate regardless of Git"
    );
    task.notification_key = Some("another-run-key".into());
    notifier.snapshot(
        "/synthetic",
        &Snapshot {
            awaiting: Some(task),
            ..Default::default()
        },
        now,
    );
    assert!(notifier.visible(now).is_some());
}

fn key(code: crossterm::event::KeyCode) -> crossterm::event::KeyEvent {
    crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
}

#[test]
fn selected_transitions_bind_run_token_and_reconfirm_after_failure() {
    use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
    use saddle_drover_plugin::{
        drover::{Operation, Request, Snapshot, Transition},
        queue::{Page, Panel},
    };
    for (status, action) in [
        ("running", Transition::Submit),
        ("awaiting_release", Transition::Accept),
        ("running", Transition::Return),
        ("awaiting_release", Transition::Return),
    ] {
        let mut detail = detail_value(status, "run-a");
        detail["task"]["actions"][action.command()] =
            json!({"target_token":"v2:$(touch bad) 'literal'","unavailable_reason":null});
        let task = serde_json::from_value(detail["task"].clone()).unwrap();
        let mut panel = Panel::default();
        panel.project = "/synthetic".into();
        panel.absorb(if status == "running" {
            Snapshot {
                current: Some(task),
                ..Default::default()
            }
        } else {
            Snapshot {
                awaiting: Some(task),
                ..Default::default()
            }
        });
        let click = if action == Transition::Return {
            saddle_drover_plugin::queue::return_click()
        } else {
            saddle_drover_plugin::queue::transition_click()
        };
        assert!(panel.key(click).is_none());
        let target = panel.confirmation_key().unwrap();
        assert!(panel.key(key(K::Enter)).is_none(), "no target yet");
        panel.absorb_confirmation(&target, Ok(serde_json::from_value(detail.clone()).unwrap()));
        if action == Transition::Return {
            assert!(
                panel.key(key(K::Enter)).is_none(),
                "reason and stopped required"
            );
            panel.paste("needs more research");
            assert!(
                panel.key(key(K::Enter)).is_none(),
                "stopped confirmation required"
            );
            panel.key(KeyEvent::new(K::Char('w'), M::CONTROL));
        }
        let Some(Request::Run(op)) = panel.key(key(K::Enter)) else {
            panic!("must confirm {action:?}");
        };
        assert!(
            matches!(&op, Operation::Transition { project, id, run_id, token, action: got, .. }
            if project == "/synthetic" && id == "T7" && run_id == "run-a" && token == "v2:$(touch bad) 'literal'" && *got == action)
        );
        assert!(
            panel.key(key(K::Enter)).is_none(),
            "no duplicate during write"
        );
        assert!(panel.key(key(K::Esc)).is_none() && matches!(panel.page, Page::Confirm(_)));
        panel.complete(
            &op,
            Err(saddle_drover_plugin::drover::TransitionError {
                code: "target_changed".into(),
                why: "new run".into(),
            }
            .into()),
        );
        assert!(
            panel.key(key(K::Enter)).is_none(),
            "no automatic or user retry of old target"
        );
        panel.key(KeyEvent::new(K::Char('r'), M::CONTROL));
        let refreshed = panel.confirmation_key().unwrap();
        assert_ne!(refreshed, target);
        panel.absorb_confirmation(&target, Ok(serde_json::from_value(detail.clone()).unwrap()));
        assert_eq!(
            panel.confirmation_key(),
            Some(refreshed.clone()),
            "late target discarded"
        );
        detail["task"]["actions"][action.command()]["target_token"] = "v2:fresh".into();
        panel.absorb_confirmation(&refreshed, Ok(serde_json::from_value(detail).unwrap()));
        if action == Transition::Return {
            assert!(
                panel.key(key(K::Enter)).is_none(),
                "work stopped must be reconfirmed"
            );
            panel.key(KeyEvent::new(K::Char('w'), M::CONTROL));
        }
        let Some(Request::Run(op)) = panel.key(key(K::Enter)) else {
            panic!("fresh confirmation");
        };
        assert!(matches!(&op, Operation::Transition { token, .. } if token=="v2:fresh"));
        panel.complete(&op, Ok("recorded".into()));
        assert!(matches!(panel.page, Page::Feedback(_)));
        assert!(
            panel.key(key(K::Enter)).is_none(),
            "success never chains another transition"
        );
    }
}

#[test]
fn a_new_run_in_the_same_group_discards_old_detail_results() {
    use saddle_drover_plugin::{drover::Snapshot, queue::Panel};
    let mut panel = Panel::default();
    let snapshot = |run: &str| Snapshot {
        current: Some(
            serde_json::from_value(detail_value("running", run)["task"].clone()).unwrap(),
        ),
        ..Default::default()
    };
    panel.absorb(snapshot("run-a"));
    panel.key(key(crossterm::event::KeyCode::Enter));
    let old = panel.detail_key().unwrap();
    panel.absorb(snapshot("run-b"));
    assert_ne!(panel.detail_key(), Some(old.clone()));
    panel.absorb_detail(
        &old,
        Ok(serde_json::from_value(detail_value("running", "run-a")).unwrap()),
    );
    assert!(panel.content.as_ref().unwrap().data.is_none());
    let new = panel.detail_key().unwrap();
    panel.absorb_detail(
        &new,
        Ok(serde_json::from_value(detail_value("running", "run-a")).unwrap()),
    );
    assert!(
        panel.content.as_ref().unwrap().data.is_none(),
        "mismatched response run rejected"
    );
}
