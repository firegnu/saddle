use rusqlite::Connection;
use saddle::telemetry::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn setting(enabled: bool) -> SettingInput {
    SettingInput {
        schema_version: 1,
        enabled,
        actor: "synthetic".into(),
    }
}

fn trace(id: &str) -> TraceInput {
    serde_json::from_value(
        json!({"schema_version":1,"trace_id":id,"origin":"ad_hoc","label":"synthetic"}),
    )
    .unwrap()
}

// Frozen pre-migration schema: never derive an old database by undoing the current schema.
fn legacy(root: &Path) -> Store {
    fs::create_dir_all(root).unwrap();
    let conn = Connection::open(root.join("telemetry.sqlite3")).unwrap();
    conn.execute_batch(include_str!("fixtures/telemetry-v1.sql"))
        .unwrap();
    conn.execute(
        "INSERT INTO recording_policy VALUES(1,1,1,'2020-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO traces VALUES('t','ad_hoc','2020-01-02T00:00:00Z',1,0,NULL,NULL,NULL,NULL,?)",
        [serde_json::to_string(&trace("t")).unwrap()],
    )
    .unwrap();
    let body = b"historical closure\0\xff\r\n";
    let hash = format!("{:x}", Sha256::digest(body));
    fs::create_dir_all(root.join("blobs/sha256")).unwrap();
    fs::write(root.join("blobs/sha256").join(&hash), body).unwrap();
    conn.execute(
        "INSERT INTO blobs VALUES(?,?)",
        rusqlite::params![hash, body.len() as i64],
    )
    .unwrap();
    let event = json!({"schema_version":1,"event_id":"old-closure","trace_id":"t","dispatch_id":null,"operation_id":null,"kind":"controller.note","observed_at":null,"producer":"synthetic","evidence_kind":"controller_statement","source_auth":"unverified","payload":{"note_kind":"closure"},"links":[],"bodies":[{"role":"text","sha256":hash,"bytes":body.len()}]});
    conn.execute("INSERT INTO events(event_id,trace_id,kind,recorded_at,fingerprint,record_json) VALUES('old-closure','t','controller.note','2020-01-03T00:00:00Z','old-fingerprint',?)", [event.to_string()]).unwrap();
    conn.execute(
        "INSERT INTO event_blobs VALUES('old-closure','text',?)",
        [hash],
    )
    .unwrap();
    Store::new(root.to_owned())
}

fn version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

fn history(conn: &Connection) -> Vec<(i64, String, String, String)> {
    conn.prepare("SELECT seq,recorded_at,fingerprint,record_json FROM events ORDER BY seq")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn v1_reads_are_unchanged_and_first_writer_migrates_without_rewriting_history() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = legacy(&root);
    let path = root.join("telemetry.sqlite3");
    let conn = Connection::open(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let original = history(&conn);
    assert_eq!(store.settings().unwrap()["enabled"], true);
    assert!(store.list().unwrap()["traces"][0]["closed_at"].is_null());
    let shown = store.show("t").unwrap();
    assert!(shown["record"]["closed_at"].is_null());
    assert_eq!(
        shown["record"]["capture_enabled"], true,
        "closure is only a declaration"
    );
    assert_eq!(
        store.events(&EventQuery::default()).unwrap()["events"][0]["event_id"],
        "old-closure"
    );
    assert_eq!(version(&conn), 1);
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "reading must not migrate or write"
    );
    assert_eq!(
        store.set_recording(None, setting(true)).unwrap()["status"],
        "duplicate"
    );
    assert_eq!(
        version(&conn),
        2,
        "the first writer must atomically upgrade v1"
    );
    assert_eq!(history(&conn), original);
    let old = store.show("old-closure").unwrap();
    assert_eq!(
        store
            .body(old["record"]["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"historical closure\0\xff\r\n"
    );
    assert!(store.show("t").unwrap()["record"]["closed_at"].is_null());
    store.create_trace(trace("new")).unwrap();
    assert_eq!(store.list().unwrap()["traces"].as_array().unwrap().len(), 2);
}

#[test]
fn migration_failure_rolls_back_column_and_version_and_disabled_writes_still_migrate() {
    let dir = tempfile::tempdir().unwrap();
    let store = legacy(dir.path());
    let conn = Connection::open(dir.path().join("telemetry.sqlite3")).unwrap();
    // A synthetic conflicting schema object makes migration fail after ALTER TABLE.
    conn.execute_batch("CREATE TRIGGER no_closed_trace_update BEFORE UPDATE ON traces BEGIN SELECT 1; END; UPDATE recording_policy SET enabled=0;").unwrap();
    let original = history(&conn);
    let error = store.create_trace(trace("new")).unwrap_err();
    assert_eq!(
        error.status, "unavailable",
        "migration must fail before the disabled gate"
    );
    assert_eq!(version(&conn), 1);
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM pragma_table_info('traces') WHERE name='closed_at'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(history(&conn), original);
    conn.execute_batch("DROP TRIGGER no_closed_trace_update")
        .unwrap();
    assert_eq!(
        store.create_trace(trace("new")).unwrap_err().status,
        "disabled"
    );
    assert_eq!(version(&conn), 2);
    assert_eq!(history(&conn), original);
}

fn close(store: &Store) -> saddle::telemetry::Result<serde_json::Value> {
    store.close_trace(TraceCloseInput {
        schema_version: 1,
        trace_id: "t".into(),
        actor: "synthetic".into(),
    })
}

fn current(root: &Path) -> Store {
    let store = Store::new(root.to_owned());
    store.set_recording(None, setting(true)).unwrap();
    store.create_trace(trace("t")).unwrap();
    store.create_dispatch(serde_json::from_value(json!({"schema_version":1,"dispatch_id":"d","trace_id":"t","kind":"implementation"})).unwrap()).unwrap();
    store
}

fn operation() -> OperationInput {
    OperationInput {
        schema_version: 1,
        trace_id: "t".into(),
        dispatch_id: "d".into(),
        kind: "agent.reply".into(),
        producer: "synthetic".into(),
        basis_event_ids: vec![],
        decision_event_id: None,
        previous_brief_event_id: None,
    }
}
fn observation() -> Observation {
    Observation {
        observed_at: serde_json::Value::Null,
        payload: json!({"target_name":"synthetic","gaps":[]}),
        bodies: vec![],
    }
}

#[test]
fn close_revokes_prepared_captures_and_preserves_unknown_inflight_results() {
    let dir = tempfile::tempdir().unwrap();
    let store = current(dir.path());
    let begun = store
        .prepare_operation(operation(), observation(), None)
        .unwrap();
    let unbegun = store
        .prepare_operation(operation(), observation(), None)
        .unwrap();
    store.record_begin(&begun).unwrap();
    close(&store).unwrap();
    assert_eq!(
        store.check_operation(&operation()).unwrap_err().code,
        "trace_closed"
    );
    assert_eq!(
        store
            .prepare_operation(operation(), observation(), None)
            .err()
            .unwrap()
            .code,
        "trace_closed"
    );
    assert_eq!(
        store.record_begin(&unbegun).unwrap_err().code,
        "trace_closed"
    );
    assert_eq!(store.record_begin(&begun).unwrap_err().code, "trace_closed");
    let end = Observation {
        observed_at: serde_json::Value::Null,
        payload: json!({"outcome":{"kind":"exited","exit_code":0},"gaps":[]}),
        bodies: vec![BodyInput {
            role: "reply".into(),
            path: dir.path().join("must-not-read"),
        }],
    };
    assert_eq!(
        store.record_end(&begun, end).unwrap_err().code,
        "trace_closed"
    );
    let shown = store.show(begun.operation_id()).unwrap();
    assert_eq!(shown["record"]["end_missing"], true);
    assert!(shown["record"]["unknown"].is_string());
    assert!(store.show(unbegun.operation_id()).is_err());
    assert!(!dir.path().join("blobs").exists());
    let mut invalid = operation();
    invalid.dispatch_id = "missing".into();
    assert_eq!(
        store.check_operation(&invalid).unwrap_err().status,
        "invalid"
    );
    store.set_recording(None, setting(false)).unwrap();
    invalid.trace_id = "missing".into();
    assert_eq!(
        store.check_operation(&invalid).unwrap_err().status,
        "invalid"
    );
    let dispatch = |id: &str| {
        serde_json::from_value(
            json!({"schema_version":1,"dispatch_id":"new","trace_id":id,"kind":"implementation"}),
        )
        .unwrap()
    };
    assert_eq!(
        store.create_dispatch(dispatch("missing")).unwrap_err().code,
        "recording_disabled"
    );
    store.set_recording(None, setting(true)).unwrap();
    assert_eq!(
        store
            .create_dispatch(dispatch("missing"))
            .unwrap_err()
            .status,
        "invalid"
    );
}

#[test]
fn close_is_atomic_and_sql_triggers_protect_migrated_and_new_stores() {
    for old in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = if old {
            legacy(dir.path())
        } else {
            current(dir.path())
        };
        if old {
            store.create_dispatch(serde_json::from_value(json!({"schema_version":1,"dispatch_id":"d","trace_id":"t","kind":"implementation"})).unwrap()).unwrap();
        }
        let mut conn = Connection::open(dir.path().join("telemetry.sqlite3")).unwrap();
        // Fail after the closing event has been inserted: neither half may survive.
        conn.execute_batch("CREATE TRIGGER fail_close BEFORE UPDATE ON traces BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
        let original = history(&conn);
        assert_eq!(close(&store).unwrap_err().status, "unavailable");
        assert_eq!(history(&conn), original);
        assert!(store.show("t").unwrap()["record"]["closed_at"].is_null());
        assert_eq!(store.show("t").unwrap()["record"]["capture_enabled"], true);
        conn.execute_batch("DROP TRIGGER fail_close").unwrap();
        store.set_recording(Some("t"), setting(false)).unwrap();
        assert_eq!(
            close(&store).unwrap()["status"],
            "stored",
            "paused trace can end"
        );
        assert_eq!(close(&store).unwrap()["status"], "duplicate");
        let before = history(&conn);
        assert_eq!(
            before.last().unwrap().1,
            store.show("t").unwrap()["record"]["closed_at"]
                .as_str()
                .unwrap()
        );
        for sql in [
            "UPDATE traces SET closed_at=NULL WHERE trace_id='t'",
            "UPDATE traces SET capture_enabled=1 WHERE trace_id='t'",
            "UPDATE traces SET generation=generation+1 WHERE trace_id='t'",
            "INSERT INTO dispatches VALUES('forbidden','t',NULL,'review','2020-01-01T00:00:00Z','{}')",
            "INSERT INTO events(event_id,trace_id,kind,recorded_at,fingerprint,record_json) VALUES('forbidden','t','controller.note','2020-01-01T00:00:00Z','x','{}')",
        ] {
            assert!(conn.execute_batch(sql).is_err(), "{sql}");
        }
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO operations VALUES('forbidden','t','d','agent.reply','synthetic',1,0)",
            [],
        )
        .unwrap();
        assert!(tx.execute("INSERT INTO events(event_id,trace_id,kind,recorded_at,fingerprint,record_json) VALUES('op-event','t','controller.note','2020-01-01T00:00:00Z','x','{}')", []).is_err());
        tx.rollback().unwrap();
        assert!(store.show("forbidden").is_err());
        assert_eq!(history(&conn), before);
    }
}

#[test]
fn concurrent_migration_close_and_capture_have_a_single_terminal_boundary() {
    use std::sync::{Arc, Barrier};
    let dir = tempfile::tempdir().unwrap();
    let store = legacy(dir.path());
    let barrier = Arc::new(Barrier::new(2));
    std::thread::scope(|scope| {
        let jobs: Vec<_> = (0..2)
            .map(|_| {
                let barrier = barrier.clone();
                let store = &store;
                scope.spawn(move || {
                    barrier.wait();
                    store.set_recording(None, setting(true)).unwrap()
                })
            })
            .collect();
        for job in jobs {
            assert_eq!(job.join().unwrap()["status"], "duplicate");
        }
    });
    store.create_dispatch(serde_json::from_value(json!({"schema_version":1,"dispatch_id":"d","trace_id":"t","kind":"implementation"})).unwrap()).unwrap();
    let capture = store
        .prepare_operation(operation(), observation(), None)
        .unwrap();
    let barrier = Barrier::new(3);
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            close(&store).unwrap()
        });
        let b = scope.spawn(|| {
            barrier.wait();
            close(&store).unwrap()
        });
        let c = scope.spawn(|| {
            barrier.wait();
            store.record_begin(&capture)
        });
        let mut statuses = [
            a.join().unwrap()["status"].as_str().unwrap().to_owned(),
            b.join().unwrap()["status"].as_str().unwrap().to_owned(),
        ];
        statuses.sort();
        assert_eq!(statuses, ["duplicate", "stored"]);
        if let Err(error) = c.join().unwrap() {
            assert_eq!(error.code, "trace_closed");
        }
    });
    let events = store
        .events(&EventQuery {
            trace_id: Some("t".into()),
            ..EventQuery::default()
        })
        .unwrap();
    let events = events["events"].as_array().unwrap();
    assert_eq!(events.last().unwrap()["payload"]["closed"], true);
    assert_eq!(
        events
            .iter()
            .filter(|e| e["payload"]["closed"] == true)
            .count(),
        1
    );
    assert_eq!(
        store.record_begin(&capture).unwrap_err().code,
        "trace_closed"
    );
}

#[test]
fn terminal_intervals_use_sequence_even_when_the_clock_moves_backwards() {
    let dir = tempfile::tempdir().unwrap();
    let store = current(dir.path());
    let conn = Connection::open(dir.path().join("telemetry.sqlite3")).unwrap();
    // Synthetic control events with intentionally non-monotonic timestamps; no history edits.
    let insert = |id: &str, trace: Option<&str>, at: &str, enabled: bool, closed: bool| {
        let record = json!({"schema_version":1,"event_id":id,"trace_id":trace,"dispatch_id":null,"operation_id":null,"kind":"recording.changed","observed_at":at,"producer":"saddle","evidence_kind":"system_control","source_auth":"system_control","payload":{"enabled":enabled,"generation":1,"actor":"synthetic","changed_at":at,"closed":closed},"links":[],"bodies":[]});
        conn.execute("INSERT INTO events(event_id,trace_id,kind,recorded_at,fingerprint,record_json) VALUES(?,?,'recording.changed',?,'synthetic',?)", rusqlite::params![id,trace,at,record.to_string()]).unwrap();
    };
    insert("future", None, "2099-01-01T00:00:00Z", true, false);
    insert("end", Some("t"), "2010-01-01T00:00:00Z", false, true);
    conn.execute("UPDATE traces SET capture_enabled=0,generation=1,closed_at='2010-01-01T00:00:00Z' WHERE trace_id='t'", []).unwrap();
    insert("after", None, "2000-01-01T00:00:00Z", false, false);
    let shown = store.show("t").unwrap();
    let history = shown["record"]["recording_history"].as_array().unwrap();
    assert_eq!(
        history
            .iter()
            .map(|e| e["event_id"].as_str().unwrap())
            .collect::<Vec<_>>()[1..],
        ["future", "end"]
    );
    let intervals = shown["record"]["recording_intervals"].as_array().unwrap();
    assert_eq!(
        intervals.last().unwrap(),
        &json!({"start":"2099-01-01T00:00:00Z","end":"2010-01-01T00:00:00Z","enabled":true})
    );
    assert!(intervals.iter().all(|i| !i["end"].is_null()));
    assert_eq!(
        store.events(&EventQuery::default()).unwrap()["events"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["event_id"],
        "after"
    );
}
