mod common;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[test]
fn agent_workspace_starts_and_restores_terminal_after_quit() {
    let temp = tempfile::tempdir().unwrap();
    let corral = common::script(temp.path(), "corral", "#!/bin/sh\necho '{\"agents\":[]}'\n");
    let config = temp.path().join("config.toml");
    std::fs::write(&config, format!("corral = {corral:?}\n")).unwrap();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_saddle"));
    cmd.args(["--config", config.to_str().unwrap()]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("HOME", temp.path());
    cmd.env("XDG_STATE_HOME", temp.path().join("state"));
    cmd.env("SADDLE_RUNTIME_DIR", temp.path().join("run"));
    cmd.cwd(temp.path());
    let mut child = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = [0; 8192];
        while let Ok(n) = reader.read(&mut bytes) {
            if n == 0 {
                break;
            }
            if tx.send(bytes[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut output = Vec::new();
    let mut answered_cursor = false;
    let mut screen = vt100::Parser::new(30, 120, 0);
    // The first frame can arrive in several reads; wait for both panes, not just the header.
    while !(screen.screen().contents().contains("Plugins")
        && screen.screen().contents().contains("Viewer"))
        && Instant::now() < deadline
    {
        if let Ok(bytes) = rx.recv_timeout(Duration::from_millis(100)) {
            screen.process(&bytes);
            output.extend(bytes);
            if !answered_cursor && output.windows(4).any(|w| w == b"\x1b[6n") {
                writer.write_all(b"\x1b[1;1R").unwrap();
                answered_cursor = true;
            }
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
    }
    let snapshot = screen.screen().contents();
    // Always attempt to stop this owned test process before assertions.
    let _ = writer.write_all(b"q");
    let end = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > end {
            child.kill().unwrap();
            break child.wait().unwrap();
        }
        thread::sleep(Duration::from_millis(20));
    };
    output.extend(rx.try_iter().flatten());
    let text = String::from_utf8_lossy(&output);
    assert!(snapshot.contains("⋯"), "{snapshot}");
    assert!(
        snapshot.contains("Agents") && snapshot.contains("Viewer"),
        "{text}"
    );
    assert!(
        text.contains("\x1b[?1049l"),
        "alternate screen was not restored: {text}"
    );
    assert!(status.success());
}

/// Raw output of saddle's first frame, run under `COLORTERM` = `colorterm` (unset for None).
fn first_frame(colorterm: Option<&str>) -> String {
    let temp = tempfile::tempdir().unwrap();
    let corral = common::script(temp.path(), "corral", "#!/bin/sh\necho '{\"agents\":[]}'\n");
    let config = temp.path().join("config.toml");
    std::fs::write(&config, format!("corral = {corral:?}\n")).unwrap();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 30,
            cols: 160,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_saddle"));
    cmd.args(["--config", config.to_str().unwrap()]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("NO_COLOR", "");
    match colorterm {
        Some(value) => cmd.env("COLORTERM", value),
        None => cmd.env_remove("COLORTERM"),
    }
    cmd.env("HOME", temp.path());
    cmd.env("XDG_STATE_HOME", temp.path().join("state"));
    cmd.env("SADDLE_RUNTIME_DIR", temp.path().join("run"));
    cmd.cwd(temp.path());
    let mut child = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = [0; 8192];
        while let Ok(n) = reader.read(&mut bytes) {
            if n == 0 || tx.send(bytes[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut output = Vec::new();
    let mut answered_cursor = false;
    // The bottom bar is drawn last in the Agents column.
    while !String::from_utf8_lossy(&output).contains("Fold") && Instant::now() < deadline {
        if let Ok(bytes) = rx.recv_timeout(Duration::from_millis(100)) {
            output.extend(bytes);
            if !answered_cursor && output.windows(4).any(|w| w == b"\x1b[6n") {
                writer.write_all(b"\x1b[1;1R").unwrap();
                answered_cursor = true;
            }
        }
    }
    let _ = writer.write_all(b"q");
    let end = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > end {
            child.kill().unwrap();
            let _ = child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[test]
fn agents_colors_follow_the_terminals_announced_color_depth() {
    // Agents background #1d1a16 is sent as 24-bit only when the terminal says it supports it.
    let truecolor = first_frame(Some("truecolor"));
    assert!(truecolor.contains("48;2;29;26;22"), "{truecolor}");
    let limited = first_frame(None);
    assert!(limited.contains("Fold"), "{limited}");
    assert!(!limited.contains("48;2;29;26;22"), "{limited}");
    assert!(limited.contains("48;5;234"), "{limited}");
}

/// Feeds saddle's output to `screen` until `done` holds, for up to five seconds.
fn pump(
    rx: &mpsc::Receiver<Vec<u8>>,
    screen: &mut vt100::Parser,
    writer: &mut Box<dyn Write + Send>,
    done: impl Fn(&vt100::Screen) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done(screen.screen()) && Instant::now() < deadline {
        if let Ok(bytes) = rx.recv_timeout(Duration::from_millis(50)) {
            if bytes.windows(4).any(|w| w == b"\x1b[6n") {
                writer.write_all(b"\x1b[1;1R").unwrap();
            }
            screen.process(&bytes);
        }
    }
}
/// The screen column where `text` starts on row `y`, for single-width text.
fn column(screen: &vt100::Screen, y: u16, text: &str) -> Option<usize> {
    let row = screen.rows(0, screen.size().1).nth(usize::from(y))?;
    row.find(text).map(|i| row[..i].chars().count())
}

#[test]
fn settings_open_with_comma_save_to_the_file_and_resize_the_sidebar_at_once() {
    let temp = tempfile::tempdir().unwrap();
    let corral = common::script(temp.path(), "corral", "#!/bin/sh\necho '{\"agents\":[]}'\n");
    let config = temp.path().join("config.toml");
    let original = format!("# mine\ncorral = {corral:?}\nleft_width = 52 # cols\n");
    std::fs::write(&config, &original).unwrap();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 40,
            cols: 160,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_saddle"));
    cmd.args(["--config", config.to_str().unwrap()]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("HOME", temp.path());
    cmd.env("XDG_STATE_HOME", temp.path().join("state"));
    cmd.env("SADDLE_RUNTIME_DIR", temp.path().join("run"));
    cmd.cwd(temp.path());
    let mut child = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = [0; 8192];
        while let Ok(n) = reader.read(&mut bytes) {
            if n == 0 || tx.send(bytes[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let mut screen = vt100::Parser::new(40, 160, 0);
    pump(&rx, &mut screen, &mut writer, |s| {
        column(s, 1, "⋯").is_some() && s.contents().contains("Fold")
    });
    let before = column(screen.screen(), 1, "⋯");
    writer.write_all(b",").unwrap();
    pump(&rx, &mut screen, &mut writer, |s| {
        s.contents().contains("Config: ~/config.toml") && s.contents().contains("Input ▸ Settings")
    });
    let opened = screen.screen().contents();
    // Ctrl-U clears the sidebar width, then Ctrl-S saves 60.
    writer.write_all(b"\x1560\x13").unwrap();
    pump(&rx, &mut screen, &mut writer, |s| {
        s.contents().contains("Settings saved") && column(s, 1, "⋯") == Some(56)
    });
    let saved = screen.screen().contents();
    let after = column(screen.screen(), 1, "⋯");
    let file = std::fs::read_to_string(&config).unwrap();
    // Esc cancels without writing.
    writer.write_all(b",").unwrap();
    pump(&rx, &mut screen, &mut writer, |s| {
        s.contents().contains("Input ▸ Settings")
    });
    writer.write_all(b"\x1599\x1b").unwrap();
    pump(&rx, &mut screen, &mut writer, |s| {
        !s.contents().contains("Config: ") && s.contents().contains("Input ▸ Agents")
    });
    let cancelled = screen.screen().contents();
    let _ = writer.write_all(b"q");
    let end = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > end {
            child.kill().unwrap();
            let _ = child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(before, Some(48), "{opened}");
    assert!(opened.contains("Input ▸ Settings"), "{opened}");
    assert_eq!(after, Some(56), "{saved}");
    assert!(saved.contains("Input ▸ Agents"), "{saved}");
    assert_eq!(
        file,
        original.replace("left_width = 52 #", "left_width = 60 #")
    );
    assert!(cancelled.contains("Input ▸ Agents"), "{cancelled}");
    assert_eq!(std::fs::read_to_string(&config).unwrap(), file);
}
