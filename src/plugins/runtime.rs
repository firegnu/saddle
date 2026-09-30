//! One owned child per runtime. All pipe IO, JSON/frame work and reaping happen off the UI.
use super::registry::Manifest;
use anyhow::{Context, Result, ensure};
use saddle_plugin_protocol::{self as wire, Message};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Picture {
    pub frame_id: u64,
    pub revision: u64,
    pub buffer: ratatui::buffer::Buffer,
}
#[derive(Clone)]
pub struct Snapshot {
    pub state: String,
    pub note: String,
    pub pid: Option<u32>,
    pub picture: Option<Arc<Picture>>,
    pub interactive: bool,
    pub log: String,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            state: "Starting".into(),
            note: String::new(),
            pid: None,
            picture: None,
            interactive: false,
            log: String::new(),
        }
    }
}
#[derive(Clone)]
pub struct Toast {
    pub plugin: String,
    pub name: String,
    pub text: String,
    pub expires: Instant,
}
#[derive(Default)]
pub struct Notifications {
    pub items: VecDeque<Toast>,
}
impl Notifications {
    pub fn expire(&mut self) {
        let now = Instant::now();
        self.items.retain(|n| n.expires > now);
    }
    pub fn clear(&mut self, id: &str) {
        self.items.retain(|n| n.plugin != id);
    }
}
pub type Notices = Arc<Mutex<Notifications>>;
#[derive(Default)]
struct Queue {
    items: VecDeque<(Message, usize)>,
    bytes: usize,
}
pub struct Runtime {
    shared: Arc<Mutex<Snapshot>>,
    queue: Arc<Mutex<Queue>>,
    stop: Arc<AtomicBool>,
    overflow: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Runtime {
    pub fn start(dir: &Path, manifest: Manifest) -> Self {
        Self::with_notices(dir, manifest, Notices::default())
    }
    pub fn with_notices(dir: &Path, manifest: Manifest, notices: Notices) -> Self {
        let shared = Arc::new(Mutex::new(Snapshot::default()));
        let queue = Arc::new(Mutex::new(Queue::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let overflow = Arc::new(AtomicBool::new(false));
        let (s, q, c, o) = (
            shared.clone(),
            queue.clone(),
            stop.clone(),
            overflow.clone(),
        );
        let dir = dir.to_owned();
        let worker = thread::spawn(move || {
            let result = run(&dir, &manifest, &s, &q, &c, &o, &notices);
            if let Err(e) = result {
                let mut state = s.lock().unwrap();
                state.state = if state.pid.is_some() {
                    "Stopping"
                } else {
                    "Failed"
                }
                .into();
                state.note = format!("{e:#}")
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(1024)
                    .collect();
                state.interactive = false;
            }
            notices.lock().unwrap().clear(&manifest.id);
        });
        Self {
            shared,
            queue,
            stop,
            overflow,
            worker: Some(worker),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.shared.lock().unwrap().clone()
    }
    pub fn state(&self) -> String {
        self.shared.lock().unwrap().state.clone()
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
    pub fn send(&self, message: Message) -> bool {
        fn weight(v: &Value) -> usize {
            match v {
                Value::String(s) => s.len() + 16,
                Value::Array(a) => 32 + a.iter().map(weight).sum::<usize>(),
                Value::Object(o) => 32 + o.iter().map(|(k, v)| k.len() + weight(v)).sum::<usize>(),
                _ => 16,
            }
        }
        let charge = match &message {
            Message::Event { data, .. } => weight(data) + 128,
            Message::Request { params, .. } => weight(params) + 128,
            _ => 512,
        };
        let mut q = self.queue.lock().unwrap();
        if q.items.len() >= wire::MAX_QUEUE || q.bytes + charge > wire::MAX_QUEUE_BYTES {
            self.overflow.store(true, Ordering::Relaxed);
            return false;
        }
        q.bytes += charge;
        q.items.push_back((message, charge));
        true
    }
    pub fn wait_for(&self, state: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.state() == state {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }
    pub fn reaped(&self) -> bool {
        self.worker.as_ref().is_none_or(|w| w.is_finished())
            && self.shared.lock().unwrap().pid.is_none()
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
fn nonblock(fd: i32) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    ensure!(
        flags >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == 0,
        "set pipe nonblocking failed"
    );
    Ok(())
}
fn state(shared: &Mutex<Snapshot>, name: &str, note: &str) {
    let mut s = shared.lock().unwrap();
    s.state = name.into();
    s.note = note.into();
    if name != "Running" {
        s.interactive = false;
    }
}
fn run(
    dir: &Path,
    m: &Manifest,
    shared: &Mutex<Snapshot>,
    queue: &Mutex<Queue>,
    cancel: &AtomicBool,
    overflow: &AtomicBool,
    notices: &Mutex<Notifications>,
) -> Result<()> {
    // Validate again at launch; a registration is not permission to run a changed identity.
    ensure!(
        Manifest::read(dir)? == *m,
        "plugin manifest changed; refresh"
    );
    let mut cmd = Command::new(m.program(dir)?);
    cmd.args(&m.args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in [
        "CORRAL_NAME",
        "CORRAL_INSTANCE",
        "SADDLE_INSTANCE",
        "SADDLE_PANE",
        "SADDLE_REVISION",
    ] {
        cmd.env_remove(key);
    }
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let mut child = cmd.spawn()?;
    shared.lock().unwrap().pid = Some(child.id());
    let result = (|| -> Result<()> {
        let mut input = child.stdin.take().context("missing input pipe")?;
        let mut output = child.stdout.take().context("missing output pipe")?;
        let mut stderr = child.stderr.take().context("missing log pipe")?;
        nonblock(input.as_raw_fd())?;
        nonblock(output.as_raw_fd())?;
        nonblock(stderr.as_raw_fd())?;
        let mut writeq = VecDeque::new();
        let mut writing = 0;
        let mut line = Vec::new();
        let mut logs = VecDeque::new();
        let start = Instant::now();
        let mut last = start;
        let mut ready = false;
        let mut stopping = None;
        let mut highest = 0;
        let mut next = 1;
        let mut ping = None;
        let mut notified = 0;
        let mut tokens = 5.0f64;
        let mut token_at = start;
        let mut size = (0u16, 0u16, 0u64);
        let mut last_frame = 0;
        let mut failed = None;
        writeq.push_back(wire::encode(&Message::request(1,"initialize",json!({"session":format!("{}-{}",child.id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()),"protocol_major":1,"capabilities":["panel.v1","notify.v1"],"width_profile":wire::PROFILE,"limits":wire::limits(),"theme":{"text":"default","muted":{"indexed":8},"background":"default","accent":{"indexed":6},"error":{"indexed":1}}})))?);
        loop {
            if let Some(status) = child.try_wait()? {
                shared.lock().unwrap().pid = None;
                ensure!(stopping.is_some(), "plugin exited: {status}");
                if let Some(e) = failed {
                    return Err(e);
                }
                state(shared, "Disabled", "");
                return Ok(());
            }
            if cancel.load(Ordering::Relaxed) && stopping.is_none() {
                state(shared, "Stopping", "");
                next += 1;
                writeq.clear();
                writing = 0;
                writeq.push_back(wire::encode(&Message::request(
                    next,
                    "shutdown",
                    json!({}),
                ))?);
                stopping = Some(Instant::now());
            }
            if stopping.is_some_and(|t: Instant| t.elapsed() >= Duration::from_secs(2)) {
                child.kill()?;
                child.wait()?;
                shared.lock().unwrap().pid = None;
                if let Some(e) = failed {
                    return Err(e);
                }
                state(shared, "Disabled", "");
                return Ok(());
            }
            if !ready && start.elapsed() >= Duration::from_secs(5) && stopping.is_none() {
                anyhow::bail!("initialize timed out")
            }
            if overflow.load(Ordering::Relaxed) && stopping.is_none() {
                state(
                    shared,
                    "Unresponsive",
                    "Input channel full; restart or disable",
                );
            }
            if ready && stopping.is_none() && shared.lock().unwrap().state == "Running" {
                if ping.is_some_and(|(_, t): (_, Instant)| t.elapsed() >= Duration::from_secs(5)) {
                    state(shared, "Unresponsive", "Plugin did not answer ping");
                } else if ping.is_none() && last.elapsed() >= Duration::from_secs(15) {
                    next += 1;
                    writeq.push_back(wire::encode(&Message::request(next, "ping", json!({})))?);
                    ping = Some((next, Instant::now()));
                }
                for _ in 0..8 {
                    let item = {
                        let mut q = queue.lock().unwrap();
                        let item = q.items.pop_front();
                        if let Some((_, n)) = &item {
                            q.bytes -= n;
                        }
                        item
                    };
                    let Some((msg, _)) = item else { break };
                    if let Message::Event { name, data } = &msg {
                        if name == "panel.open" || name == "panel.resize" {
                            size = (
                                data["cols"].as_u64().unwrap_or(0) as u16,
                                data["rows_count"].as_u64().unwrap_or(0) as u16,
                                data["size_revision"].as_u64().unwrap_or(0),
                            );
                            shared.lock().unwrap().interactive = false;
                        }
                        if name == "panel.close" {
                            size.0 = 0;
                            size.1 = 0;
                            shared.lock().unwrap().interactive = false;
                        }
                    }
                    let bytes = wire::encode(&msg)?;
                    if writeq.iter().map(Vec::len).sum::<usize>() + bytes.len()
                        > wire::MAX_QUEUE_BYTES
                    {
                        overflow.store(true, Ordering::Relaxed);
                        break;
                    }
                    writeq.push_back(bytes);
                }
            }
            if let Some(bytes) = writeq.front() {
                match input.write(&bytes[writing..]) {
                    Ok(0) => anyhow::bail!("plugin input closed"),
                    Ok(n) => {
                        writing += n;
                        if writing == bytes.len() {
                            writeq.pop_front();
                            writing = 0;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.into()),
                }
            }
            let mut chunk = [0u8; 8192];
            // Finite per-turn read budgets also keep shutdown responsive to an output flood.
            for _ in 0..8 {
                match stderr.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        logs.extend(&chunk[..n]);
                        while logs.len() > 65536 {
                            logs.pop_front();
                        }
                        shared.lock().unwrap().log =
                            String::from_utf8_lossy(logs.make_contiguous()).into_owned();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e.into()),
                }
            }
            for _ in 0..8 {
                match output.read(&mut chunk) {
                    Ok(0) => {
                        if stopping.is_none() {
                            anyhow::bail!("plugin protocol disconnected")
                        }
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e.into()),
                    Ok(n) => {
                        for &b in &chunk[..n] {
                            ensure!(line.len() < wire::MAX_LINE, "message too large");
                            line.push(b);
                            if b != b'\n' {
                                continue;
                            }
                            let message = wire::decode(&line)?;
                            line.clear();
                            last = Instant::now();
                            if stopping.is_some() {
                                continue;
                            }
                            match message {
                                Message::Response { id, result, error } => {
                                    ensure!(id <= next, "unknown response ID");
                                    if id == 1 && !ready {
                                        let result = result.context("initialize rejected")?;
                                        ensure!(
                                            result["id"] == m.id
                                                && result["version"] == m.version
                                                && result["protocol_major"] == 1
                                                && result["width_profile"] == wire::PROFILE,
                                            "plugin identity or protocol mismatch"
                                        );
                                        let caps = result["capabilities"]
                                            .as_array()
                                            .context("missing capabilities")?;
                                        ensure!(
                                            m.required_capabilities
                                                .iter()
                                                .all(|c| caps.iter().any(|v| v == c)),
                                            "missing required capability"
                                        );
                                        ready = true;
                                        state(shared, "Running", "");
                                    } else if ping.is_some_and(|(p, _)| p == id) {
                                        ensure!(error.is_none(), "ping rejected");
                                        ping = None;
                                    }
                                }
                                Message::Request { id, method, params } => {
                                    ensure!(
                                        ready && id > highest,
                                        "request before handshake or duplicate ID"
                                    );
                                    highest = id;
                                    let response = if method == "notify" {
                                        let notification = params["notification_id"]
                                            .as_u64()
                                            .filter(|id| *id > 0)
                                            .context("invalid notification ID")?;
                                        let text = params["text"]
                                            .as_str()
                                            .context("missing notification text")?;
                                        ensure!(
                                            text.len() <= 1024
                                                && !text.chars().any(char::is_control)
                                                && matches!(
                                                    params["level"].as_str(),
                                                    Some("info" | "warning" | "error")
                                                ),
                                            "invalid notification"
                                        );
                                        tokens =
                                            (tokens + token_at.elapsed().as_secs_f64()).min(5.0);
                                        token_at = Instant::now();
                                        let mut n = notices.lock().unwrap();
                                        n.expire();
                                        let status = if notification <= notified {
                                            "duplicate"
                                        } else if tokens < 1.0
                                            || n.items.len() >= 32
                                            || n.items.iter().filter(|n| n.plugin == m.id).count()
                                                >= 8
                                        {
                                            "limited"
                                        } else {
                                            tokens -= 1.0;
                                            notified = notification;
                                            n.items.push_back(Toast {
                                                plugin: m.id.clone(),
                                                name: m.name.clone(),
                                                text: text.into(),
                                                expires: Instant::now() + Duration::from_secs(5),
                                            });
                                            "accepted"
                                        };
                                        Message::response(id, json!({"status":status}))
                                    } else {
                                        Message::error(id, "unsupported", "unknown request")
                                    };
                                    let bytes = wire::encode(&response)?;
                                    ensure!(
                                        writeq.len() < 64
                                            && writeq.iter().map(Vec::len).sum::<usize>()
                                                + bytes.len()
                                                <= wire::MAX_QUEUE_BYTES,
                                        "response channel full"
                                    );
                                    writeq.push_back(bytes);
                                }
                                Message::Event { name, data } => {
                                    ensure!(ready, "event before handshake");
                                    match name.as_str() {
                                        "panel.frame" => {
                                            let frame: wire::Frame = serde_json::from_value(data)?;
                                            frame.validate()?;
                                            ensure!(
                                                frame.frame_id > last_frame
                                                    && frame.size_revision <= size.2,
                                                "invalid frame revision"
                                            );
                                            last_frame = frame.frame_id;
                                            if frame.size_revision == size.2
                                                && size.0 > 0
                                                && size.1 > 0
                                            {
                                                ensure!(
                                                    (frame.cols, frame.rows_count)
                                                        == (size.0, size.1),
                                                    "frame size mismatch"
                                                );
                                                let picture = Arc::new(Picture {
                                                    frame_id: frame.frame_id,
                                                    revision: frame.size_revision,
                                                    buffer: saddle_plugin_sdk::buffer(&frame)?,
                                                });
                                                let mut s = shared.lock().unwrap();
                                                s.picture = Some(picture);
                                                s.interactive = s.state == "Running";
                                                s.note.clear();
                                            }
                                        }
                                        "panel.error" => {
                                            ensure!(
                                                data["panel"] == "main"
                                                    && data["message"]
                                                        .as_str()
                                                        .is_some_and(|m| m.len() <= 1024
                                                            && !m.chars().any(char::is_control))
                                                    && matches!(
                                                        data["code"].as_str(),
                                                        Some(
                                                            "frame_too_large"
                                                                | "unsupported_glyph"
                                                                | "unsupported_style"
                                                                | "render_failed"
                                                        )
                                                    ),
                                                "invalid panel error"
                                            );
                                            if data["size_revision"] == size.2 {
                                                let mut s = shared.lock().unwrap();
                                                s.interactive = false;
                                                s.note = format!(
                                                    "{}: {}",
                                                    data["code"]
                                                        .as_str()
                                                        .unwrap_or("render_failed"),
                                                    data["message"].as_str().unwrap()
                                                );
                                            }
                                        }
                                        "input.rejected" => {
                                            shared.lock().unwrap().note =
                                                "Display changed; retry the action".into()
                                        }
                                        _ if name.starts_with("optional.") => {}
                                        _ => anyhow::bail!("unknown event {name}"),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if writeq.len() > 64 {
                failed = Some(anyhow::anyhow!("output queue full"));
                state(shared, "Stopping", "Protocol error");
                stopping = Some(Instant::now());
            }
            thread::sleep(Duration::from_millis(5));
        }
    })();
    // Every error path owns and reaps only this child, never a process group.
    if shared.lock().unwrap().pid.is_some() {
        state(shared, "Stopping", "Reaping plugin");
        let cleanup = match child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => child.kill().and_then(|()| child.wait().map(|_| ())),
            Err(e) => Err(e),
        };
        match cleanup {
            Ok(()) => shared.lock().unwrap().pid = None,
            Err(e) => {
                state(shared, "Stopping", &format!("Could not reap plugin: {e}"));
                return Err(e.into());
            }
        }
    }
    result
}
