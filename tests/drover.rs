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
printf '%s\n' '{"mode":{"loop":false,"gate":true},"paused":false,"current":null,"awaiting":{"id":"T5","title":"Done","body":"","key":"Done","start":"aaaa","main":"bbbb","t0":100.0,"status":"done","location":"awaiting","end":"cccc","t1":160},"pending":[],"history":[{"id":"T4","title":"Old","body":"","t0":true,"status":"done"}]}'
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
fn show_json_carries_the_manual_target_and_saved_record_without_requiring_them() {
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(temp.path(), "drover", "#!/bin/sh\ncat response\n");
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let cancel = AtomicBool::new(false);
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    std::fs::write(temp.path().join("response"), value.to_string()).unwrap();
    let old = client.show("T4", &cancel).unwrap();
    assert_eq!(old.manual_completion, None, "an older drover has no target");
    assert_eq!(old.task.completion_record, None, "no record is back-filled");

    value["manual_completion"] =
        serde_json::json!({"target_token": "opaque/+=token", "unavailable_reason": null});
    value["task"]["location"] = "awaiting".into();
    value["task"]["completion_record"] = serde_json::json!({
        "method": "manual", "reason": "调研成果已验收", "confirmed_at": 1790000000.0,
        "completion": {"scope": "current_repository", "rows": [
            {"id": "branches_merged", "state": "unmet", "reason": "branch_not_merged", "why": "t33 未合入"}
        ], "unavailable_reason": null},
        "last_check": {},
        "workspace": {"state": "dirty", "tracked_dirty": true}
    });
    std::fs::write(temp.path().join("response"), value.to_string()).unwrap();
    let detail = client.show("T4", &cancel).unwrap();
    let target = detail.manual_completion.unwrap();
    assert_eq!(target.target_token.as_deref(), Some("opaque/+=token"));
    let record = detail.task.completion_record.unwrap();
    assert_eq!(
        (record.method.as_str(), record.reason.as_str()),
        ("manual", "调研成果已验收")
    );
    assert_eq!(record.completion.unwrap().rows.unwrap()[0].state, "unmet");
    assert_eq!(record.last_check, None, "an unreadable sample is left out");
    assert!(record.workspace.unwrap().tracked_dirty);
}

#[test]
fn manual_completion_sends_the_token_verbatim_and_reports_public_codes() {
    use saddle::drover::{ManualError, Operation};
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
    let project = temp.path().display().to_string();
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let respond = |body: &str, code: i32| {
        std::fs::write(temp.path().join("response"), body).unwrap();
        std::fs::write(temp.path().join("code"), code.to_string()).unwrap();
    };
    let cancel = AtomicBool::new(false);
    let operation = Operation::CompleteManually {
        project,
        id: "T4".into(),
        token: "tok $(touch bad) '\"".into(),
        reason: "  接受 main 未前进  ".into(),
    };
    respond(
        r#"{"schema_version":1,"ok":true,"task_id":"T4","state":"awaiting_release","completion_record":{"method":"manual","reason":"接受 main 未前进","confirmed_at":1790000000.0,"completion":{"scope":"current_repository","rows":[{"id":"main_advanced","state":"unmet","reason":"main_not_advanced","why":"main 未前进"}],"unavailable_reason":null},"last_check":{},"workspace":{"state":"clean","tracked_dirty":false}}}"#,
        0,
    );
    let text = client.execute(&operation, &cancel).unwrap();
    assert!(
        text.contains("T4")
            && text.contains("Awaiting release")
            && text.contains("接受 main 未前进"),
        "{text}"
    );
    assert!(!temp.path().join("bad").exists());
    let calls = std::fs::read_to_string(temp.path().join("calls")).unwrap();
    assert_eq!(
        calls,
        "[complete-manually][T4][--target-token][tok $(touch bad) '\"][--reason][接受 main 未前进][--json]\n"
    );

    for (body, code, expected) in [
        (
            r#"{"schema_version":1,"ok":false,"error":{"code":"target_changed","why":"运行已变化"}}"#,
            3,
            "target_changed",
        ),
        (
            r#"{"schema_version":1,"ok":false,"error":{"code":"state_busy","why":"忙"}}"#,
            4,
            "state_busy",
        ),
        (
            r#"{"schema_version":1,"ok":false,"error":{"code":"invalid_arguments","why":"空原因"}}"#,
            2,
            "invalid_arguments",
        ),
    ] {
        respond(body, code);
        let error = client.execute(&operation, &cancel).unwrap_err();
        let manual = error.downcast_ref::<ManualError>().expect("a public code");
        assert_eq!(manual.code, expected);
    }
    // An older drover without the command, and answers that do not confirm this target.
    for (body, code, expected) in [
        (
            "usage: drover [add|go|next|...]\nunknown command: complete-manually",
            2,
            "does not support",
        ),
        (r#"{"schema_version":2,"ok":true}"#, 0, "schema_version"),
        (
            r#"{"schema_version":1,"ok":true,"task_id":"T5","state":"awaiting_release"}"#,
            0,
            "T5",
        ),
        (
            r#"{"schema_version":1,"ok":true,"task_id":"T4","state":"done"}"#,
            0,
            "done",
        ),
        (
            r#"{"schema_version":1,"ok":true,"task_id":"T4","state":"awaiting_release"}"#,
            1,
            "exit",
        ),
    ] {
        respond(body, code);
        let error = client.execute(&operation, &cancel).unwrap_err();
        assert!(error.downcast_ref::<ManualError>().is_none());
        assert!(format!("{error:#}").contains(expected), "{error:#}");
    }
    // The same project spelled through a symlink (macOS temp dirs sit under /var) is the same.
    let canonical = Operation::CompleteManually {
        project: temp.path().canonicalize().unwrap().display().to_string(),
        id: "T4".into(),
        token: "tok".into(),
        reason: "ok".into(),
    };
    respond(
        r#"{"schema_version":1,"ok":true,"task_id":"T4","state":"awaiting_release"}"#,
        0,
    );
    client.execute(&canonical, &cancel).unwrap();
    // A target from another project is never sent to this one.
    std::fs::remove_file(temp.path().join("calls")).unwrap();
    let elsewhere = Operation::CompleteManually {
        project: "/tmp/another-project".into(),
        id: "T4".into(),
        token: "tok".into(),
        reason: "ok".into(),
    };
    assert!(client.execute(&elsewhere, &cancel).is_err());
    assert!(!temp.path().join("calls").exists(), "nothing ran");
}

#[test]
fn return_to_pending_uses_one_bound_public_write_and_validates_the_result() {
    use saddle::drover::{Operation, ReturnError};
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
    let op = Operation::ReturnToPending {
        project: temp.path().display().to_string(),
        id: "T4".into(),
        token: "r1:$(touch bad) '".into(),
        reason: "  误派发  ".into(),
    };
    let success = serde_json::json!({
        "schema_version":1, "ok":true, "task_id":"T4", "state":"pending", "paused":true,
        "return_record":{"reason":"误派发", "dispatched_at":100, "returned_at":200, "work_stopped":true}
    });
    let run = |value: &serde_json::Value, code: i32| {
        std::fs::write(temp.path().join("response"), value.to_string()).unwrap();
        std::fs::write(temp.path().join("code"), code.to_string()).unwrap();
        client.execute(&op, &AtomicBool::new(false))
    };
    let text = run(&success, 0).unwrap();
    assert!(
        text.contains("T4 returned to Pending")
            && text.contains("Queue paused")
            && text.contains("误派发")
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("calls")).unwrap(),
        "[return-to-pending][T4][--target-token][r1:$(touch bad) '][--reason][误派发][--work-stopped][--json]\n"
    );
    assert!(!temp.path().join("bad").exists());
    for code in [
        "target_changed",
        "state_busy",
        "invalid_arguments",
        "write_failed",
    ] {
        let err = run(
            &serde_json::json!({"schema_version":1,"ok":false,"error":{"code":code,"why":"拒绝"}}),
            3,
        )
        .unwrap_err();
        assert_eq!(err.downcast_ref::<ReturnError>().unwrap().code, code);
    }
    for (field, value) in [
        ("task_id", serde_json::json!("T5")),
        ("state", serde_json::json!("done")),
        ("paused", serde_json::json!(false)),
        ("schema_version", serde_json::json!(2)),
        ("return_record", serde_json::Value::Null),
    ] {
        let mut bad = success.clone();
        bad[field] = value;
        assert!(run(&bad, 0).is_err(), "{field}");
    }
    assert!(run(&success, 1).is_err());
    let mut unconfirmed = success.clone();
    unconfirmed["return_record"]["work_stopped"] = false.into();
    assert!(run(&unconfirmed, 0).is_err());
    let Operation::ReturnToPending {
        id, token, reason, ..
    } = op
    else {
        unreachable!()
    };
    std::fs::remove_file(temp.path().join("calls")).unwrap();
    assert!(
        client
            .execute(
                &Operation::ReturnToPending {
                    project: "/another/project".into(),
                    id,
                    token,
                    reason
                },
                &AtomicBool::new(false)
            )
            .is_err()
    );
    assert!(!temp.path().join("calls").exists());
}
