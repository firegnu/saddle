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
        Self::start_with_projects(queue_script, false)
    }
    fn start_with_projects(queue_script: &str, registered: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(home.join(".drover")).unwrap();
        if registered {
            let first = dir.path().join("project-one");
            let second = dir.path().join("project-two");
            std::fs::create_dir(&first).unwrap();
            std::fs::create_dir(&second).unwrap();
            std::fs::write(
                home.join(".drover/projects"),
                format!(
                    "{}\n\n{}\n{}\n",
                    first.display(),
                    second.display(),
                    first.display()
                ),
            )
            .unwrap();
        }
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
        cmd.env("HOME", &home);
        cmd.cwd(dir.path());
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
    #[track_caller]
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
    #[track_caller]
    fn see(&mut self, text: &str) {
        self.until(|h| h.screen.screen().contents().contains(text));
    }
    fn click(&mut self, label: &str) {
        let (col, row) = self.press_button(label);
        self.send(format!("\x1b[<0;{};{}m", col + 1, row + 1).as_bytes());
    }
    fn press_button(&mut self, label: &str) -> (u16, u16) {
        self.see(label);
        let screen = self.screen.screen();
        let (rows, cols) = screen.size();
        for row in 0..rows {
            for col in 0..cols {
                if screen.cell(row, col).unwrap().contents().is_empty() {
                    continue;
                }
                let text: String = (col..cols)
                    .map(|x| screen.cell(row, x).unwrap().contents())
                    .collect();
                if text.starts_with(label) {
                    self.send(format!("\x1b[<0;{};{}M", col + 1, row + 1).as_bytes());
                    return (col, row);
                }
            }
        }
        panic!("click target not found: {label}");
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
    h.see("待处理");
    h.master
        .resize(PtySize {
            rows: 44,
            cols: 160,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(44, 160);
    h.event("size p/b 106x41");
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
    h.send(b"\x1b[6~\x1b[6~\x1b[6~\x1b[6~");
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
    let mut h = Harness::start_with_queue(
        "#!/bin/sh\necho 'QUEUE FAILED: /tmp/a-long-project-directory/another-long-directory/.drover.conf missing project'\nexit 2\n",
    );
    h.see("读取失败");
    h.see("missing"); // The full error wraps across rows in a narrow pane.
    h.see("检查 queue.cwd");
    assert!(!h.screen.screen().contents().contains("正在读取队列"));
    h.quit();
}

#[test]
fn queue_project_can_be_corrected_without_restarting_or_initializing_a_repository() {
    let script = format!(
        "#!/usr/bin/env python3\nfrom pathlib import Path\nimport sys\nif Path.cwd().name != 'chosen-project':\n    print('missing project', file=sys.stderr)\n    sys.exit(2)\n{}",
        include_str!("fixtures/drover.py")
    );
    let mut h = Harness::start_with_queue(&script);
    let project = h.dir.path().join("chosen-project");
    std::fs::create_dir(&project).unwrap();
    h.see("missing project");
    h.send(b"\tce");
    h.see("项目目录");
    h.send(b"\x15"); // Ctrl-U replaces the initial directory.
    h.send(format!("\x1b[200~{}\x1b[201~", project.display()).as_bytes());
    h.send(b"\r");
    h.see("Native queue task");
    assert!(!h.screen.screen().contents().contains("missing project"));
    h.send(b"p");
    h.see("Paused");
    h.quit();
    assert!(!project.join(".drover.conf").exists());
    assert!(!h.log("queue-events").contains("init"));
}

#[test]
fn registered_projects_load_by_default_and_mouse_buttons_route_to_the_selected_project() {
    let script = include_str!("fixtures/drover.py")
        .replace("state_file = root /", "state_file = Path.cwd() /")
        .replace(
            "title='Native queue task'",
            "title='Queue ' + Path.cwd().name",
        );
    let mut h = Harness::start_with_projects(&script, true);
    h.see("Queue project-one");
    h.click(" Project c ");
    h.see("选择项目");
    h.click("project-two");
    h.see("Queue project-two");
    h.click(" Pause p ");
    h.see("Paused");
    assert!(!h.dir.path().join("project-one/queue-state.json").exists());
    assert!(h.dir.path().join("project-two/queue-state.json").exists());
    h.click(" Project c ");
    h.click("project-one");
    h.see("Queue project-one");
    h.see("Manual");
    h.quit();
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn native_mouse_buttons_cover_forms_replies_and_stop_confirmation() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.see("Synthetic title");
    h.click(" Reply r ");
    h.see("REPLY p/a");
    h.click(" Stop x ");
    h.click(" Cancel Esc ");
    h.see("cancelled");
    assert!(!h.log("events").contains("stop "));
    h.click(" Details ↵ ");
    h.see("detail line 0");
    h.click(" Back Esc ");
    h.see(" Details ↵ ");
    h.click(" Add a ");
    h.see("Ctrl-S");
    h.send("鼠标新增".as_bytes());
    h.click("正文");
    h.send("正文内容".as_bytes());
    h.click(" Save ^s ");
    h.see("鼠标新增");
    h.until(|h| {
        h.log("queue-events")
            .contains("[\"add\", \"鼠标新增\", \"正文内容\"]")
    });
    h.master
        .resize(PtySize {
            rows: 48,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(48, 80);
    h.see(" Agents  Queue ");
    h.send(b"\x1d");
    h.see("输入 ▸ Agents");
    // Narrow-window tabs expose Agents; hit targets must follow the new rows.
    h.click(" Stop x ");
    h.click(" Stop y ");
    h.event("stop p/a");
    h.quit();
}

#[test]
fn native_queue_help_details_form_and_actions_use_only_public_cli_commands() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.send(b"\t?");
    h.see("Queue 原生看板");
    h.see(" Back Esc ");
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains(" Back Esc "));
    h.see("Native queue task");
    h.send(b"\r");
    h.see("detail line 0");
    h.send(b"\x1b[6~\x1b[6~");
    h.see("detail line 30");
    h.see(" Back Esc ");
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains(" Back Esc "));
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
    h.until(|h| !h.screen.screen().contents().contains(" Back Esc "));
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

#[test]
fn buttons_require_release_on_the_same_target() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.press_button(" Pause p ");
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(
        !h.log("queue-events").contains("[\"pause\"]"),
        "Down must not run a command"
    );
    h.send(b"\x1b[<32;130;4M\x1b[<0;130;4m"); // Drag/release over Viewer cancels, without sending a stray release.
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(
        !h.log("events").contains("input p/a "),
        "A management button gesture must not leak into Viewer"
    );
    h.quit();
}

#[test]
fn overlays_capture_input_and_narrow_tabs_keep_the_viewer_attached() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"\t");
    h.event("input p/a 09"); // Viewer keeps Tab; never changes management focus.
    h.send(b"\x1dx");
    h.see(" Stop y ");
    let input_before = h
        .log("events")
        .lines()
        .filter(|l| l.starts_with("input "))
        .count();
    h.send(b"\x1b[<0;130;4M\x1b[<0;130;4m"); // Behind modal, inside Viewer.
    h.send(b"z"); // Cancels confirmation; must not go to the agent.
    h.see("cancelled");
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|l| l.starts_with("input "))
            .count(),
        input_before
    );
    assert!(!h.log("events").contains("stop "));
    h.send(b"\tc");
    h.see(" Path e ");
    h.send(b"\x1b[<0;130;4M\x1b[<0;130;4m");
    h.send(b"\x1d");
    h.see("输入 ▸ Agents");
    h.master
        .resize(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(24, 80);
    h.see(" Agents  Queue ");
    h.click("Queue");
    h.see(" Path e "); // The suspended project picker resumes.
    h.send(b"\x1b");
    h.see("输入 ▸ Queue");
    h.see("Native queue task");
    h.click(" Pause p ");
    h.see("Paused");
    h.click("Agents");
    h.see("输入 ▸ Agents");
    h.see("Synthetic title");
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|s| s.starts_with("attach "))
            .count(),
        1
    );
    assert!(!h.log("events").contains("detached p/a"));
    h.quit();
}

#[test]
fn delayed_attach_does_not_steal_input_from_an_open_form() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.see("Native queue task");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\r\ta");
    h.see("Ctrl-S");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.see("p/a READY");
    h.send("弹层保持焦点".as_bytes());
    h.send(b"\x13");
    h.until(|h| h.log("queue-events").contains("弹层保持焦点"));
    assert!(!h.log("events").contains("input p/a "));
    h.quit();
}

#[test]
fn stop_in_progress_cannot_be_submitted_twice() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("hold-stop"), "").unwrap();
    h.send(b"xy");
    h.event("stop p/a");
    h.send(b"xy");
    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        h.pump();
    }
    let count = h.log("events").lines().filter(|l| *l == "stop p/a").count();
    std::fs::remove_file(h.dir.path().join("hold-stop")).unwrap();
    assert_eq!(
        count, 1,
        "An in-flight stop must disable keyboard and mouse resubmission"
    );
    h.quit();
}

#[test]
fn returning_to_agents_preserves_the_unsubmitted_queue_draft() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.send(b"\ta");
    h.see("Ctrl-S");
    h.send("未提交的草稿".as_bytes());
    h.see("未提交的草稿");
    h.send(b"\x1d");
    h.see("输入 ▸ Agents");
    h.send(b"\t");
    h.see("Ctrl-S");
    h.see("未提交的草稿");
    h.send(b"\x13");
    h.until(|h| h.log("queue-events").contains("未提交的草稿"));
    h.quit();
}
