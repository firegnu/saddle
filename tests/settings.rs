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
    // A bad color is reported by name, on its own page, below the theme.
    replace(&mut settings, "52");
    press(&mut settings, KeyCode::F(2));
    press(&mut settings, KeyCode::Down);
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
    assert_eq!(settings.value("queue.cwd"), None);
    assert_eq!(
        read(&path),
        "# mine\nleft_width = 70\n[queue]\ncwd = \"~/p\"\n"
    );
    let Outcome::Saved(config, restart) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!(config.queue.as_ref().unwrap()["cwd"].as_str(), Some("~/p"));
    assert!(restart.is_empty());
    assert_eq!(
        read(&path),
        "# mine\nleft_width = 52\n[queue]\ncwd = \"~/p\"\n"
    );
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
    let original = "corral = \"corral\"              # Executable name or path.\n\n[colors]\nbg = \"default\"               # Main surfaces.\nfocus = \"yellow\"\n";
    std::fs::write(&path, original).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    press(&mut settings, KeyCode::F(2));
    press(&mut settings, KeyCode::Down);
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
    assert_eq!(text.lines().count(), original.lines().count());
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
fn colors_preview_groups_examples_and_separates_them_from_real_controls() {
    use ratatui::{Terminal, backend::TestBackend};
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::open(dir.path().join("config.toml"), true);
    press(&mut settings, KeyCode::F(2));
    for width in [76, 140] {
        let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
        terminal
            .draw(|frame| {
                settings.draw(&saddle::theme::Theme::default(), frame);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..40)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        let preview = rows
            .iter()
            .position(|row| row.contains("Preview · unsaved colors"))
            .unwrap();
        assert!(rows[preview + 1].contains("Status  ● Working   ○ Idle   ? Waiting   ! Error"));
        assert!(rows[preview + 2].contains("Text    Normal   Muted   Code   Heading"));
        assert!(rows[preview + 3].trim_matches(['┃', ' ']).is_empty());
        assert!(rows[preview + 4].contains("Save Ctrl-S"));
        assert!(!rows[preview..preview + 4].join("\n").contains("‹Save›"));
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

#[test]
fn page_tabs_fit_one_row_at_normal_width_and_wrap_compactly_when_narrow() {
    use ratatui::{Terminal, backend::TestBackend};
    let dir = tempfile::tempdir().unwrap();
    for width in [76, 100, 40, 24] {
        let mut settings = Settings::open(dir.path().join("config.toml"), true);
        let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
        let mut tabs = Vec::new();
        terminal
            .draw(|frame| {
                tabs = settings
                    .draw(&saddle::theme::Theme::default(), frame)
                    .into_iter()
                    .filter(|h| matches!(h.key.code, KeyCode::F(1..=5)))
                    .collect();
            })
            .unwrap();
        assert_eq!(
            tabs.len(),
            5,
            "width {width}: all pages must remain clickable"
        );
        for (i, hit) in tabs.iter().enumerate() {
            assert_eq!(hit.area.height, 1, "width {width}: compact tabs");
            assert!(hit.area.right() <= width);
            assert!(
                tabs[..i]
                    .iter()
                    .all(|other| !hit.area.intersects(other.area))
            );
        }
        if width >= 76 {
            assert!(
                tabs.iter().all(|h| h.area.y == tabs[0].area.y),
                "Plugins must share the same row as General"
            );
        }
        let buffer = terminal.backend().buffer();
        for (hit, label) in tabs.iter().zip([
            "General F1",
            "Colors F2",
            "Advanced F3",
            "Diagnostics F4",
            "Plugins F5",
        ]) {
            let text: String = (hit.area.x..hit.area.right())
                .map(|x| buffer[(x, hit.area.y)].symbol())
                .collect();
            assert!(
                text.contains(label),
                "{label} is clipped at width {width}: {text}"
            );
        }
    }
}

#[test]
fn mascot_toggle_defaults_on_saves_a_boolean_and_can_be_cancelled_or_reset() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# keep this comment\nleft_width = 52\n").unwrap();
    let original = read(&path);
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot_enabled"), Some("true"));
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Char(' '));
    assert_eq!(settings.value("mascot_enabled"), Some("false"));
    settings.paste("invalid");
    ctrl(&mut settings, 'u');
    assert_eq!(settings.value("mascot_enabled"), Some("false"));
    assert_eq!(read(&path), original);
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot_enabled"), Some("true"));
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Enter);
    let Outcome::Saved(_, restart) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message())
    };
    assert!(restart.is_empty());
    let saved: toml::Value = toml::from_str(&read(&path)).unwrap();
    assert_eq!(saved["mascot_enabled"].as_bool(), Some(false));
    assert!(read(&path).starts_with(&original));
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot_enabled"), Some("false"));
    press(&mut settings, KeyCode::Down);
    press(&mut settings, KeyCode::Down);
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("mascot_enabled"), Some("true"));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Saved(_, _)));
    assert_eq!(
        toml::from_str::<toml::Value>(&read(&path)).unwrap()["mascot_enabled"].as_bool(),
        Some(true)
    );
}

#[test]
fn mascot_choice_cycles_through_the_pets_and_saves_the_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# keep this comment\nleft_width = 52\n").unwrap();
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot"), Some("clawd"));
    for _ in 0..3 {
        press(&mut settings, KeyCode::Down);
    }
    press(&mut settings, KeyCode::Right);
    assert_eq!(settings.value("mascot"), Some("cat"));
    // Typing and pasting do not edit a choice.
    settings.paste("dog");
    press(&mut settings, KeyCode::Char('x'));
    ctrl(&mut settings, 'u');
    assert_eq!(settings.value("mascot"), Some("cat"));
    press(&mut settings, KeyCode::Left);
    assert_eq!(settings.value("mascot"), Some("clawd"));
    press(&mut settings, KeyCode::Left);
    assert_eq!(
        settings.value("mascot"),
        Some("capybara"),
        "the choice wraps around"
    );
    press(&mut settings, KeyCode::Left);
    let Outcome::Saved(saved, restart) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message())
    };
    assert!(restart.is_empty(), "the pet changes without a restart");
    assert_eq!(saved.mascot, saddle::mascot::Pet::Cat);
    let written: toml::Value = toml::from_str(&read(&path)).unwrap();
    assert_eq!(written["mascot"].as_str(), Some("cat"));
    assert!(read(&path).starts_with("# keep this comment\nleft_width = 52\n"));
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot"), Some("cat"));
    for _ in 0..3 {
        press(&mut settings, KeyCode::Down);
    }
    ctrl(&mut settings, 'd');
    assert_eq!(
        settings.value("mascot"),
        Some("clawd"),
        "Default goes back to Clawd"
    );
}

#[test]
fn capybara_is_the_third_pet_and_saves_cancels_and_defaults_like_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# keep this comment\nleft_width = 52\n").unwrap();
    let pick = |settings: &mut Settings| {
        for _ in 0..3 {
            press(settings, KeyCode::Down);
        }
        press(settings, KeyCode::Right);
        press(settings, KeyCode::Right);
        assert_eq!(settings.value("mascot"), Some("capybara"));
    };
    let mut settings = Settings::open(path.clone(), true);
    pick(&mut settings);
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    assert_eq!(read(&path), "# keep this comment\nleft_width = 52\n");

    let mut settings = Settings::open(path.clone(), true);
    pick(&mut settings);
    let Outcome::Saved(saved, restart) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message())
    };
    assert!(restart.is_empty(), "the pet changes without a restart");
    assert_eq!(saved.mascot.name(), "capybara");
    let written: toml::Value = toml::from_str(&read(&path)).unwrap();
    assert_eq!(written["mascot"].as_str(), Some("capybara"));
    assert!(read(&path).starts_with("# keep this comment\nleft_width = 52\n"));

    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot"), Some("capybara"));
    for _ in 0..3 {
        press(&mut settings, KeyCode::Down);
    }
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("mascot"), Some("clawd"), "Default stays Clawd");
}

#[test]
fn mascot_display_sits_after_the_pet_and_chooses_auto_or_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# keep this comment\nmascot = \"cat\"\n").unwrap();
    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot_display"), Some("auto"));
    for _ in 0..4 {
        press(&mut settings, KeyCode::Down);
    }
    press(&mut settings, KeyCode::Right);
    assert_eq!(settings.value("mascot_display"), Some("blocks"));
    settings.paste("images");
    ctrl(&mut settings, 'u');
    assert_eq!(settings.value("mascot_display"), Some("blocks"));
    press(&mut settings, KeyCode::Right);
    assert_eq!(
        settings.value("mascot_display"),
        Some("auto"),
        "only two choices"
    );
    press(&mut settings, KeyCode::Left);
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    assert_eq!(read(&path), "# keep this comment\nmascot = \"cat\"\n");

    let mut settings = Settings::open(path.clone(), true);
    for _ in 0..4 {
        press(&mut settings, KeyCode::Down);
    }
    press(&mut settings, KeyCode::Right);
    let Outcome::Saved(saved, restart) = ctrl(&mut settings, 's') else {
        panic!("{}", settings.message())
    };
    assert!(restart.is_empty(), "the display changes without a restart");
    assert_eq!(saved.mascot, saddle::mascot::Pet::Cat, "the pet is its own");
    let written: toml::Value = toml::from_str(&read(&path)).unwrap();
    assert_eq!(written["mascot_display"].as_str(), Some("blocks"));
    assert_eq!(written["mascot"].as_str(), Some("cat"));
    assert!(read(&path).starts_with("# keep this comment\nmascot = \"cat\"\n"));

    let mut settings = Settings::open(path.clone(), true);
    assert_eq!(settings.value("mascot_display"), Some("blocks"));
    for _ in 0..4 {
        press(&mut settings, KeyCode::Down);
    }
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("mascot_display"), Some("auto"));
    assert_eq!(settings.value("mascot"), Some("cat"));
}

fn color_name(theme: &saddle::theme::Theme, name: &str) -> String {
    let mut theme = theme.clone();
    let color = theme
        .named_mut()
        .into_iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1;
    saddle::theme::color_name(*color)
}
/// Colors opens on the Theme row; Interface lists bg … focus first.
fn select_focus(settings: &mut Settings) {
    press(settings, KeyCode::F(2));
    for _ in 0..9 {
        press(settings, KeyCode::Down);
    }
}

#[test]
fn choosing_another_theme_loads_its_colors_and_clears_overrides_in_the_draft() {
    use saddle::theme::Preset;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let original = "# mine\nleft_width = 52\n\n[colors]\n# Interface\nfocus = \"red\"   # was mine\nborder = \"dark_gray\"\n";
    std::fs::write(&path, original).unwrap();
    let tide = Preset::Tide.theme();
    let mut settings = Settings::open(path.clone(), true);
    press(&mut settings, KeyCode::F(2));
    assert_eq!(settings.value("theme"), Some("dune"));
    assert_eq!(settings.value("colors.focus"), Some("red"));
    press(&mut settings, KeyCode::Right);
    assert_eq!(settings.value("theme"), Some("tide"));
    for name in ["focus", "border", "agents_bg", "claude"] {
        assert_eq!(
            settings.value(&format!("colors.{name}")),
            Some(color_name(&tide, name).as_str()),
            "{name}"
        );
    }
    assert!(
        settings.message().contains("Tide"),
        "{}",
        settings.message()
    );
    assert_eq!(read(&path), original);
    // Then a color can be set on top of the new theme.
    press(&mut settings, KeyCode::Down);
    replace(&mut settings, "#102030");
    let Outcome::Saved(config, restart) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert!(restart.is_empty());
    assert_eq!(config.theme, Preset::Tide);
    assert_eq!(config.colors.focus, tide.focus);
    assert_eq!(
        config.colors.bg,
        ratatui::style::Color::Rgb(0x10, 0x20, 0x30)
    );
    assert_eq!(config.overrides.keys().copied().collect::<Vec<_>>(), ["bg"]);
    let text = read(&path);
    assert!(
        text.starts_with("# mine\nleft_width = 52\ntheme = \"tide\"\n"),
        "{text}"
    );
    assert!(text.contains("# Interface"), "{text}");
    assert!(
        !text.contains("focus") && !text.contains("border"),
        "{text}"
    );
    assert!(text.contains("bg = \"#102030\""), "{text}");
}

#[test]
fn the_current_theme_keeps_overrides_and_default_makes_a_color_follow_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    // Equal to Dune's focus, but written by the user: still an override.
    let original = "[colors]\nfocus = \"yellow\"  # mine\ntext = \"default\"\n";
    std::fs::write(&path, original).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    press(&mut settings, KeyCode::F(2));
    // Default on the Theme row while already on Dune clears nothing.
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("theme"), Some("dune"));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Cancel));
    assert_eq!(read(&path), original);
    // Default on a color stops overriding it, even when the value shown stays the same.
    let mut settings = Settings::open(path.clone(), true);
    select_focus(&mut settings);
    ctrl(&mut settings, 'd');
    assert_eq!(settings.value("colors.focus"), Some("yellow"));
    let Outcome::Saved(config, _) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert_eq!(
        config.overrides.keys().copied().collect::<Vec<_>>(),
        ["text"]
    );
    assert_eq!(read(&path), "[colors]\ntext = \"default\"\n");
    // Leaving a theme and coming back is still an explicit choice: overrides are cleared.
    let mut settings = Settings::open(path.clone(), true);
    press(&mut settings, KeyCode::F(2));
    press(&mut settings, KeyCode::Right);
    press(&mut settings, KeyCode::Left);
    assert_eq!(settings.value("theme"), Some("dune"));
    assert!(matches!(ctrl(&mut settings, 's'), Outcome::Saved(..)));
    assert!(!read(&path).contains("text ="), "{}", read(&path));
}

#[test]
fn typing_a_color_overrides_it_and_cancel_drops_a_theme_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut settings = Settings::open(path.clone(), true);
    press(&mut settings, KeyCode::F(2));
    press(&mut settings, KeyCode::Right);
    assert_eq!(settings.value("theme"), Some("tide"));
    assert!(matches!(
        press(&mut settings, KeyCode::Esc),
        Outcome::Cancel
    ));
    assert_eq!(read(&path), SAMPLE);
    // Retyping the theme's own value is an explicit override.
    std::fs::write(&path, "left_width = 52\n").unwrap();
    let mut settings = Settings::open(path.clone(), true);
    select_focus(&mut settings);
    press(&mut settings, KeyCode::Backspace);
    press(&mut settings, KeyCode::Char('w'));
    assert_eq!(settings.value("colors.focus"), Some("yellow"));
    let Outcome::Saved(config, _) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert!(config.overrides.contains_key("focus"));
}

#[test]
fn the_colors_page_shows_the_theme_first_and_marks_custom_colors() {
    use ratatui::{Terminal, backend::TestBackend};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[colors]\nfocus = \"red\"\n").unwrap();
    let mut settings = Settings::open(path, true);
    press(&mut settings, KeyCode::F(2));
    let mut terminal = Terminal::new(TestBackend::new(80, 40)).unwrap();
    terminal
        .draw(|frame| {
            settings.draw(&saddle::theme::Theme::default(), frame);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rows: Vec<String> = (0..40)
        .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect())
        .collect();
    let theme = rows.iter().position(|r| r.contains("Theme")).unwrap();
    assert!(rows[theme].contains("‹ Dune ›"), "{}", rows[theme]);
    let interface = rows.iter().position(|r| r.contains("Interface")).unwrap();
    assert!(theme < interface);
    let focus = rows.iter().find(|r| r.contains(" focus ")).unwrap();
    assert!(
        focus.contains("[red") && focus.contains("custom"),
        "{focus}"
    );
    let bg = rows.iter().find(|r| r.contains(" bg ")).unwrap();
    assert!(!bg.contains("custom"), "{bg}");
}

#[test]
fn removing_the_last_color_override_keeps_the_comment_lines_above_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "left_width = 52\n\n[colors]\n# My accent tweaks.\nfocus = \"red\"  # mine\n",
    )
    .unwrap();
    let mut settings = Settings::open(path.clone(), true);
    select_focus(&mut settings);
    ctrl(&mut settings, 'd');
    let Outcome::Saved(config, _) = ctrl(&mut settings, 's') else {
        panic!("not saved: {}", settings.message());
    };
    assert!(config.overrides.is_empty());
    let text = read(&path);
    // The table may end up empty; its standalone note stays, the key's own note goes with it.
    assert!(text.contains("[colors]\n# My accent tweaks.\n"), "{text}");
    assert!(
        !text.contains("focus") && !text.contains("# mine"),
        "{text}"
    );
    assert!(saddle::config::Config::parse(&text).is_ok(), "{text}");
}

#[test]
fn settings_offers_lagoon_between_tide_and_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut settings = Settings::open(path, true);
    press(&mut settings, KeyCode::F(2));
    let mut seen = vec![settings.value("theme").unwrap().to_owned()];
    for _ in 0..3 {
        press(&mut settings, KeyCode::Right);
        seen.push(settings.value("theme").unwrap().to_owned());
    }
    assert_eq!(seen, ["dune", "tide", "lagoon", "terminal"]);
    press(&mut settings, KeyCode::Left);
    assert_eq!(settings.value("theme"), Some("lagoon"));
    assert_eq!(settings.value("colors.bg"), Some("#0c1616"));
    assert!(
        settings.message().contains("Lagoon"),
        "{}",
        settings.message()
    );
}
