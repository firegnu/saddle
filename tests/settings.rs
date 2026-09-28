use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use saddle::settings::{Outcome, Settings};
use std::path::Path;

const SAMPLE: &str = include_str!("../config.toml");

fn press(settings: &mut Settings, code: KeyCode) -> Outcome {
    settings.key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn ctrl(settings: &mut Settings, c: char) -> Outcome {
    settings.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}
fn replace(settings: &mut Settings, text: &str) {
    ctrl(settings, 'u');
    settings.paste(text);
}
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn saving_changes_only_the_edited_value_and_keeps_comments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("left_width"), Some("52"));
    replace(&mut settings, "60");
    // Editing alone writes nothing.
    assert_eq!(read(&path), SAMPLE);
    let Outcome::Saved(config, restart) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!(config.left_width, 60);
    assert!(restart.is_empty(), "{restart:?}");
    assert_eq!(
        read(&path),
        SAMPLE.replace(
            "left_width = 52               #",
            "left_width = 60               #"
        )
    );
}

#[test]
fn cancel_discards_the_draft_and_leaves_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    replace(&mut settings, "70");
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    assert_eq!(read(&path), SAMPLE);
}

#[test]
fn first_save_creates_the_file_with_only_the_changed_setting_and_flags_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing/saddle/config.toml");
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("refresh_ms"), Some("1000"));
    press(&mut settings, KeyCode::Down);
    replace(&mut settings, "2500");
    let Outcome::Saved(config, restart) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!(config.refresh_ms, 2500);
    assert_eq!(restart, vec!["Refresh interval"]);
    assert_eq!(read(&path).trim(), "refresh_ms = 2500");
}

#[test]
fn invalid_values_keep_the_draft_and_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    replace(&mut settings, "0");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(
        settings.message().contains("left_width must be positive"),
        "{}",
        settings.message()
    );
    assert_eq!(settings.value("left_width"), Some("0"));
    replace(&mut settings, "wide");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(settings.message().contains("Sidebar width"));
    // A bad color is reported by name, on its own page.
    replace(&mut settings, "52");
    press(&mut settings, KeyCode::F(2));
    replace(&mut settings, "purple-ish");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(settings.message().contains("bg"), "{}", settings.message());
    assert!(settings.message().contains("purple-ish"));
    assert_eq!(read(&path), SAMPLE);
}

#[test]
fn default_restores_one_value_but_only_save_writes_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# mine\nleft_width = 70\n[queue]\ncwd = \"~/p\"\n").unwrap();
    let mut settings = Settings::open(path.clone(), true);
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("left_width"), Some("52"));
    // Automatic (no project) is the default initial project, and removes the key.
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Down);
    assert_eq!(settings.value("queue.cwd"), Some("~/p"));
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("queue.cwd"), Some(""));
    assert_eq!(
        read(&path),
        "# mine\nleft_width = 70\n[queue]\ncwd = \"~/p\"\n"
    );
    let Outcome::Saved(config, restart) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!(config.queue.cwd, None);
    assert_eq!(restart, vec!["Initial project"]);
    assert_eq!(read(&path), "# mine\nleft_width = 52\n[queue]\n");
}

#[test]
fn an_external_change_is_never_overwritten_and_reload_offers_both_choices() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    replace(&mut settings, "60");
    let external = SAMPLE.replace("refresh_ms = 1000", "refresh_ms = 3000");
    std::fs::write(&path, &external).unwrap();
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(settings.conflict(), "{}", settings.message());
    assert_eq!(read(&path), external);
    // Keeping the edits reloads the file underneath them; nothing is saved yet.
    press(&mut settings, KeyCode::Char('k'));
    assert!(!settings.conflict());
    assert_eq!(settings.value("left_width"), Some("60"));
    assert_eq!(settings.value("refresh_ms"), Some("3000"));
    assert_eq!(read(&path), external);
    let Outcome::Saved(config, _) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!((config.left_width, config.refresh_ms), (60, 3000));
    assert_eq!(
        read(&path),
        external.replace(
            "left_width = 52               #",
            "left_width = 60               #"
        )
    );
    // Discarding takes the file as it now is.
    let mut settings = Settings::open(path.clone(), true);
    replace(&mut settings, "44");
    std::fs::write(&path, SAMPLE).unwrap();
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    press(&mut settings, KeyCode::Char('d'));
    assert!(!settings.conflict());
    assert_eq!(settings.value("left_width"), Some("52"));
    assert_eq!(read(&path), SAMPLE);
}

#[test]
fn colors_and_commands_edit_their_existing_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    press(&mut settings, KeyCode::F(2));
    assert_eq!(settings.value("colors.bg"), Some("default"));
    replace(&mut settings, "#102030");
    press(&mut settings, KeyCode::F(3));
    assert_eq!(settings.value("corral"), Some("corral"));
    replace(&mut settings, "~/bin/corral");
    let Outcome::Saved(config, restart) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!(
        config.colors.bg,
        ratatui::style::Color::Rgb(0x10, 0x20, 0x30)
    );
    assert_eq!(restart, vec!["corral command"]);
    let text = read(&path);
    assert!(text.contains("corral = \"~/bin/corral\"              # Executable name"));
    assert!(text.contains("bg = \"#102030\"               # Main surfaces."));
    assert_eq!(text.lines().count(), SAMPLE.lines().count());
}

#[test]
fn saving_without_edits_closes_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut settings = Settings::open(path.clone(), true);
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Cancel));
    assert!(!path.exists());
}

#[test]
fn every_page_and_notice_draws_in_tiny_and_normal_windows() {
    use ratatui::{Terminal, backend::TestBackend};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let theme = saddle::theme::Theme::default();
    for (w, h) in (0..24)
        .flat_map(|w| (0..14).map(move |h| (w, h)))
        .chain([(80, 24), (160, 48)])
    {
        let mut settings = Settings::open(path.clone(), false);
        for step in [KeyCode::F(1), KeyCode::F(2), KeyCode::F(3), KeyCode::Null] {
            if step == KeyCode::Null {
                replace(&mut settings, "1");
                std::fs::write(&path, "").unwrap();
                ctrl(&mut settings, 's');
                assert!(settings.conflict());
                std::fs::write(&path, SAMPLE).unwrap();
            } else {
                press(&mut settings, step);
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
fn saving_through_a_symlink_keeps_the_link_and_the_file_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("dotfiles/saddle.toml");
    std::fs::create_dir_all(real.parent().unwrap()).unwrap();
    std::fs::write(&real, SAMPLE).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = dir.path().join("config.toml");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let mut settings = Settings::open(link.clone(), true);
    replace(&mut settings, "61");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Saved(..)));
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(read(&real).contains("left_width = 61"));
    let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn a_failed_reload_after_keep_my_edits_still_keeps_them_on_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "left_width = 52\nrefresh_ms = 1000\n").unwrap();
    let mut settings = Settings::open(path.clone(), true);
    replace(&mut settings, "60");
    std::fs::write(&path, "left_width = [\n").unwrap();
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(settings.conflict());
    // Keep my edits, but the file cannot be read as a config yet.
    press(&mut settings, KeyCode::Char('k'));
    assert_eq!(settings.value("left_width"), Some("60"));
    let repaired = "left_width = 52\nrefresh_ms = 3000\n";
    std::fs::write(&path, repaired).unwrap();
    ctrl(&mut settings, 'r');
    assert_eq!(settings.value("left_width"), Some("60"));
    assert_eq!(settings.value("refresh_ms"), Some("3000"));
    assert_eq!(read(&path), repaired);
    let Outcome::Saved(config, _) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!((config.left_width, config.refresh_ms), (60, 3000));
}

#[test]
fn a_dangling_config_link_is_kept_and_saving_reports_failure() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("dotfiles/saddle.toml");
    let link = dir.path().join("config.toml");
    std::os::unix::fs::symlink(&missing, &link).unwrap();
    let mut settings = Settings::open(link.clone(), true);
    replace(&mut settings, "61");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Stay));
    assert!(
        settings.message().starts_with("Not saved"),
        "{}",
        settings.message()
    );
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read_link(&link).unwrap(), missing);
    assert!(!missing.exists());
    assert_eq!(settings.value("left_width"), Some("61"));
}

/// The General page's Task notifications row, three fields down.
fn to_notifications(settings: &mut Settings) {
    for _ in 0..3 {
        press(settings, KeyCode::Down);
    }
}
fn screen(settings: &mut Settings) -> String {
    use ratatui::{Terminal, backend::TestBackend};
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    terminal
        .draw(|frame| {
            settings.draw(&saddle::theme::Theme::default(), frame);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..24)
        .map(|y| {
            (0..90)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn task_notifications_is_a_draft_choice_saved_only_through_drover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    to_notifications(&mut settings);
    // Until Drover answers there is nothing to choose from.
    assert!(screen(&mut settings).contains("Reading Drover"));
    press(&mut settings, KeyCode::Char(' '));
    assert_eq!(settings.value("notifications"), Some(""));
    settings.channel_status(Ok(true));
    assert_eq!(settings.value("notifications"), Some("System"));
    let shown = screen(&mut settings);
    assert!(shown.contains("Task notifications"), "{shown}");
    assert!(
        shown.contains("System") && shown.contains("In saddle"),
        "{shown}"
    );
    assert!(shown.contains("across projects"), "{shown}");
    press(&mut settings, KeyCode::Char(' '));
    assert_eq!(settings.value("notifications"), Some("In saddle"));
    // Cancel sends nothing and writes nothing.
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    let mut settings = Settings::open(path.clone(), true);
    settings.channel_status(Ok(true));
    to_notifications(&mut settings);
    press(&mut settings, KeyCode::Right);
    let Outcome::SetChannel(None, false) = ctrl(&mut settings, 's') else {
        panic!("expected a drover off request: {}", settings.message());
    };
    // The config file is not the place for this setting.
    assert_eq!(read(&path), SAMPLE);
    // While Drover saves, Settings waits and cannot be cancelled into a false state.
    assert!(matches!(press(&mut settings, KeyCode::Esc), Outcome::Stay));
    let Outcome::Done(message) = settings.channel_saved(Ok(false)) else {
        panic!("not closed: {}", settings.message());
    };
    assert!(message.contains("In saddle"), "{message}");
    assert!(message.contains("next notification check"), "{message}");
}

#[test]
fn a_failed_drover_save_keeps_its_draft_and_reports_what_was_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    settings.channel_status(Ok(false));
    replace(&mut settings, "60");
    to_notifications(&mut settings);
    assert_eq!(settings.value("notifications"), Some("In saddle"));
    press(&mut settings, KeyCode::Left);
    // The config part is written first, then Drover is asked.
    let Outcome::SetChannel(Some(config), true) = ctrl(&mut settings, 's') else {
        panic!("expected config and drover on: {}", settings.message());
    };
    assert_eq!(config.left_width, 60);
    let written = read(&path);
    assert!(written.contains("left_width = 60"), "{written}");
    assert!(matches!(
        settings.channel_saved(Err("preferences_write_failed: disk full".into())),
        Outcome::Stay
    ));
    let message = settings.message().to_owned();
    assert!(message.contains("Config saved"), "{message}");
    assert!(
        message.contains("Task notifications not saved") && message.contains("disk full"),
        "{message}"
    );
    assert_eq!(settings.value("notifications"), Some("System"));
    // Saving again retries only Drover; the config is already saved.
    let Outcome::SetChannel(None, true) = ctrl(&mut settings, 's') else {
        panic!("expected a drover retry: {}", settings.message());
    };
    assert_eq!(read(&path), written);
}

#[test]
fn an_unreadable_or_unsupported_preference_is_shown_and_not_editable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut settings = Settings::open(path.clone(), true);
    settings.channel_status(Err("this drover does not support task notifications".into()));
    to_notifications(&mut settings);
    press(&mut settings, KeyCode::Char(' '));
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("notifications"), Some(""));
    let shown = screen(&mut settings);
    assert!(shown.contains("Unavailable"), "{shown}");
    assert!(shown.contains("does not support"), "{shown}");
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Cancel));
}
