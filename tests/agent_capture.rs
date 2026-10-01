use saddle::telemetry::*;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
};

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new(script: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake corral");
        fs::write(&path, format!("#!/usr/bin/python3\nimport os, sys, json, time, signal\nwith open(os.environ['CALLS'], 'ab') as f: f.write(b'call\\n')\n{script}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self { dir }
    }
    fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_saddle"));
        cmd.arg("agent")
            .arg("--corral")
            .arg(self.dir.path().join("fake corral"))
            .env("HOME", self.dir.path())
            .env("XDG_STATE_HOME", self.dir.path().join("state"))
            .env("XDG_CONFIG_HOME", self.dir.path().join("config"))
            .env("SADDLE_RUNTIME_DIR", self.dir.path().join("runtime"))
            .env("CALLS", self.dir.path().join("calls"));
        cmd
    }
    fn calls(&self) -> usize {
        fs::read(self.dir.path().join("calls"))
            .unwrap_or_default()
            .split(|b| *b == b'\n')
            .filter(|s| !s.is_empty())
            .count()
    }
    fn store(&self) -> Store {
        Store::new(self.dir.path().join("state/saddle/telemetry"))
    }
    fn recording(&self, kind: &str) -> std::path::PathBuf {
        let store = self.store();
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
                    json!({"schema_version":1,"trace_id":"t","origin":"ad_hoc","label":"test"}),
                )
                .unwrap(),
            )
            .unwrap();
        store.create_dispatch(serde_json::from_value(json!({"schema_version":1,"trace_id":"t","dispatch_id":"d","kind":"implementation"})).unwrap()).unwrap();
        let mut context = json!({"schema_version":1,"trace_id":"t","dispatch_id":"d"});
        if kind == "send" {
            context["send_kind"] = json!("initial");
        }
        let path = self.dir.path().join("context.json");
        fs::write(&path, context.to_string()).unwrap();
        path
    }
    fn events(&self) -> Vec<Value> {
        self.store().events(&EventQuery::default()).unwrap()["events"]
            .as_array()
            .unwrap()
            .clone()
    }
}

fn receipt(out: &Output) -> Value {
    out.stderr
        .split(|b| *b == b'\n')
        .rev()
        .filter_map(|line| line.strip_prefix(b"saddle-telemetry: "))
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .find(|v| v["final"] == true)
        .expect("final receipt")
}

#[test]
fn reply_capture_preserves_complete_text_numeric_at_and_public_identity() {
    let f = Fixture::new(
        "print(json.dumps({'ok':True, 'name':'actual/name', 'instance':'0123456789ab', 'at':1790812345.125, 'text':'完整\\r\\n```rust\\nfn main() {}\\n```\\x00'}))",
    );
    let context = f.recording("reply");
    let out = f
        .command()
        .arg("--record-context")
        .arg(context)
        .args(["--", "reply", "requested/name"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(receipt(&out)["end"], "stored");
    let events = f.events();
    let end = events
        .iter()
        .find(|e| e["kind"] == "agent.reply.end")
        .unwrap();
    assert_eq!(end["source_auth"], "execution_observed");
    assert_eq!(end["payload"]["at"], json!(1790812345.125));
    assert_eq!(end["payload"]["name"], "actual/name");
    assert_eq!(end["payload"]["instance"], "0123456789ab");
    assert_eq!(end["payload"]["association"], "not_proven");
    assert_eq!(
        f.store()
            .body(end["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        "完整\r\n```rust\nfn main() {}\n```\0".as_bytes()
    );
    assert_eq!(f.calls(), 1);
}

#[test]
fn start_and_send_capture_actual_inputs_without_authentication_arguments_or_label_inference() {
    let f = Fixture::new(
        "if sys.argv[1] == 'start':\n    with open(os.environ['BRIEF'], 'wb') as b: b.write(b'changed after invocation')\nprint(json.dumps({'ok':True,'name':'synthetic/name-2','instance':'0123456789ab','confirmed':False,'pending':True,'merged_with_draft':True}))",
    );
    let context = f.recording("start");
    let brief = f.dir.path().join("brief.md");
    fs::write(&brief, b"original\0\xff\r\n").unwrap();
    let out = f
        .command()
        .arg("--record-context")
        .arg(&context)
        .arg("--brief-file")
        .arg(&brief)
        .env("BRIEF", &brief)
        .args([
            "--",
            "start",
            "synthetic/name",
            "--cwd",
            "/synthetic cwd",
            "--prompt",
            "actual\r\nprompt",
            "--env",
            "AUTH=never-store-this",
            "--label",
            "role=implementer",
            "--label",
            "model=declared-model",
            "--",
            "codex",
            "-m",
            "explicit-model",
            "-c",
            "model_reasoning_effort=\"high\"",
            "--api-key",
            "never-store-this",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(receipt(&out)["end"], "stored");
    let events = f.events();
    let begin = events
        .iter()
        .find(|e| e["kind"] == "agent.start.begin")
        .unwrap();
    let snap = events
        .iter()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap();
    assert_eq!(
        begin["payload"]["explicit_parameters"],
        json!({"model":"explicit-model","effort":"high"})
    );
    assert_eq!(
        begin["payload"]["labels"],
        json!({"role":"implementer","model":"declared-model"})
    );
    assert_eq!(begin["payload"]["cwd"], "/synthetic cwd");
    assert_eq!(
        f.store()
            .body(begin["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"actual\r\nprompt"
    );
    assert_eq!(
        f.store()
            .body(snap["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"original\0\xff\r\n"
    );
    assert_eq!(begin["links"][0]["target_event_id"], snap["event_id"]);
    assert!(
        !serde_json::to_string(&events)
            .unwrap()
            .contains("never-store-this")
    );
    let mut ctx: Value = serde_json::from_slice(&fs::read(&context).unwrap()).unwrap();
    ctx["send_kind"] = json!("rework");
    ctx["previous_brief_event_id"] = snap["event_id"].clone();
    fs::write(&context, ctx.to_string()).unwrap();
    let out = f
        .command()
        .arg("--record-context")
        .arg(context)
        .arg("--brief-file")
        .arg(brief)
        .args([
            "--",
            "send",
            "synthetic/name",
            "followup\nfragment",
            "--after",
            "other/name",
            "--timeout",
            "2",
        ])
        .output()
        .unwrap();
    assert_eq!(receipt(&out)["end"], "stored");
    let events = f.events();
    let begin = events
        .iter()
        .find(|e| e["kind"] == "agent.send.begin")
        .unwrap();
    assert_eq!(begin["payload"]["send_kind"], "rework");
    assert_eq!(
        f.store()
            .body(begin["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"followup\nfragment"
    );
    let end = events
        .iter()
        .find(|e| e["kind"] == "agent.send.end")
        .unwrap();
    assert_eq!(end["payload"]["confirmed"], false);
    assert_eq!(end["payload"]["pending"], true);
    assert_eq!(end["payload"]["merged_with_draft"], true);
    let snapshot = events
        .iter()
        .rev()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap();
    assert_eq!(snapshot["links"][0]["relation"], "supersedes");
    assert_eq!(f.calls(), 2);
}

#[test]
fn timeout_reaps_only_the_direct_client_and_reports_execution_as_unknown() {
    let f = Fixture::new(
        "signal.signal(signal.SIGTERM, signal.SIG_IGN)\nwith open(os.environ['CLIENT_PID'], 'w') as p: p.write(str(os.getpid()))\nos.write(1, b'partial\\x00\\xff')\nwhile True: time.sleep(0.01)",
    );
    let context = f.recording("reply");
    let pidfile = f.dir.path().join("client-pid");
    let started = std::time::Instant::now();
    let out = f
        .command()
        .arg("--record-context")
        .arg(context)
        .args(["--timeout-ms", "500", "--", "reply", "synthetic/name"])
        .env("CLIENT_PID", &pidfile)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(124),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    assert_eq!(out.stdout, b"partial\0\xff");
    assert_eq!(receipt(&out)["executed"], true);
    assert_eq!(receipt(&out)["outcome"]["kind"], "timed_out");
    assert_eq!(f.calls(), 1);
    let pid: i32 = fs::read_to_string(pidfile).unwrap().parse().unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "direct client must be reaped"
    );
}

#[test]
fn host_final_receipt_follows_child_lookalikes_even_without_recording() {
    let f = Fixture::new(
        "os.write(2, b'saddle-telemetry: {\"final\":true,\"executed\":false}\\n')\nsys.exit(127)",
    );
    let out = f
        .command()
        .args(["--", "reply", "synthetic/name"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(127));
    assert_eq!(receipt(&out)["executed"], true);
    assert_eq!(
        receipt(&out)["outcome"],
        json!({"kind":"exited","exit_code":127})
    );
    assert_eq!(f.calls(), 1);
}

#[test]
fn unrecorded_call_preserves_argv_streams_exit_and_does_not_initialize_storage() {
    use std::os::unix::ffi::OsStringExt;
    let f = Fixture::new(
        "assert sys.argv[1:4] == ['send', 'synthetic/name', 'message \\n --not-an-option']\nassert os.fsencode(sys.argv[4]) == b'\\xff'\nassert not any(k.startswith('SADDLE_TELEMETRY') for k in os.environ)\nos.write(1, b'out\\x00\\xff\\r\\n')\nos.write(2, b'err\\x00\\xfe')\nsys.exit(7)",
    );
    let out = f
        .command()
        .arg("--brief-file")
        .arg(f.dir.path().join("unread-brief"))
        .args(["--", "send", "synthetic/name", "message \n --not-an-option"])
        .arg(std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"out\0\xff\r\n");
    assert!(out.stderr.starts_with(b"err\0\xfe\nsaddle-telemetry: "));
    assert_eq!(receipt(&out)["begin"], "not_requested");
    assert_eq!(f.calls(), 1);
    assert!(!f.dir.path().join("state").exists());
}

#[test]
fn context_validation_precedes_execution_even_when_recording_is_disabled() {
    let f = Fixture::new("print('{\"ok\":true}')");
    let path = f.recording("send");
    f.store()
        .set_recording(
            None,
            SettingInput {
                schema_version: 1,
                enabled: false,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let invalid = [
        json!({"generation":1}),
        json!({"operation_id":"forged"}),
        json!({"producer":"forged"}),
        json!({"schema_version":2}),
        json!({"send_kind":"guess"}),
        json!({"dispatch_id":"missing"}),
        json!({"basis_event_ids":["missing"]}),
        json!({"decision_event_id":"missing"}),
        json!({"previous_brief_event_id":"missing"}),
        json!({"trace_id":"other"}),
    ];
    for patch in invalid {
        let mut context = original.clone();
        context
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        fs::write(&path, context.to_string()).unwrap();
        let out = f
            .command()
            .arg("--record-context")
            .arg(&path)
            .args(["--", "send", "synthetic/name", "text"])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(125),
            "{context}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(receipt(&out)["executed"], false);
    }
    assert_eq!(f.calls(), 0);
    fs::write(&path, original.to_string()).unwrap();
    let out = f
        .command()
        .arg("--record-context")
        .arg(path)
        .arg("--brief-file")
        .arg(f.dir.path().join("unreadable"))
        .args(["--", "send", "synthetic/name", "text"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(receipt(&out)["begin"], "disabled");
    assert_eq!(f.calls(), 1);
    assert!(!f.dir.path().join("state/saddle/telemetry/tmp").exists());
}

#[test]
fn unavailable_or_uninitialized_storage_never_blocks_or_replays_business() {
    for corrupt in [false, true] {
        let f = Fixture::new("os.write(1,b'unchanged'); sys.exit(8)");
        let path = f.dir.path().join("context.json");
        fs::write(
            &path,
            json!({"schema_version":1,"trace_id":"unverified","dispatch_id":"unverified"})
                .to_string(),
        )
        .unwrap();
        if corrupt {
            fs::write(f.dir.path().join("state"), b"not a directory").unwrap();
        }
        let out = f
            .command()
            .arg("--record-context")
            .arg(path)
            .args(["--", "reply", "synthetic/name"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(8));
        assert_eq!(out.stdout, b"unchanged");
        assert_eq!(
            receipt(&out)["begin"],
            if corrupt { "unavailable" } else { "disabled" }
        );
        assert_eq!(receipt(&out)["executed"], true);
        assert_eq!(f.calls(), 1);
        if !corrupt {
            assert!(!f.dir.path().join("state").exists());
        }
    }
}

#[test]
fn spawn_failure_records_an_end_without_claiming_execution() {
    let f = Fixture::new("pass");
    let context = f.recording("reply");
    fs::remove_file(f.dir.path().join("fake corral")).unwrap();
    let out = f
        .command()
        .arg("--record-context")
        .arg(context)
        .args(["--", "reply", "synthetic/name"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(127));
    assert_eq!(receipt(&out)["executed"], false);
    assert_eq!(receipt(&out)["end"], "stored");
    let events = f.events();
    let end = events
        .iter()
        .find(|e| e["kind"] == "agent.reply.end")
        .unwrap();
    assert_eq!(end["payload"]["outcome"]["kind"], "spawn_failed");
    assert_eq!(f.calls(), 0);
    let out = f
        .command()
        .args(["--", "reply", "synthetic/name"])
        .output()
        .unwrap();
    assert_eq!(receipt(&out)["executed"], false);
}

#[test]
fn over_limit_and_unreadable_evidence_are_gaps_while_stdout_is_complete() {
    for large_brief in [false, true] {
        let f = Fixture::new(
            "print(json.dumps({'ok':True,'name':'synthetic/name','instance':'instance','text':'x'*(16*1024*1024+1),'at':1790812345.125}))",
        );
        let context = f.recording("reply");
        let out = f
            .command()
            .arg("--record-context")
            .arg(&context)
            .args(["--", "reply", "synthetic/name"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(
            serde_json::from_slice::<Value>(&out.stdout).unwrap()["text"]
                .as_str()
                .unwrap()
                .len(),
            16 * 1024 * 1024 + 1
        );
        assert_eq!(receipt(&out)["end"], "stored");
        let end = f
            .events()
            .into_iter()
            .find(|e| e["kind"] == "agent.reply.end")
            .unwrap();
        assert_eq!(end["bodies"], json!([]));
        assert!(
            end["payload"]["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!({"role":"reply","reason":"too_large"}))
        );
        let path = f.dir.path().join("brief");
        if large_brief {
            fs::File::create(&path)
                .unwrap()
                .set_len(MAX_BODY_BYTES + 1)
                .unwrap();
        }
        let mut ctx: Value = serde_json::from_slice(&fs::read(&context).unwrap()).unwrap();
        ctx["send_kind"] = json!("followup");
        fs::write(&context, ctx.to_string()).unwrap();
        let out = f
            .command()
            .arg("--record-context")
            .arg(context)
            .arg("--brief-file")
            .arg(path)
            .args(["--", "send", "synthetic/name", "message"])
            .output()
            .unwrap();
        assert_eq!(receipt(&out)["begin"], "stored");
        let events = f.events();
        assert!(!events.iter().any(|e| e["kind"] == "brief.snapshot"));
        let begin = events
            .iter()
            .find(|e| e["kind"] == "agent.send.begin")
            .unwrap();
        assert_eq!(
            begin["payload"]["gaps"],
            json!([{"role":"brief","reason":if large_brief {"too_large"} else {"unreadable"}}])
        );
        assert_eq!(f.calls(), 2);
    }
}

struct Running(Option<std::process::Child>);
impl Running {
    fn spawn(mut command: Command) -> Self {
        use std::os::unix::process::CommandExt;
        Self(Some(
            command
                .process_group(0)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        ))
    }
    fn signal(&self, group: bool, signal: i32) {
        let pid = self.0.as_ref().unwrap().id() as i32;
        assert_eq!(
            unsafe { libc::kill(if group { -pid } else { pid }, signal) },
            0
        );
    }
    fn finish(mut self) -> Output {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if self.0.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "owned test process did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        self.0.take().unwrap().wait_with_output().unwrap()
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
        }
    }
}
fn wait_for(path: &std::path::Path) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !path.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "missing test barrier {}",
            path.display()
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
const BARRIER: &str = "open(os.environ['READY'], 'w').close()\nwhile not os.path.exists(os.environ['RELEASE']): time.sleep(0.005)\nprint(json.dumps({'ok':True,'name':'synthetic/name','instance':'0123456789ab','confirmed':True}))";
fn barrier_command(f: &Fixture, context: &std::path::Path) -> Command {
    let mut cmd = f.command();
    cmd.arg("--record-context")
        .arg(context)
        .env("READY", f.dir.path().join("ready"))
        .env("RELEASE", f.dir.path().join("release"));
    cmd
}

#[test]
fn off_then_on_revokes_inflight_capture_but_same_value_does_not() {
    for scope in [None, Some("t")] {
        for change in [false, true] {
            let f = Fixture::new(BARRIER);
            let path = f.recording("send");
            let mut cmd = barrier_command(&f, &path);
            cmd.args(["--", "send", "synthetic/name", "message"]);
            let running = Running::spawn(cmd);
            wait_for(&f.dir.path().join("ready"));
            if change {
                f.store()
                    .set_recording(
                        scope,
                        SettingInput {
                            schema_version: 1,
                            enabled: false,
                            actor: "synthetic".into(),
                        },
                    )
                    .unwrap();
            }
            f.store()
                .set_recording(
                    scope,
                    SettingInput {
                        schema_version: 1,
                        enabled: true,
                        actor: "synthetic".into(),
                    },
                )
                .unwrap();
            fs::write(f.dir.path().join("release"), b"").unwrap();
            let out = running.finish();
            assert_eq!(out.status.code(), Some(0));
            assert_eq!(receipt(&out)["begin"], "stored");
            assert_eq!(
                receipt(&out)["end"],
                if change { "disabled" } else { "stored" }
            );
            assert_eq!(
                f.events()
                    .iter()
                    .filter(|e| e["kind"] == "agent.send.end")
                    .count(),
                usize::from(!change)
            );
            assert_eq!(f.calls(), 1);
        }
    }
}

#[test]
fn begin_lock_failure_recovers_only_pre_call_snapshot_without_replaying() {
    let f = Fixture::new(BARRIER);
    let context = f.recording("send");
    let brief = f.dir.path().join("brief");
    fs::write(&brief, b"before lock and invocation").unwrap();
    let lock = rusqlite::Connection::open(
        f.dir
            .path()
            .join("state/saddle/telemetry/telemetry.sqlite3"),
    )
    .unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut cmd = barrier_command(&f, &context);
    cmd.arg("--brief-file")
        .arg(&brief)
        .args(["--", "send", "synthetic/name", "message"]);
    let started = std::time::Instant::now();
    let running = Running::spawn(cmd);
    wait_for(&f.dir.path().join("ready"));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    assert!(
        !f.events()
            .iter()
            .any(|e| e["kind"] == "brief.snapshot" || e["kind"] == "agent.send.begin")
    );
    fs::write(&brief, b"changed before end").unwrap();
    lock.execute_batch("ROLLBACK").unwrap();
    fs::write(f.dir.path().join("release"), b"").unwrap();
    let out = running.finish();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(receipt(&out)["begin"], "unavailable");
    assert_eq!(receipt(&out)["end"], "stored");
    let events = f.events();
    let snapshot = events
        .iter()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap();
    let end = events
        .iter()
        .find(|e| e["kind"] == "agent.send.end")
        .unwrap();
    assert_eq!(snapshot["payload"]["publication_phase"], "end");
    assert_eq!(snapshot["payload"]["begin_missing"], true);
    assert_eq!(end["payload"]["begin_missing"], true);
    assert_eq!(end["links"][0]["target_event_id"], snapshot["event_id"]);
    assert_eq!(
        f.store()
            .body(snapshot["bodies"][0]["sha256"].as_str().unwrap())
            .unwrap(),
        b"before lock and invocation"
    );
    assert!(!events.iter().any(|e| e["kind"] == "agent.send.begin"));
    assert_eq!(f.calls(), 1);
}

#[test]
fn end_lock_failure_preserves_business_exit_and_never_replays() {
    let f = Fixture::new(&format!("{BARRIER}\nsys.exit(3)"));
    let context = f.recording("send");
    let mut cmd = barrier_command(&f, &context);
    cmd.args(["--", "send", "synthetic/name", "message"]);
    let running = Running::spawn(cmd);
    wait_for(&f.dir.path().join("ready"));
    let lock = rusqlite::Connection::open(
        f.dir
            .path()
            .join("state/saddle/telemetry/telemetry.sqlite3"),
    )
    .unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    fs::write(f.dir.path().join("release"), b"").unwrap();
    let out = running.finish();
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(receipt(&out)["end"], "unavailable");
    assert_eq!(f.calls(), 1);
    assert_eq!(
        f.events()
            .iter()
            .filter(|e| e["kind"] == "agent.send.end")
            .count(),
        0
    );
}

#[test]
fn int_and_term_to_wrapper_or_owned_group_reap_the_direct_client() {
    for signal in [libc::SIGINT, libc::SIGTERM] {
        for group in [false, true] {
            let f = Fixture::new(
                "signal.signal(signal.SIGINT, signal.SIG_DFL)\nwith open(os.environ['CLIENT_PID'], 'w') as p: p.write(str(os.getpid()))\nopen(os.environ['READY'], 'w').close()\nwhile True: time.sleep(0.01)",
            );
            let mut cmd = f.command();
            cmd.env("CLIENT_PID", f.dir.path().join("client-pid"))
                .env("READY", f.dir.path().join("ready"))
                .args(["--", "reply", "synthetic/name"]);
            let running = Running::spawn(cmd);
            wait_for(&f.dir.path().join("ready"));
            running.signal(group, signal);
            let out = running.finish();
            assert_eq!(
                out.status.code(),
                Some(128 + signal),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(receipt(&out)["executed"], true);
            let pid: i32 = fs::read_to_string(f.dir.path().join("client-pid"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
            assert_eq!(f.calls(), 1);
        }
    }
}

#[test]
fn inherited_stdin_and_streaming_stdout_work_before_client_exit() {
    use std::io::{Read, Write};
    let f = Fixture::new(
        "data = os.read(0,4)\nos.write(1,data)\nwhile not os.path.exists(os.environ['RELEASE']): time.sleep(0.005)",
    );
    let mut cmd = f.command();
    cmd.stdin(std::process::Stdio::piped())
        .env("RELEASE", f.dir.path().join("release"))
        .args(["--", "reply", "synthetic/name"]);
    let mut running = Running::spawn(cmd);
    let child = running.0.as_mut().unwrap();
    child.stdin.take().unwrap().write_all(b"a\0\xffb").unwrap();
    // Read in a scoped worker with a bounded receiver so a regression cannot hang the suite.
    let mut stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = [0; 4];
        stdout.read_exact(&mut bytes).unwrap();
        tx.send(bytes).unwrap();
    });
    let bytes = rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap();
    assert_eq!(bytes, b"a\0\xffb"[..]);
    fs::write(f.dir.path().join("release"), b"").unwrap();
    let out = running.finish();
    reader.join().unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn unknown_metadata_and_oversize_json_do_not_invent_facts_or_truncate_stdout() {
    let f = Fixture::new("os.write(1,b'x'*(32*1024*1024+1)); sys.exit(0)");
    let context = f.recording("start");
    let out = f
        .command()
        .arg("--record-context")
        .arg(context)
        .args([
            "--",
            "start",
            "synthetic/name",
            "--label",
            "model=declared",
            "--",
            "unknown-agent",
            "--model",
            "not-interpreted",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout.len(), 32 * 1024 * 1024 + 1);
    assert!(out.stdout.iter().all(|b| *b == b'x'));
    let events = f.events();
    let begin = events
        .iter()
        .find(|e| e["kind"] == "agent.start.begin")
        .unwrap();
    assert_eq!(begin["payload"]["explicit_parameters"], Value::Null);
    assert_eq!(begin["payload"]["labels"], json!({"model":"declared"}));
    let end = events
        .iter()
        .find(|e| e["kind"] == "agent.start.end")
        .unwrap();
    assert_eq!(end["payload"]["name"], Value::Null);
    assert!(
        end["payload"]["gaps"]
            .as_array()
            .unwrap()
            .contains(&json!({"role":"name","reason":"too_large"}))
    );
    assert_eq!(receipt(&out)["end"], "stored");
    assert_eq!(f.calls(), 1);
}

#[test]
fn cancelling_client_group_does_not_stop_an_independently_hosted_process() {
    struct Detached(i32);
    impl Drop for Detached {
        fn drop(&mut self) {
            unsafe {
                libc::kill(self.0, libc::SIGKILL);
            }
        }
    }
    let f = Fixture::new(
        "import subprocess\np = subprocess.Popen(['/usr/bin/python3','-c','import time; time.sleep(20)'],start_new_session=True,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)\nwith open(os.environ['DETACHED_PID'],'w') as file: file.write(str(p.pid))\nopen(os.environ['READY'],'w').close()\nwhile True: time.sleep(0.005)",
    );
    let mut cmd = f.command();
    cmd.env("DETACHED_PID", f.dir.path().join("detached-pid"))
        .env("READY", f.dir.path().join("ready"))
        .args(["--", "reply", "synthetic/name"]);
    let running = Running::spawn(cmd);
    wait_for(&f.dir.path().join("ready"));
    let pid = fs::read_to_string(f.dir.path().join("detached-pid"))
        .unwrap()
        .parse()
        .unwrap();
    let detached = Detached(pid);
    running.signal(true, libc::SIGTERM);
    let out = running.finish();
    assert_eq!(out.status.code(), Some(143));
    assert_eq!(
        unsafe { libc::kill(detached.0, 0) },
        0,
        "detached hosted process must survive client cancellation"
    );
    drop(detached);
}

#[test]
fn known_cross_trace_and_wrong_kind_links_are_rejected_before_calling_corral() {
    let f = Fixture::new("pass");
    let context = f.recording("send");
    let store = f.store();
    store
        .create_trace(
            serde_json::from_value(
                json!({"schema_version":1,"trace_id":"other","origin":"ad_hoc","label":"other"}),
            )
            .unwrap(),
        )
        .unwrap();
    let body = f.dir.path().join("evidence");
    fs::write(&body, b"synthetic evidence").unwrap();
    for (id, trace) in [("same", "t"), ("cross", "other")] {
        store.append(serde_json::from_value(json!({"schema_version":1,"event_id":id,"trace_id":trace,"kind":"requirement.recorded","observed_at":null,"producer":"synthetic","evidence_kind":"controller_statement","payload":{"declared_speaker":"synthetic","acquisition":"user_supplied_file","declared_at":null},"links":[],"bodies":[{"role":"text","path":body}]})).unwrap()).unwrap();
    }
    for patch in [
        json!({"basis_event_ids":["cross"]}),
        json!({"decision_event_id":"same"}),
    ] {
        let mut ctx =
            json!({"schema_version":1,"trace_id":"t","dispatch_id":"d","send_kind":"initial"});
        ctx.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        fs::write(&context, ctx.to_string()).unwrap();
        let out = f
            .command()
            .arg("--record-context")
            .arg(&context)
            .args(["--", "send", "synthetic/name", "text"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(125));
        assert_eq!(receipt(&out)["executed"], false);
    }
    assert_eq!(f.calls(), 0);
}

#[test]
fn stalled_output_reader_does_not_prevent_cancellation_or_reaping() {
    for timeout in [false, true] {
        let f = Fixture::new(
            "with open(os.environ['CLIENT_PID'],'w') as p: p.write(str(os.getpid()))\nopen(os.environ['READY'],'w').close()\nwhile True: os.write(1,b'x'*65536)",
        );
        let mut cmd = f.command();
        cmd.env("CLIENT_PID", f.dir.path().join("client-pid"))
            .env("READY", f.dir.path().join("ready"));
        if timeout {
            cmd.args(["--timeout-ms", "500"]);
        }
        cmd.args(["--", "reply", "synthetic/name"]);
        let running = Running::spawn(cmd);
        wait_for(&f.dir.path().join("ready"));
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !timeout {
            running.signal(false, libc::SIGTERM);
        }
        // Deliberately wait for wrapper exit without consuming stdout first.
        let out = running.finish();
        assert_eq!(out.status.code(), Some(if timeout { 124 } else { 143 }));
        let pid = fs::read_to_string(f.dir.path().join("client-pid"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(f.calls(), 1);
    }
}

#[test]
fn terminal_receipt_cannot_extend_timeout_when_stderr_is_already_full() {
    use std::os::{
        fd::{FromRawFd, OwnedFd},
        unix::process::CommandExt,
    };
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let flags = unsafe { libc::fcntl(fds[1], libc::F_GETFL) };
    assert_eq!(
        unsafe { libc::fcntl(fds[1], libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let padding = [b'x'; 512];
    while unsafe { libc::write(fds[1], padding.as_ptr().cast(), padding.len()) } > 0 {}
    assert_eq!(unsafe { libc::fcntl(fds[1], libc::F_SETFL, flags) }, 0);
    let f = Fixture::new("sys.exit(3)");
    let child = f
        .command()
        .args(["--timeout-ms", "500", "--", "reply", "synthetic/name"])
        .process_group(0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::from(writer))
        .spawn()
        .unwrap();
    let out = Running(Some(child)).finish();
    assert_eq!(
        out.status.code(),
        Some(3),
        "receipt IO cannot change known business exit"
    );
    assert_eq!(f.calls(), 1);
}
