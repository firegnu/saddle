use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn headless_uses_its_bundle_then_config_or_explicit_override_without_path_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let host = dir.path().join("saddle");
    fs::copy(env!("CARGO_BIN_EXE_saddle"), &host).unwrap();
    let script = |path: &std::path::Path, marker: &str| {
        fs::write(
            path,
            format!("#!/bin/sh\nprintf '%s\\n' '{{\"ok\":true,\"source\":\"{marker}\"}}'\n"),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    };
    let bundle = dir.path().join("corral");
    script(&bundle, "bundle");
    let path_dir = dir.path().join("path");
    fs::create_dir(&path_dir).unwrap();
    script(&path_dir.join("corral"), "path");
    let config = dir.path().join("config/saddle");
    fs::create_dir_all(&config).unwrap();
    let entry_dir = dir.path().join("entry");
    fs::create_dir(&entry_dir).unwrap();
    let entry = entry_dir.join("saddle");
    std::os::unix::fs::symlink(&host, &entry).unwrap();
    script(&entry_dir.join("corral"), "wrong-entry-neighbor");
    let run = |options: &[&str]| {
        Command::new(&entry)
            .arg("agent")
            .args(options)
            .args(["--", "reply", "fake/main"])
            .env("HOME", dir.path())
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("XDG_STATE_HOME", dir.path().join("state"))
            .env("PATH", &path_dir)
            .output()
            .unwrap()
    };
    let bundled = run(&[]);
    assert!(
        bundled.status.success(),
        "{}",
        String::from_utf8_lossy(&bundled.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bundled.stdout).unwrap()["source"],
        "bundle"
    );
    fs::remove_file(&bundle).unwrap();
    let missing = run(&[]);
    assert!(
        !missing.status.success(),
        "missing bundle must not silently invoke PATH"
    );
    assert!(missing.stdout.is_empty());
    fs::write(
        config.join("config.toml"),
        format!("corral = {:?}\n", path_dir.join("corral")),
    )
    .unwrap();
    let configured = run(&[]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&configured.stdout).unwrap()["source"],
        "path"
    );
    script(&bundle, "explicit");
    let explicit = run(&["--corral", bundle.to_str().unwrap()]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&explicit.stdout).unwrap()["source"],
        "explicit"
    );
    assert!(!dir.path().join("state/saddle/telemetry").exists());
}
