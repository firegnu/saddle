mod common;
use saddle::dispatch::{Key, State, Status};
use serde_json::{Value, json};
use std::{
    fs,
    time::{Duration, Instant},
};

const INITIAL: &str = "0123456789abcdef0123456789abcdef";
const REVIEW: &str = "fedcba9876543210fedcba9876543210";

fn sha(c: char) -> String {
    c.to_string().repeat(64)
}
fn blob(c: char) -> Value {
    json!({"sha256": sha(c), "bytes": 10})
}
fn dispatch(project: &str, id: &str, kind: &str, parent: Option<&str>) -> Value {
    json!({"schema_version": 1, "dispatch_id": id, "project": project, "task": "T38",
           "kind": kind, "parent": parent, "created_at": "2026-09-29T08:00:00.000001+00:00",
           "source": "controller_declared"})
}
fn event(id: &str, at: &str, kind: &str, source: &str, data: Value) -> Value {
    json!({"schema_version": 1, "event_id": format!("{kind}-{at}"), "observed_at": at,
           "dispatch_id": id, "project": "p", "task": "T38", "kind": kind, "source": source,
           "association": {"source": "controller_declared", "context_found": true},
           "data": data})
}
/// One initial dispatch (route, decision, start, reply, rework) and one review dispatch.
fn sample(project: &str) -> Value {
    let start = json!({"cwd": "/w", "labels": {"role": "implementer", "model": "opus[1m]", "effort": "high"},
        "agent_parameters": {"model": "opus[1m]", "effort": null}, "agent_program": "claude",
        "prompt": blob('b'), "task_file": {"path": "/w/docs/任务/T38.md", "read_at": "2026-09-29T08:03:00+00:00", "content": blob('c')},
        "snapshot_semantics": "file_before_invocation_not_proof_of_read", "operation_id": "op-start",
        "requested_at": "2026-09-29T08:03:00+00:00", "recording_gaps": []});
    let mut started = start.clone();
    started["result"] = json!({"ok": true, "name": "saddle/dev-t38", "instance": "abc123abc123", "at": 1790000000.5});
    started["exit_code"] = json!(0);
    let route = json!({"operation_id": "op-route", "route_path": "/r/route.py", "route_sha256": sha('9'),
        "summary": blob('1'), "mode": "B", "request": blob('2'), "parsed_response": null,
        "response_semantics": "parsed_json_not_http_bytes", "recording_gaps": []});
    let mut routed = route.clone();
    routed["parsed_response"] = blob('3');
    routed["suggestion"] = blob('4');
    routed["exit_code"] = json!(0);
    let reply = json!({"target_name": "saddle/dev-t38", "operation_id": "op-reply",
        "requested_at": "2026-09-29T09:00:00+00:00", "recording_gaps": []});
    let mut replied = reply.clone();
    replied["result"] =
        json!({"name": "saddle/dev-t38", "instance": "abc123abc123", "at": 1790003600.25});
    replied["exit_code"] = json!(0);
    replied["reply"] = blob('5');
    replied["instance_check"] = json!("consistent");
    replied["causal_link"] = json!("not_proven");
    let send = json!({"target_name": "saddle/dev-t38", "text": blob('6'), "kind": "rework",
        "operation_id": "op-send", "requested_at": "2026-09-29T09:10:00+00:00", "recording_gaps": [],
        "result": {"ok": true, "pending": true}, "exit_code": 0, "delivery": "queued",
        "complete_input_known": false});
    let initial = vec![
        event(
            INITIAL,
            "2026-09-29T08:01:00+00:00",
            "jev.intent",
            "recorder_observation",
            route,
        ),
        event(
            INITIAL,
            "2026-09-29T08:01:05+00:00",
            "jev.route",
            "recorder_observation",
            routed,
        ),
        event(
            INITIAL,
            "2026-09-29T08:02:00+00:00",
            "controller.decide",
            "controller_statement",
            json!({"model": "opus[1m]", "effort": "high", "budget": "目标检查与相关回归", "reason": "采纳常规档"}),
        ),
        event(
            INITIAL,
            "2026-09-29T08:03:00+00:00",
            "corral.intent",
            "recorder_observation",
            start,
        ),
        event(
            INITIAL,
            "2026-09-29T08:03:02+00:00",
            "corral.start",
            "recorder_observation",
            started,
        ),
        event(
            INITIAL,
            "2026-09-29T09:00:00+00:00",
            "corral.intent",
            "recorder_observation",
            reply,
        ),
        event(
            INITIAL,
            "2026-09-29T09:00:01+00:00",
            "corral.reply",
            "recorder_observation",
            replied,
        ),
        event(
            INITIAL,
            "2026-09-29T09:10:01+00:00",
            "corral.send",
            "recorder_observation",
            send,
        ),
    ];
    let review = vec![event(
        REVIEW,
        "2026-09-29T10:00:00+00:00",
        "controller.note",
        "controller_statement",
        json!({"kind": "review", "content": blob('7')}),
    )];
    json!({
        "ls": [dispatch(project, INITIAL, "initial", None), dispatch(project, REVIEW, "review", Some(INITIAL))],
        "show": {
            INITIAL: {"dispatch": dispatch(project, INITIAL, "initial", None), "events": initial},
            REVIEW: {"dispatch": dispatch(project, REVIEW, "review", Some(INITIAL)), "events": review},
        },
        "cat": {
            sha('1'): "SUMMARY SENT TO JEV", sha('2'): "{\"request\": 1}",
            sha('3'): "{\"FULL\": \"parsed\"}", sha('4'): json!({
                // route.py shape(): an uncertain tier keeps its top candidate only as `level`.
                "ok": true, "model": "jev-1.13.0",
                "tier": {"verdict": null, "level": "重", "score": 1.4,
                         "probabilities": {"轻": 0.1, "常规": 0.4, "重": 0.5}, "confidence": 0.5},
                "cross_review": {"verdict": "不要", "a": 0.1},
                "impact": {"verdict": "改行为", "visible": 0.1},
                "usage": {"input_tokens": 1}
            }).to_string(),
            sha('5'): "IMPLEMENTER REPLY: tests pass", sha('6'): "REWORK TEXT",
            sha('7'): "CONTROLLER REVIEW TEXT", sha('b'): "PROMPT TEXT",
            sha('c'): "TASK FILE SNAPSHOT BODY",
        },
    })
}

struct Fake {
    dir: tempfile::TempDir,
    program: String,
    project: String,
}
fn fake(data: impl FnOnce(&str) -> Value) -> Fake {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    fs::create_dir(&project).unwrap();
    let project = project.canonicalize().unwrap().display().to_string();
    let program = common::script(dir.path(), "dlog", include_str!("fixtures/dlog.py"));
    fs::write(dir.path().join("dlog.json"), data(&project).to_string()).unwrap();
    Fake {
        dir,
        program,
        project,
    }
}
impl Fake {
    fn key(&self, task: &str, seq: u64) -> Key {
        Key {
            program: self.program.clone(),
            project: self.project.clone(),
            task: task.into(),
            seq,
        }
    }
    fn calls(&self) -> Vec<Vec<String>> {
        fs::read_to_string(self.dir.path().join("dlog-calls"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
}
fn wait(state: &mut State, ready: impl Fn(&State) -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !ready(state) {
        state.tick();
        assert!(Instant::now() < end, "{:?}", state.status);
        std::thread::yield_now();
    }
}
fn loaded(state: &mut State, key: Key) {
    state.sync(key);
    wait(state, |s| !matches!(s.status, Status::Loading));
}
fn summaries(state: &State) -> Vec<String> {
    state.entries().iter().map(|e| e.summary.clone()).collect()
}
fn open(state: &mut State, containing: &str) -> String {
    state.selected = summaries(state)
        .iter()
        .position(|s| s.contains(containing))
        .unwrap_or_else(|| panic!("{containing} in {:?}", summaries(state)));
    state.open();
    wait(state, |s| {
        s.reading.as_ref().is_some_and(|r| r.text != "Loading…")
    });
    state.reading.as_ref().unwrap().text.clone()
}

#[test]
fn records_of_the_explicit_task_list_each_step_once_and_open_full_text() {
    let f = fake(sample);
    let mut state = State::default();
    loaded(&mut state, f.key("T38", 1));
    assert!(matches!(state.status, Status::Loaded), "{:?}", state.status);
    assert_eq!(
        f.calls()[0],
        ["ls", "--project", &f.project, "--task", "T38"]
    );
    // Headings keep the initial dispatch and its review apart, in recorded order.
    let headings: Vec<_> = state.records.iter().map(|r| r.heading.clone()).collect();
    assert!(headings[0].starts_with("Initial dispatch"), "{headings:?}");
    assert!(
        headings[1].contains("Review") && headings[1].contains("01234567"),
        "{headings:?}"
    );
    // An intent and its result are one step, not two dispatches.
    let rows = summaries(&state);
    assert_eq!(rows.len(), 6, "{rows:?}");
    for expected in [
        "JEV",
        "jev-1.13.0",
        "tier uncertain",
        "cross review 不要",
        "impact 改行为",
    ] {
        assert!(rows[0].contains(expected), "{expected}: {rows:?}");
    }
    assert!(
        !rows[0].contains('重'),
        "level is not the verdict: {rows:?}"
    );
    assert!(
        rows[1].contains("Controller decision")
            && rows[1].contains("opus[1m]")
            && rows[1].contains("high"),
        "{rows:?}"
    );
    assert!(
        rows[2].contains("Start")
            && rows[2].contains("saddle/dev-t38")
            && rows[2].contains("08:03"),
        "{rows:?}"
    );
    assert!(
        rows[3].contains("Reply") && rows[3].contains("consistent"),
        "{rows:?}"
    );
    assert!(
        rows[4].contains("Rework") && rows[4].contains("queued"),
        "{rows:?}"
    );
    assert!(rows[5].contains("review"), "{rows:?}");

    let start = open(&mut state, "Start");
    for expected in [
        "TASK FILE SNAPSHOT BODY",
        "PROMPT TEXT",
        "not proof",
        "Links",
        "not given",
        "abc123abc123",
        "1790000000.5",
    ] {
        assert!(start.contains(expected), "{expected}: {start}");
    }
    assert!(!start.contains("at not returned"), "{start}");
    state.back();
    assert!(state.reading.is_none());
    assert_eq!(state.selected, 2, "Esc returns to the same entry");

    let jev = open(&mut state, "JEV");
    for expected in [
        "SUMMARY SENT TO JEV",
        "{\"request\": 1}",
        "\"FULL\"",
        "\"tier\"",
    ] {
        assert!(jev.contains(expected), "{expected}: {jev}");
    }
    state.back();
    let decision = open(&mut state, "Controller decision");
    assert!(
        decision.contains("采纳常规档") && decision.contains("目标检查与相关回归"),
        "{decision}"
    );
    assert!(decision.contains("controller statement"), "{decision}");
    state.back();
    let reply = open(&mut state, "Reply");
    assert!(
        reply.contains("IMPLEMENTER REPLY") && reply.contains("not proven"),
        "{reply}"
    );
    assert!(reply.contains("implementer's report"), "{reply}");
    assert!(
        reply.contains("1790003600.25") && !reply.contains("at not returned"),
        "{reply}"
    );
    state.back();
    let rework = open(&mut state, "Rework");
    assert!(
        rework.contains("REWORK TEXT") && rework.contains("not proof of delivery"),
        "{rework}"
    );
    state.back();
    assert!(open(&mut state, "Controller review note").contains("CONTROLLER REVIEW TEXT"));
    // Reading only uses the public read commands.
    assert!(
        f.calls()
            .iter()
            .all(|c| ["ls", "show", "cat"].contains(&c[0].as_str()))
    );
}

#[test]
fn missing_recorder_empty_history_failures_and_unsupported_data_stay_distinct() {
    let mut state = State::default();
    let f = fake(|_| json!({"ls": []}));
    let mut missing = f.key("T38", 1);
    missing.program = f.dir.path().join("not-installed").display().to_string();
    loaded(&mut state, missing);
    assert!(
        matches!(state.status, Status::Missing(_)),
        "{:?}",
        state.status
    );

    loaded(&mut state, f.key("T38", 2));
    assert!(matches!(state.status, Status::Loaded) && state.records.is_empty());

    let f = fake(|_| json!({"ls_error": "dlog: PermissionError: denied\n"}));
    loaded(&mut state, f.key("T38", 3));
    assert!(
        matches!(&state.status, Status::Failed(e) if e.contains("PermissionError")),
        "{:?}",
        state.status
    );

    for data in [json!({"ls": {"not": "a list"}}), json!({"ls": "not json"})] {
        let f = fake(|_| data);
        loaded(&mut state, f.key("T38", 4));
        assert!(
            matches!(state.status, Status::Unsupported(_)),
            "{:?}",
            state.status
        );
    }
    let f = fake(|p| {
        let mut record = dispatch(p, INITIAL, "initial", None);
        record["schema_version"] = json!(2);
        json!({"ls": [record]})
    });
    loaded(&mut state, f.key("T38", 5));
    assert!(
        matches!(state.status, Status::Unsupported(_)),
        "{:?}",
        state.status
    );
}

#[test]
fn unrecorded_items_unknown_results_and_unreadable_records_are_shown_as_such() {
    let f = fake(|p| {
        let mut data = sample(p);
        // The initial dispatch cannot be read; the review one still shows.
        data["show"].as_object_mut().unwrap().remove(INITIAL);
        let events = data["show"][REVIEW]["events"].as_array_mut().unwrap();
        events.push(event(REVIEW, "2026-09-29T10:01:00+00:00", "corral.intent", "recorder_observation",
            json!({"action": "start", "operation_id": "op-lost", "cwd": "/w", "labels": {},
                   "agent_parameters": {"model": null, "effort": null}, "prompt": null,
                   "task_file": null, "recording_gaps": [{"stage": "blob", "error_type": "OSError"}]})));
        data
    });
    let mut state = State::default();
    loaded(&mut state, f.key("T38", 1));
    assert!(matches!(state.status, Status::Loaded));
    assert!(
        state.records[0]
            .error
            .as_deref()
            .is_some_and(|e| e.contains("FileNotFoundError")),
        "{:?}",
        state.records[0].error
    );
    let rows = summaries(&state);
    assert!(
        rows.iter().any(|r| r.contains("result unknown")),
        "{rows:?}"
    );
    let lost = open(&mut state, "result unknown");
    for expected in ["Not recorded", "OSError", "not given"] {
        assert!(lost.contains(expected), "{expected}: {lost}");
    }
}

#[test]
fn records_of_another_task_or_project_and_older_openings_are_not_shown() {
    let f = fake(|p| {
        let mut other = dispatch(p, REVIEW, "initial", None);
        other["task"] = json!("T37");
        let mut elsewhere = dispatch("/elsewhere", REVIEW, "initial", None);
        elsewhere["dispatch_id"] = json!("11111111111111111111111111111111");
        json!({"ls": [other, elsewhere], "show": {}, "cat": {}})
    });
    let mut state = State::default();
    loaded(&mut state, f.key("T38", 1));
    assert!(state.records.is_empty());
    assert!(state.notice.contains("2"), "{}", state.notice);
    assert!(
        f.calls().iter().all(|c| c[0] == "ls"),
        "no show for foreign records"
    );

    // A newer opening replaces the reading and its results.
    let f = fake(sample);
    state.sync(f.key("T38", 2));
    wait(&mut state, |s| !s.records.is_empty());
    open(&mut state, "Start");
    state.sync(f.key("T38", 3));
    assert!(state.reading.is_none());
    assert!(matches!(state.status, Status::Loading));
}
