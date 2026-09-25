mod common;
use saddle::corral::Client;

#[test]
fn public_json_merges_status_with_listing_and_keeps_starting_agents() {
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "corral",
        r##"#!/bin/sh
case "$1:$2" in
  ls:) echo '{"agents":[{"name":"demo/z","starting":true,"cwd":"/tmp/z"},{"name":"demo/a","cwd":"/tmp/a","instance":"abc123456"},{"name":"demo/old","incompatible":true,"proto":99}]}' ;;
  status:demo/a) echo '{"ok":true,"name":"demo/a","kind":"claude","state":"working","last_tool":"Bash","turn_started":100.0,"last_output":110.0,"attached":1,"last_input_source":"human","title":"中文标题","instance":"abc123456"}' ;;
  *) echo '{"ok":false,"error":"unexpected_command"}'; exit 1 ;;
esac
"##,
    );
    let agents = Client { program }.collect().unwrap();
    assert_eq!(agents.len(), 3);
    let a = &agents[0];
    assert_eq!(a.name, "demo/a");
    assert_eq!(a.cwd.as_deref(), Some("/tmp/a"));
    assert_eq!(a.state.as_deref(), Some("working"));
    assert_eq!(a.attached, 1);
    assert_eq!(a.title.as_deref(), Some("中文标题"));
    assert!(agents[1].incompatible);
    assert!(agents[2].starting);
    assert!(agents[2].error.is_none());
}

#[test]
fn command_errors_and_timeouts_are_not_empty_successful_lists() {
    use std::{
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "bad",
        "#!/bin/sh\necho '{\"ok\":false,\"error\":\"unavailable\"}'\nexit 1\n",
    );
    assert!(
        Client { program }
            .collect()
            .unwrap_err()
            .to_string()
            .contains("unavailable")
    );
    let program = common::script(
        temp.path(),
        "slow",
        "#!/usr/bin/env python3\nimport time\ntime.sleep(30)\n",
    );
    let start = Instant::now();
    let result =
        Client { program }.json(&["ls"], Duration::from_millis(100), &AtomicBool::new(false));
    assert!(result.unwrap_err().to_string().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(2));
}
