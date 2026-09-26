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
        Self::start_with_config(queue_script, registered, "")
    }
    fn start_with_config(queue_script: &str, registered: bool, extra: &str) -> Self {
        Self::start_with_read_chunk(queue_script, registered, extra, 16384)
    }
    fn start_with_read_chunk(
        queue_script: &str,
        registered: bool,
        extra: &str,
        read_chunk: usize,
    ) -> Self {
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
            format!("corral = {corral:?}\nrefresh_ms = 100\n[queue]\ndrover = {queue:?}\n{extra}"),
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
        cmd.env("NO_COLOR", "");
        cmd.env("HOME", &home);
        cmd.cwd(dir.path());
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            let mut bytes = vec![0; read_chunk];
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
                if screen.cell(row, col).unwrap().is_wide_continuation() {
                    continue;
                }
                let text: String = (col..cols)
                    .filter_map(|x| {
                        let cell = screen.cell(row, x).unwrap();
                        if cell.is_wide_continuation() {
                            None
                        } else if cell.contents().is_empty() {
                            Some(" ")
                        } else {
                            Some(cell.contents())
                        }
                    })
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
    h.event("input p/a 1b5b3c303b333b324d");
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
    h.event("size p/b 106x40");
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
    h.see("attach exited");
    h.event("detached p/b");
    h.send(b"\x1dr");
    h.send(b"xq"); // cancel stop with q; cancellation must not quit.
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
    h.see("Read failed");
    h.see("missing"); // The full error wraps across rows in a narrow pane.
    h.see("Check queue.cwd");
    assert!(!h.screen.screen().contents().contains("Loading tasks"));
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
    h.see("Project path");
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
    h.click("Project c");
    h.see("Projects");
    h.click("project-two");
    h.see("Queue project-two");
    h.click("Pause p");
    h.see("Paused");
    assert!(!h.dir.path().join("project-one/queue-state.json").exists());
    assert!(h.dir.path().join("project-two/queue-state.json").exists());
    h.click("Project c");
    h.click("project-one");
    h.see("Queue project-one");
    h.see("Manual");
    h.quit();
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn native_mouse_buttons_cover_forms_and_stop_confirmation() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.see("Synthetic title");
    h.click("Stop x");
    h.click("Cancel Esc");
    h.see("cancelled");
    assert!(!h.log("events").contains("stop "));
    h.click("Details ↵");
    h.see("detail line 0");
    h.click("Back Esc");
    h.see("Details ↵");
    h.click("Add a");
    h.see("Ctrl-S");
    h.send("鼠标新增".as_bytes());
    h.click("Body");
    h.send("正文内容".as_bytes());
    h.click("Save ^s");
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
    h.see("Input ▸ Agents");
    // Narrow-window tabs expose Agents; hit targets must follow the new rows.
    h.click("Stop x");
    h.click(" Stop y ");
    h.event("stop p/a");
    h.quit();
}

#[test]
fn native_queue_help_details_form_and_actions_use_only_public_cli_commands() {
    let mut h = Harness::start();
    h.see("Native queue task");
    h.send(b"\t?");
    h.see("Queue help");
    h.see("Back Esc");
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains("Back Esc"));
    h.see("Native queue task");
    h.send(b"\r");
    h.see("detail line 0");
    h.send(b"\x1b[6~\x1b[6~");
    // PgDn pages through the details inside the Tasks area.
    h.see("detail line 20");
    assert!(!h.screen.screen().contents().contains("detail line 0"));
    h.see("Back Esc");
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains("Back Esc"));
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
    h.see("Ready");
    h.send(b"l");
    h.see("Loop on");
    h.send(b"g");
    h.see("checked public criteria");
    h.see("Input ▸ Queue · Details / Result");
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains("Back Esc"));
    h.see("Native queue task");
    h.send(b"n");
    h.see("next request accepted");
    h.quit();
    assert!(!h.log("queue-events").contains("board"));
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn pending_edit_and_move_buttons_preserve_draft_focus_and_selection() {
    // PTY reads may split a redraw while the old form's text is still on screen.
    let mut h = Harness::start_with_read_chunk(include_str!("fixtures/drover.py"), false, "", 64);
    h.see("Native queue task");
    h.send(b"\ta");
    h.see("Add task");
    h.send(b"Second\tBody\x13");
    h.until(|h| !h.screen.screen().contents().contains("Add task"));
    h.see("Second");
    // The form title can disappear before its fields. Wait for the actual list row.
    h.see("T2 Second");
    h.send(b"j");
    h.see("▎T2 Second");
    h.click("Edit e");
    h.see("Edit task");
    h.see("Second");
    h.send(b" q\t\rExtra");
    h.see("Extra");
    h.send(b"\x1d");
    h.see("Click Queue to resume");
    h.send(b"\t");
    h.see("Edit task");
    h.see("Extra");
    std::fs::write(h.dir.path().join("write-error"), "synthetic write refused").unwrap();
    h.click("Save ^s");
    h.see("synthetic write refused");
    h.see("Second q");
    std::fs::remove_file(h.dir.path().join("write-error")).unwrap();
    h.click("Save ^s");
    h.until(|h| !h.screen.screen().contents().contains("Edit task"));
    h.see("Second q");
    h.see("▎T2 Second q");
    h.click("Move up u");
    h.until(|h| h.log("queue-events").contains("[\"move\", \"2\", \"1\"]"));
    // Wait for the refreshed task order before issuing the opposite movement.
    h.until(|h| {
        let text = h.screen.screen().contents();
        text.find("Second q")
            .zip(text.find("Native queue task"))
            .is_some_and(|(a, b)| a < b)
    });
    h.click("Move down d");
    h.until(|h| h.log("queue-events").contains("[\"move\", \"1\", \"2\"]"));
    h.until(|h| {
        let text = h.screen.screen().contents();
        text.find("Second q")
            .zip(text.find("Native queue task"))
            .is_some_and(|(a, b)| a > b)
    });
    h.click("Edit e");
    h.see("Second q");
    h.see("Extra");
    h.click("Cancel Esc");
    h.until(|h| !h.screen.screen().contents().contains("Edit task"));
    h.quit();
    assert!(
        h.log("queue-events")
            .contains("[\"edit\", \"2\", \"Second q\", \"Body\\nExtra\"]")
    );
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn pending_delete_button_confirms_names_the_task_and_can_be_cancelled() {
    let mut h = Harness::start();
    h.see("T1 Native queue task");
    h.send(b"\t");
    h.click("Delete x");
    h.see("Delete task");
    h.see("detail line 0");
    h.see("History as Dropped");
    h.click("Cancel Esc");
    h.until(|h| !h.screen.screen().contents().contains("Delete task"));
    h.see("T1 Native queue task");
    h.send(b"x");
    h.see("Delete task");
    h.click("Delete y");
    h.until(|h| {
        h.log("queue-events")
            .contains("[\"drop\", \"--pos\", \"1\", \"Deleted in saddle\"]")
    });
    h.until(|h| !h.screen.screen().contents().contains("Delete task"));
    h.see("Dropped");
    h.see("No active tasks");
    h.quit();
    assert_eq!(h.log("queue-events").matches("\"drop\"").count(), 1);
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
    h.see("Ready");
    h.send(b"l");
    h.see("Loop on");
    h.send(b"l");
    h.see("Loop off");
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
    h.press_button("Pause p");
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
    h.see("Path e");
    h.send(b"\x1b[<0;130;4M\x1b[<0;130;4m");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
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
    h.see("Path e"); // The suspended project picker resumes.
    h.send(b"\x1b");
    h.see("Input ▸ Queue");
    h.see("Native queue task");
    h.click("Pause p");
    h.see("Paused");
    h.click("Agents");
    h.see("Input ▸ Agents");
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
    h.see("Input ▸ Agents");
    h.send(b"\t");
    h.see("Ctrl-S");
    h.see("未提交的草稿");
    h.send(b"\x13");
    h.until(|h| h.log("queue-events").contains("未提交的草稿"));
    h.quit();
}

#[test]
fn agents_reply_entry_is_hidden_and_r_does_not_open_it() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"rs");
    h.see("Name s"); // The next key confirms the preceding r was processed.
    let screen = h.screen.screen().contents();
    assert!(!screen.contains("Last reply"), "{screen}");
    assert!(
        !screen.contains("Reply r") && !screen.contains("Hide r"),
        "{screen}"
    );
    assert!(!h.log("events").contains("reply "));
    h.quit();
}

#[test]
fn wheel_over_agents_scrollbar_reaches_last_agent_without_attaching() {
    let mut h = Harness::start();
    let agents: serde_json::Map<String, serde_json::Value> = (0..8)
        .map(|i| (format!("p/worker-{i:02}"), serde_json::json!("idle")))
        .collect();
    std::fs::write(
        h.dir.path().join("agents.json"),
        serde_json::to_vec(&agents).unwrap(),
    )
    .unwrap();
    h.see("Agents · 8");
    // Column 50 is the scrollbar, outside the text list but inside Agents.
    h.send("\x1b[<65;51;4M".repeat(60).as_bytes());
    h.see("worker-07");
    let refreshes = h
        .log("events")
        .lines()
        .filter(|line| line.starts_with("ls "))
        .count();
    h.until(|h| {
        h.log("events")
            .lines()
            .filter(|line| line.starts_with("ls "))
            .count()
            > refreshes + 1
    });
    assert!(h.screen.screen().contents().contains("worker-07"));
    h.send("\x1b[<64;51;4M".repeat(60).as_bytes());
    h.see("worker-00");
    h.send("\x1b[<65;12;4M".repeat(60).as_bytes());
    h.see("worker-07");
    assert!(!h.log("events").contains("attach "));
    h.quit();
}

#[test]
fn mouse_wheel_scrolls_queue_history_immediately_and_reaches_both_ends() {
    let mut h = Harness::start();
    let history: Vec<_> = (0..40).map(|i| serde_json::json!({"id":format!("H{i}"),"title":format!("History-{i:02}"),"status":"done"})).collect();
    std::fs::write(
        h.dir.path().join("queue-state.json"),
        serde_json::to_vec(&serde_json::json!({
            "mode":{"gate":true,"loop":false},"paused":false,"pending":[],"history":history
        }))
        .unwrap(),
    )
    .unwrap();
    h.see("History-39");
    // Wheel over the task list, while keyboard focus remains in Agents.
    h.send("\x1b[<65;12;32M".repeat(4).as_bytes());
    h.send(b"s");
    h.see("Name s"); // Barrier: all four wheel events have been processed.
    assert!(
        !h.screen.screen().contents().contains("History-39"),
        "{}",
        h.screen.screen().contents()
    );
    h.send("\x1b[<65;51;32M".repeat(80).as_bytes());
    h.see("History-00");
    let refreshes = h.log("queue-events").lines().count();
    h.until(|h| h.log("queue-events").lines().count() > refreshes + 1);
    assert!(h.screen.screen().contents().contains("History-00"));
    h.send("\x1b[<64;12;32M".repeat(80).as_bytes());
    h.see("History-39");
    h.quit();
    assert!(
        h.log("queue-events")
            .lines()
            .all(|line| line == "[\"list\", \"--json\"]")
    );
}

#[test]
#[ignore = "requires drover CLI; synthetic history and fake agents only"]
fn installed_drover_complete_history_reaches_saddle_and_scrolls_both_ends() {
    use std::process::Command;
    let drover = std::env::var("SADDLE_DROVER_BIN").expect("set SADDLE_DROVER_BIN");
    let sandbox = tempfile::tempdir().unwrap();
    let repo = sandbox.path().join("repo");
    let data = sandbox.path().join("data");
    for dir in [&repo, &data] {
        std::fs::create_dir(dir).unwrap();
    }
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(
        repo.join(".drover.conf"),
        format!("HANDOFF_DIR={}\nTASK_GATE=1\n", data.display()),
    )
    .unwrap();
    let records: String = (1..=40).map(|i| {
        format!("{}\n", serde_json::json!({"ev":"drop", "id":format!("T{i}"), "title":format!("Complete-history-{i:02}"), "reason":"Synthetic", "t":i}))
    }).collect();
    let state = data.join("tasks.state");
    std::fs::write(&state, &records).unwrap();
    let fake_corral = common::script(sandbox.path(), "corral", "#!/bin/sh\nexit 99\n");
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let wrapper = format!(
        "#!/bin/sh\n[ \"$#\" = 2 ] && [ \"$1\" = list ] && [ \"$2\" = --json ] || exit 98\nexport DROVER_CORRAL_BIN={}\ncd {}\nexec {} \"$@\"\n",
        quote(&fake_corral),
        quote(repo.to_str().unwrap()),
        quote(&drover)
    );
    let mut h = Harness::start_with_queue(&wrapper);
    h.see("40 tasks");
    h.see("Complete-history-01"); // Older than the former ten-record API limit.
    h.send("\x1b[<65;12;32M".repeat(5).as_bytes());
    h.send(b"s");
    h.see("Name s");
    assert!(!h.screen.screen().contents().contains("Complete-history-01"));
    h.send("\x1b[<65;51;32M".repeat(80).as_bytes());
    h.see("Complete-history-40");
    h.see("/40 · End");
    h.send("\x1b[<64;12;32M".repeat(80).as_bytes());
    h.see("Complete-history-01");
    h.quit();
    assert_eq!(std::fs::read_to_string(&state).unwrap(), records);
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn startup_colors_reach_agents_queue_controls_and_leave_viewer_colors_alone() {
    use vt100::Color;
    let mut h = Harness::start_with_config(
        include_str!("fixtures/drover.py"),
        false,
        r##"
[colors]
bg = "#102030"
focus = "#112233"
agent_idle = "#445566"
claude = "#778899"
overlay = "#203040"
text = "#abcdef"
"##,
    );
    h.see("Synthetic title");
    h.see("Native queue task");
    let label_cell = |h: &Harness, label: &str| {
        for y in 0..40 {
            for x in 0..140 {
                let row: String = (x..140)
                    .map(|col| h.screen.screen().cell(y, col).unwrap().contents())
                    .collect();
                if row.starts_with(label) {
                    return h.screen.screen().cell(y, x).unwrap().clone();
                }
            }
        }
        panic!("missing {label}");
    };
    assert_eq!(
        h.screen.screen().cell(0, 0).unwrap().fgcolor(),
        Color::Rgb(0x11, 0x22, 0x33)
    );
    assert_eq!(
        label_cell(&h, "idle").fgcolor(),
        Color::Rgb(0x44, 0x55, 0x66)
    );
    assert_eq!(
        label_cell(&h, "Ready").fgcolor(),
        Color::Rgb(0x44, 0x55, 0x66)
    );
    assert_eq!(
        label_cell(&h, "claude").fgcolor(),
        Color::Rgb(0x77, 0x88, 0x99)
    );
    assert_eq!(
        label_cell(&h, "Go g").fgcolor(),
        Color::Rgb(0x11, 0x22, 0x33)
    );
    assert_eq!(
        label_cell(&h, "Go g").bgcolor(),
        Color::Rgb(0x10, 0x20, 0x30)
    );
    h.send(b"\tc");
    h.see("Path e");
    assert_eq!(
        label_cell(&h, "Projects").bgcolor(),
        Color::Rgb(0x20, 0x30, 0x40)
    );
    h.send(b"\x1b\x1d\r");
    h.see("p/a READY");
    let viewer = label_cell(&h, "p/a READY");
    assert_eq!(viewer.fgcolor(), Color::Default);
    assert_eq!(viewer.bgcolor(), Color::Default);
    h.send(b"C");
    h.see("AGENT COLORS");
    let colored = label_cell(&h, "AGENT COLORS");
    assert_eq!(colored.fgcolor(), Color::Idx(1));
    assert_eq!(colored.bgcolor(), Color::Rgb(9, 8, 7));
    h.quit();
}

#[test]
fn all_pending_button_lists_every_registered_project_and_reports_read_failures() {
    let script = include_str!("fixtures/drover.py")
        .replace("state_file = root /", "state_file = Path.cwd() /")
        .replace(
            "title='Native queue task'",
            "title='Queue ' + Path.cwd().name",
        )
        .replace(
            "if args == ['list', '--json']:",
            "if Path('fail-list').exists():\n    print('synthetic project read failure', file=sys.stderr)\n    sys.exit(4)\nif args == ['list', '--json']:",
        );
    let mut h = Harness::start_with_projects(&script, true);
    h.see("Queue project-one");
    std::fs::write(h.dir.path().join("project-two/fail-list"), "").unwrap();
    h.click("All pending A");
    h.see("All pending ━");
    h.see("synthetic project read failure");
    h.see("Read failed");
    h.see("1 T1 Queue project-one");
    h.see("project-two");
    std::fs::remove_file(h.dir.path().join("project-two/fail-list")).unwrap();
    h.click("Refresh r");
    h.see("1 T1 Queue project-two");
    h.until(|h| !h.screen.screen().contents().contains("Read failed"));
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains("All pending ━"));
    h.see("Queue project-one");
    h.quit();
    let events = h.log("queue-events");
    assert!(
        events.lines().all(|line| line == "[\"list\", \"--json\"]"),
        "{events}"
    );
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn task_detail_mouse_edit_saves_and_returns_through_the_task_overlay() {
    let mut h = Harness::start();
    h.see("T1 Native queue task");
    h.click("Native queue task");
    h.see("Task t");
    h.click("Task t");
    h.see("Input ▸ Queue · Task text");
    h.see("detail line 0");
    h.click("Edit e");
    h.see("Edit task");
    h.send(b" revised\x13");
    h.until(|h| !h.screen.screen().contents().contains("Edit task"));
    h.see("Input ▸ Queue · Task text");
    h.see("Native queue task revised");
    h.click("Back Esc");
    h.see("Input ▸ Queue · Task details");
    h.click("Edit e");
    h.see("Edit task");
    h.click("Cancel Esc");
    h.see("Input ▸ Queue · Task details");
    h.click("Back Esc");
    h.see("Details ↵");
    h.see("T1 Native queue task revised");
    h.quit();
    let events = h.log("queue-events");
    assert_eq!(events.matches("[\"edit\"").count(), 1, "{events}");
    assert!(!h.log("events").contains("input p/a"));
}

#[test]
fn clicking_a_task_opens_refreshing_details_in_the_tasks_area_until_back() {
    let show = include_str!("fixtures/show.json").replace('\n', " ");
    let script = format!(
        r#"#!/usr/bin/env python3
import json, sys
from pathlib import Path
root = Path(__file__).parent
args = sys.argv[1:]
with (root / 'queue-events').open('a') as f:
    f.write(json.dumps(args) + '\n')
if args == ['list', '--json']:
    print(json.dumps(dict(mode=dict(loop=False, gate=True), paused=False, awaiting=None,
        current=dict(id='T4', title='Detail target 任务', body='list body'),
        pending=[dict(id='T5', title='Queued next', body='')],
        history=[dict(id='T3', title='Older done', status='done')])))
elif args == ['show', 'T4', '--json', '--with-agent-status']:
    print({show:?})
else:
    print('FORBIDDEN CLI: ' + repr(args), file=sys.stderr)
    sys.exit(99)
"#
    );
    let mut h = Harness::start_with_queue(&script);
    h.see("Synthetic title");
    h.see("Detail target 任务");
    h.send(b"\r");
    h.see("p/a READY");
    let shows = |h: &Harness| h.log("queue-events").matches("\"show\"").count();
    h.click("Detail target");
    h.see("Completion checks");
    h.see("Back Esc");
    h.see("Viewer · p/a");
    h.see("Input ▸ Queue");
    assert_eq!(shows(&h), 1);
    // Detail keys stay in Tasks while an agent is attached in Viewer.
    h.send(b"jk\x1b[6~\x1b[5~gnpla");
    h.click("Task t");
    h.see("Input ▸ Queue · Task text");
    h.see("list body");
    h.until(|h| shows(h) >= 2); // About five seconds later, one at a time.
    h.send(b"e\x1b"); // Running task stays read-only; Esc returns to its detail pane.
    h.see("Input ▸ Queue · Task details");
    assert!(!h.log("events").contains("input p/a"));
    let queue = h.log("queue-events");
    assert!(
        queue.lines().all(|l| l == r#"["list", "--json"]"#
            || l == r#"["show", "T4", "--json", "--with-agent-status"]"#),
        "{queue}"
    );
    h.send(b"\x1b");
    h.see("Details ↵");
    h.until(|h| !h.screen.screen().contents().contains("Completion checks"));
    let after_back = shows(&h);
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        h.pump();
    }
    assert_eq!(shows(&h), after_back, "returning to the list stops details");
    h.quit();
}

#[test]
fn show_cancel_and_escape_never_attach_and_new_cancel_keeps_the_draft() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Native queue task");
    h.see("Synthetic title");
    h.send(b"n");
    h.click("Name");
    h.send(b"\x15review-draft");
    h.see("review-draft");
    h.click("Project:");
    h.see("Choose project");
    h.click("Back Esc");
    h.see("review-draft");
    h.click("Cancel Esc");
    h.see("Input ▸ Agents");
    h.send(b"n");
    h.see("review-draft");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    for cancel in [b"\x1b".as_slice(), b"", b"\x1d"] {
        h.click("‹Show in… o›"); // Match the button, not the status-bar hint.
        h.see("Agent: p/a");
        h.see("Replace current pane");
        h.see("Split current pane");
        if cancel.is_empty() {
            h.click("Cancel Esc");
        } else {
            h.send(cancel);
        }
        h.see("Input ▸ Agents");
    }
    h.quit();
    let events = h.log("events");
    assert!(
        !events.contains("attach "),
        "Cancel/Esc/Ctrl-] must not attach: {events}"
    );
    assert!(!events.contains("start "));
    assert!(!events.contains("stop "));
}

#[test]
fn terminal_tabs_and_splits_route_input_and_close_only_owned_attaches() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"\x1djo");
    h.see("Input ▸ Show agent");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(
        !h.log("events").contains("attach p/b"),
        "Ctrl-] must close the menu without choosing a split"
    );
    h.send(b"o");
    h.see("Show agent");
    h.click("Right →");
    h.see("p/b READY");
    h.send(b"B");
    h.event("input p/b 42");
    h.send(b"\x1b[200~paste-b\x1b[201~");
    h.event("input p/b 1b5b3230307e70617374652d621b5b3230317e");
    assert!(!h.log("events").contains("detached p/a"));
    h.click("Viewer · p/a");
    h.send(b"A");
    h.event("input p/a 41");
    h.click("+ Tab");
    h.see("Tab 2");
    h.send(b"\x1dk\r"); // Already open: jump to a's existing pane, no second attach.
    h.see("p/a READY");
    h.send(b"Z");
    h.event("input p/a 5a");
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|l| *l == "attach p/a")
            .count(),
        1
    );
    h.click("Close pane");
    h.event("detached p/a");
    h.until(|h| !h.screen.screen().contents().contains("Viewer · p/a"));
    h.see("p/b READY");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    h.click("Close tab");
    h.event("detached p/b");
    h.quit();
    assert!(
        !h.log("events").contains("1b5b3c"),
        "Native controls must not send mouse input"
    );
    assert!(!h.log("events").contains("stop "));
    let agents: serde_json::Value = serde_json::from_str(&h.log("agents.json")).unwrap();
    assert_eq!(agents.as_object().unwrap().len(), 3);
}

#[test]
fn new_form_shows_bordered_inputs_and_click_positions_a_visible_cursor() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Native queue task");
    h.send(b"n");
    h.see("New agent");
    h.see("● Codex");
    h.see("project-one/codex");
    h.click("Name");
    h.send("\x15\x1b[200~a中b\x1b[201~".as_bytes());
    h.see("a中b");
    let (col, row) = h.press_button("a中b");
    h.send(format!("\x1b[<0;{};{}m", col + 1, row + 1).as_bytes());
    // Click the second terminal cell occupied by 中; insertion belongs before 中.
    h.send(
        format!(
            "\x1b[<0;{};{}M\x1b[<0;{};{}m",
            col + 3,
            row + 1,
            col + 3,
            row + 1
        )
        .as_bytes(),
    );
    h.until(|h| {
        !h.screen.screen().hide_cursor() && h.screen.screen().cursor_position() == (row, col + 1)
    });
    assert_eq!(
        h.screen.screen().cell(row - 1, col - 1).unwrap().contents(),
        "┏"
    );
    assert_eq!(
        h.screen.screen().cell(row + 1, col - 1).unwrap().contents(),
        "┗"
    );
    println!(
        "Observed New form after clicking the second cell of 中 (cursor row={}, col={}):\n{}",
        row + 1,
        col + 2,
        h.screen.screen().contents()
    );
    h.send("文\x1b[C\x1b[3~".as_bytes()); // Insert before 中, move past 中, delete b.
    h.see("a文中");
    assert!(!h.log("events").contains("start "));
    h.click("Create agent");
    h.see("a文中-actual READY");
    let args: Vec<String> = serde_json::from_str(h.log("start-args").trim()).unwrap();
    assert_eq!(args[1], "a文中");
    assert_eq!(args.last().unwrap(), "codex");
    assert!(!args.iter().any(|a| a == "--unique"));
    println!("Observed edited name a文中 and fake CLI creation: {args:?}");
    h.quit();
}

#[test]
fn new_agent_choices_create_with_defaults_without_switching_the_queue_project() {
    let script = include_str!("fixtures/drover.py").replace(
        "title='Native queue task'",
        "title='Queue ' + Path.cwd().name",
    );
    let mut h = Harness::start_with_projects(&script, true);
    h.see("Queue project-one");
    h.send(b"n");
    h.see("project-one/codex");
    h.click("Create agent");
    h.see("project-one/codex-actual READY");
    h.send(b"\x1dn");
    h.click("Project:");
    h.click("project-two ·");
    h.click("Claude");
    h.see("project-two/claude");
    h.click("Create agent");
    h.see("project-two/claude-actual READY");
    h.see("Queue project-one");
    let calls: Vec<Vec<String>> = h
        .log("start-args")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(calls.len(), 2);
    for (i, (project, agent)) in [("project-one", "codex"), ("project-two", "claude")]
        .iter()
        .enumerate()
    {
        assert_eq!(
            calls[i],
            vec![
                "start".to_string(),
                format!("{project}/{agent}"),
                "--cwd".into(),
                h.dir
                    .path()
                    .join(project)
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                "--unique".into(),
                "--".into(),
                agent.to_string()
            ]
        );
    }
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn new_agent_previews_exact_arguments_and_keeps_failed_draft() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Native queue task");
    h.send(b"n");
    h.see("New agent");
    h.see("project-one");
    h.click("Name");
    h.send(b"\x15p/new");
    h.click("Advanced");
    h.click("Command");
    h.send(b"\x15codex --model 'test model'\t");
    h.send("\x1b[200~hello\n世界\x1b[201~".as_bytes());
    h.see("Preview");
    assert!(!h.log("events").contains("start "));
    std::fs::write(h.dir.path().join("fail-start"), "").unwrap();
    h.send(b"\x13");
    h.see("synthetic start failed");
    h.see("p/new");
    h.see("test model");
    std::fs::remove_file(h.dir.path().join("fail-start")).unwrap();
    h.send(b"\x13");
    h.see("p/new-actual READY");
    let calls: Vec<Vec<String>> = h
        .log("start-args")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(calls.len(), 2);
    let cwd = h
        .dir
        .path()
        .join("project-one")
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        calls[1],
        vec![
            "start",
            "p/new",
            "--cwd",
            &cwd,
            "--prompt",
            "hello\n世界",
            "--",
            "codex",
            "--model",
            "test model"
        ]
    );
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn delayed_attach_stays_with_its_pane_and_closed_targets_are_discarded() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\r"); // A belongs to Tab 1.
    h.see("attaching p/a");
    h.send(b"jo2"); // B belongs to Tab 2.
    h.see("attaching p/b");
    h.click("Tab 1");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    h.click("Close tab");
    h.click("+ Tab"); // Empty tab remains active while B finishes in the background.
    h.send(b"\x1d\ta");
    h.see("Ctrl-S");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.event("attach p/b");
    h.send(b"draft");
    h.send(b"\x13");
    h.until(|h| h.log("queue-events").contains("draft"));
    h.until(|h| !h.screen.screen().contents().contains("Ctrl-S"));
    h.see("operation completed");
    assert!(!h.log("events").contains("attach p/a"));
    assert!(!h.screen.screen().contents().contains("p/b READY"));
    h.click("Tab 1"); // Original Tab 2 is now first.
    h.see("p/b READY");
    h.send(b"T");
    h.event("input p/b 54");
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn closing_a_start_target_keeps_the_created_agent_available_without_attaching() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    h.send(b"n");
    h.click("Name");
    h.send(b"\x15p/late\x13");
    h.event("start p/late");
    h.send(b"\x13"); // Busy submit cannot start it twice.
    h.send(b"\x1b"); // Hide form while the public start is running.
    h.see("Input ▸ Agents");
    h.click("Close tab");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    h.see("target closed or replaced");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    assert!(!h.log("events").contains("attach p/late"));
    assert_eq!(h.log("start-args").lines().count(), 1);
    h.quit();
    let agents: serde_json::Value = serde_json::from_str(&h.log("agents.json")).unwrap();
    assert!(agents.get("p/late-actual").is_some());
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn starting_in_a_hidden_tab_preserves_focus_and_exit_detaches_every_tab() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    h.send(b"\x1dn");
    h.click("Name");
    h.send(b"\x15p/hidden\x13");
    h.event("start p/hidden");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.click("Tab 1");
    h.send(b"A");
    h.event("input p/a 41");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    h.event("attach p/hidden-actual");
    h.send(b"B");
    h.event("input p/a 42");
    assert!(!h.log("events").contains("input p/hidden-actual"));
    h.click("Tab 2");
    h.see("p/hidden-actual READY");
    h.send(b"H");
    h.event("input p/hidden-actual 48");
    h.quit();
    assert!(h.log("events").contains("detached p/a"));
    assert!(h.log("events").contains("detached p/hidden-actual"));
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn closing_a_tab_detaches_all_its_splits_and_leaves_an_empty_tab() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"\x1djo6");
    h.see("p/b READY");
    h.click("Close tab");
    h.event("detached p/a");
    h.event("detached p/b");
    h.see("Select an agent on the left");
    h.quit();
    assert!(!h.log("events").contains("stop "));
    let agents: serde_json::Value = serde_json::from_str(&h.log("agents.json")).unwrap();
    assert_eq!(agents.as_object().unwrap().len(), 3);
}

#[test]
fn reselecting_the_displayed_agent_cancels_an_inflight_replacement() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\x1dj\r");
    h.see("attaching p/b");
    h.send(b"k\r");
    h.see("Input ▸ p/a");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    // Let the public status calls finish and the app consume their replies.
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        h.pump();
    }
    h.send(b"A");
    h.event("input p/a 41");
    assert!(!h.log("events").contains("attach p/b"));
    assert!(!h.log("events").contains("detached p/a"));
    h.quit();
}

#[test]
fn reselecting_a_pending_agent_never_sends_input_to_the_old_session() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\x1dj\r");
    h.see("attaching p/b");
    h.send(b"\rZ\x1b[200~pending-b\x1b[201~");
    h.send(b"\x1b[<0;56;4M\x1b[<0;56;4m");
    // A visible native page acknowledges that all preceding input was handled.
    h.send(b"\x1d\t?");
    h.see("Queue help");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.see("p/b READY");
    h.send(b"\x1d\x1b[ZB");
    h.event("input p/b 42");
    h.quit();
    let events = h.log("events");
    assert!(
        !events.contains("input p/a "),
        "input for pending B reached old A:\n{events}"
    );
    assert_eq!(
        events.lines().filter(|line| *line == "attach p/b").count(),
        1
    );
    assert!(!events.contains("stop "));
}
