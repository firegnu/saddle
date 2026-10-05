//! Synthetic draws of the narrow Agents column. Each check
//! prints the screens it looked at.
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use saddle::{
    agents::Panel,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    input::Focus,
    layout::Panes,
    terminals::Terminals,
    theme::Theme,
    ui::{self, View},
};
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
                }),
            );
        })
        .unwrap();
    let lines = rows(terminal.backend().buffer(), panes.agents.width);
    println!(
        "── Agents column, {width}-column window ──\n{}",
        lines.join("\n")
    );
    (lines, hits)
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
