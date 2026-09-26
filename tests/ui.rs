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
    let panes = Panes::with_queue(
        Rect::new(0, 0, w, h),
        &Config::default(),
        focus == Focus::Queue,
    );
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
    for (w, h) in [(160, 48), (120, 36), (80, 24)] {
        let (mut a, mut q) = fixture();
        let (buffer, hits) = render(w, h, &mut a, &mut q, Focus::Queue);
        let output = text(&buffer);
        assert!(output.contains("Input ▸ Queue"), "{output}");
        assert!(
            output.contains("T12345") && output.contains("…") && output.contains("Pending"),
            "{output}"
        );
        assert!(!hits.queue_rows.is_empty());
        for label in [
            "‹Refresh r›",
            "‹Project c›",
            "‹Details ↵›",
            "‹Add a›",
            "‹Help ?›",
        ] {
            assert!(output.contains(label), "missing {label}: {output}");
        }
        assert!(!output.contains('╭'), "{output}");
        let row = hits.queue_rows[0].0;
        let panes = Panes::with_queue(buffer.area, &Config::default(), true);
        assert_eq!(
            buffer[(panes.queue.right() - 4, row)].bg,
            ratatui::style::Color::Reset
        );
        for pane in [panes.queue, panes.tabs] {
            for y in pane.y..pane.bottom() {
                for x in pane.x..pane.right() {
                    assert_eq!(
                        buffer[(x, y)].bg,
                        ratatui::style::Color::Reset,
                        "background at {x},{y}"
                    );
                }
            }
        }
        if w < 100 {
            assert!(hits.agents.is_empty());
        }
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
                ["FIRST-TITLE", "abcdef", "/tmp/demo", "VIA human"],
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
fn repo_tree_keeps_siblings_connected_and_highlights_only_the_selected_agent() {
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
        a.by_state = by_state;
        let (buffer, hits) = render(160, 100, &mut a, &mut q, Focus::Agents);
        let output = text(&buffer);
        assert!(output.contains("…/work/demo"), "{output}");
        for label in ["SOURCE ", "DIR ", "TITLE ", "/Users/example"] {
            assert!(!output.contains(label), "{label}: {output}");
        }
        assert!(output.contains("Review changes"));
        let headline = |name: &str| hits.agents.iter().find(|(_, n)| n == name).unwrap().0;
        let first = if by_state { "demo/review" } else { "demo/main" };
        let last = if by_state { "demo/main" } else { "demo/review" };
        assert_eq!(buffer[(2, headline(first))].symbol(), "├");
        assert_eq!(buffer[(2, headline(last))].symbol(), "└");
        assert_eq!(buffer[(2, headline(first) + 1)].symbol(), "│");
        assert_eq!(buffer[(2, headline("other/main"))].symbol(), "└");
        for (y, name) in &hits.agents {
            assert_eq!(
                buffer[(45, *y)].bg,
                if name == "demo/main" {
                    theme::AGENT_SELECTED
                } else {
                    ratatui::style::Color::Reset
                }
            );
        }
        assert!(output.contains("‹Attach") || output.contains("‹Attached"));
    }
    a.follow = false;
    a.top = 2;
    a.by_state = false;
    let (buffer, _) = render(80, 20, &mut a, &mut q, Focus::Agents);
    let output = text(&buffer);
    assert!(
        output.contains("Agents · 3") && output.contains("demo/ · ↑"),
        "{output}"
    );
    assert!(output.contains("…/work/demo"), "{output}");
    for button in ["‹Attached›", "‹Open o›", "‹New n›", "‹Sort s›", "‹Stop x›"]
    {
        assert!(output.contains(button), "{output}");
    }
    assert!(!output.contains("Reply r"));
}

#[test]
fn agent_type_marks_and_names_share_brand_color_without_changing_selection_or_state() {
    use ratatui::style::Color;
    use saddle::theme;
    use unicode_width::UnicodeWidthStr;
    for (kind, label, color) in [
        ("claude", "✳ claude", Color::Rgb(0xd9, 0x77, 0x57)),
        ("codex", ">_ codex", Color::Rgb(0x8e, 0xd9, 0xc1)),
        ("pi", "π pi", Color::Rgb(0xff, 0xff, 0xff)),
        ("omp", "π omp", Color::Rgb(0xa8, 0x55, 0xf7)),
        ("custom", "custom", theme::MUTED),
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
                        Color::Reset
                    }
                );
            }
            assert!(line.contains("working"), "{line}");
            assert!(line.contains("◉ 1s"), "{line}");
            let state = 1 + line[..line.find("working").unwrap()].width() as u16;
            assert_eq!(buffer[(state, y)].fg, theme::AGENT_WORKING);
        }
    }
}

#[test]
fn agents_chrome_is_english_and_uses_terminal_colors_while_data_stays_verbatim() {
    use ratatui::style::Color;
    for state in ["working", "idle", "blocked", "starting", "unknown"] {
        let (mut a, mut q) = fixture();
        a.agents[0].state = Some(state.into());
        a.agents[0].last_tool = Some("读取文件".into());
        let (buffer, hits) = render(160, 60, &mut a, &mut q, Focus::Agents);
        let panes = Panes::with_queue(buffer.area, &Config::default(), false);
        let mut chrome = String::new();
        for y in panes.agents.y..panes.agents.bottom() {
            for x in panes.agents.x..panes.agents.right() {
                let cell = &buffer[(x, y)];
                chrome.push_str(cell.symbol());
                assert!(cell.bg == Color::Reset || cell.bg == saddle::theme::AGENT_SELECTED);
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
        assert!(chrome.contains(state), "{chrome}");
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
        a.by_state = by_state;
        let (buffer, hits) = render(160, 100, &mut a, &mut q, Focus::Agents);
        let output = text(&buffer);
        assert!(
            output.lines().next().unwrap().contains("Agents · 3"),
            "{output}"
        );
        for (repo, count) in [("demo/", "2"), ("other/", "1")] {
            let y = output
                .lines()
                .position(|line| line.starts_with(&format!("┃{repo}")))
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
fn agent_status_colors_are_distinct_and_bold() {
    use ratatui::style::{Color, Modifier};
    use unicode_width::UnicodeWidthStr;
    for (state, color) in [
        ("working", Color::Rgb(0x7f, 0xb4, 0xee)),
        ("idle", Color::Rgb(0x9c, 0xbd, 0x80)),
        ("blocked", Color::Rgb(0xe6, 0xb5, 0x66)),
        ("stalled", Color::Rgb(0xe7, 0x9b, 0x65)),
        ("error", Color::Rgb(0xef, 0x81, 0x74)),
        ("starting", Color::Rgb(0xb0, 0xa1, 0xd8)),
    ] {
        for selected in [false, true] {
            let (mut a, mut q) = fixture();
            a.selected = selected.then(|| "demo/main".into());
            a.agents[0].state = Some(state.into());
            if state == "stalled" {
                a.agents[0].state = Some("working".into());
                a.agents[0].last_output = Some(-21.0);
            } else if state == "error" {
                a.agents[0].error = Some("Synthetic error".into());
            }
            let (buffer, hits) = render(160, 48, &mut a, &mut q, Focus::Agents);
            let output = text(&buffer);
            let y = hits.agents[0].0;
            let row = output.lines().nth(y as usize).unwrap();
            let start = row[..row.find(state).unwrap()].width() as u16;
            for x in start..start + state.len() as u16 {
                assert_eq!(buffer[(x, y)].fg, color, "{state}");
                assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
            }
        }
    }
}

#[test]
fn agents_scrollbar_reaches_the_bottom_when_the_last_row_is_visible() {
    for (extra, height) in [(1, 20), (12, 40)] {
        let (mut a, mut q) = fixture();
        for index in 0..extra {
            a.agents.push(Agent {
                name: format!("demo/worker-{index:02}"),
                title: Some(format!("END-{index:02}")),
                ..Default::default()
            });
        }
        a.follow = false;
        a.top = usize::MAX;
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
        cwd: Some("/tmp/demo".into()),
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
                    (hits.list.x + 6..hits.list.right())
                        .map(|x| buffer[(x, *y)].symbol())
                        .collect::<String>()
                })
                .collect();
            for field in [
                agent.kind.as_deref().unwrap(),
                "abcdef",
                "ATT 0",
                "VIA human",
                "/tmp/demo",
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
                assert_eq!(rows.len(), 5); // main, identity, path, Git line, title
            }
            if let Some(end) = previous_end {
                assert_eq!(rows[0], end + 2); // Exactly one unselected, non-clickable spacer.
                assert!(!hits.agents.iter().any(|(y, _)| *y == end + 1));
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
    let (buffer, hits) = render(160, 60, &mut a, &mut q, Focus::Queue);
    assert!(
        text(&buffer).contains("History item 00"),
        "{}",
        text(&buffer)
    );
    let last = hits.queue_rows.last().unwrap().0;
    assert_eq!(buffer[(50, last)].symbol(), "█");
    q.select(0);
    let (_, hits) = render(160, 60, &mut a, &mut q, Focus::Queue);
    let before = q.top;
    q.wheel(10, hits.queue_rows[0].0, 3);
    render(160, 60, &mut a, &mut q, Focus::Queue);
    assert_eq!(q.selected, 0);
    assert_eq!(q.top, before + 3);
    q.absorb(q.snapshot.clone().unwrap());
    render(160, 60, &mut a, &mut q, Focus::Queue);
    assert_eq!(q.top, before + 3);
    q.wheel(0, 0, 10); // Outside the list.
    assert_eq!(q.top, before + 3);
    q.select(0);
    let (buffer, _) = render(160, 60, &mut a, &mut q, Focus::Queue);
    assert!(text(&buffer).contains("History item 39"));
}

fn render_queue(q: &mut queue::Panel, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| {
            q.draw(&saddle::theme::Theme::default(), f, f.area(), true);
            q.draw_overlay(&saddle::theme::Theme::default(), f);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn assert_label_color(buffer: &Buffer, label: &str, color: ratatui::style::Color) {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if x as usize + label.len() > buffer.area.width as usize {
                continue;
            }
            if label
                .chars()
                .enumerate()
                .all(|(i, c)| buffer[(x + i as u16, y)].symbol() == c.to_string())
            {
                for i in 0..label.len() {
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
        if matches!(q.page, queue::Page::Detail(_)) {
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
    assert!(matches!(q.page, queue::Page::Detail(_)));
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
    for (width, height) in [(52, 32), (44, 24), (34, 24)] {
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
        // Narrow panes may wrap the task buttons; the footer still sits directly above them.
        let lines: Vec<_> = output.lines().collect();
        let buttons = lines.iter().position(|l| l.contains("Details")).unwrap();
        assert!(
            lines[buttons - 1].contains("History 1–1/1 · End"),
            "{output}"
        );
        assert!(
            lines[buttons..height as usize - 1]
                .iter()
                .all(|l| l.contains('‹')),
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
    let buffer = render_queue(&mut q, 52, 40);
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
    let mut terminal = Terminal::new(TestBackend::new(80, 40)).unwrap();
    terminal
        .draw(|f| q.draw_overlay(&saddle::theme::Theme::default(), f))
        .unwrap();
    // Only the overlay's interior: wrapped rows joined back must equal the source title.
    let joined: String = text(terminal.backend().buffer())
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
    let unselected = icon(&mut a, &mut q, Focus::Queue);
    a.selected = Some("demo/high".into());
    for focus in [Focus::Agents, Focus::Queue] {
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
    let (buffer, hits) = render(160, 120, &mut a, &mut q, Focus::Agents);
    let screen = text(&buffer);
    let row = |name: &str| hits.agents.iter().find(|(_, n)| n == name).unwrap().0;
    let line = |y: u16| {
        (hits.list.x + 6..hits.list.right())
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    };
    // Each agent's rows read top to bottom, with spaces dropped so wrapped lines join up.
    let info = |name: &str| {
        let rows: Vec<_> = hits.agents.iter().filter(|(_, n)| n == name).collect();
        rows.iter()
            .map(|(y, _)| line(*y))
            .collect::<String>()
            .replace(' ', "")
    };
    let compact = |s: &str| s.replace(' ', "");
    // Agents sharing a worktree show the same numbers.
    for name in ["demo/dev", "demo/twin"] {
        assert!(
            info(name).contains(&compact("dev-t12 · C2(main) · +18 -4 · ?1")),
            "{screen}"
        );
    }
    assert!(
        screen.contains("dev-t12 · C2(main) · +18 -4 · ?1"),
        "{screen}"
    );
    let main = "main · C0(origin/main) · +0 -0 · 2 binary · ?0";
    assert!(info("demo/main").contains(&compact(main)), "{screen}");
    assert!(
        !screen.contains(main),
        "narrow panes wrap the line: {screen}"
    );
    assert!(
        info("demo/odd").contains(&compact("HEAD detached · C— · +— -— · ?—")),
        "{screen}"
    );
    assert!(info("demo/gone").contains("gitunavailable"), "{screen}");
    assert!(info("demo/new").contains("git…"), "{screen}");
    assert_label_color(&buffer, "+18", theme::AGENT_IDLE);
    assert_label_color(&buffer, "-4", theme::AGENT_ERROR);
    // The Git line sits right below the agent's own directory line.
    let y = (row("demo/main")..)
        .find(|y| line(*y).contains("/w/main"))
        .unwrap();
    assert!(line(y + 1).contains("C0(origin/main)"), "{screen}");
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
fn task_details_replace_the_list_inside_the_queue_pane_only() {
    let (mut a, _) = fixture();
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    open_detail(&mut q, Some(show_json()));
    let (buffer, hits) = render(160, 48, &mut a, &mut q, Focus::Queue);
    let panes = Panes::with_queue(buffer.area, &Config::default(), true);
    assert!(
        hits.queue_rows.is_empty(),
        "the list is not behind the details"
    );
    let all = text(&buffer);
    assert!(all.contains("demo/main") && all.contains("Viewer · demo/main"));
    for label in ["Completion checks", "Back Esc", "Task details"] {
        let (x, y) = find(&buffer, label).unwrap_or_else(|| panic!("{label}: {all}"));
        assert!(panes.queue.contains((x, y).into()), "{label}");
    }
    let outside: String = (0..buffer.area.height)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .filter(|(x, y)| !panes.queue.contains((*x, *y).into()))
        .map(|(x, y)| buffer[(x, y)].symbol().to_owned())
        .collect();
    // No overlay: nothing of the details is drawn over Agents or Viewer.
    for detail_text in ["没有收尾提交", "Completion", "Back Esc"] {
        assert!(!outside.contains(detail_text), "{detail_text}: {all}");
    }
    assert!(all.contains("Input ▸ Queue · Task details"), "{all}");
}

#[test]
fn detail_task_and_edit_buttons_work_with_mouse_and_restore_reading_positions() {
    use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers};
    let (mut a, mut q) = fixture();
    q.snapshot.as_mut().unwrap().pending[0].body = (0..90)
        .map(|i| format!("Original body line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    q.open(0);
    render(160, 48, &mut a, &mut q, Focus::Queue);
    q.key(KeyEvent::new(K::PageDown, KeyModifiers::NONE));
    let (before, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    let (x, y) = find(&before, "Task t").expect("fixed Task button");
    q.click(x, y);
    let (overlay, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    assert!(text(&overlay).contains("Original body line 0"));
    assert!(q.overlay_open());
    q.key(KeyEvent::new(K::PageDown, KeyModifiers::NONE));
    let (overlay, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    let (x, y) = find(&overlay, "Edit e").expect("Task overlay Edit button");
    q.click(x, y);
    let (editor, _) = render(160, 48, &mut a, &mut q, Focus::Queue);
    assert!(text(&editor).contains("Edit task"));
    let (x, y) = find(&editor, "Cancel Esc").unwrap();
    q.click(x, y);
    assert_eq!(
        text(&render(160, 48, &mut a, &mut q, Focus::Queue).0),
        text(&overlay)
    );
    let (x, y) = find(&overlay, "Back Esc").unwrap();
    q.click(x, y);
    assert_eq!(
        text(&render(160, 48, &mut a, &mut q, Focus::Queue).0),
        text(&before)
    );
    let (x, y) = find(&before, "Edit e").expect("fixed detail Edit button");
    q.click(x, y);
    assert!(matches!(q.page, queue::Page::Edit { .. }));
    q.key(KeyEvent::new(K::Esc, KeyModifiers::NONE));
    assert_eq!(
        text(&render(160, 48, &mut a, &mut q, Focus::Queue).0),
        text(&before)
    );

    // Once the task starts, both detail and Task overlay lose the editing entry.
    let mut fresh = q.snapshot.clone().unwrap();
    fresh.current = Some(fresh.pending.remove(0));
    q.absorb(fresh);
    for (w, h) in [(160, 48), (60, 24)] {
        let (buffer, _) = render(w, h, &mut a, &mut q, Focus::Queue);
        assert!(find(&buffer, "Task t").is_some());
        assert!(find(&buffer, "Edit e").is_none());
    }
    q.key(KeyEvent::new(K::Char('t'), KeyModifiers::NONE));
    assert!(find(&render(160, 48, &mut a, &mut q, Focus::Queue).0, "Edit e").is_none());
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
    let out = text(&render_queue(&mut q, 70, 120));
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
    let out = text(&render_queue(&mut q, 70, 120));
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
    let out = text(&render_queue(&mut q, 70, 120));
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
    let out = text(&render_queue(&mut q, 60, 30));
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
    let out = text(&render_queue(&mut q, 60, 30));
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
    render_queue(&mut q, 60, 30);
    for _ in 0..3 {
        q.key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    }
    let scrolled = text(&render_queue(&mut q, 60, 30));
    assert!(!scrolled.contains("Detail target 任务"), "{scrolled}");
    q.absorb_detail(&key, Ok(detail));
    assert_eq!(text(&render_queue(&mut q, 60, 30)), scrolled);
    let (x, y) = find(&render_queue(&mut q, 60, 30), "body line").unwrap();
    q.wheel(x, y, -1);
    assert_ne!(text(&render_queue(&mut q, 60, 30)), scrolled);

    q.absorb_detail(&key, Err(anyhow::anyhow!("show cancelled or timed out")));
    let out = text(&render_queue(&mut q, 60, 200));
    for expected in ["timed out", "stale", "Completion checks", "Retrying"] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
}
