use saddle::{
    attention::Target,
    drover::{Preference, Snapshot, Task},
    notify::{Notifier, SHOWN, identity},
};
use serde_json::json;
use std::time::{Duration, Instant};

const PROJECT: &str = "/repo/saddle";

fn awaiting(id: &str, t0: serde_json::Value) -> Task {
    Task {
        id: Some(id.into()),
        title: format!("title {id}"),
        status: Some("awaiting_release".into()),
        notification_key: Some(format!("opaque/{id}/{t0}")),
        t0: Some(t0),
        start: Some(format!("start-{id}")),
        main: Some("main-sha".into()),
        ..Default::default()
    }
}
fn snapshot(task: Option<Task>) -> Snapshot {
    Snapshot {
        awaiting: task,
        ..Default::default()
    }
}
fn in_saddle(revision: u64) -> Option<Preference> {
    Some(Preference {
        system_enabled: false,
        revision,
    })
}
fn system(revision: u64) -> Option<Preference> {
    Some(Preference {
        system_enabled: true,
        revision,
    })
}
fn shown(n: &Notifier, now: Instant) -> Vec<String> {
    n.visible(now)
        .map(|t| t.targets.iter().map(|(_, label)| label.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn missing_public_notification_key_skips_the_prompt() {
    let task = Task {
        notification_key: None,
        ..awaiting("T1", json!(1))
    };
    assert!(identity(PROJECT, &task).is_none());
    let empty = Task {
        notification_key: Some(String::new()),
        ..task
    };
    assert!(identity(PROJECT, &empty).is_none());
}

#[test]
fn first_snapshot_is_a_baseline_and_a_new_awaiting_run_prompts_once() {
    let mut n = Notifier::default();
    let now = Instant::now();
    n.preference(in_saddle(2));
    // Already awaiting at start: baseline, no prompt.
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T1", json!(10)))), now);
    assert!(n.visible(now).is_none());
    n.snapshot(PROJECT, &snapshot(None), now);
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T2", json!(20)))), now);
    assert_eq!(shown(&n, now), ["saddle · T2"]);
    // Refreshes, changed text and a brief absence do not prompt again.
    n.dismiss();
    let renamed = Task {
        title: "new words".into(),
        ..awaiting("T2", json!(20))
    };
    n.snapshot(PROJECT, &snapshot(Some(renamed)), now);
    n.snapshot(PROJECT, &snapshot(None), now);
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T2", json!(20)))), now);
    assert!(n.visible(now).is_none());
    // The same id started again is another run and prompts.
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T2", json!(21)))), now);
    assert_eq!(shown(&n, now), ["saddle · T2"]);
    assert!(matches!(
        &n.visible(now).unwrap().targets[0].0,
        Target::Task { project, id: Some(id), .. } if project == PROJECT && id == "T2"
    ));
}

#[test]
fn system_or_unknown_channel_never_prompts_and_switching_sets_a_new_baseline() {
    let mut n = Notifier::default();
    let now = Instant::now();
    // Unknown: nothing, and no baseline either.
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T1", json!(1)))), now);
    n.preference(system(0));
    n.snapshot(PROJECT, &snapshot(None), now);
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T2", json!(2)))), now);
    assert!(n.visible(now).is_none());
    // Switching to In saddle: what is already awaiting is the baseline.
    n.preference(in_saddle(1));
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T2", json!(2)))), now);
    assert!(n.visible(now).is_none());
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T3", json!(3)))), now);
    assert_eq!(shown(&n, now), ["saddle · T3"]);
    n.dismiss();
    // A failed status read confirms nothing: no prompt.
    n.preference(None);
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T4", json!(4)))), now);
    assert!(n.visible(now).is_none());
    // Off and on again between two reads shows only as a new revision: a new baseline.
    n.preference(in_saddle(3));
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T4", json!(4)))), now);
    assert!(n.visible(now).is_none());
}

#[test]
fn a_failed_read_is_not_an_empty_baseline_and_new_projects_start_with_one() {
    let mut n = Notifier::default();
    let now = Instant::now();
    n.preference(in_saddle(1));
    n.registry(&[PROJECT.into()]);
    // The first read of PROJECT failed, so nothing was recorded; its first success is the
    // baseline.
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T1", json!(1)))), now);
    assert!(n.visible(now).is_none());
    let other = "/repo/other";
    n.registry(&[PROJECT.into(), other.into()]);
    n.snapshot(other, &snapshot(Some(awaiting("T9", json!(9)))), now);
    assert!(n.visible(now).is_none());
    n.snapshot(other, &snapshot(Some(awaiting("T10", json!(10)))), now);
    assert_eq!(shown(&n, now), ["other · T10"]);
}

#[test]
fn prompts_close_after_about_five_seconds_and_merge_while_shown() {
    let mut n = Notifier::default();
    let now = Instant::now();
    n.preference(in_saddle(1));
    let other = "/repo/other";
    n.snapshot(PROJECT, &snapshot(None), now);
    n.snapshot(other, &snapshot(None), now);
    n.snapshot(PROJECT, &snapshot(Some(awaiting("T1", json!(1)))), now);
    let later = now + Duration::from_secs(2);
    n.snapshot(other, &snapshot(Some(awaiting("T7", json!(7)))), later);
    assert_eq!(shown(&n, later), ["saddle · T1", "other · T7"]);
    assert_eq!(n.visible(later).unwrap().text(), "2 tasks ready for review");
    assert!(
        n.visible(later + SHOWN - Duration::from_millis(10))
            .is_some()
    );
    assert!(n.visible(later + SHOWN).is_none());
    // A later task after it closed starts a fresh one.
    let after = later + SHOWN + Duration::from_secs(1);
    n.snapshot(other, &snapshot(Some(awaiting("T8", json!(8)))), after);
    assert_eq!(
        n.visible(after).unwrap().text(),
        "other · T8 ready for review"
    );
}
