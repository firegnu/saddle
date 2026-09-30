use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
struct Peer {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Value>,
    frame: Value,
    sequence: u64,
    trace: tempfile::TempDir,
}
impl Peer {
    fn start() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let trace = tempfile::tempdir().unwrap();
        let real = Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        let real = String::from_utf8(real.stdout).unwrap();
        let wrapper = trace.path().join("git");
        std::fs::write(&wrapper, "#!/bin/sh\nprintf 'read\\n' >> \"$DIFF_TEST_LOG\"\nexec \"$DIFF_TEST_REAL_GIT\" \"$@\"\n").unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut paths = vec![trace.path().to_path_buf()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let mut child = Command::new(env!("CARGO_BIN_EXE_saddle-diff"))
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("DIFF_TEST_REAL_GIT", real.trim())
            .env("DIFF_TEST_LOG", trace.path().join("calls"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, messages) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else {
                    break;
                };
                let value = serde_json::from_str(&line).expect("JSON protocol output");
                if tx.send(value).is_err() {
                    break;
                }
            }
        });
        let mut p = Self {
            child,
            input,
            messages,
            frame: Value::Null,
            sequence: 0,
            trace,
        };
        p.send(json!({"kind":"request","id":1,"method":"initialize","params":{"protocol_major":1,"width_profile":"saddle-grapheme-v1","capabilities":["panel.v1","view.context.v1","view.close.v1"]}}));
        assert_eq!(
            p.messages.recv_timeout(Duration::from_secs(3)).unwrap()["result"]["id"],
            "diff"
        );
        p
    }
    fn send(&mut self, value: Value) {
        writeln!(self.input, "{value}").unwrap();
        self.input.flush().unwrap();
    }
    fn event(&mut self, name: &str, data: Value) {
        self.send(json!({"kind":"event","name":name,"data":data}));
    }
    fn context(&mut self, cwd: &Path) {
        self.event("optional.view_context", json!({"cwd":cwd}));
    }
    fn open(&mut self) {
        self.event(
            "panel.open",
            json!({"panel":"main","cols":150,"rows_count":40,"size_revision":1}),
        );
        self.event("panel.focus", json!({"focused":true}));
    }
    fn frame_text(frame: &Value) -> String {
        frame["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                r.as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|s| s["text"].as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn until(&mut self, text: &str) -> String {
        let start = Instant::now();
        let mut last = String::new();
        while start.elapsed() < Duration::from_secs(10) {
            if let Ok(v) = self.messages.recv_timeout(Duration::from_millis(100)) {
                assert_ne!(v["name"], "panel.error", "{v}");
                if v["name"] == "panel.frame" {
                    let f: saddle_plugin_sdk::protocol::Frame =
                        serde_json::from_value(v["data"].clone()).unwrap();
                    f.validate().unwrap();
                    self.frame = v["data"].clone();
                    last = Self::frame_text(&self.frame);
                    if last.contains(text) {
                        return last;
                    }
                }
            }
        }
        panic!("Missing {text}: {last}");
    }
    fn key(&mut self, ch: &str) {
        self.sequence += 1;
        self.event("input", json!({"panel":"main","frame_id":self.frame["frame_id"],"size_revision":1,"input_id":self.sequence,"event":{"type":"key","phase":"press","modifiers":[],"code":{"char":ch}}}));
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn git(path: &Path, args: &[&str]) {
    let o = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    std::fs::write(d.path().join("a.txt"), "first\n").unwrap();
    std::fs::write(d.path().join("b.txt"), "second\n").unwrap();
    d
}
#[test]
fn continuous_view_updates_unfocused_switches_modes_and_reopens_fresh() {
    let d = repo();
    let other = repo();
    std::fs::write(other.path().join("a.txt"), "another_project\n").unwrap();
    let mut p = Peer::start();
    p.context(d.path());
    p.open();
    let text = p.until("second");
    assert!(text.contains("first") && text.contains("2 files"));
    // Switching to the already-open view has no new source cwd.
    p.event("optional.view_context", json!({"cwd":null}));
    // The SDK suppresses identical frames; change layout to observe the retained content.
    p.key("v");
    p.until("second");
    p.event("panel.focus", json!({"focused":false}));
    std::fs::write(d.path().join("a.txt"), "automatically_updated\n").unwrap();
    p.until("automatically_updated");
    p.event("panel.focus", json!({"focused":true}));
    p.key("3");
    p.until("No changes in this mode.");
    git(d.path(), &["add", "a.txt"]);
    p.until("automatically_updated");
    p.key("1");
    p.until("second");
    p.context(other.path());
    let text = p.until("another_project");
    assert!(!text.contains("automatically_updated"));
    p.event("panel.close", json!({"panel":"main"}));
    // A ping provides an ordered barrier after close; no refresh frames follow it.
    p.send(json!({"kind":"request","id":2,"method":"ping","params":{}}));
    while p.messages.recv_timeout(Duration::from_secs(3)).unwrap()["id"] != 2 {}
    let calls = std::fs::read(p.trace.path().join("calls")).unwrap();
    std::fs::write(other.path().join("a.txt"), "edited_while_closed\n").unwrap();
    assert!(
        p.messages
            .recv_timeout(Duration::from_millis(1100))
            .is_err()
    );
    assert_eq!(
        calls,
        std::fs::read(p.trace.path().join("calls")).unwrap(),
        "Closed view must stop Git polling"
    );
    p.open();
    p.until("edited_while_closed");
    // Reopening from outside Git must not leave the previous project's diff visible.
    let plain = tempfile::tempdir().unwrap();
    p.context(plain.path());
    let text = p.until("STALE / error");
    assert!(!text.contains("edited_while_closed"));
}

#[test]
fn switching_context_during_a_scan_discards_the_old_result() {
    let slow = repo();
    for n in 0..150 {
        std::fs::write(
            slow.path().join(format!("{n:03}.txt")),
            "obsolete_project\n",
        )
        .unwrap();
    }
    let current = repo();
    std::fs::write(current.path().join("a.txt"), "current_project\n").unwrap();
    let mut p = Peer::start();
    p.context(slow.path());
    p.open();
    p.until("Loading");
    // Wait for actual Git calls, ensuring the old job has started before switching.
    let start = Instant::now();
    while std::fs::read(p.trace.path().join("calls"))
        .unwrap_or_default()
        .len()
        < 30
    {
        assert!(start.elapsed() < Duration::from_secs(3));
        thread::sleep(Duration::from_millis(10));
    }
    p.context(current.path());
    let text = p.until("current_project");
    assert!(!text.contains("obsolete_project"));
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1100) {
        if let Ok(v) = p.messages.recv_timeout(Duration::from_millis(100)) {
            if v["name"] == "panel.frame" {
                assert!(!Peer::frame_text(&v["data"]).contains("obsolete_project"));
            }
            assert_ne!(v["name"], "panel.error");
        }
    }
}
