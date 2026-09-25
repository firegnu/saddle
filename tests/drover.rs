mod common;
use saddle::drover::Client;
#[test]
fn project_registry_preserves_order_deduplicates_and_reports_read_errors() {
    use saddle::drover::registered_projects;
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("projects");
    assert!(registered_projects(&registry).unwrap().is_empty());
    std::fs::write(
        &registry,
        "/tmp/saddle-first\n\n/tmp/saddle-second\n/tmp/saddle-first\n",
    )
    .unwrap();
    assert_eq!(
        registered_projects(&registry).unwrap(),
        ["/tmp/saddle-first", "/tmp/saddle-second"]
    );
    assert!(registered_projects(temp.path()).is_err());
}
#[test]
fn queue_reads_public_json_in_the_configured_project_without_launching_a_board() {
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
[ "$1" = list ] && [ "$2" = --json ] || exit 99
[ -f project-marker ] || exit 98
printf '%s\n' '{"mode":{"loop":true,"gate":true},"paused":true,"current":{"id":"T1","title":"Doing","body":"current body"},"awaiting":null,"pending":[{"id":null,"title":"中文任务","body":"task detail"}],"history":[{"id":"T0","title":"Past","status":"dropped","reason":"test"}]}'
"#,
    );
    std::fs::write(temp.path().join("project-marker"), "").unwrap();
    let snapshot = Client {
        program,
        cwd: temp.path().into(),
    }
    .snapshot()
    .unwrap();
    assert!(snapshot.paused && snapshot.mode.r#loop && snapshot.mode.gate);
    assert_eq!(snapshot.current.unwrap().id.as_deref(), Some("T1"));
    assert_eq!(snapshot.pending[0].title, "中文任务");
    assert_eq!(snapshot.pending[0].body, "task detail");
    assert_eq!(snapshot.history[0].reason.as_deref(), Some("test"));
}

#[test]
fn operations_use_literal_public_arguments_and_surface_failure_feedback() {
    use saddle::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
case "$1" in
 add) printf '%s\n%s' "$2" "$3" ;;
 pause|resume) printf '%s' "$1" ;;
 loop) printf 'loop %s' "$2" ;;
 go) echo 'criteria not met'; exit 9 ;;
 *) exit 99 ;;
esac
"#,
    );
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let cancel = AtomicBool::new(false);
    let title = "Literal $(touch bad) 中文";
    let body = "first\nsecond";
    assert_eq!(
        client
            .execute(
                &Operation::Add {
                    title: title.into(),
                    body: body.into()
                },
                &cancel
            )
            .unwrap(),
        format!("{title}\n{body}")
    );
    assert!(!temp.path().join("bad").exists());
    for (op, expected) in [
        (Operation::Pause(true), "pause"),
        (Operation::Pause(false), "resume"),
        (Operation::Loop(true), "loop on"),
        (Operation::Loop(false), "loop off"),
    ] {
        assert_eq!(client.execute(&op, &cancel).unwrap(), expected);
    }
    assert!(
        client
            .execute(&Operation::Go, &cancel)
            .unwrap_err()
            .to_string()
            .contains("criteria not met")
    );
}

#[test]
fn pending_edit_and_move_check_public_data_and_pass_literal_arguments() {
    use saddle::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let client = Client {
        program: common::script(temp.path(), "drover", include_str!("fixtures/drover.py")),
        cwd: temp.path().into(),
    };
    let cancel = AtomicBool::new(false);
    client
        .execute(
            &Operation::Add {
                title: "Second".into(),
                body: "Body".into(),
            },
            &cancel,
        )
        .unwrap();
    let op = Operation::Edit {
        pending: client.snapshot().unwrap().pending,
        index: 1,
        title: "中文 $(touch bad) 'quoted'".into(),
        body: "first\nsecond `touch bad`".into(),
    };
    std::fs::write(temp.path().join("queue-events"), "").unwrap();
    client.execute(&op, &cancel).unwrap();
    let events: Vec<serde_json::Value> = std::fs::read_to_string(temp.path().join("queue-events"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(
        events,
        [
            serde_json::json!(["list", "--json"]),
            serde_json::json!([
                "edit",
                "2",
                "中文 $(touch bad) 'quoted'",
                "first\nsecond `touch bad`"
            ])
        ]
    );
    let state = client.snapshot().unwrap();
    assert_eq!(state.pending[1].title, "中文 $(touch bad) 'quoted'");
    assert_eq!(state.pending[1].body, "first\nsecond `touch bad`");
    assert!(!temp.path().join("bad").exists());
    client
        .execute(
            &Operation::Move {
                pending: state.pending,
                index: 1,
                to: 0,
            },
            &cancel,
        )
        .unwrap();
    let state = client.snapshot().unwrap();
    assert_eq!(state.pending[0].id.as_deref(), Some("T2"));
    assert_eq!(state.pending[1].id.as_deref(), Some("T1"));
    std::fs::write(
        temp.path().join("write-error"),
        "queue locked: actual error",
    )
    .unwrap();
    let error = client
        .execute(
            &Operation::Edit {
                pending: state.pending,
                index: 0,
                title: "failed".into(),
                body: "".into(),
            },
            &cancel,
        )
        .unwrap_err();
    assert!(error.to_string().contains("queue locked: actual error"));
    assert_eq!(
        client.snapshot().unwrap().pending[0].title,
        "中文 $(touch bad) 'quoted'"
    );
}

#[test]
fn pending_delete_checks_public_data_and_drops_by_position() {
    use saddle::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let client = Client {
        program: common::script(temp.path(), "drover", include_str!("fixtures/drover.py")),
        cwd: temp.path().into(),
    };
    let cancel = AtomicBool::new(false);
    client
        .execute(
            &Operation::Add {
                title: "Second".into(),
                body: "Body".into(),
            },
            &cancel,
        )
        .unwrap();
    let pending = client.snapshot().unwrap().pending;
    let delete = Operation::Delete {
        pending: pending.clone(),
        index: 1,
    };
    std::fs::write(
        temp.path().join("write-error"),
        "queue locked: drop refused",
    )
    .unwrap();
    let error = client.execute(&delete, &cancel).unwrap_err();
    assert!(error.to_string().contains("queue locked: drop refused"));
    std::fs::remove_file(temp.path().join("write-error")).unwrap();
    std::fs::write(temp.path().join("queue-events"), "").unwrap();
    client.execute(&delete, &cancel).unwrap();
    let events: Vec<serde_json::Value> = std::fs::read_to_string(temp.path().join("queue-events"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(
        events,
        [
            serde_json::json!(["list", "--json"]),
            serde_json::json!(["drop", "--pos", "2", "Deleted in saddle"])
        ]
    );
    let state = client.snapshot().unwrap();
    assert_eq!(state.pending, pending[..1]);
    assert_eq!(state.history.last().unwrap().title, "Second");
    assert_eq!(
        state.history.last().unwrap().status.as_deref(),
        Some("dropped")
    );
    // The same confirmed target is now stale, so nothing more is written.
    std::fs::write(temp.path().join("queue-events"), "").unwrap();
    assert!(
        client
            .execute(&delete, &cancel)
            .unwrap_err()
            .to_string()
            .contains("Pending tasks changed")
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("queue-events")).unwrap(),
        "[\"list\", \"--json\"]\n"
    );
}

#[test]
fn stale_pending_content_order_or_state_never_sends_a_write() {
    use saddle::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let client = Client {
        program: common::script(temp.path(), "drover", include_str!("fixtures/drover.py")),
        cwd: temp.path().into(),
    };
    let state = serde_json::json!({"mode":{}, "paused":false, "current":null, "awaiting":null, "history":[], "pending":[
        {"id":null,"title":"First","body":"Original"}, {"id":"T2","title":"Second","body":"Other"}
    ]});
    let state_path = temp.path().join("queue-state.json");
    std::fs::write(&state_path, state.to_string()).unwrap();
    let pending = client.snapshot().unwrap().pending;
    let edit = Operation::Edit {
        pending: pending.clone(),
        index: 0,
        title: "Draft".into(),
        body: "Draft body".into(),
    };
    let movement = Operation::Move {
        pending,
        index: 1,
        to: 0,
    };
    let cancel = AtomicBool::new(false);
    std::fs::write(temp.path().join("queue-events"), "").unwrap();

    let mut changed = state.clone();
    changed["pending"][0]["body"] = "External edit".into();
    std::fs::write(&state_path, changed.to_string()).unwrap();
    assert!(
        client
            .execute(&edit, &cancel)
            .unwrap_err()
            .to_string()
            .contains("Pending tasks changed")
    );
    changed = state.clone();
    changed["pending"].as_array_mut().unwrap().swap(0, 1);
    std::fs::write(&state_path, changed.to_string()).unwrap();
    assert!(
        client
            .execute(&movement, &cancel)
            .unwrap_err()
            .to_string()
            .contains("Pending tasks changed")
    );
    changed = state;
    changed["current"] = changed["pending"].as_array_mut().unwrap().remove(0);
    std::fs::write(&state_path, changed.to_string()).unwrap();
    assert!(
        client
            .execute(&edit, &cancel)
            .unwrap_err()
            .to_string()
            .contains("Pending tasks changed")
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("queue-events")).unwrap(),
        "[\"list\", \"--json\"]\n".repeat(3)
    );
}

#[test]
fn all_pending_reads_every_project_through_public_json_and_keeps_failures_separate() {
    use saddle::drover::PendingLoad;
    use std::time::Duration;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
[ "$1" = list ] && [ "$2" = --json ] || exit 99
if [ -f fail ]; then echo 'synthetic unreadable queue' >&2; exit 3; fi
name=$(basename "$PWD")
printf '{"mode":{},"paused":false,"current":{"title":"not pending"},"awaiting":null,"pending":[{"id":"T1","title":"%s 中文待办","body":"body"}],"history":[{"title":"old"}]}\n' "$name"
"#,
    );
    let projects: Vec<String> = ["alpha", "broken", "gamma"]
        .iter()
        .map(|name| {
            let dir = temp.path().join(name);
            std::fs::create_dir(&dir).unwrap();
            dir.display().to_string()
        })
        .collect();
    std::fs::write(temp.path().join("broken/fail"), "").unwrap();
    let load = PendingLoad::start(&program, &projects);
    let mut results: Vec<_> = (0..3)
        .map(|_| load.updates.recv_timeout(Duration::from_secs(10)).unwrap())
        .collect();
    results.sort_by_key(|(index, _)| *index);
    let titles = |i: usize| -> Vec<String> {
        results[i]
            .1
            .as_ref()
            .unwrap()
            .iter()
            .map(|t| t.title.clone())
            .collect()
    };
    assert_eq!(titles(0), ["alpha 中文待办"]);
    assert_eq!(titles(2), ["gamma 中文待办"]);
    let error = format!("{:#}", results[1].1.as_ref().unwrap_err());
    assert!(error.contains("synthetic unreadable queue"), "{error}");
}

#[test]
fn task_detail_reads_public_show_json_and_rejects_failures_or_unknown_schemas() {
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
[ $# = 4 ] && [ "$1 $2 $3 $4" = "show T4 --json --with-agent-status" ] || exit 99
[ -f project-marker ] || exit 98
cat response
exit $(cat code)
"#,
    );
    std::fs::write(temp.path().join("project-marker"), "").unwrap();
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let respond = |body: &str, code: i32| {
        std::fs::write(temp.path().join("response"), body).unwrap();
        std::fs::write(temp.path().join("code"), code.to_string()).unwrap();
    };
    let cancel = AtomicBool::new(false);
    let good: serde_json::Value = serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    respond(&good.to_string(), 0);
    let detail = client.show("T4", &cancel).unwrap();
    assert_eq!(
        (detail.task.id.as_str(), detail.task.location.as_str()),
        ("T4", "current")
    );
    assert_eq!(detail.task.body, "原始正文\n第二行");
    assert_eq!(detail.timing.release_wait_seconds, None);
    assert_eq!(detail.git.range_commits, Some(7));
    assert_eq!(detail.git.end_head, None);
    assert_eq!(
        detail.git.unavailable_reasons["end_head"],
        "end_head_not_recorded"
    );
    let rows = detail.completion.rows.as_ref().unwrap();
    assert_eq!(
        rows.iter().map(|r| r.state.as_str()).collect::<Vec<_>>(),
        ["unmet", "met", "unavailable", "not_run"]
    );
    assert_eq!(detail.last_check.status, "missing");
    assert_eq!(detail.hold.enabled, None);
    assert_eq!(detail.attention.unmet_rows, ["completion_marker"]);
    assert_eq!(
        detail.attention.agent.as_ref().unwrap().idle_for,
        Some(300.0)
    );

    let mut newer = good.clone();
    newer["schema_version"] = 2.into();
    respond(&newer.to_string(), 0);
    let error = format!("{:#}", client.show("T4", &cancel).unwrap_err());
    assert!(error.contains("schema_version"), "{error}");

    respond(
        r#"{"schema_version":1,"ok":false,"observed_at":1,"error":{"code":"task_not_found","why":"没有该任务"}}"#,
        2,
    );
    let error = format!("{:#}", client.show("T4", &cancel).unwrap_err());
    assert!(
        error.contains("task_not_found") && error.contains("没有该任务"),
        "{error}"
    );

    let mut partial = good.clone();
    partial.as_object_mut().unwrap().remove("completion");
    respond(&partial.to_string(), 0);
    assert!(client.show("T4", &cancel).is_err());

    respond(&good.to_string(), 3);
    assert!(client.show("T4", &cancel).is_err());

    respond("Traceback: synthetic crash", 1);
    let error = format!("{:#}", client.show("T4", &cancel).unwrap_err());
    assert!(error.contains("synthetic crash"), "{error}");
}

#[test]
fn detail_worker_queries_one_at_a_time_and_stops_when_dropped() {
    use saddle::drover::DetailWorker;
    use std::time::{Duration, Instant};
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("response"),
        include_str!("fixtures/show.json"),
    )
    .unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
echo start >> calls
[ -f hang ] && exec sleep 30
sleep 0.1
echo end >> calls
cat response
"#,
    );
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let calls = || std::fs::read_to_string(temp.path().join("calls")).unwrap_or_default();
    let worker = DetailWorker::start(client.clone(), "T4".into(), Duration::from_millis(50));
    for _ in 0..3 {
        let detail = worker
            .updates
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        assert_eq!(detail.task.id, "T4");
    }
    let started = Instant::now();
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(2));
    let log = calls();
    assert!(
        log.lines()
            .collect::<Vec<_>>()
            .chunks(2)
            .all(|pair| pair == ["start", "end"] || pair == ["start"]),
        "queries overlapped: {log}"
    );
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(calls(), log, "a dropped worker must not query again");

    // A hung query is cancelled instead of blocking whoever drops the worker.
    std::fs::write(temp.path().join("hang"), "").unwrap();
    std::fs::remove_file(temp.path().join("calls")).unwrap();
    let worker = DetailWorker::start(client, "T4".into(), Duration::from_millis(50));
    let deadline = Instant::now() + Duration::from_secs(5);
    while calls().is_empty() {
        assert!(Instant::now() < deadline, "query never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    let started = Instant::now();
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(2));
}
