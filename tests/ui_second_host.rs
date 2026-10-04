//! Synthetic draws of the host status bar, notices, toasts, Agents empty states, stop/close
//! confirmations and terminal pane surroundings. Each check prints the screens it looked at.
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
};
use saddle::{
    agents::Panel,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    input::Focus,
    layout::Panes,
    search::Search,
    terminals::{self, Control, Place, Terminals},
    theme::Theme,
    ui::{self, View},
};
use unicode_width::UnicodeWidthStr;

fn rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += symbol.width().max(1) as u16;
            }
            line
        })
        .collect()
}
fn show(label: &str, buffer: &Buffer) -> Vec<String> {
    let lines = rows(buffer);
    println!("── {label} ──\n{}", lines.join("\n"));
    lines
}
fn find(buffer: &Buffer, label: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        if let Some(i) = line.find(label) {
            return (line[..i].chars().count() as u16, y);
        }
    }
    panic!("{label} not drawn:\n{}", rows(buffer).join("\n"));
}
fn panel(agents: Vec<Agent>) -> Panel {
    let mut panel = Panel {
        follow: true,
        ..Default::default()
    };
    panel.absorb(agents, None, 100.0);
    panel
}
fn agent(name: &str, cwd: &str) -> Agent {
    Agent {
        name: name.into(),
        kind: Some("codex".into()),
        state: Some("idle".into()),
        cwd: Some(cwd.into()),
        instance: Some("abcdef123".into()),
        state_started: Some(90.0),
        ..Default::default()
    }
}
fn workspace(
    size: (u16, u16),
    focus: Focus,
    terminals: &Terminals,
    search: Option<&mut Search>,
    panel: &mut Panel,
) -> (Buffer, ui::Hits) {
    let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
    let mut hits = ui::Hits::default();
    terminal
        .draw(|frame| {
            hits = ui::draw_workspace(
                frame,
                panel,
                View {
                    colors: &Theme::default(),
                    panes: Panes::new(frame.area(), &Config::default()),
                    focus,
                    showing: None,
                    local: &[],
                    viewer: None,
                    projects: &[],
                    viewer_note: "",
                    reply: "",
                    now: 100.0,
                    pointer: &Pointer::default(),
                },
                Some(ui::Workspace {
                    terminals,
                    mascot: &mut Default::default(),
                    mascot_enabled: false,
                    placement: None,
                    search,
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
    (terminal.backend().buffer().clone(), hits)
}
fn draw(size: (u16, u16), paint: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
    terminal.draw(paint).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn workspace_and_overlay_status_bars_share_one_target_and_help_shape() {
    let t = Theme::default();
    let terminals = Terminals::new("unused-fake-corral".into());
    let mut p = panel(vec![agent("p/a", "/tmp/p")]);
    let (buffer, _) = workspace((100, 12), Focus::Agents, &terminals, None, &mut p);
    let normal = show("workspace", &buffer);
    // Overlays: Telemetry's own status line, Plugins, Plugin settings and a plugin view.
    type DrawBar<'a> = Box<dyn Fn(&mut ratatui::Frame, Rect) + 'a>;
    let bars: [(&str, DrawBar<'_>); 4] = [
        (
            "Telemetry",
            Box::new(|f, a| ui::status_text(&t, f, a, " Input ▸ Telemetry · Esc Close")),
        ),
        (
            "Plugins",
            Box::new(|f, a| ui::status_bar(&t, f, a, "Plugins", "Esc Close")),
        ),
        (
            "Plugin settings",
            Box::new(|f, a| ui::status_bar(&t, f, a, "Plugin settings", "Esc Back")),
        ),
        (
            "Drover",
            Box::new(|f, a| ui::status_bar(&t, f, a, "Drover", "Esc Close  Ctrl-] Agents")),
        ),
    ];
    let mut lines = vec![normal[11].clone()];
    let mut shapes = vec![(buffer.clone(), "Agents".to_string(), 11)];
    for (target, bar) in &bars {
        let buffer = draw((100, 1), |f| bar(f, f.area()));
        lines.push(rows(&buffer)[0].clone());
        shapes.push((buffer, target.to_string(), 0));
    }
    println!("── status bars ──\n{}", lines.join("\n"));
    assert!(
        lines[1].starts_with(" Input ▸ Telemetry  Esc Close"),
        "{}",
        lines[1]
    );
    assert!(lines[4].starts_with(" Input ▸ Drover  Esc Close  Ctrl-] Agents"));
    for (buffer, target, y) in shapes {
        let badge = format!(" Input ▸ {target} ").width() as u16;
        for x in 0..badge {
            let cell = &buffer[(x, y)];
            assert_eq!(
                (cell.fg, cell.bg),
                (t.input_text, t.focus),
                "{target} badge {x}"
            );
        }
        // One blank column, then the help in the secondary colour.
        assert_eq!(buffer[(badge, y)].symbol(), " ");
        assert_eq!(buffer[(badge + 1, y)].fg, t.muted, "{target} help");
    }
}

#[test]
fn a_short_notice_no_longer_leaves_the_end_of_the_bar_behind() {
    let t = Theme::default();
    let long = "Esc Close  Ctrl-] Agents  · Plugin is restarting after an unexpected exit";
    let notice = "Layout save failed: disk full";
    // Before: the notice was painted straight over the bar without clearing it.
    let before = draw((100, 1), |f| {
        ui::status_bar(&t, f, f.area(), "Drover", long);
        f.render_widget(Paragraph::new(notice).style(t.base()), f.area());
    });
    let before = rows(&before)[0].clone();
    // After: the notice helper clears the row, and so does every status bar.
    let after = draw((100, 1), |f| {
        ui::status_bar(&t, f, f.area(), "Drover", long);
        ui::status_notice(&t, f, f.area(), notice);
    });
    let after = rows(&after)[0].clone();
    let shorter = draw((100, 1), |f| {
        ui::status_bar(&t, f, f.area(), "Drover", long);
        ui::status_bar(&t, f, f.area(), "Plugins", "Esc Close");
    });
    let shorter = rows(&shorter)[0].clone();
    println!("── before ──\n{before}\n── after ──\n{after}\n── shorter bar ──\n{shorter}");
    assert!(
        before.contains("unexpected exit"),
        "the old residue: {before}"
    );
    assert_eq!(after.trim_end(), notice);
    assert_eq!(shorter.trim_end(), " Input ▸ Plugins  Esc Close");
}

#[test]
fn the_toast_stays_clear_of_the_active_pane_bottom_controls() {
    let t = Theme::default();
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let first = terminals.active_pane().id;
    terminals.focus(first);
    terminals.reserve(Place::Right, Some("p/b".into()));
    let area = Rect::new(0, 0, 90, 16);
    let toast = |f: &mut ratatui::Frame, area: Rect| {
        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(
            Paragraph::new("Task T7 needs review")
                .block(t.block(" Drover ", false))
                .style(t.base()),
            area,
        );
    };
    // Before: four rows ending on the terminal area's bottom row.
    let old = Rect::new(area.right() - 48, area.bottom() - 4, 48, 4);
    let mut hits = Vec::new();
    let before = draw((90, 16), |f| {
        hits = terminals::draw(&t, f, area, &terminals, true, &[]);
        toast(f, old);
    });
    show("before", &before);
    let controls: Vec<_> = hits
        .iter()
        .filter(|(_, c)| {
            matches!(
                c,
                Control::Split(_) | Control::ClosePane(_) | Control::Zoom(_)
            )
        })
        .map(|(h, _)| h.area)
        .collect();
    assert!(!controls.is_empty());
    assert!(
        controls.iter().any(|c| c.intersects(old)),
        "the old overlap"
    );
    let new = terminals::toast_area(area).unwrap();
    let after = draw((90, 16), |f| {
        terminals::draw(&t, f, area, &terminals, true, &[]);
        toast(f, new);
    });
    let lines = show("after", &after);
    assert_eq!((new.width, new.height, new.right()), (48, 4, area.right()));
    assert_eq!(new.bottom(), area.bottom() - 1);
    assert!(hits.iter().all(|(h, _)| !h.area.intersects(new)));
    assert!(lines[15].contains("Split ▾") && lines[15].contains("Close pane"));
    // Too small an area still shows no toast, as before.
    assert_eq!(terminals::toast_area(Rect::new(0, 0, 11, 20)), None);
    assert_eq!(terminals::toast_area(Rect::new(0, 0, 40, 3)), None);
}

#[test]
fn no_agents_and_a_search_without_matches_read_differently() {
    let t = Theme::default();
    let terminals = Terminals::new("unused-fake-corral".into());
    let mut empty = panel(vec![]);
    let (buffer, _) = workspace((100, 20), Focus::Agents, &terminals, None, &mut empty);
    let lines = show("no agents", &buffer);
    let at = lines.iter().position(|l| l.contains("No agents")).unwrap();
    assert!(
        lines[at + 1].contains("n New starts one"),
        "{}",
        lines[at + 1]
    );
    let (x, y) = find(&buffer, "n New starts one");
    assert_eq!(buffer[(x, y)].fg, t.agents_accent);
    // The bottom bar keeps its New control with the same key.
    assert!(lines.iter().any(|l| l.contains("n New s Sort")));

    let mut search = Search::default();
    let (buffer, _) = workspace(
        (100, 20),
        Focus::Agents,
        &terminals,
        Some(&mut search),
        &mut empty,
    );
    show("search, no agents", &buffer);
    find(&buffer, "No agents to search.");

    let mut some = panel(vec![agent("p/alpha", "/tmp/p")]);
    let mut search = Search::default();
    search.paste("zzz");
    let (buffer, _) = workspace(
        (100, 20),
        Focus::Agents,
        &terminals,
        Some(&mut search),
        &mut some,
    );
    let text = show("search, no match", &buffer).join("\n");
    assert!(text.contains("No agents match “zzz”."), "{text}");
    assert!(!text.contains("No agents to search"));
}

#[test]
fn stop_and_close_lead_with_the_target_and_mark_the_consequence() {
    let t = Theme::default();
    let terminals = Terminals::new("unused-fake-corral".into());
    let name = "saddle/dev-一个非常长的代理名字-with-a-long-tail";
    let mut p = panel(vec![Agent {
        last_tool: Some("apply_patch".into()),
        ..agent(name, "/tmp/p")
    }]);
    p.confirm = Some(name.into());
    let (buffer, hits) = workspace((100, 24), Focus::Agents, &terminals, None, &mut p);
    let text = show("stop", &buffer).join("\n");
    assert!(text.contains(&format!("Stop {name}?")) && text.contains("Instance: abcdef123"));
    assert!(text.contains("Input ▸ Confirm stop  y Stop  Any other key cancels"));
    let (x, y) = find(&buffer, "Stop saddle/");
    assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(x, y)].fg, t.bright);
    let (x, y) = find(&buffer, "Instance:");
    assert_eq!(buffer[(x, y)].fg, t.muted);
    let (x, y) = find(&buffer, "This stops the agent");
    assert_eq!(buffer[(x, y)].fg, t.danger);
    let (x, y) = find(&buffer, "Press y");
    assert_eq!(buffer[(x, y)].fg, t.muted);
    // Same two buttons and keys.
    let keys: Vec<_> = hits.buttons.iter().map(|h| h.key.code).collect();
    use crossterm::event::KeyCode;
    assert_eq!(keys, [KeyCode::Esc, KeyCode::Char('y')]);

    let snapshot = serde_json::json!([
        {"pane": 3, "shell": "zsh", "running_shell": true,
         "cwd": "/Users/example/Developer/personal_projs/saddle-worktrees/一个很长的工作目录"},
        {"pane": 4, "shell": "zsh", "running_shell": false, "cwd": "/tmp/idle"},
        {"pane": 5, "shell": "bash", "running_shell": true, "cwd": "/tmp/build"},
    ]);
    let mut hits = Vec::new();
    let buffer = draw((100, 24), |f| {
        let area = f.area();
        f.buffer_mut()
            .set_style(area, Style::default().bg(t.overlay));
        hits = ui::draw_close_terminals(&t, f, &snapshot, 0);
    });
    let text = show("close terminals", &buffer).join("\n");
    assert!(text.contains("End these running terminals and their foreground tasks?"));
    assert!(text.contains("Pane 3 · zsh") && text.contains("Pane 5 · bash"));
    assert!(!text.contains("/tmp/idle"));
    // The full directory stays readable, wrapped rather than cut.
    assert!(text.contains("  /Users/example/Developer/") && text.contains("工作目录"));
    let (x, y) = find(&buffer, "End these");
    assert_eq!(buffer[(x, y)].fg, t.danger);
    assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
    let (x, y) = find(&buffer, "/tmp/build");
    assert_eq!(buffer[(x, y)].fg, t.muted);
    let (x, y) = find(&buffer, "Agent displays");
    assert_eq!(buffer[(x, y)].fg, t.text);
    let keys: Vec<_> = hits.iter().map(|h| h.key.code).collect();
    assert_eq!(keys, [KeyCode::Esc, KeyCode::Char('y')]);
}

#[test]
fn long_shell_directories_keep_their_last_levels_in_the_pane_title() {
    let t = Theme::default();
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let pane = terminals.active_pane().id;
    let cwd = "/Users/example/Developer/personal_projs/saddle-worktrees/ui2-host/plugins/drover";
    terminals.get_mut(pane).unwrap().viewer.shell = Some(saddle::viewer::Shell {
        program: "zsh".into(),
        cwd: cwd.into(),
        state: "running",
        exit_code: None,
        env: vec![],
    });
    for width in [120, 60, 40] {
        let area = Rect::new(0, 0, width, 10);
        let mut hits = Vec::new();
        let buffer = draw((width, 10), |f| {
            hits = terminals::draw(&t, f, area, &terminals, true, &[]);
        });
        let lines = show(&format!("shell title {width}"), &buffer);
        let title = &lines[3];
        assert!(title.contains("Terminal · running · "), "{title}");
        if width == 120 {
            assert!(title.contains(cwd), "{title}");
        } else {
            assert!(title.contains("…/") && title.contains("drover "), "{title}");
        }
        // The title is still the pane's focus target, as wide as what is drawn.
        let target = hits
            .iter()
            .find(|(_, c)| matches!(c, Control::Pane(_)))
            .unwrap()
            .0
            .area;
        assert!(target.right() < width - 1);
    }
}

#[test]
fn saved_pane_actions_sit_right_below_their_explanation() {
    let t = Theme::default();
    let layout = serde_json::from_value(serde_json::json!({"version": 2, "active": 1, "tabs": [
        {"id": 1, "active": 2,
         "tree": {"kind": "split", "value": {"vertical": false, "ratio": 50,
            "first": {"kind": "leaf", "value": 2}, "second": {"kind": "leaf", "value": 3}}},
         "panes": [
            {"id": 2, "content": {"kind": "agent", "name": "saddle/dev-ui2-host",
              "cwd": "/Users/example/Developer/personal_projs/saddle-worktrees/ui2-host",
              "instance": null}},
            {"id": 3, "content": {"kind": "shell", "cwd": "/tmp/build"}}]}
    ]}))
    .unwrap();
    let terminals = Terminals::restore("unused-fake-corral".into(), layout).unwrap();
    for height in [24, 9] {
        let area = Rect::new(0, 0, 80, height);
        let mut hits = Vec::new();
        let buffer = draw((80, height), |f| {
            hits = terminals::draw(&t, f, area, &terminals, true, &[]);
        });
        let lines = show(&format!("saved panes {height}"), &buffer);
        let row = |label: &str| find(&buffer, label).1;
        let create = row("Create new agent");
        let choose = row("Choose existing agent");
        let open = row("Open terminal");
        assert_eq!(choose, create + 1);
        // Each action stays a clickable target on the row it is drawn on.
        let at = |y: u16, wanted: fn(&Control) -> bool| {
            hits.iter().any(|(h, c)| h.area.y == y && wanted(c))
        };
        assert!(at(create, |c| matches!(c, Control::CreateAgent(_))));
        assert!(at(choose, |c| matches!(c, Control::ChooseAgent(_))));
        assert!(at(open, |c| matches!(c, Control::OpenTerminal(_))));
        // The directory sits under the name as secondary text, keeping its last levels.
        let (x, y) = find(&buffer, "…/saddle-worktrees/ui2-host");
        assert_eq!(y, row("saddle/dev-ui2-host  ") + 1);
        assert_eq!(buffer[(x, y)].fg, t.muted);
        if height == 24 {
            assert_eq!(row("Saved agent position."), y + 1);
            // One blank row after the explanation, instead of the bottom of a tall pane.
            assert!(
                lines[usize::from(create) - 1]
                    .trim_matches(['│', '┃', ' '])
                    .is_empty()
            );
            assert!(create < height - 3, "{create}");
            assert_eq!(open, row("commands are not replayed.") + 2);
        } else {
            // A short pane still keeps the actions on its last body rows, now on clean rows.
            assert_eq!(choose, height - 2);
            assert!(lines[usize::from(create)].contains("Create new agent      "));
        }
    }
}
