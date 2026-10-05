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
    attributes_answered: bool,
    raw: Vec<u8>,
}
impl Harness {
    fn start() -> Self {
        let mut h = Self::start_prepared("", 16384, |_| {});
        h.see("Synthetic title");
        h.send(b"skk");
        h.see("┃ ○ a ");
        h.see("s Name");
        h
    }
    fn start_with_projects() -> Self {
        let mut h = Self::start_prepared("", 16384, |dir| {
            for name in ["project-one", "project-two"] {
                std::fs::create_dir(dir.join(name)).unwrap();
            }
            std::fs::write(
                dir.join("metadata.json"),
                serde_json::json!({
                    "p/a": {"cwd": dir.join("project-one").canonicalize().unwrap()},
                    "p/b": {"cwd": dir.join("project-two").canonicalize().unwrap()}
                })
                .to_string(),
            )
            .unwrap();
        });
        h.see("Synthetic title");
        h.send(b"skk");
        h.see("┃ ○ a ");
        h.see("s Name");
        h
    }
    fn start_with_config(extra: &str) -> Self {
        Self::start_with_read_chunk(extra, 16384)
    }
    fn start_with_read_chunk(extra: &str, read_chunk: usize) -> Self {
        let mut h = Self::start_prepared(extra, read_chunk, |_| {});
        h.see("Synthetic title");
        h.send(b"skk");
        h.see("┃ ○ a ");
        h.see("s Name");
        h
    }
    fn start_prepared(
        extra: &str,
        read_chunk: usize,
        prepare: impl FnOnce(&std::path::Path),
    ) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let corral = common::script(dir.path(), "corral", include_str!("fixtures/corral.py"));
        std::fs::write(
            dir.path().join("agents.json"),
            r#"{"p/a":"idle","p/b":"working","p/taken":"idle"}"#,
        )
        .unwrap();
        let config = dir.path().join("config.toml");
        std::fs::write(
            &config,
            // Text-based workflow assertions need unoccluded output. Overlay tests
            // explicitly enable the decoration and check its permitted occlusion.
            format!("corral = {corral:?}\nrefresh_ms = 100\nmascot_enabled = false\n{extra}"),
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
        cmd.env(
            "PATH",
            format!(
                "{}:{}",
                dir.path().display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
        cmd.env("XDG_STATE_HOME", dir.path().join("state"));
        for name in [
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_RUNTIME_DIR",
            "TMPDIR",
        ] {
            let path = dir.path().join("isolated").join(name);
            std::fs::create_dir_all(&path).unwrap();
            cmd.env(name, path);
        }
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
        let mut harness = Self {
            dir,
            child,
            master: pair.master,
            writer,
            output,
            screen: vt100::Parser::new(40, 140, 0),
            cursor_answered: false,
            attributes_answered: false,
            raw: Vec::new(),
        };
        harness.see("Agents ·");
        harness
    }
    fn pump(&mut self) {
        if let Ok(bytes) = self.output.recv_timeout(Duration::from_millis(30)) {
            self.screen.process(&bytes);
            self.raw.extend(bytes);
            if !self.cursor_answered && self.raw.windows(4).any(|w| w == b"\x1b[6n") {
                self.send(b"\x1b[1;1R");
                self.cursor_answered = true;
            }
            // Like a terminal without pictures: device attributes, no graphics reply.
            if !self.attributes_answered && self.raw.windows(3).any(|w| w == b"\x1b[c") {
                self.send(b"\x1b[?62;22c");
                self.attributes_answered = true;
            }
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).unwrap();
    }
    fn log(&self, filename: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(filename)).unwrap_or_default()
    }
    fn input_hex(&self, name: &str) -> String {
        let prefix = format!("input {name} ");
        self.log("events")
            .lines()
            .filter_map(|line| line.strip_prefix(&prefix))
            .collect()
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
    fn settle(&mut self) {
        // A visible label may arrive before the rest of a PTY frame.
        let end = Instant::now() + Duration::from_millis(120);
        while Instant::now() < end {
            self.pump();
        }
    }
    fn header_tool(&mut self, label: &str) {
        self.click(label);
    }
    fn click(&mut self, label: &str) {
        self.settle();
        let (col, row) = self.press_button(label);
        self.send(format!("\x1b[<0;{};{}m", col + 1, row + 1).as_bytes());
    }
    fn press_button(&mut self, label: &str) -> (u16, u16) {
        self.see(label);
        let (col, row) = self
            .locate_from(label, 0, 0)
            .unwrap_or_else(|| panic!("click target not found: {label}"));
        self.send(format!("\x1b[<0;{};{}M", col + 1, row + 1).as_bytes());
        (col, row)
    }
    fn locate(&self, label: &str, first_row: u16) -> Option<(u16, u16)> {
        self.locate_from(label, first_row, 0)
    }
    fn locate_from(&self, label: &str, first_row: u16, first_col: u16) -> Option<(u16, u16)> {
        let screen = self.screen.screen();
        let (rows, cols) = screen.size();
        for row in first_row..rows {
            for col in first_col..cols {
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
        self.see(title);
        self.settle();
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
fn shift_enter_survives_outer_terminal_negotiation_and_viewer_pty() {
    use alacritty_terminal::{Term, event::VoidListener, term::TermMode, vte::ansi::Processor};

    let mut h = Harness::start();
    h.send(b"\r");
    h.see("p/a READY");

    // Model the outer terminal, not a preconstructed KeyEvent. In the Kitty protocol's
    // legacy C0 table Shift+Enter is CR; disambiguation preserves its Shift modifier.
    // https://sw.kovidgoyal.net/kitty/keyboard-protocol/
    let mut outer = Term::new(
        alacritty_terminal::term::Config {
            kitty_keyboard: true,
            ..Default::default()
        },
        &saddle::terminal::Size {
            rows: 40,
            cols: 140,
        },
        VoidListener,
    );
    let mut parser = Processor::<alacritty_terminal::vte::ansi::StdSyncHandler>::new();
    // Give the main and alternate screens distinct pre-existing mode stacks.
    parser.advance(&mut outer, b"\x1b[>2u\x1b[?1049h\x1b[>4u\x1b[?1049l");
    parser.advance(&mut outer, &h.raw);
    let output_offset = h.raw.len();
    let shift_enter: &[u8] = if outer.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES) {
        b"\x1b[13;2u"
    } else {
        b"\r"
    };
    h.send(shift_enter);
    h.event("input p/a ");
    assert_eq!(
        h.input_hex("p/a"),
        "1b5b31333b3275",
        "Shift+Enter must reach the prompt distinctly from submit (CR = 0d)"
    );
    h.quit();
    parser.advance(&mut outer, &h.raw[output_offset..]);
    assert_eq!(
        *outer.mode() & TermMode::KITTY_KEYBOARD_PROTOCOL,
        TermMode::REPORT_EVENT_TYPES,
        "exiting must preserve the main screen's keyboard mode"
    );
    parser.advance(&mut outer, b"\x1b[?1049h");
    assert_eq!(
        *outer.mode() & TermMode::KITTY_KEYBOARD_PROTOCOL,
        TermMode::REPORT_ALTERNATE_KEYS,
        "exiting must pop Saddle's keyboard mode on the alternate screen"
    );
}

#[test]
fn viewer_preserves_modified_enter_bytes_and_legacy_input() {
    let mut h = Harness::start();
    h.send(b"\r");
    h.see("p/a READY");
    let mut expected = String::new();
    for (input, hex) in [
        (b"\x1b[13;2u".as_slice(), "1b5b31333b3275"), // Shift+Enter
        (b"\r", "0d"),                                // submit
        (b"\x1b[13;5u", "1b5b31333b3575"),            // Ctrl+Enter
        (b"\x1b\r", "1b0d"),                          // Alt+Enter
        (b"\x1b[13;3u", "1b0d"),                      // enhanced Alt+Enter
        (b"\n", "0a"),                                // legacy Ctrl-J / newline binding
        (b"\x1b[99;5u", "03"),                        // enhanced Ctrl-C
        (b"\t\x1b[Z", "091b5b5a"),                    // Tab / Shift-Tab stay in Viewer
        (b"\x1b[9;2u", "1b5b5a"),                     // enhanced Shift-Tab
        (b"\x1b[200~a\nb\x1b[201~", "1b5b3230307e610a621b5b3230317e"),
    ] {
        h.send(input);
        expected.push_str(hex);
        h.until(|h| h.input_hex("p/a").len() >= expected.len());
        assert_eq!(h.input_hex("p/a"), expected);
    }
    h.send(b"\x1b[93;5u"); // Enhanced Ctrl-] returns focus without reaching the PTY.
    h.see("Input ▸ Agents");
    assert_eq!(h.input_hex("p/a"), expected);
    h.quit();
}

#[test]
fn full_workflow_routes_input_switches_safely_and_survives_disappearance() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    assert!(!h.log("events").contains("reply "));
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"q");
    h.event("input p/a 71");
    h.send("\x1b[200~中文\nhello\x1b[201~".as_bytes());
    h.event("1b5b3230307ee4b8ade696870a68656c6c6f1b5b3230317e");
    let (col, row) = h.locate("p/a READY", 0).unwrap();
    h.send(format!("\x1b[<0;{};{}M", col + 3, row + 2).as_bytes());
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
    h.event("size p/b 106x38"); // Decoration does not consume terminal rows.
    // Agent disappearance returns to the prompt, without selecting another viewer.
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"blocked","p/new":"idle","p/taken":"idle"}"#,
    )
    .unwrap();
    h.event("detached p/b");
    h.send(b"\x1d"); // Return focus to Agents after disappearance.
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
    h.see("Synthetic title");
    // Locate the agent headline so host navigation rows do not change this gesture.
    h.press_button("┃ ○ a ");
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
fn native_mouse_buttons_cover_forms_and_stop_confirmation() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.click("x Stop");
    h.click("Cancel Esc");
    h.see("cancelled");
    assert!(!h.log("events").contains("stop "));
    h.send(b",");
    h.see("Sidebar width");
    h.click("52");
    h.send(b"\x1552\x13");
    h.see("Input ▸ Agents");
    h.master
        .resize(PtySize {
            rows: 48,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(48, 80);
    // Wait for the resized Agents column to end at column 34.
    h.until(|h| h.screen.screen().cell(0, 33).unwrap().contents() == "┓");
    h.send(b"\x1d");
    h.see("Input ▸ Agents");
    // Narrow windows keep Agents on the left; hit targets must follow the new rows.
    h.click("x Stop");
    h.click("‹Stop y›");
    h.event("stop p/a");
    h.quit();
}

#[test]
fn buttons_require_release_on_the_same_target() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("p/a READY");
    h.press_button("Settings");
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(
        !h.contents().contains("Input ▸ Settings"),
        "Down must not open Settings"
    );
    h.send(b"\x1b[<32;130;4M\x1b[<0;130;4m"); // Drag/release over Viewer cancels, without sending a stray release.
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        h.pump();
    }
    assert!(!h.contents().contains("Input ▸ Settings"));
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
    h.send(b",");
    h.see("Sidebar width");
    h.send(b"\x1b[<0;130;4M\x1b[<0;130;4m");
    h.master
        .resize(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    h.screen.screen_mut().set_size(24, 80);
    // Settings occupies the resized screen while keeping background input captured.
    h.until(|h| h.screen.screen().cell(0, 79).unwrap().contents() == "┓");
    // The first border can arrive before the resized frame and input loop finish.
    h.until(|h| h.locate("Input ▸ Settings", 23).is_some());
    h.see("Sidebar width");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.see("Synthetic title");
    h.until(|h| h.screen.screen().cell(0, 33).unwrap().contents() == "┓");
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
    std::fs::write(h.dir.path().join("hold-status"), "").unwrap();
    h.send(b"\r");
    h.send(b",");
    h.see("Sidebar width");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.event("attach p/a");
    h.send(b"\x1570\x13");
    h.until(|h| h.log("config.toml").contains("left_width = 70"));
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
    h.see("worker-00");
    h.see("z Fold");
    assert!(!h.contents().contains("worker-07"));
    let (_, row) = h.locate("worker-00", 0).unwrap();
    // Column 50 is the scrollbar, outside the text list but inside Agents.
    // Use an agent row: terminal row 4 is the header separator, not the list.
    let down = format!("\x1b[<65;51;{}M", row + 1).repeat(60);
    let up = format!("\x1b[<64;51;{}M", row + 1).repeat(60);
    // At this width the controls occupy one row, separated from the list by a rule.
    let scrollbar_bottom = h.locate("z Fold", 0).unwrap().1 - 2;
    // Each expanded fixture has five rows; wait until the last one's final row
    // reaches the bottom, not merely until its headline first enters the viewport.
    let at_bottom = |h: &Harness| {
        h.locate("worker-07", 0)
            .is_some_and(|(_, y)| y + 4 == scrollbar_bottom)
            && h.screen
                .screen()
                .cell(scrollbar_bottom, 50)
                .unwrap()
                .contents()
                == "█"
    };
    h.send(down.as_bytes());
    h.until(at_bottom);
    assert!(!h.contents().contains("worker-00"));
    let last_row = h.locate("worker-07", 0).unwrap();
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
    assert_eq!(h.locate("worker-07", 0), Some(last_row));
    assert!(!h.contents().contains("worker-00"));
    h.send(up.as_bytes());
    h.see("worker-00");
    h.until(|h| h.locate("worker-00", 0).is_some_and(|(_, y)| y == row));
    assert!(!h.contents().contains("worker-07"));
    h.send(format!("\x1b[<65;12;{}M", row + 1).repeat(60).as_bytes());
    h.until(at_bottom);
    assert!(!h.log("events").contains("attach "));
    h.quit();
}

#[test]
fn startup_colors_reach_agents_and_viewer_defaults() {
    use vt100::Color;
    let mut h = Harness::start_with_config(
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
    // Agents has its own palette.
    assert_eq!(
        label_cell(&h, "idle").fgcolor(),
        Color::Rgb(0x55, 0x66, 0x77)
    );
    assert_eq!(
        label_cell(&h, "claude").fgcolor(),
        Color::Rgb(0x77, 0x88, 0x99)
    );
    h.send(b"\r");
    h.see("p/a READY");
    let viewer = label_cell(&h, "p/a READY");
    assert_eq!(viewer.fgcolor(), Color::Rgb(0xab, 0xcd, 0xef));
    assert_eq!(viewer.bgcolor(), Color::Rgb(0x10, 0x20, 0x30));
    h.send(b"C");
    h.see("AGENT COLORS");
    let colored = label_cell(&h, "AGENT COLORS");
    assert_eq!(colored.fgcolor(), Color::Idx(1));
    assert_eq!(colored.bgcolor(), Color::Rgb(9, 8, 7));
    h.quit();
}

#[test]
fn placement_cancel_and_escape_never_attach_and_new_cancel_keeps_the_draft() {
    let mut h = Harness::start_with_projects();
    h.see("Synthetic title");
    h.send(b"n");
    h.click("Regular");
    h.click("Name");
    h.send(b"\x15review-draft");
    h.see("review-draft");
    h.click("▾ Ctrl-P");
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
    let mut h = Harness::start_with_projects();
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
fn new_agent_choices_create_with_defaults_in_the_selected_project() {
    let mut h = Harness::start_with_projects();
    h.send(b"n");
    h.see("main");
    h.click("▾ Ctrl-P");
    h.click("project-one  ");
    h.click("Create agent");
    h.see("main-actual READY");
    h.see("Controller · agents/main-actual");
    h.send(b"\x1dn");
    h.click("▾ Ctrl-P");
    h.click("project-two  ");
    h.click("Claude");
    h.see("main");
    h.click("Create agent");
    h.until(|h| h.log("start-args").lines().count() == 2);
    h.until(|h| !h.contents().contains("Create agent"));
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
    let mut h = Harness::start_with_projects();
    h.send(b"n");
    h.see("New agent");
    h.click("▾ Ctrl-P");
    h.click("project-one  ");
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
    h.send(b"\x1d,");
    h.see("Sidebar width");
    std::fs::remove_file(h.dir.path().join("hold-status")).unwrap();
    h.event("attach p/b");
    h.send(b"\x1570\x13");
    h.until(|h| h.log("config.toml").contains("left_width = 70"));
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
    h.click("Current pane (←/→)");
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
    h.click("Agent · p/a");
    // A visible native page acknowledges that all preceding input was handled.
    h.send(b"\x1d,");
    h.see("Settings");
    h.send(b"\x1b");
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
    h.press_button("┃ ○ a "); // Mouse: the first agent row opens p/a in the current pane.
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
    h.see("Left  ←");
    h.send(b"\x1d"); // Ctrl-] closes the menu and returns to Agents.
    h.see("Input ▸ Agents");
    assert!(!h.contents().contains("Left  ←"));
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
    h.press_button("┃ ○ a ");
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
    // A tab title arrives before the PTY's remaining redraw bytes.
    h.see("Input ▸ p/a");
    h.see("INPUT RECEIVED");
    // The moved pane keeps its session and output; p/b stays behind in Tab 1.
    assert!(h.contents().contains("INPUT RECEIVED"), "{}", h.contents());
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
    h.see("Input ▸ p/a");
    h.see("INPUT RECEIVED");
    let a = h.locate("Agent · p/a", 0).unwrap();
    let b = h.locate("Agent · p/b", 0).unwrap();
    assert!(a.0 == b.0 && a.1 > b.1, "p/a must move below p/b");
    assert!(h.contents().contains("INPUT RECEIVED"), "{}", h.contents());
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
    h.press_button("┃ ○ a ");
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
    let mut h = Harness::start_with_projects();
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
    let mut h = Harness::start_with_projects();
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
    let mut h = Harness::start_with_projects();
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
    let mut h = Harness::start_with_projects();
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
    let mut h = Harness::start_with_projects();
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
fn t51_location_new_can_choose_the_originating_pane_with_existing_confirmation() {
    let mut h = Harness::start_with_projects();
    h.see("Synthetic title");
    h.click("│ + │");
    h.click_in("Open content in a new tab", "Terminal");
    h.see("SHELL READY");
    let initial = h.ctl(&["inspect"]);
    h.click("│ + │");
    h.click_in("Open content in a new tab", "New agent…");
    h.see("Create agent");
    h.click("Advanced");
    h.see("New tab (←/→)");
    h.click("New tab (←/→)"); // Split left
    h.send(b"\x1b[D\x1b[D");
    h.see("Current pane (←/→)");
    h.send(b"\x13");
    h.see("End these running terminals");
    h.click("End shells y");
    h.event("start agents/main");
    h.see("agents/main-actual READY");
    let after = h.ctl(&["inspect"]);
    h.quit();
    assert_eq!(after["tabs"].as_array().unwrap().len(), 2, "{after}");
    assert_eq!(after["active_pane"], initial["active_pane"]);
    assert_eq!(after["tabs"][1]["panes"].as_array().unwrap().len(), 1);
    assert_eq!(
        after["tabs"][1]["panes"][0]["id"],
        initial["tabs"][1]["panes"][0]["id"]
    );
    assert_eq!(after["tabs"][1]["panes"][0]["agent"], "agents/main-actual");
    assert!(!h.log("events").contains("stop "));
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
    let form_uses_source = h.popup("New agent").contains("agent-project ▾ Ctrl-P");
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
        "pending pane inspection reports an unrelated cwd"
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
    h.send(b"\x1dn\x13"); // Current-pane New uses the selected project.
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
    let form_uses_original = h.popup("New agent").contains("original-project ▾ Ctrl-P");
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
    h.see("Search ━");
    h.see("Input ▸ Search");
    h.send(b"zebra");
    h.settle(); // filtering resizes the popup; wait for its complete frame
    h.until(|h| {
        let popup = h.popup("Search ━");
        popup.contains("p/b") && !popup.contains("p/a") && !popup.contains("p/taken")
    });
    h.send(b"\r");
    h.see("p/b READY");
    h.until(|h| !h.contents().contains("Search ━"));
    h.send(b"B");
    h.event("input p/b 42");
    // Esc cancels and keeps the display target; nothing is attached.
    h.send(b"\x1d/a");
    h.see("Search ━");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("Search ━"));
    h.see("Input ▸ Agents");
    // The bar entry opens it too; clicking a candidate opens that agent by its name.
    h.click("/ Search");
    h.see("Search ━");
    h.send(b"p/a");
    h.settle(); // filtering resizes the popup; wait for its complete frame
    h.until(|h| {
        let popup = h.popup("Search ━");
        popup.contains("p/a") && !popup.contains("p/b")
    });
    h.click_in("Search ━", "p/a  demo");
    h.see("p/a READY");
    h.until(|h| !h.contents().contains("Search ━"));
    // Put p/b beside p/a and focus p/a; searching p/b jumps to its open pane.
    h.click("Split ▾");
    h.click("Right →");
    h.click_in("Open content on the right", "p/b");
    h.see("p/b READY");
    h.click("Agent · p/a");
    h.see("Input ▸ p/a");
    h.send(b"\x1d/");
    h.see("Search ━");
    h.send(b"p/");
    h.settle(); // filtering resizes the popup; wait for its complete frame
    h.until(|h| {
        let popup = h.popup("Search ━");
        popup.contains("p/taken") && popup.matches("Open").count() == 2
    });
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("Search ━"));
    let attaches = |h: &Harness| {
        h.log("events")
            .lines()
            .filter(|l| l.starts_with("attach "))
            .count()
    };
    let before = attaches(&h);
    h.send(b"/p/b");
    h.see("Search ━");
    h.settle();
    h.until(|h| h.popup("Search ━").contains("p/b  zebra-project"));
    h.send(b"\r");
    h.until(|h| !h.contents().contains("Search ━"));
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
    h.see("Input ▸ Search");
    h.settle(); // the filtered popup has fewer rows than the initial search
    h.until(|h| {
        let popup = h.popup("Search ━");
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

#[test]
fn t22_attention_gathers_agents_and_opens_the_selected_target() {
    let mut h = Harness::start_prepared("", 16384, |dir| {
        std::fs::write(
            dir.join("agents.json"),
            r#"{"p/a":"blocked","p/b":"blocked"}"#,
        )
        .unwrap();
    });
    h.see("Synthetic title");
    h.see("Attention · 2");
    h.send(b"a");
    h.see("Attention ━");
    h.click_in("Attention ━", "p/b");
    h.see("p/b READY");
    std::fs::write(
        h.dir.path().join("agents.json"),
        r#"{"p/a":"idle","p/b":"blocked"}"#,
    )
    .unwrap();
    h.see("Attention · 1");
    h.quit();
    assert_eq!(
        h.log("events")
            .lines()
            .filter(|l| l.starts_with("attach "))
            .collect::<Vec<_>>(),
        ["attach p/b"]
    );
    assert!(!h.log("events").contains("stop "));
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
    // Wait until the public poll has observed this attachment before simulating disappearance.
    // READY alone can precede that poll, especially without the old task workers at startup.
    h.until(|h| h.contents().matches("abcdef · ATT 1 · VIA human").count() == 2);
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
    let mut h = Harness::start_prepared("", 16384, t25_seed);
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
    let mut h = Harness::start_prepared("", 16384, t25_seed);
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
    let mut h = Harness::start_prepared("", 16384, |root| {
        t25_seed(root);
        std::fs::write(
            root.join("metadata.json"),
            r#"{"p/a":{"instance":"replacement"}}"#,
        )
        .unwrap();
    });
    h.click("p/gone");
    h.see("identity changed");
    assert!(!h.log("events").contains("attach p/a"));
    h.quit();

    let mut h = Harness::start_prepared("", 16384, |root| {
        t25_seed(root);
        std::fs::write(root.join("p-a"), "other attachment").unwrap();
    });
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
        let mut h = Harness::start_prepared("", 16384, |root| {
            t25_seed(root);
            std::fs::write(root.join("state/saddle/layout.json"), original).unwrap();
        });
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
        let mut h = Harness::start_prepared("", 16384, |root| {
            t25_seed(root);
            std::fs::write(root.join("hold-status"), "").unwrap();
        });
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
    let mut h = Harness::start_prepared("", 16384, |root| {
        t25_seed(root);
        std::fs::write(root.join("agents.json"), r#"{"p/a":"exited"}"#).unwrap();
    });
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
    let mut h = Harness::start_prepared("", 16384, |root| {
        let parent = root.join("state/saddle");
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();
    });
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
fn unified_search_opens_each_settings_page_and_keeps_terminal_input_local() {
    let mut h = Harness::start();
    let config = h.log("config.toml");
    for (page, content) in [
        ("General", "Sidebar width"),
        ("Colors", "Interface"),
        ("Advanced", "corral command"),
        ("Diagnostics", "Copy summary c"),
        ("Updates", "Refresh r"),
    ] {
        h.send(format!("/{page}").as_bytes());
        h.see(&format!("Settings › {page}"));
        h.send(b"\r");
        h.see(content);
        h.send(b"\x1b");
        h.see("Input ▸ Agents");
    }
    assert_eq!(
        h.log("config.toml"),
        config,
        "navigation must not save settings"
    );
    h.send(b"/Tasks");
    h.see("No entries match “Tasks”.");
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.send(b"\r");
    h.see("p/a READY");
    h.send(b"/");
    h.event("input p/a 2f");
    h.click("/ Search");
    h.see("Input ▸ Search");
    h.send(b"Colors");
    h.see("Settings › Colors");
    h.send(b"\x1b");
    h.see("Input ▸ p/a");
    h.click("/ Search");
    h.send(b"General\r");
    h.see("Sidebar width");
    h.send(b"\x1570/"); // edit a draft: slash stays in the field
    h.see("70/");
    assert!(!h.contents().contains("Search ━"));
    h.send(b"\x1b");
    h.see("Input ▸ p/a");
    assert_eq!(h.log("config.toml"), config);
    h.quit();
    assert_eq!(h.input_hex("p/a"), "2f", "search and settings input leaked");
}

#[test]
fn host_ignores_legacy_queue_configuration() {
    let mut h = Harness::start_prepared(
        "[queue]\ndrover='/does/not/exist'\ncwd='/unused'\n",
        16384,
        |_| {},
    );
    h.see("Synthetic title");
    h.send(b",");
    h.see("Settings");
    h.send(b"\x1bOS");
    h.see("Diagnostics");
    let end = Instant::now() + Duration::from_millis(600);
    while Instant::now() < end {
        h.pump();
    }
    assert!(
        h.log("queue-events").is_empty(),
        "host called Drover: {}",
        h.log("queue-events")
    );
    assert!(!h.screen.screen().contents().contains("drover command"));
    h.send(b"\x1b");
    h.see("Input ▸ Agents");
    h.quit();
}

#[test]
fn clawd_patrol_keeps_layout_and_does_not_send_mouse_input() {
    let mut h = Harness::start_prepared("", 16384, |root| {
        let path = root.join("config.toml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            path,
            text.replace("mascot_enabled = false", "mascot_enabled = true"),
        )
        .unwrap();
    });
    h.see("Synthetic title");
    h.send(b"skk");
    h.see("┃ ○ a ");
    h.send(b"\r");
    h.see("p/a READY");
    let spot = |h: &Harness| {
        (0..5)
            .flat_map(|y| (75..140).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let c = h.screen.screen().cell(y, x).unwrap();
                c.fgcolor() == vt100::Color::Rgb(217, 119, 87)
                    || c.bgcolor() == vt100::Color::Rgb(217, 119, 87)
            })
    };
    h.until(|h| spot(h).is_some());
    let start = spot(&h).unwrap().0;
    h.until(|h| spot(h).is_some_and(|(x, _)| x != start));
    assert!(h.input_hex("p/a").is_empty());
    // The fake agent enables SGR mouse input. A click on painted body pixels must
    // be consumed, then the real key acknowledges it.
    let (x, y) = spot(&h).unwrap();
    h.send(format!("\x1b[<0;{};{}M\x1b[<0;{};{}mZ", x + 1, y + 1, x + 1, y + 1).as_bytes());
    h.event("input p/a 5a");
    assert_eq!(h.input_hex("p/a"), "5a");
    let mut agents: serde_json::Value = serde_json::from_str(&h.log("agents.json")).unwrap();
    agents["p/a"] = serde_json::json!("blocked");
    std::fs::write(h.dir.path().join("agents.json"), agents.to_string()).unwrap();
    h.see("waiting");
    h.until(|h| spot(h).is_some());
    assert_eq!(h.locate_from("Agent · p/a", 0, 52).unwrap().1, 3);
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn mascot_config_and_settings_toggle_live_without_changing_agent_input_or_layout() {
    let mut h = Harness::start_prepared("", 16384, |root| {
        let path = root.join("config.toml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            path,
            text.replace("mascot_enabled = true", "mascot_enabled = false"),
        )
        .unwrap();
    });
    h.see("Synthetic title");
    h.send(b"skk");
    h.see("┃ ○ a ");
    h.send(b"\r");
    h.see("p/a READY");
    let visible = |h: &Harness| {
        (0..7).any(|y| {
            (52..140).any(|x| {
                h.screen.screen().cell(y, x).unwrap().fgcolor() == vt100::Color::Rgb(217, 119, 87)
            })
        })
    };
    assert!(!visible(&h), "startup config disables the mascot");
    let original = h.ctl(&["inspect"])["tabs"].clone();
    h.header_tool("Settings");
    h.see("Mascot");
    h.see("Disabled");
    h.click("Mascot");
    h.see("Enabled");
    h.until(|h| h.screen.screen().hide_cursor());
    assert!(!visible(&h), "the draft must not apply before Save");
    h.send(b"\x13");
    h.until(|h| !h.contents().contains("General F1") && visible(h));
    assert_eq!(
        toml::from_str::<toml::Value>(&h.log("config.toml")).unwrap()["mascot_enabled"].as_bool(),
        Some(true)
    );
    h.header_tool("Settings");
    h.click("Mascot");
    h.see("Disabled");
    h.send(b"\x1b");
    h.until(|h| !h.contents().contains("General F1") && visible(h));
    h.header_tool("Settings");
    h.send(b"\x1b[B\x1b[B \x13");
    h.until(|h| !h.contents().contains("General F1") && !visible(h));
    assert_eq!(
        toml::from_str::<toml::Value>(&h.log("config.toml")).unwrap()["mascot_enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(h.ctl(&["inspect"])["tabs"], original);
    assert_eq!(h.locate_from("Agent · p/a", 0, 52).unwrap().1, 3);
    assert!(h.input_hex("p/a").is_empty());
    h.quit();
    assert!(!h.log("events").contains("stop "));
}

#[test]
fn settings_header_opens_directly_and_restores_terminal_focus() {
    let mut h = Harness::start();
    h.see("Synthetic title");
    h.send(b"\r");
    h.see("READY");
    let before = h.log("events");
    h.click("Settings");
    h.see("General F1");
    h.see("Input ▸ Settings");
    assert!(!h.contents().contains("Input ▸ More"));
    assert_eq!(
        h.ctl(&[
            "open",
            "--relative-to",
            "active",
            "--place",
            "tab",
            "--shell"
        ])["error"]["code"],
        "busy"
    );
    // Both halves of a background click stay inside the Settings modal.
    h.send(b"\x1b[<0;130;4M\x1b[<0;130;4m");
    h.click("Cancel Esc");
    h.until(|h| !h.contents().contains("General F1"));
    assert_eq!(h.ctl(&["inspect"])["focus"], "viewer");
    assert_eq!(
        h.log("events").matches("input ").count(),
        before.matches("input ").count(),
        "Settings input never reaches the agent"
    );
    h.quit();
}
