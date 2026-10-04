//! Drover's optional dispatch recording through the public `saddle telemetry`/`saddle agent` CLI
//! of the real debug host, with a fake Corral and isolated telemetry state.
#[path = "common/interpreted_script.rs"]
mod fixture;
use saddle_drover_plugin::{
    core,
    drover::{Operation, Transition},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const CORRAL: &str = r#"#!/usr/bin/env python3
import json, os, sys, time
from pathlib import Path
root = Path(__file__).parent
with (root / 'corral-calls').open('a') as f:
    f.write(json.dumps({'args': sys.argv[1:], 'host_env': 'SADDLE_HOST_BIN' in os.environ}) + '\n')
if (root / 'hang').exists():
    (root / 'corral-pid').write_text(str(os.getpid()))
    time.sleep(30)
answer = root / 'answer'
print(answer.read_text() if answer.exists() else json.dumps({'ok': True, 'confirmed': True, 'merged_with_draft': False}))
code = root / 'code'
sys.exit(int(code.read_text()) if code.exists() else 0)
"#;

struct Project {
    dir: tempfile::TempDir,
    root: PathBuf,
}
impl Project {
    fn new(conf: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project").canonicalize_or_create();
        std::fs::create_dir(root.join("data")).unwrap();
        std::fs::write(
            root.join(".drover.conf"),
            format!("HANDOFF_DIR=data\nMAIN_AGENT=p/main\n{conf}"),
        )
        .unwrap();
        std::fs::write(root.join("data/queue.md"), "## T1 First\nBody\n").unwrap();
        fixture::script(dir.path(), "corral", CORRAL);
        Self { dir, root }
    }
    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
    fn corral(&self) -> String {
        self.path("corral").display().to_string()
    }
    /// The real host with this project's telemetry state; logs which subcommand ran.
    fn host(&self) -> PathBuf {
        self.host_with_state(&self.path("state"))
    }
    fn host_with_state(&self, state: &Path) -> PathBuf {
        fixture::script(
            self.dir.path(),
            "host",
            &format!(
                "#!/bin/sh\necho \"$1 $2\" >> {:?}\nXDG_STATE_HOME={:?} exec {:?} \"$@\"\n",
                self.path("host-calls"),
                state,
                env!("CARGO_BIN_EXE_saddle")
            ),
        )
        .into()
    }
    /// A host whose agent entry runs Corral once but leaves `stderr` instead of its receipt.
    fn host_without_receipt(&self, stderr: &str) -> PathBuf {
        fixture::script(
            self.dir.path(),
            "host",
            &format!(
                r#"#!/usr/bin/env python3
import os, subprocess, sys
args = sys.argv[1:]
if args[0] == 'telemetry':
    os.environ['XDG_STATE_HOME'] = {state:?}
    os.execv({saddle:?}, [{saddle:?}] + args)
corral = args[args.index('--corral') + 1]
out = subprocess.run([corral] + args[args.index('--') + 1:], capture_output=True)
sys.stdout.buffer.write(out.stdout)
sys.stderr.write({stderr:?})
sys.exit(out.returncode)
"#,
                state = self.path("state"),
                saddle = env!("CARGO_BIN_EXE_saddle"),
            ),
        )
        .into()
    }
    fn telemetry(&self, args: &[&str]) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_saddle"))
            .arg("telemetry")
            .args(args)
            .env("XDG_STATE_HOME", self.path("state"))
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn enable(&self, enabled: bool) {
        let input = self.path("setting.json");
        std::fs::write(
            &input,
            json!({"schema_version":1,"enabled":enabled,"actor":"synthetic"}).to_string(),
        )
        .unwrap();
        let receipt = self.telemetry(&["settings", "set", "--input", input.to_str().unwrap()]);
        assert_eq!(receipt["ok"], true, "{receipt}");
    }
    fn calls(&self) -> Vec<Value> {
        std::fs::read_to_string(self.path("corral-calls"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
    fn dispatch_with(
        &self,
        record: Option<bool>,
        corral: &str,
        host: Option<&Path>,
        cancel: &AtomicBool,
    ) -> Value {
        let v = core::list(&self.root).unwrap();
        let op = Operation::DispatchPending {
            project: self.root.display().to_string(),
            pos: 1,
            token: v["pending"][0]["actions"]["dispatch-pending"]["target_token"]
                .as_str()
                .unwrap()
                .into(),
            record,
        };
        core::execute_with(&self.root, &op, corral, host, cancel, None).unwrap()
    }
    fn dispatch(&self, record: Option<bool>, host: Option<&Path>) -> Value {
        self.dispatch_with(record, &self.corral(), host, &AtomicBool::new(false))
    }
    fn transition(&self, action: Transition, host: Option<&Path>) -> Value {
        let v = core::list(&self.root).unwrap();
        let t = if v["current"].is_object() {
            &v["current"]
        } else {
            &v["awaiting"]
        };
        core::execute_with(
            &self.root,
            &Operation::Transition {
                project: self.root.display().to_string(),
                id: t["id"].as_str().unwrap().into(),
                run_id: t["run_id"].as_str().unwrap().into(),
                token: t["actions"][action.command()]["target_token"]
                    .as_str()
                    .unwrap()
                    .into(),
                action,
                reason: "needs revision".into(),
            },
            &self.corral(),
            host,
            &AtomicBool::new(false),
            None,
        )
        .unwrap()
    }
    fn events(&self, trace: &str) -> Vec<Value> {
        self.telemetry(&["events", "--trace-id", trace])["events"]
            .as_array()
            .unwrap()
            .clone()
    }
}
trait Canonical {
    fn canonicalize_or_create(self) -> PathBuf;
}
impl Canonical for PathBuf {
    fn canonicalize_or_create(self) -> PathBuf {
        std::fs::create_dir_all(&self).unwrap();
        self.canonicalize().unwrap()
    }
}
fn plain_message() -> Value {
    json!(["send", "p/main", "TASK T1: First\n\nBody"])
}

#[test]
fn unselected_recording_keeps_the_plain_send_and_never_calls_saddle() {
    let p = Project::new("");
    assert_eq!(core::list(&p.root).unwrap()["record_default"], false);
    let host = p.host();
    let v = p.dispatch(None, Some(&host));
    assert_eq!(v["delivery"]["status"], "confirmed", "{v}");
    assert_eq!(v["record"]["status"], "recorded");
    assert_eq!(v["telemetry"]["status"], "not_requested", "{v}");
    let calls = p.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["args"], plain_message());
    assert_eq!(calls[0]["host_env"], false);
    assert!(!p.path("host-calls").exists(), "no telemetry call at all");
    assert!(
        !p.path("state").exists(),
        "no telemetry store for an unrecorded dispatch"
    );

    // An explicit "no" for this dispatch wins over a project that records by default.
    let p = Project::new("TELEMETRY_RECORD=on\n");
    assert_eq!(core::list(&p.root).unwrap()["record_default"], true);
    let host = p.host();
    let v = p.dispatch(Some(false), Some(&host));
    assert_eq!(v["telemetry"]["status"], "not_requested", "{v}");
    assert_eq!(p.calls()[0]["args"], plain_message());
    assert!(!p.path("host-calls").exists());
}

#[test]
fn without_a_record_context_the_delivery_goes_the_plain_way_once() {
    // Global recording off (never enabled), no host, and an unusable telemetry state directory.
    for case in ["disabled", "host_unavailable", "unavailable"] {
        let p = Project::new("");
        let host = match case {
            "disabled" => Some(p.host()),
            "unavailable" => {
                std::fs::write(p.path("not-a-directory"), "").unwrap();
                Some(p.host_with_state(&p.path("not-a-directory")))
            }
            _ => None,
        };
        let v = p.dispatch(Some(true), host.as_deref());
        assert_eq!(v["telemetry"]["status"], "no_context", "{case}: {v}");
        assert_eq!(v["telemetry"]["reason"], case, "{case}: {v}");
        assert_eq!(v["delivery"]["status"], "confirmed", "{case}: {v}");
        assert_eq!(v["state"], "running");
        let calls = p.calls();
        assert_eq!(calls.len(), 1, "{case}");
        assert_eq!(calls[0]["args"], plain_message(), "{case}");
        assert!(
            !std::fs::read_to_string(p.path("host-calls"))
                .unwrap_or_default()
                .contains("agent"),
            "{case}: never through the agent entry without context"
        );
    }
}

#[test]
fn slow_host_startup_exhausts_preparation_budget_and_sends_plain_once() {
    let p = Project::new("");
    p.enable(true);
    let host = fixture::script(
        p.dir.path(),
        "slow-host",
        &format!(
            r#"#!/usr/bin/python3
import os, sys, time
from pathlib import Path
root = Path(__file__).parent
(root / 'startup-pid').write_text(str(os.getpid()))
# Deliberate startup delay: the real host must not run before the preparation deadline.
time.sleep(3)
(root / 'host-started').touch()
os.environ['XDG_STATE_HOME'] = {state:?}
os.execv({saddle:?}, [{saddle:?}] + sys.argv[1:])
"#,
            state = p.path("state"),
            saddle = env!("CARGO_BIN_EXE_saddle"),
        ),
    );
    let began = Instant::now();
    let v = p.dispatch(Some(true), Some(Path::new(&host)));
    let elapsed = began.elapsed();
    let pid: i32 = std::fs::read_to_string(p.path("startup-pid"))
        .expect("slow-start fixture entered before preparation timed out")
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "slow host was reaped");
    assert!(!p.path("host-started").exists());
    assert!(elapsed >= Duration::from_millis(300) && elapsed < Duration::from_secs(1));
    assert_eq!(v["telemetry"]["status"], "no_context", "{v}");
    assert_eq!(v["telemetry"]["reason"], "budget_exhausted", "{v}");
    assert_eq!(v["delivery"]["status"], "confirmed", "{v}");
    assert_eq!(v["record"]["status"], "recorded", "{v}");
    assert_eq!(v["state"], "running");
    let calls = p.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["args"], plain_message());
    assert_eq!(p.telemetry(&["list"])["traces"], json!([]));
}

#[test]
fn a_recorded_dispatch_sends_once_with_its_context_and_follows_each_transition() {
    let p = Project::new("TELEMETRY_RECORD=on\n");
    p.enable(true);
    let host = p.host();
    let v = p.dispatch(None, Some(&host));
    assert_eq!(v["delivery"]["status"], "confirmed", "{v}");
    assert_eq!(v["record"]["status"], "recorded");
    let telemetry = &v["telemetry"];
    assert_eq!(telemetry["status"], "context", "{v}");
    assert_eq!(telemetry["send"]["receipt"], "matched", "{v}");
    assert_eq!(telemetry["send"]["executed"], true);
    assert_eq!(telemetry["send"]["begin"], "stored");
    assert_eq!(telemetry["send"]["end"], "stored");
    assert_eq!(telemetry["transition"]["status"], "stored", "{v}");
    let (trace, dispatch) = (
        telemetry["trace_id"].as_str().unwrap(),
        telemetry["dispatch_id"].as_str().unwrap(),
    );
    let run = v["run_id"].as_str().unwrap();

    let calls = p.calls();
    assert_eq!(calls.len(), 1, "exactly one delivery");
    assert_eq!(
        calls[0]["host_env"], false,
        "Corral does not receive the host path"
    );
    let sent = calls[0]["args"][2].as_str().unwrap();
    assert!(
        sent.starts_with("TASK T1: First\n\nBody\n\n---\n"),
        "{sent}"
    );
    for line in [
        format!("trace_id: {trace}"),
        format!("dispatch_id: {dispatch}"),
        "task: T1".into(),
        format!("run: {run}"),
    ] {
        assert!(sent.contains(&line), "{line}: {sent}");
    }

    let scope = p.root.to_str().unwrap();
    let list = p.telemetry(&[
        "list",
        "--kind",
        "drover.task",
        "--scope",
        scope,
        "--key",
        "T1",
    ]);
    let traces = list["traces"].as_array().unwrap();
    assert_eq!(traces.len(), 1, "{list}");
    assert_eq!(traces[0]["trace_id"], trace);
    assert_eq!(traces[0]["binding"]["run"], run);
    let shown = p.telemetry(&["show", "--id", trace]);
    assert_eq!(
        shown["record"]["dispatches"][0]["kind"], "controller_handoff",
        "{shown}"
    );
    let events = p.events(trace);
    let kinds: Vec<_> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["agent.send.begin", "agent.send.end", "task.transition"],
        "{events:?}"
    );
    assert_eq!(events[0]["payload"]["send_kind"], "initial");
    assert_eq!(events[0]["dispatch_id"], dispatch);
    let message = events[0]["bodies"][0]["sha256"].as_str().unwrap();
    let body = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args(["telemetry", "body", "--sha256", message])
        .env("XDG_STATE_HOME", p.path("state"))
        .output()
        .unwrap();
    assert_eq!(body.stdout, sent.as_bytes(), "the whole sent text is saved");
    let transition = &events[2]["payload"];
    assert_eq!(
        (&transition["from"], &transition["to"]),
        (&json!("pending"), &json!("running"))
    );
    assert!(
        transition["business_committed_at"].is_string(),
        "{transition}"
    );
    assert_eq!(transition["binding"]["key"], "T1");

    // Later transitions find the same run's trace by its full binding.
    let v = p.transition(Transition::Submit, Some(&host));
    assert_eq!(v["state"], "awaiting_release");
    assert_eq!(v["telemetry"]["status"], "stored", "{v}");
    assert_eq!(v["telemetry"]["trace_id"], trace);
    assert_eq!(v["telemetry"]["close"]["status"], "not_attempted");
    assert!(p.telemetry(&["show", "--id", trace])["record"]["closed_at"].is_null());
    let v = p.transition(Transition::Accept, Some(&host));
    assert_eq!(v["telemetry"]["status"], "stored", "{v}");
    assert_eq!(v["telemetry"]["close"]["status"], "stored", "{v}");
    assert!(p.telemetry(&["show", "--id", trace])["record"]["closed_at"].is_string());
    let events = p.events(trace);
    let moves: Vec<_> = events
        .iter()
        .filter(|e| e["kind"] == "task.transition")
        .map(|e| {
            (
                e["payload"]["from"].as_str().unwrap().to_owned(),
                e["payload"]["to"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        moves,
        [
            ("pending".into(), "running".into()),
            ("running".into(), "awaiting_release".into()),
            ("awaiting_release".into(), "done".into())
        ]
    );
    assert_eq!(p.calls().len(), 1, "transitions never deliver again");
}

#[test]
fn a_started_delivery_without_a_matching_receipt_is_unknown_and_never_resent() {
    let other = || {
        format!(
            "saddle-telemetry: {}\n",
            json!({"schema_version":1,"call_id":uuid::Uuid::new_v4().to_string(),"final":false})
        )
    };
    let closing = format!(
        "saddle-telemetry: {}\n",
        json!({"schema_version":1,"call_id":uuid::Uuid::new_v4().to_string(),"final":true,"executed":true})
    );
    for stderr in [String::new(), format!("{}{closing}", other())] {
        let p = Project::new("TELEMETRY_RECORD=on\n");
        p.enable(true);
        let host = p.host_without_receipt(&stderr);
        let v = p.dispatch(None, Some(&host));
        assert_eq!(v["telemetry"]["status"], "context", "{v}");
        assert_eq!(v["telemetry"]["send"]["receipt"], "missing", "{v}");
        assert_eq!(v["delivery"]["status"], "unknown", "{v}");
        assert_eq!(
            v["delivery"]["corral_exit_code"], 0,
            "Corral's own result is kept"
        );
        assert_eq!(v["record"]["status"], "not_attempted");
        assert_eq!(v["ok"], false);
        assert_eq!(p.calls().len(), 1, "not resent, not sent the plain way");
        assert!(core::list(&p.root).unwrap()["current"].is_null());
    }
}

#[test]
fn only_a_paired_executed_false_receipt_says_nothing_was_sent() {
    // Saddle could not start Corral: the paired receipt says executed=false.
    let p = Project::new("TELEMETRY_RECORD=on\n");
    p.enable(true);
    let host = p.host();
    let missing = p.path("no-such-corral").display().to_string();
    let v = p.dispatch_with(None, &missing, Some(&host), &AtomicBool::new(false));
    assert_eq!(v["telemetry"]["status"], "context", "{v}");
    assert_eq!(v["telemetry"]["send"]["receipt"], "matched", "{v}");
    assert_eq!(v["telemetry"]["send"]["executed"], false, "{v}");
    assert_eq!(v["delivery"]["status"], "not_executed", "{v}");
    assert_eq!(v["record"]["status"], "not_attempted");
    assert!(core::list(&p.root).unwrap()["current"].is_null());

    // Corral itself exiting 125 or 127 is a started delivery with an unknown result.
    for code in ["125", "127"] {
        let p = Project::new("TELEMETRY_RECORD=on\n");
        p.enable(true);
        std::fs::write(p.path("code"), code).unwrap();
        let host = p.host();
        let v = p.dispatch(None, Some(&host));
        assert_eq!(v["telemetry"]["status"], "context", "{v}");
        assert_eq!(v["telemetry"]["send"]["receipt"], "matched", "{v}");
        assert_eq!(v["telemetry"]["send"]["executed"], true, "{v}");
        assert_eq!(v["delivery"]["status"], "unknown", "{code}: {v}");
        assert_eq!(v["record"]["status"], "not_attempted");
        assert_eq!(p.calls().len(), 1);
    }
}

#[test]
fn cancelling_a_recorded_delivery_stops_its_process_group_and_stays_unknown() {
    let p = Project::new("TELEMETRY_RECORD=on\n");
    p.enable(true);
    std::fs::write(p.path("hang"), "").unwrap();
    let host = p.host();
    let cancel = Arc::new(AtomicBool::new(false));
    let pid_file = p.path("corral-pid");
    let stopper = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !pid_file.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            std::thread::sleep(Duration::from_millis(50));
            cancel.store(true, Ordering::Relaxed);
        })
    };
    let began = Instant::now();
    let v = p.dispatch_with(None, &p.corral(), Some(&host), &cancel);
    stopper.join().unwrap();
    assert!(began.elapsed() < Duration::from_secs(10));
    assert_eq!(v["telemetry"]["status"], "context", "{v}");
    assert_eq!(v["delivery"]["status"], "unknown", "{v}");
    assert_eq!(v["telemetry"]["send"]["receipt"], "missing", "{v}");
    assert_eq!(v["record"]["status"], "not_attempted");
    let pid: i32 = std::fs::read_to_string(p.path("corral-pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    // The Corral client, a grandchild in the entry's group, was stopped with it.
    while unsafe { libc::kill(pid, 0) } == 0 {
        assert!(
            Instant::now() < deadline,
            "Corral client {pid} still running"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(p.calls().len(), 1);
}

#[test]
fn task_records_and_telemetry_failures_are_reported_apart() {
    // Telemetry turned off later: the task still moves, the transition says disabled.
    let p = Project::new("TELEMETRY_RECORD=on\n");
    p.enable(true);
    let host = p.host();
    let v = p.dispatch(None, Some(&host));
    assert_eq!(v["telemetry"]["status"], "context", "{v}");
    let trace = v["telemetry"]["trace_id"].as_str().unwrap().to_owned();
    p.enable(false);
    let v = p.transition(Transition::Submit, Some(&host));
    assert_eq!(v["ok"], true);
    assert_eq!(v["state"], "awaiting_release");
    assert_eq!(v["record"]["status"], "recorded");
    assert_eq!(v["telemetry"]["status"], "disabled", "{v}");
    assert_eq!(p.events(&trace).len(), 3, "nothing appended while disabled");

    // Drover cannot save the start: no transition is claimed for the trace.
    let p = Project::new("TELEMETRY_RECORD=on\n");
    p.enable(true);
    let data = p.root.join("data");
    std::fs::write(data.join(".tasks.lock"), "").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o555)).unwrap();
    let host = p.host();
    let v = p.dispatch(None, Some(&host));
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(v["telemetry"]["status"], "context", "{v}");
    assert_eq!(v["delivery"]["status"], "confirmed", "{v}");
    assert_eq!(v["record"]["status"], "unknown", "{v}");
    assert_eq!(
        v["telemetry"]["transition"]["status"], "not_attempted",
        "{v}"
    );
    let trace = v["telemetry"]["trace_id"].as_str().unwrap();
    assert!(
        p.events(trace)
            .iter()
            .all(|e| e["kind"] != "task.transition"),
        "no transition for a start Drover did not save"
    );
    assert_eq!(p.calls().len(), 1);
}

#[test]
fn the_project_recording_default_is_saved_in_its_configuration() {
    let p = Project::new("CHECK_CMD=make check\n");
    let before = std::fs::read_to_string(p.root.join(".drover.conf")).unwrap();
    for on in [true, false] {
        let v = core::execute_with(
            &p.root,
            &Operation::RecordDefault(on),
            &p.corral(),
            None,
            &AtomicBool::new(false),
            None,
        )
        .unwrap();
        assert_eq!(v["ok"], true, "{v}");
        assert_eq!(core::list(&p.root).unwrap()["record_default"], on);
        let conf = std::fs::read_to_string(p.root.join(".drover.conf")).unwrap();
        assert_eq!(
            conf,
            format!(
                "{before}TELEMETRY_RECORD={}\n",
                if on { "on" } else { "off" }
            )
        );
    }
    assert!(p.calls().is_empty() && !p.path("state").exists());
}

#[test]
fn without_a_main_agent_the_context_goes_into_the_manual_text() {
    let p = Project::new("");
    std::fs::write(
        p.root.join(".drover.conf"),
        "HANDOFF_DIR=data\nTELEMETRY_RECORD=on\n",
    )
    .unwrap();
    p.enable(true);
    let host = p.host();
    let v = p.dispatch(None, Some(&host));
    assert_eq!(v["delivery"]["status"], "not_sent", "{v}");
    assert_eq!(v["record"]["status"], "recorded");
    assert_eq!(v["telemetry"]["status"], "context", "{v}");
    assert!(v["telemetry"]["send"].is_null(), "nothing was sent");
    assert_eq!(v["telemetry"]["transition"]["status"], "stored", "{v}");
    let text = v["manual_text"].as_str().unwrap();
    assert!(
        text.starts_with("TASK T1: First\n\nBody\n\n---\n"),
        "{text}"
    );
    assert!(text.contains(&format!(
        "trace_id: {}",
        v["telemetry"]["trace_id"].as_str().unwrap()
    )));
    assert!(p.calls().is_empty());
    assert!(
        !std::fs::read_to_string(p.path("host-calls"))
            .unwrap()
            .contains("agent")
    );
}

#[test]
fn returning_a_run_records_the_committed_reason_on_that_trace() {
    let p = Project::new("");
    p.enable(true);
    let host = p.host();
    let sent = p.dispatch(Some(true), Some(&host));
    assert_eq!(sent["telemetry"]["status"], "context", "{sent}");
    let trace = sent["telemetry"]["trace_id"].as_str().unwrap();
    p.transition(Transition::Submit, Some(&host));
    let returned = p.transition(Transition::Return, Some(&host));
    assert_eq!(returned["state"], "pending");
    assert_eq!(returned["telemetry"]["status"], "stored", "{returned}");
    assert_eq!(
        returned["telemetry"]["close"]["status"], "stored",
        "{returned}"
    );
    let events = p.events(trace);
    assert_eq!(events.last().unwrap()["payload"]["closed"], true);
    let event = events
        .iter()
        .find(|e| e["kind"] == "task.transition" && e["payload"]["to"] == "pending")
        .unwrap();
    assert_eq!(event["payload"]["from"], "awaiting_release");
    assert_eq!(event["bodies"][0]["role"], "reason", "{event}");
    let out = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args([
            "telemetry",
            "body",
            "--sha256",
            event["bodies"][0]["sha256"].as_str().unwrap(),
        ])
        .env("XDG_STATE_HOME", p.path("state"))
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, b"needs revision");
    assert_eq!(p.calls().len(), 1, "return must never send again");
    let next = p.dispatch(Some(true), Some(&host));
    assert_eq!(next["telemetry"]["status"], "context", "{next}");
    assert_ne!(next["run_id"], sent["run_id"]);
    assert_ne!(next["telemetry"]["trace_id"], trace);
    assert!(
        p.events(next["telemetry"]["trace_id"].as_str().unwrap())
            .iter()
            .filter(|e| e["kind"] == "task.transition")
            .all(|e| e["bodies"].as_array().unwrap().is_empty())
    );
}

fn append_report(p: &Project, sent: &Value, id: &str, review: bool, text: &str) {
    assert_eq!(sent["telemetry"]["status"], "context", "{sent}");
    let body = p.path(&format!("{id}.txt"));
    std::fs::write(&body, text).unwrap();
    let event = json!({"schema_version":1,"event_id":id,"trace_id":sent["telemetry"]["trace_id"],
        "dispatch_id": if review {sent["telemetry"]["dispatch_id"].clone()} else {Value::Null},
        "kind":if review {"review.recorded"} else {"controller.note"}, "observed_at":null,
        "producer":"synthetic controller","evidence_kind":"controller_statement",
        "payload":if review {json!({"reviewer":"controller","verdict":"passed"})} else {json!({"note_kind":"closure"})},
        "links":[],"bodies":[{"role":"text","path":body}]});
    let input = p.path("report.json");
    std::fs::write(&input, event.to_string()).unwrap();
    let answer = p.telemetry(&["append", "--input", input.to_str().unwrap()]);
    assert_eq!(answer["status"], "stored", "{answer}");
}

#[test]
fn run_reports_read_only_the_exact_run_and_still_read_when_recording_is_off() {
    use saddle_drover_plugin::telemetry::{binding, reports};
    let p = Project::new("");
    p.enable(true);
    let host = p.host();
    let old = p.dispatch(Some(true), Some(&host));
    append_report(&p, &old, "old-review", true, "OLD RUN ONLY");
    p.transition(Transition::Return, Some(&host));
    let sent = p.dispatch(Some(true), Some(&host));
    let long = format!(
        "Verified three examples\n{}\nREPORT TAIL",
        "中文证据\n".repeat(100)
    );
    append_report(&p, &sent, "current-review", true, &long);
    append_report(
        &p,
        &sent,
        "current-closure",
        false,
        "Cleanup failed; no acceptance claim.",
    );
    p.enable(false);
    let before = core::list(&p.root).unwrap();
    let mut detail: saddle_drover_plugin::drover::Detail = serde_json::from_value(
        core::show(&p.root, "T1", &p.corral(), &AtomicBool::new(false)).unwrap(),
    )
    .unwrap();
    let calls = p.calls().len(); // show may query the configured controller's public status.
    detail.load_reports(Some(&host), &AtomicBool::new(false));
    let found = detail.reports;
    assert_eq!(found.state, "available", "{found:?}");
    assert_eq!(
        found.trace_id.as_deref(),
        sent["telemetry"]["trace_id"].as_str()
    );
    assert_eq!(found.entries.len(), 2);
    assert_eq!(found.entries[0].event_id, "current-review");
    assert_eq!(found.entries[0].verdict.as_deref(), Some("passed"));
    assert_eq!(found.entries[0].text.as_deref(), Some(long.as_str()));
    assert_eq!(
        found.entries[1].text.as_deref(),
        Some("Cleanup failed; no acceptance claim.")
    );
    assert_eq!(
        core::list(&p.root).unwrap(),
        before,
        "query cannot submit or accept"
    );
    assert_eq!(p.calls().len(), calls, "query cannot deliver");
    for (scope, task, run) in [
        ("/other-project", "T1", sent["run_id"].as_str().unwrap()),
        (
            p.root.to_str().unwrap(),
            "T2",
            sent["run_id"].as_str().unwrap(),
        ),
        (p.root.to_str().unwrap(), "T1", "missing-run"),
    ] {
        let empty = reports(
            Some(&host),
            &binding(scope, task, run),
            &AtomicBool::new(false),
        );
        assert_eq!(empty.state, "not_recorded", "{empty:?}");
        assert!(empty.entries.is_empty());
    }
}

#[test]
fn report_queries_do_not_initialize_a_store_or_hide_query_errors_as_no_records() {
    use saddle_drover_plugin::telemetry::{binding, reports};
    let p = Project::new("");
    let host = p.host();
    let b = binding(p.root.to_str().unwrap(), "T1", "unrecorded-run");
    assert_eq!(
        reports(Some(&host), &b, &AtomicBool::new(false)).state,
        "not_recorded"
    );
    assert!(
        !p.path("state").exists(),
        "read must not initialize storage"
    );
    let broken = fixture::script(
        p.dir.path(),
        "broken-host",
        "#!/bin/sh\necho '{\"ok\":false}'\nexit 1\n",
    );
    let failure = reports(Some(Path::new(&broken)), &b, &AtomicBool::new(false));
    assert_eq!(failure.state, "unavailable");
    assert!(failure.entries.is_empty());
    assert_eq!(
        reports(None, &b, &AtomicBool::new(false)).state,
        "unavailable"
    );
}

fn terminal_host(p: &Project, mode: &str) -> PathBuf {
    fixture::script(p.dir.path(), "terminal-host", &format!(r#"#!/usr/bin/python3
import json, sys, time
from pathlib import Path
root = Path({root:?})
mode = {mode:?}
args = sys.argv[1:]
data = json.loads(Path(args[args.index('--input')+1]).read_text()) if '--input' in args else None
committed = json.loads((root/'project/data/tasks.state').read_text().splitlines()[-1])
reason = Path(data['bodies'][0]['path']).read_text() if data and data.get('bodies') else None
with (root/'terminal-calls').open('a') as out:
    out.write(json.dumps({{'args':args,'input':data,'committed':committed,'reason':reason}})+'\n')
if args[1] == 'list':
    if mode == 'lookup_failure':
        print(json.dumps({{'ok':False,'status':'unavailable'}}))
    else:
        print(json.dumps({{'ok':True,'traces':[] if mode == 'missing' else [{{'trace_id':'synthetic-run-trace'}}]}}))
elif args[1] == 'append':
    if mode == 'append_timeout': time.sleep(3)
    print(json.dumps({{'ok':mode != 'append_failure','status':'disabled' if mode == 'append_failure' else 'stored'}}))
elif args[1:3] == ['trace','close']:
    if mode == 'close_timeout': time.sleep(3)
    print(json.dumps({{'ok':mode != 'close_failure','status':'unavailable' if mode == 'close_failure' else 'stored'}}))
else:
    raise AssertionError(args)
"#, root=p.dir.path(), mode=mode)).into()
}
fn terminal_calls(p: &Project) -> Vec<Value> {
    std::fs::read_to_string(p.path("terminal-calls"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn terminal_transitions_close_after_committed_reason_and_submit_stays_open() {
    for action in [Transition::Accept, Transition::Return] {
        let p = Project::new("");
        let sent = p.dispatch(Some(false), None);
        let host = terminal_host(&p, "normal");
        let submitted = p.transition(Transition::Submit, Some(&host));
        assert_eq!(submitted["telemetry"]["close"]["status"], "not_attempted");
        assert_eq!(terminal_calls(&p).len(), 2);
        let ended = p.transition(action, Some(&host));
        assert_eq!(ended["record"]["status"], "recorded");
        assert_eq!(ended["telemetry"]["status"], "stored");
        assert_eq!(ended["telemetry"]["close"]["status"], "stored");
        let calls = terminal_calls(&p);
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[2]["args"][1], "list");
        assert_eq!(calls[3]["args"][1], "append");
        assert_eq!(calls[4]["args"][1], "trace");
        assert_eq!(calls[4]["args"][2], "close");
        assert_eq!(calls[3]["input"]["trace_id"], calls[4]["input"]["trace_id"]);
        assert_eq!(
            calls[3]["input"]["payload"]["binding"]["run"],
            sent["run_id"]
        );
        assert_eq!(
            calls[2]["args"].as_array().unwrap().last().unwrap(),
            &sent["run_id"]
        );
        for call in &calls[2..] {
            assert_eq!(
                call["committed"]["ev"],
                if action == Transition::Return {
                    "returned"
                } else {
                    "accepted"
                }
            );
        }
        if action == Transition::Return {
            assert_eq!(calls[3]["reason"], "needs revision");
            assert_eq!(
                calls[4]["committed"]["return_record"]["reason"],
                "needs revision"
            );
        }
        assert_eq!(p.calls().len(), 1, "no business replay");
    }
}

#[test]
fn terminal_close_has_its_own_budget_and_failures_do_not_replay_or_undo_business() {
    for (mode, transition, close) in [
        ("append_failure", "disabled", "stored"),
        ("append_timeout", "budget_exhausted", "stored"),
        ("close_failure", "stored", "unavailable"),
        ("close_timeout", "stored", "budget_exhausted"),
        ("missing", "not_recorded", "not_attempted"),
        ("lookup_failure", "unavailable", "not_attempted"),
    ] {
        let p = Project::new("");
        p.dispatch(Some(false), None);
        let host = terminal_host(&p, mode);
        let start = Instant::now();
        let result = p.transition(Transition::Return, Some(&host));
        let elapsed = start.elapsed();
        assert_eq!(result["state"], "pending", "{mode}: {result}");
        assert_eq!(
            result["telemetry"]["status"], transition,
            "{mode}: {result}"
        );
        assert_eq!(
            result["telemetry"]["close"]["status"], close,
            "{mode}: {result}"
        );
        if mode == "close_timeout" {
            assert!(
                elapsed >= Duration::from_millis(300) && elapsed < Duration::from_secs(1),
                "{elapsed:?}"
            );
        }
        let calls = terminal_calls(&p);
        assert_eq!(
            calls.len(),
            if close == "not_attempted" { 1 } else { 3 },
            "{mode}"
        );
        assert_eq!(
            calls[0]["committed"]["return_record"]["reason"],
            "needs revision"
        );
        assert_eq!(p.calls().len(), 1);
        let state = std::fs::read_to_string(p.root.join("data/tasks.state")).unwrap();
        assert_eq!(
            state
                .lines()
                .filter(|l| serde_json::from_str::<Value>(l).unwrap()["ev"] == "returned")
                .count(),
            1
        );
    }
}
