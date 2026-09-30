use std::process::Command;

#[test]
fn ctl_instances_returns_json_without_starting_a_tui() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args(["ctl", "instances"])
        .env("SADDLE_RUNTIME_DIR", dir.path().join("run"))
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .env_remove("SADDLE_INSTANCE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["instances"], serde_json::json!([]));
    assert!(!output.stdout.contains(&0x1b));
}

#[test]
fn plugin_command_is_a_generic_control_operation() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_saddle"))
        .args([
            "ctl",
            "plugin",
            "--plugin",
            "sample",
            "--method",
            "list",
            "--params",
            "{}",
            "--instance",
            "0000000000000001",
            "--request-id",
            "read-one",
        ])
        .env("SADDLE_RUNTIME_DIR", dir.path().join("run"))
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "instance_unavailable", "{value}");
}
