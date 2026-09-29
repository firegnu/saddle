use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};
use saddle::{
    agents,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    drover::{Snapshot, Task},
    input::Focus,
    layout::Panes,
    queue,
    ui::{self, View},
};
fn fixture() -> (agents::Panel, queue::Panel) {
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
            ..Default::default()
        }],
        None,
        100.0,
    );
    let mut queue = queue::Panel::default();
    queue.project = "/tmp/demo".into();
    queue.absorb(Snapshot {
        pending: vec![Task {
            id: Some("T12345".into()),
            title: "中文任务很长需要截断但状态必须仍然可见".repeat(3),
            ..Default::default()
        }],
        ..Default::default()
    });
    (agents, queue)
}
fn render(
    w: u16,
    h: u16,
    a: &mut agents::Panel,
    q: &mut queue::Panel,
    focus: Focus,
) -> (Buffer, ui::Hits) {
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
                    queue: q,
                    viewer_note: "测试终端",
                    reply: "上一轮回复",
                    now: 100.0,
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
fn queue_reports_no_active_tasks_even_when_history_exists() {
    let (mut a, mut q) = fixture();
    for history in [
        vec![Task {
            title: "Past task".into(),
            status: Some("done".into()),
            ..Default::default()
        }],
        vec![],
    ] {
        q.absorb(Snapshot {
            history,
            ..Default::default()
        });
        let (buffer, _) = render(160, 60, &mut a, &mut q, Focus::Queue);
        assert!(
            text(&buffer).contains("No active tasks"),
            "{}",
            text(&buffer)
        );
    }
    q.read_error = Some("Synthetic read error".into());
    let (buffer, _) = render(160, 60, &mut a, &mut q, Focus::Queue);
    assert!(!text(&buffer).contains("No active tasks"));
    let (buffer, _) = render(160, 60, &mut a, &mut queue::Panel::default(), Focus::Queue);
    assert!(text(&buffer).contains("Loading tasks"));
    assert!(!text(&buffer).contains("No active tasks"));
}

#[test]
fn management_layouts_keep_cjk_status_and_input_target_visible() {
    for (w, h, outlined) in [
        (160, 48, true),
        (120, 36, true),
        (160, 24, false),
        (80, 48, false),
        (80, 24, false),
    ] {
        let (mut a, mut q) = fixture();
        let (buffer, hits) = render(w, h, &mut a, &mut q, Focus::Queue);
        let output = text(&buffer);
        assert!(output.contains("Input ▸ Tasks"), "{output}");
        assert!(
            output.contains("T12345") && output.contains("…") && output.contains("Pending"),
            "{output}"
        );
        assert!(!hits.queue_rows.is_empty());
        for label in ["‹Add task a›", "‹Help ?›", "‹Close Esc›"] {
            assert!(output.contains(label), "missing {label}: {output}");
        }
        for label in [
            "demo ▾ c",
            "Next n",
            "Check & release g",
            "Pause p",
            "Loop l",
            "Refresh r",
            "● Task text t",
            "○ Run details ↵",
        ] {
            let (x, y) = find(&buffer, label).expect(label);
            if outlined {
                assert_eq!(buffer[(x - 2, y - 1)].symbol(), "╭", "{label}: {output}");
                assert_eq!(buffer[(x - 2, y + 1)].symbol(), "╰", "{label}: {output}");
            } else {
                assert!(output.contains(&format!("‹{label}›")), "{label}: {output}");
            }
        }
        let (x, y) = find(&buffer, "Refresh r").unwrap();
        assert!(matches!(
            q.click(x - if outlined { 2 } else { 1 }, y - u16::from(outlined)),
            Some(saddle::drover::Request::Refresh)
        ));
        // The popup is one dialog surface over Agents and Viewer, which take no clicks.
        let panes = Panes::new(buffer.area, &Config::default());
        let overlay = saddle::theme::Theme::default().overlay;
        for y in panes.tasks.y..panes.tasks.bottom() {
            for x in panes.tasks.x..panes.tasks.right() {
                assert_eq!(buffer[(x, y)].bg, overlay, "background at {x},{y}");
            }
        }
        assert!(hits.agents.is_empty() && hits.terminal.is_empty());
    }
}
#[test]
fn overlays_remove_background_targets_and_small_frames_do_not_panic() {
    let (mut a, mut q) = fixture();
    q.page = queue::Page::Projects;
    let (_, hits) = render(120, 36, &mut a, &mut q, Focus::Queue);
    assert!(hits.agents.is_empty() && hits.queue_rows.is_empty());
    q.page = queue::Page::List;
    a.confirm = Some("demo/main".into());
    let (buffer, hits) = render(120, 36, &mut a, &mut q, Focus::Agents);
    assert!(text(&buffer).contains("abcdef123"));
    assert!(hits.agents.is_empty() && hits.queue_rows.is_empty());
    for w in 1..12 {
        for h in 1..10 {
            render(w, h, &mut a, &mut q, Focus::Agents);
            a.confirm = None;
            q.page = queue::Page::Add {
                title: "中文".into(),
                body: "正文".into(),
                body_focus: true,
            };
            render(w, h, &mut a, &mut q, Focus::Queue);
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
fn working_dot_advances_every_360ms() {
    let (mut a, mut q) = fixture();
    let frame = |a: &mut agents::Panel, q: &mut queue::Panel, now: f64| {
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
                        queue: q,
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

#[test]
fn queue_history_scrollbar_reaches_the_end_with_the_last_task_visible() {
    let (mut a, mut q) = fixture();
    q.absorb(Snapshot {
        history: (0..40)
            .map(|i| Task {
                id: Some(format!("T{i}")),
                title: format!("History item {i:02}"),
                status: Some("done".into()),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    });
    q.select(39);
    let (buffer, hits) = render(160, 40, &mut a, &mut q, Focus::Queue);
    assert!(
        text(&buffer).contains("History item 00"),
        "{}",
        text(&buffer)
    );
    let last = hits.queue_rows.last().unwrap().0;
    // The list's scrollbar thumb reaches its last row, left of the content divider.
    assert!(
        (0..60).any(|x| buffer[(x, last)].symbol() == "█"),
        "{}",
        text(&buffer)
    );
    q.select(0);
    let (_, hits) = render(160, 40, &mut a, &mut q, Focus::Queue);
    let before = q.top;
    let panes = Panes::new(buffer.area, &Config::default());
    q.wheel(panes.tasks.x + 10, hits.queue_rows[0].0, 3);
    render(160, 40, &mut a, &mut q, Focus::Queue);
    assert_eq!(q.selected, 0);
    assert_eq!(q.top, before + 3);
    q.absorb(q.snapshot.clone().unwrap());
    render(160, 40, &mut a, &mut q, Focus::Queue);
    assert_eq!(q.top, before + 3);
    q.wheel(0, 0, 10); // Outside the list.
    assert_eq!(q.top, before + 3);
    q.select(0);
    let (buffer, _) = render(160, 40, &mut a, &mut q, Focus::Queue);
    assert!(text(&buffer).contains("History item 39"));
}

fn render_queue(q: &mut queue::Panel, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| {
            q.draw(&saddle::theme::Theme::default(), f, f.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
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
fn queue_chrome_is_english_and_preserves_source_text() {
    let (_, mut q) = fixture();
    q.absorb(Snapshot {
        pending: vec![Task {
            title: "原始任务".into(),
            body: "原始正文".into(),
            reason: Some("原始原因".into()),
            ..Default::default()
        }],
        ..Default::default()
    });
    let check = |q: &mut queue::Panel| {
        let output = text(&render_queue(q, 80, 32));
        if q.view == queue::View::Details {
            for value in ["原始任务", "原始正文", "原始原因"] {
                assert!(output.contains(value), "{output}");
            }
        }
        let chrome = output
            .replace("原始任务", "")
            .replace("原始正文", "")
            .replace("原始原因", "")
            .replace("原始反馈", "");
        assert!(
            !chrome
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "{output}"
        );
    };
    for page in [
        queue::Page::List,
        queue::Page::Help,
        queue::Page::Projects,
        queue::Page::Project("/tmp/demo".into()),
        queue::Page::Add {
            title: "原始任务".into(),
            body: "原始正文".into(),
            body_focus: false,
        },
        queue::Page::Feedback("原始反馈".into()),
        queue::Page::AllPending,
    ] {
        q.page = page;
        check(&mut q);
    }
    q.page = queue::Page::List;
    q.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(q.page, queue::Page::List) && q.view == queue::View::Details);
    check(&mut q);
}

#[test]
fn queue_states_and_operation_results_have_semantic_colors() {
    use saddle::{drover::Operation, theme as t};
    let mut q = queue::Panel::default();
    for (snapshot, label, color) in [
        (Snapshot::default(), "Idle", t::AGENT_IDLE),
        (
            Snapshot {
                current: Some(Task::default()),
                ..Default::default()
            },
            "Running",
            t::AGENT_WORKING,
        ),
        (
            Snapshot {
                awaiting: Some(Task::default()),
                ..Default::default()
            },
            "Awaiting",
            t::AGENT_BLOCKED,
        ),
        (
            Snapshot {
                pending: vec![Task::default()],
                ..Default::default()
            },
            "Ready",
            t::AGENT_IDLE,
        ),
        (
            Snapshot {
                paused: true,
                ..Default::default()
            },
            "Paused",
            t::AGENT_BLOCKED,
        ),
    ] {
        q.absorb(snapshot);
        let buffer = render_queue(&mut q, 52, 32);
        assert_label_color(&buffer, label, color);
        assert_label_color(&buffer, "Loop off", t::DIM);
        assert!(text(&buffer).contains("Auto")); // Paused does not replace mode.
        assert!(text(&buffer).contains("Queue: "), "{label}");
    }
    q.snapshot.as_mut().unwrap().mode.r#loop = true;
    assert_label_color(&render_queue(&mut q, 52, 32), "Loop on", t::AGENT_IDLE);
    q.read_error = Some("Synthetic read error".into());
    assert_label_color(&render_queue(&mut q, 52, 32), "Read failed", t::AGENT_ERROR);
    q.absorb(Snapshot {
        history: ["done", "failed", "dropped", "unknown"]
            .into_iter()
            .map(|status| Task {
                status: Some(status.into()),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    });
    let buffer = render_queue(&mut q, 52, 32);
    for (label, color) in [
        ("Done", t::AGENT_IDLE),
        ("Failed", t::AGENT_ERROR),
        ("Dropped", t::AGENT_STALLED),
        ("unknown", t::MUTED),
    ] {
        assert_label_color(&buffer, label, color);
    }
    q.complete(&Operation::Next, Err(anyhow::anyhow!("Synthetic failure")));
    assert_label_color(
        &render_queue(&mut q, 80, 32),
        "Synthetic failure",
        t::AGENT_ERROR,
    );
    q.complete(&Operation::Next, Ok("Synthetic success".into()));
    assert_label_color(
        &render_queue(&mut q, 80, 32),
        "Synthetic success",
        t::AGENT_IDLE,
    );
}

#[test]
fn short_history_has_a_footer_at_the_bottom_of_its_available_area() {
    for (width, height) in [(80, 32), (64, 24), (52, 24)] {
        let mut q = queue::Panel::default();
        q.absorb(Snapshot {
            history: vec![Task {
                title: "Past task".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        let buffer = render_queue(&mut q, width, height);
        let output = text(&buffer);
        assert!(output.contains("No active tasks"));
        assert!(output.contains("Past task"));
        // Beside or above the content, the footer is the list's last row, right above a rule.
        let lines: Vec<_> = output.lines().collect();
        let footer = lines
            .iter()
            .position(|l| l.contains("History 1–1/1 · End"))
            .unwrap_or_else(|| panic!("{output}"));
        let below: String = lines[footer + 1]
            .chars()
            .filter(|c| !matches!(c, '┃' | ' '))
            .collect();
        assert!(
            !below.is_empty() && below.chars().all(|c| matches!(c, '─' | '┴')),
            "{output}"
        );
    }
}

#[test]
fn task_groups_and_row_statuses_have_distinct_colors() {
    use saddle::theme as t;
    let task = |title: &str, status: Option<&str>| Task {
        title: title.into(),
        status: status.map(Into::into),
        ..Default::default()
    };
    let mut q = queue::Panel::default();
    q.absorb(Snapshot {
        current: Some(task("task-current", None)),
        awaiting: Some(task("task-awaiting", None)),
        pending: vec![task("task-pending", None)],
        history: vec![
            task("task-done", Some("done")),
            task("task-failed", Some("failed")),
            task("task-dropped", Some("dropped")),
            task("task-unknown", Some("unknown")),
            task("task-missing", None),
        ],
        ..Default::default()
    });
    let buffer = render_queue(&mut q, 120, 40);
    for (label, color) in [
        ("Current 1", t::AGENT_WORKING),
        ("Awaiting 1", t::AGENT_BLOCKED),
        ("Pending 1", t::AGENT_STARTING),
        ("History 5", t::MUTED),
    ] {
        assert_label_color(&buffer, label, color);
    }
    for (title, label, color) in [
        ("task-current", "Running", t::AGENT_WORKING),
        ("task-awaiting", "Awaiting", t::AGENT_BLOCKED),
        ("task-pending", "Pending", t::AGENT_STARTING),
        ("task-done", "Done", t::AGENT_IDLE),
        ("task-failed", "Failed", t::AGENT_ERROR),
        ("task-dropped", "Dropped", t::AGENT_STALLED),
        ("task-unknown", "unknown", t::MUTED),
        ("task-missing", "—", t::DIM),
    ] {
        let rows = text(&buffer);
        let y = rows
            .lines()
            .position(|line| line.contains(title))
            .unwrap_or_else(|| panic!("missing {title}: {rows}"));
        let cells: Vec<_> = (0..buffer.area.width)
            .map(|x| buffer[(x, y as u16)].symbol().to_string())
            .collect();
        let label: Vec<_> = label.chars().map(String::from).collect();
        let x = cells
            .windows(label.len())
            .rposition(|w| w == label.as_slice())
            .unwrap_or_else(|| panic!("missing {label:?} in {title}: {rows}"));
        for i in 0..label.len() {
            assert_eq!(buffer[((x + i) as u16, y as u16)].fg, color, "{title}");
        }
    }
}

#[test]
fn all_pending_overlay_names_projects_reports_each_state_and_scrolls_to_the_last_task() {
    let mut q = queue::Panel::default();
    q.absorb(Snapshot::default());
    let task = |id: &str, title: &str| Task {
        id: Some(id.into()),
        title: title.into(),
        body: "正文不在汇总里".into(),
        ..Default::default()
    };
    let long = "很长的原文标题需要完整折行显示".repeat(8);
    q.all_pending = vec![
        (
            "/work/alpha".into(),
            Some(Ok(vec![task("T1", "原文第一件"), task("T2", &long)])),
        ),
        ("/work/beta".into(), None),
        (
            "/work/gamma".into(),
            Some(Err("synthetic read failure".into())),
        ),
        ("/work/delta".into(), Some(Ok(Vec::new()))),
    ];
    q.page = queue::Page::AllPending;
    let buffer = render_queue(&mut q, 80, 40);
    let output = text(&buffer);
    for value in [
        " All pending ",
        "alpha",
        "/work/alpha",
        "1 T1 原文第一件",
        "beta",
        "Loading…",
        "gamma",
        "Read failed",
        "synthetic read failure",
        "delta",
        "No pending",
        "Refresh r",
        "Back Esc",
    ] {
        assert!(output.contains(value), "missing {value}:\n{output}");
    }
    // Only the popup's interior: wrapped rows joined back must equal the source title.
    let joined: String = output
        .lines()
        .filter_map(|l| {
            let (start, end) = (l.find('┃')?, l.rfind('┃')?);
            (start < end).then(|| l[start + '┃'.len_utf8()..end].trim().to_string())
        })
        .collect();
    assert!(
        joined.contains(&long),
        "long titles wrap without clipping: {joined}"
    );
    assert!(!output.contains("正文不在汇总里"));
    assert_label_color(&buffer, "Read failed", saddle::theme::AGENT_ERROR);

    q.all_pending = vec![(
        "/work/many".into(),
        Some(Ok((1..=60)
            .map(|i| task(&format!("T{i}"), &format!("Task number {i}")))
            .collect())),
    )];
    let first = text(&render_queue(&mut q, 80, 32));
    assert!(first.contains("Task number 1 ") && !first.contains("Task number 60"));
    for _ in 0..30 {
        q.key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageDown,
            crossterm::event::KeyModifiers::NONE,
        ));
    }
    let last = text(&render_queue(&mut q, 80, 32));
    assert!(last.contains("Task number 60"), "{last}");
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
    let icon = |a: &mut agents::Panel, q: &mut queue::Panel, focus| {
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

fn show_json() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/show.json")).unwrap()
}
fn detail_queue(list_task: serde_json::Value) -> queue::Panel {
    let mut q = queue::Panel::default();
    q.project = "/tmp/demo".into();
    let location = list_task["at"].as_str().unwrap_or("current").to_owned();
    let mut state = serde_json::json!({"mode":{}, "paused":false, "current":null, "awaiting":null, "pending":[], "history":[]});
    if location == "history" {
        state["history"] = serde_json::json!([list_task]);
    } else {
        state[location] = list_task;
    }
    q.absorb(serde_json::from_value(state).unwrap());
    q
}
fn open_detail(q: &mut queue::Panel, value: Option<serde_json::Value>) -> saddle::queue::DetailKey {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    q.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let key = q.detail_key().expect("numbered task must be queried");
    if let Some(value) = value {
        q.absorb_detail(&key, Ok(serde_json::from_value(value).unwrap()));
    }
    key
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
fn run_details_sit_beside_the_list_inside_the_popup_only() {
    let (mut a, _) = fixture();
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    open_detail(&mut q, Some(show_json()));
    let (buffer, hits) = render(160, 48, &mut a, &mut q, Focus::Queue);
    let panes = Panes::new(buffer.area, &Config::default());
    assert!(
        !hits.queue_rows.is_empty(),
        "the list stays beside the details"
    );
    let all = text(&buffer);
    for label in [
        "Completion checks",
        "Run details ↵",
        "Task text t",
        "Close Esc",
    ] {
        let (x, y) = find(&buffer, label).unwrap_or_else(|| panic!("{label}: {all}"));
        assert!(panes.tasks.contains((x, y).into()), "{label}");
    }
    let outside: String = (0..buffer.area.height)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .filter(|(x, y)| !panes.tasks.contains((*x, *y).into()))
        .map(|(x, y)| buffer[(x, y)].symbol().to_owned())
        .collect();
    // Nothing of the details spills over Agents or Viewer outside the popup.
    for detail_text in ["没有收尾提交", "Completion", "Close Esc"] {
        assert!(!outside.contains(detail_text), "{detail_text}: {all}");
    }
    assert!(all.contains("Input ▸ Tasks · Run details"), "{all}");
}

#[test]
fn task_tabs_mark_the_chosen_view_and_details_color_structured_states() {
    use ratatui::{layout::Position, style::Modifier};
    use saddle::theme::{self, Theme};
    let style = |buffer: &Buffer, label: &str, skip: u16, len: u16| {
        let (x, y) = find(buffer, label).unwrap_or_else(|| panic!("{label}: {}", text(buffer)));
        let cells: Vec<_> = (x + skip..x + skip + len)
            .map(|x| &buffer[(x, y)])
            .collect();
        let fg = cells[0].fg;
        assert!(cells.iter().all(|c| c.fg == fg), "{label}: mixed colours");
        (fg, cells[0].modifier.contains(Modifier::BOLD))
    };
    let heading = Theme::default().reply_heading;
    let (mut a, _) = fixture();
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    // Outlined in the large popup, compact in the small one; both flip with the view.
    for (w, h) in [(160, 48), (80, 24)] {
        q.view = queue::View::Text;
        let (buffer, _) = render(w, h, &mut a, &mut q, Focus::Queue);
        assert_eq!(style(&buffer, "● Task text t", 0, 11), (theme::FOCUS, true));
        assert_eq!(style(&buffer, "○ Run details ↵", 0, 13).0, theme::MUTED);
        assert!(!style(&buffer, "○ Run details ↵", 0, 13).1);
        assert!(find(&buffer, "○ Task text t").is_none());
    }
    open_detail(&mut q, Some(show_json()));
    for (w, h) in [(160, 48), (80, 24)] {
        let (buffer, _) = render(w, h, &mut a, &mut q, Focus::Queue);
        assert_eq!(
            style(&buffer, "● Run details ↵", 0, 13),
            (theme::FOCUS, true)
        );
        assert_eq!(style(&buffer, "○ Task text t", 0, 11).0, theme::MUTED);
        assert!(find(&buffer, "● Task text t").is_none());
    }
    // Hovering either tab brightens it, yet the chosen one keeps its mark and weight.
    let colors = Theme::default();
    for label in ["● Run details ↵", "○ Task text t"] {
        let (buffer, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
        let (x, y) = find(&buffer, label).unwrap();
        let mut pointer = Pointer::default();
        pointer.hover = Some(Position::new(x, y));
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        terminal
            .draw(|frame| {
                ui::draw(
                    frame,
                    &mut a,
                    View {
                        colors: &colors,
                        panes: Panes::new(frame.area(), &Config::default()),
                        focus: Focus::Queue,
                        showing: Some("demo/main"),
                        local: &["demo/main".to_string()],
                        viewer: None,
                        queue: &mut q,
                        viewer_note: "",
                        reply: "",
                        now: 100.0,
                        pointer: &pointer,
                    },
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let (fg, bold) = style(buffer, "● Run details ↵", 0, 13);
        assert!(bold, "{label}");
        assert_eq!(
            fg,
            if label.starts_with('●') {
                theme::BRIGHT
            } else {
                theme::FOCUS
            }
        );
        let (fg, bold) = style(buffer, "○ Task text t", 0, 11);
        assert!(!bold, "{label}");
        assert_eq!(
            fg,
            if label.starts_with('○') {
                theme::BRIGHT
            } else {
                theme::MUTED
            }
        );
    }

    // Headings take reply_heading; states their semantic colour; everything else stays neutral.
    let buffer = render_queue(&mut q, 106, 120);
    let color = |label: &str, skip: u16, len: u16| style(&buffer, label, skip, len).0;
    for label in [
        "Completion checks",
        "Last check",
        "Progress",
        "Route, hold",
        "Records",
    ] {
        assert_eq!(style(&buffer, label, 0, 5), (heading, true), "{label}");
    }
    assert_eq!(color("Running", 0, 7), theme::AGENT_WORKING);
    assert_eq!(color("Suggested · idle", 0, 9), theme::AGENT_BLOCKED);
    assert_eq!(color("Suggested · idle", 9, 7), theme::TEXT);
    assert_eq!(color("completion marker", 0, 17), theme::AGENT_BLOCKED);
    assert_eq!(color("saddle/main · idle", 0, 14), theme::TEXT);
    assert_eq!(color("saddle/main · idle", 14, 4), theme::AGENT_IDLE);
    assert_eq!(color("idle · via send", 4, 11), theme::TEXT);
    assert_eq!(color("Unknown · task body", 0, 7), theme::MUTED);
    assert_eq!(color("Hold", 0, 4), theme::MUTED);

    let mut value = show_json();
    value["hold"] =
        serde_json::json!({"enabled":true,"scope":"current_events","unavailable_reason":null});
    value["attention"]["agent"]["state"] = "working".into();
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    open_detail(&mut q, Some(value));
    let buffer = render_queue(&mut q, 106, 120);
    let color = |label: &str, skip: u16, len: u16| style(&buffer, label, skip, len).0;
    assert_eq!(color("On · stop", 0, 2), theme::AGENT_BLOCKED);
    assert_eq!(color("On · stop", 2, 7), theme::TEXT);
    assert_eq!(color("saddle/main · working", 14, 7), theme::AGENT_WORKING);
    assert_ne!(heading, theme::TEXT, "headings stand apart from body text");
}

#[test]
fn content_views_and_edit_keep_reading_positions() {
    use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers};
    let (mut a, mut q) = fixture();
    q.snapshot.as_mut().unwrap().pending[0].body = (0..90)
        .map(|i| format!("Original body line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    q.select(0);
    let (first, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    assert!(text(&first).contains("Original body line 0"));
    q.key(KeyEvent::new(K::PageDown, KeyModifiers::NONE));
    let (before, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    assert!(!text(&before).contains("Original body line 0 "));
    let (x, y) = find(&before, "Run details ↵").expect("fixed view switch");
    q.click(x, y);
    let (details, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    assert_eq!(q.view, queue::View::Details);
    assert!(find(&details, "T12345").is_some());
    let (x, y) = find(&details, "Task text t").unwrap();
    q.click(x, y);
    assert_eq!(
        text(&render(160, 48, &mut a, &mut q, Focus::Queue).0),
        text(&before)
    );
    let (x, y) = find(&before, "Edit e").expect("pending Edit button");
    q.click(x, y);
    let (editor, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    assert!(text(&editor).contains("Edit task"));
    let (x, y) = find(&editor, "Cancel Esc").unwrap();
    q.click(x, y);
    assert_eq!(
        text(&render(160, 48, &mut a, &mut q, Focus::Queue).0),
        text(&before)
    );

    // Once the task starts, the editing entry goes away in both layouts.
    let mut fresh = q.snapshot.clone().unwrap();
    fresh.current = Some(fresh.pending.remove(0));
    q.absorb(fresh);
    for (w, h) in [(160, 48), (60, 24)] {
        let (buffer, _) = render(w, h, &mut a, &mut q, Focus::Queue);
        assert!(find(&buffer, "Task text t").is_some());
        assert!(find(&buffer, "Edit e").is_none());
    }
}

#[test]
fn task_details_present_each_contract_section_without_inventing_values() {
    // Current: recomputed checks, unknown Git values, warnings and hostile text.
    let mut value = show_json();
    value["git"]["range_commits"] = serde_json::Value::Null;
    value["git"]["unavailable_reasons"]["range_commits"] = "git_query_failed".into();
    value["warnings"] =
        serde_json::json!([{"code":"snapshot_changed","sources":["tasks.state","git_refs"]}]);
    value["task"]["body"] = "safe \u{1b}[31mred\u{7} text".into();
    value["completion"]["rows"][0]["why"] = "没有收尾提交 \u{1b}]0;title\u{7}".into();
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    open_detail(&mut q, Some(value));
    let out = text(&render_queue(&mut q, 106, 120));
    for expected in [
        "T4",
        "Running",
        "41m",
        "Detail target 任务",
        "Completion checks",
        "recomputed now",
        "✗ Completion marker",
        "✓ Main advanced",
        "? Branches merged",
        "· Check command",
        "没有收尾提交",
        "Git query failed",
        "not only this task",
        "Last check",
        "Missing",
        "not mean it never ran",
        "重",
        "cross review",
        "current task file",
        "task body not recorded",
        "Suggested",
        "inferred",
        "saddle/main",
        "changed during read",
        "tasks.state",
        "safe [31mred text",
    ] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
    assert!(!out.contains('\u{1b}') && !out.contains('\u{7}'));
    assert!(
        !out.contains("Commits      0") && !out.contains(" 0 commits"),
        "{out}"
    );
    let narrow = text(&render_queue(&mut q, 34, 200));
    assert!(narrow.contains("Completion checks"), "{narrow}");

    // Awaiting: the recorded done stands apart from today's recomputation.
    let mut value = show_json();
    value["task"]["location"] = "awaiting".into();
    value["task"]["status"] = "done".into();
    value["timing"]["ended_at"] = 1790363000.into();
    value["timing"]["elapsed_seconds"] = 1598.into();
    value["timing"]["release_wait_seconds"] = 20.into();
    value["attention"] = serde_json::json!({"state":"awaiting_release","reason":"awaiting_release","unmet_rows":[],"agent":null,"inference":false});
    let mut q =
        detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务", "at":"awaiting"}));
    open_detail(&mut q, Some(value));
    let out = text(&render_queue(&mut q, 106, 120));
    for expected in ["Awaiting release", "recorded done stands", "20s so far"] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }

    // History: nothing current is applied; the cache is a retained sample.
    let mut value = show_json();
    value["task"]["location"] = "history".into();
    value["task"]["status"] = "done".into();
    value["timing"]["ended_at"] = 1790363000.into();
    value["timing"]["released_at"] = 1790363030.into();
    value["timing"]["release_wait_seconds"] = 30.into();
    value["git"]["main_commits_since_start"] = serde_json::Value::Null;
    value["git"]["unavailable_reasons"] =
        serde_json::json!({"main_commits_since_start":"completion_main_not_recorded"});
    value["completion"] = serde_json::json!({"scope":"recorded_history","rows":null,"unavailable_reason":"completion_snapshot_not_recorded"});
    value["last_check"] = serde_json::json!({"status":"stale","applicable":true,"scope":"retained_sample","checked_at":null,"ok":null,
        "record":{"task":"T4","main":"abc","cmd":"cargo test","why":"2 failed","ok":false,"t":1790362900},
        "stale_reasons":["main_changed"],"unavailable_reason":null});
    value["hold"] =
        serde_json::json!({"enabled":true,"scope":"task_end_events","unavailable_reason":null});
    value["attention"] = serde_json::json!({"state":"not_applicable","reason":"historical_task","unmet_rows":[],"agent":null,"inference":false});
    let mut q = detail_queue(
        serde_json::json!({"id":"T4", "title":"Detail target 任务", "status":"done", "at":"history"}),
    );
    open_detail(&mut q, Some(value));
    let out = text(&render_queue(&mut q, 106, 120));
    for expected in [
        "Done",
        "completion snapshot not recorded",
        "not applied",
        "main at completion not recorded",
        "Stale",
        "main changed",
        "Retained sample",
        "cargo test",
        "failed",
        "2 failed",
        "Released",
        "Hold",
        "On",
    ] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
    for absent in ["recomputed now", "Completion marker", "Suggested"] {
        assert!(!out.contains(absent), "unexpected {absent}: {out}");
    }
}

#[test]
fn detail_loading_and_failures_never_fake_data_and_refreshes_keep_the_scroll() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    let key = open_detail(&mut q, None);
    let out = text(&render_queue(&mut q, 100, 40));
    assert!(
        out.contains("Loading details") && out.contains("Detail target 任务"),
        "{out}"
    );
    assert!(
        !out.contains("Completion checks") && !out.contains("Elapsed"),
        "{out}"
    );
    q.absorb_detail(
        &key,
        Err(anyhow::anyhow!("drover show: task_not_found: 没有该任务")),
    );
    let out = text(&render_queue(&mut q, 100, 40));
    assert!(
        out.contains("task_not_found") && out.contains("Retrying"),
        "{out}"
    );
    assert!(!out.contains("Completion checks"), "{out}");

    let mut value = show_json();
    value["task"]["body"] = (0..80)
        .map(|i| format!("body line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
        .into();
    let detail: saddle::drover::Detail = serde_json::from_value(value).unwrap();
    q.absorb_detail(&key, Ok(detail.clone()));
    let top = text(&render_queue(&mut q, 100, 40));
    for _ in 0..3 {
        q.key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    }
    let scrolled = text(&render_queue(&mut q, 100, 40));
    assert_ne!(scrolled, top);
    q.absorb_detail(&key, Ok(detail));
    assert_eq!(text(&render_queue(&mut q, 100, 40)), scrolled);
    let (x, y) = find(&render_queue(&mut q, 100, 40), "body line").unwrap();
    q.wheel(x, y, -1);
    assert_ne!(text(&render_queue(&mut q, 100, 40)), scrolled);

    q.absorb_detail(&key, Err(anyhow::anyhow!("show cancelled or timed out")));
    let out = text(&render_queue(&mut q, 100, 200));
    for expected in ["timed out", "stale", "Completion checks", "Retrying"] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
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
    let (mut panel, mut queue) = fixture();
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
                        queue: &mut queue,
                        viewer_note: "",
                        reply: "",
                        now: 100.0,
                        pointer,
                    },
                    Some(ui::Workspace {
                        terminals: &terminals,
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
fn agents_own_the_left_column_and_tasks_open_as_a_large_popup() {
    let (mut a, mut q) = fixture();
    let (buffer, _) = render(160, 48, &mut a, &mut q, Focus::Agents);
    let screen = text(&buffer);
    assert!(screen.contains("Tasks · "), "{screen}");
    assert!(!screen.contains("T12345"), "{screen}");
    // The Agents border runs down to the status row.
    assert_eq!(buffer[(0, 46)].symbol(), "┗");
    let (buffer, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    let screen = text(&buffer);
    let (x, y) = find(&buffer, "┏ Tasks ").expect(&screen);
    assert!(
        (11..=13).contains(&x) && y == 4,
        "popup opens centered: {x},{y}"
    );
    // The list row and the selected task's text are both on screen, side by side.
    let (_, tabs_y) = find(&buffer, "Task text").unwrap();
    let row = screen.lines().nth(usize::from(tabs_y - 1)).unwrap();
    assert!(
        row.contains("Pending 1"),
        "list and content share rows: {row}"
    );
    assert_eq!(screen.matches("T12345").count(), 2, "{screen}");
    for label in ["Check & release g", "Refresh r", "Add task a", "Close Esc"] {
        assert!(screen.contains(label), "{label}: {screen}");
    }
}

#[test]
fn closed_tasks_entry_keeps_the_projects_short_status_in_semantic_colors() {
    use saddle::theme as t;
    let (mut a, _) = fixture();
    let task = || Task {
        id: Some("T1".into()),
        title: "Task".into(),
        ..Default::default()
    };
    // The entry ends the Agents header row, inside the border and its padding.
    let entry = |a: &mut agents::Panel, q: &mut queue::Panel, width| {
        let (buffer, _) = render(width, 40, a, q, Focus::Agents);
        let panes = Panes::new(buffer.area, &Config::default());
        let top: String = (0..panes.agents.right())
            .map(|x| buffer[(x, 1)].symbol().to_owned())
            .collect();
        assert!(top.ends_with(" ┃"), "{top}");
        (buffer, top)
    };
    let mut q = queue::Panel::default();
    let (buffer, top) = entry(&mut a, &mut q, 160);
    assert!(top.contains("Tasks · Loading… Tab ┃"), "{top}");
    assert_label_color(&buffer, "Tasks · ", t::AGENTS_DIM);
    for (snapshot, label, color) in [
        (
            Snapshot {
                awaiting: Some(task()),
                current: Some(task()),
                pending: vec![task()],
                ..Default::default()
            },
            "Awaiting release",
            t::AGENT_BLOCKED,
        ),
        (
            Snapshot {
                current: Some(task()),
                pending: vec![task()],
                ..Default::default()
            },
            "Running",
            t::AGENT_WORKING,
        ),
        (
            Snapshot {
                paused: true,
                pending: vec![task()],
                ..Default::default()
            },
            "Paused",
            t::AGENT_BLOCKED,
        ),
        (
            Snapshot {
                pending: vec![task(), task(), task()],
                ..Default::default()
            },
            "3 pending",
            t::AGENT_STARTING,
        ),
        (Snapshot::default(), "Idle", t::AGENT_IDLE),
    ] {
        q.absorb(snapshot);
        let (buffer, top) = entry(&mut a, &mut q, 160);
        assert!(top.contains(&format!("Tasks · {label} Tab ┃")), "{top}");
        assert_label_color(&buffer, label, color);
    }
    // A failed read is shown as such, never as the last good state.
    q.read_error = Some("synthetic".into());
    let (buffer, top) = entry(&mut a, &mut q, 160);
    assert!(top.contains("Tasks · Read failed Tab ┃"), "{top}");
    assert_label_color(&buffer, "Read failed", t::AGENT_ERROR);
    // Narrow Agents columns keep the status and drop the key hint first.
    q.read_error = None;
    q.absorb(Snapshot {
        awaiting: Some(task()),
        ..Default::default()
    });
    let (_, top) = entry(&mut a, &mut q, 80);
    assert!(top.contains("Tasks · Awaiting ┃"), "{top}");
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
    let mut q = queue::Panel::default();
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
                    queue: &mut q,
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
        " Agents · 4                  Tasks · Loading… Tab ",
        " Attention · 0                           Settings ",
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
        " │ ◓ dev-t20-workspace-1 >_ codex working      0s ",
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
    assert_label_color(&buffer, "ATT 0", t::AGENTS_DIMMER);
    assert_label_color(&buffer, "↑3", t::AGENTS_YELLOW);
    // Group heading: accent name, faint line, dim count.
    assert_eq!(buffer[(2, 4)].fg, t::AGENTS_ACCENT);
    assert_eq!(buffer[(10, 4)].fg, t::AGENTS_FAINT);
    assert_eq!(buffer[(47, 4)].fg, t::AGENTS_DIM);
    assert_label_color(&buffer, "0s", t::AGENTS_BLUE);
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
    // More than five agents fold on their own; the selected one stays open.
    let mut many = spec_sample();
    for i in 0..2 {
        many.agents.push(Agent {
            name: format!("saddle/extra-{i}"),
            state: Some("idle".into()),
            ..Default::default()
        });
    }
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
fn settings_entry_sits_right_of_attention_and_wraps_below_it_when_narrow() {
    use crossterm::event::KeyCode;
    let (mut a, mut q) = fixture();
    let (buffer, hits) = render(160, 30, &mut a, &mut q, Focus::Agents);
    let lines = agents_lines(&buffer);
    assert!(
        lines[2].trim_matches('┃').trim_end().ends_with("Settings")
            && lines[2].contains("Attention · 0"),
        "{lines:#?}"
    );
    assert!(lines[3].contains('─'), "{lines:#?}");
    let entry = hits
        .buttons
        .iter()
        .find(|h| h.key.code == KeyCode::Char(','))
        .expect("Settings entry is clickable");
    assert_eq!((entry.area.y, entry.area.width), (2, 8));
    assert_eq!(entry.area.right(), 52 - 2);

    // A narrow column puts Settings on its own row; the list moves down one row.
    let (buffer, hits) = render(80, 30, &mut a, &mut q, Focus::Agents);
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

#[test]
fn manual_completion_is_a_running_task_button_and_its_page_shows_checks_and_reason() {
    let mut q = queue::Panel::default();
    q.project = "/tmp/project-a".into();
    q.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "awaiting": null,
            "current": {"id":"T4", "title":"Research"},
            "pending": [{"id":"T5", "title":"Next"}], "history": []
        }))
        .unwrap(),
    );
    let screen = text(&render_queue(&mut q, 150, 40));
    assert!(screen.contains("Mark complete manually…"), "{screen}");
    q.select(1);
    assert!(!text(&render_queue(&mut q, 150, 40)).contains("Mark complete manually"));
    q.select(0);
    let buffer = render_queue(&mut q, 150, 40);
    // Clicking the button opens the page; the click needs no shortcut key.
    let (x, y) = (0..buffer.area.height)
        .find_map(|y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect();
            row.find("Mark complete manually")
                .map(|at| (row[..at].chars().count() as u16, y))
        })
        .expect("the button is drawn");
    assert!(q.click(x, y).is_none());
    let key = q.manual_key().expect("the page reads its target");
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    value["manual_completion"] =
        serde_json::json!({"target_token": "tok", "unavailable_reason": null});
    q.absorb_manual(&key, Ok(serde_json::from_value(value).unwrap()));
    q.paste("keep the research branch");
    let screen = text(&render_queue(&mut q, 150, 40));
    for words in [
        "Mark complete manually",
        "project-a",
        "T4 · Research",
        "as reported now, not run",
        "Completion marker",
        "saved sample, not run now",
        "keep the research branch",
        "Mark complete ↵",
        "Cancel Esc",
        "send go or next",
    ] {
        assert!(screen.contains(words), "{words}\n{screen}");
    }
    // Small windows still draw without panicking.
    render_queue(&mut q, 40, 12);
}

#[test]
fn queue_task_text_shows_selected_task_status_apart_from_queue_status() {
    use saddle::theme as t;
    let mut q = queue::Panel::default();
    q.absorb(Snapshot {
        current: Some(Task {
            id: Some("T1".into()),
            title: "Now".into(),
            ..Default::default()
        }),
        pending: vec![Task {
            id: Some("T2".into()),
            title: "Later".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let buffer = render_queue(&mut q, 100, 32);
    assert!(text(&buffer).contains("Queue: Running"));
    assert_status_label(&buffer, "Running T1 Now", 7, t::AGENT_WORKING);
    q.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::NONE,
    ));
    let buffer = render_queue(&mut q, 100, 32);
    assert!(text(&buffer).contains("Queue: Running"));
    assert_status_label(&buffer, "Pending T2 Later", 7, t::AGENT_STARTING);
}

/// The row that starts with `row` has its first `len` cells in `color`.
fn assert_status_label(buffer: &Buffer, row: &str, len: u16, color: ratatui::style::Color) {
    let n = row.chars().count() as u16;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width.saturating_sub(n) {
            if row
                .chars()
                .enumerate()
                .all(|(i, c)| buffer[(x + i as u16, y)].symbol() == c.to_string())
            {
                for i in 0..len {
                    assert_eq!(buffer[(x + i, y)].fg, color, "{row}");
                }
                return;
            }
        }
    }
    panic!("missing {row}: {}", text(buffer));
}
