mod common;
use saddle::{terminal::Size, viewer::Viewer};
use std::{
    thread,
    time::{Duration, Instant},
};

#[test]
fn choosing_current_agent_again_during_detach_cancels_the_obsolete_switch() {
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "corral",
        r#"#!/usr/bin/env python3
from pathlib import Path
import os, signal, sys, time
root = Path(__file__).parent
def stop(*_):
    time.sleep(0.1)
    sys.exit(0)
signal.signal(signal.SIGINT,stop)
with (root / 'events').open('a') as f: f.write(sys.argv[2] + '\n')
os.write(1,b'READY')
while True: time.sleep(1)
"#,
    );
    let size = Size { rows: 10, cols: 40 };
    let mut viewer = Viewer::new(program);
    viewer.select("p/a".into()).unwrap();
    viewer.tick(size).unwrap();
    let events = temp.path().join("events");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !events.exists() || viewer.showing.as_deref() != Some("p/a") {
        assert!(Instant::now() < deadline);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    viewer.select("p/b".into()).unwrap();
    viewer.select("p/a".into()).unwrap();
    while std::fs::read_to_string(&events).unwrap().lines().count() < 2 {
        assert!(Instant::now() < deadline);
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(viewer.showing.as_deref(), Some("p/a"));
    assert_eq!(std::fs::read_to_string(events).unwrap(), "p/a\np/a\n");
}

#[test]
fn t25_closed_during_identity_check_never_publishes_a_session() {
    use saddle::viewer::AgentMetadata;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(temp.path(), "corral", include_str!("fixtures/corral.py"));
    std::fs::write(temp.path().join("agents.json"), r#"{"p/a":"idle"}"#).unwrap();
    std::fs::write(temp.path().join("hold-status"), "").unwrap();
    let mut viewer = Viewer::new(program);
    viewer
        .select_agent(
            "p/a".into(),
            AgentMetadata {
                cwd: Some(temp.path().display().to_string()),
                instance: Some("abcdef123".into()),
            },
        )
        .unwrap();
    let size = Size { rows: 10, cols: 40 };
    viewer.tick(size).unwrap();
    let events = || std::fs::read_to_string(temp.path().join("events")).unwrap_or_default();
    let deadline = Instant::now() + Duration::from_secs(4);
    while !events().contains("status p/a") {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    viewer.close().unwrap();
    std::fs::remove_file(temp.path().join("hold-status")).unwrap();
    while !viewer.closed() {
        assert!(Instant::now() < deadline, "{}", events());
        viewer.tick(size).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    assert!(viewer.session.is_none());
    assert!(viewer.showing.is_none());
    assert!(!events().contains("stop "));
}
