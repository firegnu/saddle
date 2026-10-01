use super::*;
use std::{
    cell::RefCell,
    os::{
        fd::RawFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

type Hook = Box<dyn FnOnce(RawFd)>;
thread_local! {
    static AFTER_LOCK: RefCell<Option<Hook>> = const { RefCell::new(None) };
}
pub(super) fn after_lock(fd: RawFd) {
    if let Some(hook) = AFTER_LOCK.take() {
        hook(fd);
    }
}

// Park a real child before exec while Registry owns its actual lock fd.
// The thread-local hook affects only this test's next write, not sibling tests.
struct InheritedLock {
    gate: UnixStream,
    worker: Option<JoinHandle<std::io::Result<ExitStatus>>>,
}
impl InheritedLock {
    fn start(lock: RawFd) -> Self {
        use std::os::fd::AsRawFd;
        let (mut gate, child_gate) = UnixStream::pair().unwrap();
        for socket in [&gate, &child_gate] {
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
        }
        let worker = thread::spawn(move || {
            let fd = child_gate.as_raw_fd();
            let mut command = Command::new("/usr/bin/true");
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            unsafe {
                command.pre_exec(move || {
                    // Only async-signal-safe syscalls between fork and exec.
                    if libc::setsid() < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    let flags = libc::fcntl(lock, libc::F_GETFD);
                    if flags < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    let ready = u8::from(flags & libc::FD_CLOEXEC != 0);
                    if libc::write(fd, (&ready as *const u8).cast(), 1) != 1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    let mut release = 0u8;
                    if libc::read(fd, (&mut release as *mut u8).cast(), 1) != 1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let mut child = command.spawn()?;
            drop(child_gate);
            child.wait()
        });
        let mut ready = [0];
        gate.read_exact(&mut ready).unwrap();
        assert_eq!(
            ready,
            [1],
            "child inherited the CLOEXEC registry lock before exec"
        );
        Self {
            gate,
            worker: Some(worker),
        }
    }
    fn finish(mut self) {
        self.gate.write_all(&[1]).unwrap();
        assert!(
            self.worker
                .take()
                .unwrap()
                .join()
                .unwrap()
                .unwrap()
                .success()
        );
    }
}
impl Drop for InheritedLock {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.gate.write_all(&[1]);
            let _ = worker.join();
        }
    }
}

fn registry_exit_releases_inherited_lock(stale: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plugins.toml");
    let mut registry = Registry::with_reserved(path.clone(), ["test.core"]);
    if stale {
        // A foreign edit invalidates the empty baseline without taking an earlier
        // lock that another test's fork could inherit during fixture setup.
        fs::write(
            &path,
            "version=1\nplugins=[]\n[core.'test.core']\nenabled=false\n",
        )
        .unwrap();
    }
    let child = Arc::new(Mutex::new(None));
    let save_child = child.clone();
    AFTER_LOCK.set(Some(Box::new(move |fd| {
        *save_child.lock().unwrap() = Some(InheritedLock::start(fd));
    })));
    let first = registry.core_enabled("test.core", true);
    if stale {
        assert_eq!(
            first.unwrap_err().to_string(),
            "plugin registry changed; refresh first"
        );
        registry.refresh().unwrap();
    } else {
        first.unwrap();
    }
    // The child is still parked with the first write's fd. Exercise the real next write.
    let next = registry.core_enabled("test.core", false);
    child.lock().unwrap().take().unwrap().finish();
    next.expect("completed registry transaction must unlock before child exec");
    assert!(!Registry::open(path.clone()).core["test.core"].enabled);
    assert!(path.with_extension("lock").exists());
}

#[test]
fn successful_write_unlocks_before_inheriting_child_exec() {
    registry_exit_releases_inherited_lock(false);
}

#[test]
fn stale_write_unlocks_before_inheriting_child_exec() {
    registry_exit_releases_inherited_lock(true);
}

#[test]
fn lock_errors_preserve_os_cause() {
    // A real failed syscall, without constructing an invalid File/OwnedFd.
    let error = acquire_lock(-1).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .and_then(|e| e.raw_os_error()),
        Some(libc::EBADF),
        "{error:#}"
    );
    assert!(!error.to_string().contains("busy"));
}
