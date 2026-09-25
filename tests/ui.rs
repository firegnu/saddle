use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};
use saddle::{
    agents,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    drover::{Snapshot, Task},
    input::Focus,
    layout::Panes,
    queue, theme,
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
fn management_layouts_keep_cjk_status_and_input_target_visible() {
    for (w, h) in [(160, 48), (120, 36), (80, 24)] {
        let (mut a, mut q) = fixture();
        let (buffer, hits) = render(w, h, &mut a, &mut q, Focus::Queue);
        let output = text(&buffer);
        assert!(output.contains("输入 ▸ Queue"), "{output}");
        assert!(
            output.contains("T12345") && output.contains("…") && output.contains("待办"),
            "{output}"
        );
        assert!(!hits.queue_rows.is_empty());
        let row = hits.queue_rows[0].0;
        let panes = Panes::with_queue(buffer.area, &Config::default(), true);
        assert_eq!(buffer[(panes.queue.right() - 4, row)].bg, theme::SELECTED);
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
