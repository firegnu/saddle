mod common;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[test]
fn three_pane_app_starts_and_restores_terminal_after_quit() {
    let temp = tempfile::tempdir().unwrap();
    let corral = common::script(temp.path(), "corral", "#!/bin/sh\necho '{\"agents\":[]}'\n");
    let queue = common::script(
        temp.path(),
        "queue",
        "#!/usr/bin/env python3\nimport time\nprint('FAKE QUEUE READY',flush=True)\ntime.sleep(30)\n",
    );
    let config = temp.path().join("config.toml");
    std::fs::write(
        &config,
        format!("corral = {corral:?}\n[queue]\ncommand = [{queue:?}]\n"),
    )
    .unwrap();
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
    while !screen.screen().contents().contains("FAKE QUEUE READY") && Instant::now() < deadline {
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
    assert!(snapshot.contains("FAKE QUEUE READY"), "{snapshot}");
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
