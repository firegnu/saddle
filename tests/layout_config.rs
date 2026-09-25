use ratatui::layout::Rect;
use saddle::{config::Config, layout::Panes};

#[test]
fn responsive_management_panes_reserve_a_status_row() {
    let config = Config::parse("left_width = 40\nleft_split = 0.6").unwrap();
    let panes = Panes::new(Rect::new(0, 0, 120, 40), &config);
    assert_eq!(panes.agents, Rect::new(0, 0, 40, 23));
    assert_eq!(panes.queue, Rect::new(0, 23, 40, 16));
    assert_eq!(panes.viewer, Rect::new(40, 0, 80, 39));
    let small = Panes::new(Rect::new(3, 2, 80, 24), &Config::default());
    assert_eq!(small.agents, Rect::new(3, 3, 34, 22));
    assert!(small.queue.is_empty());
    assert_eq!(small.viewer, Rect::new(37, 2, 46, 23));
}

#[test]
fn defaults_and_invalid_configuration_are_explicit() {
    let default = Config::parse("").unwrap();
    assert_eq!(default.queue.drover, "drover");
    assert_eq!(default.left_width, 52);
    for invalid in [
        "left_split = 0.0",
        "left_split = 1.0",
        "left_split = nan",
        "left_width = 0",
        "refresh_ms = 0",
        "corral = ''",
        "[queue]\ndrover = ''",
    ] {
        assert!(Config::parse(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn external_queue_ui_commands_are_rejected() {
    assert!(Config::parse("[queue]\ncommand = ['drover', 'board']").is_err());
}

#[test]
fn tabs_and_status_cover_tiny_and_normal_windows_without_overlap() {
    for (w, h) in
        (0..8)
            .flat_map(|w| (0..8).map(move |h| (w, h)))
            .chain([(80, 24), (120, 36), (160, 48)])
    {
        let area = Rect::new(3, 2, w, h);
        for queue in [false, true] {
            let p = Panes::with_queue(area, &Config::default(), queue);
            let rects = [p.agents, p.queue, p.viewer, p.status, p.tabs];
            assert_eq!(rects.iter().map(|r| r.area()).sum::<u32>(), area.area());
            for (i, r) in rects.iter().enumerate().filter(|(_, r)| !r.is_empty()) {
                assert_eq!(r.intersection(area), *r);
                for other in &rects[i + 1..] {
                    assert!(r.intersection(*other).is_empty());
                }
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
        cmd.env("HOME", &home).env_remove("XDG_CONFIG_HOME");
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
    assert_eq!(example.queue.drover, defaults.queue.drover);
    assert_eq!(example.queue.cwd, None);
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
    assert_eq!(partial.colors.agent_selected, Color::Rgb(0x30, 0x2a, 0x23));
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
