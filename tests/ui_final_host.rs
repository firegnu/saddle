//! Synthetic draws of the narrow Agents column and the plugin manager buttons. Each check
//! prints the screens it looked at.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Modifier};
use saddle::{
    agents::Panel,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    input::Focus,
    layout::Panes,
    plugins::{Manager, resources::Resources, ui::Page},
    settings::Settings,
    terminals::Terminals,
    theme::Theme,
    ui::{self, View},
};
use saddle_core_plugin::{Call, Completion, CorePlugin, Manifest};
use unicode_width::UnicodeWidthStr;

fn rows(buffer: &Buffer, width: u16) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += symbol.width().max(1) as u16;
            }
            line
        })
        .collect()
}
fn agent(name: &str, kind: &str, state: &str, effort: Option<&str>) -> Agent {
    let mut labels = serde_json::Map::new();
    if let Some(effort) = effort {
        labels.insert("effort".into(), effort.into());
    }
    Agent {
        name: name.into(),
        kind: Some(kind.into()),
        state: Some(state.into()),
        cwd: Some("/tmp/saddle".into()),
        instance: Some("abcdef123".into()),
        state_started: Some(40.0),
        turn_started: Some(40.0),
        labels,
        ..Default::default()
    }
}
/// The Agents column of a `width`-column workspace, one line per row.
fn agents_column(width: u16, agents: Vec<Agent>, local: &[String]) -> (Vec<String>, ui::Hits) {
    agents_column_pinned(width, agents, local, None).0
}
/// The same column with a pinned header entry; also returns the drawn buffer for styles.
fn agents_column_pinned(
    width: u16,
    agents: Vec<Agent>,
    local: &[String],
    pinned: Option<ui::Pinned<'_>>,
) -> ((Vec<String>, ui::Hits), Buffer) {
    let mut panel = Panel {
        follow: true,
        ..Default::default()
    };
    panel.absorb(agents, None, 100.0);
    panel.toggle_fold();
    let terminals = Terminals::new("unused-fake-corral".into());
    let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
    let mut hits = ui::Hits::default();
    let panes = Panes::new(terminal.get_frame().area(), &Config::default());
    terminal
        .draw(|frame| {
            hits = ui::draw_workspace(
                frame,
                &mut panel,
                View {
                    colors: &Theme::default(),
                    panes,
                    focus: Focus::Agents,
                    showing: None,
                    local,
                    viewer: None,
                    projects: &[],
                    viewer_note: "",
                    reply: "",
                    now: 100.0,
                    pointer: &Pointer::default(),
                },
                Some(ui::Workspace {
                    terminals: &terminals,
                    mascot: &mut Default::default(),
                    mascot_enabled: false,
                    placement: None,
                    search: None,
                    form: None,
                    program: "unused-fake-corral",
                    modal: false,
                    attention: ui::Attention {
                        items: &[],
                        loading: false,
                        popup: None,
                    },
                    settings: None,
                    updates: false,
                    pinned,
                }),
            );
        })
        .unwrap();
    let lines = rows(terminal.backend().buffer(), panes.agents.width);
    println!(
        "── Agents column, {width}-column window ──\n{}",
        lines.join("\n")
    );
    ((lines, hits), terminal.backend().buffer().clone())
}

#[test]
fn narrow_agents_rows_keep_the_name_before_the_time() {
    let sample = || {
        vec![
            agent("saddle/dev-ui2-host", "codex", "working", Some("high")),
            agent("saddle/main", "claude", "idle", None),
            agent("saddle/test-a", "codex", "exited", None),
        ]
    };
    let local = ["saddle/main".to_owned()];
    // Wide columns keep 3a: name, type, effort, state label and time on one row.
    let (lines, _) = agents_column(120, sample(), &local);
    let row = |lines: &[String], name: &str| {
        lines
            .iter()
            .find(|l| l.contains(name))
            .unwrap_or_else(|| panic!("{name} missing:\n{}", lines.join("\n")))
            .clone()
    };
    let main = row(&lines, "main ");
    assert!(
        main.contains("○") && main.contains("idle") && main.contains("⦿ 1m"),
        "{main}"
    );
    let host = row(&lines, "dev-ui2-host");
    assert!(host.contains("working") && host.contains(" 1m"), "{host}");
    // Before the change a 26-column Agents column (52-column window) drew no name at all,
    // only `◓  >_ working      1`. Now the name comes first; time, then the state label give
    // way, and the status dot, type and connection mark stay.
    for width in [52, 60] {
        let (lines, hits) = agents_column(width, sample(), &local);
        let host = row(&lines, "dev-ui2");
        assert!(host.contains("◓") && host.contains(">_"), "{host}");
        assert!(!host.contains("1m ") && !host.ends_with(" 1 ┃"), "{host}");
        let main = row(&lines, "main");
        assert!(
            main.contains("○") && main.contains("✳") && main.contains("⦿"),
            "{main}"
        );
        assert!(row(&lines, "test-a").contains("✕"));
        // Rows still select the same agents.
        for name in ["saddle/dev-ui2-host", "saddle/main", "saddle/test-a"] {
            assert!(hits.agents.iter().any(|(_, n)| n == name), "{name}");
        }
    }
    // Without the effort column there is room for a short name and the state label.
    let mut plain = sample();
    plain[0].labels.clear();
    let (lines, _) = agents_column(60, plain, &local);
    assert!(row(&lines, "main").contains("idle"), "{}", lines.join("\n"));
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
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
/// The page drawn `width` columns wide, and the text of its underlined cells.
fn underlined(page: &mut Page, m: &Manager, settings: &Settings, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
    terminal
        .draw(|f| page.draw(&Theme::default(), f, m, settings))
        .unwrap();
    let buffer = terminal.backend().buffer();
    println!("{}", rows(buffer, width).join("\n"));
    let area = buffer.area;
    (area.top()..area.bottom())
        .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
        .filter(|&p| buffer[p].modifier.contains(Modifier::UNDERLINED))
        .map(|p| buffer[p].symbol().to_owned())
        .collect()
}

#[test]
fn cut_plugin_buttons_still_show_keyboard_focus() {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings::open(dir.path().join("config.toml"), true);
    let mut m = Manager::with_resources(
        dir.path().join("config/saddle/plugins.toml"),
        CATALOG,
        Resources::new(dir.path().join("home"), dir.path().join("state/saddle")),
    );
    let mut page = Page::default();
    // Management page, 20 columns: the two resource buttons are wider than their row.
    let labels = [
        "Enable",
        "Sync resources",
        "Remove resources",
        "Add local…",
        "Refresh",
        "Back",
    ];
    for label in labels {
        page.event(key(KeyCode::Tab), &mut m);
        let shown = underlined(&mut page, &m, &settings, 20);
        println!("focus {label}: underlined {shown:?}");
        assert!(
            shown.len() > 3 && label.starts_with(&shown),
            "{label}: {shown:?}"
        );
    }
    // Add local, 17 columns: Read manifest and Add disabled are cut. From Back, Tab wraps to
    // the list and on to Add local….
    for _ in 0..5 {
        page.event(key(KeyCode::Tab), &mut m);
    }
    page.event(key(KeyCode::Enter), &mut m);
    for label in ["Read manifest", "Add disabled", "Cancel"] {
        page.event(key(KeyCode::Tab), &mut m);
        let shown = underlined(&mut page, &m, &settings, 17);
        println!("focus {label}: underlined {shown:?}");
        assert!(
            shown.len() > 3 && label.starts_with(&shown),
            "{label}: {shown:?}"
        );
    }
}

#[test]
fn a_pinned_entry_joins_more_wraps_with_it_and_gives_way_first() {
    let sample = || vec![agent("saddle/main", "claude", "idle", None)];
    let pin = |title, available| Some(ui::Pinned { title, available });
    let fixed = |lines: &[String]| {
        let all = lines.join("\n");
        assert!(all.contains('⋯'), "More missing:\n{all}");
    };
    // Wide: the pin shares the title row, left of More, and its hit area is that text.
    let ((lines, hits), _) = agents_column_pinned(120, sample(), &[], pin("Tasks", true));
    let row = usize::from(hits.pinned.y);
    assert!(lines[row].contains("Agents · 1") && lines[row].contains("Tasks   ⋯"));
    assert_eq!(hits.pinned.width, 5);
    fixed(&lines);
    // A long title takes at most 12 columns.
    let ((lines, hits), _) =
        agents_column_pinned(120, sample(), &[], pin("Extraordinarily long", true));
    assert!(lines.join("\n").contains("Extraordina…   ⋯"));
    assert_eq!(hits.pinned.width, 12);
    // Narrow: the group wraps below the title together.
    let ((lines, hits), _) = agents_column_pinned(40, sample(), &[], pin("Tasks", true));
    let row = usize::from(hits.pinned.y);
    assert!(lines[row].contains("Tasks   ⋯") && !lines[row].contains("Agents ·"));
    fixed(&lines);
    // When even the wrapped row cannot hold both, the pin is hidden and nothing else moves.
    let ((plain, _), _) = agents_column_pinned(40, sample(), &[], None);
    let ((lines, hits), _) =
        agents_column_pinned(40, sample(), &[], pin("Extraordinarily long", true));
    assert!(hits.pinned.is_empty());
    assert_eq!(lines, plain);
    fixed(&lines);
    // An entry that cannot open now stays in place, dimmed.
    let ((_, hits), buffer) = agents_column_pinned(120, sample(), &[], pin("Tasks", false));
    assert_eq!(
        buffer[(hits.pinned.x, hits.pinned.y)].fg,
        Theme::default().muted
    );
    let ((_, hits), buffer) = agents_column_pinned(120, sample(), &[], pin("Tasks", true));
    assert_eq!(
        buffer[(hits.pinned.x, hits.pinned.y)].fg,
        Theme::default().agents_text
    );
}
