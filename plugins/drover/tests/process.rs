//! Exercise the actual executable and SDK wire path with an isolated HOME and native task data.
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
        std::fs::create_dir(root.path().join("data")).unwrap();
        std::fs::write(root.path().join(".drover.conf"), "HANDOFF_DIR=data\n").unwrap();
        std::fs::write(
            root.path().join("data/queue.md"),
            "## T1 Native queue task\nOriginal body\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join(".drover/notifications.json"),
            r#"{"schema_version":1,"system_enabled":false,"revision":1}"#,
        )
        .unwrap();
        common::script(
            root.path(),
            "drover",
            "#!/bin/sh\necho forbidden >> \"$(dirname \"$0\")/old-cli-calls\"\nexit 99\n",
        );
        common::script(
            root.path(),
            "osascript",
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$(dirname \"$0\")/system-notifications\"\n",
        );
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
    fn api(&mut self, method: &str, mut params: serde_json::Value) -> serde_json::Value {
        params["project"] = json!(self.root.path());
        self.send(Message::event(
            "command.invoke",
            json!({"id":"request","method":method,"params":params}),
        ));
        loop {
            let m = self.receive();
            self.record(&m);
            if let Message::Event { name, data } = m
                && name == "command.result"
            {
                return data["result"].clone();
            }
        }
    }
    fn key(&mut self, c: &str) {
        self.input(json!({"type":"key","code":{"char":c},"modifiers":[],"phase":"press"}));
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self
            .input
            .write_all(&protocol::encode(&Message::request(100, "shutdown", json!({}))).unwrap());
        let _ = self.input.flush();
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
    assert_eq!(
        std::fs::read_to_string(p.root.path().join("data/queue.md")).unwrap(),
        "## T1 Native queue task\nOriginal body\n"
    );
    assert!(!p.root.path().join("data/tasks.state").exists());
    assert!(!p.root.path().join("old-cli-calls").exists());
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
    std::fs::create_dir(p.root.path().join(".drover/notifications.json.lock")).unwrap();
    p.input(json!({"type":"key","code":{"char":"s"},"modifiers":["control"],"phase":"press"}));
    p.see("Not saved:");
    assert!(!p.root.path().join("system-notifications").exists());
    std::fs::remove_dir(p.root.path().join(".drover/notifications.json.lock")).unwrap();
    p.input(json!({"type":"key","code":{"char":"s"},"modifiers":["control"],"phase":"press"}));
    p.see("Saved.");
    let preference: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(p.root.path().join(".drover/notifications.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(preference["system_enabled"], true);
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
    let start = json!({"ev":"start","id":"T9","title":"Review plugin task","body":"x".repeat(20000),"key":"Review plugin task","run_id":"r9","t":1.0});
    let submit = json!({"ev":"submitted","id":"T9","run_id":"r9","t":2.0});
    let events = format!("{start}\n{submit}\n");
    std::fs::write(p.root.path().join("data/tasks.state"), &events).unwrap();
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
    assert!(attention);
    assert_eq!(notifications, 1);
    assert!(p.child.try_wait().unwrap().is_none());
    assert_eq!(
        std::fs::read_to_string(p.root.path().join("data/tasks.state")).unwrap(),
        events
    );
    assert!(!p.root.path().join("old-cli-calls").exists());
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

#[test]
fn plugin_commands_own_the_complete_flow_and_system_notification_channel() {
    let mut p = Running::start();
    let initial = p.api("list", json!({}));
    assert_eq!(initial["pending"][0]["id"], "T1");
    let started=p.api("dispatch",json!({"pos":1,"target_token":initial["pending"][0]["actions"]["dispatch-pending"]["target_token"]}));
    assert_eq!(started["state"], "running");
    let current = p.api("list", json!({}));
    let submitted = p.api(
        "submit",
        json!({"id":"T1","target_token":current["current"]["actions"]["done"]["target_token"]}),
    );
    assert_eq!(submitted["state"], "awaiting_release");
    let waiting = p.api("list", json!({}));
    assert!(waiting["awaiting"]["notification_key"].is_string());
    let accepted = p.api(
        "accept",
        json!({"id":"T1","target_token":waiting["awaiting"]["actions"]["go"]["target_token"]}),
    );
    assert_eq!(accepted["state"], "done");
    p.api("notifications", json!({"system_enabled":true}));
    // Let the observer read the new channel and establish its baseline before a new run.
    let until = Instant::now() + Duration::from_millis(800);
    while Instant::now() < until {
        if let Ok(m) = p.output.recv_timeout(Duration::from_millis(50)) {
            p.record(&m);
        }
    }
    assert!(!p.root.path().join("system-notifications").exists());
    p.api("add", json!({"title":"System channel"}));
    let next = p.api("list", json!({}));
    p.api("dispatch",json!({"pos":1,"target_token":next["pending"][0]["actions"]["dispatch-pending"]["target_token"]}));
    let current = p.api("list", json!({}));
    p.api(
        "submit",
        json!({"id":"T2","target_token":current["current"]["actions"]["done"]["target_token"]}),
    );
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        if let Ok(m) = p.output.recv_timeout(Duration::from_millis(50)) {
            if let Message::Request { method, .. } = &m {
                assert_ne!(
                    method, "notify",
                    "System mode cannot also create an internal toast"
                );
            }
            p.record(&m);
        }
    }
    let sent = std::fs::read_to_string(p.root.path().join("system-notifications")).unwrap();
    assert_eq!(sent.matches("T2 ready for review").count(), 1);
    assert!(!p.root.path().join("old-cli-calls").exists());
}

#[test]
fn a_second_plugin_cannot_own_the_same_user_data() {
    let mut first = Running::start();
    let queue = std::fs::read(first.root.path().join("data/queue.md")).unwrap();
    let mut second = Command::new(env!("CARGO_BIN_EXE_saddle-drover"))
        .current_dir(first.root.path())
        .env("HOME", first.root.path())
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    second.stdin.as_mut().unwrap().write_all(&protocol::encode(&Message::request(1,"initialize",json!({"protocol_major":1,"width_profile":protocol::PROFILE,"capabilities":protocol::CAPABILITIES,"theme":{}}))).unwrap()).unwrap();
    drop(second.stdin.take());
    let output = second.wait_with_output().unwrap();
    assert!(!output.status.success(), "second owner must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("already owned"));
    assert_eq!(
        std::fs::read(first.root.path().join("data/queue.md")).unwrap(),
        queue
    );
    let result = first.api("list", json!({}));
    assert_eq!(result["pending_total"], 1);
}
