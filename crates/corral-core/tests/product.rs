//! Optional installed-shape check. The core itself never links or imports the host.
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    path::PathBuf,
    process::{Child, Command},
    time::{Duration, Instant},
};
struct Product {
    root: tempfile::TempDir,
    core: PathBuf,
    host: PathBuf,
}
impl Product {
    fn cmd(&self, path: &PathBuf) -> Command {
        let mut c = Command::new(path);
        c.env("HOME", self.root.path())
            .env("XDG_CONFIG_HOME", self.root.path().join("config"))
            .env("XDG_STATE_HOME", self.root.path().join("state"))
            .env("SADDLE_RUNTIME_DIR", self.root.path().join("runtime"))
            .env("CORRAL_HOME", self.root.path().join("pens"))
            .env("SHELL", "/bin/sh")
            .env("TERM", "xterm-256color")
            .env_remove("CODEX_SANDBOX")
            .env_remove("CORRAL_NAME")
            .env_remove("CORRAL_INSTANCE");
        c
    }
    fn json(&self, path: &PathBuf, args: &[&str]) -> Value {
        let o = self.cmd(path).args(args).output().unwrap();
        assert!(
            o.status.success(),
            "{args:?} {} {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    }
    fn ui(&self) -> Ui {
        let mut master = -1;
        let mut slave = -1;
        let mut size = libc::winsize {
            ws_row: 35,
            ws_col: 130,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut size,
                )
            },
            0
        );
        let master = unsafe { fs::File::from_raw_fd(master) };
        let slave = unsafe { fs::File::from_raw_fd(slave) };
        unsafe {
            libc::fcntl(master.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }
        let mut c = self.cmd(&self.host);
        c.current_dir(self.root.path())
            .stdin(slave.try_clone().unwrap())
            .stdout(slave.try_clone().unwrap())
            .stderr(fs::File::create(self.root.path().join("host.stderr")).unwrap());
        unsafe {
            c.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = c.spawn().unwrap();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = stop.clone();
        let mut reader = master.try_clone().unwrap();
        let drain = std::thread::spawn(move || {
            let mut bytes = [0; 65536];
            let mut pending = Vec::new();
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(n) = reader.read(&mut bytes) {
                    pending.extend_from_slice(&bytes[..n]);
                    if pending.windows(4).any(|s| s == b"\x1b[6n") {
                        let _ = reader.write_all(b"\x1b[1;1R");
                        pending.clear();
                    }
                    if pending.len() > 4 {
                        pending.drain(..pending.len() - 4);
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });
        Ui {
            child,
            master,
            stop,
            drain: Some(drain),
        }
    }
    fn instance(&self, ui: &mut Ui, agent: bool) -> Value {
        let until = Instant::now() + Duration::from_secs(8);
        loop {
            let v = self.json(&self.host, &["ctl", "instances"]);
            if let Some(v) = v["instances"].as_array().and_then(|a| a.first())
                && v["instance"].is_string()
                && (!agent
                    || v["tabs"].as_array().into_iter().flatten().any(|tab| {
                        tab["panes"].as_array().into_iter().flatten().any(|pane| {
                            pane["agent"] == "product/main"
                                && pane["state"] == "running"
                                && pane["corral_instance"].is_string()
                        })
                    }))
            {
                return v.clone();
            }
            assert!(
                ui.child.try_wait().unwrap().is_none(),
                "TUI exited: {}",
                fs::read_to_string(self.root.path().join("host.stderr")).unwrap_or_default()
            );
            assert!(Instant::now() < until, "{v}");
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}
impl Drop for Product {
    fn drop(&mut self) {
        let _ = self
            .cmd(&self.core)
            .args(["stop", "product/main", "--timeout", "4"])
            .output();
    }
}
struct Ui {
    child: Child,
    master: fs::File,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    drain: Option<std::thread::JoinHandle<()>>,
}
impl Ui {
    fn close(&mut self) {
        self.master.write_all(b"\x1dq").unwrap();
        let end = Instant::now() + Duration::from_secs(6);
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                assert!(s.success());
                return;
            }
            assert!(Instant::now() < end, "TUI did not close");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Ui {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(drain) = self.drain.take() {
            let _ = drain.join();
        }
    }
}
#[test]
#[ignore = "set SADDLE_TEST_HOST to a built host; creates only a temporary product and synthetic cat agent"]
fn tui_exit_and_reopen_preserve_agent_identity_and_runtime() {
    let root = tempfile::tempdir_in("/tmp").unwrap();
    let core = root.path().join("corral");
    let host = root.path().join("saddle");
    fs::copy(
        std::env::var_os("SADDLE_TEST_CORE").unwrap_or_else(|| env!("CARGO_BIN_EXE_corral").into()),
        &core,
    )
    .unwrap();
    fs::copy(
        std::env::var_os("SADDLE_TEST_HOST").expect("explicit host required"),
        &host,
    )
    .unwrap();
    let product = Product { root, core, host };
    let started = product.json(
        &product.core,
        &["start", "product/main", "--cwd", "/tmp", "--", "/bin/cat"],
    );
    let mut ui = product.ui();
    let first = product.instance(&mut ui, false);
    let opened = product.json(
        &product.host,
        &[
            "ctl",
            "open",
            "--instance",
            first["instance"].as_str().unwrap(),
            "--relative-to",
            "active",
            "--place",
            "tab",
            "--agent",
            "product/main",
            "--focus",
        ],
    );
    assert_eq!(opened["ok"], true, "{opened}");
    product.instance(&mut ui, true);
    let until = Instant::now() + Duration::from_secs(6);
    loop {
        if product.json(&product.core, &["status", "product/main"])["attached"] == 1 {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(30));
    }
    let before = product.json(&product.core, &["where", "product/main"]);
    ui.close();
    let status = product.json(&product.core, &["status", "product/main"]);
    assert_eq!(status["instance"], started["instance"]);
    let mut reopened = product.ui();
    let instance = product.instance(&mut reopened, true);
    assert!(
        instance
            .to_string()
            .contains(started["instance"].as_str().unwrap()),
        "{instance}"
    );
    assert_eq!(
        product.json(&product.core, &["where", "product/main"])["agent_pid"],
        before["agent_pid"]
    );
    reopened.close();
    assert_eq!(
        product.json(&product.core, &["status", "product/main"])["instance"],
        started["instance"]
    );
    assert_eq!(
        product.json(&product.host, &["telemetry", "list"])["traces"],
        json!([])
    );
}
