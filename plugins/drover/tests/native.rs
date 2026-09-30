use saddle_drover_plugin::drover::Client;
#[test]
fn task_data_is_read_without_an_installed_drover_cli() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(
        dir.path().join(".drover.conf"),
        format!("HANDOFF_DIR={}\n", data.display()),
    )
    .unwrap();
    std::fs::write(data.join("queue.md"), "## T1 Native task\nOriginal body\n").unwrap();
    let client = Client {
        corral: "/missing/retired-drover".into(),
        cwd: dir.path().into(),
    };
    let snapshot = client
        .snapshot()
        .expect("plugin must own reads without the retired CLI");
    assert_eq!(snapshot.pending[0].id.as_deref(), Some("T1"));
    assert_eq!(snapshot.pending[0].body, "Original body");
}

use saddle_drover_plugin::{
    core,
    drover::{Operation, Transition},
};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
fn project() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("data")).unwrap();
    std::fs::write(d.path().join(".drover.conf"), "HANDOFF_DIR=data\n").unwrap();
    std::fs::write(
        d.path().join("data/queue.md"),
        "## T1 First\nOriginal body\n\n## T2 Second\nSecond body\n",
    )
    .unwrap();
    d
}
fn dispatch(d: &tempfile::TempDir, pos: usize) -> Value {
    let v = core::list(d.path()).unwrap();
    core::execute(
        d.path(),
        &Operation::DispatchPending {
            project: d.path().display().to_string(),
            pos: pos as u64,
            token: v["pending"][pos - 1]["actions"]["dispatch-pending"]["target_token"]
                .as_str()
                .unwrap()
                .into(),
        },
        "/missing/corral",
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn transition(
    d: &tempfile::TempDir,
    action: Transition,
    token: Option<String>,
) -> anyhow::Result<Value> {
    let v = core::list(d.path())?;
    let t = if v["current"].is_object() {
        &v["current"]
    } else if v["awaiting"].is_object() {
        &v["awaiting"]
    } else {
        &v["history"][0]
    };
    core::execute(
        d.path(),
        &Operation::Transition {
            project: d.path().display().to_string(),
            id: t["id"].as_str().unwrap().into(),
            run_id: t["run_id"].as_str().unwrap().into(),
            token: token.unwrap_or_else(|| {
                t["actions"][action.command()]["target_token"]
                    .as_str()
                    .unwrap()
                    .into()
            }),
            action,
            reason: "needs revision".into(),
        },
        "/missing/corral",
        &AtomicBool::new(false),
    )
}
#[test]
fn explicit_submission_acceptance_and_return_preserve_history_without_git_or_cli() {
    let d = project();
    let first = dispatch(&d, 1);
    assert_eq!(first["state"], "running");
    assert!(
        first["manual_text"]
            .as_str()
            .unwrap()
            .contains("Original body")
    );
    let v = core::list(d.path()).unwrap();
    let old = v["current"]["actions"]["done"]["target_token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(transition(&d, Transition::Accept, Some(old.clone())).is_err());
    transition(&d, Transition::Return, None).unwrap();
    let v = core::list(d.path()).unwrap();
    assert_eq!(v["pending"][0]["id"], "T1");
    assert_eq!(v["paused"], false);
    dispatch(&d, 2);
    assert!(transition(&d, Transition::Submit, Some(old)).is_err());
    transition(&d, Transition::Submit, None).unwrap();
    let v = core::list(d.path()).unwrap();
    assert!(v["current"].is_null());
    assert_eq!(v["awaiting"]["id"], "T2");
    assert_eq!(v["pending"][0]["id"], "T1");
    transition(&d, Transition::Accept, None).unwrap();
    let v = core::list(d.path()).unwrap();
    assert_eq!(v["history"][0]["id"], "T2");
    assert!(v["history"][0]["t2"].is_number());
    assert!(v["current"].is_null());
    dispatch(&d, 1);
    let v = core::list(d.path()).unwrap();
    assert_eq!(
        v["current"]["previous_runs"][0]["return_history"][0]["reason"],
        "needs revision"
    );
}
#[test]
fn old_completion_does_not_invent_acceptance_and_bad_new_acceptance_is_rejected() {
    let d = project();
    let p = d.path().join("data/tasks.state");
    let start = json!({"ev":"start","id":"T1","title":"First","key":"First","t":1.0,"sha":"abc","main":"def"});
    let done = json!({"ev":"done","id":"T1","gate":false,"t":2.0});
    std::fs::write(&p, format!("{start}\n{done}\n")).unwrap();
    let before = std::fs::read(&p).unwrap();
    let v = core::list(d.path()).unwrap();
    assert_eq!(v["history"][0]["status"], "done");
    assert!(v["history"][0].get("t2").is_none());
    assert_eq!(std::fs::read(&p).unwrap(), before);
    let run = v["history"][0]["run_id"].clone();
    let accepted = json!({"ev":"accepted","id":"T1","run_id":run,"t":3});
    std::fs::write(&p, format!("{start}\n{done}\n{accepted}\n")).unwrap();
    assert!(core::list(d.path()).is_err());
    std::fs::write(
        &p,
        format!("{start}\n{done}\n{{\"ev\":\"go\",\"id\":\"T1\",\"t\":3}}\n"),
    )
    .unwrap();
    assert_eq!(core::list(d.path()).unwrap()["history"][0]["t2"], 3);
}
#[test]
fn edits_are_guarded_and_do_not_change_active_tasks() {
    let d = project();
    dispatch(&d, 1);
    let v = core::list(d.path()).unwrap();
    let pending = serde_json::from_value(v["pending"].clone()).unwrap();
    let op = Operation::Edit {
        pending,
        index: 0,
        title: "Updated".into(),
        body: "New body".into(),
    };
    core::execute(d.path(), &op, "/missing/corral", &AtomicBool::new(false)).unwrap();
    assert!(core::execute(d.path(), &op, "/missing/corral", &AtomicBool::new(false)).is_err());
    let v = core::list(d.path()).unwrap();
    assert_eq!(v["current"]["title"], "First");
    assert_eq!(v["pending"][0]["title"], "Updated");
    assert!(
        core::execute(
            d.path(),
            &Operation::Add {
                title: "Bad".into(),
                body: "## hidden task".into()
            },
            "/missing/corral",
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[test]
fn command_list_is_bounded_and_edits_reject_a_stale_queue_token() {
    use saddle_drover_plugin::api;
    let d = project();
    let p = json!({"project":d.path()});
    let stop = AtomicBool::new(false);
    let body = "large body\n".repeat(10000);
    std::fs::write(
        d.path().join("data/queue.md"),
        format!("## T1 Large\n{body}"),
    )
    .unwrap();
    let v = api::call("/missing/corral", "list", &p, &stop).unwrap();
    assert!(
        v.to_string().len() < 48000,
        "RPC list must not include all task bodies"
    );
    assert!(v["queue_token"].is_string());
    let mut edit = json!({"project":d.path(),"pos":1,"title":"Edited","body":"new","queue_token":v["queue_token"]});
    api::call("/missing/corral", "edit", &edit, &stop).unwrap();
    assert!(api::call("/missing/corral", "edit", &edit, &stop).is_err());
    let fresh = api::call("/missing/corral", "list", &p, &stop).unwrap();
    edit["queue_token"] = fresh["queue_token"].clone();
    api::call("/missing/corral", "drop", &edit, &stop).unwrap();
    assert!(
        core::list(d.path()).unwrap()["pending"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn concurrent_writes_and_changed_then_restored_queue_cannot_reuse_a_token() {
    use std::os::fd::AsRawFd;
    let d = project();
    let stop = AtomicBool::new(false);
    let before = core::list(d.path()).unwrap();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(d.path().join("data/.tasks.lock"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let op = Operation::Add {
        title: "blocked".into(),
        body: String::new(),
    };
    assert!(
        format!(
            "{:#}",
            core::execute(d.path(), &op, "/missing/corral", &stop).unwrap_err()
        )
        .contains("state_busy")
    );
    assert_eq!(core::list(d.path()).unwrap(), before);
    drop(lock);
    let path = d.path().join("data/queue.md");
    let raw = std::fs::read_to_string(&path).unwrap();
    core::atomic_write(&path, "## T9 Other\n").unwrap();
    core::atomic_write(&path, &raw).unwrap();
    let p = json!({"project":d.path(),"pos":1,"title":"must not save","queue_token":before["queue_token"]});
    assert!(saddle_drover_plugin::api::call("/missing/corral", "edit", &p, &stop).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), raw);
}
