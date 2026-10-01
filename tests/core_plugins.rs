use serde_json::Value;
use std::process::Command;

fn paired(stderr: &[u8]) -> Value {
    assert!(stderr.ends_with(b"\n"), "incomplete stderr: {stderr:?}");
    let lines: Vec<_> = stderr.split(|b| *b == b'\n').collect();
    let parse = |line: &[u8]| -> Value {
        serde_json::from_slice(line.strip_prefix(b"saddle-plugin: ").expect("host prefix")).unwrap()
    };
    let first = parse(lines[0]);
    let last = parse(lines[lines.len() - 2]);
    assert_eq!(first["schema_version"], 1);
    assert_eq!(first["final"], false);
    assert_eq!(last["schema_version"], 1);
    assert_eq!(last["final"], true);
    assert_eq!(first["call_id"], last["call_id"]);
    assert_eq!(
        uuid::Uuid::parse_str(first["call_id"].as_str().unwrap())
            .unwrap()
            .get_version_num(),
        4
    );
    last
}

#[test]
fn headless_run_rejects_unknown_plugin_with_paired_receipt_without_initializing_storage() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args(["plugin", "run", "test.core", "route"])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(125));
    assert!(out.stdout.is_empty());
    let receipt = paired(&out.stderr);
    assert_eq!(receipt["executed"], false);
    assert_eq!(receipt["error"]["code"], "unknown_plugin");
    assert_eq!(receipt["begin"], "not_requested");
    assert!(receipt["outcome"].is_null());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn external_registry_writes_preserve_unknown_core_switches() {
    use saddle::plugins::registry::Registry;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plugins.toml");
    std::fs::write(&path, "version=1\n[[plugins]]\nid='test.external'\ndirectory='/synthetic/plugin'\nenabled=false\n[core.future]\nenabled=true\n").unwrap();
    let mut registry = Registry::open(path.clone());
    assert!(registry.error.is_none());
    registry.enabled("test.external", true).unwrap();
    let saved: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        saved
            .get("core")
            .and_then(|c| c.get("future"))
            .and_then(|c| c.get("enabled"))
            .and_then(toml::Value::as_bool),
        Some(true)
    );
}

use saddle_core_plugin::{Call, Completion, CorePlugin, Manifest, Operation};
use std::{
    fs,
    io::Write,
    os::fd::AsRawFd,
    process::{Output, Stdio},
};

struct Fake;
static FAKE: Fake = Fake;
static CATALOG: saddle::plugins::core::Catalog = &[&FAKE];
static MANIFEST: Manifest = Manifest {
    id: "test.core",
    name: "Synthetic core",
    version: "1",
    commands: &[
        saddle_core_plugin::Command {
            name: "route",
            capture: Some(Operation::Route),
        },
        saddle_core_plugin::Command {
            name: "plain",
            capture: None,
        },
    ],
    resources: &[],
    setup_note: "Synthetic setup note",
    setup_files: &[],
};
fn route_begin(model: &str) -> saddle_core_plugin::Begin {
    saddle_core_plugin::Begin::Route(saddle_core_plugin::RouteBegin {
        router_model: model.into(),
        router_version: Some("test-v1".into()),
        rules_version: None,
        summary: b"summary\0\xff".to_vec(),
        request: b"{\"synthetic\":true}".to_vec(),
    })
}
impl CorePlugin for Fake {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }
    fn run(&self, call: Call<'_>) -> Completion {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::var_os("TEST_CALLS").unwrap())
            .unwrap()
            .write_all(b"call\n")
            .unwrap();
        let mut bytes = vec![];
        call.stdin.read_to_end(&mut bytes).unwrap();
        let mode = std::env::var("TEST_MODE").unwrap_or_default();
        if mode == "panic_before" {
            panic!("synthetic plugin panic before begin");
        }
        if mode == "disable_before_begin" {
            set_recording(None, false);
        }
        if mode == "enable_before_begin" {
            set_recording(None, true);
        }
        if mode != "no_begin" {
            call.recorder.begin(route_begin(if mode == "bad_begin" {
                ""
            } else {
                "synthetic-router"
            }));
        }
        match mode.as_str() {
            "double" => call.recorder.begin(route_begin("ignored-second-begin")),
            "panic" => panic!("synthetic plugin panic"),
            "flip_global" | "flip_trace" | "recover_flip" => {
                let trace = if mode == "flip_trace" {
                    Some("t")
                } else {
                    None
                };
                set_recording(trace, false);
                set_recording(trace, true);
            }
            "disable_plugin" => {
                let path = std::path::PathBuf::from(std::env::var_os("XDG_CONFIG_HOME").unwrap())
                    .join("saddle/plugins.toml");
                saddle::plugins::registry::Registry::with_reserved(path, ["test.core"])
                    .core_enabled("test.core", false)
                    .unwrap();
            }
            "final_full" => fill_pipe(libc::STDERR_FILENO),
            // Simulate a library diagnostic that bypasses the plugin's returned output.
            "stderr_no_lf" => std::io::stderr()
                .write_all(b"synthetic library diagnostic without LF")
                .unwrap(),
            _ => (),
        }
        if mode == "recover" || mode == "recover_flip" {
            let store = Store::from_environment().unwrap();
            let before = store.events(&EventQuery::default()).unwrap();
            assert!(
                !before["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["kind"] == "brief.snapshot" || e["kind"] == "route.begin")
            );
            let state = std::path::PathBuf::from(std::env::var_os("XDG_STATE_HOME").unwrap());
            rusqlite::Connection::open(state.join("saddle/telemetry/telemetry.sqlite3"))
                .unwrap()
                .execute_batch("DROP TRIGGER fail_begin")
                .unwrap();
            fs::write(
                std::env::var_os("TEST_BRIEF").unwrap(),
                b"changed after begin",
            )
            .unwrap();
        }
        let end = match mode.as_str() {
            "no_end" => None,
            "missing" => Some(saddle_core_plugin::End::Route(
                saddle_core_plugin::RouteEnd {
                    response: saddle_core_plugin::Captured::Missing(
                        saddle_core_plugin::Missing::TooLarge,
                    ),
                    suggestion: saddle_core_plugin::Captured::Missing(
                        saddle_core_plugin::Missing::Unrecognized,
                    ),
                },
            )),
            _ => Some(saddle_core_plugin::End::Route(
                saddle_core_plugin::RouteEnd {
                    response: saddle_core_plugin::Captured::Bytes(b"{\"response\":true}".to_vec()),
                    suggestion: saddle_core_plugin::Captured::Bytes(b"suggestion\0\xff".to_vec()),
                },
            )),
        };
        Completion {
            exit_code: match mode.as_str() {
                "invalid_code" => 255,
                "success" | "no_end" => 0,
                _ => 7,
            },
            stdout: bytes,
            end,
        }
    }
}
fn set_recording(trace: Option<&str>, enabled: bool) {
    Store::from_environment()
        .unwrap()
        .set_recording(
            trace,
            SettingInput {
                schema_version: 1,
                enabled,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
}
fn fill_pipe(fd: i32) {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let filler = [b'x'; 512];
    loop {
        let n = unsafe { libc::write(fd, filler.as_ptr().cast(), filler.len()) };
        if n < 0 {
            assert_eq!(
                std::io::Error::last_os_error().kind(),
                std::io::ErrorKind::WouldBlock
            );
            break;
        }
    }
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFL, flags) }, 0);
}

// Subprocess-only seam: no production test plugin or environment override is installed.
#[test]
fn plugin_driver() {
    let Some(args) = std::env::var_os("TEST_CORE_ARGS") else {
        return;
    };
    let output = fs::File::create(std::env::var_os("TEST_OUTPUT").unwrap()).unwrap();
    assert_eq!(
        unsafe { libc::dup2(output.as_raw_fd(), libc::STDOUT_FILENO) },
        libc::STDOUT_FILENO
    );
    let args: Vec<String> = serde_json::from_str(args.to_str().unwrap()).unwrap();
    let stdout_before = unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL) };
    let stderr_before = unsafe { libc::fcntl(libc::STDERR_FILENO, libc::F_GETFL) };
    if std::env::var("TEST_MODE").as_deref() == Ok("stdout_closed") {
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        unsafe {
            libc::close(fds[0]);
            libc::dup2(fds[1], libc::STDOUT_FILENO);
            libc::close(fds[1]);
        }
    }
    let code = saddle::plugins::cli::run(
        &args.into_iter().map(Into::into).collect::<Vec<_>>(),
        CATALOG,
    );
    if std::env::var("TEST_MODE").as_deref() != Ok("stdout_closed") {
        assert_eq!(
            unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL) } & libc::O_NONBLOCK,
            stdout_before & libc::O_NONBLOCK
        );
    }
    assert_eq!(
        unsafe { libc::fcntl(libc::STDERR_FILENO, libc::F_GETFL) } & libc::O_NONBLOCK,
        stderr_before & libc::O_NONBLOCK
    );
    std::process::exit(code);
}
struct Fixture {
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    fn registry(&self, body: &str) {
        fs::create_dir_all(self.dir.path().join("config/saddle")).unwrap();
        fs::write(self.dir.path().join("config/saddle/plugins.toml"), body).unwrap();
    }
    fn enable(&self) {
        self.registry("version=1\nplugins=[]\n[core.'test.core']\nenabled=true\n");
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args(["--exact", "plugin_driver", "--nocapture"])
            .env("TEST_CORE_ARGS", serde_json::to_string(args).unwrap())
            .env("TEST_OUTPUT", self.dir.path().join("output"))
            .env("TEST_CALLS", self.dir.path().join("calls"))
            .env("HOME", self.dir.path())
            .env("XDG_CONFIG_HOME", self.dir.path().join("config"))
            .env("XDG_STATE_HOME", self.dir.path().join("state"))
            .stdout(Stdio::null());
        cmd
    }
    fn invoke(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn calls(&self) -> usize {
        fs::read(self.dir.path().join("calls"))
            .unwrap_or_default()
            .len()
            / 5
    }
}
#[test]
fn injected_plugin_runs_once_and_preserves_stdin_stdout_and_business_code() {
    let f = Fixture::new();
    f.enable();
    let input = f.dir.path().join("input");
    fs::write(&input, b"synthetic\0\xff\r\n").unwrap();
    let out = f
        .command(&[
            "run",
            "test.core",
            "route",
            "--brief-file",
            "/does/not/exist",
        ])
        .stdin(fs::File::open(input).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(f.dir.path().join("output")).unwrap(),
        b"synthetic\0\xff\r\n"
    );
    let r = paired(&out.stderr);
    assert_eq!(r["executed"], true);
    assert_eq!(
        r["outcome"],
        serde_json::json!({"kind":"exited","exit_code":7})
    );
    assert_eq!(r["begin"], "not_requested");
    assert_eq!(r["end"], "not_requested");
    assert_eq!(f.calls(), 1);
    assert!(!f.dir.path().join("state").exists());
}

use saddle::telemetry::{EventQuery, SettingInput, Store};
use serde_json::json;
impl Fixture {
    fn store(&self) -> Store {
        Store::new(self.dir.path().join("state/saddle/telemetry"))
    }
    fn recording(&self) -> std::path::PathBuf {
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
        let path = self.dir.path().join("context.json");
        fs::write(
            &path,
            r#"{"schema_version":1,"trace_id":"t","dispatch_id":"d"}"#,
        )
        .unwrap();
        path
    }
    fn events(&self) -> Vec<Value> {
        self.store().events(&EventQuery::default()).unwrap()["events"]
            .as_array()
            .unwrap()
            .clone()
    }
}
#[test]
fn recorder_saves_original_bodies_and_atomic_brief_with_host_owned_source() {
    let f = Fixture::new();
    f.enable();
    let context = f.recording();
    let brief = f.dir.path().join("brief");
    fs::write(&brief, b"brief\0\xff\r\n").unwrap();
    let out = f.invoke(&[
        "run",
        "test.core",
        "route",
        "--record-context",
        context.to_str().unwrap(),
        "--brief-file",
        brief.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r = paired(&out.stderr);
    assert_eq!(r["begin"], "stored", "{r}");
    assert_eq!(r["end"], "stored");
    let events = f.events();
    let begin = events.iter().find(|e| e["kind"] == "route.begin").unwrap();
    let end = events.iter().find(|e| e["kind"] == "route.end").unwrap();
    let snapshot = events
        .iter()
        .find(|e| e["kind"] == "brief.snapshot")
        .unwrap();
    for e in [begin, end, snapshot] {
        assert_eq!(e["producer"], "saddle.plugin.test.core");
        assert_eq!(e["evidence_kind"], "execution_observed");
        assert_eq!(e["source_auth"], "execution_observed");
        assert_eq!(e["operation_id"], r["operation_id"]);
    }
    for (e, role, bytes) in [
        (begin, "summary", b"summary\0\xff".as_slice()),
        (begin, "request", b"{\"synthetic\":true}"),
        (end, "response", b"{\"response\":true}"),
        (end, "suggestion", b"suggestion\0\xff"),
        (snapshot, "brief", b"brief\0\xff\r\n"),
    ] {
        let body = e["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["role"] == role)
            .unwrap();
        assert_eq!(
            f.store().body(body["sha256"].as_str().unwrap()).unwrap(),
            bytes
        );
    }
    assert_eq!(begin["links"][0]["target_event_id"], snapshot["event_id"]);
    assert_eq!(
        end["payload"]["outcome"],
        json!({"kind":"exited","exit_code":7})
    );
    assert_eq!(f.calls(), 1);
}

#[test]
fn run_refusals_never_enter_plugin_and_always_pair_boundaries() {
    for (registry, args, code) in [
        (
            "version=1\nplugins=[]\n",
            vec!["run", "test.core", "route"],
            "plugin_disabled",
        ),
        (
            "version=1\nplugins=[]\n[core.'test.core']\nenabled=true",
            vec!["run", "test.core", "unknown"],
            "unknown_command",
        ),
        (
            "version=1\nplugins=[]\n[core.'test.core']\nenabled=true",
            vec!["run", "test.core", "plain", "--record-context", "/absent"],
            "capture_not_supported",
        ),
        (
            "broken = [",
            vec!["run", "test.core", "route"],
            "registry_unavailable",
        ),
        (
            "version=1\n[[plugins]]\nid='test.core'\ndirectory='/synthetic'\nenabled=false\n[core.'test.core']\nenabled=true",
            vec!["run", "test.core", "route"],
            "plugin_conflict",
        ),
        (
            "version=1\n[[plugins]]\nid='external'\ndirectory='/synthetic'\nenabled=false",
            vec!["run", "external", "route"],
            "external_plugin",
        ),
        (
            "version=1\nplugins=[]",
            vec!["run", "test.core"],
            "invalid_arguments",
        ),
        (
            "version=1\nplugins=[]",
            vec!["run", "test.core", "route", "--brief-file", "relative"],
            "absolute_path_required",
        ),
        (
            "version=1\nplugins=[]",
            vec!["run", "test.core", "route", "--", "extra"],
            "invalid_option",
        ),
        (
            "version=1\nplugins=[]",
            vec![
                "--config",
                "/tmp/a",
                "--config",
                "/tmp/b",
                "run",
                "test.core",
                "route",
            ],
            "duplicate_option",
        ),
        (
            "version=1\nplugins=[]",
            vec![
                "run",
                "test.core",
                "route",
                "--brief-file",
                "/a",
                "--brief-file",
                "/b",
            ],
            "duplicate_option",
        ),
    ] {
        let f = Fixture::new();
        f.registry(registry);
        let out = f.invoke(&args);
        assert_eq!(out.status.code(), Some(125), "{args:?}");
        assert_eq!(out.stderr.iter().filter(|b| **b == b'\n').count(), 2);
        let r = paired(&out.stderr);
        assert_eq!(r["error"]["code"], code, "{args:?}: {r}");
        assert_eq!(r["executed"], false);
        assert_eq!(r["begin"], "not_requested");
        assert_eq!(r["end"], "not_requested");
        assert!(r["outcome"].is_null() && r["operation_id"].is_null());
        assert_eq!(f.calls(), 0);
        assert!(!f.dir.path().join("state").exists());
    }
}

#[test]
fn status_is_read_only_and_explicit_config_locates_only_the_registry() {
    let f = Fixture::new();
    let missing = f.dir.path().join("not-created/config.toml");
    for args in [
        vec!["status"],
        vec!["--config", missing.to_str().unwrap(), "status", "test.core"],
    ] {
        let out = f.invoke(&args);
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stderr.is_empty());
        let status: Value =
            serde_json::from_slice(&fs::read(f.dir.path().join("output")).unwrap()).unwrap();
        assert_eq!(status["core"][0]["state"], "disabled");
        assert_eq!(status["core"][0]["commands"][0]["capture"], "route");
        assert_eq!(status["core"][0]["setup_note"], "Synthetic setup note");
        assert!(status["core"][0].get("installed").is_none());
        assert_eq!(fs::read_dir(f.dir.path()).unwrap().count(), 1); // Only driver output.
    }
    let config = f.dir.path().join("selected");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("plugins.toml"),
        "version=1\nplugins=[]\n[core.'test.core']\nenabled=true",
    )
    .unwrap();
    let out = f.invoke(&[
        "--config",
        config.join("anything.toml").to_str().unwrap(),
        "run",
        "test.core",
        "route",
    ]);
    assert_eq!(
        out.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(f.invoke(&["status", "unknown"]).status.code(), Some(2));
    f.registry("broken=[");
    assert_eq!(f.invoke(&["status"]).status.code(), Some(1));
    assert!(!f.dir.path().join("state").exists());
}

#[test]
fn management_switches_preserve_external_entries_and_enforce_reserved_ids_and_baseline() {
    use saddle::plugins::{
        Manager,
        core::State,
        registry::{Manifest as ExternalManifest, Registry},
    };
    let f = Fixture::new();
    let path = f.dir.path().join("plugins.toml");
    let mut manager = Manager::with_core(path.clone(), CATALOG);
    assert_eq!(manager.core_catalog().len(), 1);
    assert_eq!(manager.core_state("test.core"), Some(State::Disabled));
    let mut stale = Registry::with_reserved(path.clone(), ["test.core"]);
    manager.core_enabled("test.core", true).unwrap();
    assert_eq!(manager.core_state("test.core"), Some(State::Enabled));
    assert!(stale.core_enabled("test.core", false).is_err());
    stale.refresh().unwrap();
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    assert!(
        stale
            .core_enabled("test.core", false)
            .unwrap_err()
            .to_string()
            .contains("busy")
    );
    assert!(Registry::open(path.clone()).core["test.core"].enabled);
    drop(lock);
    stale.core_enabled("test.core", false).unwrap();
    assert!(!Registry::open(path.clone()).core["test.core"].enabled);
    let dir = f.dir.path().join("external");
    fs::create_dir(&dir).unwrap();
    std::os::unix::fs::symlink("/bin/echo", dir.join("entry")).unwrap();
    let manifest: ExternalManifest = serde_json::from_value(json!({"manifest_version":1,"id":"test.core","name":"Synthetic","version":"1","protocol_major":1,"executable":"entry","args":[],"required_capabilities":[]})).unwrap();
    fs::write(dir.join("plugin.toml"), toml::to_string(&manifest).unwrap()).unwrap();
    assert!(
        stale
            .add(&dir, &manifest)
            .unwrap_err()
            .to_string()
            .contains("reserved by a built-in plugin")
    );
    // A legacy binary can drop the optional core table; the new host defaults safely off.
    fs::write(&path, "version=1\nplugins=[]").unwrap();
    stale.refresh().unwrap();
    assert!(stale.core.is_empty());
    assert_eq!(
        saddle::plugins::core::state(&stale, "test.core"),
        State::Disabled
    );
    fs::write(&path, format!("version=1\n[[plugins]]\nid='test.core'\ndirectory='{}'\nenabled=false\n[core.'test.core']\nenabled=true", dir.display())).unwrap();
    stale.refresh().unwrap();
    assert_eq!(
        saddle::plugins::core::state(&stale, "test.core"),
        State::Conflict
    );
    assert!(stale.core_enabled("test.core", true).is_err());
    stale.enabled("test.core", true).unwrap(); // Existing external entries retain their operations.
    assert!(Registry::open(path).entries[0].enabled);
}

#[test]
fn preflight_states_are_stable_and_not_reached_is_only_for_recordable_calls() {
    for (setup, mode, expected) in [
        ("none", "no_begin", "not_requested"),
        ("none", "normal", "not_requested"),
        ("uninitialized", "normal", "disabled"),
        ("disabled", "enable_before_begin", "disabled"),
        ("disabled", "no_begin", "disabled"),
        ("disabled", "normal", "disabled"),
        ("unavailable", "no_begin", "unavailable"),
        ("unavailable", "normal", "unavailable"),
        ("ready", "no_begin", "not_reached"),
        ("ready", "bad_begin", "unavailable"),
        ("ready", "disable_before_begin", "disabled"),
    ] {
        let f = Fixture::new();
        f.enable();
        let path = if setup == "none" {
            None
        } else if setup == "uninitialized" {
            let path = f.dir.path().join("context");
            fs::write(
                &path,
                r#"{"schema_version":1,"trace_id":"t","dispatch_id":"d"}"#,
            )
            .unwrap();
            Some(path)
        } else {
            Some(f.recording())
        };
        if setup == "disabled" {
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
        }
        if setup == "unavailable" {
            fs::remove_file(
                f.dir
                    .path()
                    .join("state/saddle/telemetry/telemetry.sqlite3"),
            )
            .unwrap();
            fs::create_dir(
                f.dir
                    .path()
                    .join("state/saddle/telemetry/telemetry.sqlite3"),
            )
            .unwrap();
        }
        let mut args = vec![
            "run",
            "test.core",
            "route",
            "--brief-file",
            "/does/not/exist",
        ];
        if let Some(path) = &path {
            args.extend(["--record-context", path.to_str().unwrap()]);
        }
        let out = f.command(&args).env("TEST_MODE", mode).output().unwrap();
        assert_eq!(out.status.code(), Some(7));
        let r = paired(&out.stderr);
        assert_eq!(r["begin"], expected, "{setup}/{mode}: {r}");
        assert_eq!(r["end"], expected);
        assert!(r["operation_id"].is_null());
        assert_eq!(f.calls(), 1);
        if setup == "none" || setup == "uninitialized" {
            assert!(!f.dir.path().join("state").exists());
        }
    }
}

#[test]
fn panic_and_illegal_codes_produce_unknown_end_without_replaying() {
    for (mode, code, begin, outcome) in [
        ("panic", 126, "stored", json!({"kind":"unknown"})),
        ("invalid_code", 126, "stored", json!({"kind":"unknown"})),
        (
            "panic_before",
            126,
            "not_reached",
            json!({"kind":"unknown"}),
        ),
        (
            "success",
            0,
            "stored",
            json!({"kind":"exited","exit_code":0}),
        ),
    ] {
        let f = Fixture::new();
        f.enable();
        let context = f.recording();
        let out = f
            .command(&[
                "run",
                "test.core",
                "route",
                "--record-context",
                context.to_str().unwrap(),
            ])
            .env("TEST_MODE", mode)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(code));
        let r = paired(&out.stderr);
        assert_eq!(r["executed"], true);
        assert_eq!(r["begin"], begin);
        assert_eq!(r["outcome"], outcome);
        if begin == "stored" {
            assert_eq!(r["end"], "stored");
            let events = f.events();
            let end = events.iter().find(|e| e["kind"] == "route.end").unwrap();
            assert_eq!(end["payload"]["outcome"], outcome);
            if code == 126 {
                assert!(end["bodies"].as_array().unwrap().is_empty());
            }
        } else {
            assert_eq!(r["end"], "not_reached");
        }
        assert_eq!(f.calls(), 1);
    }
}

#[test]
fn capture_generation_cannot_be_revived_but_plugin_disable_does_not_cancel_current_call() {
    for mode in ["flip_global", "flip_trace", "disable_plugin"] {
        let f = Fixture::new();
        f.enable();
        let context = f.recording();
        let args = [
            "run",
            "test.core",
            "route",
            "--record-context",
            context.to_str().unwrap(),
        ];
        let out = f.command(&args).env("TEST_MODE", mode).output().unwrap();
        assert_eq!(out.status.code(), Some(7));
        let r = paired(&out.stderr);
        assert_eq!(r["begin"], "stored");
        assert_eq!(
            r["end"],
            if mode == "disable_plugin" {
                "stored"
            } else {
                "disabled"
            }
        );
        let events = f.events();
        assert_eq!(
            events.iter().filter(|e| e["kind"] == "route.end").count(),
            usize::from(mode == "disable_plugin")
        );
        assert_eq!(f.calls(), 1);
        if mode == "disable_plugin" {
            let next = f.invoke(&args);
            assert_eq!(next.status.code(), Some(125));
            assert_eq!(paired(&next.stderr)["error"]["code"], "plugin_disabled");
            assert_eq!(f.calls(), 1);
        }
    }
}

#[test]
fn failed_begin_recovers_original_brief_and_links_only_with_original_generation() {
    for mode in ["recover", "recover_flip"] {
        let f = Fixture::new();
        f.enable();
        let context = f.recording();
        let brief = f.dir.path().join("brief");
        fs::write(&brief, b"original brief\0\xff").unwrap();
        let body = f.dir.path().join("requirement");
        fs::write(&body, b"synthetic requirement").unwrap();
        f.store().append(serde_json::from_value(json!({"schema_version":1,"event_id":"basis","trace_id":"t","kind":"requirement.recorded","observed_at":null,"producer":"synthetic","evidence_kind":"controller_statement","payload":{"declared_speaker":"synthetic","acquisition":"user_supplied_file","declared_at":null},"links":[],"bodies":[{"role":"text","path":body}]})).unwrap()).unwrap();
        fs::write(
            &context,
            r#"{"schema_version":1,"trace_id":"t","dispatch_id":"d","basis_event_ids":["basis"]}"#,
        )
        .unwrap();
        let db = rusqlite::Connection::open(
            f.dir
                .path()
                .join("state/saddle/telemetry/telemetry.sqlite3"),
        )
        .unwrap();
        db.execute_batch("CREATE TRIGGER fail_begin BEFORE INSERT ON events WHEN NEW.phase='begin' BEGIN SELECT RAISE(ABORT,'synthetic'); END").unwrap();
        let out = f
            .command(&[
                "run",
                "test.core",
                "route",
                "--record-context",
                context.to_str().unwrap(),
                "--brief-file",
                brief.to_str().unwrap(),
            ])
            .env("TEST_MODE", mode)
            .env("TEST_BRIEF", &brief)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(7));
        let r = paired(&out.stderr);
        assert_eq!(r["begin"], "unavailable");
        assert_eq!(
            r["end"],
            if mode == "recover" {
                "stored"
            } else {
                "disabled"
            }
        );
        let events = f.events();
        assert!(!events.iter().any(|e| e["kind"] == "route.begin"));
        if mode == "recover" {
            let snapshot = events
                .iter()
                .find(|e| e["kind"] == "brief.snapshot")
                .unwrap();
            assert_eq!(snapshot["payload"]["publication_phase"], "end");
            assert_eq!(snapshot["payload"]["begin_missing"], true);
            assert_eq!(
                f.store()
                    .body(snapshot["bodies"][0]["sha256"].as_str().unwrap())
                    .unwrap(),
                b"original brief\0\xff"
            );
            let end = events.iter().find(|e| e["kind"] == "route.end").unwrap();
            assert_eq!(end["payload"]["begin_missing"], true);
            assert!(
                end["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"relation":"based_on","target_event_id":"basis"}))
            );
            assert!(end["links"].as_array().unwrap().contains(
                &json!({"relation":"uses_brief","target_event_id":snapshot["event_id"]})
            ));
        } else {
            assert!(
                !events
                    .iter()
                    .any(|e| e["kind"] == "brief.snapshot" || e["kind"] == "route.end")
            );
        }
        assert_eq!(f.calls(), 1);
    }
}

#[test]
fn stdout_epipe_only_adds_receipt_gap_and_keeps_business_code_and_valid_end() {
    let f = Fixture::new();
    f.enable();
    let context = f.recording();
    let input = f.dir.path().join("input");
    fs::write(&input, b"no trailing newline").unwrap();
    let out = f
        .command(&[
            "run",
            "test.core",
            "route",
            "--record-context",
            context.to_str().unwrap(),
        ])
        .env("TEST_MODE", "stdout_closed")
        .stdin(fs::File::open(input).unwrap())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    let r = paired(&out.stderr);
    assert!(
        r["gaps"]
            .as_array()
            .unwrap()
            .contains(&json!({"role":"output","reason":"write_failed"})),
        "{r}"
    );
    assert_eq!(r["end"], "stored");
    let events = f.events();
    let end = events.iter().find(|e| e["kind"] == "route.end").unwrap();
    assert!(
        !end["payload"]["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["role"] == "output")
    );
    assert_eq!(end["payload"]["outcome"]["exit_code"], 7);
    assert_eq!(f.calls(), 1);
}

#[test]
fn explicit_invalid_contexts_are_rejected_before_execution_even_when_disabled() {
    for (changes, code) in [
        (json!({"schema_version":2}), "invalid_context"),
        (json!({"trace_id":""}), "invalid_context"),
        (
            json!({"trace_id":"other"}),
            "invalid_context_or_association",
        ),
        (
            json!({"dispatch_id":"other"}),
            "invalid_context_or_association",
        ),
        (
            json!({"basis_event_ids":["missing"]}),
            "invalid_context_or_association",
        ),
        (
            json!({"basis_event_ids":["same","same"]}),
            "invalid_context",
        ),
        (
            json!({"previous_brief_event_id":"missing"}),
            "invalid_context_combination",
        ),
        (
            json!({"decision_event_id":null}),
            "invalid_context_combination",
        ),
        (
            json!({"send_kind":"initial"}),
            "invalid_context_combination",
        ),
        (json!({"unknown":true}), "invalid_context"),
    ] {
        let f = Fixture::new();
        f.enable();
        let path = f.recording();
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
        let mut context = json!({"schema_version":1,"trace_id":"t","dispatch_id":"d"});
        context
            .as_object_mut()
            .unwrap()
            .extend(changes.as_object().unwrap().clone());
        fs::write(&path, context.to_string()).unwrap();
        let out = f.invoke(&[
            "run",
            "test.core",
            "route",
            "--record-context",
            path.to_str().unwrap(),
        ]);
        assert_eq!(out.status.code(), Some(125));
        let r = paired(&out.stderr);
        assert_eq!(r["error"]["code"], code, "{context}: {r}");
        assert_eq!(r["executed"], false);
        assert_eq!(r["begin"], "not_requested");
        assert_eq!(r["end"], "not_requested");
        assert_eq!(f.calls(), 0);
    }
}

#[test]
fn context_reader_rejects_links_fifos_oversized_and_duplicate_fields() {
    for mode in ["symlink", "fifo", "oversized", "duplicate"] {
        let f = Fixture::new();
        f.enable();
        let path = f.dir.path().join("context");
        match mode {
            "symlink" => std::os::unix::fs::symlink("/does/not/exist", &path).unwrap(),
            "fifo" => {
                use std::os::unix::ffi::OsStrExt;
                let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "oversized" => fs::write(&path, vec![b' '; 65537]).unwrap(),
            _ => fs::write(
                &path,
                r#"{"schema_version":1,"trace_id":"first","trace_id":"last","dispatch_id":"d"}"#,
            )
            .unwrap(),
        }
        let mut child = f
            .command(&[
                "run",
                "test.core",
                "route",
                "--record-context",
                path.to_str().unwrap(),
            ])
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let status = wait_bounded(&mut child);
        let mut bytes = vec![];
        std::io::Read::read_to_end(child.stderr.as_mut().unwrap(), &mut bytes).unwrap();
        assert_eq!(status.code(), Some(125));
        assert_eq!(paired(&bytes)["executed"], false);
        assert_eq!(f.calls(), 0);
        assert!(!f.dir.path().join("state").exists());
    }
}

#[test]
fn non_utf8_arguments_have_a_paired_parse_refusal_in_the_real_binary() {
    use std::os::unix::ffi::OsStringExt;
    let f = Fixture::new();
    let out = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args(["plugin", "run", "test.core", "route", "--brief-file"])
        .arg(std::ffi::OsString::from_vec(b"/synthetic/\xff".to_vec()))
        .env("HOME", f.dir.path())
        .env("XDG_CONFIG_HOME", f.dir.path().join("config"))
        .env("XDG_STATE_HOME", f.dir.path().join("state"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(125));
    assert_eq!(paired(&out.stderr)["error"]["code"], "invalid_utf8");
    assert!(out.stdout.is_empty());
}

#[test]
fn duplicate_begin_is_ignored_and_missing_end_bodies_are_explicit_gaps() {
    for mode in ["double", "no_end", "missing"] {
        let f = Fixture::new();
        f.enable();
        let context = f.recording();
        let out = f
            .command(&[
                "run",
                "test.core",
                "route",
                "--record-context",
                context.to_str().unwrap(),
            ])
            .env("TEST_MODE", mode)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(if mode == "no_end" { 0 } else { 7 })
        );
        let r = paired(&out.stderr);
        assert_eq!(r["begin"], "stored");
        assert_eq!(r["end"], "stored");
        let events = f.events();
        let begins: Vec<_> = events
            .iter()
            .filter(|e| e["kind"] == "route.begin")
            .collect();
        assert_eq!(begins.len(), 1);
        assert_eq!(begins[0]["payload"]["router_model"], "synthetic-router");
        let end = events.iter().find(|e| e["kind"] == "route.end").unwrap();
        if mode == "double" {
            assert!(
                r["gaps"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"role":"capture","reason":"unrecognized"}))
            );
        } else {
            assert!(end["bodies"].as_array().unwrap().is_empty());
            for (role, reason) in [
                (
                    "response",
                    if mode == "no_end" {
                        "not_available"
                    } else {
                        "too_large"
                    },
                ),
                (
                    "suggestion",
                    if mode == "no_end" {
                        "not_available"
                    } else {
                        "unrecognized"
                    },
                ),
            ] {
                assert!(
                    end["payload"]["gaps"]
                        .as_array()
                        .unwrap()
                        .contains(&json!({"role":role,"reason":reason}))
                );
            }
        }
        assert_eq!(f.calls(), 1);
    }
}

fn wait_bounded(child: &mut std::process::Child) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("synthetic child exceeded 5 second safety bound");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn library_stderr_without_lf_keeps_terminal_receipt_on_its_own_line() {
    let f = Fixture::new();
    f.enable();
    let input = f.dir.path().join("input");
    let business = b"synthetic result\0\xff";
    fs::write(&input, business).unwrap();
    let out = f
        .command(&["run", "test.core", "route"])
        .env("TEST_MODE", "stderr_no_lf")
        .stdin(fs::File::open(input).unwrap())
        .output()
        .unwrap();
    let stdout = fs::read(f.dir.path().join("output")).unwrap();
    eprintln!(
        "exit={:?}, calls={}, stdout={stdout:?}, stderr={:?}",
        out.status.code(),
        f.calls(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(7));
    assert_eq!(stdout, business);
    assert_eq!(f.calls(), 1);
    assert!(!f.dir.path().join("state").exists());
    let r = paired(&out.stderr);
    assert_eq!(r["executed"], true);
    assert_eq!(r["outcome"], json!({"kind":"exited","exit_code":7}));
    assert_eq!(r["begin"], "not_requested");
    assert_eq!(r["end"], "not_requested");
    assert_eq!(
        out.stderr.split(|b| *b == b'\n').nth(1).unwrap(),
        b"synthetic library diagnostic without LF"
    );
}

#[test]
fn full_stderr_has_bounded_receipts_no_replay_and_no_final_after_failed_start() {
    use std::os::fd::{FromRawFd, OwnedFd};
    for mode in ["start_full", "final_full", "panic"] {
        let f = Fixture::new();
        f.enable();
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        let (mut reader, writer) =
            unsafe { (fs::File::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        if mode != "final_full" {
            fill_pipe(writer.as_raw_fd());
        }
        let started = std::time::Instant::now();
        let mut child = f
            .command(&["run", "test.core", "route"])
            .env("TEST_MODE", mode)
            .stderr(Stdio::from(writer))
            .spawn()
            .unwrap();
        // Intentionally do not read any stderr until after process exit.
        let status = wait_bounded(&mut child);
        assert_eq!(status.code(), Some(if mode == "panic" { 126 } else { 7 }));
        assert!(started.elapsed() < std::time::Duration::from_secs(4));
        let mut stderr = vec![];
        std::io::Read::read_to_end(&mut reader, &mut stderr).unwrap();
        assert_eq!(f.calls(), 1);
        let text = String::from_utf8(stderr).unwrap();
        if mode != "final_full" {
            assert!(!text.contains("saddle-plugin:")); // No final appended even after a failed start.
        } else {
            assert!(text.starts_with("saddle-plugin: "));
            assert!(text.lines().next().unwrap().contains("\"final\":false"));
            assert_eq!(text.matches("saddle-plugin:").count(), 1);
        }
        assert!(!text.contains("\"final\":true"));
    }
}
