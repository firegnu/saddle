use ratatui::layout::Rect;
use saddle::{config::Config, layout::Panes};

#[test]
fn agents_own_the_left_column_beside_the_viewer() {
    // left_split is still accepted for existing configs but no longer splits the column.
    let config = Config::parse("left_width = 40\nleft_split = 0.6").unwrap();
    let panes = Panes::new(Rect::new(0, 0, 120, 40), &config);
    assert_eq!(panes.agents, Rect::new(0, 0, 40, 39));
    assert_eq!(panes.viewer, Rect::new(40, 0, 80, 39));
    let small = Panes::new(Rect::new(3, 2, 80, 24), &Config::default());
    assert_eq!(small.agents, Rect::new(3, 2, 34, 23));
    assert_eq!(small.viewer, Rect::new(37, 2, 46, 23));
}

#[test]
fn defaults_and_invalid_configuration_are_explicit() {
    let default = Config::parse("").unwrap();
    assert!(default.queue.is_none());
    assert_eq!(default.left_width, 52);
    for invalid in [
        "left_split = 0.0",
        "left_split = 1.0",
        "left_split = nan",
        "left_width = 0",
        "refresh_ms = 0",
        "corral = ''",
    ] {
        assert!(Config::parse(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn legacy_queue_settings_are_inert() {
    assert!(Config::parse("[queue]\ncommand = ['drover', 'board']").is_ok());
}

#[test]
fn panes_and_status_cover_tiny_and_normal_windows() {
    for (w, h) in
        (0..8)
            .flat_map(|w| (0..8).map(move |h| (w, h)))
            .chain([(80, 24), (120, 36), (160, 48)])
    {
        let area = Rect::new(3, 2, w, h);
        let p = Panes::new(area, &Config::default());
        let rects = [p.agents, p.viewer, p.status];
        assert_eq!(rects.iter().map(|r| r.area()).sum::<u32>(), area.area());
        for (i, r) in rects.iter().enumerate().filter(|(_, r)| !r.is_empty()) {
            assert_eq!(r.intersection(area), *r);
            for other in &rects[i + 1..] {
                assert!(r.intersection(*other).is_empty());
            }
        }
    }
}

#[test]
fn startup_selects_absolute_xdg_then_home_with_explicit_path_taking_priority() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let xdg = dir.path().join("xdg");
    let fallback = home.join(".config/saddle/config.toml");
    let xdg_file = xdg.join("saddle/config.toml");
    let explicit = dir.path().join("explicit.toml");
    for path in [&fallback, &xdg_file, &explicit] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Fail before opening a terminal or invoking any external CLI.
        std::fs::write(path, "refresh_ms = 0").unwrap();
    }
    for (value, expected, override_path) in [
        (Some(xdg.as_os_str()), &xdg_file, false),
        (None, &fallback, false),
        (Some(std::ffi::OsStr::new("")), &fallback, false),
        (Some(std::ffi::OsStr::new("relative")), &fallback, false),
        (Some(xdg.as_os_str()), &explicit, true),
    ] {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_saddle"));
        cmd.env("HOME", &home)
            .env("XDG_STATE_HOME", dir.path().join("state"))
            .env_remove("XDG_CONFIG_HOME");
        if let Some(value) = value {
            cmd.env("XDG_CONFIG_HOME", value);
        }
        if override_path {
            cmd.arg("--config").arg(&explicit);
        }
        let output = cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(
            stderr.contains(&format!("invalid config {}", expected.display())),
            "{stderr}"
        );
        assert!(stderr.contains("refresh_ms must be positive"), "{stderr}");
    }
}

#[test]
fn default_example_missing_files_and_partial_colors_keep_current_defaults() {
    use ratatui::style::Color;
    let example = Config::parse(include_str!("../config.toml")).unwrap();
    let defaults = Config::default();
    assert_eq!(example.corral, defaults.corral);
    assert_eq!(example.left_width, defaults.left_width);
    assert_eq!(example.left_split, defaults.left_split);
    assert_eq!(example.refresh_ms, defaults.refresh_ms);
    assert!(example.queue.is_none());
    assert!(
        Config::parse("[queue]\ndrover='unused'\ncwd='/unused'")
            .unwrap()
            .queue
            .is_some()
    );
    assert_eq!(example.colors, defaults.colors);
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        Config::load(&dir.path().join("missing.toml"))
            .unwrap()
            .colors,
        defaults.colors
    );
    let partial = Config::parse("[colors]\nfocus = '#12aBcD'\ntext = 'default'").unwrap();
    assert_eq!(partial.colors.focus, Color::Rgb(0x12, 0xab, 0xcd));
    assert_eq!(partial.colors.text, Color::Reset);
    assert_eq!(partial.colors.agent_selected, Color::Rgb(0x2b, 0x26, 0x21));
    for (name, color) in [
        ("reset", Color::Reset),
        ("black", Color::Black),
        ("red", Color::Red),
        ("green", Color::Green),
        ("yellow", Color::Yellow),
        ("blue", Color::Blue),
        ("magenta", Color::Magenta),
        ("cyan", Color::Cyan),
        ("gray", Color::Gray),
        ("dark_gray", Color::DarkGray),
        ("light_red", Color::LightRed),
        ("light_green", Color::LightGreen),
        ("light_yellow", Color::LightYellow),
        ("light_blue", Color::LightBlue),
        ("light_magenta", Color::LightMagenta),
        ("light_cyan", Color::LightCyan),
        ("white", Color::White),
    ] {
        let c = Config::parse(&format!("[colors]\nfocus = '{name}'")).unwrap();
        assert_eq!(c.colors.focus, color, "{name}");
    }
}

#[test]
fn invalid_colors_report_the_config_path_and_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.toml");
    for invalid in [
        "'not-a-color'",
        "'#fff'",
        "'#12345g'",
        "'#1234567'",
        "'#１２'",
        "123",
        "[]",
    ] {
        std::fs::write(&path, format!("[colors]\nfocus = {invalid}")).unwrap();
        let error = format!("{:#}", Config::load(&path).unwrap_err());
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert!(error.contains("focus"), "{error}");
    }
    assert!(Config::parse("[colors]\nfocsu = 'red'").is_err());
}

#[test]
fn agents_palette_takes_nearest_256_colors_unless_the_terminal_announces_truecolor() {
    use ratatui::style::Color;
    use saddle::theme::{Theme, nearest_256, truecolor};
    assert!(truecolor(Some("truecolor")) && truecolor(Some("24bit")));
    assert!(!truecolor(None) && !truecolor(Some("")) && !truecolor(Some("256")));
    // xterm cube and gray ramp: warm near-black is gray 234, olive and red are cube entries.
    assert_eq!(
        nearest_256(Color::Rgb(0x1d, 0x1a, 0x16)),
        Color::Indexed(234)
    );
    assert_eq!(
        nearest_256(Color::Rgb(0xbd, 0xb8, 0x6a)),
        Color::Indexed(143)
    );
    assert_eq!(
        nearest_256(Color::Rgb(0xe3, 0x72, 0x64)),
        Color::Indexed(167)
    );
    assert_eq!(
        nearest_256(Color::Rgb(0xe8, 0xdf, 0xcc)),
        Color::Indexed(253)
    );
    assert_eq!(nearest_256(Color::Yellow), Color::Yellow);
    let theme = Theme::default();
    assert_eq!(theme.clone().for_terminal(true), theme);
    let reduced = theme.clone().for_terminal(false);
    for color in [
        reduced.agents_bg,
        reduced.agents_border,
        reduced.agents_rule,
        reduced.agents_faint,
        reduced.agents_text,
        reduced.agents_branch,
        reduced.agents_dim,
        reduced.agents_dimmer,
        reduced.agents_accent,
        reduced.agents_green,
        reduced.agents_red,
        reduced.agents_blue,
        reduced.agents_yellow,
        reduced.agents_purple,
        reduced.agent_selected,
        reduced.claude,
        reduced.codex,
    ] {
        assert!(matches!(color, Color::Indexed(_)), "{color:?}");
    }
    assert_eq!(reduced.agents_bg, Color::Indexed(234));
    // Colors shared with Tasks and the rest of the interface keep their configured values.
    assert_eq!(reduced.agent_working, theme.agent_working);
    assert_eq!(reduced.agent_idle, theme.agent_idle);
    assert_eq!(reduced.focus, theme.focus);
    assert_eq!(reduced.bg, theme.bg);
    // A configured ANSI name stays as the user wrote it.
    let ansi = Config::parse("[colors]\nagents_text = 'white'")
        .unwrap()
        .colors
        .for_terminal(false);
    assert_eq!(ansi.agents_text, Color::White);
}

#[test]
fn mascot_option_defaults_on_and_accepts_only_toml_booleans() {
    assert!(Config::default().mascot_enabled);
    assert!(Config::parse("").unwrap().mascot_enabled);
    assert!(
        Config::parse(include_str!("../config.toml"))
            .unwrap()
            .mascot_enabled
    );
    assert!(
        Config::parse("mascot_enabled = true")
            .unwrap()
            .mascot_enabled
    );
    assert!(
        !Config::parse("mascot_enabled = false")
            .unwrap()
            .mascot_enabled
    );
    for invalid in ["mascot_enabled = 'false'", "mascot_enabled = 0"] {
        assert!(Config::parse(invalid).is_err());
    }
}

#[test]
fn mascot_choice_defaults_to_clawd_and_accepts_only_known_pets() {
    use saddle::mascot::Pet;
    assert_eq!(Config::default().mascot, Pet::Clawd);
    assert_eq!(Config::parse("").unwrap().mascot, Pet::Clawd);
    assert_eq!(
        Config::parse(include_str!("../config.toml"))
            .unwrap()
            .mascot,
        Pet::Clawd
    );
    assert_eq!(Config::parse("mascot = 'cat'").unwrap().mascot, Pet::Cat);
    for invalid in ["mascot = 'dog'", "mascot = true"] {
        assert!(Config::parse(invalid).is_err());
    }
}

#[test]
fn mascot_display_defaults_to_auto_and_accepts_only_auto_or_blocks() {
    use saddle::mascot::Display;
    assert_eq!(Config::default().mascot_display, Display::Auto);
    assert_eq!(Config::parse("").unwrap().mascot_display, Display::Auto);
    assert_eq!(
        Config::parse(include_str!("../config.toml"))
            .unwrap()
            .mascot_display,
        Display::Auto
    );
    assert_eq!(
        Config::parse("mascot_display = 'blocks'")
            .unwrap()
            .mascot_display,
        Display::Blocks
    );
    for invalid in ["mascot_display = 'images'", "mascot_display = true"] {
        assert!(Config::parse(invalid).is_err());
    }
}

#[test]
fn a_theme_supplies_every_color_and_colors_overrides_only_the_keys_written() {
    use ratatui::style::Color;
    use saddle::theme::{Preset, Theme};
    // No theme key: Dune, the original look, with nothing marked as overridden.
    let old = Config::parse("left_width = 52").unwrap();
    assert_eq!(old.theme, Preset::Dune);
    assert_eq!(old.colors, Theme::default());
    assert!(old.overrides.is_empty());
    assert_eq!(Preset::Dune.theme(), Theme::default());
    // A value equal to the theme's is still an override.
    let same = Config::parse("[colors]\nfocus = 'yellow'").unwrap();
    assert_eq!(same.colors, Theme::default());
    assert_eq!(same.overrides.get("focus"), Some(&Color::Yellow));
    // Tide is a whole palette: shared areas and the Agents column both change.
    let tide = Preset::Tide.theme();
    let dune = Theme::default();
    for (name, differs) in [
        ("focus", tide.focus != dune.focus),
        ("border", tide.border != dune.border),
        ("muted", tide.muted != dune.muted),
        ("agents_bg", tide.agents_bg != dune.agents_bg),
        ("agents_text", tide.agents_text != dune.agents_text),
        ("agent_selected", tide.agent_selected != dune.agent_selected),
    ] {
        assert!(differs, "Tide keeps Dune's {name}");
    }
    let chosen = Config::parse("theme = 'tide'\n[colors]\nfocus = 'red'").unwrap();
    assert_eq!(chosen.theme, Preset::Tide);
    assert_eq!(chosen.colors.focus, Color::Red);
    assert_eq!(chosen.colors.agents_bg, tide.agents_bg);
    assert_eq!(
        chosen.overrides.keys().copied().collect::<Vec<_>>(),
        ["focus"]
    );
    // Terminal uses only the terminal's default and ANSI palette.
    let mut terminal = Preset::Terminal.theme();
    for (name, color) in terminal.named_mut() {
        assert!(
            !matches!(color, Color::Rgb(..) | Color::Indexed(_)),
            "{name} = {color:?}"
        );
    }
    // The user's upgraded file reads as Dune plus exactly what it says.
    let mine = Config::parse(
        "[colors]\nagents_bg = 'default'\nagent_selected = '#302a23'\nclaude = '#d97757'\ncodex = '#8ed9c1'\nfocus = 'yellow'",
    )
    .unwrap();
    assert_eq!(mine.theme, Preset::Dune);
    assert_eq!(mine.colors.agents_bg, Color::Reset);
    assert_eq!(mine.colors.claude, Color::Rgb(0xd9, 0x77, 0x57));
    assert_eq!(mine.colors.agents_text, dune.agents_text);
    assert_eq!(mine.overrides.len(), 5);
    let error = format!("{:#}", Config::parse("theme = 'ocean'").unwrap_err());
    assert!(
        error.contains("theme") && error.contains("ocean"),
        "{error}"
    );
    assert!(Config::parse("theme = 'Dune'").is_err());
}

#[test]
fn lagoon_is_a_whole_palette_on_the_reference_green() {
    use ratatui::style::Color;
    use saddle::theme::Preset;
    assert_eq!(
        Preset::ALL.map(Preset::label),
        ["Dune", "Tide", "Lagoon", "Terminal"]
    );
    let chosen = Config::parse("theme = 'lagoon'\n[colors]\nfocus = 'red'").unwrap();
    assert_eq!(chosen.theme, Preset::Lagoon);
    let lagoon = Preset::Lagoon.theme();
    let green = Color::Rgb(0x0c, 0x16, 0x16);
    assert_eq!((lagoon.bg, lagoon.agents_bg), (green, green));
    assert_eq!(chosen.colors.focus, Color::Red);
    assert_eq!(chosen.colors.bg, green);
    assert_eq!(
        chosen.overrides.keys().copied().collect::<Vec<_>>(),
        ["focus"]
    );
    let mut lagoon = lagoon;
    for (name, color) in lagoon.named_mut() {
        if !matches!(name, "overlay") {
            assert!(matches!(color, Color::Rgb(..)), "{name} = {color:?}");
        }
    }
}
