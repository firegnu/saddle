mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use saddle::{
    diagnostics::{Checks, Probe, Report, check},
    layout_state::Store,
    settings::{Outcome, Settings},
    terminals::Terminals,
};
use std::{
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime},
};

fn press(settings: &mut Settings, code: KeyCode) -> Outcome {
    settings.key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn screen(settings: &mut Settings) -> String {
    use ratatui::{Terminal, backend::TestBackend};
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    terminal
        .draw(|frame| {
            settings.draw(&saddle::theme::Theme::default(), frame);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..40)
        .map(|y| {
            (0..100)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
/// Nothing recorded yet and the command checks still running.
fn empty_report(config: PathBuf) -> Report {
    Report {
        corral: "corral".into(),
        agents: None,
        config_path: config,
        config_from_file: false,
        layout_path: None,
        restore: None,
        save: None,
        save_off: false,
        checked: SystemTime::now(),
        checks: None,
    }
}

#[test]
fn diagnostics_in_settings_show_the_host_groups_and_never_call_the_unrecorded_a_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut settings = Settings::open(path.clone(), true);
    assert!(matches!(
        press(&mut settings, KeyCode::F(4)),
        Outcome::Diagnose
    ));
    settings.diagnose(empty_report(path.clone()));
    let shown = screen(&mut settings);
    for text in [
        "Diagnostics",
        "Program / commands",
        "Agents",
        "Configuration",
        "Layout",
        "Refresh",
        "Copy summary",
        "Close",
        "checking…",
        "no read yet",
    ] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
    assert!(!shown.contains("OK"), "{shown}");
    // Keys go to the page, not to the hidden settings fields.
    settings.paste("99");
    assert_eq!(settings.value("left_width"), Some("52"));

    settings.checked(Checks {
        corral: Probe {
            path: Ok("/opt/bin/corral".into()),
            version: Some(Ok("0.1.0 (contract 1)".into())),
        },
        config: Ok(false),
    });
    let shown = screen(&mut settings);
    for text in ["/opt/bin/corral", "0.1.0 (contract 1)", "defaults"] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
    let Outcome::Copy(summary) = press(&mut settings, KeyCode::Char('c')) else {
        panic!("no summary");
    };
    for text in [
        "Program / commands",
        "Agents",
        "Configuration",
        "Layout",
        "0.1.0 (contract 1)",
        "no read yet",
    ] {
        assert!(summary.contains(text), "{text} missing:\n{summary}");
    }
    settings.copied(Ok(()));
    assert!(
        settings.message().contains("Copied"),
        "{}",
        settings.message()
    );
    assert!(matches!(
        press(&mut settings, KeyCode::Char('r')),
        Outcome::Diagnose
    ));
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
}

#[test]
fn recorded_results_show_their_errors_and_the_summary_hides_the_home_folder() {
    let home = std::env::var("HOME").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::open(dir.path().join("config.toml"), true);
    press(&mut settings, KeyCode::F(4));
    let at = SystemTime::now();
    settings.diagnose(Report {
        agents: Some((at, Err("corral ls timed out".into()))),
        config_path: format!("{home}/.config/saddle/config.toml").into(),
        config_from_file: true,
        layout_path: Some(format!("{home}/.local/state/saddle/layout.json").into()),
        restore: Some((at, Err("bad layout".into()))),
        save_off: true,
        ..empty_report(PathBuf::new())
    });
    let shown = screen(&mut settings);
    for text in ["corral ls timed out", "bad layout"] {
        assert!(shown.contains(text), "{text} missing:\n{shown}");
    }
    let Outcome::Copy(summary) = press(&mut settings, KeyCode::Char('c')) else {
        panic!("no summary");
    };
    assert!(summary.contains("corral ls timed out"), "{summary}");
    assert!(
        summary.contains("~/.config/saddle/config.toml"),
        "{summary}"
    );
    assert!(!summary.contains(&home), "{summary}");
}

#[test]
fn checks_find_the_commands_and_read_public_versions_within_a_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let corral = common::script(
        dir.path(),
        "corral",
        "#!/bin/sh\n[ \"$1\" = --version ] && echo '{\"ok\": true, \"version\": \"0.1.0\", \"contract\": \"1\"}'\n",
    );
    let slow = common::script(dir.path(), "slow", "#!/bin/sh\nsleep 5\n");
    let config = dir.path().join("config.toml");
    let cancel = AtomicBool::new(false);

    let checks = check(&corral, &config, Duration::from_secs(5), &cancel);
    assert_eq!(checks.corral.path, Ok(PathBuf::from(&corral)));
    assert_eq!(checks.corral.version, Some(Ok("0.1.0 (contract 1)".into())));
    assert_eq!(checks.config, Ok(false));

    std::fs::write(&config, "left_width = 0\n").unwrap();
    let started = Instant::now();
    let checks = check(&slow, &config, Duration::from_millis(200), &cancel);
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        matches!(&checks.corral.version, Some(Err(e)) if e.contains("timed out")),
        "{:?}",
        checks.corral.version
    );
    assert!(checks.config.is_err(), "{:?}", checks.config);

    std::fs::write(&config, "left_width = 40\n").unwrap();
    let checks = check(&corral, &config, Duration::from_secs(5), &cancel);
    assert_eq!(checks.config, Ok(true));
}

#[test]
fn the_layout_store_keeps_its_latest_restore_and_save_results() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    let (mut store, _) = Store::open(Ok(path.clone()));
    assert_eq!(store.path(), Some(path.as_path()));
    assert!(matches!(store.restore, Some((_, Ok(false)))));
    assert!(store.saved.is_none());
    let terminals = Terminals::new("unused-fake-corral".into());
    store.save(&terminals, false);
    assert!(matches!(store.saved, Some((_, Ok(())))));

    let (store, _) = Store::open(Ok(path.clone()));
    assert!(matches!(store.restore, Some((_, Ok(true)))));
    std::fs::write(&path, "{").unwrap();
    let (store, _) = Store::open(Ok(path));
    assert!(matches!(&store.restore, Some((_, Err(_)))));
    assert!(store.protected());
}

#[test]
fn the_diagnostics_page_draws_in_tiny_and_normal_windows() {
    use ratatui::{Terminal, backend::TestBackend};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let theme = saddle::theme::Theme::default();
    for (w, h) in (0..24)
        .flat_map(|w| (0..14).map(move |h| (w, h)))
        .chain([(80, 24), (160, 48)])
    {
        let mut settings = Settings::open(path.clone(), false);
        press(&mut settings, KeyCode::F(4));
        for report in [false, true] {
            if report {
                settings.diagnose(Report {
                    agents: Some((SystemTime::now(), Err("x ".repeat(80)))),
                    ..empty_report(path.clone())
                });
                press(&mut settings, KeyCode::Down);
            }
            Terminal::new(TestBackend::new(w, h))
                .unwrap()
                .draw(|frame| {
                    settings.draw(&theme, frame);
                })
                .unwrap();
        }
    }
}

#[test]
fn a_config_error_keeps_its_place_and_kind_but_not_the_configured_value() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    let cancel = AtomicBool::new(false);
    for (text, kind) in [
        ("left_width = \"SYNTHETIC_PRIVATE_VALUE\"\n", "invalid type"),
        (
            "[colors]\nbg = \"SYNTHETIC_PRIVATE_VALUE\"\n",
            "invalid color",
        ),
        ("refresh_ms = 10\nleft_width = 70000\n", "line 2"),
    ] {
        std::fs::write(&config, text).unwrap();
        let checks = check("true", &config, Duration::from_secs(5), &cancel);
        let error = checks.config.as_ref().unwrap_err();
        assert!(error.contains("line"), "{error}");
        assert!(error.contains(kind), "{error}");
        for value in ["SYNTHETIC_PRIVATE_VALUE", "70000"] {
            assert!(!error.contains(value), "{error}");
        }
        let summary = Report {
            checks: Some(checks),
            ..empty_report(config.clone())
        }
        .summary();
        assert!(!summary.contains("SYNTHETIC_PRIVATE_VALUE"), "{summary}");
    }
}
