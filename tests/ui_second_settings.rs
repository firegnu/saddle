//! Second UI batch, Settings inner pages: Diagnostics and Updates share one report layout, the
//! report pages keep their labels and values aligned.
//! Only synthetic reports and temporary directories are used.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, Terminal, backend::TestBackend, buffer::Buffer, style::Modifier};
use saddle::{
    diagnostics::Report,
    settings::Settings,
    theme::Theme,
    updates::{self, Row, Tone},
};
use std::{path::Path, time::SystemTime};

fn render(w: u16, h: u16, draw: impl FnOnce(&mut Frame)) -> (String, Buffer) {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(draw).unwrap();
    let b = terminal.backend().buffer().clone();
    let text = (0..h)
        .map(|y| (0..w).map(|x| b[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    (text, b)
}
fn find(text: &str, needle: &str) -> (u16, u16) {
    text.lines()
        .enumerate()
        .find_map(|(y, line)| {
            line.find(needle)
                .map(|i| (line[..i].chars().count() as u16, y as u16))
        })
        .unwrap_or_else(|| panic!("{needle} not shown:\n{text}"))
}

#[test]
fn diagnostics_and_updates_share_one_label_column_and_keep_every_word() {
    let t = Theme::default();
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::open(dir.path().join("config.toml"), true);
    settings.open_updates();
    let long = "saddle/dev-synthetic-agent-with-a-long-name";
    settings.set_updates(updates::Page {
        rows: vec![
            Row::Heading("Updates need attention".into()),
            Row::Item("Saddle".into(), "Current".into(), Tone::Good),
            Row::Item(
                "Installed Corral".into(),
                "/opt/synthetic/new/bin/corral".into(),
                Tone::Good,
            ),
            Row::Item(
                long.into(),
                "Upgrade available; last failure: the synthetic pen refused the upgrade request because it was busy reading input".into(),
                Tone::Action,
            ),
            Row::Heading("Last upgrade receipt".into()),
            Row::Item(
                "unknown — inspect public status before retrying".into(),
                "no answer from the synthetic corral".into(),
                Tone::Bad,
            ),
        ],
        upgrade: true,
    });
    let (text, b) = render(120, 50, |f| {
        settings.draw(&t, f);
    });
    println!("Updates page:\n{text}");
    let (label_x, saddle_y) = find(&text, "Saddle ");
    let (value_x, current_y) = find(&text, "Current");
    let (path_x, _) = find(&text, "/opt/synthetic/new/bin/corral");
    assert_eq!(saddle_y, current_y);
    assert_eq!(value_x, path_x, "values share one column:\n{text}");
    // A label wider than the column takes its own row; its value starts under the value column.
    let (long_x, long_y) = find(&text, long);
    assert_eq!(long_x, label_x);
    let (upgrade_x, upgrade_y) = find(&text, "Upgrade available");
    assert_eq!((upgrade_x, upgrade_y), (value_x, long_y + 1), "{text}");
    for word in ["busy reading input", "no answer from the synthetic corral"] {
        assert!(text.contains(word), "{word} cut:\n{text}");
    }
    let (receipt_x, receipt_y) = find(&text, "no answer from the synthetic corral");
    assert_eq!(receipt_x, value_x);
    assert_eq!(find(&text, "unknown — inspect").1 + 1, receipt_y);
    // Labels are the quiet column; values keep their state colours.
    assert_eq!(b[(label_x, saddle_y)].fg, t.muted);
    assert_eq!(b[(value_x, current_y)].fg, t.text);
    assert_eq!(b[(upgrade_x, upgrade_y)].fg, t.unread);
    assert_eq!(b[(receipt_x, receipt_y)].fg, t.danger);
    // Headings rely on weight, not the focus colour; a blank row sets the receipt apart.
    for heading in ["Updates need attention", "Last upgrade receipt"] {
        let (x, y) = find(&text, heading);
        assert!(b[(x, y)].modifier.contains(Modifier::BOLD), "{heading}");
        assert_ne!(b[(x, y)].fg, t.focus, "{heading}");
    }
    let (_, heading_y) = find(&text, "Last upgrade receipt");
    let above = text.lines().nth(usize::from(heading_y) - 1).unwrap();
    assert!(!above.chars().any(char::is_alphanumeric), "{text}");
    // The page purposes keep their own main actions.
    let (upgrade_button, button_y) = find(&text, "Upgrade all u");
    assert_eq!(b[(upgrade_button, button_y)].fg, t.focus);
    let (refresh, _) = find(&text, "Refresh r");
    assert_eq!(b[(refresh, button_y)].fg, t.text);

    // Diagnostics: the same label and value columns.
    settings.key(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));
    settings.diagnose(Report {
        corral: "corral".into(),
        agents: Some((SystemTime::now(), Err("synthetic ls timed out".into()))),
        config_path: Path::new("/synthetic/config.toml").into(),
        config_from_file: true,
        layout_path: None,
        restore: None,
        save: None,
        save_off: false,
        checked: SystemTime::now(),
        checks: None,
        versions: None,
    });
    let (text, b) = render(120, 50, |f| {
        settings.draw(&t, f);
    });
    println!("Diagnostics page:\n{text}");
    let (diag_label_x, version_y) = find(&text, "saddle version");
    let (diag_value_x, _) = find(&text, env!("CARGO_PKG_VERSION"));
    assert_eq!(diag_label_x, label_x);
    assert_eq!(diag_value_x, value_x);
    let (failed_x, failed_y) = find(&text, "failed at");
    assert_eq!(failed_x, value_x);
    assert_eq!(b[(failed_x, failed_y)].fg, t.danger);
    assert_eq!(b[(diag_label_x, version_y)].fg, t.muted);
    let (refresh, button_y) = find(&text, "Refresh r");
    assert_eq!(b[(refresh, button_y)].fg, t.focus);
}
