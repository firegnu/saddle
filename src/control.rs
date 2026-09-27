//! Bounded, local JSON control transport. Only the UI loop mutates the workspace.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

#[path = "control_cli.rs"]
mod cli;
pub use cli::run;

pub const LIMIT: usize = 64 * 1024;
pub const RECORD_LIMIT: usize = 256;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Caller {
    pub name: Option<String>,
    pub corral_instance: Option<String>,
    pub saddle_instance: Option<String>,
    pub pane: Option<u64>,
    pub revision: Option<u64>,
}
impl Caller {
    pub fn environment() -> Self {
        let env = |key| std::env::var(key).ok().filter(|s| !s.is_empty());
        Self {
            name: env("CORRAL_NAME"),
            corral_instance: env("CORRAL_INSTANCE"),
            saddle_instance: env("SADDLE_INSTANCE"),
            pane: env("SADDLE_PANE").and_then(|s| s.parse().ok()),
            revision: env("SADDLE_REVISION").and_then(|s| s.parse().ok()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Content {
    Shell {
        cwd: Option<String>,
    },
    Agent {
        name: String,
    },
    NewAgent {
        name: String,
        cwd: Option<String>,
        role: String,
        prompt: Option<String>,
        argv: Vec<String>,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CloseTarget {
    Pane(u64),
    Tab(u64),
    All,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Inspect,
    Request {
        request: String,
    },
    Open {
        relative_to: String,
        place: crate::terminals::Place,
        content: Content,
        focus: bool,
    },
    Close {
        target: CloseTarget,
        confirmation: Option<String>,
        confirm_shells: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub instance: String,
    pub request_id: Option<String>,
    pub caller: Caller,
    pub operation: Operation,
}
pub struct Incoming {
    pub message: Message,
    pub reply: SyncSender<Value>,
}
pub fn error(code: &str, message: impl ToString) -> Value {
    json!({"ok": false, "error": {"code": code, "message": message.to_string()}})
}
pub fn random_id() -> Result<String> {
    let mut bytes = [0_u8; 8];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn runtime_dir() -> Result<PathBuf> {
    let absolute = |name| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    if let Some(dir) = std::env::var_os("SADDLE_RUNTIME_DIR") {
        anyhow::ensure!(
            PathBuf::from(dir).is_absolute(),
            "SADDLE_RUNTIME_DIR must be absolute"
        );
    }
    let dir = if let Some(dir) = absolute("SADDLE_RUNTIME_DIR") {
        dir
    } else if let Some(dir) = absolute("XDG_RUNTIME_DIR") {
        dir.join("saddle")
    } else {
        absolute("XDG_CACHE_HOME")
            .or_else(|| absolute("HOME").map(|p| p.join(".cache")))
            .context("no private runtime directory; set SADDLE_RUNTIME_DIR to an absolute path")?
            .join("saddle/run")
    };
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    let metadata = fs::symlink_metadata(&dir)?;
    if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
        bail!("runtime directory must be owned by the current user and not a symlink");
    }
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}
fn socket_path(id: &str) -> Result<PathBuf> {
    if id.len() != 16 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid instance ID");
    }
    let path = runtime_dir()?.join(format!("{id}.sock"));
    // sockaddr_un.sun_path is 104 bytes on macOS, 108 on Linux.
    if path.as_os_str().as_encoded_bytes().len() >= 104 {
        bail!(
            "runtime socket path too long; set SADDLE_RUNTIME_DIR to a shorter private directory"
        );
    }
    Ok(path)
}
fn read_json(stream: &mut UnixStream) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        stream.set_read_timeout(Some(
            deadline
                .checked_duration_since(Instant::now())
                .context("read timed out")?,
        ))?;
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            bail!("incomplete control message");
        }
        let end = chunk[..n].iter().position(|b| *b == b'\n');
        bytes.extend_from_slice(&chunk[..end.unwrap_or(n)]);
        if bytes.len() > LIMIT {
            bail!("control message exceeds 64 KiB");
        }
        if end.is_some() {
            return Ok(serde_json::from_slice(&bytes)?);
        }
    }
}
fn write_json(stream: &mut UnixStream, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() > LIMIT {
        bail!("control message exceeds 64 KiB");
    }
    bytes.push(b'\n');
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(&bytes)?;
    Ok(())
}
fn connect(path: &std::path::Path) -> Result<UnixStream> {
    use std::os::fd::{AsRawFd, FromRawFd};
    // Nonblocking connect also bounds a full Unix listen backlog.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    unsafe {
        libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
    }
    stream.set_nonblocking(true)?;
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as _;
    for (out, byte) in address
        .sun_path
        .iter_mut()
        .zip(path.as_os_str().as_encoded_bytes())
    {
        *out = *byte as _;
    }
    let length = std::mem::size_of_val(&address) as libc::socklen_t;
    #[cfg(target_os = "macos")]
    {
        address.sun_len = length as u8;
    }
    let status =
        unsafe { libc::connect(fd, &address as *const _ as *const libc::sockaddr, length) };
    if status < 0 {
        let error = std::io::Error::last_os_error();
        if !matches!(
            error.raw_os_error(),
            Some(libc::EINPROGRESS) | Some(libc::EAGAIN)
        ) {
            return Err(error.into());
        }
        let mut poll = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLOUT,
            revents: 0,
        };
        anyhow::ensure!(
            unsafe { libc::poll(&mut poll, 1, 500) } > 0,
            "socket connection timed out"
        );
        if let Some(error) = stream.take_error()? {
            return Err(error.into());
        }
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}
pub fn exchange(message: &Message) -> Result<Value> {
    let mut stream = connect(&socket_path(&message.instance)?)
        .context("instance unavailable (it may have exited or restarted)")?;
    write_json(&mut stream, message)?;
    read_json(&mut stream)
}
pub fn instances(caller: &Caller) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(3);
    for entry in fs::read_dir(runtime_dir()?)?.take(256) {
        anyhow::ensure!(
            Instant::now() < deadline,
            "instance discovery timed out; use an explicit --instance"
        );
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|s| s != "sock") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Ok(value) = exchange(&Message {
            instance: id.into(),
            request_id: None,
            caller: caller.clone(),
            operation: Operation::Inspect,
        }) && value["ok"] == true
        {
            result.push(value);
        }
    }
    result.sort_by(|a, b| a["instance"].as_str().cmp(&b["instance"].as_str()));
    Ok(result)
}

pub struct Server {
    pub id: String,
    pub incoming: Receiver<Incoming>,
    path: PathBuf,
    stop: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
}
impl Server {
    pub fn start() -> Result<Self> {
        let id = random_id()?;
        let path = socket_path(&id)?;
        let listener = UnixListener::bind(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, incoming) = mpsc::sync_channel::<Incoming>(32);
        let (sockets, rx) = mpsc::sync_channel::<UnixStream>(8);
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = Vec::new();
        for _ in 0..4 {
            let (rx, tx, stop) = (rx.clone(), tx.clone(), stop.clone());
            workers.push(thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let stream = rx.lock().unwrap().recv_timeout(Duration::from_millis(50));
                    let Ok(mut stream) = stream else {
                        continue;
                    };
                    let value = match read_json(&mut stream)
                        .and_then(|v| Ok(serde_json::from_value::<Message>(v)?))
                    {
                        Ok(message) => {
                            let (reply, wait) = mpsc::sync_channel(1);
                            let id = message.request_id.clone();
                            if tx.try_send(Incoming { message, reply }).is_err() {
                                error("busy", "control queue full")
                            } else {
                                wait.recv_timeout(Duration::from_secs(1)).unwrap_or_else(
                                    |_| json!({"ok":true,"state":"uncertain","request_id":id}),
                                )
                            }
                        }
                        Err(e) => error("invalid_request", e),
                    };
                    if write_json(&mut stream, &value).is_err() {
                        let _ = write_json(
                            &mut stream,
                            &error("response_too_large", "response exceeds transport limit"),
                        );
                    }
                }
            }));
        }
        let done = stop.clone();
        workers.push(thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = sockets.try_send(stream);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
        }));
        Ok(Self {
            id,
            incoming,
            path,
            stop,
            workers,
        })
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = fs::remove_file(&self.path);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
