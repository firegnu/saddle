mod common;
use alacritty_terminal::grid::Dimensions;
use saddle::{pty::Session, terminal::Size};
use std::{
    thread,
    time::{Duration, Instant},
};
fn wait_for(session: &Session, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let text: String = session
            .screen
            .lock()
            .unwrap()
            .term
            .grid()
            .display_iter()
            .map(|c| c.c)
            .collect();
        if text.contains(expected) {
            return;
        }
        assert!(Instant::now() < deadline, "missing {expected:?}: {text:?}");
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn pty_input_resize_and_interrupt_only_end_the_owned_attach_process() {
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "fake-attach",
        r#"#!/usr/bin/env python3
import os, signal, sys, tty
# Synthetic agent lifetime is independent of its attach process.
open('agent-alive', 'w').write('alive')
tty.setraw(0)
def size(*args):
    s = os.get_terminal_size(0)
    os.write(1, ('\r\nSIZE=%dx%d\r\n' % (s.columns, s.lines)).encode())
def stop(*args):
    open('detached', 'w').write('SIGINT')
    sys.exit(0)
signal.signal(signal.SIGINT, stop)
signal.signal(signal.SIGWINCH, size)
size()
os.write(1, b'READY\r\n')
while True:
    data = os.read(0, 4096)
    if not data: break
    os.write(1, b'INPUT:' + data + b'\r\n')
"#,
    );
    let mut session =
        Session::spawn(&[program], Some(temp.path()), Size { rows: 10, cols: 40 }).unwrap();
    wait_for(&session, "READY");
    session.send(b"hello".to_vec()).unwrap();
    wait_for(&session, "INPUT:hello");
    session.resize(Size { rows: 12, cols: 50 }).unwrap();
    wait_for(&session, "SIZE=50x12");
    assert_eq!(session.screen.lock().unwrap().term.columns(), 50);
    session.interrupt().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !session.poll_exit().unwrap() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(temp.path().join("detached")).unwrap(),
        "SIGINT"
    );
    assert!(temp.path().join("agent-alive").exists());
}
