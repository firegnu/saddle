use serde_json::{Value, json};
use std::process::{Command, Output};

fn run(dir: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_saddle"))
        .arg("telemetry")
        .args(args)
        .env("HOME", dir.path())
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("SADDLE_RUNTIME_DIR", dir.path().join("runtime"))
        .output()
        .unwrap()
}

fn response(output: Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn submit(dir: &tempfile::TempDir, command: &[&str], value: Value, code: i32) -> Value {
    let path = dir.path().join("input.json");
    std::fs::write(&path, value.to_string()).unwrap();
    let mut args = command.to_vec();
    args.extend(["--input", path.to_str().unwrap()]);
    response(run(dir, &args), code)
}

fn setting(enabled: bool) -> Value {
    json!({"schema_version":1,"enabled":enabled,"actor":"synthetic controller"})
}

#[test]
fn trace_close_is_irreversible_readable_and_independent_of_global_recording() {
    let dir = tempfile::tempdir().unwrap();
    let close = json!({"schema_version":1,"trace_id":"t1","actor":"synthetic"});
    let missing = submit(&dir, &["trace", "close"], close.clone(), 2);
    assert_eq!(missing["error"]["message"], "unknown trace_id");
    assert!(!dir.path().join("state").exists());
    submit(&dir, &["settings", "set"], setting(true), 0);
    submit(&dir, &["trace", "create"], trace("t1"), 0);
    let event = requirement(&dir, "u1", "t1");
    submit(&dir, &["append"], event.clone(), 0);
    submit(&dir, &["settings", "set"], setting(false), 0);
    assert_eq!(
        submit(&dir, &["trace", "close"], close.clone(), 0)["status"],
        "stored"
    );
    assert_eq!(
        submit(&dir, &["trace", "close"], close.clone(), 0)["status"],
        "duplicate"
    );
    let shown = response(run(&dir, &["show", "--id", "t1"]), 0);
    let record = &shown["record"];
    assert!(record["closed_at"].is_string());
    assert_eq!(record["capture_enabled"], false);
    assert_eq!(record["generation"], 1);
    assert_eq!(
        response(run(&dir, &["list"]), 0)["traces"][0]["closed_at"],
        record["closed_at"]
    );
    std::fs::remove_file(dir.path().join("body.bin")).unwrap();
    for global in [false, true] {
        submit(&dir, &["settings", "set"], setting(global), 0);
        assert_eq!(
            submit(&dir, &["trace", "create"], trace("t1"), 4)["error"]["code"],
            "trace_closed"
        );
        assert_eq!(
            submit(
                &dir,
                &["dispatch", "create"],
                json!({"schema_version":1,"dispatch_id":"d","trace_id":"t1","kind":"implementation"}),
                4
            )["error"]["code"],
            "trace_closed"
        );
        assert_eq!(
            submit(&dir, &["append"], event.clone(), 4)["error"]["code"],
            "trace_closed",
            "do not read the removed body"
        );
        assert_eq!(
            submit(
                &dir,
                &["trace", "set-recording"],
                json!({"schema_version":1,"trace_id":"t1","enabled":true,"actor":"synthetic"}),
                4
            )["error"]["code"],
            "trace_closed"
        );
        assert_eq!(
            submit(
                &dir,
                &["trace", "set-recording"],
                json!({"schema_version":1,"trace_id":"t1","enabled":false,"actor":"synthetic"}),
                0
            )["status"],
            "duplicate"
        );
    }
    let again = response(run(&dir, &["show", "--id", "t1"]), 0);
    assert_eq!(
        again, shown,
        "later global changes cannot extend a closed trace's history"
    );
    let intervals = record["recording_intervals"].as_array().unwrap();
    assert!(intervals.iter().all(|i| i["end"].is_string()));
    assert_eq!(intervals.last().unwrap()["end"], record["closed_at"]);
    let history = record["recording_history"].as_array().unwrap();
    let last = history.last().unwrap();
    assert_eq!(last["payload"]["closed"], true);
    assert_eq!(last["payload"]["changed_at"], record["closed_at"]);
    let stored = response(run(&dir, &["show", "--id", "u1"]), 0);
    let body = run(
        &dir,
        &[
            "body",
            "--sha256",
            stored["record"]["bodies"][0]["sha256"].as_str().unwrap(),
        ],
    );
    assert_eq!(body.stdout, b"synthetic\0\xff\r\n");
    let mut invalid = close;
    invalid["enabled"] = json!(true);
    submit(&dir, &["trace", "close"], invalid, 2);
}

fn trace(id: &str) -> Value {
    json!({"schema_version":1,"trace_id":id,"origin":"ad_hoc","label":"synthetic"})
}

fn requirement(dir: &tempfile::TempDir, id: &str, trace_id: &str) -> Value {
    let path = dir.path().join("body.bin");
    std::fs::write(&path, b"synthetic\0\xff\r\n").unwrap();
    json!({"schema_version":1,"event_id":id,"trace_id":trace_id,"kind":"requirement.recorded",
        "observed_at":null,"producer":"synthetic controller","evidence_kind":"controller_statement",
        "payload":{"declared_speaker":"user (declared)","acquisition":"controller_transcription","declared_at":"2020-01-01T00:00:00Z"},
        "links":[],"bodies":[{"role":"text","path":path}]})
}

#[test]
fn append_preserves_bytes_source_and_immutable_content() {
    let dir = tempfile::tempdir().unwrap();
    submit(&dir, &["settings", "set"], setting(true), 0);
    submit(&dir, &["trace", "create"], trace("t1"), 0);
    let event = requirement(&dir, "u1", "t1");
    assert_eq!(
        submit(&dir, &["append"], event.clone(), 0)["status"],
        "stored"
    );
    assert_eq!(
        submit(&dir, &["append"], event.clone(), 0)["status"],
        "duplicate"
    );
    let alternate = dir.path().join("same-bytes");
    std::fs::write(&alternate, b"synthetic\0\xff\r\n").unwrap();
    let mut same = event.clone();
    same["bodies"][0]["path"] = json!(alternate);
    assert_eq!(submit(&dir, &["append"], same, 0)["status"], "duplicate");
    let shown = response(run(&dir, &["show", "--id", "u1"]), 0);
    assert_eq!(shown["record"]["source_auth"], "unverified");
    assert_eq!(shown["record"]["late_submission"], true);
    let hash = shown["record"]["bodies"][0]["sha256"].as_str().unwrap();
    let body = run(&dir, &["body", "--sha256", hash]);
    assert!(body.status.success());
    assert_eq!(body.stdout, b"synthetic\0\xff\r\n");
    std::fs::write(dir.path().join("body.bin"), b"changed").unwrap();
    assert_eq!(
        submit(&dir, &["append"], event.clone(), 3)["status"],
        "conflict"
    );
    let mut forged = event.clone();
    forged["event_id"] = json!("forged");
    forged["evidence_kind"] = json!("execution_observed");
    assert_eq!(submit(&dir, &["append"], forged, 2)["status"], "invalid");
    submit(&dir, &["settings", "set"], setting(false), 0);
    std::fs::remove_file(dir.path().join("body.bin")).unwrap();
    assert_eq!(submit(&dir, &["append"], event, 4)["status"], "disabled");
}

#[test]
fn reuse_is_explicit_and_pages_keep_the_original_upper_bound() {
    let dir = tempfile::tempdir().unwrap();
    submit(&dir, &["settings", "set"], setting(true), 0);
    for id in ["t1", "t2"] {
        submit(&dir, &["trace", "create"], trace(id), 0);
    }
    submit(&dir, &["append"], requirement(&dir, "u1", "t1"), 0);
    let reused = json!({"schema_version":1,"event_id":"reused","trace_id":"t2","kind":"evidence.reused",
        "observed_at":null,"producer":"synthetic plugin","evidence_kind":"plugin_statement",
        "payload":{"reused_by":"synthetic plugin"},"bodies":[],"links":[{"relation":"carried_from","target_event_id":"u1"}]});
    submit(&dir, &["append"], reused, 0);
    let shown = response(run(&dir, &["show", "--id", "reused"]), 0);
    assert_eq!(
        shown["record"]["payload"]["acquisition"],
        "controller_transcription"
    );
    assert_eq!(shown["record"]["source_auth"], "unverified");
    let first = response(run(&dir, &["events", "--limit", "1"]), 0);
    let upper = first["upper_seq"].as_i64().unwrap().to_string();
    let after = first["next_after_seq"].as_i64().unwrap().to_string();
    submit(&dir, &["append"], requirement(&dir, "u2", "t1"), 0);
    let remaining = response(
        run(
            &dir,
            &["events", "--after-seq", &after, "--upper-seq", &upper],
        ),
        0,
    );
    assert_eq!(remaining["events"].as_array().unwrap().len(), 2);
    assert!(
        !remaining["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event_id"] == "u2")
    );
    let mut illegal = requirement(&dir, "bad", "t2");
    illegal["links"] = json!([{"relation":"supersedes","target_event_id":"u1"}]);
    assert_eq!(submit(&dir, &["append"], illegal, 2)["status"], "invalid");
}

#[test]
fn switches_gate_creation_and_stable_ids_do_not_duplicate_records() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        submit(&dir, &["trace", "create"], trace("t1"), 4)["status"],
        "disabled"
    );
    let mut changed = trace("t1");
    changed["label"] = json!("different while closed");
    assert_eq!(
        submit(&dir, &["trace", "create"], changed, 4)["status"],
        "disabled"
    );
    assert!(!dir.path().join("state").exists());
    assert_eq!(
        submit(&dir, &["settings", "set"], setting(true), 0)["status"],
        "stored"
    );
    assert_eq!(
        submit(&dir, &["settings", "set"], setting(true), 0)["status"],
        "duplicate"
    );
    assert_eq!(
        response(run(&dir, &["settings", "get"]), 0)["generation"],
        1
    );
    assert_eq!(
        submit(&dir, &["trace", "create"], trace("t1"), 0)["status"],
        "stored"
    );
    assert_eq!(
        submit(&dir, &["trace", "create"], trace("t1"), 0)["status"],
        "duplicate"
    );
    let mut changed = trace("t1");
    changed["label"] = json!("different");
    assert_eq!(
        submit(&dir, &["trace", "create"], changed, 3)["status"],
        "conflict"
    );
    let dispatch =
        json!({"schema_version":1,"dispatch_id":"d1","trace_id":"t1","kind":"implementation"});
    assert_eq!(
        submit(&dir, &["dispatch", "create"], dispatch.clone(), 0)["status"],
        "stored"
    );
    assert_eq!(
        submit(&dir, &["dispatch", "create"], dispatch, 0)["status"],
        "duplicate"
    );
    submit(
        &dir,
        &["trace", "set-recording"],
        json!({"schema_version":1,"trace_id":"t1","enabled":false,"actor":"synthetic"}),
        0,
    );
    assert_eq!(
        submit(&dir, &["trace", "create"], trace("t1"), 4)["status"],
        "disabled"
    );
    submit(&dir, &["settings", "set"], setting(false), 0);
    assert_eq!(
        response(run(&dir, &["list"]), 0)["traces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn event_schema_rejects_forged_sources_unknown_fields_and_invalid_links() {
    let dir = tempfile::tempdir().unwrap();
    submit(&dir, &["settings", "set"], setting(true), 0);
    submit(&dir, &["trace", "create"], trace("t1"), 0);
    let event = requirement(&dir, "source", "t1");
    submit(&dir, &["append"], event.clone(), 0);
    let mut invalid = Vec::new();
    let mut v = event.clone();
    v["schema_version"] = json!(2);
    invalid.push(v);
    let mut v = event.clone();
    v["payload"]["acquisition"] = json!("direct_user_input");
    invalid.push(v);
    let mut v = event.clone();
    v["evidence_kind"] = json!("system_control");
    invalid.push(v);
    let mut v = event.clone();
    v["payload"]["made_up"] = json!("field");
    invalid.push(v);
    let mut v = event.clone();
    v["bodies"] = json!([]);
    invalid.push(v);
    let mut v = event.clone();
    v["dispatch_id"] = json!("missing");
    invalid.push(v);
    let mut v = event.clone();
    v["operation_id"] = json!("missing");
    invalid.push(v);
    let mut v = event.clone();
    v["observed_at"] = json!("2020-01-01T08:00:00+08:00");
    invalid.push(v);
    let mut v = event.clone();
    v["links"] = json!([{"relation":"uses_brief","target_event_id":"source"}]);
    invalid.push(v);
    let mut v = event.clone();
    v["links"] = json!([{"relation":"supersedes","target_event_id":"future"}]);
    invalid.push(v);
    let mut v = event.clone();
    v["kind"] = json!("authorization.recorded");
    v["payload"] = json!({"declared_speaker":"synthetic","acquisition":"plugin_provided","context_complete":true});
    invalid.push(v);
    for (i, mut v) in invalid.into_iter().enumerate() {
        v["event_id"] = json!(format!("bad-{i}"));
        assert_eq!(submit(&dir, &["append"], v, 2)["status"], "invalid");
    }
    let mut revised = event.clone();
    revised["event_id"] = json!("revised");
    revised["links"] = json!([{"relation":"supersedes","target_event_id":"source"}]);
    submit(&dir, &["append"], revised.clone(), 0);
    revised["links"] = json!([]);
    assert_eq!(submit(&dir, &["append"], revised, 3)["status"], "conflict");
    assert_eq!(
        response(run(&dir, &["events"]), 0)["events"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn bindings_are_opaque_unique_per_run_and_queries_do_not_merge_attempts() {
    let dir = tempfile::tempdir().unwrap();
    submit(&dir, &["settings", "set"], setting(true), 0);
    let binding = json!({"kind":"synthetic.plugin","scope":"opaque-project","key":"T1","run":"r1"});
    let mut first = trace("t1");
    first["origin"] = json!("task");
    first["binding"] = binding;
    submit(&dir, &["trace", "create"], first.clone(), 0);
    let mut duplicate_binding = first.clone();
    duplicate_binding["trace_id"] = json!("t2");
    submit(&dir, &["trace", "create"], duplicate_binding.clone(), 3);
    duplicate_binding["binding"]["run"] = json!("r2");
    submit(&dir, &["trace", "create"], duplicate_binding, 0);
    let mut partial = trace("partial");
    partial["origin"] = json!("task");
    submit(&dir, &["trace", "create"], partial, 2);
    let listing = response(
        run(
            &dir,
            &[
                "list",
                "--kind",
                "synthetic.plugin",
                "--scope",
                "opaque-project",
                "--key",
                "T1",
            ],
        ),
        0,
    );
    assert_eq!(listing["traces"].as_array().unwrap().len(), 2);
    assert_eq!(listing["traces"][0]["registration"], "未见运行登记声明");
    let exact = response(
        run(
            &dir,
            &[
                "list",
                "--kind",
                "synthetic.plugin",
                "--scope",
                "opaque-project",
                "--key",
                "T1",
                "--run",
                "r2",
            ],
        ),
        0,
    );
    assert_eq!(exact["traces"].as_array().unwrap().len(), 1);
    assert_eq!(exact["traces"][0]["trace_id"], "t2");
    let transition = json!({"schema_version":1,"event_id":"transition","trace_id":"t1","kind":"task.transition","observed_at":null,"producer":"declared plugin","evidence_kind":"plugin_statement","payload":{"binding":first["binding"],"from":"ready","to":"running","business_committed_at":null},"bodies":[],"links":[]});
    submit(&dir, &["append"], transition, 0);
    assert_eq!(
        response(run(&dir, &["show", "--id", "t1"]), 0)["record"]["registration"],
        "登记声明未核验"
    );
}

#[test]
fn empty_missing_and_corrupt_bodies_are_distinct_and_paths_are_regular_absolute() {
    let dir = tempfile::tempdir().unwrap();
    submit(&dir, &["settings", "set"], setting(true), 0);
    submit(&dir, &["trace", "create"], trace("t1"), 0);
    let event = requirement(&dir, "empty", "t1");
    std::fs::write(dir.path().join("body.bin"), []).unwrap();
    submit(&dir, &["append"], event.clone(), 0);
    let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let body = run(&dir, &["body", "--sha256", hash]);
    assert!(body.status.success());
    assert!(body.stdout.is_empty());
    let path = dir
        .path()
        .join("state/saddle/telemetry/blobs/sha256")
        .join(hash);
    std::fs::write(&path, b"not empty").unwrap();
    let corrupt = run(&dir, &["body", "--sha256", hash]);
    assert_eq!(corrupt.status.code(), Some(1));
    assert!(corrupt.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&corrupt.stderr).unwrap()["error"]["code"],
        "body_corrupt"
    );
    std::fs::remove_file(path).unwrap();
    let missing = run(&dir, &["body", "--sha256", hash]);
    assert_eq!(
        serde_json::from_slice::<Value>(&missing.stderr).unwrap()["error"]["code"],
        "body_missing"
    );
    let content = requirement(&dir, "corrupt-hash", "t1");
    submit(&dir, &["append"], content, 0);
    let record = response(run(&dir, &["show", "--id", "corrupt-hash"]), 0);
    let hash = record["record"]["bodies"][0]["sha256"].as_str().unwrap();
    let bytes = record["record"]["bodies"][0]["bytes"].as_u64().unwrap() as usize;
    std::fs::write(
        dir.path()
            .join("state/saddle/telemetry/blobs/sha256")
            .join(hash),
        vec![b'x'; bytes],
    )
    .unwrap();
    let corrupt = run(&dir, &["body", "--sha256", hash]);
    assert_eq!(corrupt.status.code(), Some(1));
    assert!(corrupt.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&corrupt.stderr).unwrap()["error"]["code"],
        "body_corrupt"
    );
    let mut relative = event.clone();
    relative["event_id"] = json!("relative");
    relative["bodies"][0]["path"] = json!("body.bin");
    submit(&dir, &["append"], relative, 2);
    let fifo = dir.path().join("fifo");
    let path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut event = event;
    event["event_id"] = json!("fifo");
    event["bodies"][0]["path"] = json!(fifo);
    submit(&dir, &["append"], event, 2);
}

#[test]
fn stdin_and_help_work_without_config_and_boolean_settings_are_strict() {
    use std::io::Write;
    use std::process::Stdio;
    let dir = tempfile::tempdir().unwrap();
    let help = run(&dir, &["--help"]);
    assert!(help.status.success());
    assert!(!dir.path().join("state").exists());
    let mut child = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args(["telemetry", "settings", "set", "--input", "-"])
        .env("HOME", dir.path())
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(setting(true).to_string().as_bytes())
        .unwrap();
    assert_eq!(
        response(child.wait_with_output().unwrap(), 0)["status"],
        "stored"
    );
    for invalid in [json!("true"), json!(1), Value::Null] {
        let mut input = setting(false);
        input["enabled"] = invalid;
        assert_eq!(
            submit(&dir, &["settings", "set"], input, 2)["status"],
            "invalid"
        );
    }
    assert_eq!(
        response(run(&dir, &["settings", "get"]), 0)["enabled"],
        true
    );
}

#[test]
fn headless_queries_do_not_initialize_storage_and_default_is_off() {
    let dir = tempfile::tempdir().unwrap();
    let settings = response(run(&dir, &["settings", "get"]), 0);
    assert_eq!(settings["enabled"], false);
    assert_eq!(settings["generation"], 0);
    assert_eq!(settings["initialized"], false);
    let listing = response(run(&dir, &["list"]), 0);
    assert_eq!(listing["initialized"], false);
    assert_eq!(listing["traces"], json!([]));
    assert!(!dir.path().join("state").exists());
}
