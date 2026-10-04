//! Synthetic draws of the Drover display polish: grouped task actions keep their clicks, and
//! retired or ambiguous wording stays gone.
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use saddle_drover_plugin::{
    drover::{Request, Snapshot, Task},
    queue::{self, Page},
    theme::Theme,
};

fn render(q: &mut queue::Panel, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| {
            q.draw(&Theme::default(), f, f.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
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
    (0..buffer.area.height).find_map(|y| {
        let row: Vec<_> = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol().to_owned())
            .collect();
        let chars: Vec<_> = label.chars().map(String::from).collect();
        row.windows(chars.len())
            .position(|w| w == chars.as_slice())
            .map(|x| (x as u16, y))
    })
}
fn pending_panel() -> queue::Panel {
    let mut q = queue::Panel::default();
    q.project = "/tmp/demo".into();
    q.absorb(
        serde_json::from_value(serde_json::json!({
            "mode": {}, "paused": false, "awaiting": null, "current": null,
            "pending": [{"id":"T1", "title":"First", "body":"Body text", "actions":{"dispatch-pending":
                            {"pos":1, "target_token":"d1:one", "unavailable_reason":null}}},
                        {"id":"T2", "title":"Second"}],
            "history": []
        }))
        .unwrap(),
    );
    q
}

#[test]
fn grouped_task_actions_keep_their_clicks_at_each_size() {
    for (width, height) in [(150, 40), (100, 30), (80, 24)] {
        let mut q = pending_panel();
        let buffer = render(&mut q, width, height);
        let screen = text(&buffer);
        for label in [
            "‹Add task a›",
            "‹Edit e›",
            "‹Delete x›",
            "‹Help ?›",
            "‹Close Esc›",
        ] {
            assert!(screen.contains(label), "{width}x{height} {label}\n{screen}");
        }
        assert!(screen.contains('│'), "group divider\n{screen}");
        for (label, opened) in [
            ("Edit e", "Edit task"),
            ("Delete x", "Delete task"),
            ("Help ?", "Tasks help"),
        ] {
            q.page = Page::List;
            let buffer = render(&mut q, width, height);
            let (x, y) = find(&buffer, label).expect(label);
            assert!(q.click(x, y).is_none(), "{label}");
            let screen = text(&render(&mut q, width, height));
            assert!(
                screen.contains(opened),
                "{width}x{height} {label}\n{screen}"
            );
        }
        q.page = Page::List;
        let buffer = render(&mut q, width, height);
        let (x, y) = find(&buffer, "Dispatch selected").expect("dispatch");
        assert!(matches!(q.click(x, y), Some(Request::Run(_))));
    }
}

#[test]
fn delete_path_and_empty_wording_is_current() {
    let mut q = pending_panel();
    q.page = Page::Delete {
        pending: q.snapshot.as_ref().unwrap().pending.clone(),
        index: 0,
    };
    let screen = text(&render(&mut q, 120, 36));
    assert!(screen.contains("History as Dropped"), "{screen}");
    assert!(screen.contains("T1 · First") && screen.contains("Body text"));
    assert!(!screen.contains("drover drop"), "{screen}");

    q.page = Page::Project("/tmp/demo".into());
    let screen = text(&render(&mut q, 120, 36));
    assert!(!screen.contains("queue.cwd"), "{screen}");
    assert!(screen.contains("Project path"), "{screen}");

    let mut q = queue::Panel::default();
    q.absorb(Snapshot::default());
    let screen = text(&render(&mut q, 120, 36));
    assert!(screen.contains("No active tasks · Add task a"), "{screen}");
    assert!(screen.contains("No history yet"), "{screen}");
    q.absorb(Snapshot {
        history: vec![Task {
            title: "Old".into(),
            status: Some("done".into()),
            ..Default::default()
        }],
        ..Default::default()
    });
    let screen = text(&render(&mut q, 120, 36));
    assert!(screen.contains("No active tasks · Add task a"), "{screen}");
    assert!(!screen.contains("No history yet"), "{screen}");
}

#[test]
fn short_task_editor_keeps_a_body_line_and_normal_size_keeps_the_hint() {
    let mut q = pending_panel();
    let editor = || Page::Add {
        title: queue::Input::new("Short title".into()),
        body: queue::Input::new("Visible body line".into()),
        body_focus: false,
    };
    q.page = editor();
    let screen = text(&render(&mut q, 80, 10));
    assert!(screen.contains("Visible body line"), "80x10\n{screen}");
    q.page = editor();
    let screen = text(&render(&mut q, 120, 36));
    assert!(screen.contains("Visible body line"), "{screen}");
    assert!(screen.contains("Tab Switch field"), "{screen}");
}
