//! Exercise the actual executable and SDK wire path with an isolated HOME and fake CLI.
use saddle_plugin_sdk::{
    buffer,
    protocol::{self, Frame, Message},
};
use serde_json::json;
use std::{
    io::{BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
mod common;
struct Running {
    child: Child,
    input: ChildStdin,
    output: Receiver<Message>,
    frame: Option<Frame>,
    input_id: u64,
    root: tempfile::TempDir,
}
impl Running {
    fn start() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".drover")).unwrap();
        std::fs::write(
            root.path().join(".drover/projects"),
            root.path().display().to_string(),
        )
        .unwrap();
        let fixture = include_str!("fixtures/drover.py").replace("if len(args) == 4", "if args[:1] == ['notifications']:\n    pref = root / 'system-notifications'\n    if args[1] != 'status':\n        if (root / 'fail-preference').exists():\n            print(json.dumps(dict(schema_version=1, ok=False, error=dict(code='preferences_write_failed', message='synthetic failure')))); sys.exit(1)\n        pref.write_text(args[1])\n    print(json.dumps(dict(schema_version=1, ok=True, scope='user', system_enabled=pref.exists() and pref.read_text() == 'on', revision=2 if pref.exists() else 1)))\n    sys.exit(0)\nif len(args) == 4");
        common::script(root.path(), "drover", &fixture);
        let mut child = Command::new(env!("CARGO_BIN_EXE_saddle-drover"))
            .args(["--refresh-ms", "100"])
            .current_dir(root.path())
            .env("HOME", root.path())
            .env("PATH", format!("{}:/usr/bin:/bin", root.path().display()))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let (send, output) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(Some(m)) = protocol::read(&mut stdout) {
                if send.send(m).is_err() {
                    break;
                }
            }
        });
        let mut this = Self {
            child,
            input,
            output,
            frame: None,
            input_id: 0,
            root,
        };
        this.send(Message::request(1, "initialize", json!({"protocol_major":1,"width_profile":protocol::PROFILE,"capabilities":protocol::CAPABILITIES,"theme":{}})));
        assert!(matches!(this.receive(), Message::Response { id: 1, .. }));
        this.send(Message::event(
            "panel.open",
            json!({"cols":100,"rows_count":30,"size_revision":1}),
        ));
        this.send(Message::event("panel.focus", json!({"focused":true})));
        this.see("Native queue task");
        this
    }
    fn send(&mut self, message: Message) {
        self.input
            .write_all(&protocol::encode(&message).unwrap())
            .unwrap();
        self.input.flush().unwrap();
    }
    fn receive(&mut self) -> Message {
        self.output
            .recv_timeout(Duration::from_secs(15))
            .expect("plugin response timed out")
    }
    fn record(&mut self, message: &Message) {
        match message {
            Message::Request { id, .. } => {
                self.send(Message::response(*id, json!({"status":"accepted"})))
            }
            Message::Event { name, data } if name == "panel.frame" => {
                let frame: Frame = serde_json::from_value(data.clone()).unwrap();
                frame.validate().unwrap();
                self.frame = Some(frame);
            }
            Message::Event { name, data } if name == "panel.error" => {
                panic!("render failed: {data}")
            }
            _ => {}
        }
    }
    fn see(&mut self, text: &str) {
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(Instant::now() < end, "missing {text}");
            let message = self.receive();
            self.record(&message);
            if matches!(message, Message::Event {ref name, ..} if name == "panel.frame") {
                let b = buffer(self.frame.as_ref().unwrap()).unwrap();
                let text_now: String = b.content.iter().map(|c| c.symbol()).collect();
                if text_now.contains(text) {
                    return;
                }
            }
        }
    }
    fn input(&mut self, event: serde_json::Value) {
        self.input_id += 1;
        let frame = self.frame.as_ref().unwrap();
        self.send(Message::event("input", json!({"input_id":self.input_id,"frame_id":frame.frame_id,"size_revision":frame.size_revision,"event":event})));
    }
    fn key(&mut self, c: &str) {
        self.input(json!({"type":"key","code":{"char":c},"modifiers":[],"phase":"press"}));
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        self.send(Message::request(100, "shutdown", json!({})));
        let end = Instant::now() + Duration::from_secs(3);
        while self.child.try_wait().unwrap().is_none() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.child.try_wait().unwrap().is_none() {
            self.child.kill().unwrap();
        }
        self.child.wait().unwrap();
    }
}
#[test]
fn task_form_has_cursor_multiline_paste_and_escape_without_writes() {
    let mut p = Running::start();
    p.key("a");
    p.see("Save ^s");
    assert!(p.frame.as_ref().unwrap().cursor.is_some());
    assert!(p.frame.as_ref().unwrap().escape_input);
    p.input(json!({"type":"paste","text":"draft title"}));
    p.see("draft title");
    p.input(json!({"type":"key","code":{"name":"tab"},"modifiers":[],"phase":"press"}));
    p.see("Save ^s");
    p.input(json!({"type":"paste","text":"line one\nline two"}));
    p.see("line two");
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Native queue task");
    assert!(!p.frame.as_ref().unwrap().escape_input);
    let events = std::fs::read_to_string(p.root.path().join("queue-events")).unwrap();
    assert!(
        events.lines().all(|line| {
            let args: Vec<String> = serde_json::from_str(line).unwrap();
            matches!(args[0].as_str(), "list" | "show" | "notifications")
        }),
        "{events}"
    );
}

#[test]
fn list_close_requests_only_the_view_to_close() {
    let mut p = Running::start();
    p.key("q");
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(
            Instant::now() < end,
            "list Close must request closing its host view"
        );
        if let Ok(message) = p.output.recv_timeout(Duration::from_millis(100)) {
            if matches!(&message, Message::Event {name, ..} if name == "panel.close_request") {
                break;
            }
            p.record(&message);
        }
    }
    assert!(
        p.child.try_wait().unwrap().is_none(),
        "closing a view must not stop its background process"
    );
}

#[test]
fn notification_preference_requires_save_and_a_failed_save_can_be_retried() {
    let mut p = Running::start();
    p.key("N");
    p.see("Selected: In Saddle");
    p.key("s");
    p.see("Selected: System");
    assert!(!p.root.path().join("system-notifications").exists());
    std::fs::write(p.root.path().join("fail-preference"), "fail").unwrap();
    p.input(json!({"type":"key","code":{"char":"s"},"modifiers":["control"],"phase":"press"}));
    p.see("Not saved:");
    assert!(!p.root.path().join("system-notifications").exists());
    std::fs::remove_file(p.root.path().join("fail-preference")).unwrap();
    p.input(json!({"type":"key","code":{"char":"s"},"modifiers":["control"],"phase":"press"}));
    p.see("Saved.");
    assert_eq!(
        std::fs::read_to_string(p.root.path().join("system-notifications")).unwrap(),
        "on"
    );
}

#[test]
fn closing_view_keeps_attention_and_new_awaiting_notifications_running_without_transitions() {
    let mut p = Running::start();
    p.key("N");
    p.see("Selected: In Saddle");
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Native queue task");
    p.key("r"); // A known preference followed by a fresh empty baseline.
    let until = Instant::now() + Duration::from_millis(700);
    while Instant::now() < until {
        if let Ok(m) = p.output.recv_timeout(Duration::from_millis(50)) {
            p.record(&m);
        }
    }
    p.send(Message::event("panel.close", json!({})));
    p.send(Message::request(2, "ping", json!({})));
    loop {
        let m = p.receive();
        p.record(&m);
        if matches!(m, Message::Response { id: 2, .. }) {
            break;
        }
    }
    std::fs::write(p.root.path().join("queue-state.json"), json!({"paused":false,"pending":[],"history":[],"current":null,"awaiting":{"id":"T9","title":"Review plugin task","body":"x".repeat(20000),"status":"awaiting_release","run_id":"r9","notification_key":"T9/r9"}}).to_string()).unwrap();
    let mut attention = false;
    let mut notifications = 0;
    let end = Instant::now() + Duration::from_secs(2);
    while Instant::now() < end {
        if let Ok(m) = p.output.recv_timeout(Duration::from_millis(50)) {
            match &m {
                Message::Request { method, params, .. } if method == "attention.replace" => {
                    let snapshot: protocol::AttentionSnapshot =
                        serde_json::from_value(params.clone()).unwrap();
                    snapshot.validate().unwrap();
                    attention |= snapshot.items.iter().any(|i| i.title.contains("T9"));
                }
                Message::Request { method, params, .. } if method == "notify" => {
                    assert!(params["text"].as_str().unwrap().contains("T9"));
                    assert!(params["target"]["target"]["id"].is_string());
                    notifications += 1;
                }
                Message::Event { name, .. } if name == "panel.frame" => {
                    panic!("closed view should not render")
                }
                _ => {}
            }
            p.record(&m);
        }
    }
    assert!(
        attention,
        "calls: {}",
        std::fs::read_to_string(p.root.path().join("queue-events")).unwrap()
    );
    assert_eq!(notifications, 1);
    assert!(p.child.try_wait().unwrap().is_none());
    let calls = std::fs::read_to_string(p.root.path().join("queue-events")).unwrap();
    assert!(calls.lines().all(|line| {
        let args: Vec<String> = serde_json::from_str(line).unwrap();
        matches!(args[0].as_str(), "list" | "show" | "notifications")
    }));
}

#[test]
fn mouse_release_completes_the_pressed_button_after_its_feedback_frame() {
    let mut p = Running::start();
    let frame = p.frame.as_ref().unwrap().clone();
    let b = buffer(&frame).unwrap();
    let (x, y) = (0..b.area.height)
        .find_map(|y| {
            (0..b.area.width.saturating_sub(9))
                .find(|x| {
                    (0..9).map(|n| b[(*x + n, y)].symbol()).collect::<String>() == "Close Esc"
                })
                .map(|x| (x, y))
        })
        .expect("Close button");
    p.input(json!({"type":"mouse","action":"down","button":"left","x":x,"y":y}));
    p.see("Close Esc");
    p.input_id += 1;
    p.send(Message::event("input", json!({"input_id":p.input_id,"frame_id":frame.frame_id,"size_revision":frame.size_revision,"event":{"type":"mouse","action":"up","button":"left","x":x,"y":y}})));
    let end = Instant::now() + Duration::from_secs(2);
    let mut closed = false;
    while Instant::now() < end {
        if let Ok(m) = p.output.recv_timeout(Duration::from_millis(50)) {
            if matches!(&m, Message::Event {name,..} if name == "panel.close_request") {
                closed = true;
                break;
            }
            p.record(&m);
        }
    }
    assert!(
        closed,
        "release from the pressed frame must complete that mouse gesture"
    );
}

#[test]
fn keyboard_burst_is_ordered_across_redraws_and_replayed_input_is_rejected() {
    let mut p = Running::start();
    p.key("a");
    p.see("Save ^s");
    let frame = p.frame.as_ref().unwrap().clone();
    // All events were typed against the same displayed frame, before new frames arrive.
    for (id, c) in [(2, "a"), (3, "b"), (4, "c"), (3, "x")] {
        p.send(Message::event("input", json!({"input_id":id,"frame_id":frame.frame_id,"size_revision":frame.size_revision,"event":{"type":"key","code":{"char":c},"modifiers":[],"phase":"press"}})));
    }
    p.see("abc");
    p.send(Message::request(2, "ping", json!({})));
    loop {
        let m = p.receive();
        p.record(&m);
        if matches!(m, Message::Response { id: 2, .. }) {
            break;
        }
    }
    let b = buffer(p.frame.as_ref().unwrap()).unwrap();
    let text: String = b.content.iter().map(|c| c.symbol()).collect();
    assert!(!text.contains("abcx"));
}

#[test]
fn focus_loss_cancels_a_mouse_press_even_if_the_old_picture_is_still_visible() {
    let mut p = Running::start();
    let b = buffer(p.frame.as_ref().unwrap()).unwrap();
    let (x, y) = (0..b.area.height)
        .find_map(|y| {
            (0..b.area.width - 9)
                .find(|x| {
                    (0..9).map(|n| b[(*x + n, y)].symbol()).collect::<String>() == "Close Esc"
                })
                .map(|x| (x, y))
        })
        .unwrap();
    p.input(json!({"type":"mouse","action":"down","button":"left","x":x,"y":y}));
    p.see("Close Esc");
    p.send(Message::event("panel.focus", json!({"focused":false})));
    p.send(Message::event("panel.focus", json!({"focused":true})));
    p.input(json!({"type":"mouse","action":"up","button":"left","x":x,"y":y}));
    p.send(Message::request(2, "ping", json!({})));
    let mut rejected = false;
    loop {
        let m = p.receive();
        assert!(!matches!(&m, Message::Event {name,..} if name == "panel.close_request"));
        rejected |= matches!(&m,Message::Event{name,..} if name == "input.rejected");
        p.record(&m);
        if matches!(m, Message::Response { id: 2, .. }) {
            break;
        }
    }
    assert!(rejected);
}
