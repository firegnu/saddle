use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn runtime_uses_path_then_config_or_explicit_path_without_bundle_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let host = dir.path().join("saddle");
    fs::copy(std::env::current_exe().unwrap(), &host).unwrap();
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
            .args(["--exact", "runtime_probe", "--nocapture"])
            .env("SADDLE_RESOLVE_PROBE", dir.path().join("probe.json"))
            .env(
                "SADDLE_RESOLVE_EXPLICIT",
                options.get(1).copied().unwrap_or(""),
            )
            .env("HOME", dir.path())
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("XDG_STATE_HOME", dir.path().join("state"))
            .env("PATH", &path_dir)
            .output()
            .unwrap()
    };
    let default = run(&[]);
    assert!(
        default.status.success(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(dir.path().join("probe.json")).unwrap()
        )
        .unwrap()["source"],
        "path"
    );
    fs::remove_file(path_dir.join("corral")).unwrap();
    let missing = run(&[]);
    assert!(
        !missing.status.success(),
        "missing PATH command must not silently invoke the bundle"
    );
    assert!(String::from_utf8_lossy(&missing.stderr).contains("not found on PATH"));
    script(&path_dir.join("corral"), "configured");
    fs::write(
        config.join("config.toml"),
        format!("corral = {:?}\n", path_dir.join("corral")),
    )
    .unwrap();
    assert!(run(&[]).status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(dir.path().join("probe.json")).unwrap()
        )
        .unwrap()["source"],
        "configured"
    );
    script(&bundle, "explicit");
    assert!(
        run(&["--corral", bundle.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(dir.path().join("probe.json")).unwrap()
        )
        .unwrap()["source"],
        "explicit"
    );
    assert!(!dir.path().join("state/saddle/telemetry").exists());
}

#[test]
fn runtime_probe() {
    let Some(output) = std::env::var_os("SADDLE_RESOLVE_PROBE") else {
        return;
    };
    let config = saddle::config::Config::load(&saddle::config::default_path()).unwrap();
    let explicit = std::env::var("SADDLE_RESOLVE_EXPLICIT").unwrap_or_default();
    let command = if explicit.is_empty() {
        &config.corral
    } else {
        &explicit
    };
    let program = match saddle::agent_program::resolve(command) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    let result = Command::new(program)
        .args(["reply", "fake/main"])
        .output()
        .unwrap();
    assert!(result.status.success());
    fs::write(output, result.stdout).unwrap();
}
