mod common;
use saddle_drover_plugin::drover::Client;
#[test]
fn project_registry_preserves_order_deduplicates_and_reports_read_errors() {
    use saddle_drover_plugin::drover::registered_projects;
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
// CLI transport tests were replaced by native storage and public Corral tests below.
use saddle_drover_plugin::{
    core,
    drover::{DetailWorker, Operation, PendingLoad},
};
use serde_json::json;
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
fn project() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir(d.path().join("data")).unwrap();
    fs::write(d.path().join(".drover.conf"), "HANDOFF_DIR=data\n").unwrap();
    fs::write(d.path().join("data/queue.md"), "## T1 First\nBody\n").unwrap();
    d
}
fn client(d: &tempfile::TempDir) -> Client {
    Client {
        corral: "/missing/corral".into(),
        cwd: d.path().into(),
    }
}
#[test]
fn literal_edits_moves_and_deletes_preserve_data_and_reject_stale_selections() {
    let d = project();
    let c = client(&d);
    let stop = AtomicBool::new(false);
    c.execute(
        &Operation::Add {
            title: "Second".into(),
            body: "Body".into(),
        },
        &stop,
    )
    .unwrap();
    let original = c.snapshot().unwrap().pending;
    let edit = Operation::Edit {
        pending: original.clone(),
        index: 1,
        title: "中文 $(touch bad) 'quoted'".into(),
        body: "first\nsecond `touch bad`".into(),
    };
    c.execute(&edit, &stop).unwrap();
    assert!(c.execute(&edit, &stop).is_err());
    assert!(!d.path().join("bad").exists());
    let p = c.snapshot().unwrap().pending;
    assert_eq!(p[1].body, "first\nsecond `touch bad`");
    c.execute(
        &Operation::Move {
            pending: p,
            index: 1,
            to: 0,
        },
        &stop,
    )
    .unwrap();
    let p = c.snapshot().unwrap().pending;
    assert_eq!(p[0].id.as_deref(), Some("T2"));
    c.execute(
        &Operation::Delete {
            pending: p,
            index: 0,
        },
        &stop,
    )
    .unwrap();
    let v = c.snapshot().unwrap();
    assert_eq!(v.pending.len(), 1);
    assert_eq!(v.pending[0].id.as_deref(), Some("T1"));
    assert_eq!(v.history[0].status.as_deref(), Some("dropped"));
    for paused in [true, false] {
        c.execute(&Operation::Pause(paused), &stop).unwrap();
        assert_eq!(c.snapshot().unwrap().paused, paused);
    }
}
#[test]
fn pending_survey_isolates_a_broken_project_and_preserves_order() {
    let a = project();
    let b = project();
    let c = project();
    fs::write(b.path().join("data/tasks.state"), "invalid JSON").unwrap();
    let names: Vec<_> = [&a, &b, &c]
        .iter()
        .map(|d| d.path().display().to_string())
        .collect();
    let load = PendingLoad::start("/missing/corral", &names);
    let mut rows: Vec<_> = (0..3)
        .map(|_| load.updates.recv_timeout(Duration::from_secs(5)).unwrap())
        .collect();
    rows.sort_by_key(|r| r.0);
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(rows[0].1.as_ref().unwrap().len(), 1);
    assert!(rows[1].1.is_err());
    assert_eq!(rows[2].1.as_ref().unwrap().len(), 1);
}
#[test]
fn detail_worker_cancels_corral_queries_and_never_overlaps() {
    let d = project();
    fs::write(
        d.path().join(".drover.conf"),
        "HANDOFF_DIR=data\nMAIN_AGENT=p/main\n",
    )
    .unwrap();
    let corral = common::script(
        d.path(),
        "corral",
        r#"#!/bin/sh
echo start >> calls
[ -f hang ] && exec sleep 30
sleep 0.05
echo end >> calls
echo '{}'
"#,
    );
    let c = Client {
        corral,
        cwd: d.path().into(),
    };
    let calls = || fs::read_to_string(d.path().join("calls")).unwrap_or_default();
    let w = DetailWorker::start(c.clone(), "T1".into(), Duration::from_millis(50));
    for _ in 0..3 {
        let detail = w
            .updates
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(detail.task.id.as_deref(), Some("T1"));
        assert_eq!(detail.evidence.git.state, "unavailable");
    }
    drop(w);
    let log = calls();
    assert!(
        log.lines()
            .collect::<Vec<_>>()
            .chunks(2)
            .all(|v| v == ["start", "end"] || v == ["start"])
    );
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(calls(), log);
    fs::write(d.path().join("hang"), "").unwrap();
    fs::remove_file(d.path().join("calls")).unwrap();
    let w = DetailWorker::start(c, "T1".into(), Duration::from_millis(50));
    let deadline = Instant::now() + Duration::from_secs(5);
    while calls().is_empty() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let began = Instant::now();
    drop(w);
    assert!(began.elapsed() < Duration::from_secs(2));
}
#[test]
fn dispatch_records_delivery_separately_and_never_retries_corral() {
    for (code, ok, confirmed, delivery, recorded) in [
        (0, true, true, "confirmed", true),
        (0, true, false, "unconfirmed", true),
        (3, false, false, "unconfirmed", true),
        (7, false, false, "rejected", false),
        (1, false, false, "unknown", false),
    ] {
        let d = project();
        fs::write(
            d.path().join(".drover.conf"),
            "HANDOFF_DIR=data\nMAIN_AGENT=p/main\n",
        )
        .unwrap();
        let response = json!({"ok":ok,"confirmed":confirmed,"merged_with_draft":false});
        fs::write(d.path().join("answer"), response.to_string()).unwrap();
        let corral = common::script(
            d.path(),
            "corral",
            &format!("#!/bin/sh\necho call >> calls\ncat answer\nexit {code}\n"),
        );
        let v = core::list(d.path()).unwrap();
        let op = Operation::DispatchPending {
            project: d.path().display().to_string(),
            pos: 1,
            token: v["pending"][0]["actions"]["dispatch-pending"]["target_token"]
                .as_str()
                .unwrap()
                .into(),
        };
        let v = core::execute(d.path(), &op, &corral, &AtomicBool::new(false)).unwrap();
        assert_eq!(v["delivery"]["status"], delivery);
        assert_eq!(
            v["record"]["status"],
            if recorded {
                "recorded"
            } else {
                "not_attempted"
            }
        );
        assert_eq!(
            fs::read_to_string(d.path().join("calls")).unwrap(),
            "call\n"
        );
        assert_eq!(
            core::list(d.path()).unwrap()["current"].is_object(),
            recorded
        );
        if recorded {
            assert!(core::execute(d.path(), &op, &corral, &AtomicBool::new(false)).is_err());
            assert_eq!(
                fs::read_to_string(d.path().join("calls")).unwrap(),
                "call\n"
            );
        }
    }
}
