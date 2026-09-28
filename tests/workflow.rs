mod common;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

/// The periodic read-only check of Drover's notification preference, as fake drovers log it.
const NOTIFICATION_STATUS: &str = r#"["notifications", "status", "--json"]"#;

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
        let mut harness = Self::start_prepared(queue_script, registered, extra, read_chunk, |_| {});
        // Wait for a unique selected row as well as the updated sort mode.
        harness.see("Synthetic title");
        harness.send(b"skk");
        harness.see("┃ ○ a ");
        harness.see("s Name");
        harness
    }
    fn start_prepared(
        queue_script: &str,
        registered: bool,
        extra: &str,
        read_chunk: usize,
        prepare: impl FnOnce(&std::path::Path),
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
        prepare(dir.path());
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
        // Color assertions expect the configured 24-bit values.
        cmd.env("COLORTERM", "truecolor");
        cmd.env("NO_COLOR", "");
        cmd.env("HOME", &home);
        cmd.env("XDG_STATE_HOME", dir.path().join("state"));
        cmd.env("SADDLE_RUNTIME_DIR", dir.path().join("run"));
        cmd.env("CORRAL_NAME", "synthetic-parent");
        cmd.env("CORRAL_INSTANCE", "synthetic-parent-instance");
        let shell = common::script(
            dir.path(),
            "shell",
            r#"#!/usr/bin/env python3
import os, signal, sys, tty, json
from pathlib import Path
root = Path(__file__).parent
(root / ('shell-' + os.environ['SADDLE_PANE'])).write_text(json.dumps(dict(os.environ)))
(root / ('shell-cwd-' + os.environ['SADDLE_PANE'])).write_text(os.getcwd())
tty.setraw(0)
def stop(*_):
    sys.exit(0)
signal.signal(signal.SIGHUP, stop)
os.write(1, b'SHELL READY\r\n')
received = b''
while True:
    data = os.read(0, 4096)
    received += data
    if received.endswith(b'exit'): sys.exit(7)
    os.write(1, b'SHELL INPUT:' + data + b'\r\n')
"#,
        );
        cmd.env("SHELL", shell);
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
        let (col, row) = self
            .locate(label, 0)
            .unwrap_or_else(|| panic!("click target not found: {label}"));
        self.send(format!("\x1b[<0;{};{}M", col + 1, row + 1).as_bytes());
        (col, row)
    }
    fn locate(&self, label: &str, first_row: u16) -> Option<(u16, u16)> {
        let screen = self.screen.screen();
        let (rows, cols) = screen.size();
        for row in first_row..rows {
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
                    return Some((col, row));
                }
            }
        }
        None
    }
    /// The inner area (left, top, right, bottom) of the box whose top border carries `title`.
    fn boxed(&self, title: &str) -> (u16, u16, u16, u16) {
        let (col, row) = self.locate(title, 0).unwrap();
        let screen = self.screen.screen();
        let cell = |r: u16, c: u16| screen.cell(r, c).unwrap().contents();
        let left = (0..col)
            .rev()
            .find(|&c| matches!(cell(row, c), "┏" | "┌"))
            .unwrap();
        let right = (col..screen.size().1)
            .find(|&c| matches!(cell(row, c), "┓" | "┐"))
            .unwrap();
        let bottom = (row + 1..screen.size().0)
            .find(|&r| matches!(cell(r, left), "┗" | "└"))
            .unwrap();
        (left + 1, row + 1, right, bottom)
    }
    /// Text inside the bordered box whose top border carries `title`.
    fn popup(&self, title: &str) -> String {
        let (left, top, right, bottom) = self.boxed(title);
        let screen = self.screen.screen();
        (top..bottom)
            .map(|r| {
                (left..right)
                    .map(|c| {
                        let text = screen.cell(r, c).unwrap().contents();
                        if text.is_empty() { " " } else { text }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    /// Screen position of `label` on its row inside the `title` popup.
    fn row_in(&mut self, title: &str, label: &str) -> (u16, u16) {
        self.see(title);
        self.until(|h| h.popup(title).contains(label));
        let (left, top, _, _) = self.boxed(title);
        let (row, text) = self
            .popup(title)
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains(label))
            .map(|(i, line)| (top + i as u16, line.to_owned()))
            .unwrap();
        (
            left + text[..text.find(label).unwrap()].chars().count() as u16,
            row,
        )
    }
    /// Clicks the row of the `title` popup that shows `label`.
    fn click_in(&mut self, title: &str, label: &str) {
        let (col, row) = self.row_in(title, label);
        let (x, y) = (col + 1, row + 1);
        self.send(format!("\x1b[<0;{x};{y}M\x1b[<0;{x};{y}m").as_bytes());
    }
    fn contents(&self) -> String {
        self.screen.screen().contents()
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
    h.see("Tasks · ");
    h.see("Synthetic title");
    assert!(!h.log("events").contains("reply "));
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"q");
    h.event("input p/a 71");
    h.send("\x1b[200~中文\nhello\x1b[201~".as_bytes());
    h.event("1b5b3230307ee4b8ade696870a68656c6c6f1b5b3230317e");
    h.send(b"\x1b[<0;56;6M");
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
    h.see("waiting");
    h.master
        .resize(PtySize {
            rows: 44,
            cols: 160,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(44, 160);
    h.event("size p/b 106x38");
    // Agents keep the whole left column; Tasks opens over the Viewer without resizing it.
    h.until(|h| h.screen.screen().cell(42, 0).unwrap().contents() == "└");
    h.send(b"\x1d\t");
    h.see("Native queue task");
    // Native Queue translates actions into public CLI calls, never a PTY.
    h.send(b"p");
    h.until(|h| h.log("queue-events").contains("[\"pause\"]"));
    h.see("Paused");
    // Agent disappearance returns to the prompt, without selecting another viewer.
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"blocked","p/new":"idle","p/taken":"idle"}"#,
    )
    .unwrap();
    h.event("detached p/b");
    h.send(b"\x1d"); // Reveal the Viewer after the Tasks popup has observed disappearance.
    h.see("attach exited");
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
    h.see("Tasks · ");
    h.see("Synthetic title");
    // One-based SGR coordinates: first agent headline is screen row 6.
    h.send(b"\x1b[<0;5;6M");
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
    h.see("Tasks · ");
    h.send(b"\t");
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("missing project");
    h.send(b"ce");
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
    h.click("Tasks · ");
    h.see("Queue project-one");
    h.click("project-one ▾ c");
    h.see("Projects");
    h.click("project-two");
    h.see("Queue project-two");
    h.click("Pause p");
    h.see("Paused");
    assert!(!h.dir.path().join("project-one/queue-state.json").exists());
    assert!(h.dir.path().join("project-two/queue-state.json").exists());
    h.click("project-two ▾ c");
    h.click("project-one");
    h.see("Queue project-one");
    h.see("Manual");
    h.quit();
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn native_mouse_buttons_cover_forms_and_stop_confirmation() {
    let mut h = Harness::start();
    h.see("Tasks · ");
    h.see("Synthetic title");
    h.click("x Stop");
    h.click("Cancel Esc");
    h.see("cancelled");
    assert!(!h.log("events").contains("stop "));
    h.click("Tasks · ");
    h.see("detail line 0");
    h.click("Run details ↵");
    h.see("From the queue list");
    h.click("Task text t");
    h.until(|h| !h.contents().contains("From the queue list"));
    h.click("Add task a");
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
    // Wait for the redraw at the new size: the Agents column ends at column 34.
    h.until(|h| matches!(h.screen.screen().cell(0, 33).unwrap().contents(), "┐" | "┓"));
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    // Narrow windows keep Agents on the left; hit targets must follow the new rows.
    h.click("x Stop");
    h.click("‹Stop y›");
    h.event("stop p/a");
    h.quit();
}

#[test]
fn native_queue_help_details_form_and_actions_use_only_public_cli_commands() {
    let mut h = Harness::start();
    h.see("Tasks · ");
    h.send(b"\t?");
    h.see("Tasks help");
    h.see("Back Esc");
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains("Back Esc"));
    h.see("Native queue task");
    h.send(b"\r");
    h.see("Input ▸ Tasks · Run details");
    h.see("detail line 0");
    h.send(b"\x1b[6~\x1b[6~\x1b[6~");
    // Three pages reach line 50 with the outlined toolbars above the details.
    h.see("detail line 50");
    assert!(!h.screen.screen().contents().contains("detail line 0"));
    h.send(b"t");
    h.see("detail line 0");
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
    h.see("Input ▸ Tasks · Help / Result");
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("Native queue task"); // Add waits for queue data.
    h.send(b"a");
    h.see("─ Add task");
    h.send(b"Second\tBody\x13");
    h.until(|h| !h.screen.screen().contents().contains("─ Add task"));
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
    h.see("Input ▸ Agents");
    h.until(|h| !h.contents().contains("Edit task"));
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("T1 Native queue task");
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("Synthetic native task");
    h.send(b"\r");
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
    h.until(|h| h.screen.screen().cell(48, 0).unwrap().contents() == "└");
    h.see("原生新增");
    h.quit();
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn buttons_require_release_on_the_same_target() {
    let mut h = Harness::start();
    h.see("Tasks · ");
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.press_button("Tasks · ");
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(
        !h.contents().contains("Close Esc"),
        "Down must not open Tasks"
    );
    h.send(b"\x1b[<32;130;4M\x1b[<0;130;4m"); // Drag/release over Viewer cancels, without sending a stray release.
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(!h.contents().contains("Close Esc"));
    assert!(
        !h.log("events").contains("input p/a "),
        "A management button gesture must not leak into Viewer"
    );
    h.quit();
}

#[test]
fn overlays_capture_input_and_narrow_windows_keep_the_viewer_attached() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"\t");
    h.event("input p/a 09"); // Viewer keeps Tab; never changes management focus.
    h.send(b"\x1dx");
    h.see("‹Stop y›");
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
    // Wait for the redraw at the new size: the Agents column ends at column 34.
    h.until(|h| matches!(h.screen.screen().cell(0, 33).unwrap().contents(), "┐" | "┓"));
    h.click("Tasks · ");
    h.see("Path e"); // The suspended project picker resumes.
    h.send(b"\x1b");
    h.see("Input ▸ Tasks");
    h.see("Native queue task");
    h.click("Pause p");
    h.see("Paused");
    h.click("Close Esc");
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
    h.see("Tasks · ");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\r\t");
    h.see("Native queue task"); // Add waits for queue data.
    h.send(b"a");
    h.see("Ctrl-S");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.event("attach p/a"); // The Viewer is behind the Tasks popup.
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("Native queue task"); // Add waits for queue data.
    h.send(b"a");
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
    h.see("s Sort"); // The next key confirms the preceding r was processed.
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
    h.see("Tasks · ");
    h.send(b"\t");
    // The list is the popup's left third; the selected task's text is beside it.
    let list = |h: &Harness| h.screen.screen().rows(0, 40).collect::<Vec<_>>().join("\n");
    h.until(|h| list(h).contains("History-39"));
    h.send("\x1b[<65;12;20M".repeat(4).as_bytes());
    h.send(b"\r");
    h.see("Input ▸ Tasks · Run details"); // Barrier: all four wheel events have been processed.
    assert!(!list(&h).contains("History-39"), "{}", list(&h));
    // Column 40 is the list's scrollbar.
    h.send("\x1b[<65;40;20M".repeat(80).as_bytes());
    h.until(|h| list(h).contains("History-00"));
    let refreshes = h.log("queue-events").lines().count();
    h.until(|h| h.log("queue-events").lines().count() > refreshes + 1);
    assert!(list(&h).contains("History-00"));
    h.send("\x1b[<64;12;20M".repeat(80).as_bytes());
    h.until(|h| list(h).contains("History-39"));
    h.quit();
    assert!(
        h.log("queue-events")
            .lines()
            .all(|line| line == "[\"list\", \"--json\"]" || line == NOTIFICATION_STATUS)
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("40 tasks");
    // The list is the popup's left third; the selected task's text is beside it.
    let list = |h: &Harness| h.screen.screen().rows(0, 40).collect::<Vec<_>>().join("\n");
    h.until(|h| list(h).contains("Complete-history-01")); // Older than the former ten-record API limit.
    h.send("\x1b[<65;12;20M".repeat(5).as_bytes());
    h.send(b"?");
    h.see("Tasks help"); // Barrier: the wheel events have been processed.
    h.send(b"\x1b");
    h.until(|h| list(h).contains("Complete-history-"));
    assert!(!list(&h).contains("Complete-history-01"));
    h.send("\x1b[<65;40;20M".repeat(80).as_bytes());
    h.until(|h| list(h).contains("Complete-history-40"));
    h.see("/40 · End");
    h.send("\x1b[<64;12;20M".repeat(80).as_bytes());
    h.until(|h| list(h).contains("Complete-history-01"));
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
agents_green = "#556677"
claude = "#778899"
overlay = "#203040"
text = "#abcdef"
"##,
    );
    h.see("Synthetic title");
    h.see("Tasks · ");
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
    // Agents has its own palette; the shared status accents keep coloring Tasks.
    assert_eq!(
        label_cell(&h, "idle").fgcolor(),
        Color::Rgb(0x55, 0x66, 0x77)
    );
    assert_eq!(
        label_cell(&h, "claude").fgcolor(),
        Color::Rgb(0x77, 0x88, 0x99)
    );
    h.send(b"\t");
    h.see("Native queue task");
    h.see("Delete x"); // The whole popup has been drawn.
    assert_eq!(
        label_cell(&h, "Ready").fgcolor(),
        Color::Rgb(0x44, 0x55, 0x66)
    );
    assert_eq!(
        label_cell(&h, "Check & release g").fgcolor(),
        Color::Rgb(0x11, 0x22, 0x33)
    );
    // Tasks is a dialog surface, like the other popups.
    assert_eq!(
        label_cell(&h, "Check & release g").bgcolor(),
        Color::Rgb(0x20, 0x30, 0x40)
    );
    h.send(b"c");
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
    h.see("Tasks · ");
    h.send(b"\t");
    h.see("Queue project-one");
    // The loaded footer gains task actions; wait for it before locating a button there.
    h.see("Delete x");
    std::fs::write(h.dir.path().join("project-two/fail-list"), "").unwrap();
    h.click("All pending A");
    h.see("─ All pending ─");
    h.see("synthetic project read failure");
    h.see("Read failed");
    h.see("1 T1 Queue project-one");
    h.see("project-two");
    std::fs::remove_file(h.dir.path().join("project-two/fail-list")).unwrap();
    // The header's Refresh is disabled while a sub-page is open; use the page's own key.
    h.send(b"r");
    h.see("1 T1 Queue project-two");
    h.until(|h| !h.screen.screen().contents().contains("Read failed"));
    h.send(b"\x1b");
    h.until(|h| !h.screen.screen().contents().contains("─ All pending ─"));
    h.see("Queue project-one");
    h.quit();
    let events = h.log("queue-events");
    assert!(
        events
            .lines()
            .all(|line| line == "[\"list\", \"--json\"]" || line == NOTIFICATION_STATUS),
        "{events}"
    );
    assert!(!h.log("events").contains("attach "));
}

#[test]
fn task_edit_from_run_details_saves_and_returns_to_the_same_view() {
    let mut h = Harness::start();
    h.see("Tasks · ");
    h.click("Tasks · ");
    h.see("T1 Native queue task");
    h.click("Run details ↵");
    h.see("Input ▸ Tasks · Run details");
    h.see("detail line 0");
    h.click("Edit e");
    h.see("Edit task");
    h.send(b" revised\x13");
    h.until(|h| !h.screen.screen().contents().contains("Edit task"));
    h.see("Input ▸ Tasks · Run details");
    h.see("Native queue task revised");
    h.click("Task text t");
    h.until(|h| !h.contents().contains("Run details  "));
    h.click("Edit e");
    h.see("Edit task");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Edit task"));
    h.see("Task text t");
    h.see("T1 Native queue task revised");
    h.quit();
    let events = h.log("queue-events");
    assert_eq!(events.matches("[\"edit\"").count(), 1, "{events}");
    assert!(!h.log("events").contains("input p/a"));
}

#[test]
fn run_details_refresh_beside_the_list_until_tasks_closes() {
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
    h.send(b"\r");
    h.see("p/a READY");
    let shows = |h: &Harness| h.log("queue-events").matches("\"show\"").count();
    h.click("Tasks · ");
    h.see("Detail target 任务");
    h.click("Detail target");
    h.see("list body");
    assert_eq!(shows(&h), 0, "Task text needs no show");
    h.click("Run details ↵");
    h.see("Completion checks");
    h.see("Input ▸ Tasks · Run details");
    assert_eq!(shows(&h), 1);
    // Scroll keys stay in Tasks while an agent is attached in Viewer.
    h.send(b"\x1b[6~\x1b[5~");
    h.until(|h| shows(h) >= 2); // About five seconds later, one at a time.
    h.send(b"e"); // A running task stays read-only.
    assert!(!h.log("events").contains("input p/a"));
    let queue = h.log("queue-events");
    assert!(
        queue.lines().all(|l| l == r#"["list", "--json"]"#
            || l == r#"["show", "T4", "--json", "--with-agent-status"]"#
            || l == NOTIFICATION_STATUS),
        "{queue}"
    );
    h.send(b"\x1b");
    h.see("Input ▸ p/a"); // Closing returns to the Viewer it was opened from.
    h.see("Agent · p/a");
    h.until(|h| !h.screen.screen().contents().contains("Completion checks"));
    let after_close = shows(&h);
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        h.pump();
    }
    assert_eq!(shows(&h), after_close, "closing Tasks stops details");
    h.quit();
}

#[test]
fn placement_cancel_and_escape_never_attach_and_new_cancel_keeps_the_draft() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Tasks · ");
    h.see("Synthetic title");
    h.send(b"n");
    h.click("Regular");
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
    // The status line can still be the stale one from before `n`; only Esc hides the draft.
    // Clicking before Esc is read would merge them into one read, and crossterm parses the
    // ESC ESC as a single Esc, turning the rest of the mouse report into plain characters.
    h.until(|h| !h.contents().contains("review-draft"));
    h.see("Input ▸ Agents");
    for cancel in [b"\x1b".as_slice(), b"", b"\x1d"] {
        h.click("│ + │");
        h.see("Open content in a new tab");
        h.see("p/a");
        if cancel.is_empty() {
            h.click("Cancel Esc");
        } else {
            h.send(cancel);
        }
        h.until(|h| !h.contents().contains("Open content in a new tab"));
        // Esc and Cancel return to the Viewer that opened it; Ctrl-] returns to Agents.
        h.see(if cancel == b"\x1d" {
            "Input ▸ Agents"
        } else {
            "Input ▸ Viewer"
        });
        assert_eq!(h.contents().matches(" ×│").count(), 1);
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
    h.click("Split ▾");
    h.see("Input ▸ Split pane");
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
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "p/b");
    h.see("p/b READY");
    h.send(b"B");
    h.event("input p/b 42");
    h.send(b"\x1b[200~paste-b\x1b[201~");
    h.event("input p/b 1b5b3230307e70617374652d621b5b3230317e");
    assert!(!h.log("events").contains("detached p/a"));
    h.click("Agent · p/a");
    h.send(b"A");
    h.event("input p/a 41");
    h.click("│ + │");
    h.click_in("Open content in a new tab", "p/b"); // Moves B's pane into Tab 2.
    h.until(|h| h.contents().matches(" ×│").count() == 2);
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
    h.until(|h| !h.screen.screen().contents().contains("Agent · p/a"));
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
    h.see("Tasks · ");
    h.send(b"n");
    h.see("New agent");
    h.see("● Codex");
    h.see("main");
    h.click("Regular");
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
    h.click("Controller");
    h.see("Controller name · read-only");
    h.click("Name");
    h.send(b"\x15\x1b[200~cannot-edit\x1b[201~");
    h.click("Regular");
    h.see("a文中");
    assert!(!h.log("events").contains("start "));
    h.click("Create agent");
    h.see("a文中-actual READY");
    h.see("Regular · agents/a文中-actual");
    let args: Vec<String> = serde_json::from_str(h.log("start-args").trim()).unwrap();
    assert_eq!(args[1], "agents/a文中");
    assert!(args.windows(2).any(|w| w == ["--label", "role=regular"]));
    assert_eq!(&args[args.len() - 2..], ["codex", "--yolo"]);
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
    h.see("Tasks · ");
    h.send(b"n");
    h.see("main");
    h.click("Create agent");
    h.see("main-actual READY");
    h.see("Controller · agents/main-actual");
    h.send(b"\x1dn");
    h.click("Project:");
    h.click("project-two ·");
    h.click("Claude");
    h.see("main");
    h.click("Create agent");
    h.until(|h| h.log("start-args").lines().count() == 2);
    h.until(|h| !h.contents().contains("Create agent"));
    h.send(b"\x1d\t");
    h.see("Queue project-one");
    let calls: Vec<Vec<String>> = h
        .log("start-args")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(calls.len(), 2);
    for (i, (project, command)) in [
        ("project-one", vec!["codex", "--yolo"]),
        ("project-two", vec!["claude"]),
    ]
    .iter()
    .enumerate()
    {
        let mut expected = vec![
            "start".to_string(),
            "agents/main".to_string(),
            "--cwd".into(),
            h.dir
                .path()
                .join(project)
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            "--label".into(),
            "role=controller".into(),
            "--".into(),
        ];
        expected.extend(command.iter().map(|arg| arg.to_string()));
        assert_eq!(calls[i], expected);
    }
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn new_agent_previews_exact_arguments_and_keeps_failed_draft() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Tasks · ");
    h.send(b"n");
    h.see("New agent");
    h.see("project-one");
    h.click("Regular");
    h.click("Prefix");
    h.send(b"\x15p");
    h.click("Name");
    h.send(b"\x15new");
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
            "--label",
            "role=regular",
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
    h.click("│ + │");
    h.click_in("Open content in a new tab", "p/b"); // B belongs to Tab 2.
    h.see("attaching p/b");
    h.click("│p/a ");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    h.click("Close tab");
    // Another tab remains active while B finishes in the background; p/taken is attached
    // elsewhere, so its tab stays empty.
    h.click("│ + │");
    h.click_in("Open content in a new tab", "p/taken");
    h.send(b"\x1d\t");
    h.see("Native queue task"); // Add waits for queue data.
    h.send(b"a");
    h.see("Ctrl-S");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.event("attach p/b");
    h.send(b"draft");
    h.send(b"\x13");
    h.until(|h| h.log("queue-events").contains("draft"));
    h.until(|h| !h.screen.screen().contents().contains("Ctrl-S"));
    h.see("operation completed");
    h.send(b"\x1b"); // Close Tasks to reach the terminal tabs it covers.
    h.see("Input ▸ Agents");
    assert!(!h.log("events").contains("attach p/a"));
    assert!(!h.screen.screen().contents().contains("p/b READY"));
    h.click("│p/b "); // The remaining tab keeps its agent name.
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
    h.click("Regular");
    h.click("Prefix");
    h.send(b"\x15p");
    h.click("Name");
    h.send(b"\x15late\x13");
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
    h.click("Regular");
    h.click("Prefix");
    h.send(b"\x15p");
    h.click("Name");
    h.send(b"\x15hidden");
    h.click("Advanced");
    h.click("Open in: Current pane");
    h.send(b"\x13");
    h.event("start p/hidden");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.click("│p/a ");
    h.send(b"A");
    h.event("input p/a 41");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    h.event("attach p/hidden-actual");
    h.send(b"B");
    h.event("input p/a 42");
    assert!(!h.log("events").contains("input p/hidden-actual"));
    h.click("│p/hidden-actual ");
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
    h.click("Split ▾");
    h.click("Below ↓");
    h.click_in("Open content below", "p/b");
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
    h.see("Tasks help");
    h.send(b"q"); // Close Tasks so the Viewer is visible again.
    h.see("Input ▸ Agents");
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

#[test]
fn split_and_new_tab_choose_the_place_before_the_agent_and_cancel_leaves_no_layout() {
    let mut h = Harness::start();
    std::fs::write(
        h.dir.path().join("labels.json"),
        r#"{"p/a":{"role":"controller"},"p/b":{"role":"regular"}}"#,
    )
    .unwrap();
    h.see("Synthetic title");
    h.send(b"\x1b[<0;5;6M"); // Mouse: the first agent row opens p/a in the current pane.
    h.see("p/a READY");
    // Cancelling at either step keeps the single pane and tab.
    h.click("Split ▾");
    h.see("Right →");
    h.see("Below ↓");
    println!("SPLIT menu:\n{}", h.contents());
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Right →"));
    h.click("Split ▾");
    h.see("Right →");
    h.send(b"\x1b[C"); // The arrow key picks the same side as the button.
    h.see("Open content on the right");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Open content on the right"));
    h.click("│ + │");
    h.see("Open content in a new tab");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Open content in a new tab"));
    h.click("Split ▾");
    h.see("Left ←");
    h.send(b"\x1d"); // Ctrl-] closes the menu and returns to Agents.
    h.see("Input ▸ Agents");
    assert!(!h.contents().contains("Left ←"));
    assert_eq!(h.contents().matches(" ×│").count(), 1, "{}", h.contents());
    assert_eq!(
        h.contents().matches("Controller · p/a").count(),
        1,
        "{}",
        h.contents()
    );
    assert!(!h.contents().contains("Viewer"), "{}", h.contents());
    assert!(!h.log("events").contains("attach p/b"));
    // The mouse path: Split → Right → agent. The pane's own agent is not a candidate.
    h.click("Split ▾");
    h.click("Right →");
    h.see("Open content on the right");
    let popup = h.popup("Open content on the right");
    println!("PICKER:\n{}", h.contents());
    assert!(
        popup.contains("p/b") && popup.contains("p/taken"),
        "{popup}"
    );
    assert!(!popup.contains("p/a"), "{popup}");
    assert!(!popup.contains("Move here"), "{popup}");
    assert!(popup.contains("Cancel Esc"), "{popup}");
    h.click_in("Open content on the right", "p/b");
    h.see("p/b READY");
    let a = h.locate("Controller · p/a", 0).unwrap();
    let b = h.locate("Regular · p/b", 0).unwrap();
    assert!(a.1 == b.1 && a.0 < b.0, "p/b must open right of p/a");
    h.send(b"B");
    h.event("input p/b 42");
    h.quit();
    let events = h.log("events");
    assert_eq!(events.lines().filter(|l| *l == "attach p/a").count(), 1);
    assert_eq!(events.lines().filter(|l| *l == "attach p/b").count(), 1);
    assert!(!events.contains("stop "));
    assert!(
        !events.contains("1b5b3c"),
        "Native controls must not send mouse input"
    );
}

#[test]
fn choosing_an_open_agent_moves_its_session_without_attaching_again() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\x1b[<0;5;6M");
    h.see("p/a READY");
    h.send(b"A");
    h.see("INPUT RECEIVED");
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "p/b");
    h.see("p/b READY");
    h.click("│ + │");
    h.see("Open content in a new tab");
    let popup = h.popup("Open content in a new tab");
    println!("MOVE picker:\n{}", h.contents());
    for line in popup.lines() {
        if line.contains("p/a") || line.contains("p/b") {
            assert!(line.contains("Move here"), "{popup}");
        } else if line.contains("p/taken") {
            assert!(!line.contains("Move here"), "{popup}");
        }
    }
    h.click_in("Open content in a new tab", "p/a");
    h.until(|h| h.contents().matches(" ×│").count() == 2);
    h.see("Agent · p/a");
    // The moved pane keeps its session and output; p/b stays behind in Tab 1.
    assert!(h.contents().contains("INPUT RECEIVED"));
    assert!(!h.contents().contains("Agent · p/b"));
    h.send(b"Z");
    h.event("input p/a 5a");
    h.click("│p/b ");
    h.see("Agent · p/b");
    assert!(!h.contents().contains("Agent · p/a"));
    // Moving it back as a split leaves its emptied tab nowhere.
    h.click("Split ▾");
    h.click("Below ↓");
    h.see("Open content below");
    let popup = h.popup("Open content below");
    assert!(
        popup.contains("p/a") && popup.contains("Move here"),
        "{popup}"
    );
    assert!(!popup.contains("p/b"), "{popup}");
    h.click_in("Open content below", "p/a");
    h.until(|h| h.contents().matches(" ×│").count() == 1);
    h.see("Agent · p/a");
    let a = h.locate("Agent · p/a", 0).unwrap();
    let b = h.locate("Agent · p/b", 0).unwrap();
    assert!(a.0 == b.0 && a.1 > b.1, "p/a must move below p/b");
    assert!(h.contents().contains("INPUT RECEIVED"));
    h.send(b"Y");
    h.event("input p/a 59");
    let events = h.log("events");
    assert_eq!(events.lines().filter(|l| *l == "attach p/a").count(), 1);
    assert!(!events.contains("detach"), "{events}");
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn moving_an_attaching_agent_keeps_its_request_with_the_moved_pane() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\x1b[<0;5;6M");
    h.see("p/a READY");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "p/b");
    h.see("Attaching p/b");
    h.click("│ + │");
    h.see("Open content in a new tab");
    let popup = h.popup("Open content in a new tab");
    assert!(
        popup
            .lines()
            .any(|l| l.contains("p/b") && l.contains("Move here")),
        "{popup}"
    );
    h.click_in("Open content in a new tab", "p/b");
    h.until(|h| h.contents().matches(" ×│").count() == 2);
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.see("p/b READY");
    h.send(b"B");
    h.event("input p/b 42");
    h.click("│p/a ");
    h.see("Agent · p/a");
    h.until(|h| !h.contents().contains("p/b READY"));
    assert!(!h.contents().contains("Attaching p/b"));
    h.quit();
    let events = h.log("events");
    assert_eq!(events.lines().filter(|l| *l == "attach p/b").count(), 1);
    assert!(!events.contains("stop "));
}

#[test]
fn a_candidate_click_opens_only_the_agent_it_was_pressed_on() {
    const TITLE: &str = "Open content in a new tab";
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.click("│ + │");
    let (col, row) = h.row_in(TITLE, "p/b");
    let fg = |h: &Harness| h.screen.screen().cell(row, col).unwrap().fgcolor();
    let press = |h: &mut Harness, down: bool| {
        let kind = if down { 'M' } else { 'm' };
        h.send(format!("\x1b[<0;{};{}{kind}", col + 1, row + 1).as_bytes());
    };
    // Press p/b; a public ls refresh then puts p/aa on that row before the release.
    let idle = fg(&h);
    press(&mut h, true);
    h.until(|h| fg(h) != idle); // The pressed paint shows the press was handled.
    let pressed = fg(&h);
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"idle","p/aa":"idle","p/b":"working","p/taken":"idle"}"#,
    )
    .unwrap();
    h.until(|h| h.locate("p/aa", row) == Some((col, row)));
    press(&mut h, false);
    h.until(|h| fg(h) != pressed); // The release was handled.
    println!("REFRESHED under press:\n{}", h.contents());
    assert!(
        h.contents().contains(TITLE) && h.popup(TITLE).contains("p/aa"),
        "the release must not open the agent that replaced the pressed row:\n{}",
        h.contents()
    );
    // A refresh that leaves the pressed name on its row still opens it.
    let (col, row) = h.row_in(TITLE, "p/b");
    let fg = |h: &Harness| h.screen.screen().cell(row, col).unwrap().fgcolor();
    let idle = fg(&h);
    h.send(format!("\x1b[<0;{};{}M", col + 1, row + 1).as_bytes());
    h.until(|h| fg(h) != idle);
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"idle","p/aa":"idle","p/b":"working","p/c":"idle","p/taken":"idle"}"#,
    )
    .unwrap();
    h.until(|h| h.popup(TITLE).contains("p/c"));
    assert_eq!(h.locate("p/b", row), Some((col, row)));
    h.send(format!("\x1b[<0;{};{}m", col + 1, row + 1).as_bytes());
    h.see("p/b READY");
    h.send(b"Z");
    h.event("input p/b 5a");
    h.quit();
    let events = h.log("events");
    assert!(!events.contains("attach p/aa"), "{events}");
    assert!(!events.contains("input p/aa"), "{events}");
    assert_eq!(events.lines().filter(|l| *l == "attach p/b").count(), 1);
    assert!(!events.contains("stop "));
}

#[test]
fn tasks_entry_opens_the_popup_and_closing_returns_to_the_previous_target() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.see("Tasks · ");
    // Agents own the whole left column; the task list only lives in the popup.
    assert!(!h.contents().contains("Native queue task"));
    h.send(b"\r");
    h.see("p/a READY");
    h.see("Input ▸ p/a");
    h.click("Tasks · ");
    h.see("Input ▸ Tasks");
    // List and the selected task's text sit side by side.
    h.see("T1 Native queue task");
    h.see("detail line 0");
    h.see("Close Esc");
    h.send(b"\x1b");
    h.see("Input ▸ p/a");
    h.until(|h| !h.contents().contains("Native queue task"));
    h.send(b"z");
    h.event("input p/a 7a");
    // Tab from Agents opens it again with the same selection; Esc goes back to Agents.
    h.send(b"\x1d\t");
    h.see("Input ▸ Tasks");
    h.see("detail line 0");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.quit();
}

impl Harness {
    fn ctl(&self, args: &[&str]) -> serde_json::Value {
        self.ctl_as(args, &[])
    }
    fn ctl_as(&self, args: &[&str], env: &[(&str, &str)]) -> serde_json::Value {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_saddle"));
        cmd.arg("ctl")
            .args(args)
            .env("SADDLE_RUNTIME_DIR", self.dir.path().join("run"))
            .env("XDG_STATE_HOME", self.dir.path().join("state"))
            .env_remove("CORRAL_NAME")
            .env_remove("CORRAL_INSTANCE")
            .env_remove("SADDLE_INSTANCE")
            .env_remove("SADDLE_PANE")
            .env_remove("SADDLE_REVISION");
        for (key, value) in env {
            cmd.env(key, value);
        }
        let output = cmd.output().unwrap();
        serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)))
    }
    fn operation(&mut self, initial: &serde_json::Value) -> serde_json::Value {
        let id = initial["request_id"].as_str().unwrap();
        let instance = initial["instance"].as_str().unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let value = self.ctl(&["request", id, "--instance", instance]);
            if !matches!(
                value["state"].as_str(),
                Some("accepted" | "starting" | "attaching")
            ) {
                return value;
            }
            assert!(Instant::now() < deadline, "{value}");
            self.pump();
        }
    }
}

#[test]
fn ctl_shell_creation_is_idempotent_preserves_focus_and_confirms_close() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    let initial = h.ctl(&["inspect"]);
    assert_eq!(initial["ok"], true, "{initial}");
    let instance = initial["instance"].as_str().unwrap();
    let open = [
        "open",
        "--instance",
        instance,
        "--relative-to",
        "active",
        "--place",
        "right",
        "--shell",
        "--request-id",
        "shell-one",
    ];
    let accepted = h.ctl(&open);
    assert_eq!(accepted["accepted"], true, "{accepted}");
    let result = h.operation(&accepted);
    assert_eq!(result["state"], "complete", "{result}");
    let pane = result["pane"].as_u64().unwrap().to_string();
    let current = h.ctl(&["inspect", "--instance", instance]);
    assert_eq!(current["active_pane"], initial["active_pane"]);
    assert_eq!(current["focus"], initial["focus"]);
    assert_eq!(current["tabs"][0]["panes"].as_array().unwrap().len(), 2);
    assert_eq!(h.ctl(&open)["pane"], result["pane"]);
    let conflict = h.ctl(&[
        "open",
        "--instance",
        instance,
        "--relative-to",
        "active",
        "--place",
        "left",
        "--shell",
        "--request-id",
        "shell-one",
    ]);
    assert_eq!(conflict["error"]["code"], "request_conflict");
    h.until(|h| h.dir.path().join(format!("shell-{pane}")).exists());
    let env: serde_json::Value = serde_json::from_str(&h.log(&format!("shell-{pane}"))).unwrap();
    assert!(env.get("CORRAL_NAME").is_none());
    assert_eq!(env["SADDLE_INSTANCE"], instance);
    assert_eq!(env["SADDLE_PANE"], pane);
    let close = h.ctl(&[
        "close",
        "--instance",
        instance,
        "--pane",
        &pane,
        "--request-id",
        "close-one",
    ]);
    assert_eq!(close["state"], "confirmation_required", "{close}");
    assert_eq!(
        h.ctl(&["inspect"])["tabs"][0]["panes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let confirmed = h.ctl(&[
        "close",
        "--instance",
        instance,
        "--pane",
        &pane,
        "--confirmation",
        close["confirmation"].as_str().unwrap(),
        "--confirm-shells",
        "--request-id",
        "close-confirmed",
    ]);
    assert_eq!(confirmed["state"], "complete", "{confirmed}");
    assert_eq!(
        h.ctl(&["inspect"])["tabs"][0]["panes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    h.quit();
}

#[test]
fn ctl_self_uses_corral_identity_and_late_start_preserves_user_draft() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    let initial = h.ctl(&["inspect"]);
    let instance = initial["instance"].as_str().unwrap();
    let caller = [
        ("CORRAL_NAME", "p/a"),
        ("CORRAL_INSTANCE", "abcdef123"),
        ("SADDLE_INSTANCE", "stale-shell-hint"),
        ("SADDLE_PANE", "999"),
    ];
    let own = h.ctl_as(&["inspect"], &caller);
    assert_eq!(own["caller"]["pane"], initial["active_pane"], "{own}");
    let invalid = h.ctl_as(
        &[
            "open",
            "--instance",
            instance,
            "--place",
            "tab",
            "--agent",
            "p/b",
        ],
        &[("CORRAL_NAME", "p/a"), ("CORRAL_INSTANCE", "reused-name")],
    );
    assert_eq!(invalid["ok"], false);
    let opened = h.ctl_as(
        &[
            "open",
            "--place",
            "right",
            "--agent",
            "p/b",
            "--request-id",
            "show-b",
        ],
        &caller,
    );
    assert_eq!(h.operation(&opened)["state"], "complete");
    let unchanged = h.ctl(&["inspect"]);
    assert_eq!(unchanged["active_pane"], initial["active_pane"]);
    assert_eq!(unchanged["focus"], initial["focus"]);
    h.see("p/b READY");
    h.click("Agent · p/b");
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    let project = h.dir.path().join("project-one");
    let args = [
        "open",
        "--instance",
        instance,
        "--place",
        "down",
        "--name",
        "p/new-exact",
        "--cwd",
        project.to_str().unwrap(),
        "--request-id",
        "new-one",
        "--",
        "program",
        "one argument",
        "$(literal)",
    ];
    let start = h.ctl_as(&args, &caller);
    assert_eq!(start["relative_to"]["pane"], initial["active_pane"]);
    h.event("start p/new-exact");
    let repeated = h.ctl_as(&args, &caller);
    assert_eq!(repeated["pane"], start["pane"]);
    h.send(b"\x1dn");
    h.see("New agent");
    h.click("Regular");
    h.click("Name");
    h.send(b"\x15my-draft");
    h.see("my-draft");
    let busy = h.ctl_as(
        &[
            "open",
            "--instance",
            instance,
            "--place",
            "left",
            "--agent",
            "p/b",
        ],
        &caller,
    );
    assert_eq!(busy["error"]["code"], "busy");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    let completed = h.operation(&start);
    assert_eq!(completed["state"], "complete", "{completed}");
    assert_eq!(completed["agent_created"]["name"], "p/new-exact-actual");
    assert!(h.contents().contains("my-draft"));
    let after = h.ctl(&["inspect"]);
    assert_eq!(after["active_pane"], opened["pane"]);
    let argv: serde_json::Value =
        serde_json::from_str(h.log("start-args").lines().next().unwrap()).unwrap();
    assert_eq!(
        argv,
        serde_json::json!([
            "start",
            "p/new-exact",
            "--cwd",
            project.canonicalize().unwrap().to_str().unwrap(),
            "--label",
            "role=regular",
            "--",
            "program",
            "one argument",
            "$(literal)"
        ])
    );
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|l| *l == "start p/new-exact")
            .count(),
        1
    );
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("my-draft"));
    h.quit();
}

#[test]
fn terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Synthetic title");
    let initial = h.ctl(&["inspect"]);
    h.click("Split ▾");
    h.click("Below ↓");
    h.click_in("Open content below", "New agent…");
    h.see("Create agent");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Create agent"));
    assert_eq!(h.ctl(&["inspect"])["tabs"], initial["tabs"]);
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "New agent…");
    h.click("Create agent");
    h.see("agents/main-actual READY");
    let layout = h.ctl(&["inspect"]);
    assert_eq!(layout["tabs"][0]["panes"].as_array().unwrap().len(), 2);
    assert_eq!(layout["tabs"][0]["layout"]["value"]["vertical"], false);
    h.click("│ + │");
    h.click_in("Open content in a new tab", "Terminal");
    h.see("SHELL READY");
    let shell = h.ctl(&["inspect"]);
    let instance = shell["instance"].as_str().unwrap();
    let pane = shell["active_pane"].as_u64().unwrap().to_string();
    let env: serde_json::Value = serde_json::from_str(&h.log(&format!("shell-{pane}"))).unwrap();
    let own = h.ctl_as(
        &["inspect"],
        &[
            ("SADDLE_INSTANCE", instance),
            ("SADDLE_PANE", &pane),
            ("SADDLE_REVISION", env["SADDLE_REVISION"].as_str().unwrap()),
        ],
    );
    assert_eq!(own["caller"]["pane"], shell["active_pane"]);
    h.click("Close pane");
    h.see("End these running terminals");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("End these running terminals"));
    h.send(b"x");
    h.see("SHELL INPUT:x");
    h.send(b"\x1dq");
    h.see("End these running terminals");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("End these running terminals"));
    h.send(b"\x1b[Zexit");
    h.see("exited 7");
    h.click("Close pane");
    h.until(|h| !h.contents().contains("exited 7"));
    assert!(!h.contents().contains("End these running terminals"));
    h.quit();
}

#[test]
fn ctl_mixed_tab_confirmation_is_atomic_and_expires_after_layout_change() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    let initial = h.ctl(&["inspect"]);
    let instance = initial["instance"].as_str().unwrap();
    let tab = initial["active_tab"].as_u64().unwrap().to_string();
    let shell = h.ctl(&[
        "open",
        "--instance",
        instance,
        "--relative-to",
        "active",
        "--place",
        "right",
        "--shell",
        "--cwd",
        h.dir.path().to_str().unwrap(),
    ]);
    assert_eq!(h.operation(&shell)["state"], "complete");
    let first = h.ctl(&["close", "--instance", instance, "--tab", &tab]);
    assert_eq!(first["state"], "confirmation_required");
    assert_eq!(first["targets"].as_array().unwrap().len(), 2);
    assert!(!h.log("events").contains("detaching p/a"));
    let b = h.ctl(&[
        "open",
        "--instance",
        instance,
        "--relative-to",
        "active",
        "--place",
        "down",
        "--agent",
        "p/b",
    ]);
    assert_eq!(h.operation(&b)["state"], "complete");
    let stale = h.ctl(&[
        "close",
        "--instance",
        instance,
        "--tab",
        &tab,
        "--confirmation",
        first["confirmation"].as_str().unwrap(),
        "--confirm-shells",
    ]);
    assert_eq!(stale["ok"], false);
    assert_eq!(
        h.ctl(&["inspect"])["tabs"][0]["panes"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let fresh = h.ctl(&["close", "--instance", instance, "--tab", &tab]);
    let done = h.ctl(&[
        "close",
        "--instance",
        instance,
        "--tab",
        &tab,
        "--confirmation",
        fresh["confirmation"].as_str().unwrap(),
        "--confirm-shells",
    ]);
    assert_eq!(done["state"], "complete", "{done}");
    h.event("detached p/a");
    h.event("detached p/b");
    assert!(!h.log("events").contains("stop "));
    h.quit();
}

#[test]
fn ctl_closed_start_target_records_creation_without_attaching_or_stopping() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Synthetic title");
    let initial = h.ctl(&["inspect"]);
    let instance = initial["instance"].as_str().unwrap();
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    let start = h.ctl(&[
        "open",
        "--instance",
        instance,
        "--relative-to",
        "active",
        "--place",
        "tab",
        "--name",
        "p/orphan",
        "--",
        "codex",
        "--yolo",
    ]);
    h.event("start p/orphan");
    let pane = start["pane"].as_u64().unwrap().to_string();
    let close = h.ctl(&["close", "--instance", instance, "--pane", &pane]);
    assert_eq!(close["state"], "complete");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    let result = h.operation(&start);
    assert_eq!(result["state"], "target_invalid", "{result}");
    assert_eq!(result["agent_created"]["name"], "p/orphan-actual");
    assert!(!h.log("events").contains("attach p/orphan-actual"));
    assert!(!h.log("events").contains("stop "));
    h.quit();
}

#[test]
fn replacing_a_running_shell_confirms_then_keeps_the_same_pane() {
    let mut h = Harness::start_with_projects(include_str!("fixtures/drover.py"), true);
    h.see("Synthetic title");
    h.click("│ + │");
    h.click_in("Open content in a new tab", "Terminal");
    h.see("SHELL READY");
    let initial = h.ctl(&["inspect"]);
    h.send(b"\x1d\r");
    h.see("End these running terminals");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("End these running terminals"));
    assert_eq!(h.ctl(&["inspect"])["tabs"], initial["tabs"]);
    h.send(b"\r");
    h.see("End these running terminals");
    h.click("End shells y");
    h.see("p/a READY");
    assert_eq!(h.ctl(&["inspect"])["active_pane"], initial["active_pane"]);
    h.send(b"A");
    h.event("input p/a 41");
    assert!(!h.log("events").contains("stop "));
    h.quit();
}

#[test]
fn control_socket_is_private_and_slow_clients_do_not_block_terminal_input() {
    use std::os::unix::{fs::PermissionsExt, net::UnixStream};
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    let view = h.ctl(&["inspect"]);
    let instance = view["instance"].as_str().unwrap();
    let run = h.dir.path().join("run");
    let socket = run.join(format!("{instance}.sock"));
    assert_eq!(
        std::fs::metadata(&run).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let mut slow: Vec<_> = (0..4)
        .map(|_| UnixStream::connect(&socket).unwrap())
        .collect();
    for stream in &mut slow {
        stream.write_all(b"{").unwrap();
    }
    h.send(b"Z");
    h.event("input p/a 5a");
    drop(slow);
    let bad = h.ctl(&[
        "open",
        "--instance",
        instance,
        "--relative-to",
        "999999",
        "--place",
        "right",
        "--shell",
    ]);
    assert_eq!(bad["ok"], false);
    h.quit();
    assert!(!socket.exists());
    assert_eq!(
        h.ctl(&["inspect", "--instance", instance])["error"]["code"],
        "instance_unavailable"
    );
}

#[test]
fn t20_rework_failed_current_start_preserves_original_identity() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    let caller = [("CORRAL_NAME", "p/a"), ("CORRAL_INSTANCE", "abcdef123")];
    let before = h.ctl_as(&["inspect"], &caller);
    assert!(before["caller"]["pane"].is_u64(), "{before}");
    std::fs::write(h.dir.path().join("fail-start"), "").unwrap();
    h.send(b"\x1dn\x13");
    h.see("synthetic start failed");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    let after = h.ctl_as(&["inspect"], &caller);
    h.send(b"\x1b[ZA");
    h.event("input p/a 41");
    h.quit();
    assert_eq!(
        after["caller"]["pane"], before["caller"]["pane"],
        "original live attach lost its caller identity"
    );
    for field in ["cwd", "corral_instance", "agent", "state"] {
        assert_eq!(
            after["tabs"][0]["panes"][0][field], before["tabs"][0]["panes"][0][field],
            "{field}"
        );
    }
}
#[test]
fn t20_rework_cancelled_picker_new_preserves_hidden_agents_draft() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"n");
    h.click("Regular");
    h.click("Name");
    h.send(b"\x15preserve-my-draft");
    h.see("preserve-my-draft");
    h.click("Advanced");
    h.click("Command");
    h.send(b"\x15program 'draft argument'\tdraft prompt");
    h.see("draft prompt");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.see("Create agent");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Create agent"));
    h.send(b"\x1dn");
    h.see("Create agent");
    let retained = ["preserve-my-draft", "draft argument", "draft prompt"]
        .iter()
        .all(|text| h.contents().contains(text));
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    h.quit();
    assert!(
        retained,
        "cancelled picker overwrote the original Agents New draft"
    );
}
#[test]
fn t20_rework_delayed_picker_attach_preserves_later_agents_focus() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.click("│ + │");
    h.click_in("Open content in a new tab", "p/b");
    h.see("Attaching p/b");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    let before = h.ctl(&["inspect"]);
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.see("p/b READY");
    let after = h.ctl(&["inspect"]);
    h.quit();
    assert_eq!(
        after["focus"], before["focus"],
        "async completion stole the later Agents focus"
    );
}

#[test]
fn t20_rework_failed_attach_does_not_report_display_complete() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("fail-attach"), "").unwrap();
    let start = h.ctl(&[
        "open",
        "--relative-to",
        "active",
        "--place",
        "right",
        "--agent",
        "p/b",
    ]);
    assert_eq!(start["accepted"], true, "{start}");
    h.event("attach p/b");
    h.until(|h| {
        let v = h.ctl(&["inspect"]);
        matches!(
            v["tabs"][0]["panes"][1]["state"].as_str(),
            Some("failed" | "disconnected")
        )
    });
    let result = h.operation(&start);
    h.quit();
    assert_ne!(
        result["state"], "complete",
        "failed corral attach was recorded as display complete"
    );
    assert_eq!(result["ok"], false, "{result}");
    assert_eq!(result["exit_code"], 1, "{result}");
}

#[test]
fn t20_rework_failed_status_and_reclaim_preserve_displayed_metadata() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    let caller = [("CORRAL_NAME", "p/a"), ("CORRAL_INSTANCE", "abcdef123")];
    let before = h.ctl_as(&["inspect"], &caller);
    let other = h.dir.path().join("t20-other-cwd");
    std::fs::create_dir(&other).unwrap();
    std::fs::write(
        h.dir.path().join("metadata.json"),
        serde_json::json!({
            "p/b": {"cwd":other,"instance":"other-instance"},
            "p/taken": {"cwd":other,"instance":"taken-instance"}
        })
        .to_string(),
    )
    .unwrap();
    h.send(b"\x1djj");
    h.see("t20-other-cwd"); // Public listing has absorbed the distinct target metadata.
    h.send(b"\r");
    h.see("attached elsewhere");
    let failed = h.ctl_as(&["inspect"], &caller);
    assert_eq!(failed["caller"]["pane"], before["caller"]["pane"]);
    for field in ["cwd", "corral_instance", "agent", "state"] {
        assert_eq!(
            failed["tabs"][0]["panes"][0][field], before["tabs"][0]["panes"][0][field],
            "{field}"
        );
    }
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"k\r");
    h.see("attaching p/b");
    h.send(b"k\r");
    h.see("Input ▸ p/a");
    let reclaimed = h.ctl_as(&["inspect"], &caller);
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.send(b"A");
    h.event("input p/a 41");
    assert_eq!(reclaimed["caller"]["pane"], before["caller"]["pane"]);
    assert_eq!(
        reclaimed["tabs"][0]["panes"][0]["cwd"],
        before["tabs"][0]["panes"][0]["cwd"]
    );
    h.quit();
    assert!(!h.log("events").contains("attach p/b"));
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn t20_rework_reclaimed_start_preserves_original_metadata_after_late_result() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    let caller = [("CORRAL_NAME", "p/a"), ("CORRAL_INSTANCE", "abcdef123")];
    let before = h.ctl_as(&["inspect"], &caller);
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    h.send(b"\x1dn\x13");
    h.event("start agents/main");
    h.see("Starting…");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    h.send(b"\r");
    h.see("Input ▸ p/a");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    h.see("target closed or replaced");
    let after = h.ctl_as(&["inspect"], &caller);
    h.send(b"A");
    h.event("input p/a 41");
    h.quit();
    assert_eq!(after["caller"]["pane"], before["caller"]["pane"]);
    assert_eq!(
        after["tabs"][0]["panes"][0]["cwd"],
        before["tabs"][0]["panes"][0]["cwd"]
    );
    assert!(!h.log("events").contains("attach agents/main-actual"));
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn t20_rework_completed_picker_new_restores_hidden_agents_draft() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"n");
    h.click("Regular");
    h.click("Name");
    h.send(b"\x15original-draft");
    h.see("original-draft");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.see("Create agent");
    h.send(b"\x13");
    h.see("agents/main-actual READY");
    h.send(b"\x1dn");
    h.see("Create agent");
    h.see("original-draft");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    h.quit();
    assert_eq!(h.log("start-args").lines().count(), 1);
}

#[test]
fn t20_rework_late_picker_start_does_not_clear_reopened_agents_draft() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"n");
    h.click("Regular");
    h.click("Name");
    h.send(b"\x15original-draft");
    h.see("original-draft");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.send(b"\x13");
    h.event("start agents/main");
    h.see("Starting…");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    h.send(b"\x1dn");
    h.see("original-draft");
    h.click("Name");
    h.send(b"\x15original-draft-edited");
    h.see("original-draft-edited");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    h.until(|h| h.ctl(&["inspect"])["tabs"][1]["panes"][0]["state"] == "running");
    h.see("original-draft-edited");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    h.quit();
}

#[test]
fn t20_rework_delayed_agents_attach_preserves_later_agents_focus() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\r");
    h.see("attaching p/a");
    h.send(b"\x1d"); // Explicit focus intent, even though Focus is already Agents.
    h.see("Input ▸ Agents");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.see("p/a READY");
    let after = h.ctl(&["inspect"]);
    h.send(b"n");
    h.see("Create agent");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Create agent"));
    h.quit();
    assert_eq!(after["focus"], "agents");
    assert!(!h.log("events").contains("input p/a 6e"));
}

#[test]
fn t20_rework_completed_request_tracks_late_attach_exit_and_closed_target() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    let start = h.ctl(&[
        "open",
        "--relative-to",
        "active",
        "--place",
        "right",
        "--agent",
        "p/b",
        "--focus",
    ]);
    assert_eq!(h.operation(&start)["pty"], "running");
    h.see("p/b READY");
    h.send(b"X");
    h.until(|h| h.ctl(&["inspect"])["tabs"][0]["panes"][1]["state"] == "failed");
    let result = h.operation(&start);
    assert_eq!(result["state"], "failed", "{result}");
    assert_eq!(result["pty"], "failed");
    assert_eq!(result["exit_code"], 1);
    let again = h.ctl(&[
        "open",
        "--relative-to",
        "active",
        "--place",
        "down",
        "--agent",
        "p/a",
    ]);
    assert_eq!(h.operation(&again)["pty"], "running");
    let pane = again["pane"].as_u64().unwrap().to_string();
    let closed = h.ctl(&[
        "close",
        "--instance",
        again["instance"].as_str().unwrap(),
        "--pane",
        &pane,
    ]);
    assert_eq!(closed["ok"], true, "{closed}");
    let closed_result = h.operation(&again);
    assert_eq!(closed_result["state"], "target_invalid", "{closed_result}");
    assert_ne!(closed_result["pty"], "running");
    assert_eq!(h.operation(&start)["exit_code"], 1);
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn t20_rework_hidden_busy_agents_draft_keeps_its_own_start_result() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    std::fs::write(h.dir.path().join("fail-start"), "").unwrap();
    h.send(b"n");
    h.click("Regular");
    h.click("Name");
    h.send(b"\x15busy-original\x13");
    h.event("start agents/busy-original");
    h.see("Starting…");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.see("Create agent");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    // Inspect is serviced after action results, so the failed reservation acknowledges
    // that the original form's result has arrived while the temporary form is open.
    h.until(|h| h.ctl(&["inspect"])["tabs"][0]["panes"][0]["state"] == "disconnected");
    assert!(!h.popup("New agent").contains("synthetic start failed"));
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    h.send(b"\x1dn");
    h.see("busy-original");
    h.see("synthetic start failed");
    h.see("Create agent"); // Original busy Ticket cleared, allowing retry.
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    h.quit();
}

#[test]
fn t20_r1_pending_new_pane_keeps_known_source_cwd_for_shell() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    let source_cwd = h.dir.path().join("agent-project");
    std::fs::create_dir(&source_cwd).unwrap();
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    let start = h.ctl(&[
        "open",
        "--relative-to",
        "active",
        "--place",
        "tab",
        "--focus",
        "--name",
        "p/cwd-probe",
        "--cwd",
        source_cwd.to_str().unwrap(),
        "--",
        "synthetic-program",
    ]);
    assert_eq!(start["accepted"], true, "{start}");
    h.event("start p/cwd-probe");
    let pane = start["pane"].as_u64().unwrap().to_string();
    let initial = h.ctl(&["inspect"]);
    let shell = h.ctl(&[
        "open",
        "--relative-to",
        &pane,
        "--place",
        "right",
        "--shell",
    ]);
    assert_eq!(h.operation(&shell)["pty"], "running", "{shell}");
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.see("Create agent");
    let form_uses_source = h.popup("New agent").contains("Project: agent-project");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    let shell_pane = shell["pane"].as_u64().unwrap().to_string();
    let instance = shell["instance"].as_str().unwrap();
    let close = h.ctl(&["close", "--pane", &shell_pane, "--instance", instance]);
    let confirmed = h.ctl(&[
        "close",
        "--pane",
        &shell_pane,
        "--instance",
        instance,
        "--confirmation",
        close["confirmation"].as_str().unwrap(),
        "--confirm-shells",
    ]);
    assert_eq!(confirmed["ok"], true, "{confirmed}");
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    assert_eq!(h.operation(&start)["state"], "complete");
    h.quit();
    assert!(
        form_uses_source,
        "location New forgot the pending source directory"
    );
    assert_eq!(shell["cwd_source"], "source_pane");
    assert_eq!(initial["tabs"][1]["panes"][0]["cwd_source"], "pane");
    assert_eq!(
        shell["cwd"], start["cwd"],
        "new shell forgot the source pane's already known agent project"
    );
    assert_eq!(
        initial["tabs"][1]["panes"][0]["cwd"], start["cwd"],
        "pending pane inspection reports an unrelated Tasks cwd"
    );
}

#[test]
fn t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases() {
    let mut h = Harness::start();
    let old_cwd = h.dir.path().join("original-project");
    std::fs::create_dir(&old_cwd).unwrap();
    std::fs::write(
        h.dir.path().join("metadata.json"),
        serde_json::json!({
            "p/a": {"cwd":old_cwd,"instance":"abcdef123"}
        })
        .to_string(),
    )
    .unwrap();
    h.see("original-project"); // The public listing has absorbed A's known directory.
    h.send(b"\r");
    h.see("p/a READY");
    let before = h.ctl(&["inspect"]);
    let old_pane = before["active_pane"].as_u64().unwrap().to_string();
    std::fs::write(h.dir.path().join("hold-start"), "").unwrap();
    std::fs::write(h.dir.path().join("hold-detach"), "").unwrap();
    h.send(b"\x1dn\x13"); // Current-pane New uses the different Tasks project.
    h.event("start agents/main");
    h.see("Starting…");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    let during_start = h.ctl(&["inspect"]);
    assert_eq!(during_start["tabs"][0]["panes"][0]["state"], "starting");
    let shell = h.ctl(&[
        "open",
        "--relative-to",
        &old_pane,
        "--place",
        "right",
        "--shell",
        "--focus",
    ]);
    assert_eq!(h.operation(&shell)["pty"], "running", "{shell}");
    h.see("SHELL READY");
    h.send(b"exit");
    h.until(|h| h.ctl(&["inspect"])["tabs"][0]["panes"][1]["state"] == "exited");
    let shell_pane = shell["pane"].as_u64().unwrap().to_string();
    let closed = h.ctl(&[
        "close",
        "--instance",
        shell["instance"].as_str().unwrap(),
        "--pane",
        &shell_pane,
    ]);
    assert_eq!(closed["ok"], true, "{closed}");

    // The old attach is still displayed while Viewer waits for it to detach.
    std::fs::remove_file(h.dir.path().join("hold-start")).unwrap();
    h.event("detaching p/a");
    let during_attach = h.ctl(&["inspect"]);
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.see("Create agent");
    let form_uses_original = h.popup("New agent").contains("Project: original-project");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Input ▸ New agent"));
    std::fs::remove_file(h.dir.path().join("hold-detach")).unwrap();
    h.see("agents/main-actual READY");
    let after = h.ctl(&["inspect"]);
    h.quit();

    assert_eq!(during_attach["tabs"][0]["panes"][0]["state"], "attaching");
    for snapshot in [during_start, during_attach] {
        let pane = &snapshot["tabs"][0]["panes"][0];
        assert_eq!(pane["cwd"], before["tabs"][0]["panes"][0]["cwd"]);
        assert_eq!(pane["cwd_source"], "pane");
        assert_eq!(pane["corral_instance"], "abcdef123");
    }
    assert_eq!(shell["cwd"], old_cwd.to_str().unwrap());
    assert_eq!(shell["cwd_source"], "source_pane");
    assert!(
        form_uses_original,
        "location New inherited the replacement's directory"
    );
    let args: Vec<String> =
        serde_json::from_str(h.log("start-args").lines().next().unwrap()).unwrap();
    assert_eq!(after["tabs"][0]["panes"][0]["cwd"], args[3]);
    assert_ne!(
        after["tabs"][0]["panes"][0]["cwd"],
        before["tabs"][0]["panes"][0]["cwd"]
    );
}

#[test]
fn fold_toggles_by_key_and_bar_and_background_tab_agents_keep_the_local_mark() {
    let mut h = Harness::start();
    let titles = |h: &Harness| h.contents().matches("Synthetic title").count();
    assert_eq!(titles(&h), 3);
    h.send(b"z");
    h.see("z Expand");
    h.until(|h| titles(h) == 1); // Only the selected p/a stays expanded.
    h.click("z Expand");
    h.see("z Fold");
    h.until(|h| titles(h) == 3);
    h.send(b"\r");
    h.see("p/a READY");
    // Shown in a background tab, p/a is still displayed by this saddle.
    h.click("│ + │");
    h.click_in("Open content in a new tab", "p/b");
    h.see("p/b READY");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    let row = |h: &Harness, name: &str| {
        h.contents()
            .lines()
            .find(|line| line.contains(&format!("○ {name} ")))
            .unwrap_or_default()
            .to_owned()
    };
    h.until(|h| row(h, "a").contains('⦿'));
    assert!(h.contents().contains("⦿ 56y"));
    // p/taken has a public attach count but no display here.
    assert!(!row(&h, "taken").contains('⦿'), "{}", h.contents());
    h.quit();
}

#[test]
fn t23_search_opens_agents_by_name_or_project_and_jumps_to_open_panes() {
    let mut h = Harness::start();
    std::fs::write(
        h.dir.path().join("metadata.json"),
        r#"{"p/b":{"cwd":"/tmp/zebra-project"}}"#,
    )
    .unwrap();
    h.see("/ Search");
    // `/` in Agents opens the popup; typing filters and never reaches a terminal.
    h.send(b"/");
    h.see("Search agents");
    h.see("Input ▸ Search agents");
    h.send(b"zebra");
    h.until(|h| {
        let popup = h.popup("Search agents");
        popup.contains("p/b") && !popup.contains("p/a") && !popup.contains("p/taken")
    });
    h.send(b"\r");
    h.see("p/b READY");
    h.until(|h| !h.contents().contains("Search agents"));
    h.send(b"B");
    h.event("input p/b 42");
    // Esc cancels and keeps the display target; nothing is attached.
    h.send(b"\x1d/a");
    h.see("Search agents");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Search agents"));
    h.see("Input ▸ Agents");
    // The bar entry opens it too; clicking a candidate opens that agent by its name.
    h.click("/ Search");
    h.see("Search agents");
    h.send(b"p/a");
    h.until(|h| {
        let popup = h.popup("Search agents");
        popup.contains("p/a") && !popup.contains("p/b")
    });
    h.click_in("Search agents", "p/a  demo");
    h.see("p/a READY");
    h.until(|h| !h.contents().contains("Search agents"));
    // Put p/b beside p/a and focus p/a; searching p/b jumps to its open pane.
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "p/b");
    h.see("p/b READY");
    h.click("Agent · p/a");
    h.see("Input ▸ p/a");
    h.send(b"\x1d/");
    h.see("Search agents");
    h.send(b"p/");
    h.until(|h| {
        let popup = h.popup("Search agents");
        popup.contains("p/taken") && popup.matches("Open").count() == 2
    });
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Search agents"));
    let attaches = |h: &Harness| {
        h.log("events")
            .lines()
            .filter(|l| l.starts_with("attach "))
            .count()
    };
    let before = attaches(&h);
    h.send(b"/p/b");
    h.see("Search agents");
    h.until(|h| h.popup("Search agents").contains("p/b  zebra-project"));
    h.send(b"\r");
    h.until(|h| !h.contents().contains("Search agents"));
    h.see("Input ▸ p/b");
    h.send(b"Q");
    h.event("input p/b 51");
    assert_eq!(attaches(&h), before, "an open agent is not attached again");
    h.quit();
    let events = h.log("events");
    let inputs: Vec<_> = events.lines().filter(|l| l.starts_with("input ")).collect();
    assert_eq!(
        inputs,
        ["input p/b 42", "input p/b 51"],
        "search typing must not reach a terminal"
    );
    assert!(!events.contains("stop "));
}

#[test]
fn t23_zoom_fills_the_terminal_area_and_restore_keeps_layout_and_focus() {
    let mut h = Harness::start();
    h.send(b"\r");
    h.see("p/a READY");
    h.event("size p/a ");
    let full = h
        .log("events")
        .lines()
        .find(|l| l.starts_with("size p/a "))
        .unwrap()
        .trim_start_matches("size p/a ")
        .to_owned();
    // A single pane has nothing to zoom.
    h.see("Split ▾");
    assert!(!h.contents().contains("Zoom"), "{}", h.contents());
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "p/b");
    h.see("p/b READY");
    h.see("Zoom");
    let layout = |h: &Harness| {
        (
            h.locate("Agent · p/a", 0),
            h.locate("Agent · p/b", 0),
            h.locate("Agents · ", 0),
        )
    };
    let before = layout(&h);
    let (a, b, _) = before;
    assert!(a.unwrap().1 == b.unwrap().1 && a.unwrap().0 < b.unwrap().0);
    h.click("Zoom");
    h.see("Restore");
    h.until(|h| !h.contents().contains("Agent · p/a"));
    h.event(&format!("size p/b {full}"));
    let zoomed = h.locate("Agent · p/b", 0).unwrap();
    assert_eq!(
        zoomed.0,
        a.unwrap().0,
        "zoomed pane starts where the split did"
    );
    assert!(h.contents().contains("Agents · 3"), "Agents stay visible");
    assert_eq!(h.contents().matches(" ×│").count(), 1, "tab strip stays");
    // Terminal keys still go to the zoomed pane; the hidden pane keeps running.
    h.send(b"Z");
    h.event("input p/b 5a");
    h.click("Restore");
    h.until(|h| h.contents().contains("Agent · p/a"));
    h.see("Zoom");
    assert_eq!(
        layout(&h),
        before,
        "restore returns the same split and ratio"
    );
    h.send(b"Y");
    h.event("input p/b 59");
    h.quit();
    let events = h.log("events");
    assert_eq!(events.lines().filter(|l| *l == "attach p/a").count(), 1);
    assert_eq!(events.lines().filter(|l| *l == "attach p/b").count(), 1);
    let quit_at = events.find("detaching").unwrap();
    assert!(
        !events[..quit_at].contains("detach"),
        "zoom must not reattach or close sessions: {events}"
    );
    assert!(
        !events.contains("1b5b3c"),
        "native controls send no mouse input"
    );
}

#[test]
fn t23_search_click_on_an_open_agent_consumes_the_whole_mouse_gesture() {
    let mut h = Harness::start();
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"\x1d/p/a");
    h.see("Input ▸ Search agents");
    h.until(|h| {
        let popup = h.popup("Search agents");
        popup.contains("p/a  demo") && !popup.contains("p/b") && !popup.contains("p/taken")
    });
    // Press on the row body over the Viewer; the popup closes and p/a takes focus before the
    // release, which must not reach its terminal.
    let (col, row) = h.locate("p/a  demo", 0).unwrap();
    let (x, y) = (col + 21, row + 1);
    h.send(format!("\x1b[<0;{x};{y}M").as_bytes());
    h.see("Input ▸ p/a");
    h.send(format!("\x1b[<0;{x};{y}m").as_bytes());
    h.send(b"Q");
    h.event("input p/a 51");
    h.quit();
    let events = h.log("events");
    let inputs: Vec<_> = events.lines().filter(|l| l.starts_with("input ")).collect();
    assert_eq!(inputs, ["input p/a 51"], "search click leaked mouse bytes");
    assert_eq!(events.lines().filter(|l| *l == "attach p/a").count(), 1);
}

/// Two registered projects share task number T3: awaiting release in one, failed in the other.
const T22_QUEUE: &str = r#"#!/usr/bin/env python3
import json, sys
from pathlib import Path
root = Path(__file__).parent
args = sys.argv[1:]
with (root / 'queue-events').open('a') as f:
    f.write(json.dumps(args) + '\n')
if (Path.cwd() / 'fail-list').exists():
    print('synthetic project read failure', file=sys.stderr)
    sys.exit(4)
mode = dict(loop=False, gate=True)
state_file = Path.cwd() / 'queue-state.json'
if state_file.exists():
    state = json.loads(state_file.read_text())
elif Path.cwd().name == 'project-one':
    state = dict(mode=mode, paused=False, current=None,
        awaiting=dict(id='T3', title='Ready to ship', body='awaiting body text'),
        pending=[dict(id='T4', title='Next up', body='')],
        history=[dict(id='T2', title='Old done', body='', status='done')])
else:
    state = dict(mode=mode, paused=False, current=None, awaiting=None, pending=[],
        history=[dict(id='T3', title='Broke build', body='failure body text', status='failed', reason='check failed'),
                 dict(id='T1', title='Fine one', body='fine body text', status='done')])
if args == ['list', '--json']:
    print(json.dumps(state))
    sys.exit(0)
print('FORBIDDEN CLI: ' + repr(args), file=sys.stderr)
sys.exit(99)
"#;

#[test]
fn t22_attention_gathers_agents_and_every_project_and_opens_targets() {
    let mut h = Harness::start_with_projects(T22_QUEUE, true);
    // p/taken starts waiting for input; p/b finishes its turn unseen, which is a new reply.
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"idle","p/b":"idle","p/taken":"blocked"}"#,
    )
    .unwrap();
    h.see("Attention · 4");
    h.send(b"a");
    h.see("Input ▸ Attention");
    h.until(|h| {
        let popup = h.popup("Attention ━");
        popup.contains("New reply") && popup.contains("project-two · T3")
    });
    let popup = h.popup("Attention ━");
    let at = |text: &str| {
        popup
            .find(text)
            .unwrap_or_else(|| panic!("{text}:\n{popup}"))
    };
    assert!(at("Needs attention") < at("p/taken") && at("p/taken") < at("New replies"));
    assert!(at("Waiting for input") < at("New replies"));
    assert!(at("project-one · T3") < at("New replies") && popup.contains("Awaiting release"));
    assert!(at("project-two · T3") < at("New replies") && popup.contains("Failed"));
    assert!(at("New replies") < at("p/b"), "{popup}");
    assert_eq!(popup.matches("p/b").count(), 1, "{popup}");
    // Opening a task switches Tasks to its project and selects it by identity.
    h.click_in("Attention ━", "project-two · T3");
    h.until(|h| !h.contents().contains("Attention ━"));
    h.see("Input ▸ Tasks");
    h.see("failure body text");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    // The row click that opened Tasks is over; the next click on an agent row still attaches.
    h.click("○ a ");
    h.see("p/a READY");
    // The entry opens Attention by mouse too.
    // Mark seen hides only that project's failed T3; the other project's T3 stays.
    h.click("Attention · 4");
    h.see("Input ▸ Attention");
    h.send(b"\x1b[B\x1b[B");
    h.send(b"m");
    h.until(|h| !h.popup("Attention ━").contains("project-two · T3"));
    assert!(h.popup("Attention ━").contains("project-one · T3"));
    h.see("Attention · 3");
    // An agent row opens that agent; viewing it clears its new reply.
    h.click_in("Attention ━", "p/b");
    h.see("p/b READY");
    h.see("Attention · 2");
    // Waiting and awaiting release leave only when their public state changes.
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"idle","p/b":"idle","p/taken":"idle"}"#,
    )
    .unwrap();
    h.see("Attention · 1");
    std::fs::write(
        h.dir.path().join("project-one/queue-state.json"),
        r#"{"mode":{"loop":false,"gate":true},"paused":false,"current":null,"awaiting":null,"pending":[],"history":[]}"#,
    )
    .unwrap();
    h.see("Attention · 0");
    // A failed read is reported, never shown as nothing to do.
    std::fs::write(h.dir.path().join("project-two/fail-list"), "").unwrap();
    h.see("Attention · 1");
    h.send(b"\x1da");
    h.see("Input ▸ Attention");
    h.until(|h| {
        let popup = h.popup("Attention ━");
        popup.contains("project-two") && popup.contains("Read failed")
    });
    h.send(b"\x1b");
    h.quit();
    let queue = h.log("queue-events");
    assert!(
        queue
            .lines()
            .all(|l| l == r#"["list", "--json"]"# || l == NOTIFICATION_STATUS),
        "{queue}"
    );
    let events = h.log("events");
    assert!(
        !events.contains("stop ") && !events.contains("input "),
        "{events}"
    );
}

#[test]
fn history_keeps_its_keys_and_mouse_from_the_agent_until_esc_returns_to_live_input() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    for i in 1..=20 {
        h.send(b"C");
        h.until(|h| h.log("events").matches("input p/a 43").count() == i);
    }
    h.until(|h| h.contents().matches("AGENT COLORS").count() > 5);
    h.until(|h| !h.contents().contains("p/a READY"));
    h.click(" History ");
    h.see(" Live Esc ");
    // Nothing selected yet, so this does not touch the clipboard.
    h.click(" Copy ");
    h.see("select text first");
    h.send(b"\x1b[5~/READY");
    h.see("/READY");
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"n\x1b[A\x1b[B\x1b[6~");
    let (col, row) = h.locate("AGENT COLORS", 0).unwrap();
    let (x, y) = (col + 1, row + 1);
    h.send(
        format!(
            "\x1b[<0;{x};{y}M\x1b[<32;{};{y}M\x1b[<0;{};{y}m",
            x + 4,
            x + 4
        )
        .as_bytes(),
    );
    h.send(format!("\x1b[<64;{x};{y}M").as_bytes());
    h.send(b"\x1b");
    h.see(" History ");
    h.until(|h| !h.contents().contains(" Live Esc "));
    h.send(b"q");
    h.event("input p/a 71");
    let inputs: Vec<_> = h
        .log("events")
        .lines()
        .filter(|l| l.starts_with("input p/a"))
        .map(str::to_owned)
        .collect();
    assert_eq!(inputs.len(), 21, "{inputs:?}");
    assert_eq!(inputs.last().unwrap(), "input p/a 71");
}

#[test]
fn t25_layout_is_saved_while_open_and_retains_an_exited_agent_on_quit() {
    let mut h = Harness::start();
    h.send(b"\r");
    h.see("p/a READY");
    let path = h.dir.path().join("state/saddle/layout.json");
    h.until(|_| path.exists());
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["version"], 1);
    assert_eq!(saved["tabs"][0]["panes"][0]["content"]["name"], "p/a");
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/b":"working","p/taken":"idle"}"#,
    )
    .unwrap();
    h.event("detached p/a");
    h.until(|h| h.ctl(&["inspect"])["tabs"][0]["panes"][0]["state"] == "disconnected");
    h.quit();
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(saved["tabs"][0]["panes"][0]["content"]["name"], "p/a");
    assert_eq!(saved["tabs"][0]["panes"][0]["content"]["cwd"], "/tmp/demo");
    assert_eq!(
        saved["tabs"][0]["panes"][0]["content"]["instance"],
        "abcdef123"
    );
    assert!(!h.log("events").contains("stop "));
}

fn t25_seed(root: &std::path::Path) {
    use serde_json::json;
    let file = root.join("state/saddle/layout.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let layout = json!({"version":1,"active":4,"tabs":[
        {"id":1,"active":2,"tree":{"kind":"split","value":{"vertical":false,"ratio":30,
            "first":{"kind":"leaf","value":1},"second":{"kind":"leaf","value":2}}},"panes":[
            {"id":1,"content":{"kind":"agent","name":"p/a","cwd":root,"instance":"abcdef123"}},
            {"id":2,"content":{"kind":"agent","name":"p/gone","cwd":root,"instance":"old"}}]},
        {"id":4,"active":4,"tree":{"kind":"leaf","value":4},"panes":[
            {"id":4,"content":{"kind":"shell","cwd":root}}]}
    ]});
    std::fs::write(file, serde_json::to_vec_pretty(&layout).unwrap()).unwrap();
}

#[test]
fn t25_startup_restores_tabs_splits_focus_and_only_reconnects_original_live_agents() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        t25_seed,
    );
    h.see("Tasks · ");
    let state = h.ctl(&["inspect"]);
    assert_eq!(state["tabs"].as_array().unwrap().len(), 2, "{state}");
    assert_eq!(state["active_tab"], 4);
    assert_eq!(state["active_pane"], 4);
    assert_eq!(state["tabs"][0]["layout"]["value"]["ratio"], 30);
    h.event("attach p/a");
    h.until(|h| h.ctl(&["inspect"])["tabs"][0]["panes"][0]["state"] == "running");
    let state = h.ctl(&["inspect"]);
    assert_eq!(state["active_tab"], 4, "restore completion stole focus");
    assert_eq!(state["tabs"][0]["panes"][1]["agent"], "p/gone");
    assert_eq!(state["tabs"][1]["panes"][0]["kind"], "shell");
    h.see("Open terminal");
    assert!(!h.dir.path().join("shell-4").exists());
    assert!(!h.log("events").contains("attach p/gone"));
    assert!(!h.log("events").contains("start "));
    h.quit();
}

#[test]
fn t25_placeholders_create_on_confirmation_choose_existing_and_open_fresh_shell() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        t25_seed,
    );
    h.see("Open terminal");
    h.click("Open terminal");
    h.see("SHELL READY");
    assert_eq!(
        std::fs::read_to_string(h.dir.path().join("shell-cwd-4")).unwrap(),
        h.dir.path().canonicalize().unwrap().display().to_string()
    );
    assert!(!h.contents().contains("SHELL INPUT:"));
    h.send(b"exit");
    h.see("exited 7");
    h.click("p/gone");
    h.see("Create new agent");
    h.click("Create new agent");
    h.see("Input ▸ New agent");
    assert!(
        !h.dir.path().join("start-args").exists(),
        "opening the form must not start anything"
    );
    h.send(b"\x13");
    h.event("start p/gone");
    h.see("p/gone-actual READY");
    let args: Vec<String> =
        serde_json::from_str(h.log("start-args").lines().next().unwrap()).unwrap();
    assert_eq!(
        args,
        [
            "start",
            "p/gone",
            "--cwd",
            h.dir.path().to_str().unwrap(),
            "--label",
            "role=regular",
            "--",
            "codex",
            "--yolo"
        ]
    );
    // An attach exit retains the same location and allows selecting a different live agent.
    h.send(b"X");
    h.see("Choose existing agent");
    h.click("Choose existing agent");
    h.see("Open content here");
    h.send(b"j\r"); // Sorted choices: p/a, p/b; use the modal's keyboard selection.
    h.see("p/b READY");
    let state = h.ctl(&["inspect"]);
    assert_eq!(state["active_pane"], 2);
    assert_eq!(state["tabs"][0]["panes"][1]["agent"], "p/b");
    h.send(b"X");
    h.see("Choose existing agent");
    h.click("Choose existing agent");
    h.see("Open content here");
    h.send(b"\r"); // p/a is already open in the neighboring pane: move that session.
    h.until(|h| {
        h.ctl(&["inspect"])["tabs"][0]["panes"]
            .as_array()
            .unwrap()
            .len()
            == 1
    });
    assert_eq!(h.ctl(&["inspect"])["active_pane"], 1);
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|line| *line == "attach p/a")
            .count(),
        1
    );
    assert!(!h.log("events").contains("stop "));
    h.quit();
}

#[test]
fn t25_changed_or_busy_identity_stays_placeholder_and_corrupt_file_survives_exit() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            t25_seed(root);
            std::fs::write(
                root.join("metadata.json"),
                r#"{"p/a":{"instance":"replacement"}}"#,
            )
            .unwrap();
        },
    );
    h.click("p/gone");
    h.see("identity changed");
    assert!(!h.log("events").contains("attach p/a"));
    h.quit();

    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            t25_seed(root);
            std::fs::write(root.join("p-a"), "other attachment").unwrap();
        },
    );
    h.click("p/gone");
    h.until(|h| {
        h.ctl(&["inspect"])["tabs"][0]["panes"][0]["note"]
            .as_str()
            .is_some_and(|note| note.contains("attached elsewhere"))
    });
    h.see("elsewhere;");
    assert!(!h.log("events").contains("attach p/a"));
    h.quit();

    for original in ["{broken", r#"{"version":999,"active":1,"tabs":[]}"#] {
        let mut h = Harness::start_prepared(
            include_str!("fixtures/drover.py"),
            false,
            "",
            16384,
            |root| {
                t25_seed(root);
                std::fs::write(root.join("state/saddle/layout.json"), original).unwrap();
            },
        );
        h.see("Cannot restore layout");
        let state = h.ctl(&["inspect"]);
        assert_eq!(state["tabs"][0]["panes"][0]["kind"], "empty");
        assert_eq!(state["tabs"].as_array().unwrap().len(), 1);
        h.quit();
        assert_eq!(
            std::fs::read_to_string(h.dir.path().join("state/saddle/layout.json")).unwrap(),
            original
        );
    }
}

#[test]
fn t25_delayed_restore_discards_closed_or_replaced_targets_and_keeps_new_focus() {
    for close in [true, false] {
        let mut h = Harness::start_prepared(
            include_str!("fixtures/drover.py"),
            false,
            "",
            16384,
            |root| {
                t25_seed(root);
                std::fs::write(root.join("hold-status"), "").unwrap();
            },
        );
        h.see("Open terminal");
        h.event("status p/a");
        let state = h.ctl(&["inspect"]);
        let instance = state["instance"].as_str().unwrap().to_owned();
        if close {
            let result = h.ctl(&[
                "close",
                "--instance",
                &instance,
                "--pane",
                "1",
                "--request-id",
                "close-restore",
            ]);
            assert_eq!(result["state"], "complete", "{result}");
        } else {
            // Replace a pending restore using the ordinary New form at that location.
            h.click("p/gone");
            h.click("Agent · p/a");
            h.send(b"\x1dn");
            h.see("Input ▸ New agent");
            h.send(b"\x13");
            h.event("start agents/main");
            h.until(|h| {
                h.ctl(&["inspect"])["tabs"][0]["panes"][0]["agent"] == "agents/main-actual"
            });
        }
        std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
        h.see("Synthetic title");
        h.until(|h| {
            h.ctl(&["inspect"])["tabs"][0]["panes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["state"] != "accepted")
        });
        if !close {
            h.see("agents/main-actual READY");
        }
        assert!(
            !h.log("events").contains("attach p/a"),
            "{}",
            h.log("events")
        );
        if close {
            assert_eq!(h.ctl(&["inspect"])["active_tab"], 4);
        }
        h.quit();
    }
}

#[test]
fn t25_exited_original_status_is_not_reattached() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            t25_seed(root);
            std::fs::write(root.join("agents.json"), r#"{"p/a":"exited"}"#).unwrap();
        },
    );
    h.see("Tasks · ");
    h.until(|h| {
        !matches!(
            h.ctl(&["inspect"])["tabs"][0]["panes"][0]["state"].as_str(),
            Some("accepted" | "attaching")
        )
    });
    let state = h.ctl(&["inspect"]);
    assert_eq!(
        state["tabs"][0]["panes"][0]["state"], "disconnected",
        "{state}"
    );
    assert!(!h.log("events").contains("attach p/a"));
    h.quit();
}

#[test]
fn t25_save_failure_is_visible_and_app_remains_usable() {
    use std::os::unix::fs::PermissionsExt;
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            let parent = root.join("state/saddle");
            std::fs::create_dir_all(&parent).unwrap();
            std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        },
    );
    h.see("Layout save failed");
    h.see("Synthetic title");
    let state = h.ctl(&["inspect"]);
    let instance = state["instance"].as_str().unwrap();
    let opened = h.ctl(&[
        "open",
        "--instance",
        instance,
        "--relative-to",
        "active",
        "--place",
        "tab",
        "--agent",
        "p/a",
        "--request-id",
        "write-failed-open",
    ]);
    assert_eq!(h.operation(&opened)["state"], "complete");
    assert!(!h.dir.path().join("state/saddle/layout.json").exists());
    std::fs::set_permissions(
        h.dir.path().join("state/saddle"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    h.quit();
    let saved: serde_json::Value =
        serde_json::from_str(&h.log("state/saddle/layout.json")).unwrap();
    assert_eq!(saved["tabs"].as_array().unwrap().len(), 2);
}

#[test]
fn task_links_open_explicit_file_and_return_without_terminal_input() {
    let mut h =
        Harness::start_prepared(
            include_str!("fixtures/drover.py"),
            false,
            "",
            16384,
            |root| {
                std::fs::write(root.join("delivery.md"), "SYNTHETIC DELIVERY\nsecond line")
                    .unwrap();
                std::fs::write(root.join("queue-state.json"), serde_json::json!({
                "mode": {}, "paused": false, "history": [],
                "pending": [{"id":"T1", "title":"Link task", "body":"Artifact: delivery.md"}]
            }).to_string()).unwrap();
            },
        );
    h.see("Synthetic title");
    h.click("Tasks · ");
    h.see("Link task");
    h.click("Links");
    h.see("Files");
    h.see("Task text");
    h.send(b"\r");
    h.see("SYNTHETIC DELIVERY");
    h.send(b"\x1b");
    h.see("Files");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Close Esc"));
    assert!(!h.log("events").contains("input "));
    assert!(!h.log("queue-events").contains("go"));
    h.quit();
}

#[test]
fn task_links_validate_original_instance_before_attach_and_before_existing_navigation() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            std::fs::write(
                root.join("metadata.json"),
                r#"{"p/a":{"instance":"012345abcdef"}}"#,
            )
            .unwrap();
            std::fs::write(root.join("queue-state.json"), serde_json::json!({
            "mode": {}, "paused": false, "history": [],
            "pending": [{"id":"T1", "title":"Agent link task", "body":"Agent: p/a | instance=012345abcdef"}]
        }).to_string()).unwrap();
        },
    );
    h.see("Synthetic title");
    h.click("Tasks · ");
    h.click("Links");
    h.see("› p/a");
    h.send(b"\r");
    h.see("p/a READY");
    h.see("Input ▸ p/a");
    h.click("Tasks · ");
    h.see("› p/a");
    h.send(b"\r");
    h.see("Input ▸ p/a");
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|line| *line == "attach p/a")
            .count(),
        1
    );
    std::fs::write(
        h.dir.path().join("metadata.json"),
        r#"{"p/a":{"instance":"fedcba543210"}}"#,
    )
    .unwrap();
    h.click("Tasks · ");
    h.see("› p/a");
    h.send(b"\r");
    h.see("identity changed");
    assert!(h.contents().contains("Close Esc"));
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|line| *line == "attach p/a")
            .count(),
        1
    );
    assert!(!h.log("events").contains("input p/a"));
    h.quit();
}

#[test]
fn task_links_switching_task_during_status_never_opens_the_old_agent() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            std::fs::write(
                root.join("metadata.json"),
                r#"{"p/a":{"instance":"012345abcdef"}}"#,
            )
            .unwrap();
            std::fs::write(root.join("queue-state.json"), serde_json::json!({
            "mode": {}, "paused": false, "history": [],
            "pending": [{"id":"T1", "title":"Link source", "body":"Agent: p/a | instance=012345abcdef"},
                        {"id":"T2", "title":"Other task", "body":"Agent: p/taken | instance=012345abcdef"}]
        }).to_string()).unwrap();
        },
    );
    h.see("Synthetic title");
    h.click("Tasks · ");
    h.click("Links");
    h.see("› p/a");
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\r");
    h.see("Checking agent");
    h.click("Other task");
    h.see("› p/taken");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.send(b"\r");
    h.see("identity changed");
    assert!(h.contents().contains("Close Esc"));
    assert!(!h.log("events").contains("attach "));
    assert!(!h.log("events").contains("start "));
    h.quit();
}

#[test]
fn task_links_attach_failure_stays_in_tasks_and_unknown_identity_is_disabled() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            std::fs::write(
                root.join("metadata.json"),
                r#"{"p/a":{"instance":"012345abcdef"}}"#,
            )
            .unwrap();
            std::fs::write(root.join("fail-attach"), "").unwrap();
            std::fs::write(root.join("queue-state.json"), serde_json::json!({
            "mode": {}, "paused": false, "history": [],
            "pending": [{"id":"T1", "title":"Broken connection", "body":"Agent: p/a | instance=012345abcdef\nAgent: p/b"}]
        }).to_string()).unwrap();
        },
    );
    h.see("Synthetic title");
    h.click("Tasks · ");
    h.click("Links");
    h.see("› p/a");
    h.send(b"\r");
    h.see("Agent unavailable");
    h.send(b"\x1b[B\r");
    h.see("› p/b");
    h.see("Identity unknown");
    assert!(h.contents().contains("Close Esc"));
    assert!(!h.log("events").contains("attach p/b"));
    h.quit();
}

#[test]
fn task_links_shell_exit_during_replacement_confirmation_releases_the_request() {
    let mut h = Harness::start_prepared(
        include_str!("fixtures/drover.py"),
        false,
        "",
        16384,
        |root| {
            std::fs::write(
                root.join("metadata.json"),
                r#"{"p/a":{"instance":"012345abcdef"}}"#,
            )
            .unwrap();
            std::fs::write(root.join("recovery.txt"), "RECOVERY FILE CONTENT").unwrap();
            std::fs::write(root.join("queue-state.json"), serde_json::json!({
                "mode": {}, "paused": false, "history": [],
                "pending": [{"id":"T1", "title":"Shell link recovery", "body":"Artifact: recovery.txt\nAgent: p/a | instance=012345abcdef"}]
            }).to_string()).unwrap();
        },
    );
    h.see("Synthetic title");
    common::script(
        h.dir.path(),
        "shell",
        r#"#!/usr/bin/env python3
from pathlib import Path
import os, time, sys
root = Path(__file__).parent
os.write(1, b'SHELL READY\r\n')
while not (root / 'exit-shell').exists():
    time.sleep(0.01)
sys.exit(7)
"#,
    );
    h.click("│ + │");
    h.click_in("Open content in a new tab", "Terminal");
    h.see("SHELL READY");
    h.click("Tasks · ");
    h.click("Links");
    h.see("› recovery.txt");
    h.send(b"\x1b[B\r");
    h.see("End these running terminals");
    std::fs::write(h.dir.path().join("exit-shell"), "").unwrap();
    // Wait for the synthetic shell's actual exit, not a timing-dependent delay.
    h.until(|h| h.ctl(&["inspect"])["tabs"][1]["panes"][0]["state"] == "exited");
    let exited = h.ctl(&["inspect"]);
    h.send(b"y");
    // The popup can disappear partway through a PTY frame; wait for the new Tasks message.
    h.see("Close target changed; retry the link.");
    assert!(!h.contents().contains("End these running terminals"));
    assert!(
        !h.contents().contains("Checking agent"),
        "a rejected replacement must finish its Links request"
    );
    h.see("› p/a");
    assert_eq!(h.ctl(&["inspect"])["tabs"], exited["tabs"]);
    assert!(!h.log("events").contains("attach p/a"));
    // Both actions work without switching away from Links or losing its selection.
    h.send(b"\x1b[A\r");
    h.see("RECOVERY FILE CONTENT");
    h.send(b"\x1b");
    h.see("› recovery.txt");
    h.send(b"\x1b[B\r");
    h.see("p/a READY");
    h.see("Input ▸ p/a");
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|line| *line == "attach p/a")
            .count(),
        1
    );
    assert!(!h.log("events").contains("stop "));
    h.quit();
}

/// Two registered projects; each awaits the task in its `awaiting.json`, if any. The user's
/// notification preference lives in `notify.json` beside the script, as Drover's public CLI
/// reports and changes it.
const NOTIFY_QUEUE: &str = r#"#!/usr/bin/env python3
import json, sys
from pathlib import Path
root = Path(__file__).parent
args = sys.argv[1:]
with (root / 'queue-events').open('a') as f:
    f.write(json.dumps([Path.cwd().name] + args) + '\n')
prefs = root / 'notify.json'
if len(args) == 3 and args[0] == 'notifications' and args[1] in ('status', 'on', 'off') and args[2] == '--json':
    p = json.loads(prefs.read_text())
    if args[1] != 'status' and p['system_enabled'] != (args[1] == 'on'):
        p = dict(system_enabled=args[1] == 'on', revision=p['revision'] + 1)
        prefs.write_text(json.dumps(p))
    print(json.dumps(dict(schema_version=1, ok=True, scope='user', application='next_notification_check', **p)))
    sys.exit(0)
if args == ['list', '--json']:
    task = Path.cwd() / 'awaiting.json'
    awaiting = json.loads(task.read_text()) if task.exists() else None
    print(json.dumps(dict(mode=dict(loop=False, gate=True), paused=False, current=None,
                          awaiting=awaiting, pending=[], history=[])))
    sys.exit(0)
print('FORBIDDEN CLI: ' + repr(args), file=sys.stderr)
sys.exit(99)
"#;

impl Harness {
    /// Makes `project` await a new run of `id`, started at `t0`.
    fn awaiting(&self, project: &str, id: &str, t0: f64) {
        std::fs::write(
            self.dir.path().join(project).join("awaiting.json"),
            serde_json::json!({
                "id": id, "title": format!("Ship {id}"), "body": "", "key": "", "status": "done",
                "location": "awaiting", "start": format!("s-{id}-{t0}"), "main": "m", "t0": t0,
                "end": "e", "t1": t0 + 1.0,
            })
            .to_string(),
        )
        .unwrap();
    }
    /// Waits for two more cross-project rounds, so every answer so far has been used.
    fn rounds(&mut self) {
        let reads = |h: &Self| {
            h.log("queue-events")
                .matches(r#"["project-two", "list", "--json"]"#)
                .count()
        };
        let start = reads(self);
        self.until(|h| reads(h) >= start + 2);
    }
    /// The screen position of `text` on the row showing `row_text`.
    fn on_row(&self, row_text: &str, text: &str) -> (u16, u16) {
        let (_, row) = self.locate(row_text, 0).unwrap();
        let screen = self.screen.screen();
        let col = (0..screen.size().1)
            .find(|&c| screen.cell(row, c).unwrap().contents() == text)
            .unwrap();
        (col, row)
    }
}

#[test]
fn in_saddle_prompts_new_awaiting_tasks_without_taking_input_and_system_stays_quiet() {
    let mut h = Harness::start_prepared(NOTIFY_QUEUE, true, "", 16384, |dir| {
        std::fs::write(
            dir.join("notify.json"),
            r#"{"system_enabled": false, "revision": 1}"#,
        )
        .unwrap();
    });
    // project-one already awaits T3 at start: that is the baseline, never prompted.
    h.awaiting("project-one", "T3", 100.0);
    h.see("Synthetic title");
    h.send(b"skk");
    h.see("┃ ○ a ");
    h.send(b"\r");
    h.see("p/a READY");
    h.see("Input ▸ p/a");
    h.rounds();
    h.see("Attention · 1");
    assert!(
        !h.contents().contains("ready for review"),
        "{}",
        h.contents()
    );

    // A new awaiting run prompts at the bottom right; typing still reaches the agent.
    h.awaiting("project-two", "T5", 200.0);
    h.see("project-two · T5 ready for review");
    let (col, row) = h.locate("project-two · T5 ready for review", 0).unwrap();
    assert!(row > 30 && col > 70, "at {col},{row}:\n{}", h.contents());
    h.send(b"Q");
    h.event("input p/a 51");
    assert!(h.contents().contains("Input ▸ p/a"));
    // Mouse actions on the prompt stay with it; the close mark only closes it.
    let (x, y) = (col + 3, row + 1);
    h.send(format!("\x1b[<64;{x};{y}M\x1b[<35;{x};{y}M").as_bytes());
    let (cx, cy) = h.on_row("project-two · T5 ready for review", "×");
    h.send(
        format!(
            "\x1b[<0;{};{}M\x1b[<0;{};{}m",
            cx + 1,
            cy + 1,
            cx + 1,
            cy + 1
        )
        .as_bytes(),
    );
    h.until(|h| !h.contents().contains("ready for review"));
    h.send(b"W");
    h.event("input p/a 57");
    assert!(
        !h.log("events").contains("1b5b3c"),
        "mouse reached the agent: {}",
        h.log("events")
    );
    // Refreshes do not prompt the same run again; releasing is not done by closing.
    h.rounds();
    assert!(!h.contents().contains("ready for review"));
    h.see("Attention · 2");

    // Clicking a prompt opens its task in its project's Tasks.
    h.awaiting("project-one", "T6", 300.0);
    h.see("project-one · T6 ready for review");
    h.click("project-one · T6 ready for review");
    h.see("Input ▸ Tasks");
    h.see("Ship T6");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");

    // Several at once are one prompt that opens Attention.
    h.awaiting("project-one", "T7", 400.0);
    h.awaiting("project-two", "T8", 500.0);
    h.see("2 tasks ready for review");
    h.click("2 tasks ready for review");
    h.see("Input ▸ Attention");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");

    // Settings shows Drover's choice and saves a new one only through Drover.
    h.send(b",");
    h.see("Drover tasks across projects");
    h.send(b"\x1b[B\x1b[B\x1b[B ");
    h.see("•Task notifications");
    h.send(b"\x13");
    h.see("Task notifications: System");
    assert!(
        h.log("queue-events")
            .contains(r#""notifications", "on", "--json"]"#),
        "{}",
        h.log("queue-events")
    );
    // System: Drover notifies; saddle does not prompt.
    h.rounds();
    h.awaiting("project-two", "T9", 600.0);
    h.rounds();
    h.rounds();
    assert!(
        !h.contents().contains("ready for review"),
        "{}",
        h.contents()
    );
    h.quit();
    let queue = h.log("queue-events");
    assert!(
        queue.lines().all(|l| l.contains(r#""list", "--json"]"#)
            || l.contains(r#""notifications", "status", "--json"]"#)
            || l.contains(r#""notifications", "on", "--json"]"#)),
        "{queue}"
    );
    assert!(!h.log("events").contains("stop "), "{}", h.log("events"));
}

#[test]
#[ignore = "requires the delivered drover CLI in SADDLE_DROVER_NOTIFY_BIN; isolated HOME/XDG and synthetic projects only"]
fn real_drover_notification_channel_prompts_dedups_switches_and_opens_tasks() {
    let drover = std::env::var("SADDLE_DROVER_NOTIFY_BIN").expect("set SADDLE_DROVER_NOTIFY_BIN");
    // Logs like NOTIFY_QUEUE, then runs the real CLI with every XDG and runtime path inside
    // the sandbox and a corral that refuses everything.
    let wrapper = format!(
        r#"#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
root = Path(__file__).parent
with (root / 'queue-events').open('a') as f:
    f.write(json.dumps([Path.cwd().name] + sys.argv[1:]) + '\n')
env = dict(os.environ)
for name in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME', 'XDG_STATE_HOME', 'XDG_RUNTIME_DIR', 'TMPDIR'):
    env[name] = str(root / 'xdg' / name)
    os.makedirs(env[name], exist_ok=True)
env['DROVER_CORRAL_BIN'] = str(root / 'no-corral')
os.execve({drover:?}, [{drover:?}] + sys.argv[1:], env)
"#
    );
    let mut h = Harness::start_prepared(&wrapper, true, "", 16384, |dir| {
        common::script(
            dir,
            "no-corral",
            "#!/bin/sh\ntouch \"$(dirname \"$0\")/no-corral-called\"\nexit 99\n",
        );
        for project in ["project-one", "project-two"] {
            let repo = dir.join(project);
            let data = dir.join(format!("{project}-data"));
            std::fs::create_dir_all(&data).unwrap();
            std::fs::write(data.join("tasks.state"), "").unwrap();
            assert!(
                std::process::Command::new("git")
                    .args(["init", "-q", "-b", "main"])
                    .current_dir(&repo)
                    .status()
                    .unwrap()
                    .success()
            );
            std::fs::write(
                repo.join(".drover.conf"),
                format!(
                    "HANDOFF_DIR={}\nTASK_GATE=1\nMAIN_AGENT=fake/main\n",
                    data.display()
                ),
            )
            .unwrap();
        }
    });
    let status = |h: &Harness| {
        let output = std::process::Command::new(h.dir.path().join("queue"))
            .args(["notifications", "status", "--json"])
            .env("HOME", h.dir.path().join("home"))
            .current_dir(h.dir.path())
            .output()
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        (value["system_enabled"].clone(), value["revision"].clone())
    };
    // A synthetic run of T1 that is done and awaits release.
    let finish = |h: &Harness, project: &str, t: f64| {
        let events = [
            serde_json::json!({"ev": "start", "id": "T1", "title": format!("Ship {project}"), "body": "b",
                "sha": "a".repeat(40), "main": "b".repeat(40), "t": t}),
            serde_json::json!({"ev": "done", "id": "T1", "sha": "c".repeat(40), "t": t + 60.0, "gate": true}),
        ];
        let text: String = events.iter().map(|e| format!("{e}\n")).collect();
        std::fs::write(
            h.dir.path().join(format!("{project}-data/tasks.state")),
            text,
        )
        .unwrap();
    };
    h.see("Synthetic title");
    h.send(b"skk");
    h.see("┃ ○ a ");
    assert_eq!(status(&h), (true.into(), 0.into()));
    // Settings reads System (Drover's default) and switches to In saddle through Drover.
    h.send(b",");
    h.see("Drover tasks across projects");
    h.send(b"\x1b[B\x1b[B\x1b[B \x13");
    h.see("Task notifications: In saddle");
    assert_eq!(status(&h), (false.into(), 1.into()));
    h.rounds();
    h.rounds();
    // A new awaiting run prompts once and opens its task.
    finish(&h, "project-one", 1790000000.25);
    h.see("project-one · T1 ready for review");
    h.click("project-one · T1 ready for review");
    h.see("Input ▸ Tasks");
    h.see("Ship project-one");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.rounds();
    h.rounds();
    assert!(
        !h.contents().contains("ready for review"),
        "{}",
        h.contents()
    );
    // Back to System: Drover notifies, saddle stays quiet; Attention keeps both tasks.
    h.send(b",");
    h.see("Drover tasks across projects");
    h.send(b"\x1b[B\x1b[B\x1b[B \x13");
    h.see("Task notifications: System");
    assert_eq!(status(&h), (true.into(), 2.into()));
    h.rounds();
    finish(&h, "project-two", 1790000100.5);
    h.see("Attention · 2");
    h.rounds();
    h.rounds();
    assert!(
        !h.contents().contains("ready for review"),
        "{}",
        h.contents()
    );
    h.quit();
    let queue = h.log("queue-events");
    assert!(
        queue.lines().all(|l| l.contains(r#""list", "--json"]"#)
            || l.contains(r#""notifications", "status", "--json"]"#)
            || l.contains(r#""notifications", "on", "--json"]"#)
            || l.contains(r#""notifications", "off", "--json"]"#)),
        "{queue}"
    );
    assert!(!h.dir.path().join("no-corral-called").exists());
}
