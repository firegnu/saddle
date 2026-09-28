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
        status: Some("done".into()),
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
fn identity_follows_drover_v1_with_binary64_start_time() {
    let task = awaiting("T1", json!(100));
    assert_eq!(
        identity(PROJECT, &task).unwrap(),
        [
            PROJECT,
            "T1",
            "4059000000000000",
            "start-T1",
            "main-sha",
            "awaiting_release"
        ]
    );
    // 100 and 100.0 are one start time; -0 counts as +0.
    let float: serde_json::Value = serde_json::from_str("100.0").unwrap();
    assert_eq!(
        identity(PROJECT, &awaiting("T1", float)).unwrap()[2],
        "4059000000000000"
    );
    let zero: serde_json::Value = serde_json::from_str("-0.0").unwrap();
    assert_eq!(
        identity(PROJECT, &awaiting("T1", zero)).unwrap()[2],
        "0000000000000000"
    );
    // A fractional Unix time converts exactly, not through its decimal text.
    let t: serde_json::Value = serde_json::from_str("1790000000.123456").unwrap();
    assert_eq!(
        identity(PROJECT, &awaiting("T1", t)).unwrap()[2],
        format!("{:016x}", 1790000000.123456f64.to_bits())
    );
}

#[test]
fn missing_or_invalid_identity_fields_skip_the_prompt() {
    for task in [
        Task {
            t0: None,
            ..awaiting("T1", json!(1))
        },
        Task {
            t0: Some(json!(true)),
            ..awaiting("T1", json!(1))
        },
        Task {
            t0: Some(json!(null)),
            ..awaiting("T1", json!(1))
        },
        Task {
            start: Some(String::new()),
            ..awaiting("T1", json!(1))
        },
        Task {
            main: None,
            ..awaiting("T1", json!(1))
        },
        Task {
            id: None,
            ..awaiting("T1", json!(1))
        },
    ] {
        assert!(identity(PROJECT, &task).is_none(), "{task:?}");
    }
    let mut n = Notifier::default();
    let now = Instant::now();
    n.preference(in_saddle(1));
    n.snapshot(PROJECT, &snapshot(None), now);
    let broken = Task {
        main: None,
        ..awaiting("T2", json!(5))
    };
    n.snapshot(PROJECT, &snapshot(Some(broken)), now);
    assert!(n.visible(now).is_none());
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
