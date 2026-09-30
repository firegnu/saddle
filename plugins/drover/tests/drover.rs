mod common;
use saddle_drover_plugin::drover::Client;
#[test]
fn project_registry_preserves_order_deduplicates_and_reports_read_errors() {
    use saddle_drover_plugin::drover::registered_projects;
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
printf '%s\n' '{"schema_version":2,"ok":true,"project":"/synthetic","mode":{"loop":true,"gate":true},"paused":true,"current":{"id":"T1","title":"Doing","body":"current body"},"awaiting":null,"pending":[{"id":null,"title":"中文任务","body":"task detail"}],"history":[{"id":"T0","title":"Past","status":"dropped","reason":"test"}]}'
"#,
    );
    std::fs::write(temp.path().join("project-marker"), "").unwrap();
    let snapshot = Client {
        program,
        cwd: temp.path().into(),
    }
    .snapshot()
    .unwrap();
    assert!(snapshot.paused);
    assert_eq!(snapshot.current.unwrap().id.as_deref(), Some("T1"));
    assert_eq!(snapshot.pending[0].title, "中文任务");
    assert_eq!(snapshot.pending[0].body, "task detail");
    assert_eq!(snapshot.history[0].reason.as_deref(), Some("test"));
}

#[test]
fn operations_use_literal_public_arguments_and_surface_failure_feedback() {
    use saddle_drover_plugin::drover::Operation;
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
    ] {
        assert_eq!(client.execute(&op, &cancel).unwrap(), expected);
    }
}

#[test]
fn pending_edit_and_move_check_public_data_and_pass_literal_arguments() {
    use saddle_drover_plugin::drover::Operation;
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
    use saddle_drover_plugin::drover::Operation;
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
    use saddle_drover_plugin::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let client = Client {
        program: common::script(temp.path(), "drover", include_str!("fixtures/drover.py")),
        cwd: temp.path().into(),
    };
    let state = serde_json::json!({"schema_version":2,"ok":true,"project":"/synthetic","mode":{}, "paused":false, "current":null, "awaiting":null, "history":[], "pending":[
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
    use saddle_drover_plugin::drover::PendingLoad;
    use std::time::Duration;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
[ "$1" = list ] && [ "$2" = --json ] || exit 99
if [ -f fail ]; then echo 'synthetic unreadable queue' >&2; exit 3; fi
name=$(basename "$PWD")
printf '{"schema_version":2,"ok":true,"project":"/synthetic","mode":{},"paused":false,"current":{"title":"not pending"},"awaiting":null,"pending":[{"id":"T1","title":"%s 中文待办","body":"body"}],"history":[{"title":"old"}]}\n' "$name"
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
    assert_eq!(detail.task.id.as_deref(), Some("T4"));
    assert_eq!(detail.task.status.as_deref(), Some("running"));
    assert_eq!(detail.task.body, "原始正文\n第二行");
    assert_eq!(detail.evidence.git.state, "unavailable");
    assert_eq!(detail.evidence.last_check.state, "stale");

    let mut newer = good.clone();
    newer["schema_version"] = 3.into();
    respond(&newer.to_string(), 0);
    let error = format!("{:#}", client.show("T4", &cancel).unwrap_err());
    assert!(error.contains("schema_version"), "{error}");

    respond(
        r#"{"schema_version":2,"ok":false,"observed_at":1,"error":{"code":"task_not_found","why":"没有该任务"}}"#,
        2,
    );
    let error = format!("{:#}", client.show("T4", &cancel).unwrap_err());
    assert!(
        error.contains("task_not_found") && error.contains("没有该任务"),
        "{error}"
    );

    let mut partial = good.clone();
    partial.as_object_mut().unwrap().remove("evidence");
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
    use saddle_drover_plugin::drover::DetailWorker;
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
        assert_eq!(detail.task.id.as_deref(), Some("T4"));
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

#[test]
fn notification_preference_uses_the_public_json_contract_and_reports_failures() {
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    // Answers from the file named after the action; logs every call and its cwd.
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
root=$(dirname "$0")
printf '%s %s %s\n' "$1" "$2" "$3" >> "$root/calls"
[ "$1" = notifications ] && [ "$3" = --json ] || exit 99
cat "$root/answer"
exit $(cat "$root/code")
"#,
    );
    let client = Client {
        program,
        cwd: temp.path().join("not-a-project"),
    };
    let answer = |text: &str, code: i32| {
        std::fs::write(temp.path().join("answer"), text).unwrap();
        std::fs::write(temp.path().join("code"), code.to_string()).unwrap();
    };
    let cancel = AtomicBool::new(false);
    answer(
        r#"{"schema_version":1,"ok":true,"scope":"user","system_enabled":false,"revision":3,"application":"next_notification_check"}"#,
        0,
    );
    let preference = client.notifications(None, &cancel).unwrap();
    assert!(!preference.system_enabled);
    assert_eq!(preference.revision, 3);
    client.notifications(Some(true), &cancel).unwrap();
    client.notifications(Some(false), &cancel).unwrap();
    assert_eq!(
        std::fs::read_to_string(temp.path().join("calls")).unwrap(),
        "notifications status --json\nnotifications on --json\nnotifications off --json\n"
    );
    let error = |client: &Client| {
        format!(
            "{:#}",
            client.notifications(Some(false), &cancel).unwrap_err()
        )
    };
    // Documented codes are named; the message is kept for people.
    answer(
        r#"{"schema_version":1,"ok":false,"error":{"code":"preferences_invalid","message":"bad file"}}"#,
        2,
    );
    let text = error(&client);
    assert!(
        text.contains("preferences_invalid") && text.contains("bad file"),
        "{text}"
    );
    // Unknown codes still fail, never as success.
    answer(
        r#"{"schema_version":1,"ok":false,"error":{"code":"brand_new","message":"later"}}"#,
        2,
    );
    let text = error(&client);
    assert!(
        text.contains("brand_new") && text.contains("later"),
        "{text}"
    );
    // An older drover without the command answers in text.
    answer("usage: drover add|list|next ...", 2);
    let text = error(&client);
    assert!(
        text.contains("does not support task notifications"),
        "{text}"
    );
    // A success object with a failing exit, or wrong field types, is not trusted.
    answer(
        r#"{"schema_version":1,"ok":true,"scope":"user","system_enabled":false,"revision":3,"application":"next_notification_check"}"#,
        2,
    );
    assert!(client.notifications(None, &cancel).is_err());
    answer(
        r#"{"schema_version":1,"ok":true,"scope":"user","system_enabled":"no","revision":3,"application":"next_notification_check"}"#,
        0,
    );
    assert!(client.notifications(None, &cancel).is_err());
    answer(
        r#"{"schema_version":2,"ok":true,"scope":"user","system_enabled":false,"revision":3}"#,
        0,
    );
    assert!(client.notifications(None, &cancel).is_err());
}

#[test]
fn list_json_keeps_the_awaiting_start_identity_fields() {
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
printf '%s\n' '{"schema_version":2,"ok":true,"project":"/synthetic","mode":{"loop":false,"gate":true},"paused":false,"current":null,"awaiting":{"id":"T5","title":"Done","body":"","key":"Done","start":"aaaa","main":"bbbb","t0":100.0,"status":"done","location":"awaiting","end":"cccc","t1":160},"pending":[],"history":[{"id":"T4","title":"Old","body":"","t0":true,"status":"done"}]}'
"#,
    );
    let snapshot = Client {
        program,
        cwd: temp.path().into(),
    }
    .snapshot()
    .unwrap();
    let awaiting = snapshot.awaiting.unwrap();
    assert_eq!(awaiting.start.as_deref(), Some("aaaa"));
    assert_eq!(awaiting.main.as_deref(), Some("bbbb"));
    assert_eq!(awaiting.t0, Some(serde_json::json!(100.0)));
    // A malformed start time elsewhere does not break the snapshot.
    assert_eq!(snapshot.history[0].t0, Some(serde_json::json!(true)));
}

#[test]
fn dispatch_selected_sends_the_shown_target_once_and_reports_delivery_and_record_apart() {
    use saddle_drover_plugin::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
for arg in "$@"; do printf '[%s]' "$arg" >> calls; done
echo >> calls
cat response
exit $(cat code)
"#,
    );
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let op = Operation::DispatchPending {
        project: temp.path().display().to_string(),
        pos: 2,
        token: "d1:$(touch bad) '".into(),
    };
    assert_eq!(
        op.args(),
        [
            "dispatch-pending",
            "--pos",
            "2",
            "--target-token",
            "d1:$(touch bad) '",
            "--json"
        ]
    );
    let run = |value: &serde_json::Value, code: i32| {
        std::fs::write(temp.path().join("response"), value.to_string()).unwrap();
        std::fs::write(temp.path().join("code"), code.to_string()).unwrap();
        client.execute(&op, &AtomicBool::new(false))
    };
    let answer = |ok: bool,
                  task: serde_json::Value,
                  state: serde_json::Value,
                  delivery: &str,
                  exit: serde_json::Value,
                  record: &str| {
        serde_json::json!({
            "schema_version":2, "ok":ok, "task_id":task, "run_id":"run-2", "state":state,
            "delivery":{"status":delivery, "attempted":!exit.is_null(), "corral_exit_code":exit,
                        "confirmed":delivery == "confirmed", "merged_with_draft":false},
            "record":{"status":record}, "manual_text":null
        })
    };
    let confirmed = answer(
        true,
        "T2".into(),
        "running".into(),
        "confirmed",
        0.into(),
        "recorded",
    );
    let text = run(&confirmed, 0).unwrap();
    assert!(text.contains("T2") && text.contains("Delivered"), "{text}");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("calls")).unwrap(),
        "[dispatch-pending][--pos][2][--target-token][d1:$(touch bad) '][--json]\n",
        "one call with the shown position and token, verbatim"
    );
    assert!(!temp.path().join("bad").exists());

    // Manual mode: recorded as started, but nothing was sent; the text is for pasting.
    let mut manual = answer(
        true,
        "T2".into(),
        "running".into(),
        "not_sent",
        serde_json::Value::Null,
        "recorded",
    );
    manual["manual_text"] = "请做 T2：调研\n正文".into();
    let text = run(&manual, 0).unwrap();
    assert!(
        text.contains("not sent")
            && text.contains("请做 T2：调研\n正文")
            && !text.contains("Delivered"),
        "{text}"
    );

    // Everything else is shown as a failure, never as delivered, from its structured fields.
    let mut draft = confirmed.clone();
    draft["delivery"]["merged_with_draft"] = true.into();
    let mut unconfirmed = answer(
        false,
        "T2".into(),
        "running".into(),
        "unconfirmed",
        3.into(),
        "recorded",
    );
    unconfirmed["error"] = serde_json::json!({"code":"delivery_unconfirmed","why":"未确认"});
    let mut rejected = answer(
        false,
        "T2".into(),
        "pending".into(),
        "rejected",
        7.into(),
        "not_attempted",
    );
    rejected["error"] = serde_json::json!({"code":"send_rejected","why":"主控忙"});
    let mut unknown = answer(
        false,
        "T2".into(),
        "pending".into(),
        "unknown",
        serde_json::Value::Null,
        "not_attempted",
    );
    unknown["error"] = serde_json::json!({"code":"delivery_unknown","why":"超时"});
    let mut changed = answer(
        false,
        serde_json::Value::Null,
        serde_json::Value::Null,
        "not_attempted",
        serde_json::Value::Null,
        "not_attempted",
    );
    changed["error"] = serde_json::json!({"code":"target_changed","why":"队列变了"});
    let mut unwritten = answer(
        false,
        "T2".into(),
        "unknown".into(),
        "confirmed",
        0.into(),
        "unknown",
    );
    unwritten["error"] = serde_json::json!({"code":"write_failed","why":"磁盘满"});
    let contradictory = answer(
        true,
        "T2".into(),
        "running".into(),
        "unconfirmed",
        0.into(),
        "recorded",
    );
    let mut schema = confirmed.clone();
    schema["schema_version"] = 3.into();
    for (value, code, words) in [
        (&draft, 0, vec!["draft", "Delivered"]),
        (
            &unconfirmed,
            8,
            vec!["not confirmed", "delivery_unconfirmed", "recorded"],
        ),
        (
            &rejected,
            8,
            vec!["rejected", "7", "send_rejected", "主控忙"],
        ),
        (&unknown, 8, vec!["unknown", "delivery_unknown", "超时"]),
        (&changed, 3, vec!["target_changed", "队列变了", "Refresh"]),
        (&unwritten, 2, vec!["Delivered", "write_failed", "unknown"]),
        (&contradictory, 0, vec!["not confirmed"]),
        (&confirmed, 1, vec!["Delivered"]),
        (&schema, 0, vec!["schema_version"]),
    ] {
        let error = format!("{:#}", run(value, code).unwrap_err());
        for word in words {
            assert!(error.contains(word), "{word}: {error}");
        }
        assert!(
            error.contains("not retried") || error.contains("schema_version"),
            "{error}"
        );
        if value != &draft && value != &unwritten && value != &confirmed {
            assert!(!error.contains("Delivered"), "{error}");
        }
    }
    // An older drover without the command: no JSON, nothing else is sent.
    std::fs::write(temp.path().join("response"), "未知命令 dispatch-pending").unwrap();
    std::fs::write(temp.path().join("code"), "2").unwrap();
    let error = format!(
        "{:#}",
        client.execute(&op, &AtomicBool::new(false)).unwrap_err()
    );
    assert!(error.contains("does not support"), "{error}");

    std::fs::remove_file(temp.path().join("calls")).unwrap();
    let elsewhere = Operation::DispatchPending {
        project: "/another/project".into(),
        pos: 2,
        token: "d1:aa".into(),
    };
    assert!(client.execute(&elsewhere, &AtomicBool::new(false)).is_err());
    assert!(
        !temp.path().join("calls").exists(),
        "another project's target is never sent"
    );
}

#[test]
fn dispatch_selected_feedback_claims_only_what_the_answer_states() {
    use saddle_drover_plugin::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
echo call >> calls
[ -f slow ] && sleep 5
cat response
exit $(cat code)
"#,
    );
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let op = Operation::DispatchPending {
        project: temp.path().display().to_string(),
        pos: 2,
        token: "d1:two".into(),
    };
    let run = |value: serde_json::Value, code: i32| {
        std::fs::write(temp.path().join("response"), value.to_string()).unwrap();
        std::fs::write(temp.path().join("code"), code.to_string()).unwrap();
        format!(
            "{:#}",
            client.execute(&op, &AtomicBool::new(false)).unwrap_err()
        )
    };
    // A refused target says nothing about where the task is now.
    let error = run(
        serde_json::json!({
            "schema_version":2, "ok":false, "task_id":null, "state":null,
            "delivery":{"status":"not_attempted", "attempted":false, "corral_exit_code":null,
                        "confirmed":null, "merged_with_draft":null},
            "record":{"status":"not_attempted"}, "manual_text":null,
            "error":{"code":"target_changed", "why":"队列变了"}
        }),
        3,
    );
    assert!(!error.contains("Pending"), "{error}");
    assert!(
        error.contains("not recorded") && error.contains("Refresh"),
        "{error}"
    );
    // A draft merged into an unconfirmed delivery is not a delivery.
    let error = run(
        serde_json::json!({
            "schema_version":2, "ok":false, "task_id":"T2", "state":"current",
            "delivery":{"status":"unconfirmed", "attempted":true, "corral_exit_code":3,
                        "confirmed":false, "merged_with_draft":true},
            "record":{"status":"recorded"}, "manual_text":null,
            "error":{"code":"delivery_unconfirmed", "why":"未确认"}
        }),
        8,
    );
    assert!(
        error.contains("draft") && error.contains("not confirmed") && !error.contains("Delivered"),
        "{error}"
    );
    // A run that ends without an answer may still have sent the task.
    std::fs::write(temp.path().join("slow"), "").unwrap();
    std::fs::remove_file(temp.path().join("calls")).unwrap();
    let error = format!(
        "{:#}",
        client.execute(&op, &AtomicBool::new(true)).unwrap_err()
    );
    for words in ["cannot confirm", "not retried", "main agent"] {
        assert!(error.contains(words), "{words}: {error}");
    }
    let calls = std::fs::read_to_string(temp.path().join("calls")).unwrap_or_default();
    assert!(calls.lines().count() <= 1, "no retry: {calls}");
}
