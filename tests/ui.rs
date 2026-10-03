use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};
use saddle::{
    agents,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    input::Focus,
    layout::Panes,
    ui::{self, View},
};
fn fixture() -> (agents::Panel, ()) {
    let mut agents = agents::Panel {
        follow: true,
        ..Default::default()
    };
    agents.absorb(
        vec![Agent {
            name: "demo/main".into(),
            kind: Some("codex".into()),
            state: Some("working".into()),
            instance: Some("abcdef123".into()),
            title: Some("中文任务".into()),
            cwd: Some("/tmp/demo".into()),
            last_output: Some(99.0),
            turn_started: Some(99.0),
            ..Default::default()
        }],
        None,
        100.0,
    );
    (agents, ())
}
fn render(w: u16, h: u16, a: &mut agents::Panel, _q: &mut (), focus: Focus) -> (Buffer, ui::Hits) {
    render_at(w, h, a, focus, 100.0)
}
fn render_at(w: u16, h: u16, a: &mut agents::Panel, focus: Focus, now: f64) -> (Buffer, ui::Hits) {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    let panes = Panes::new(Rect::new(0, 0, w, h), &Config::default());
    let mut hits = ui::Hits::default();
    terminal
        .draw(|frame| {
            hits = ui::draw(
                frame,
                a,
                View {
                    colors: &saddle::theme::Theme::default(),
                    panes,
                    focus,
                    showing: Some("demo/main"),
                    local: &["demo/main".to_string()],
                    viewer: None,
                    projects: &[],
                    viewer_note: "测试终端",
                    reply: "上一轮回复",
                    now,
                    pointer: &Pointer::default(),
                },
            );
        })
        .unwrap();
    (terminal.backend().buffer().clone(), hits)
}
fn text(buffer: &Buffer) -> String {
    use unicode_width::UnicodeWidthStr;
    let mut text = String::new();
    for y in 0..buffer.area.height {
        let mut x = 0;
        while x < buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            text.push_str(symbol);
            x += symbol.width().max(1) as u16;
        }
        text.push('\n');
    }
    text
}
#[test]
fn overlays_remove_background_targets_and_small_frames_do_not_panic() {
    let (mut a, mut q) = fixture();
    a.confirm = Some("demo/main".into());
    let (buffer, hits) = render(120, 36, &mut a, &mut q, Focus::Agents);
    assert!(text(&buffer).contains("abcdef123"));
    assert!(hits.agents.is_empty());
    for w in 1..12 {
        for h in 1..10 {
            render(w, h, &mut a, &mut q, Focus::Agents);
            a.confirm = None;
        }
    }
}

#[test]
fn each_agents_extra_info_stays_with_its_row_when_reply_opens() {
    let (mut a, mut q) = fixture();
    a.agents[0].title = Some("FIRST-TITLE".into());
    a.agents[0].last_input_source = Some("human".into());
    a.agents[0].cwd = Some("/tmp/first".into());
    a.agents.push(Agent {
        name: "demo/second".into(),
        kind: Some("claude".into()),
        instance: Some("second123".into()),
        cwd: Some("/tmp/second".into()),
        title: Some("SECOND-TITLE".into()),
        state: Some("idle".into()),
        ..Default::default()
    });
    for reply in [false, true] {
        a.show_reply = reply;
        let (buffer, hits) = render(160, 100, &mut a, &mut q, Focus::Agents);
        for (name, markers) in [
            (
                "demo/main",
                ["FIRST-TITLE", "abcdef", "/tmp/first", "VIA human"],
            ),
            (
                "demo/second",
                ["SECOND-TITLE", "second", "/tmp/second", "claude"],
            ),
        ] {
            let rows: String = hits
                .agents
                .iter()
                .filter(|(_, n)| n == name)
                .map(|(y, _)| {
                    (0..52)
                        .map(|x| buffer[(x, *y)].symbol())
                        .collect::<String>()
                })
                .collect();
            assert!(!rows.contains("abcdef123") && !rows.contains("second123"));
            for marker in markers {
                assert!(
                    rows.contains(marker),
                    "{name} missing {marker}, reply={reply}: {rows}"
                );
            }
        }
        assert_eq!(hits.reply.is_empty(), !reply);
        if reply {
            assert!(text(&buffer).contains("上一轮回复"));
        }
    }
}

#[test]
fn group_headings_and_gutters_replace_the_tree_and_highlight_only_the_selected_agent() {
    use saddle::theme;
    let (mut a, mut q) = fixture();
    a.agents[0].cwd = Some("/Users/example/Developer/work/demo".into());
    a.agents[0].last_input_source = Some("human".into());
    a.agents.extend([
        Agent {
            name: "demo/review".into(),
            state: Some("blocked".into()),
            title: Some("Review changes".into()),
            ..Default::default()
        },
        Agent {
            name: "other/main".into(),
            state: Some("idle".into()),
            ..Default::default()
        },
    ]);
    for by_state in [false, true] {
        a.by_name = !by_state;
        let (buffer, hits) = render(160, 100, &mut a, &mut q, Focus::Agents);
        let output = text(&buffer);
        // The directory repeats the group, so only its parent shows.
        assert!(output.contains("…/work/ "), "{output}");
        let panel = agents_lines(&buffer).join("\n");
        for label in ["SOURCE ", "DIR ", "TITLE ", "/Users/example", "├", "└"] {
            assert!(!panel.contains(label), "{label}: {panel}");
        }
        assert!(output.contains("Review changes"));
        let headline = |name: &str| hits.agents.iter().find(|(_, n)| n == name).unwrap().0;
        let first = if by_state { "demo/review" } else { "demo/main" };
        let last = if by_state { "demo/main" } else { "demo/review" };
        // A group heading sits right above its first agent; entries follow without gaps and
        // groups are one blank row apart.
        let lines: Vec<_> = output.lines().collect();
        assert!(lines[usize::from(headline(first)) - 1].contains("demo/ ─"));
        let first_rows = hits.agents.iter().filter(|(_, n)| n == first).count() as u16;
        assert_eq!(headline(last), headline(first) + first_rows);
        let last_rows = hits.agents.iter().filter(|(_, n)| n == last).count() as u16;
        assert_eq!(headline("other/main"), headline(last) + last_rows + 2);
        for (y, name) in &hits.agents {
            let selected = name == "demo/main";
            assert_eq!(buffer[(2, *y)].symbol(), if selected { "┃" } else { "│" });
            assert_eq!(
                buffer[(2, *y)].fg,
                if selected {
                    theme::AGENTS_ACCENT
                } else {
                    theme::AGENTS_FAINT
                }
            );
            assert_eq!(
                buffer[(45, *y)].bg,
                if selected {
                    theme::AGENT_SELECTED
                } else {
                    theme::AGENTS_BG
                }
            );
        }
        assert!(output.contains("[Attached]"), "{output}");
    }
    a.follow = false;
    a.top = 2;
    a.by_name = true;
    let (buffer, _) = render(80, 20, &mut a, &mut q, Focus::Agents);
    let output = text(&buffer);
    assert!(
        output.contains("Agents · 3") && output.contains("demo/ · ↑"),
        "{output}"
    );
    for button in ["[Attached]", "n New", "s Name", "x Stop", "z Fold"] {
        assert!(output.contains(button), "{output}");
    }
    assert!(!output.contains("Reply r"));
    assert!(!output.contains("Show in"), "{output}");
}

#[test]
fn agent_type_marks_and_names_share_brand_color_without_changing_selection_or_state() {
    use ratatui::style::Color;
    use saddle::theme;
    use unicode_width::UnicodeWidthStr;
    for (kind, label, color) in [
        ("claude", "✳ claude", Color::Rgb(0xe2, 0x83, 0x5e)),
        ("codex", ">_ codex", Color::Rgb(0x79, 0xd4, 0xb4)),
        ("pi", "π pi", Color::Rgb(0xff, 0xff, 0xff)),
        ("omp", "π omp", Color::Rgb(0xa8, 0x55, 0xf7)),
        ("custom", "custom", theme::AGENTS_DIM),
    ] {
        for selected in [false, true] {
            let (mut a, mut q) = fixture();
            a.agents[0].kind = Some(kind.into());
            a.selected = selected.then(|| "demo/main".into());
            let (buffer, hits) = render(160, 48, &mut a, &mut q, Focus::Agents);
            let y = hits.agents[0].0;
            let mut line = String::new();
            let mut column = 1;
            while column < 50 {
                let symbol = buffer[(column, y)].symbol();
                line.push_str(symbol);
                column += symbol.width().max(1) as u16;
            }
            let offset = line.find(label).expect(&line);
            let x = 1 + line[..offset].width() as u16;
            for column in x..x + label.width() as u16 {
                assert_eq!(buffer[(column, y)].fg, color, "{label}");
                if kind == "codex" {
                    assert!(
                        buffer[(column, y)]
                            .modifier
                            .contains(ratatui::style::Modifier::BOLD)
                    );
                }
                assert_eq!(
                    buffer[(column, y)].bg,
                    if selected {
                        theme::AGENT_SELECTED
                    } else {
                        theme::AGENTS_BG
                    }
                );
            }
            assert!(line.contains("working"), "{line}");
            assert!(line.contains("⦿ 1s"), "{line}");
            let state = 1 + line[..line.find("working").unwrap()].width() as u16;
            assert_eq!(buffer[(state, y)].fg, theme::AGENTS_BLUE);
        }
    }
}

#[test]
fn agents_chrome_is_english_and_uses_terminal_colors_while_data_stays_verbatim() {
    // Public states keep their meaning; blocked reads as waiting in Agents.
    for (state, shown) in [
        ("working", "working"),
        ("idle", "idle"),
        ("blocked", "waiting"),
        ("starting", "starting"),
        ("unknown", "unknown"),
    ] {
        let (mut a, mut q) = fixture();
        a.agents[0].state = Some(state.into());
        a.agents[0].last_tool = Some("读取文件".into());
        let (buffer, hits) = render(160, 60, &mut a, &mut q, Focus::Agents);
        let panes = Panes::new(buffer.area, &Config::default());
        let mut chrome = String::new();
        for y in panes.agents.y..panes.agents.bottom() {
            for x in panes.agents.x..panes.agents.right() {
                let cell = &buffer[(x, y)];
                chrome.push_str(cell.symbol());
                // A wide character's second cell is drawn with the character itself.
                let continuation = x > panes.agents.x
                    && unicode_width::UnicodeWidthStr::width(buffer[(x - 1, y)].symbol()) == 2;
                assert!(
                    continuation
                        || cell.bg == saddle::theme::AGENTS_BG
                        || cell.bg == saddle::theme::AGENT_SELECTED,
                    "{x},{y} {:?}",
                    cell.bg
                );
            }
        }
        // Buffer wide-cell continuations are spaces, so remove them for this language check.
        let compact = chrome.replace(' ', "");
        assert!(compact.contains("中文任务"));
        let internal = compact.replace("中文任务", "").replace("读取文件", "");
        assert!(
            !internal
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "{chrome}"
        );
        assert!(chrome.contains(shown), "{chrome}");
        assert!(!chrome.contains("Reply") && hits.reply.is_empty());
        assert!(text(&buffer).contains("Input ▸ Agents"));
    }
    let (mut a, mut q) = fixture();
    a.confirm = Some("demo/main".into());
    let (buffer, _) = render(160, 48, &mut a, &mut q, Focus::Agents);
    let output = text(&buffer);
    assert!(output.contains("Stop demo/main?") && output.contains("Instance: abcdef123"));
}

#[test]
fn agent_totals_and_repo_counts_stay_visible_and_aligned() {
    let (mut a, mut q) = fixture();
    a.agents.extend([
        Agent {
            name: "demo/review".into(),
            ..Default::default()
        },
        Agent {
            name: "other/main".into(),
            ..Default::default()
        },
    ]);
    for by_state in [false, true] {
        a.by_name = !by_state;
        let (buffer, hits) = render(160, 100, &mut a, &mut q, Focus::Agents);
        let output = text(&buffer);
        assert!(
            output.lines().nth(1).unwrap().contains("Agents · 3"),
            "{output}"
        );
        for (repo, count) in [("demo/", "2"), ("other/", "1")] {
            let y = output
                .lines()
                .position(|line| line.starts_with(&format!("┃ {repo}")))
                .unwrap() as u16;
            assert_eq!(buffer[(hits.list.right() - 3, y)].symbol(), "(");
            assert_eq!(buffer[(hits.list.right() - 2, y)].symbol(), count);
            assert_eq!(buffer[(hits.list.right() - 1, y)].symbol(), ")");
        }
    }
    a.agents.truncate(1);
    a.agents[0].name = "a-very-long-repository-name-that-needs-truncation/main".into();
    a.selected = Some(a.agents[0].name.clone());
    let (buffer, hits) = render(80, 24, &mut a, &mut q, Focus::Agents);
    assert!(text(&buffer).contains("Agents · 1"));
    assert_eq!(buffer[(hits.list.right() - 3, hits.list.y)].symbol(), "(");
    assert_eq!(buffer[(hits.list.right() - 2, hits.list.y)].symbol(), "1");
    a.agents.clear();
    a.selected = None;
    let (buffer, _) = render(80, 24, &mut a, &mut q, Focus::Agents);
    assert!(text(&buffer).contains("Agents · 0"));
}

#[test]
fn agent_states_have_the_designed_dots_colors_labels_and_activity() {
    use ratatui::style::Modifier;
    use saddle::theme as t;
    use unicode_width::UnicodeWidthStr;
    // (case, dot, label, color, activity line)
    for (case, dot, label, color, activity) in [
        (
            "waiting",
            "?",
            "waiting",
            t::AGENTS_YELLOW,
            Some("ASK waiting for input"),
        ),
        (
            "error",
            "!",
            "error",
            t::AGENTS_RED,
            Some("ERR Synthetic error"),
        ),
        (
            "working",
            "◐",
            "working",
            t::AGENTS_BLUE,
            Some("DOING 读取文件"),
        ),
        ("idle", "○", "idle", t::AGENTS_GREEN, None),
        ("exited", "✕", "exited", t::AGENTS_FAINT, None),
        // Extra public states keep their own recognisable look.
        (
            "stalled",
            "▲",
            "stalled",
            t::AGENT_STALLED,
            Some("DOING 读取文件"),
        ),
        ("starting", "◌", "starting", t::AGENT_STARTING, None),
        ("unknown", "·", "unknown", t::AGENTS_DIM, None),
    ] {
        for selected in [false, true] {
            let (mut a, mut q) = fixture();
            a.selected = selected.then(|| "demo/main".into());
            a.agents[0].last_tool = Some("读取文件".into());
            a.agents[0].state = Some(
                match case {
                    "waiting" => "blocked",
                    "stalled" | "error" => "working",
                    other => other,
                }
                .into(),
            );
            if case == "stalled" {
                a.agents[0].last_output = Some(-21.0);
            } else if case == "error" {
                a.agents[0].error = Some("Synthetic error".into());
            }
            let (buffer, hits) = render(160, 48, &mut a, &mut q, Focus::Agents);
            let output = text(&buffer);
            let y = hits.agents[0].0;
            let row = output.lines().nth(y as usize).unwrap();
            // The dot follows the gutter and its padding; working cycles ◐◓◑◒.
            let shown = buffer[(4, y)].symbol();
            if case == "working" {
                assert!("◐◓◑◒".contains(shown), "{row}");
            } else {
                assert_eq!(shown, dot, "{case}: {row}");
            }
            assert_eq!(buffer[(4, y)].fg, color, "{case}");
            let start = row[..row.find(label).expect(row)].width() as u16;
            for x in start..start + label.len() as u16 {
                assert_eq!(buffer[(x, y)].fg, color, "{case}");
                assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
            }
            let name = &buffer[(6, y)];
            assert_eq!(
                name.fg,
                if case == "exited" {
                    t::AGENTS_FAINT
                } else {
                    t::AGENTS_TEXT
                },
                "{case}"
            );
            let rows: String = hits
                .agents
                .iter()
                .map(|(y, _)| output.lines().nth(*y as usize).unwrap())
                .collect();
            match activity {
                Some(line) => {
                    assert!(rows.contains(line), "{case}: {rows}");
                    let label = line.split(' ').next().unwrap();
                    let (x, y) = find(&buffer, label).unwrap();
                    assert_eq!(buffer[(x, y)].fg, color, "{case}");
                }
                None => assert!(
                    !["ASK", "ERR", "DOING"].iter().any(|l| rows.contains(l)),
                    "{case}: {rows}"
                ),
            }
        }
    }
}

#[test]
fn working_duration_survives_output_and_animation_refreshes() {
    let (mut a, _) = fixture();
    a.agents[0].turn_started = Some(70.0);
    for (now, output, expected) in [
        (100.0, 99.0, "30s"),
        (100.4, 100.3, "30s"),
        (101.0, 100.9, "31s"),
    ] {
        let mut updated = a.agents[0].clone();
        updated.last_output = Some(output);
        a.absorb(vec![updated], None, now);
        let (buffer, _) = render_at(160, 40, &mut a, Focus::Agents, now);
        let lines = agents_lines(&buffer);
        let headline = lines.iter().find(|line| line.contains("working")).unwrap();
        assert!(headline.contains(expected), "{headline}");
        let doing = lines.iter().find(|line| line.contains("DOING")).unwrap();
        assert!(doing.contains(expected), "{doing}");
    }
}

#[test]
fn idle_and_waiting_duration_use_state_entry_not_output_events_or_turn() {
    for (state, label) in [("idle", "idle"), ("blocked", "waiting")] {
        let (mut a, _) = fixture();
        for (now, since, expected) in [
            (100.0, 75.0, "25s"),
            (101.0, 75.0, "26s"),
            (102.0, 101.0, "1s"),
        ] {
            let agent = serde_json::from_value(serde_json::json!({
                "name":"demo/main", "instance":"abcdef123", "state":state,
                "turn_started":20.0, "state_started":since,
                "last_output":now, "last_event_at":now
            }))
            .unwrap();
            a.absorb(vec![agent], None, now);
            let (buffer, _) = render_at(160, 40, &mut a, Focus::Agents, now);
            let lines = agents_lines(&buffer);
            let headline = lines.iter().find(|line| line.contains(label)).unwrap();
            assert!(headline.contains(expected), "{headline}");
            if state == "blocked" {
                let ask = lines.iter().find(|line| line.contains("ASK")).unwrap();
                assert!(ask.contains(expected), "{ask}");
            }
        }
    }
}

#[test]
fn missing_state_origin_is_unknown_even_with_recent_output_and_events() {
    for (state, label) in [
        ("working", "working"),
        ("idle", "idle"),
        ("blocked", "waiting"),
    ] {
        let (mut a, _) = fixture();
        a.absorb(
            vec![
                serde_json::from_value(serde_json::json!({
                    "name":"demo/main", "state":state,
                    "last_output":99.0, "last_event_at":99.0,
                    "turn_started": if state == "working" { None } else { Some(20.0) }
                }))
                .unwrap(),
            ],
            None,
            100.0,
        );
        let (buffer, _) = render_at(160, 40, &mut a, Focus::Agents, 100.0);
        let lines = agents_lines(&buffer);
        let headline = lines.iter().find(|line| line.contains(label)).unwrap();
        assert!(headline.contains('—'), "{headline}");
        for line in lines
            .iter()
            .filter(|line| line.contains("ASK") || line.contains("DOING"))
        {
            assert!(line.contains("· —"), "{line}");
        }
    }
}

#[test]
fn working_dot_advances_every_360ms() {
    let (mut a, mut q) = fixture();
    let frame = |a: &mut agents::Panel, _q: &mut (), now: f64| {
        let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        let panes = Panes::new(Rect::new(0, 0, 160, 40), &Config::default());
        terminal
            .draw(|frame| {
                ui::draw(
                    frame,
                    a,
                    View {
                        colors: &saddle::theme::Theme::default(),
                        panes,
                        focus: Focus::Agents,
                        showing: None,
                        local: &[],
                        viewer: None,
                        projects: &[],
                        viewer_note: "",
                        reply: "",
                        now,
                        pointer: &Pointer::default(),
                    },
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let y = (0..40).find(|y| buffer[(6, *y)].symbol() == "m").unwrap();
        buffer[(4, y)].symbol().to_owned()
    };
    let start = frame(&mut a, &mut q, 100.1);
    assert_eq!(frame(&mut a, &mut q, 100.15), start);
    assert_ne!(frame(&mut a, &mut q, 100.47), start);
}

#[test]
fn agents_scrollbar_reaches_the_bottom_when_the_last_row_is_visible() {
    for (extra, height) in [(3, 20), (12, 40)] {
        let (mut a, mut q) = fixture();
        for index in 0..extra {
            // The instance line is each entry's last row.
            a.agents.push(Agent {
                name: format!("demo/worker-{index:02}"),
                instance: Some(format!("END-{index:02}")),
                ..Default::default()
            });
        }
        a.follow = false;
        a.top = usize::MAX;
        a.fold = Some(false);
        let (buffer, hits) = render(160, height, &mut a, &mut q, Focus::Agents);
        let output = text(&buffer);
        assert!(
            output.contains(&format!("END-{:02}", extra - 1)),
            "{output}"
        );
        assert_eq!(
            buffer[(hits.list.right(), hits.list.bottom() - 1)].symbol(),
            "█",
            "{output}"
        );
    }
}

#[test]
fn multi_agent_layout_gives_names_room_and_keeps_every_field_with_its_agent() {
    let (mut a, mut q) = fixture();
    a.agents = [
        ("claude-demo-1", "claude"),
        ("omp-demo-1", "omp"),
        ("pi-demo-1", "pi"),
    ]
    .into_iter()
    .map(|(name, kind)| Agent {
        name: format!("demo/{name}"),
        kind: Some(kind.into()),
        state: Some("idle".into()),
        instance: Some("abcdef123".into()),
        cwd: Some("/tmp/work".into()),
        title: Some(format!("{kind} demo ready")),
        last_input_source: Some("human".into()),
        ..Default::default()
    })
    .collect();
    a.selected = Some("demo/claude-demo-1".into());
    for width in [160, 80] {
        let (buffer, hits) = render(width, 100, &mut a, &mut q, Focus::Agents);
        let output = text(&buffer);
        let mut previous_end = None;
        for agent in &a.agents {
            let rows: Vec<_> = hits
                .agents
                .iter()
                .filter(|(_, name)| name == &agent.name)
                .map(|(y, _)| *y)
                .collect();
            let info: String = rows
                .iter()
                .map(|y| {
                    (hits.list.x + 2..hits.list.right())
                        .map(|x| buffer[(x, *y)].symbol())
                        .collect::<String>()
                })
                .collect();
            // Narrow panels show the type's mark alone.
            let kind = match agent.kind.as_deref().unwrap() {
                "claude" if width == 80 => "✳",
                "omp" | "pi" if width == 80 => "π",
                kind => kind,
            };
            for field in [
                kind,
                "abcdef",
                "ATT 0",
                "VIA human",
                "/tmp/work",
                agent.title.as_deref().unwrap(),
            ] {
                assert!(
                    info.replace(' ', "").contains(&field.replace(' ', "")),
                    "missing {field}: {info}"
                );
            }
            if width == 160 {
                let name = agent.name.strip_prefix("demo/").unwrap();
                assert!(output.lines().nth(rows[0] as usize).unwrap().contains(name));
                assert_eq!(info.matches(name).count(), 1);
                assert_eq!(rows.len(), 5); // main, title, Git line, path, identity
            }
            if let Some(end) = previous_end {
                assert_eq!(rows[0], end + 1); // Entries of a group follow without spacers.
            }
            previous_end = rows.last().copied();
        }
    }
}

fn assert_label_color(buffer: &Buffer, label: &str, color: ratatui::style::Color) {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if x as usize + label.chars().count() > buffer.area.width as usize {
                continue;
            }
            if label
                .chars()
                .enumerate()
                .all(|(i, c)| buffer[(x + i as u16, y)].symbol() == c.to_string())
            {
                for i in 0..label.chars().count() {
                    assert_eq!(buffer[(x + i as u16, y)].fg, color, "{label}");
                }
                return;
            }
        }
    }
    panic!("missing {label}: {}", text(buffer));
}

#[test]
fn delegated_effort_shows_strength_bars_and_unknown_stays_blank() {
    use saddle::theme;
    let (mut a, mut q) = fixture();
    let (before, _) = render(160, 48, &mut a, &mut q, Focus::Agents);
    assert!(!text(&before).contains(['⣄', '⣴'])); // no labels: no icon column
    a.agents = [
        ("medium", Some("medium")),
        ("high", Some("high")),
        ("xhigh", Some("xhigh")),
        ("manual", None),
        ("other", Some("max")),
    ]
    .into_iter()
    .map(|(name, effort)| Agent {
        name: format!("demo/{name}"),
        kind: Some("claude".into()),
        state: Some("idle".into()),
        labels: effort
            .map(|e| {
                serde_json::json!({ "effort": e })
                    .as_object()
                    .unwrap()
                    .clone()
            })
            .unwrap_or_default(),
        ..Default::default()
    })
    .collect();
    for width in [160, 80] {
        let (buffer, hits) = render(width, 60, &mut a, &mut q, Focus::Agents);
        let mut columns = Vec::new();
        for (name, lit) in [
            ("medium", 1),
            ("high", 2),
            ("xhigh", 3),
            ("manual", 0),
            ("other", 0),
        ] {
            let y = hits
                .agents
                .iter()
                .find(|(_, n)| n == &format!("demo/{name}"))
                .unwrap()
                .0;
            let line: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
            let found = (0..width).find(|x| matches!(buffer[(*x, y)].symbol(), "⣄" | "⣴"));
            if lit == 0 {
                assert!(found.is_none(), "{line}");
                continue;
            }
            let x = found.expect(&line);
            columns.push(x);
            // Three bars occupy two cells, with distinct shapes and theme colors for each tier.
            let icon: String = (x..x + 2).map(|x| buffer[(x, y)].symbol()).collect();
            assert_eq!(icon, ["⣄⡀", "⣴⡀", "⣴⡇"][lit - 1], "{line}");
            let color = [
                theme::AGENT_IDLE,
                theme::AGENT_WORKING,
                theme::AGENT_STARTING,
            ][lit - 1];
            for i in 0..2 {
                let fg = buffer[(x + i as u16, y)].fg;
                assert_eq!(
                    fg,
                    if i == 0 || lit == 3 {
                        color
                    } else {
                        theme::DIM
                    },
                    "{name} bar {i}: {line}"
                );
            }
            assert!(line.contains("idle"), "{line}");
        }
        assert!(columns.windows(2).all(|w| w[0] == w[1]), "{columns:?}");
    }
}

#[test]
fn selecting_an_agent_keeps_its_effort_icon_tier() {
    let (mut a, mut q) = fixture();
    a.follow = false;
    a.agents = ["high", "other"]
        .into_iter()
        .map(|name| Agent {
            name: format!("demo/{name}"),
            kind: Some("claude".into()),
            state: Some("idle".into()),
            labels: serde_json::json!({ "effort": "high" })
                .as_object()
                .unwrap()
                .clone(),
            ..Default::default()
        })
        .collect();
    // The two icon cells sit right before " idle" on the agent's main row.
    let icon = |a: &mut agents::Panel, q: &mut (), focus| {
        let (buffer, hits) = render(160, 48, a, q, focus);
        let y = hits
            .agents
            .iter()
            .find(|(_, n)| n == "demo/high")
            .unwrap()
            .0;
        let x = (0..156)
            .find(|x| {
                (0..4).all(|i| buffer[(x + i, y)].symbol() == &"idle"[i as usize..=i as usize])
            })
            .unwrap();
        (x - 3..x - 1)
            .map(|x| (buffer[(x, y)].symbol().to_owned(), buffer[(x, y)].fg))
            .collect::<Vec<_>>()
    };
    a.selected = Some("demo/other".into());
    let unselected = icon(&mut a, &mut q, Focus::Viewer);
    assert_eq!(unselected[0].0, "⣴");
    a.selected = Some("demo/high".into());
    for focus in [Focus::Agents, Focus::Viewer] {
        assert_eq!(icon(&mut a, &mut q, focus), unselected, "{focus:?}");
    }
}

#[test]
fn git_summary_line_follows_each_agents_directory_and_wraps_when_narrow() {
    use saddle::{
        git::{Changes, Head, Summary},
        theme,
    };
    let (mut a, mut q) = fixture();
    a.agents = [
        ("dev", "/w/dev"),
        ("twin", "/w/dev"),
        ("main", "/w/main"),
        ("odd", "/w/odd"),
        ("gone", "/w/gone"),
        ("new", "/w/new"),
    ]
    .into_iter()
    .map(|(name, cwd)| Agent {
        name: format!("demo/{name}"),
        state: Some("idle".into()),
        cwd: Some(cwd.into()),
        ..Default::default()
    })
    .collect();
    let summary =
        |branch: &str, ahead: Option<(u64, &str)>, changes: Option<(u64, u64, u64)>, untracked| {
            Some(Summary {
                head: if branch.is_empty() {
                    Head::Detached
                } else {
                    Head::Branch(branch.into())
                },
                ahead: ahead.map(|(n, base)| (n, base.into())),
                changes: changes.map(|(added, deleted, binary)| Changes {
                    added,
                    deleted,
                    binary,
                }),
                untracked,
            })
        };
    a.git = [
        (
            "/w/dev",
            summary("dev-t12", Some((2, "main")), Some((18, 4, 0)), Some(1)),
        ),
        (
            "/w/main",
            summary("main", Some((0, "origin/main")), Some((0, 0, 2)), Some(0)),
        ),
        ("/w/odd", summary("", None, None, None)),
        ("/w/gone", None),
    ]
    .into_iter()
    .map(|(cwd, s)| (cwd.to_string(), s))
    .collect();
    a.fold = Some(false);
    let (buffer, hits) = render(160, 120, &mut a, &mut q, Focus::Agents);
    let screen = text(&buffer);
    let line = |y: u16| {
        (hits.list.x + 4..hits.list.right())
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    };
    let rows = |name: &str| -> Vec<String> {
        hits.agents
            .iter()
            .filter(|(_, n)| n == name)
            .map(|(y, _)| line(*y))
            .collect()
    };
    let git = |name: &str| -> String {
        let rows = rows(name);
        let at = rows.iter().position(|r| r.starts_with('⎇')).expect(&screen);
        rows[at].clone()
    };
    // Agents sharing a worktree show the same numbers; ↑ counts commits beyond the base.
    for name in ["demo/dev", "demo/twin"] {
        let row = git(name);
        assert!(row.starts_with("⎇ dev-t12 ↑2 main "), "{row}");
        assert!(row.trim_end().ends_with("+18 -4 ?1"), "{row}");
    }
    // A branch that repeats the agent's name shows only the mark.
    let row = git("demo/main");
    assert!(row.starts_with("⎇ ↑0 origin/main "), "{row}");
    assert!(row.trim_end().ends_with("+0 -0 2 binary ?0"), "{row}");
    let row = git("demo/odd");
    assert!(row.starts_with("⎇ HEAD detached ↑— "), "{row}");
    assert!(row.trim_end().ends_with("+— -— ?—"), "{row}");
    assert!(
        rows("demo/gone")[1].starts_with("git unavailable"),
        "{screen}"
    );
    assert!(rows("demo/new")[1].starts_with("git …"), "{screen}");
    let panel = agents_lines(&buffer).join("\n");
    assert!(!panel.contains("C2(") && !panel.contains('↓'), "{panel}");
    assert_label_color(&buffer, "+18", theme::AGENTS_GREEN);
    assert_label_color(&buffer, "-4", theme::AGENTS_RED);
    assert_label_color(&buffer, "↑2", theme::AGENTS_YELLOW);
    assert_label_color(&buffer, "↑0", theme::AGENTS_FAINT);
    assert_label_color(&buffer, "dev-t12", theme::AGENTS_BRANCH);
    // The directory follows its Git line.
    let main = rows("demo/main");
    let at = main.iter().position(|r| r.starts_with('⎇')).unwrap();
    assert!(main[at + 1].starts_with("/w/"), "{main:?}");
}

fn find(buffer: &Buffer, label: &str) -> Option<(u16, u16)> {
    let width = label.chars().count() as u16;
    (0..buffer.area.height).find_map(|y| {
        (0..buffer.area.width.saturating_sub(width - 1)).find_map(|x| {
            label
                .chars()
                .enumerate()
                .all(|(i, c)| buffer[(x + i as u16, y)].symbol() == c.to_string())
                .then_some((x, y))
        })
    })
}

#[test]
fn selected_agent_has_an_accent_gutter_and_background_on_every_line() {
    use ratatui::style::Modifier;
    use saddle::theme;
    let (mut a, mut q) = fixture();
    a.agents[0].title = Some("long title that must be cut ".repeat(12));
    a.agents.push(Agent {
        name: "demo/review".into(),
        state: Some("idle".into()),
        ..Default::default()
    });
    a.selected = Some("demo/main".into());
    for focus in [Focus::Agents, Focus::Viewer] {
        let (buffer, hits) = render(160, 60, &mut a, &mut q, focus);
        let rows = |name: &str| -> Vec<u16> {
            hits.agents
                .iter()
                .filter(|(_, n)| n == name)
                .map(|(y, _)| *y)
                .collect()
        };
        let selected = rows("demo/main");
        // Headline, title (cut to one line), activity, Git, path and instance.
        assert_eq!(selected.len(), 6, "{}", text(&buffer));
        assert!(text(&buffer).contains("long title that must be cut long"));
        for &y in &selected {
            assert_eq!(buffer[(2, y)].symbol(), "┃");
            assert_eq!(buffer[(2, y)].fg, theme::AGENTS_ACCENT);
            assert_eq!(buffer[(2, y)].bg, theme::AGENTS_BG);
            for x in 3..50 {
                assert_eq!(buffer[(x, y)].bg, theme::AGENT_SELECTED, "{x},{y}");
            }
        }
        assert!(buffer[(6, selected[0])].modifier.contains(Modifier::BOLD));
        for y in rows("demo/review") {
            assert_eq!(buffer[(2, y)].symbol(), "│");
            assert_eq!(buffer[(2, y)].fg, theme::AGENTS_FAINT);
            assert_eq!(buffer[(10, y)].bg, theme::AGENTS_BG);
        }
    }
}

#[test]
fn tab_hover_and_press_cover_the_whole_frame_with_separate_close_targets() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{layout::Position, style::Color};
    use saddle::terminals::{Control, Place, Terminals};
    let (mut panel, _) = fixture();
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let first = terminals.reserve(Place::Current, Some("p/a".into()));
    let second = terminals.reserve(Place::Tab, Some("p/b".into()));
    terminals.focus(first.pane);
    let colors = saddle::theme::Theme::default();
    let mut render = |pointer: &Pointer| {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut hits = ui::Hits::default();
        terminal
            .draw(|frame| {
                hits = ui::draw_workspace(
                    frame,
                    &mut panel,
                    View {
                        colors: &colors,
                        panes: Panes::new(frame.area(), &Config::default()),
                        focus: Focus::Viewer,
                        showing: None,
                        local: &[],
                        viewer: None,
                        projects: &[],
                        viewer_note: "",
                        reply: "",
                        now: 100.0,
                        pointer,
                    },
                    Some(ui::Workspace {
                        terminals: &terminals,
                        mascot: &mut Default::default(),
                        mascot_enabled: true,
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
        (terminal.backend().buffer().clone(), hits)
    };
    let (_, hits) = render(&Pointer::default());
    let body = hits
        .terminal
        .iter()
        .find(|(_, c)| matches!(c, Control::Tab(id) if *id == second.pane))
        .unwrap()
        .0
        .clone();
    let close = hits
        .terminal
        .iter()
        .find(|(_, c)| matches!(c, Control::CloseTab(id) if *id == second.pane))
        .unwrap()
        .0
        .clone();
    let targets: Vec<_> = hits
        .terminal
        .iter()
        .map(|(h, _)| (Focus::Viewer, h.clone()))
        .collect();
    for target in [&body, &close] {
        let point = Position::new(target.area.x, target.area.y + 1);
        let mut pointer = Pointer::default();
        pointer.hover = Some(point);
        for pressed in [false, true] {
            if pressed {
                pointer.event(
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: point.x,
                        row: point.y,
                        modifiers: KeyModifiers::NONE,
                    },
                    &targets,
                );
            }
            let (buffer, after) = render(&pointer);
            for y in body.area.y..body.area.bottom() {
                for x in body.area.x..close.area.right() {
                    assert_eq!(
                        buffer[(x, y)].fg,
                        if pressed { colors.focus } else { colors.bright },
                        "split highlight at {x},{y}"
                    );
                    assert_eq!(buffer[(x, y)].bg, Color::Reset);
                }
            }
            assert!(
                after
                    .terminal
                    .iter()
                    .any(|(h, c)| h == &body && matches!(c, Control::Tab(_)))
            );
            assert!(
                after
                    .terminal
                    .iter()
                    .any(|(h, c)| h == &close && matches!(c, Control::CloseTab(_)))
            );
        }
    }
}

#[test]
fn viewer_title_shows_the_public_role_label_or_agent_without_guessing() {
    let (mut a, mut q) = fixture();
    for (labels, title) in [
        (
            serde_json::json!({ "role": "controller" }),
            "Controller · demo/main",
        ),
        (
            serde_json::json!({ "role": "regular" }),
            "Regular · demo/main",
        ),
        (
            serde_json::json!({ "role": "implementer" }),
            "Implementer · demo/main",
        ),
        (
            serde_json::json!({ "role": "reviewer" }),
            "Reviewer · demo/main",
        ),
        (serde_json::json!({ "role": "tester" }), "Agent · demo/main"),
        (
            serde_json::json!({ "role": "Controller" }),
            "Agent · demo/main",
        ),
        (serde_json::json!({}), "Agent · demo/main"),
    ] {
        a.agents[0].labels = labels.as_object().unwrap().clone();
        let (buffer, _) = render(160, 40, &mut a, &mut q, Focus::Agents);
        let screen = text(&buffer);
        assert!(screen.contains(title), "{screen}");
        assert!(!screen.contains("Viewer · "), "{screen}");
    }
}

/// The four agents of the 3a design sample: corral / drover / saddle ×2.
fn spec_sample() -> agents::Panel {
    use saddle::git::{Changes, Head, Summary};
    let home = "/Users/example/Developer/personal_projs";
    let mut a = agents::Panel {
        follow: true,
        ..Default::default()
    };
    let agent = |name: &str, kind: &str, state: &str, cwd: String, instance: &str| Agent {
        name: name.into(),
        kind: Some(kind.into()),
        state: Some(state.into()),
        cwd: Some(cwd),
        instance: Some(instance.into()),
        ..Default::default()
    };
    a.absorb(
        vec![
            Agent {
                title: Some("✳ Claude Code".into()),
                attached: 1,
                last_input_source: Some("human".into()),
                last_output: Some(89.0),
                state_started: Some(89.0),
                ..agent(
                    "corral/main",
                    "claude",
                    "idle",
                    format!("{home}/corral"),
                    "160f6f01",
                )
            },
            Agent {
                title: Some("drover".into()),
                last_input_source: Some("agent".into()),
                last_output: Some(-500.0),
                state_started: Some(-500.0),
                ..agent(
                    "drover/main",
                    "codex",
                    "idle",
                    format!("{home}/drover"),
                    "be790d01",
                )
            },
            Agent {
                title: Some("⠪ t20-terminal-workspace".into()),
                last_tool: Some("apply_patch".into()),
                turn_started: Some(-380.0),
                last_output: Some(100.0),
                last_input_source: Some("send".into()),
                ..agent(
                    "saddle/dev-t20-workspace-1",
                    "codex",
                    "working",
                    format!("{home}/saddle-worktrees/t20-terminal-workspace"),
                    "b88a3701",
                )
            },
            Agent {
                last_output: Some(-20.0),
                state_started: Some(-20.0),
                last_input_source: Some("send".into()),
                ..agent(
                    "saddle/main",
                    "codex",
                    "idle",
                    format!("{home}/saddle"),
                    "154fac01",
                )
            },
        ],
        None,
        100.0,
    );
    let summary = |branch: &str, ahead: u64, base: &str, (added, deleted), untracked| {
        Some(Summary {
            head: Head::Branch(branch.into()),
            ahead: Some((ahead, base.into())),
            changes: Some(Changes {
                added,
                deleted,
                binary: 0,
            }),
            untracked: Some(untracked),
        })
    };
    a.git = [
        (
            format!("{home}/corral"),
            summary("main", 0, "origin/main", (0, 0), 0),
        ),
        (
            format!("{home}/drover"),
            summary("main", 0, "origin/main", (0, 0), 1),
        ),
        (
            format!("{home}/saddle-worktrees/t20-terminal-workspace"),
            summary("t20-terminal-workspace", 0, "main", (127, 3), 3),
        ),
        (
            format!("{home}/saddle"),
            summary("main", 3, "origin/main", (0, 0), 0),
        ),
    ]
    .into_iter()
    .collect();
    a.select(Some("corral/main".into()));
    a
}
/// Rows of the Agents pane, one string per screen row.
fn agents_lines(buffer: &Buffer) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let panes = Panes::new(buffer.area, &Config::default());
    (panes.agents.y..panes.agents.bottom())
        .map(|y| {
            let mut line = String::new();
            let mut x = panes.agents.x;
            while x < panes.agents.right() {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += symbol.width().max(1) as u16;
            }
            line
        })
        .collect()
}

/// Renders the Agents pane of `a` with `local` as this saddle's displayed agents.
fn render_panel(width: u16, a: &mut agents::Panel, local: &[String]) -> (Buffer, ui::Hits) {
    let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
    let panes = Panes::new(Rect::new(0, 0, width, 40), &Config::default());
    let mut hits = ui::Hits::default();
    terminal
        .draw(|frame| {
            hits = ui::draw(
                frame,
                a,
                View {
                    colors: &saddle::theme::Theme::default(),
                    panes,
                    focus: Focus::Agents,
                    showing: local.first().map(String::as_str),
                    local,
                    viewer: None,
                    projects: &[],
                    viewer_note: "",
                    reply: "",
                    now: 100.0,
                    pointer: &Pointer::default(),
                },
            );
        })
        .unwrap();
    (terminal.backend().buffer().clone(), hits)
}

#[test]
fn design_sample_fits_fifty_columns_without_wrapping() {
    let mut a = spec_sample();
    let (buffer, hits) = render_panel(160, &mut a, &["corral/main".into()]);
    let panes = Panes::new(buffer.area, &Config::default());
    assert_eq!(panes.agents.width - 2, 50);
    let lines = agents_lines(&buffer);
    let body: Vec<_> = lines[1..lines.len() - 1]
        .iter()
        .map(|l| l.trim_start_matches('┃').trim_end_matches('┃').to_owned())
        .collect();
    // Row for row as in the design (spinner frames as drawn at this instant).
    let expected = [
        " Agents · 4                              Settings ",
        " Attention · 0                                    ",
        " ──────────────────────────────────────────────── ",
        " corral/ ──────────────────────────────────── (1) ",
        " ┃ ○ main                ✳ claude idle      ⦿ 11s ",
        " ┃   ✳ Claude Code                                ",
        " ┃   ⎇ ↑0 origin/main                    +0 -0 ?0 ",
        " ┃   …/personal_projs/                            ",
        " ┃   160f6f · ATT 1 · VIA human                   ",
        "                                                  ",
        " drover/ ──────────────────────────────────── (1) ",
        " │ ○ main                >_ codex idle        10m ",
        " │   ⎇ ↑0 origin/main                    +0 -0 ?1 ",
        " │   …/personal_projs/                            ",
        " │   be790d · ATT 0 · VIA agent                   ",
        "                                                  ",
        " saddle/ ──────────────────────────────────── (2) ",
        " │ ◓ dev-t20-workspace-1 >_ codex working      8m ",
        " │   ⠪ t20-terminal-workspace                     ",
        " │   DOING apply_patch · 8m                       ",
        " │   ⎇ t20-terminal-workspace ↑0 main  +127 -3 ?3 ",
        " │   …/saddle-worktrees/t20-terminal-workspace    ",
        " │   b88a37 · ATT 0 · VIA send                    ",
        " │ ○ main                >_ codex idle         2m ",
        " │   ⎇ ↑3 origin/main                    +0 -0 ?0 ",
        " │   …/personal_projs/                            ",
        " │   154fac · ATT 0 · VIA send                    ",
    ];
    for (i, line) in expected.iter().enumerate() {
        assert_eq!(&body[i], line, "row {i}:\n{}", body.join("\n"));
    }
    assert!(
        !body
            .iter()
            .any(|l| l.contains('…') && !l.contains("…/") && !l.contains("Loading…"))
    );
    let bar = &body[body.len() - 1];
    // T23 adds Search; the existing bar layout falls back to one-column gaps to stay on one row.
    assert_eq!(
        bar.trim_end(),
        " [Attached] / Search n New s Sort x Stop z Fold"
    );
    assert_eq!(&body[body.len() - 2], expected[2]);
    // Every row of an entry belongs to it; headings, blanks and rules take no clicks.
    assert_eq!(hits.agents.len(), 5 + 4 + 6 + 4);
    use saddle::theme as t;
    assert_label_color(&buffer, "⦿ 11s", t::AGENTS_GREEN);
    assert_label_color(&buffer, "ATT 1", t::AGENTS_GREEN);
    assert_label_color(&buffer, "ATT 0", t::AGENTS_DIM);
    assert_label_color(&buffer, "↑3", t::AGENTS_YELLOW);
    // Group heading: accent name, faint line, dim count.
    assert_eq!(buffer[(2, 4)].fg, t::AGENTS_ACCENT);
    assert_eq!(buffer[(10, 4)].fg, t::AGENTS_FAINT);
    assert_eq!(buffer[(47, 4)].fg, t::AGENTS_DIM);
    assert_label_color(&buffer, "8m", t::AGENTS_BLUE);
    assert_label_color(&buffer, "…/personal_projs/", t::AGENTS_DIM);
}

#[test]
fn a_diff_too_wide_for_its_row_moves_whole_to_the_next_row_right_aligned() {
    use saddle::git::Changes;
    let mut a = spec_sample();
    let cwd = a
        .agents
        .iter()
        .find(|x| x.name == "saddle/dev-t20-workspace-1")
        .unwrap()
        .cwd
        .clone()
        .unwrap();
    a.git.get_mut(&cwd).unwrap().as_mut().unwrap().changes = Some(Changes {
        added: 12847,
        deleted: 3291,
        binary: 0,
    });
    a.git.get_mut(&cwd).unwrap().as_mut().unwrap().untracked = Some(128);
    let (buffer, _) = render_panel(160, &mut a, &[]);
    let lines = agents_lines(&buffer);
    let at = lines
        .iter()
        .position(|l| l.contains("⎇ t20-terminal-workspace"))
        .unwrap();
    assert_eq!(
        lines[at],
        "┃ │   ⎇ t20-terminal-workspace ↑0 main             ┃"
    );
    let diff = "+12847 -3291 ?128";
    assert_eq!(
        lines[at + 1],
        format!("┃ │{}{diff} ┃", " ".repeat(50 - 3 - diff.len()))
    );
    assert!(lines[at + 2].contains("…/saddle-worktrees/t20-terminal-workspace"));
}

#[test]
fn selecting_saddle_main_highlights_only_that_entry() {
    use saddle::theme as t;
    let mut a = spec_sample();
    a.select(Some("saddle/main".into()));
    let (buffer, hits) = render_panel(160, &mut a, &[]);
    for (y, name) in &hits.agents {
        let selected = name == "saddle/main";
        assert_eq!(buffer[(2, *y)].symbol(), if selected { "┃" } else { "│" });
        for x in 3..50 {
            assert_eq!(
                buffer[(x, *y)].bg,
                if selected {
                    t::AGENT_SELECTED
                } else {
                    t::AGENTS_BG
                },
                "{name} at {x},{y}"
            );
        }
    }
    // The other saddle entry is unchanged by the move.
    assert!(
        hits.agents
            .iter()
            .any(|(_, n)| n == "saddle/dev-t20-workspace-1")
    );
}

#[test]
fn forty_two_columns_show_agent_icons_and_keep_columns_aligned() {
    let mut a = spec_sample();
    let (buffer, hits) = render_panel(120, &mut a, &["corral/main".into()]);
    let panes = Panes::new(buffer.area, &Config::default());
    assert_eq!(panes.agents.width - 2, 42);
    let headlines: Vec<_> = hits
        .agents
        .iter()
        .filter(|(y, _)| "○◐◓◑◒".contains(buffer[(4, *y)].symbol()))
        .map(|(y, name)| (*y, name.clone()))
        .collect();
    assert_eq!(headlines.len(), 4);
    let right = panes.agents.right() - 3;
    for (y, name) in &headlines {
        let line: String = (0..panes.agents.width)
            .map(|x| buffer[(x, *y)].symbol())
            .collect();
        // Agent column: the mark alone, two columns wide, at the same place on every row.
        let mark = buffer[(24, *y)].symbol().to_owned() + buffer[(25, *y)].symbol();
        assert!(mark == "✳ " || mark == ">_", "{name}: {line}");
        assert_eq!(buffer[(23, *y)].symbol(), " ", "{line}");
        assert!(
            !line.contains("claude") && !line.contains("codex"),
            "{line}"
        );
        // State column after it; times end at the right edge.
        let state = buffer[(27, *y)].symbol();
        assert!(state == "i" || state == "w", "{line}");
        assert_ne!(buffer[(right, *y)].symbol(), " ", "{line}");
        assert_eq!(buffer[(right + 1, *y)].symbol(), " ", "{line}");
    }
    let text = agents_lines(&buffer).join("\n");
    assert!(text.contains("dev-t20-workspac…"), "{text}");
    assert!(text.contains("…/t20-terminal-workspace"), "{text}");
}

#[test]
fn folding_keeps_only_the_first_row_of_unselected_agents() {
    let mut a = spec_sample();
    let expanded = render_panel(160, &mut a, &[]).1.agents.len();
    a.toggle_fold();
    let (buffer, hits) = render_panel(160, &mut a, &[]);
    let rows = |name: &str| hits.agents.iter().filter(|(_, n)| n == name).count();
    assert_eq!(rows("corral/main"), 5);
    for name in ["drover/main", "saddle/dev-t20-workspace-1", "saddle/main"] {
        assert_eq!(rows(name), 1, "{name}");
    }
    assert!(hits.agents.len() < expanded);
    assert!(agents_lines(&buffer).join("\n").contains("z Expand"));
    // An initial list above five defaults to folded; the selected one stays open.
    let mut initial_agents = spec_sample().agents;
    for i in 0..2 {
        initial_agents.push(Agent {
            name: format!("saddle/extra-{i}"),
            state: Some("idle".into()),
            ..Default::default()
        });
    }
    let mut many = agents::Panel::default();
    many.absorb(initial_agents, None, 100.0);
    let (_, hits) = render_panel(160, &mut many, &[]);
    assert_eq!(hits.agents.len(), 5 + 5);
}

#[test]
fn effort_keeps_its_first_row_slot_when_folded_and_narrow() {
    use saddle::theme as t;
    // The user's explicit choice: keep effort where it was, after the type, before the state.
    let label = |effort: &str| {
        serde_json::json!({ "effort": effort })
            .as_object()
            .unwrap()
            .clone()
    };
    let mut a = spec_sample();
    for agent in &mut a.agents {
        match agent.name.as_str() {
            "corral/main" => agent.labels = label("xhigh"),
            "saddle/dev-t20-workspace-1" => agent.labels = label("high"),
            _ => {}
        }
    }
    for i in 0..3 {
        a.agents.push(Agent {
            name: format!("saddle/extra-{i}"),
            kind: Some("codex".into()),
            state: Some("idle".into()),
            labels: label("medium"),
            ..Default::default()
        });
    }
    a.toggle_fold();
    assert!(a.folded());
    for (width, brand, name, cut) in [
        (160, 8, 16, "dev-t20-workspa…"),
        (120, 2, 14, "dev-t20-works…"),
    ] {
        let (buffer, hits) = render_panel(width, &mut a, &[]);
        let headlines: Vec<_> = hits
            .agents
            .iter()
            .filter(|(y, _)| buffer[(4, *y)].symbol() != " ")
            .map(|(y, n)| (*y, n.clone()))
            .collect();
        assert_eq!(headlines.len(), 7, "one first row each, folded or not");
        // Dot, name, type, effort, state: fixed columns with the name giving way.
        let effort_x = 6 + name + 1 + brand + 1;
        for (y, n) in &headlines {
            let line: String = (0..60).map(|x| buffer[(x, *y)].symbol()).collect();
            let icon: String = (effort_x..effort_x + 2)
                .map(|x| buffer[(x as u16, *y)].symbol())
                .collect();
            let expected = match n.as_str() {
                "corral/main" => "⣴⡇",
                "saddle/dev-t20-workspace-1" => "⣴⡀",
                n if n.starts_with("saddle/extra") => "⣄⡀",
                _ => "  ",
            };
            assert_eq!(icon, expected, "{width}: {line}");
            if expected == "⣴⡇" {
                assert_eq!(buffer[(effort_x as u16, *y)].fg, t::AGENT_STARTING);
            }
            assert_eq!(buffer[(effort_x as u16 + 2, *y)].symbol(), " ", "{line}");
            let state = buffer[(effort_x as u16 + 3, *y)].symbol();
            assert!(state == "i" || state == "w", "{line}");
        }
        let text = agents_lines(&buffer).join("\n");
        assert!(text.contains(&format!("{cut} ")), "{text}");
    }
    // Nothing known: the slot is not reserved.
    let mut plain = spec_sample();
    let (buffer, _) = render_panel(160, &mut plain, &[]);
    assert!(
        agents_lines(&buffer)
            .join("\n")
            .contains("dev-t20-workspace-1 >_ codex")
    );
}

#[test]
fn paths_show_in_full_when_they_fit_and_lose_leading_levels_only_when_too_wide() {
    let mut a = agents::Panel::default();
    let long = "/Users/example/Developer/personal_projs/saddle-worktrees/t20-terminal-workspace";
    a.absorb(
        vec![
            Agent {
                name: "demo/fits".into(),
                cwd: Some("/tmp/team/project".into()),
                ..Default::default()
            },
            Agent {
                name: "demo/long".into(),
                cwd: Some(long.into()),
                ..Default::default()
            },
            Agent {
                name: "demo/group".into(),
                cwd: Some("/tmp/team/demo".into()),
                ..Default::default()
            },
        ],
        None,
        100.0,
    );
    for (width, long_shown) in [
        (160, "…/saddle-worktrees/t20-terminal-workspace"),
        (120, "…/t20-terminal-workspace"),
    ] {
        let (buffer, _) = render_panel(width, &mut a, &[]);
        let lines = agents_lines(&buffer);
        let path = |start: &str| {
            lines
                .iter()
                .map(|l| l.replace('┃', "│"))
                .find(|l| l.contains(start))
                .unwrap_or_else(|| panic!("{start}: {}", lines.join("\n")))
        };
        assert!(path("/tmp/team/project").contains("│   /tmp/team/project "));
        assert!(path(long_shown).contains(&format!("│   {long_shown} ")));
        // The group's own directory keeps the design's parent-only form.
        assert!(path("…/team/").contains("│   …/team/ "));
    }
}

#[test]
fn attached_reads_in_text_color_but_offers_no_second_attach_click() {
    use saddle::theme as t;
    let mut a = spec_sample();
    // corral/main is selected and displayed here.
    let (buffer, hits) = render_panel(160, &mut a, &["corral/main".into()]);
    let (x, y) = find(&buffer, "[Attached]").unwrap();
    for i in 0..10 {
        assert_eq!(buffer[(x + i, y)].fg, t::AGENTS_TEXT);
    }
    let enter = |hits: &ui::Hits| {
        hits.buttons
            .iter()
            .filter(|h| h.key.code == crossterm::event::KeyCode::Enter)
            .count()
    };
    assert_eq!(enter(&hits), 0);
    assert!(hits.buttons.iter().all(|h| !h.area.contains((x, y).into())));
    // Not displayed: the usual Attach control, clickable.
    let (buffer, hits) = render_panel(160, &mut a, &[]);
    let (x, y) = find(&buffer, "↵ Attach").unwrap();
    assert_eq!(buffer[(x, y)].fg, t::AGENTS_ACCENT);
    assert_eq!(enter(&hits), 1);
}

#[test]
fn settings_entry_shares_title_row_and_wraps_below_status_when_narrow() {
    use crossterm::event::KeyCode;
    let (mut a, mut q) = fixture();
    let (buffer, hits) = render(160, 30, &mut a, &mut q, Focus::Agents);
    let lines = agents_lines(&buffer);
    assert!(
        lines[1].trim_matches('┃').trim_end().ends_with("Settings")
            && lines[2].contains("Attention · 0"),
        "{lines:#?}"
    );
    assert!(lines[3].contains('─'), "{lines:#?}");
    let entry = hits
        .buttons
        .iter()
        .find(|h| h.key.code == KeyCode::Char(','))
        .expect("Settings entry is clickable");
    assert_eq!((entry.area.y, entry.area.width), (1, 8));
    assert_eq!(entry.area.right(), 52 - 2);

    // A narrow column puts Settings on its own row; the list moves down one row.
    let (buffer, hits) = render(40, 30, &mut a, &mut q, Focus::Agents);
    let lines = agents_lines(&buffer);
    assert!(!lines[2].contains("Settings"), "{lines:#?}");
    assert!(lines[2].contains("Attention"), "{lines:#?}");
    assert!(
        lines[3].trim_matches('┃').trim_end().ends_with("Settings"),
        "{lines:#?}"
    );
    assert!(lines[4].contains('─'), "{lines:#?}");
    let entry = hits
        .buttons
        .iter()
        .find(|h| h.key.code == KeyCode::Char(','))
        .unwrap();
    assert_eq!(entry.area.y, 3);
    assert!(hits.agents.iter().all(|(row, _)| *row >= 5));
}

fn render_header_actions(
    left_width: u16,
    pointer: &Pointer,
    items: &[saddle::attention::Item],
) -> (Buffer, ui::Hits) {
    render_header(left_width, pointer, items, false)
}
fn render_header(
    left_width: u16,
    pointer: &Pointer,
    items: &[saddle::attention::Item],
    updates: bool,
) -> (Buffer, ui::Hits) {
    let (mut panel, _) = fixture();
    let terminals = saddle::terminals::Terminals::new("unused-fake-corral".into());
    let colors = saddle::theme::Theme::default();
    let config = Config {
        left_width,
        ..Default::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    let mut hits = ui::Hits::default();
    terminal
        .draw(|frame| {
            hits = ui::draw_workspace(
                frame,
                &mut panel,
                View {
                    colors: &colors,
                    panes: Panes::new(frame.area(), &config),
                    focus: Focus::Agents,
                    showing: None,
                    local: &[],
                    viewer: None,
                    projects: &[],
                    viewer_note: "",
                    reply: "",
                    now: 100.0,
                    pointer,
                },
                Some(ui::Workspace {
                    terminals: &terminals,
                    mascot: &mut Default::default(),
                    mascot_enabled: true,
                    placement: None,
                    search: None,
                    form: None,
                    program: "unused-fake-corral",
                    modal: false,
                    attention: ui::Attention {
                        items,
                        loading: false,
                        popup: None,
                    },
                    settings: None,
                    updates,
                }),
            );
        })
        .unwrap();
    (terminal.backend().buffer().clone(), hits)
}

#[test]
fn header_actions_align_with_title_and_attention_when_they_fit() {
    use crossterm::event::KeyCode;
    use saddle::theme;
    for (width, plugin_y, settings_y, telemetry_y) in [
        (52, 2, 2, 1),
        (36, 2, 2, 1),
        (35, 4, 4, 3),
        (30, 4, 4, 3),
        (20, 4, 5, 3),
    ] {
        let (buffer, hits) = render_header_actions(width, &Pointer::default(), &[]);
        let settings = hits
            .buttons
            .iter()
            .find(|h| h.key.code == KeyCode::Char(','))
            .unwrap()
            .area;
        let attention = hits
            .buttons
            .iter()
            .find(|h| h.key.code == KeyCode::Char('a'))
            .unwrap()
            .area;
        let telemetry = hits
            .buttons
            .iter()
            .find(|h| h.key.code == KeyCode::Char('t'))
            .expect("Telemetry entry is clickable")
            .area;
        assert_eq!(settings.y, settings_y, "width {width}: Settings row");
        assert_eq!(hits.plugins.y, plugin_y, "width {width}: Plugins row");
        assert_eq!(telemetry.y, telemetry_y, "width {width}: Telemetry row");
        assert!(!telemetry.intersects(settings) && !telemetry.intersects(hits.plugins));
        assert!(!telemetry.intersects(attention));
        assert!(hits.list.y > telemetry.y);
        assert_eq!(attention.y, 2, "Attention keeps its own second row");
        assert!(!settings.intersects(hits.plugins));
        assert!(!settings.intersects(attention));
        assert!(!hits.plugins.intersects(attention));
        assert!(hits.list.y > settings.y.max(attention.y));
        assert_eq!(settings.right(), width - 2);
        assert_eq!(telemetry.right(), settings.right());
        for (rect, label) in [
            (settings, "Settings"),
            (hits.plugins, "Plugins"),
            (telemetry, "Telemetry"),
        ] {
            let text: String = (rect.x..rect.right())
                .map(|x| buffer[(x, rect.y)].symbol())
                .collect();
            assert_eq!(text, label);
            assert_eq!(buffer[(rect.x, rect.y)].fg, theme::AGENTS_TEXT);
        }
    }
}

#[test]
fn header_actions_highlight_on_hover_and_attention_emphasizes_pending_items() {
    use crossterm::event::KeyCode;
    use saddle::{
        attention::{Item, Kind, Target},
        theme,
    };
    let (_, hits) = render_header_actions(52, &Pointer::default(), &[]);
    let settings = hits
        .buttons
        .iter()
        .find(|h| h.key.code == KeyCode::Char(','))
        .unwrap()
        .area;
    for rect in [hits.plugins, settings] {
        let mut pointer = Pointer::default();
        pointer.hover = Some((rect.x, rect.y).into());
        let (buffer, _) = render_header_actions(52, &pointer, &[]);
        assert_eq!(buffer[(rect.x, rect.y)].fg, theme::BRIGHT);
    }
    let item = Item {
        target: Target::Agent("demo/main".into()),
        kind: Kind::Waiting,
        label: "Demo".into(),
        note: String::new(),
    };
    let (buffer, _) = render_header_actions(52, &Pointer::default(), &[item]);
    let (x, y) = find(&buffer, "Attention").unwrap();
    assert_eq!(buffer[(x, y)].fg, theme::AGENTS_YELLOW);
    assert_eq!(buffer[(x + 12, y)].fg, theme::AGENTS_YELLOW);
    let (buffer, _) = render_header_actions(52, &Pointer::default(), &[]);
    assert_eq!(buffer[(x, y)].fg, theme::AGENTS_DIM);
}

#[test]
fn a_dot_beside_settings_shows_that_installed_updates_need_the_user() {
    use crossterm::event::KeyCode;
    for width in [52, 30, 20] {
        for (updates, dot) in [(false, " "), (true, "•")] {
            let (buffer, hits) = render_header(width, &Pointer::default(), &[], updates);
            let settings = hits
                .buttons
                .iter()
                .find(|h| h.key.code == KeyCode::Char(','))
                .unwrap()
                .area;
            let text: String = (settings.x..settings.right())
                .map(|x| buffer[(x, settings.y)].symbol())
                .collect();
            assert_eq!(text, "Settings", "width {width}");
            assert_eq!(
                buffer[(settings.right(), settings.y)].symbol(),
                dot,
                "width {width}"
            );
        }
    }
}
