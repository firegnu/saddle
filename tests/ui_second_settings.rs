//! Second UI batch, Settings inner pages: Diagnostics and Updates share one report layout, the
//! Plugins management page, Add local and the launcher keep focus, main actions and errors apart.
//! Only synthetic reports, plugins and temporary directories are used.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, Terminal, backend::TestBackend, buffer::Buffer, style::Modifier};
use saddle::{
    diagnostics::Report,
    plugins::{
        Manager,
        palette::{Item, Palette},
        resources::Resources,
        ui::Page,
    },
    settings::Settings,
    theme::Theme,
    updates::{self, Row, Tone},
};
use saddle_core_plugin::{Call, Completion, CorePlugin, Manifest};
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
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
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

struct Fake;
static FAKE: Manifest = Manifest {
    id: "test.builtin",
    name: "Synthetic built-in",
    version: "1",
    commands: &[],
    resources: &[],
    setup_note: "",
    setup_files: &[],
};
impl CorePlugin for Fake {
    fn manifest(&self) -> &'static Manifest {
        &FAKE
    }
    fn run(&self, _: Call<'_>) -> Completion {
        Completion {
            exit_code: 0,
            stdout: vec![],
            end: None,
        }
    }
}
static PLUGIN: Fake = Fake;
static CATALOG: saddle::plugins::core::Catalog = &[&PLUGIN];
fn manager(dir: &Path, catalog: saddle::plugins::core::Catalog) -> Manager {
    Manager::with_resources(
        dir.join("config/saddle/plugins.toml"),
        catalog,
        Resources::new(dir.join("home"), dir.join("state/saddle")),
    )
}

#[test]
fn plugin_management_keeps_focus_main_action_and_errors_apart() {
    let t = Theme::default();
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings::open(dir.path().join("config.toml"), true);
    let mut m = manager(dir.path(), CATALOG);
    let mut page = Page::default();
    let (text, b) = render(110, 40, |f| page.draw(&t, f, &m, &settings));
    println!("Plugins management:\n{text}");
    // Action labels without a key show no part of themselves as one, and no hit number shows.
    let (add, y) = find(&text, "Add local…");
    assert_eq!(
        b[(add, y)].fg,
        b[(add + 4, y)].fg,
        "local… looks like a key"
    );
    let (sync, _) = find(&text, "Sync resources");
    assert_eq!(b[(sync, y)].fg, b[(sync + 5, y)].fg);
    let actions = text.lines().nth(usize::from(y)).unwrap();
    for n in 1..=7 {
        assert!(!actions.contains(&format!("F{n}")), "{actions}");
    }
    // The selected row is marked by its background, weight and marker; the marker shows focus.
    let (marker, row) = find(&text, "› Synthetic built-");
    let name = marker + 2;
    assert_eq!(b[(name, row)].bg, t.agent_selected);
    assert_eq!(b[(name, row)].fg, t.text);
    assert!(b[(name, row)].modifier.contains(Modifier::BOLD));
    assert_eq!(b[(marker, row)].fg, t.focus);
    // The details title, above the first row, is a heading rather than a focus cue.
    let (title, title_y) = find(&text, " Synthetic built-in ");
    assert!(title_y < row);
    assert_ne!(b[(title + 1, title_y)].fg, t.focus);

    // Tab focus: underlined, distinct from a primary action; the list keeps its selection.
    page.event(key(KeyCode::Tab), &mut m);
    let (text, b) = render(110, 40, |f| page.draw(&t, f, &m, &settings));
    let (enable, y) = find(&text, "Enable");
    assert!(
        b[(enable, y)].modifier.contains(Modifier::UNDERLINED),
        "{text}"
    );
    assert_eq!(b[(enable, y)].fg, t.focus);
    assert!(!b[(add, y)].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(b[(name, row)].bg, t.agent_selected);
    assert_ne!(
        b[(marker, row)].fg,
        t.focus,
        "list marker still shows focus"
    );

    // An immediate result is a quiet notice; Enter keeps acting on the focused control.
    page.event(key(KeyCode::Enter), &mut m);
    page.event(key(KeyCode::Enter), &mut m);
    let (text, b) = render(110, 40, |f| page.draw(&t, f, &m, &settings));
    let (x, y) = find(&text, "Disabled. Installed resources are kept");
    assert_eq!(b[(x, y)].fg, t.muted, "{text}");

    // A registry that cannot be read is an error.
    let broken = tempfile::tempdir().unwrap();
    let registry = broken.path().join("config/saddle/plugins.toml");
    std::fs::create_dir_all(registry.parent().unwrap()).unwrap();
    std::fs::write(&registry, "not [ toml").unwrap();
    let m = manager(broken.path(), &[]);
    let error = m.registry.error.clone().expect("synthetic registry error");
    let mut page = Page::default();
    let (text, b) = render(110, 40, |f| page.draw(&t, f, &m, &settings));
    let shown: String = error.chars().take(20).collect();
    let (x, y) = find(&text, &shown);
    assert_eq!(b[(x, y)].fg, t.danger, "{text}");
}

#[test]
fn add_local_shows_its_main_step_and_errors() {
    let t = Theme::default();
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings::open(dir.path().join("config.toml"), true);
    let mut m = manager(dir.path(), &[]);
    let mut page = Page::default();
    // No plugin rows: Tab past the list and the disabled Open/Enable/Restart to Add local….
    for _ in 0..4 {
        page.event(key(KeyCode::Tab), &mut m);
    }
    page.event(key(KeyCode::Enter), &mut m);
    let (text, b) = render(100, 30, |f| page.draw(&t, f, &m, &settings));
    println!("Add local:\n{text}");
    assert!(text.contains("Add local plugin"), "{text}");
    // The field is named once; its placeholder is an example rather than the same name.
    assert_eq!(text.matches("Plugin directory").count(), 1, "{text}");
    // Reading the manifest is the step to take; adding stays unavailable until it succeeds.
    let (read, y) = find(&text, "‹Read manifest");
    let read = read + 1;
    assert_eq!(b[(read, y)].fg, t.focus);
    assert_eq!(b[(read + 5, y)].fg, t.focus, "manifest looks like a key");
    let (add, _) = find(&text, "Add disabled");
    assert_eq!(b[(add, y)].fg, t.dim);
    let (cancel, _) = find(&text, "Cancel");
    assert_eq!(b[(cancel, y)].fg, t.text);
    let (hint_x, hint_y) = find(&text, "Enter a directory");
    assert_eq!(b[(hint_x, hint_y)].fg, t.muted);
    // Keyboard focus on the main step is underlined as well as primary.
    page.event(key(KeyCode::Tab), &mut m);
    let (text, b) = render(100, 30, |f| page.draw(&t, f, &m, &settings));
    let (read, y) = find(&text, "‹Read manifest");
    let read = read + 1;
    assert!(b[(read, y)].modifier.contains(Modifier::UNDERLINED));
    page.event(key(KeyCode::Enter), &mut m);
    let (text, b) = render(100, 30, |f| page.draw(&t, f, &m, &settings));
    println!("Add local failure:\n{text}");
    let message = page.message.clone();
    assert!(!message.is_empty());
    let shown: String = message.chars().take(16).collect();
    let (x, y) = find(&text, &shown);
    assert_eq!(b[(x, y)].fg, t.danger, "{text}");
}

fn item(id: &str, state: &str, note: &str, builtin: bool) -> Item {
    Item {
        id: id.into(),
        title: id.into(),
        state: state.into(),
        note: note.into(),
        has_view: !builtin,
        opened: false,
        pid: None,
        builtin,
    }
}

#[test]
fn the_launcher_keeps_opening_managing_and_failures_apart() {
    let t = Theme::default();
    let mut p = Palette::default();
    p.update(vec![
        item("Viewer", "Running", "", false),
        item("Settings helper", "Enabled", "", true),
        item("Broken", "Failed", "synthetic crash at start", false),
        item("Resting", "Disabled", "", false),
    ]);
    let draw = |p: &mut Palette| render(80, 24, |f| p.draw(f, &t));
    let (text, b) = draw(&mut p);
    println!("Launcher:\n{text}");
    let (open, y) = find(&text, "‹Open›");
    assert_eq!(b[(open, y)].fg, t.focus);
    let (manage, y) = find(&text, "‹Manage›");
    assert_ne!(b[(manage, y)].fg, t.focus, "managing is not opening");
    assert_ne!(b[(manage, y)].fg, t.dim);
    assert!(text.contains("Esc Close"), "{text}");
    for _ in 0..2 {
        p.event(&key(KeyCode::Down));
    }
    let (text, b) = draw(&mut p);
    let (x, y) = find(&text, "synthetic crash at start");
    assert_eq!(b[(x, y)].fg, t.danger, "{text}");
    p.event(&key(KeyCode::Down));
    let (text, b) = draw(&mut p);
    let (x, y) = find(&text, "Enable this plugin");
    assert_eq!(b[(x, y)].fg, t.muted, "{text}");
    // Footer focus is underlined, not only recoloured.
    p.event(&key(KeyCode::Tab));
    let (text, b) = draw(&mut p);
    let (x, y) = find(&text, "‹Manage plugins›");
    assert!(
        b[(x + 1, y)].modifier.contains(Modifier::UNDERLINED),
        "{text}"
    );
    let (x, y) = find(&text, "‹Close›");
    assert!(!b[(x + 1, y)].modifier.contains(Modifier::UNDERLINED));
}
