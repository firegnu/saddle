use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    layout::{Position, Rect},
};
use saddle_drover_plugin as saddle;
use saddle_drover_plugin::{
    drover::{Snapshot, Task},
    queue,
};
#[derive(Clone, Copy)]
enum Focus {
    Queue,
}
#[derive(Default)]
struct Hits {
    queue_rows: Vec<(u16, usize)>,
    agents: Vec<()>,
    terminal: Vec<()>,
}
struct Panes {
    tasks: Rect,
}
impl Panes {
    fn new(area: Rect, _: &Config) -> Self {
        Self { tasks: area }
    }
}
#[derive(Default)]
struct Config;
fn fixture() -> ((), queue::Panel) {
    let mut q = queue::Panel::default();
    q.project = "/tmp/demo".into();
    q.absorb(Snapshot {
        pending: vec![Task {
            id: Some("T12345".into()),
            title: "中文任务很长需要截断但状态必须仍然可见".repeat(3),
            ..Default::default()
        }],
        ..Default::default()
    });
    ((), q)
}
fn render(w: u16, h: u16, _: &mut (), q: &mut queue::Panel, _: Focus) -> (Buffer, Hits) {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut hits = Hits::default();
    t.draw(|f| {
        hits.queue_rows = q.draw(&Default::default(), f, Rect::new(0, 0, w, h));
    })
    .unwrap();
    (t.backend().buffer().clone(), hits)
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

fn render_queue(q: &mut queue::Panel, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| {
            q.draw(&saddle::theme::Theme::default(), f, f.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn show_json() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/show.json")).unwrap()
}
fn detail_queue(mut list_task: serde_json::Value) -> queue::Panel {
    let mut q = queue::Panel::default();
    q.project = "/tmp/demo".into();
    list_task["run_id"] = "run-4".into();
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
        (80, 48, true),
        (80, 24, false),
    ] {
        let (mut a, mut q) = fixture();
        let (buffer, hits) = render(w, h, &mut a, &mut q, Focus::Queue);
        let output = text(&buffer);

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
            "Pause p",
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
        let panes = Panes::new(buffer.area, &Config);
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
fn queue_history_scrollbar_reaches_the_end_with_the_last_task_visible() {
    let (mut a, mut q) = fixture();
    q.absorb(Snapshot {
        history: (0..40)
            .rev()
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
    let panes = Panes::new(buffer.area, &Config);
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
            title: queue::Input::new("原始任务".into()),
            body: queue::Input::new("原始正文".into()),
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
        assert!(!text(&buffer).contains("Loop"));
        assert!(text(&buffer).contains("Queue: "), "{label}");
    }
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
    q.complete(
        &Operation::Pause(true),
        Err(anyhow::anyhow!("Synthetic failure")),
    );
    assert_label_color(
        &render_queue(&mut q, 80, 32),
        "Synthetic failure",
        t::AGENT_ERROR,
    );
    q.complete(&Operation::Pause(true), Ok("Synthetic success".into()));
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
    // The plugin renders content without an outer popup frame. Wrapped rows
    // joined back must still contain the complete source title.
    let joined: String = output.lines().map(str::trim).collect();
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
fn run_details_sit_beside_the_list_inside_the_popup_only() {
    let (mut a, _) = fixture();
    let mut q = detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务"}));
    open_detail(&mut q, Some(show_json()));
    let (buffer, hits) = render(160, 48, &mut a, &mut q, Focus::Queue);
    let panes = Panes::new(buffer.area, &Config);
    assert!(
        !hits.queue_rows.is_empty(),
        "the list stays beside the details"
    );
    let all = text(&buffer);
    for label in [
        "Repository reference",
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
}

#[test]
fn task_tabs_mark_the_chosen_view_and_details_color_structured_states() {
    use ratatui::style::Modifier;
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
    // Reference failures keep their semantic color and do not become completion gates.
    let buffer = render_queue(&mut q, 106, 120);
    for label in ["Repository reference", "Last check", "Run records"] {
        assert_eq!(style(&buffer, label, 0, 5), (heading, true), "{label}");
    }
    assert!(text(&buffer).contains("stale"));
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
    let mut value = show_json();
    value["task"]["status"] = "done".into();
    value["task"]["t1"] = 1790363000.into();
    value["task"]["submission"] = serde_json::json!({"event":"done","gate":false,"t":1790363000});
    value["task"]["body"] = "safe \u{1b}[31mred\u{7} text".into();
    value["task"]["completion_record"] = serde_json::json!({"method":"manual","reason":"legacy override","last_check":{"record":{"ok":false,"why":"old failure"}}});
    value["task"]["previous_runs"] = serde_json::json!([{
        "run_id":"older-run", "t0":1790300000,"t1":1790301000,
        "submission":{"event":"submitted"},
        "return_history":[{"reason":"more research","work_stopped":true,"returned_at":1790302000}]
    }]);
    let mut q =
        detail_queue(serde_json::json!({"id":"T4", "title":"Detail target 任务", "at":"history"}));
    open_detail(&mut q, Some(value.clone()));
    let out = text(&render_queue(&mut q, 150, 150));
    for expected in [
        "Done",
        "Repository reference",
        "unavailable",
        "stale",
        "failed earlier",
        "not recorded",
        "Legacy manual completion",
        "legacy override",
        "old failure",
        "Previous run 1",
        "older-run",
        "more research",
        "safe [31mred text",
    ] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
    assert!(!out.contains('\u{1b}') && !out.contains('\u{7}'));
    assert!(
        !out.contains("Completion checks") && !out.contains("Result       passed"),
        "{out}"
    );
    // Only a real t2 gives an acceptance time.
    value["task"]["t2"] = 1790363030.into();
    let key = q.detail_key().unwrap();
    q.absorb_detail(&key, Ok(serde_json::from_value(value).unwrap()));
    let recorded = text(&render_queue(&mut q, 150, 150));
    let accepted = recorded
        .lines()
        .find(|line| line.contains("Accepted"))
        .unwrap();
    assert!(!accepted.contains("not recorded"));
    render_queue(&mut q, 34, 200);
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
        !out.contains("Repository reference") && !out.contains("Elapsed"),
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
    assert!(!out.contains("Repository reference"), "{out}");

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
    for expected in ["timed out", "stale", "Repository reference", "Retrying"] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
}

#[test]
fn return_to_pending_is_a_running_task_confirmation() {
    let mut q = queue::Panel::default();
    q.project = "/tmp/project-a".into();
    q.absorb(
        serde_json::from_value(serde_json::json!({
            "mode":{}, "paused":false, "awaiting":null, "history":[],
            "current": {"id":"T4", "title":"Research", "run_id":"run-4", "status":"running"},
            "pending": [{"id":"T5", "title":"Next"}]
        }))
        .unwrap(),
    );
    let buffer = render_queue(&mut q, 150, 40);
    let screen = text(&buffer);
    assert!(screen.contains("Return to pending…"), "{screen}");
    let (x, y) = (0..buffer.area.height)
        .find_map(|y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect();
            row.find("Return to pending")
                .map(|at| (row[..at].chars().count() as u16, y))
        })
        .unwrap();
    assert!(q.click(x, y).is_none(), "opening never writes");
    let screen = text(&render_queue(&mut q, 150, 40));
    for words in [
        "Return to pending",
        "T4 · Research",
        "unchanged",
        "stopped",
        "Reason",
        "Cancel Esc",
    ] {
        assert!(screen.contains(words), "{words}\n{screen}");
    }
    q.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ));
    q.view = queue::View::Dispatch;
    q.key(queue::return_click());
    let key = q.confirmation_key().unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    value["task"]["actions"]["return-to-pending"] =
        serde_json::json!({"target_token":"r1:test", "unavailable_reason":null});
    q.absorb_confirmation(&key, Ok(serde_json::from_value(value).unwrap()));
    q.paste("Wrong dispatch");
    let find_button = |buffer: &Buffer, label: &str| {
        (0..buffer.area.height)
            .find_map(|y| {
                let row: String = (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_owned())
                    .collect();
                row.find(label)
                    .map(|at| (row[..at].chars().count() as u16, y))
            })
            .unwrap()
    };
    let buffer = render_queue(&mut q, 150, 40);
    let (x, y) = find_button(&buffer, "[ ] Work has stopped");
    assert!(q.click(x, y).is_none());
    let buffer = render_queue(&mut q, 150, 40);
    let (x, y) = find_button(&buffer, "Return to pending ↵");
    let Some(saddle::drover::Request::Run(op)) = q.click(x, y) else {
        panic!("confirmation click must submit even from Dispatch")
    };
    assert!(matches!(
        op,
        saddle::drover::Operation::Transition {
            action: saddle::drover::Transition::Return,
            ..
        }
    ));
    q.complete(&op, Ok("returned".into()));
    q.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ));
    q.select(1);
    assert!(!text(&render_queue(&mut q, 150, 40)).contains("Return to pending…"));
}

#[test]
fn submit_is_a_selected_running_task_button_with_an_explicit_confirmation() {
    let mut q = queue::Panel::default();
    q.project = "/tmp/project-a".into();
    q.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "awaiting": null,
            "current": {"id":"T4", "title":"Research", "run_id":"run-4", "status":"running"},
            "pending": [{"id":"T5", "title":"Next"}], "history": []
        }))
        .unwrap(),
    );
    let screen = text(&render_queue(&mut q, 150, 40));
    assert!(screen.contains("Submit for review"), "{screen}");
    q.select(1);
    assert!(!text(&render_queue(&mut q, 150, 40)).contains("Submit for review"));
    q.select(0);
    let buffer = render_queue(&mut q, 150, 40);
    // Clicking the button opens the page; the click needs no shortcut key.
    let (x, y) = (0..buffer.area.height)
        .find_map(|y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect();
            row.find("Submit for review")
                .map(|at| (row[..at].chars().count() as u16, y))
        })
        .expect("the button is drawn");
    assert!(q.click(x, y).is_none());
    let key = q.confirmation_key().expect("the page reads its target");
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    value["task"]["title"] = "Research".into();
    value["task"]["actions"]["done"] =
        serde_json::json!({"target_token": "tok", "unavailable_reason": null});
    q.absorb_confirmation(&key, Ok(serde_json::from_value(value).unwrap()));
    let screen = text(&render_queue(&mut q, 150, 40));
    for words in [
        "Submit for review",
        "project-a",
        "T4 · Research",
        "wait for acceptance",
        "reference only",
        "run-4",
        "Cancel Esc",
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
#[test]
fn task_editor_click_puts_the_shown_cursor_where_text_goes() {
    use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
    let mut q = queue::Panel::default();
    q.absorb(Snapshot {
        pending: vec![Task {
            id: Some("T1".into()),
            title: "中文标题".into(),
            body: "第一行\n第二行".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    q.key(KeyEvent::new(K::Char('e'), M::NONE));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut draw = |q: &mut queue::Panel| {
        terminal
            .draw(|f| {
                q.draw(&saddle::theme::Theme::default(), f, f.area());
            })
            .unwrap();
        (
            terminal.backend().buffer().clone(),
            terminal.backend().cursor_position(),
        )
    };
    let (buffer, _) = draw(&mut q);
    let (x, y) = find(&buffer, "标").expect("title shown");
    q.click(x, y);
    assert_eq!(draw(&mut q).1, Position::new(x, y), "cursor at the click");
    q.key(KeyEvent::new(K::Char('X'), M::NONE));
    let (x, y) = find(&buffer, "二").expect("body shown");
    q.click(x + 1, y);
    assert_eq!(
        draw(&mut q).1,
        Position::new(x, y),
        "cursor snaps to the wide char"
    );
    q.key(KeyEvent::new(K::Left, M::NONE));
    assert_eq!(
        draw(&mut q).1,
        Position::new(x - 2, y),
        "cursor follows Left"
    );
    q.paste("Y");
    let request = q.key(KeyEvent::new(K::Char('s'), M::CONTROL));
    let Some(saddle::drover::Request::Run(saddle::drover::Operation::Edit { title, body, .. })) =
        request
    else {
        panic!("Ctrl-S saves the edit");
    };
    assert_eq!(
        (title.as_str(), body.as_str()),
        ("中文X标题", "第一行\nY第二行")
    );
}

#[test]
fn returned_pending_run_details_show_the_saved_history() {
    let mut q = queue::Panel::default();
    let record = serde_json::json!({"reason":"Wrong dispatch", "dispatched_at":1790000000, "returned_at":1790000100, "work_stopped":true});
    q.absorb(serde_json::from_value(serde_json::json!({"mode":{}, "paused":true, "current":null, "awaiting":null, "history":[], "pending":[{
        "id":"T4", "title":"Research", "run_id":"run-4", "return_history":[record.clone()]
    }]})).unwrap());
    q.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ));
    let key = q.detail_key().expect("returned Pending supports details");
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/show.json")).unwrap();
    value["task"]["location"] = "pending".into();
    value["task"]["status"] = "pending".into();
    value["task"]["return_history"] = serde_json::json!([record]);
    q.absorb_detail(&key, Ok(serde_json::from_value(value).unwrap()));
    let screen = text(&render_queue(&mut q, 150, 48));
    for words in [
        "Return history",
        "Wrong dispatch",
        "Returned",
        "Work stopped",
        "user's confirmation",
    ] {
        assert!(screen.contains(words), "{words}\n{screen}");
    }
    assert!(!screen.contains("Unknown status: pending"), "{screen}");
}

#[test]
fn dispatch_selected_is_a_pending_task_button_that_sends_the_shown_target() {
    let mut q = queue::Panel::default();
    q.project = "/tmp/project-a".into();
    let snapshot = |second: serde_json::Value| {
        serde_json::from_value(serde_json::json!({
            "mode":{}, "paused":false, "current":null, "awaiting":null,
            "history":[{"id":"T0", "title":"Old", "status":"done"}],
            "pending":[
                {"id":"T1", "title":"First", "actions":{"dispatch-pending":{"pos":1, "target_token":"d1:one", "unavailable_reason":null}}},
                {"id":"T2", "title":"Second", "actions":{"dispatch-pending":second}}
            ]
        }))
        .unwrap()
    };
    q.absorb(snapshot(
        serde_json::json!({"pos":2, "target_token":"d1:two", "unavailable_reason":null}),
    ));
    q.select(1);
    let buffer = render_queue(&mut q, 150, 40);
    let screen = text(&buffer);
    assert!(
        screen.contains("Dispatch selected") && !screen.contains("Next n"),
        "{screen}"
    );
    let (x, y) = find(&buffer, "Dispatch selected").unwrap();
    let Some(saddle::drover::Request::Run(op)) = q.click(x, y) else {
        panic!("the button sends the selected task")
    };
    assert_eq!(
        op,
        saddle::drover::Operation::DispatchPending {
            project: "/tmp/project-a".into(),
            pos: 2,
            token: "d1:two".into(),
        }
    );
    q.complete(&op, Ok("done".into()));
    q.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ));
    // Shown but inert when the target is unavailable.
    q.absorb(snapshot(
        serde_json::json!({"pos":2, "target_token":null, "unavailable_reason":"target_ambiguous"}),
    ));
    let buffer = render_queue(&mut q, 150, 40);
    let (x, y) = find(&buffer, "Dispatch selected").expect("still drawn");
    assert!(q.click(x, y).is_none());
    // History offers no dispatch.
    q.select(2);
    assert!(!text(&render_queue(&mut q, 150, 40)).contains("Dispatch selected"));
}
