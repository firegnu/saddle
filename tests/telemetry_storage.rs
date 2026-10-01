use saddle::telemetry::*;
use serde_json::{Value, json};

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("telemetry"));
    store
        .set_recording(
            None,
            SettingInput {
                schema_version: 1,
                enabled: true,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
    store
        .create_trace(
            serde_json::from_value(
                json!({"schema_version":1,"trace_id":"t","origin":"ad_hoc","label":"synthetic"}),
            )
            .unwrap(),
        )
        .unwrap();
    store.create_dispatch(serde_json::from_value(json!({"schema_version":1,"trace_id":"t","dispatch_id":"d","kind":"implementation"})).unwrap()).unwrap();
    (dir, store)
}

fn operation(kind: &str) -> OperationInput {
    OperationInput {
        schema_version: 1,
        trace_id: "t".into(),
        dispatch_id: "d".into(),
        kind: kind.into(),
        producer: "saddle.synthetic".into(),
        basis_event_ids: vec![],
        decision_event_id: None,
        previous_brief_event_id: None,
    }
}

fn begin() -> Observation {
    Observation {
        observed_at: Value::Null,
        payload: json!({"target_name":"synthetic","send_kind":"initial","gaps":[]}),
        bodies: vec![],
    }
}

fn end() -> Observation {
    Observation {
        observed_at: Value::Null,
        payload: json!({"outcome":{"kind":"exited","exit_code":0},"confirmed":null,"pending":null,"merged_with_draft":null,"name":null,"instance":null,"at":null,"gaps":[{"role":"confirmed","reason":"not_available"},{"role":"pending","reason":"not_available"},{"role":"merged_with_draft","reason":"not_available"},{"role":"name","reason":"not_available"},{"role":"instance","reason":"not_available"},{"role":"at","reason":"not_available"}]}),
        bodies: vec![],
    }
}

#[test]
fn operation_snapshot_is_atomic_and_old_generation_never_revives() {
    let (dir, store) = store();
    let brief = dir.path().join("brief");
    std::fs::write(&brief, b"original brief").unwrap();
    let capture = store
        .prepare_operation(operation("agent.send"), begin(), Some(brief.clone()))
        .unwrap();
    std::fs::write(&brief, b"changed after capture").unwrap();
    store.record_begin(&capture).unwrap();
    let events = store.events(&EventQuery::default()).unwrap();
    let events = events["events"].as_array().unwrap();
    let snapshot = events
        .iter()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap();
    let begin_event = events
        .iter()
        .find(|e| e["kind"] == "agent.send.begin")
        .unwrap();
    assert_eq!(
        begin_event["links"][0]["target_event_id"],
        snapshot["event_id"]
    );
    assert_eq!(
        store
            .body(snapshot["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"original brief"
    );
    store
        .set_recording(
            None,
            SettingInput {
                schema_version: 1,
                enabled: true,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
    store.record_end(&capture, end()).unwrap();
    let stale = store
        .prepare_operation(operation("agent.send"), begin(), Some(brief))
        .unwrap();
    store.record_begin(&stale).unwrap();
    for enabled in [false, true] {
        store
            .set_recording(
                None,
                SettingInput {
                    schema_version: 1,
                    enabled,
                    actor: "synthetic".into(),
                },
            )
            .unwrap();
    }
    assert_eq!(
        store.record_end(&stale, end()).unwrap_err().status,
        "disabled"
    );
    let shown = store.show("t").unwrap();
    let operations = shown["record"]["operations"].as_array().unwrap();
    assert_eq!(
        operations
            .iter()
            .filter(|o| o["end_missing"] == true)
            .count(),
        1
    );
}

#[test]
fn failed_begin_rolls_back_the_batch_and_end_uses_the_original_snapshot() {
    let (dir, store) = store();
    let brief = dir.path().join("brief");
    std::fs::write(&brief, b"before execution").unwrap();
    let capture = store
        .prepare_operation(operation("agent.send"), begin(), Some(brief.clone()))
        .unwrap();
    let fault = rusqlite::Connection::open(dir.path().join("telemetry/telemetry.sqlite3")).unwrap();
    fault.execute_batch("CREATE TRIGGER fail_begin BEFORE INSERT ON events WHEN NEW.phase='begin' BEGIN SELECT RAISE(ABORT,'synthetic write failure'); END;").unwrap();
    assert_eq!(
        store.record_begin(&capture).unwrap_err().status,
        "unavailable"
    );
    assert_eq!(
        store.events(&EventQuery::default()).unwrap()["events"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(store.show(capture.operation_id()).is_err());
    fault.execute_batch("DROP TRIGGER fail_begin;").unwrap();
    std::fs::write(brief, b"after execution").unwrap();
    store.record_end(&capture, end()).unwrap();
    let snapshot = store.events(&EventQuery::default()).unwrap()["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap()
        .clone();
    assert_eq!(snapshot["payload"]["publication_phase"], "end");
    assert_eq!(snapshot["payload"]["begin_missing"], true);
    assert_eq!(
        store
            .body(snapshot["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"before execution"
    );
    assert_eq!(
        store.show(capture.end_event_id()).unwrap()["record"]["payload"]["begin_missing"],
        true
    );
    assert_eq!(
        store.record_end(&capture, end()).unwrap()["status"],
        "duplicate"
    );
    assert_eq!(store.record_begin(&capture).unwrap_err().status, "invalid");
}

#[test]
fn failed_close_does_not_change_policy_and_lock_wait_is_bounded() {
    let (dir, store) = store();
    let lock = rusqlite::Connection::open(dir.path().join("telemetry/telemetry.sqlite3")).unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let started = std::time::Instant::now();
    let result = store.set_recording(
        None,
        SettingInput {
            schema_version: 1,
            enabled: false,
            actor: "synthetic".into(),
        },
    );
    assert_eq!(result.unwrap_err().status, "unavailable");
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert_eq!(store.settings().unwrap()["enabled"], true);
    assert_eq!(store.settings().unwrap()["generation"], 1);
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        store.events(&EventQuery::default()).unwrap()["events"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn closed_capture_cannot_publish_staged_bytes_or_read_an_end_body() {
    let (dir, store) = store();
    let brief = dir.path().join("brief");
    std::fs::write(&brief, b"staged only").unwrap();
    let capture = store
        .prepare_operation(operation("agent.send"), begin(), Some(brief))
        .unwrap();
    store
        .set_recording(
            Some("t"),
            SettingInput {
                schema_version: 1,
                enabled: false,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
    store
        .set_recording(
            Some("t"),
            SettingInput {
                schema_version: 1,
                enabled: true,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
    assert_eq!(store.record_begin(&capture).unwrap_err().status, "disabled");
    assert!(!dir.path().join("telemetry/blobs").exists());
    let mut observation = end();
    observation.bodies.push(BodyInput {
        role: "reply".into(),
        path: dir.path().join("missing"),
    });
    assert_eq!(
        store.record_end(&capture, observation).unwrap_err().status,
        "disabled"
    );
    assert!(store.show(capture.operation_id()).is_err());
}

#[test]
fn reply_without_end_is_unknown_but_not_an_unknown_delivery() {
    let (_dir, store) = store();
    let begin = Observation {
        observed_at: Value::Null,
        payload: json!({"target_name":"synthetic","gaps":[]}),
        bodies: vec![],
    };
    let capture = store
        .prepare_operation(operation("agent.reply"), begin, None)
        .unwrap();
    store.record_begin(&capture).unwrap();
    let record = store.show(capture.operation_id()).unwrap()["record"].clone();
    assert_eq!(record["end_missing"], true);
    assert_eq!(record["delivery_result_unknown"], false);
    assert_eq!(record["unknown"], "回复查询未完成/结果未知");
}

#[test]
fn concurrent_close_and_publication_have_one_ordered_outcome() {
    for _ in 0..12 {
        let (dir, store) = store();
        let file = dir.path().join("brief");
        std::fs::write(&file, b"racing snapshot").unwrap();
        let capture = store
            .prepare_operation(operation("agent.send"), begin(), Some(file))
            .unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let other = barrier.clone();
        let root = dir.path().join("telemetry");
        let closer = std::thread::spawn(move || {
            other.wait();
            Store::new(root).set_recording(
                None,
                SettingInput {
                    schema_version: 1,
                    enabled: false,
                    actor: "synthetic".into(),
                },
            )
        });
        barrier.wait();
        let publication = store.record_begin(&capture);
        closer.join().unwrap().unwrap();
        let events = store.events(&EventQuery::default()).unwrap()["events"]
            .as_array()
            .unwrap()
            .clone();
        let close = events
            .iter()
            .find(|e| e["kind"] == "recording.changed" && e["payload"]["enabled"] == false)
            .unwrap();
        match publication {
            Ok(_) => {
                let begin = events
                    .iter()
                    .find(|e| e["kind"] == "agent.send.begin")
                    .unwrap();
                let snapshot = events
                    .iter()
                    .find(|e| e["kind"] == "brief.snapshot")
                    .unwrap();
                assert!(snapshot["seq"].as_i64().unwrap() < begin["seq"].as_i64().unwrap());
                assert!(begin["seq"].as_i64().unwrap() < close["seq"].as_i64().unwrap());
            }
            Err(error) => {
                assert_eq!(error.status, "disabled");
                assert_eq!(events.len(), 2);
                assert!(!dir.path().join("telemetry/blobs").exists());
            }
        }
    }
}

#[test]
fn unreadable_brief_is_a_gap_without_a_fake_snapshot() {
    let (dir, store) = store();
    // A directory is not a readable regular body file.
    let capture = store
        .prepare_operation(
            operation("agent.send"),
            begin(),
            Some(dir.path().to_owned()),
        )
        .unwrap();
    store.record_begin(&capture).unwrap();
    let events = store.events(&EventQuery::default()).unwrap()["events"]
        .as_array()
        .unwrap()
        .clone();
    assert!(!events.iter().any(|e| e["kind"] == "brief.snapshot"));
    let begin = events
        .iter()
        .find(|e| e["kind"] == "agent.send.begin")
        .unwrap();
    assert!(
        begin["payload"]["gaps"]
            .as_array()
            .unwrap()
            .contains(&json!({"role":"brief","reason":"unreadable"}))
    );
    assert!(begin["links"].as_array().unwrap().is_empty());
}

#[test]
fn body_limit_preserves_full_bytes_and_oversize_is_an_explicit_gap() {
    let (dir, store) = store();
    let path = dir.path().join("large");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_BODY_BYTES).unwrap();
    let started = std::time::Instant::now();
    let capture = store
        .prepare_operation(operation("agent.send"), begin(), Some(path.clone()))
        .unwrap();
    let staged = started.elapsed();
    let publication = std::time::Instant::now();
    store.record_begin(&capture).unwrap();
    eprintln!(
        "synthetic 16 MiB: stage={staged:?}, publish+commit={:?}",
        publication.elapsed()
    );
    let snapshot = store.events(&EventQuery::default()).unwrap()["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap()
        .clone();
    let body = store
        .body(snapshot["bodies"][0]["sha256"].as_str().unwrap())
        .unwrap();
    assert_eq!(body.len() as u64, MAX_BODY_BYTES);
    assert!(body.iter().all(|byte| *byte == 0));
    file.set_len(MAX_BODY_BYTES + 1).unwrap();
    let large = store
        .prepare_operation(operation("agent.send"), begin(), Some(path))
        .unwrap();
    store.record_begin(&large).unwrap();
    let begin = store.show(large.begin_event_id()).unwrap()["record"].clone();
    assert!(
        begin["payload"]["gaps"]
            .as_array()
            .unwrap()
            .contains(&json!({"role":"brief","reason":"too_large"}))
    );
    assert!(begin["links"].as_array().unwrap().is_empty());
}

#[test]
fn bundled_sqlite_schema_permissions_and_versions_are_checkable() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, store) = store();
    assert!(rusqlite::version_number() >= 3_051_003);
    assert_eq!(
        store.settings().unwrap()["sqlite_version"],
        rusqlite::version()
    );
    let root = dir.path().join("telemetry");
    assert_eq!(
        std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(root.join("telemetry.sqlite3"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let db = rusqlite::Connection::open(root.join("telemetry.sqlite3")).unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 8);
    assert_eq!(
        db.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    let count: i64 = db
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
    db.pragma_update(None, "user_version", 2).unwrap();
    assert_eq!(store.settings().unwrap_err().status, "unavailable");
    assert_eq!(
        store
            .set_recording(
                None,
                SettingInput {
                    schema_version: 1,
                    enabled: false,
                    actor: "synthetic".into()
                }
            )
            .unwrap_err()
            .status,
        "unavailable"
    );
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn execution_payloads_are_closed_and_declared_labels_do_not_replace_parameters() {
    let (_dir, store) = store();
    let payload = json!({"cwd":"/synthetic","agent_program":"fake","explicit_parameters":{"model":"actual-request","effort":"high"},"labels":{"model":"label-only","role":"reviewer"},"gaps":[]});
    let mut bad = payload.clone();
    bad["explicit_parameters"]["token"] = json!("synthetic-token");
    assert_eq!(
        store
            .prepare_operation(
                operation("agent.start"),
                Observation {
                    observed_at: Value::Null,
                    payload: bad,
                    bodies: vec![]
                },
                None
            )
            .err()
            .unwrap()
            .status,
        "invalid"
    );
    let capture = store
        .prepare_operation(
            operation("agent.start"),
            Observation {
                observed_at: Value::Null,
                payload,
                bodies: vec![],
            },
            None,
        )
        .unwrap();
    store.record_begin(&capture).unwrap();
    let shown = store.show(capture.begin_event_id()).unwrap()["record"].clone();
    assert_eq!(
        shown["payload"]["explicit_parameters"]["model"],
        "actual-request"
    );
    assert_eq!(shown["payload"]["labels"]["model"], "label-only");
    let mut invalid = begin();
    invalid.payload["send_kind"] = json!("guessed");
    assert_eq!(
        store
            .prepare_operation(operation("agent.send"), invalid, None)
            .err()
            .unwrap()
            .status,
        "invalid"
    );
}

#[test]
fn route_and_reply_preserve_success_bodies_and_unknowns() {
    let (dir, store) = store();
    let path = dir.path().join("data");
    std::fs::write(&path, b"{\"synthetic\":true}\r\n").unwrap();
    let route=store.prepare_operation(operation("route"),Observation{observed_at:Value::Null,payload:json!({"router_model":"synthetic","router_version":null,"rules_version":"v1","gaps":[{"role":"router_version","reason":"not_available"}]}),bodies:vec![BodyInput{role:"summary".into(),path:path.clone()},BodyInput{role:"request".into(),path:path.clone()}]},None).unwrap();
    store.record_begin(&route).unwrap();
    let result = Observation {
        observed_at: Value::Null,
        payload: json!({"outcome":{"kind":"exited","exit_code":0},"gaps":[]}),
        bodies: vec![
            BodyInput {
                role: "response".into(),
                path: path.clone(),
            },
            BodyInput {
                role: "suggestion".into(),
                path: path.clone(),
            },
        ],
    };
    store.record_end(&route, result).unwrap();
    let shown = store.show(route.end_event_id()).unwrap()["record"].clone();
    assert_eq!(shown["bodies"].as_array().unwrap().len(), 2);
    for body in shown["bodies"].as_array().unwrap() {
        assert_eq!(
            store.body(body["sha256"].as_str().unwrap()).unwrap(),
            b"{\"synthetic\":true}\r\n"
        );
    }
    let reply = store
        .prepare_operation(
            operation("agent.reply"),
            Observation {
                observed_at: Value::Null,
                payload: json!({"target_name":"synthetic","gaps":[]}),
                bodies: vec![],
            },
            None,
        )
        .unwrap();
    store.record_end(&reply,Observation{observed_at:Value::Null,payload:json!({"outcome":{"kind":"exited","exit_code":0},"name":"synthetic","instance":"instance","at":"opaque public at","association":"not_proven","gaps":[]}),bodies:vec![BodyInput{role:"reply".into(),path}]}).unwrap();
    let shown = store.show(reply.end_event_id()).unwrap()["record"].clone();
    assert_eq!(shown["payload"]["association"], "not_proven");
    assert_eq!(shown["payload"]["begin_missing"], true);
}

#[test]
fn stored_events_cannot_acquire_new_links_after_commit() {
    let (dir, store) = store();
    let file = dir.path().join("body");
    std::fs::write(&file, b"synthetic").unwrap();
    for id in ["u1", "u2"] {
        store.append(serde_json::from_value(json!({"schema_version":1,"event_id":id,"trace_id":"t","kind":"requirement.recorded","observed_at":null,"producer":"synthetic","evidence_kind":"controller_statement","payload":{"declared_speaker":"synthetic","acquisition":"controller_transcription","declared_at":null},"links":[],"bodies":[{"role":"text","path":file}]})).unwrap()).unwrap();
    }
    let db = rusqlite::Connection::open(dir.path().join("telemetry/telemetry.sqlite3")).unwrap();
    assert!(
        db.execute("INSERT INTO event_links VALUES('u2','supersedes','u1')", [])
            .is_err()
    );
    assert!(
        db.execute(
            "UPDATE events SET recorded_at='2020-01-01T00:00:00Z' WHERE event_id='u1'",
            []
        )
        .is_err()
    );
    assert!(
        db.execute("DELETE FROM event_blobs WHERE event_id='u1'", [])
            .is_err()
    );
}

#[test]
fn missing_staged_brief_is_an_end_gap_and_never_rereads_the_input_file() {
    let (dir, store) = store();
    let file = dir.path().join("brief");
    std::fs::write(&file, b"before execution").unwrap();
    let capture = store
        .prepare_operation(operation("agent.send"), begin(), Some(file.clone()))
        .unwrap();
    for item in std::fs::read_dir(dir.path().join("telemetry/tmp")).unwrap() {
        std::fs::remove_file(item.unwrap().path()).unwrap();
    }
    std::fs::write(file, b"changed after execution").unwrap();
    store.record_end(&capture, end()).unwrap();
    let record = store.show(capture.end_event_id()).unwrap()["record"].clone();
    assert_eq!(record["payload"]["begin_missing"], true);
    assert!(
        record["payload"]["gaps"]
            .as_array()
            .unwrap()
            .contains(&json!({"role":"brief","reason":"not_available"}))
    );
    assert!(record["links"].as_array().unwrap().is_empty());
    assert!(!dir.path().join("telemetry/blobs").exists());
}

#[test]
fn malformed_internal_payloads_are_errors_not_panics() {
    let (_dir, store) = store();
    let capture = store
        .prepare_operation(operation("agent.send"), begin(), None)
        .unwrap();
    for payload in [json!(true), json!("invalid"), json!([]), Value::Null] {
        assert_eq!(
            store
                .prepare_operation(
                    operation("agent.send"),
                    Observation {
                        observed_at: Value::Null,
                        payload: payload.clone(),
                        bodies: vec![]
                    },
                    None
                )
                .err()
                .unwrap()
                .status,
            "invalid"
        );
        assert_eq!(
            store
                .record_end(
                    &capture,
                    Observation {
                        observed_at: Value::Null,
                        payload,
                        bodies: vec![]
                    }
                )
                .unwrap_err()
                .status,
            "invalid"
        );
    }
}
