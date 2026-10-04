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
        Self::start_with_legacy_arg(false)
    }
    fn start_with_legacy_arg(legacy: bool) -> Self {
        Self::start_prepared(legacy, |_| {})
    }
    /// `prepare` runs after the native project exists and before the plugin starts.
    fn start_prepared(legacy: bool, prepare: impl FnOnce(&std::path::Path)) -> Self {
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
        let dlog = common::script(
            root.path(),
            "dlog",
            "#!/bin/sh\necho forbidden >> \"$(dirname \"$0\")/dlog-calls\"\nprintf '{\"dispatches\":[]}'\n",
        );
        prepare(root.path());
        let mut command = Command::new(env!("CARGO_BIN_EXE_saddle-drover"));
        if legacy {
            command.args(["--dispatch-log", &dlog]);
        }
        let mut child = command
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

#[test]
fn telemetry_link_asks_the_host_for_every_run_of_the_task_and_shows_a_refusal() {
    let mut p = Running::start();
    // The opening frame already shows the selected T1 with its link.
    let b = buffer(p.frame.as_ref().unwrap()).unwrap();
    let label: Vec<String> = "Telemetry ↗".chars().map(String::from).collect();
    let (x, y) = (0..b.area.height)
        .find_map(|y| {
            (0..b.area.width.saturating_sub(label.len() as u16))
                .find(|x| {
                    use unicode_width::UnicodeWidthStr;
                    let mut column = *x;
                    label.iter().all(|symbol| {
                        let same = column < b.area.width && b[(column, y)].symbol() == symbol;
                        column += symbol.width() as u16;
                        same
                    })
                })
                .map(|x| (x, y))
        })
        .expect("Telemetry button");
    p.input(json!({"type":"mouse","action":"down","button":"left","x":x,"y":y}));
    p.input(json!({"type":"mouse","action":"up","button":"left","x":x,"y":y}));
    let end = Instant::now() + Duration::from_secs(5);
    let (id, params) = loop {
        assert!(Instant::now() < end, "no telemetry.open request");
        let m = p.receive();
        if let Message::Request { id, method, params } = &m
            && method == "telemetry.open"
        {
            break (*id, params.clone());
        }
        p.record(&m);
    };
    let scope = p.root.path().canonicalize().unwrap();
    assert_eq!(
        params,
        json!({"input_id":p.input_id,"filter":{"kind":"drover.task","scope":scope,"key":"T1"}}),
        "kind, scope and key only: every run of T1"
    );
    p.send(Message::error(
        id,
        "stale_input",
        "Navigation requires current user input",
    ));
    p.see("Telemetry did not open");
}

#[test]
fn legacy_log_argument_is_ignored_and_no_log_consumer_runs() {
    let mut p = Running::start_with_legacy_arg(true);
    // Visit all current views, including the old third view on the baseline,
    // allowing the normal background tick to run before checking the marker.
    for _ in 0..3 {
        p.input(json!({"type":"key","code":{"name":"tab"},"modifiers":[],"phase":"press"}));
        p.see("Native queue task");
        p.key("r");
        let until = Instant::now() + Duration::from_millis(250);
        while Instant::now() < until {
            if let Ok(m) = p.output.recv_timeout(Duration::from_millis(25)) {
                p.record(&m);
            }
        }
    }
    assert!(
        !p.root.path().join("dlog-calls").exists(),
        "legacy path must never execute"
    );
    let screen: String = p
        .frame
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .flat_map(|row| row.iter())
        .map(|span| span.text.as_str())
        .collect();
    assert!(
        !screen.contains("○ Dispatch") && !screen.contains("● Dispatch"),
        "{screen}"
    );
    assert!(screen.contains("Telemetry ↗"), "{screen}");
    p.key("q");
    loop {
        let m = p.receive();
        if matches!(&m, Message::Event {name, ..} if name == "panel.close_request") {
            break;
        }
        p.record(&m);
    }
}

#[test]
fn project_onboarding_ui_browses_selects_receiver_saves_and_preserves_tasks() {
    let mut p = Running::start();
    p.send(Message::event(
        "panel.resize",
        json!({"cols":80,"rows_count":24,"size_revision":2}),
    ));
    p.see("Native queue task");
    common::script(
        p.root.path(),
        "corral",
        "#!/bin/sh\n[ \"$1\" = ls ] || exit 99\nprintf '%s' '{\"agents\":[{\"name\":\"demo/main\"}]}'\n",
    );
    let repo = p.root.path().join("new-repo");
    std::fs::create_dir(&repo).unwrap();
    std::fs::write(repo.join("AGENTS.md"), "manual rules\n").unwrap();
    let queue = std::fs::read(p.root.path().join("data/queue.md")).unwrap();
    p.key("c");
    p.see("Add project");
    p.key("a");
    p.see("Already added to Tasks");
    assert!(p.frame.as_ref().unwrap().escape_input);
    p.input(json!({"type":"key","code":{"char":"u"},"modifiers":["control"],"phase":"press"}));
    p.see("Choose a directory, then Check");
    p.input(json!({"type":"paste","text":repo}));
    p.see("new-repo");
    p.input(
        json!({"type":"key","code":{"name":"function","number":2},"modifiers":[],"phase":"press"}),
    );
    p.see("Use directory F3");
    p.input(
        json!({"type":"key","code":{"name":"function","number":3},"modifiers":[],"phase":"press"}),
    );
    p.see("Not added to Tasks");
    let preview = buffer(p.frame.as_ref().unwrap()).unwrap();
    let text: String = preview.content.iter().map(|c| c.symbol()).collect();
    for label in [
        "Directory",
        "Project short name",
        "Default receiver",
        "Save ^s",
        "Cancel Esc",
        "AGENTS.md",
    ] {
        assert!(text.contains(label), "missing {label} at 80x24: {text}");
    }
    assert!(!repo.join(".drover.conf").exists());
    p.input(
        json!({"type":"key","code":{"name":"function","number":4},"modifiers":[],"phase":"press"}),
    );
    p.see("demo/main");
    p.input(json!({"type":"key","code":{"name":"down"},"modifiers":[],"phase":"press"}));
    p.see("> demo/main");
    p.input(json!({"type":"key","code":{"name":"enter"},"modifiers":[],"phase":"press"}));
    p.see("Save ^s");
    p.input(json!({"type":"key","code":{"char":"s"},"modifiers":["control"],"phase":"press"}));
    p.see("Project saved; no task dispatched");
    assert!(
        std::fs::read_to_string(repo.join(".drover.conf"))
            .unwrap()
            .contains("MAIN_AGENT=demo/main\n")
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("AGENTS.md")).unwrap(),
        "manual rules\n"
    );
    p.key("c");
    p.see("Added · demo/main");
    p.key("s");
    p.see("Already added to Tasks");
    p.input(
        json!({"type":"key","code":{"name":"function","number":5},"modifiers":[],"phase":"press"}),
    );
    p.see("Not specified (manual handoff)");
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Added · demo/main");
    assert!(
        std::fs::read_to_string(repo.join(".drover.conf"))
            .unwrap()
            .contains("MAIN_AGENT=demo/main\n"),
        "Cancel must not save the draft"
    );
    p.key("s");
    p.see("Already added to Tasks");
    p.input(
        json!({"type":"key","code":{"name":"function","number":5},"modifiers":[],"phase":"press"}),
    );
    p.see("Not specified (manual handoff)");
    p.input(json!({"type":"key","code":{"char":"s"},"modifiers":["control"],"phase":"press"}));
    p.see("Project saved; no task dispatched");
    assert!(
        std::fs::read_to_string(repo.join(".drover.conf"))
            .unwrap()
            .contains("MAIN_AGENT=\n")
    );
    assert_eq!(
        std::fs::read(p.root.path().join("data/queue.md")).unwrap(),
        queue
    );
    assert!(!p.root.path().join("data/tasks.state").exists());
    assert!(!p.root.path().join(".drover/new-repo/tasks.state").exists());
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}
/// The native project becomes a Git repository with a worktree `wt`, an added empty
/// project `other`, and an unregistered repository `lone`.
fn repositories(root: &std::path::Path) {
    git(root, &["init", "-q"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "init"]);
    git(root, &["worktree", "add", "-q", "wt"]);
    let other = root.join("other");
    std::fs::create_dir_all(other.join("data")).unwrap();
    git(&other, &["init", "-q"]);
    std::fs::write(other.join(".drover.conf"), "HANDOFF_DIR=data\n").unwrap();
    std::fs::write(other.join("data/queue.md"), "").unwrap();
    std::fs::write(
        root.join(".drover/projects"),
        format!("{}\n{}\n", root.display(), other.display()),
    )
    .unwrap();
    let lone = root.join("lone");
    std::fs::create_dir(&lone).unwrap();
    git(&lone, &["init", "-q"]);
}
impl Running {
    fn text(&self) -> String {
        let b = buffer(self.frame.as_ref().unwrap()).unwrap();
        b.content.iter().map(|c| c.symbol()).collect()
    }
    /// Keep recording frames for `wait`, so a later result has had time to arrive.
    fn pump_for(&mut self, wait: Duration) {
        let end = Instant::now() + wait;
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            if let Ok(message) = self.output.recv_timeout(left) {
                self.record(&message);
            }
        }
    }
    /// Like `see`, but the frame already shown may hold the text.
    fn see_now(&mut self, text: &str) {
        if !self.text().contains(text) {
            self.see(text);
        }
    }
    fn open_from(&mut self, cwd: &std::path::Path) {
        self.send(Message::event(
            "optional.view_context",
            json!({"cwd": cwd.display().to_string()}),
        ));
    }
}

#[test]
fn opening_from_an_added_project_without_tasks_shows_that_project() {
    let mut p = Running::start_prepared(false, repositories);
    let other = p.root.path().join("other");
    p.open_from(&other);
    p.see_now("other ▾ c");
    p.see_now("Showing other · from ");
    assert!(!p.text().contains("Native queue task"), "{}", p.text());
    // A worktree belongs to its added repository, preferring the same top level.
    let wt = p.root.path().join("wt");
    p.open_from(&wt);
    p.see("Native queue task");
    p.see_now(&format!("Showing {} · from ", p.shown_name()));
    assert!(p.text().contains("/wt "), "{}", p.text());
}

#[test]
fn opening_from_an_unmatched_or_unreadable_source_keeps_the_shown_project_and_says_why() {
    let mut p = Running::start_prepared(false, repositories);
    let lone = p.root.path().join("lone");
    p.open_from(&lone);
    p.see_now("no added project matches");
    assert!(p.text().contains("Native queue task"));
    assert!(!p.text().contains("not a Tasks project"));
    let plain = tempfile::tempdir().unwrap();
    p.open_from(plain.path());
    p.see_now("couldn't match");
    assert!(p.text().contains("Native queue task"));
    assert!(!p.text().contains("no added project matches"));
}

#[test]
fn opening_over_a_draft_or_preferences_keeps_the_page_and_names_the_shown_project() {
    let mut p = Running::start_prepared(false, repositories);
    let other = p.root.path().join("other");
    p.key("a");
    p.see("Save ^s");
    p.input(json!({"type":"paste","text":"kept draft"}));
    p.see("kept draft");
    p.open_from(&other);
    p.see_now("kept your current page");
    p.pump_for(Duration::from_millis(600));
    assert!(p.text().contains("kept draft"));
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Native queue task");
    assert!(!p.text().contains("other ▾ c"));

    p.key("N");
    p.see("Task notifications");
    p.open_from(&other);
    p.see_now("kept your current page");
    p.pump_for(Duration::from_millis(600));
    assert!(p.text().contains("Task notifications"));
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Native queue task");
    assert!(!p.text().contains("other ▾ c"));
}

#[test]
fn input_while_locating_discards_the_result_but_hovering_does_not() {
    let mut p = Running::start_prepared(false, |root| {
        repositories(root);
        common::script(
            root,
            "git",
            "#!/bin/sh\nsleep 1\nexec /usr/bin/git \"$@\"\n",
        );
    });
    let other = p.root.path().join("other");
    p.open_from(&other);
    p.see_now("locating");
    p.key("j");
    p.see_now("not applied");
    p.pump_for(Duration::from_secs(4));
    assert!(p.text().contains("Native queue task"));
    assert!(!p.text().contains("other ▾ c"));

    p.open_from(&other);
    p.see_now("locating");
    p.input(json!({"type":"mouse","action":"move","button":null,"x":5,"y":5,"dx":0,"dy":0,"modifiers":[]}));
    p.see_now("other ▾ c");
}

impl Running {
    /// Close the view and open it again from `cwd` at `cols`×`rows`, as the host does.
    fn reopen_from(&mut self, cwd: &std::path::Path, cols: u16, rows: u16, revision: u64) {
        self.send(Message::event("panel.close", json!({})));
        self.open_from(cwd);
        self.send(Message::event(
            "panel.open",
            json!({"cols":cols,"rows_count":rows,"size_revision":revision}),
        ));
        self.send(Message::event("panel.focus", json!({"focused":true})));
    }
    fn resize(&mut self, cols: u16, rows: u16, revision: u64) {
        self.send(Message::event(
            "panel.resize",
            json!({"cols":cols,"rows_count":rows,"size_revision":revision}),
        ));
    }
    fn first_row(&self) -> String {
        let b = buffer(self.frame.as_ref().unwrap()).unwrap();
        (0..b.area.width).map(|x| b[(x, 0)].symbol()).collect()
    }
    fn shown_name(&self) -> String {
        self.root
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into()
    }
}

#[test]
fn a_kept_draft_names_the_shown_project_in_an_80_by_24_window() {
    // An 80×24 Saddle gives the Tasks overlay a 62×17 inside.
    let mut p = Running::start_prepared(false, repositories);
    p.resize(62, 17, 2);
    p.see("Native queue task");
    p.key("a");
    p.see("Save ^s");
    p.input(json!({"type":"paste","text":"kept draft"}));
    p.see("kept draft");
    let other = p.root.path().join("other");
    p.reopen_from(&other, 62, 17, 3);
    p.see_now("kept your");
    p.pump_for(Duration::from_millis(400));
    let shown = format!("Showing {}", p.shown_name());
    assert!(p.text().contains(&shown), "{shown} missing:\n{}", p.text());
    assert!(p.text().contains("kept your current page"), "{}", p.text());
    assert!(p.text().contains("kept draft"));
}

#[test]
fn reopened_setup_and_preferences_keep_a_source_row_the_form_does_not_cover() {
    // A 120×36 Saddle gives a 94×26 inside, where the setup form is as tall as the view.
    let mut p = Running::start_prepared(false, repositories);
    p.resize(94, 26, 2);
    p.see("Native queue task");
    let other = p.root.path().join("other");
    p.key("c");
    p.see("Status / receiver");
    p.key("a");
    p.see("Add project");
    p.reopen_from(&other, 94, 26, 3);
    p.see_now("Add project");
    p.pump_for(Duration::from_millis(400));
    let shown = format!("Showing {}", p.shown_name());
    assert!(
        p.first_row().contains(&shown) && p.first_row().contains("kept your current page"),
        "{}",
        p.text()
    );
    assert!(p.text().contains("Add project"));
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Status / receiver");
    p.input(json!({"type":"key","code":{"name":"esc"},"modifiers":[],"phase":"press"}));
    p.see("Native queue task");

    // Preferences in the 80×24 inside are centred on the first row unless it is kept.
    p.resize(62, 17, 4);
    p.see("Native queue task");
    p.key("N");
    p.see("Task notifications");
    p.reopen_from(&other, 62, 17, 5);
    p.see_now("Task notifications");
    p.pump_for(Duration::from_millis(400));
    assert!(
        p.first_row().contains(&shown) && p.first_row().contains("kept your current page"),
        "{}",
        p.text()
    );
}

#[test]
fn a_matched_project_whose_queue_cannot_be_read_opens_on_its_read_error() {
    let mut p = Running::start_prepared(false, |root| {
        repositories(root);
        let queue = root.join("other/data/queue.md");
        std::fs::remove_file(&queue).unwrap();
        std::fs::create_dir(&queue).unwrap();
    });
    let other = p.root.path().join("other");
    p.open_from(&other);
    p.see_now("other ▾ c");
    p.see_now("Read failed");
    p.pump_for(Duration::from_millis(400));
    assert!(!p.text().contains("Add project"), "{}", p.text());
    assert!(p.text().contains("Showing other"), "{}", p.text());
    // Nothing was written or repaired.
    assert!(other.join("data/queue.md").is_dir());
    assert!(!other.join("data/tasks.state").exists());
}
