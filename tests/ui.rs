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
    assert!(output.contains("‹Attached› ‹Sort s› ‹Stop x›"), "{output}");
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
                assert_eq!(rows.len(), 4);
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
            q.draw(f, f.area(), true);
            q.draw_overlay(f);
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
    for page in [
        queue::Page::List,
        queue::Page::Detail,
        queue::Page::Help,
        queue::Page::Projects,
        queue::Page::Project("/tmp/demo".into()),
        queue::Page::Add {
            title: "原始任务".into(),
            body: "原始正文".into(),
            body_focus: false,
        },
        queue::Page::Feedback("原始反馈".into()),
    ] {
        q.page = page;
        let buffer = render_queue(&mut q, 80, 32);
        let output = text(&buffer);
        if matches!(q.page, queue::Page::Detail) {
            for value in ["原始任务", "原始正文", "原始原因"] {
                assert!(output.contains(value));
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
    }
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
        assert!(
            output
                .lines()
                .nth(height as usize - 3)
                .unwrap()
                .contains("History 1–1/1 · End"),
            "{output}"
        );
        assert!(
            output
                .lines()
                .nth(height as usize - 2)
                .unwrap()
                .contains("Details")
        );
    }
}
