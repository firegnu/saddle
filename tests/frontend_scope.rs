use std::{fs, process::Command};

#[test]
fn retired_commands_are_absent_and_leave_old_data_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let files = [
        "state/saddle/telemetry/telemetry.sqlite3",
        "state/saddle/plugin-resources.json",
        "config/saddle/plugins.toml",
        ".drover/sentinel",
        ".agents/skills/corral-dispatch/SKILL.md",
    ];
    for file in files {
        let path = dir.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"old data: leave untouched").unwrap();
    }
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_saddle"))
            .args(args)
            .env("HOME", dir.path())
            .env("XDG_STATE_HOME", dir.path().join("state"))
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .output()
            .unwrap()
    };
    let help = run(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap().to_lowercase();
    for retired in ["plugin", "telemetry", "saddle agent", "drover"] {
        assert!(
            !text.contains(retired),
            "{retired} still advertised: {text}"
        );
    }
    for args in [
        vec!["plugin", "status"],
        vec!["telemetry", "settings", "get"],
        vec!["agent", "--help"],
        vec![
            "ctl", "plugin", "--plugin", "drover", "--method", "list", "--params", "{}",
        ],
    ] {
        let out = run(&args);
        assert!(!out.status.success(), "{args:?}");
    }
    let ctl = run(&["ctl", "--help"]);
    assert!(ctl.status.success());
    let text = String::from_utf8(ctl.stdout).unwrap();
    for kept in ["inspect", "open", "close"] {
        assert!(text.contains(kept), "{text}");
    }
    assert!(!text.contains("plugin"), "{text}");
    for file in files {
        assert_eq!(
            fs::read(dir.path().join(file)).unwrap(),
            b"old data: leave untouched"
        );
    }
}
