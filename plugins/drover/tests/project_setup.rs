use saddle_drover_plugin::api;
use serde_json::json;
use std::{process::Command, sync::atomic::AtomicBool};

// A separate process owns HOME; no process-wide environment mutation in parallel tests.
#[test]
fn project_setup_isolated() {
    if std::env::var_os("SADDLE_SETUP_TEST").is_none() {
        let home = tempfile::tempdir().unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "project_setup_isolated", "--nocapture"])
            .env("HOME", home.path())
            .env("SADDLE_SETUP_TEST", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let repo = tempfile::tempdir().unwrap();
    let p = repo.path().to_str().unwrap();
    let call =
        |method: &str, value| api::call("/missing/corral", method, &value, &AtomicBool::new(false));
    let info = call("project-info", json!({"project":p}))
        .expect("project inspection must be available without initializing data");
    assert_eq!(info["state"], "new");
    assert!(!repo.path().join(".drover.conf").exists());
    assert!(
        !std::path::Path::new(&std::env::var("HOME").unwrap())
            .join(".drover")
            .exists()
    );
    let saved = call(
        "project-save",
        json!({"project":p,"name":"sample","main_agent":"demo/main","token":info["token"]}),
    )
    .unwrap();
    assert_eq!(saved["ok"], true);
    assert!(
        std::fs::read_to_string(repo.path().join(".gitignore"))
            .unwrap()
            .contains(".drover.conf")
    );
    let info = call("project-info", json!({"project":p})).unwrap();
    assert_eq!(info["state"], "registered");
    assert_eq!(info["main_agent"], "demo/main");
    assert!(!repo.path().join("AGENTS.md").exists());
    let conf = repo.path().join(".drover.conf");
    let original = std::fs::read_to_string(&conf).unwrap();
    std::fs::write(
        &conf,
        format!("{original}TELEMETRY_RECORD=off\n# keep me\n"),
    )
    .unwrap();
    assert!(
        call(
            "project-save",
            json!({"project":p,"main_agent":"changed/main","token":info["token"]})
        )
        .is_err(),
        "stale form must not overwrite external edits"
    );
    let info = call("project-info", json!({"project":p})).unwrap();
    call(
        "project-save",
        json!({"project":p,"main_agent":"","token":info["token"]}),
    )
    .unwrap();
    let config = std::fs::read_to_string(&conf).unwrap();
    assert!(config.contains("MAIN_AGENT=\n"));
    assert!(config.ends_with("TELEMETRY_RECORD=off\n# keep me\n"));
    let data = std::path::PathBuf::from(info["data_dir"].as_str().unwrap());
    std::fs::write(data.join("queue.md"), "## T1 Keep task\nBody\n").unwrap();
    let before = std::fs::read(&conf).unwrap();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data.join(".tasks.lock"))
        .unwrap();
    use std::os::fd::AsRawFd;
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let fresh = call("project-info", json!({"project":p})).unwrap();
    assert!(
        call(
            "project-save",
            json!({"project":p,"main_agent":"new/main","token":fresh["token"]})
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&conf).unwrap(), before);
    assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_UN) }, 0);
    drop(lock);
    call(
        "project-save",
        json!({"project":p,"main_agent":"new/main","token":fresh["token"]}),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(data.join("queue.md")).unwrap(),
        "## T1 Keep task\nBody\n"
    );
    assert!(!data.join("tasks.state").exists());
    let collision = tempfile::tempdir().unwrap();
    let info = call("project-info", json!({"project":collision.path()})).unwrap();
    assert!(
        call(
            "project-save",
            json!({"project":collision.path(),"name":"sample","token":info["token"]})
        )
        .is_err()
    );
    assert!(!collision.path().join(".drover.conf").exists());
    let target = collision.path().join("ignored-user-file");
    std::fs::write(&target, "leave alone\n").unwrap();
    std::os::unix::fs::symlink(&target, collision.path().join(".gitignore")).unwrap();
    let saved = call(
        "project-save",
        json!({"project":collision.path(),"name":"other","token":info["token"]}),
    )
    .unwrap();
    assert!(saved["warning"].as_str().unwrap().contains("not updated"));
    assert_eq!(std::fs::read_to_string(target).unwrap(), "leave alone\n");
    assert_eq!(
        call("project-info", json!({"project":collision.path()})).unwrap()["state"],
        "registered"
    );
    let broken = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("missing", broken.path().join(".drover.conf")).unwrap();
    assert_eq!(
        call("project-info", json!({"project":broken.path()})).unwrap()["state"],
        "unavailable"
    );
    // A valid old config not in the registry is reused without changing its binding or data.
    let old = tempfile::tempdir().unwrap();
    std::fs::write(old.path().join(".drover.conf"), &original).unwrap();
    let q = old.path().to_str().unwrap();
    let info = call("project-info", json!({"project":q})).unwrap();
    assert_eq!(info["state"], "existing");
    call(
        "project-save",
        json!({"project":q,"token":info["token"],"main_agent":"do-not-write"}),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(old.path().join(".drover.conf")).unwrap(),
        original
    );
    std::fs::write(&conf, "HANDOFF_DIR=/missing/data\n").unwrap();
    let info = call("project-info", json!({"project":p})).unwrap();
    assert_eq!(info["state"], "unavailable");
    assert_eq!(info["registered"], true);
    assert!(call("project-save", json!({"project":p,"token":info["token"]})).is_err());
}

#[test]
fn projects_offer_add_and_settings_in_the_ui() {
    use ratatui::{Terminal, backend::TestBackend};
    use saddle_drover_plugin::queue::{Page, Panel};
    let mut panel = Panel::default();
    panel.page = Page::Projects;
    panel.projects = vec!["/tmp/demo".into()];
    for (width, height) in [(48, 24), (80, 24), (160, 50)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                panel.draw(&Default::default(), f, f.area());
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        for label in ["Projects", "Add project", "Settings", "/tmp/demo"] {
            assert!(text.contains(label), "{width}x{height}: {label}");
        }
        assert!(!text.contains("Record default"));
        assert!(!text.contains("Pause p"));
        assert_eq!(panel.project_rows.len(), 1);
        for (row, _) in &panel.project_rows {
            assert!(
                terminal
                    .backend()
                    .buffer()
                    .area
                    .contains(ratatui::layout::Position::new(row.x, row.y))
            );
        }
    }
}
