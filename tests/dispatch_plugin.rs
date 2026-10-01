use serde_json::Value;
use std::process::{Command, Stdio};

#[test]
fn dispatch_is_registered_but_disabled_without_creating_state() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args(["plugin", "status", "dispatch"])
        .env_remove("TYPESAFE_API_KEY")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["core"][0]["id"], "dispatch");
    assert_eq!(value["core"][0]["state"], "disabled");
    assert_eq!(value["core"][0]["commands"][0]["capture"], "route");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

// Compile the exact business modules into this test-only host harness so its HTTP
// boundary can be synthetic. No URL/key override or host dependency enters the plugin.
#[path = "../plugins/dispatch/src/json.rs"]
mod json;
#[path = "../plugins/dispatch/src/route.rs"]
mod route;
#[path = "../plugins/dispatch/src/rules.rs"]
mod rules;
use saddle::plugins::{
    Manager,
    core::Catalog,
    resources::{Outcome, Resources},
};
use saddle::telemetry::{EventQuery, SettingInput, Store};
use saddle_core_plugin::{Call, Completion, CorePlugin, Manifest};
use serde_json::json;
use std::{
    fs,
    os::{fd::AsRawFd, unix::fs::symlink},
    path::Path,
};
const KEY: &str = "synthetic-key-integration-03c";
const RESPONSE:&[u8]=br#"{ "answers":{"tier":{"probabilities":{"2":1},"confidence":1,"score":2},"cross_core_rules":{"noul":1},"visible":{"noul":0},"doc_only":{"noul":0}},"unused":{"z":1,"a":2,"z":3},"model":"synthetic" }"#;
struct SyntheticRoute;
static SYNTHETIC: SyntheticRoute = SyntheticRoute;
static CATALOG: Catalog = &[&SYNTHETIC];
static PRODUCTION: Catalog = &[&saddle_dispatch_plugin::PLUGIN];
impl CorePlugin for SyntheticRoute {
    fn manifest(&self) -> &'static Manifest {
        saddle_dispatch_plugin::PLUGIN.manifest()
    }
    fn run(&self, call: Call<'_>) -> Completion {
        route::run(
            call,
            Some(KEY),
            |body, key, first| {
                assert_eq!(key, KEY);
                assert!(first);
                let path = std::env::var_os("TEST_DISPATCH_REQUEST").unwrap();
                fs::write(path, body).unwrap();
                Ok(route::Response {
                    status: 200,
                    body: RESPONSE.to_vec(),
                })
            },
            |_| panic!("success must not retry"),
        )
    }
}
#[test]
fn dispatch_driver() {
    let Some(args) = std::env::var_os("TEST_DISPATCH_ARGS") else {
        return;
    };
    let output = fs::File::create(std::env::var_os("TEST_DISPATCH_OUTPUT").unwrap()).unwrap();
    assert_eq!(
        unsafe { libc::dup2(output.as_raw_fd(), libc::STDOUT_FILENO) },
        libc::STDOUT_FILENO
    );
    let args: Vec<String> = serde_json::from_str(args.to_str().unwrap()).unwrap();
    std::process::exit(saddle::plugins::cli::run(
        &args.into_iter().map(Into::into).collect::<Vec<_>>(),
        CATALOG,
    ));
}
fn paired(bytes: &[u8]) -> Value {
    assert!(bytes.ends_with(b"\n"));
    let lines: Vec<_> = bytes.split(|b| *b == b'\n').collect();
    let parse = |b: &[u8]| {
        serde_json::from_slice::<Value>(b.strip_prefix(b"saddle-plugin: ").unwrap()).unwrap()
    };
    let start = parse(lines[0]);
    let end = parse(lines[lines.len() - 2]);
    assert_eq!(start["schema_version"], 1);
    assert_eq!(start["final"], false);
    assert_eq!(end["schema_version"], 1);
    assert_eq!(end["final"], true);
    assert_eq!(start["call_id"], end["call_id"]);
    assert_eq!(
        uuid::Uuid::parse_str(start["call_id"].as_str().unwrap())
            .unwrap()
            .get_version_num(),
        4
    );
    end
}
fn isolated(command: &mut Command, root: &Path) {
    command
        .env_remove("TYPESAFE_API_KEY")
        .env("HOME", root.join("home"))
        .stdin(Stdio::null());
    for (key, dir) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ] {
        command.env(key, root.join(dir));
    }
}
fn body(store: &Store, event: &Value, role: &str) -> Vec<u8> {
    let b = event["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["role"] == role)
        .unwrap();
    store.body(b["sha256"].as_str().unwrap()).unwrap()
}
#[test]
fn dispatch_resources_and_route_capture_integrate_without_business_or_secret_leaks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let home = root.join("home");
    fs::create_dir_all(home.join(".claude")).unwrap();
    fs::create_dir_all(home.join(".agents/skills")).unwrap();
    let old = home.join(".agents/skills/corral-dispatch");
    symlink("/synthetic/old-skill", &old).unwrap();
    let mut manager = Manager::with_resources(
        root.join("config/saddle/plugins.toml"),
        PRODUCTION,
        Resources::new(home.clone(), root.join("state/saddle")),
    );
    manager.core_enabled("dispatch", true).unwrap();
    let receipt = manager.core_receipt("dispatch").unwrap();
    assert_eq!(receipt.resources[0].targets[0].result, Outcome::Installed);
    assert_eq!(receipt.resources[0].targets[1].result, Outcome::Conflict);
    let installed = home.join(".claude/skills/corral-dispatch");
    for file in saddle_dispatch_plugin::PLUGIN.manifest().resources[0].files {
        assert_eq!(fs::read(installed.join(file.path)).unwrap(), file.bytes);
    }
    assert_eq!(
        fs::read_link(&old).unwrap(),
        Path::new("/synthetic/old-skill")
    );
    assert_eq!(
        manager
            .core_resources("dispatch")
            .unwrap()
            .as_ref()
            .unwrap()
            .setup_files[0]
            .paths,
        vec![installed.join("项目AGENTS模板.md")]
    );

    // Real binary, enabled real plugin, no credentials: business failure, no telemetry DB.
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_saddle"));
    isolated(&mut cmd, root);
    let result = cmd
        .args(["plugin", "run", "dispatch", "route"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(paired(&result.stderr)["begin"], "not_requested");
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["error"],
        "TYPESAFE_API_KEY is not set"
    );
    assert!(!root.join("state/saddle/telemetry").exists());

    let store = Store::new(root.join("state/saddle/telemetry"));
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
    let context = root.join("context.json");
    fs::write(
        &context,
        br#"{"schema_version":1,"trace_id":"t","dispatch_id":"d"}"#,
    )
    .unwrap();
    let brief = root.join("brief.md");
    fs::write(&brief, b"SYNTHETIC_BRIEF_ONLY\0\xff").unwrap();
    let input = root.join("summary");
    fs::write(&input, b"  SYNTHETIC_SUMMARY_ONLY\n").unwrap();
    let output = root.join("output");
    let request = root.join("request");
    let args = [
        "run",
        "dispatch",
        "route",
        "--record-context",
        context.to_str().unwrap(),
        "--brief-file",
        brief.to_str().unwrap(),
    ];
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    isolated(&mut cmd, root);
    let result = cmd
        .args(["--exact", "dispatch_driver", "--nocapture"])
        .env("TEST_DISPATCH_ARGS", serde_json::to_string(&args).unwrap())
        .env("TEST_DISPATCH_OUTPUT", &output)
        .env("TEST_DISPATCH_REQUEST", &request)
        .stdin(fs::File::open(input).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt = paired(&result.stderr);
    assert_eq!(receipt["executed"], true);
    assert_eq!(receipt["begin"], "stored");
    assert_eq!(receipt["end"], "stored");
    let stdout = fs::read(output).unwrap();
    let request = fs::read(request).unwrap();
    assert!(!String::from_utf8_lossy(&request).contains("BRIEF"));
    let events = store.events(&EventQuery::default()).unwrap();
    let events = events["events"].as_array().unwrap();
    let event = |kind| events.iter().find(|e| e["kind"] == kind).unwrap();
    let begin = event("route.begin");
    let end = event("route.end");
    let snapshot = event("brief.snapshot");
    assert_eq!(
        events.iter().filter(|e| e["kind"] == "route.begin").count(),
        1
    );
    for event in [begin, end, snapshot] {
        assert_eq!(event["producer"], "saddle.plugin.dispatch");
        assert_eq!(event["evidence_kind"], "execution_observed");
        assert_eq!(event["operation_id"], receipt["operation_id"]);
    }
    assert_eq!(body(&store, begin, "summary"), b"SYNTHETIC_SUMMARY_ONLY");
    assert_eq!(body(&store, begin, "request"), request);
    assert_eq!(body(&store, end, "suggestion"), stdout);
    assert_eq!(
        body(&store, snapshot, "brief"),
        b"SYNTHETIC_BRIEF_ONLY\0\xff"
    );
    let response = body(&store, end, "response");
    assert!(String::from_utf8_lossy(&response).contains("\"unused\":{\"z\":3,\"a\":2}"));
    assert_eq!(
        end["payload"]["outcome"],
        json!({"kind":"exited","exit_code":0})
    );
    for event in events {
        assert!(!event.to_string().contains(KEY));
        for b in event["bodies"].as_array().unwrap() {
            assert!(
                !String::from_utf8_lossy(&store.body(b["sha256"].as_str().unwrap()).unwrap())
                    .contains(KEY)
            );
        }
    }
    assert!(!String::from_utf8_lossy(&result.stderr).contains(KEY));
    manager.core_enabled("dispatch", false).unwrap();
    assert!(installed.join("SKILL.md").exists());
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_saddle"));
    isolated(&mut cmd, root);
    let result = cmd
        .args(["plugin", "run", "dispatch", "route"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(125));
    assert_eq!(paired(&result.stderr)["error"]["code"], "plugin_disabled");
    manager.core_remove_resources("dispatch").unwrap();
    assert!(!installed.exists());
    assert!(old.is_symlink());
}
