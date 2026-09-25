mod common;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

struct Harness {
    dir: tempfile::TempDir,
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Receiver<Vec<u8>>,
    screen: vt100::Parser,
    cursor_answered: bool,
    raw: Vec<u8>,
}
impl Harness {
    fn start() -> Self {
        Self::start_with_queue(include_str!("fixtures/drover.py"))
    }
    fn start_with_queue(queue_script: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let corral = common::script(dir.path(), "corral", include_str!("fixtures/corral.py"));
        let queue = common::script(dir.path(), "queue", queue_script);
        std::fs::write(
            dir.path().join("agents.json"),
            r#"{"p/a":"idle","p/b":"working","p/taken":"idle"}"#,
        )
        .unwrap();
        let config = dir.path().join("config.toml");
        std::fs::write(
            &config,
            format!("corral = {corral:?}\nrefresh_ms = 100\n[queue]\ndrover = {queue:?}\n"),
        )
        .unwrap();
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 40,
                cols: 140,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_saddle"));
        cmd.args(["--config", config.to_str().unwrap()]);
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            let mut bytes = [0; 16384];
            while let Ok(n) = reader.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                if sender.send(bytes[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self {
            dir,
            child,
            master: pair.master,
            writer,
            output,
            screen: vt100::Parser::new(40, 140, 0),
            cursor_answered: false,
            raw: Vec::new(),
        }
    }
    fn pump(&mut self) {
        if let Ok(bytes) = self.output.recv_timeout(Duration::from_millis(30)) {
            self.screen.process(&bytes);
            self.raw.extend(bytes);
            if !self.cursor_answered && self.raw.windows(4).any(|w| w == b"\x1b[6n") {
                self.send(b"\x1b[1;1R");
                self.cursor_answered = true;
            }
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).unwrap();
    }
    fn log(&self, filename: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(filename)).unwrap_or_default()
    }
    fn until(&mut self, mut predicate: impl FnMut(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while !predicate(self) {
            assert!(
                Instant::now() < deadline,
                "screen:\n{}\nevents:\n{}\nqueue:\n{}",
                self.screen.screen().contents(),
                self.log("events"),
                self.log("queue-events")
            );
            self.pump();
        }
    }
    fn see(&mut self, text: &str) {
        self.until(|h| h.screen.screen().contents().contains(text));
    }
    fn event(&mut self, text: &str) {
        self.until(|h| h.log("events").contains(text));
    }
    fn quit(&mut self) {
        self.send(b"\x1dq");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.pump();
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "saddle failed to exit");
        }
        while let Ok(bytes) = self.output.try_recv() {
            self.raw.extend(bytes);
        }
        assert!(self.raw.windows(8).any(|w| w == b"\x1b[?1049l"));
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.writer.write_all(b"\x1dq");
            let deadline = Instant::now() + Duration::from_secs(5);
            while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                self.pump();
            }
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}

#[test]
fn full_workflow_routes_input_switches_safely_and_survives_disappearance() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.see("Synthetic title");
    assert!(!h.log("events").contains("reply "));
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"q");
    h.event("input p/a 71");
    h.send("\x1b[200~中文\nhello\x1b[201~".as_bytes());
    h.event("1b5b3230307ee4b8ade696870a68656c6c6f1b5b3230317e");
    h.send(b"\x1b[<0;56;4M");
    h.event("input p/a 1b5b3c303b333b334d");
    h.send(b"\x1dj\r");
    h.see("p/b READY");
    let log = h.log("events");
    assert!(log.find("detached p/a").unwrap() < log.find("attach p/b").unwrap());
    assert!(h.dir.path().join("p-b").exists());
    assert!(!h.dir.path().join("p-a").exists());
    // A new listing item appears without restarting; changing status is visible.
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"blocked","p/b":"working","p/new":"idle","p/taken":"idle"}"#,
    )
    .unwrap();
    h.see("blocked");
    h.master
        .resize(PtySize {
            rows: 44,
            cols: 160,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(44, 160);
    h.event("size p/b 106x42");
    h.until(|h| h.screen.screen().cell(22, 0).unwrap().contents() == "┌");
    h.see("Native queue task");
    // Native Queue translates actions into public CLI calls, never a PTY.
    h.send(b"\x1d\tp");
    h.until(|h| h.log("queue-events").contains("[\"pause\"]"));
    h.see("Paused");
    // Agent disappearance returns to the prompt, without selecting another viewer.
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"blocked","p/new":"idle","p/taken":"idle"}"#,
    )
    .unwrap();
    h.see("attach 已退出");
    h.event("detached p/b");
    h.send(b"\x1dr");
    h.see("REPLY p/a");
    h.send(b"\x1b[6~");
    h.see("line 15");
    h.send(b"rxq"); // cancel stop with q; cancellation must not quit.
    h.see("cancelled");
    assert!(!h.log("events").contains("stop "));
    h.send(b"jj\r");
    h.see("attached elsewhere");
    assert!(!h.log("events").contains("attach p/taken"));
    // Explicit x/y stops only the selected synthetic agent.
    h.send(b"kkxy");
    h.event("stop p/a");
    h.quit();
    let log = h.log("events");
    assert_eq!(log.lines().filter(|s| s.starts_with("attach ")).count(), 2);
    assert_eq!(
        log.lines()
            .filter(|s| s.starts_with("stop "))
            .collect::<Vec<_>>(),
        ["stop p/a"]
    );
    let agents: serde_json::Value = serde_json::from_str(&h.log("agents.json")).unwrap();
    assert!(agents.get("p/new").is_some());
    assert!(agents.get("p/taken").is_some());
    assert!(!h.log("queue-events").contains("board"));
}

#[test]
fn mouse_selection_attaches_and_quit_remains_responsive_during_output_flood() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.see("Synthetic title");
    // One-based SGR coordinates: first agent headline is screen row 3.
    h.send(b"\x1b[<0;5;3M");
    h.see("p/a READY");
    h.send(b"F");
    h.event("flood p/a");
    let start = Instant::now();
    h.quit();
    assert!(start.elapsed() < Duration::from_secs(3));
    let log = h.log("events");
    assert!(log.contains("detached p/a"));
    assert!(!log.contains("stop "));
    assert!(!h.log("queue-events").contains("board"));
    assert!(!h.dir.path().join("p-a").exists());
    let agents: serde_json::Value = serde_json::from_str(&h.log("agents.json")).unwrap();
    assert_eq!(agents.as_object().unwrap().len(), 3);
}

#[test]
fn failed_queue_data_request_keeps_actionable_error_visible() {
    let mut h =
        Harness::start_with_queue("#!/bin/sh\necho 'QUEUE FAILED: missing project'\nexit 2\n");
    h.see("QUEUE FAILED: missing project");
    h.see("检查 queue.cwd");
    h.quit();
}

#[test]
fn native_queue_help_details_form_and_actions_use_only_public_cli_commands() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.send(b"\t?");
    h.see("Queue 原生看板");
    h.send(b"\x1b");
    h.see("Native queue task");
    h.send(b"\r");
    h.see("detail line 0");
    h.send(b"\x1b[6~");
    h.see("detail line 15");
    h.send(b"\x1b");
    h.see("Native queue task");
    h.send(b"a");
    h.see("Ctrl-S");
    h.send("\x1b[200~新增任务\x1b[201~".as_bytes());
    h.send(b"\t");
    h.send("\x1b[200~正文一\n正文二\x1b[201~".as_bytes());
    h.send(b"\x13");
    h.until(|h| !h.screen.screen().contents().contains("Ctrl-S"));
    h.see("新增任务");
    h.until(|h| {
        h.log("queue-events")
            .contains("[\"add\", \"新增任务\", \"正文一\\n正文二\"]")
    });
    h.send(b"p");
    h.see("Paused");
    h.send(b"p");
    h.see("Manual");
    h.send(b"l");
    h.see("loop on");
    h.send(b"g");
    h.see("checked public criteria");
    h.send(b"\x1b");
    h.see("Native queue task");
    h.send(b"n");
    h.see("next request accepted");
    h.quit();
    assert!(!h.log("queue-events").contains("board"));
    assert!(!h.log("events").contains("attach "));
}

#[test]
#[ignore = "requires installed drover CLI; no external UI, real queues, or agents"]
fn installed_drover_cli_drives_the_native_queue_in_an_isolated_project() {
    use std::process::Command;
    let drover = std::env::var("SADDLE_DROVER_BIN").expect("set SADDLE_DROVER_BIN");
    let sandbox = tempfile::tempdir().unwrap();
    let home = sandbox.path().join("home");
    let repo = sandbox.path().join("repo");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&repo).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );
    let cli = |args: &[&str]| {
        let out = Command::new(&drover)
            .args(args)
            .env("HOME", &home)
            .current_dir(&repo)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    cli(&["init", "saddle-native-acceptance"]);
    cli(&["add", "Synthetic native task", "隔离验收正文"]);
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let wrapper = format!(
        "#!/bin/sh\nexport HOME={}\ncd {}\nexec {} \"$@\"\n",
        quote(home.to_str().unwrap()),
        quote(repo.to_str().unwrap()),
        quote(&drover)
    );
    let mut h = Harness::start_with_queue(&wrapper);
    h.see("Synthetic native task");
    h.send(b"\t\r");
    h.see("隔离验收正文");
    h.send(b"p");
    h.see("Paused");
    let state: serde_json::Value = serde_json::from_str(&cli(&["list", "--json"])).unwrap();
    assert_eq!(state["paused"], true);
    h.send(b"p");
    h.see("Manual");
    h.send(b"l");
    h.see("loop on");
    h.send(b"l");
    h.see("loop off");
    h.send(b"a");
    h.see("Ctrl-S");
    h.send("\x1b[200~原生新增\x1b[201~".as_bytes());
    h.send(b"\t");
    h.send("\x1b[200~第一行\n第二行\x1b[201~".as_bytes());
    h.send(b"\x13");
    h.until(|h| !h.screen.screen().contents().contains("Ctrl-S"));
    h.see("原生新增");
    let state: serde_json::Value = serde_json::from_str(&cli(&["list", "--json"])).unwrap();
    assert_eq!(state["pending"][1]["body"], "第一行\n第二行");
    h.master
        .resize(PtySize {
            rows: 50,
            cols: 180,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(50, 180);
    h.until(|h| h.screen.screen().cell(25, 0).unwrap().contents() == "┌");
    h.see("原生新增");
    h.quit();
    assert!(!h.log("events").contains("attach "));
}
