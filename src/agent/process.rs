//! Supervise only this invocation's short-lived CLI. Never stop an agent by name.
use super::*;
use std::{
    io, mem,
    os::{
        fd::{AsRawFd, RawFd},
        unix::process::ExitStatusExt,
    },
    process::{ExitStatus, Stdio},
    sync::atomic::{AtomicI32, Ordering},
    time::{Duration, Instant},
};

// Independent limit on the JSON envelope. Business output keeps streaming after overflow.
const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
static SIGNAL: AtomicI32 = AtomicI32::new(0);
extern "C" fn signal_received(signal: libc::c_int) {
    let _ = SIGNAL.compare_exchange(0, signal, Ordering::Relaxed, Ordering::Relaxed);
}

pub(super) struct Signals(Vec<(i32, libc::sigaction)>);
impl Signals {
    pub fn install() -> io::Result<Self> {
        SIGNAL.store(0, Ordering::Relaxed);
        let mut guard = Self(vec![]);
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // Only an atomic store occurs in the signal handler. No locks, IO or allocation.
            let mut action: libc::sigaction = unsafe { mem::zeroed() };
            action.sa_sigaction = signal_received as *const () as usize;
            let mut old = unsafe { mem::zeroed() };
            if unsafe { libc::sigaction(signal, &action, &mut old) } != 0 {
                return Err(io::Error::last_os_error());
            }
            guard.0.push((signal, old));
        }
        Ok(guard)
    }
    pub fn received(&self) -> i32 {
        SIGNAL.load(Ordering::Relaxed)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for (signal, old) in &self.0 {
            unsafe {
                libc::sigaction(*signal, old, std::ptr::null_mut());
            }
        }
    }
}

pub(super) struct Result {
    pub executed: bool,
    pub code: i32,
    pub outcome: Value,
    pub stdout: Vec<u8>,
    pub output_gap: Option<&'static str>,
    pub stderr_terminated: bool,
    pub streams_complete: bool,
}

fn nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// The final receipt shares the same output backpressure/cancellation boundary as business IO.
/// An incomplete line is intentionally unusable as a final receipt.
pub(super) fn write_receipt(mut bytes: &[u8], signals: &Signals, deadline: Option<Instant>) {
    while !bytes.is_empty() {
        let mut ready = libc::pollfd {
            fd: libc::STDERR_FILENO,
            events: libc::POLLOUT,
            revents: 0,
        };
        if unsafe { libc::poll(&mut ready, 1, 0) } > 0 {
            let count =
                unsafe { libc::write(ready.fd, bytes.as_ptr().cast(), bytes.len().min(512)) };
            if count > 0 {
                bytes = &bytes[count as usize..];
                continue;
            }
            if count < 0
                && !matches!(
                    io::Error::last_os_error().kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                )
            {
                return;
            }
        }
        if signals.received() != 0 || deadline.is_some_and(|d| Instant::now() >= d) {
            return;
        }
        unsafe {
            libc::poll(&mut ready, 1, 5);
        }
    }
}

pub(super) fn run(
    invocation: &Invocation<'_>,
    capture: bool,
    signals: &Signals,
    deadline: Option<Instant>,
) -> Result {
    let mut result = Result {
        executed: false,
        code: 127,
        outcome: json!({"kind":"spawn_failed"}),
        stdout: vec![],
        output_gap: None,
        stderr_terminated: true,
        streams_complete: true,
    };
    if signals.received() != 0 {
        result.code = 128 + signals.received();
        result.outcome = json!({"kind":"signaled","signal":signals.received()});
        return result;
    }
    if deadline.is_some_and(|d| Instant::now() >= d) {
        result.code = 124;
        result.outcome = json!({"kind":"timed_out"});
        return result;
    }
    // Preserve stdin and the environment. Caller-owned process group is inherited unchanged.
    let child = Command::new(&invocation.program)
        .args(invocation.business)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return result;
    };
    result.executed = true;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    if nonblocking(stdout.as_raw_fd())
        .and_then(|_| nonblocking(stderr.as_raw_fd()))
        .is_err()
    {
        let _ = child.kill();
        let _ = child.wait();
        result.code = 1;
        result.outcome = json!({"kind":"unknown"});
        result.output_gap = Some("unreadable");
        return result;
    }
    let mut status: Option<ExitStatus> = None;
    let mut stopping = None;
    let mut timed_out = false;
    let mut pipes = [true, true];
    let mut buffer = [0; 16384];
    let mut pending = [Vec::new(), Vec::new()];
    let mut offsets = [0, 0];
    let mut sink_failed = [false, false];
    loop {
        // Drain bounded chunks so a noisy child cannot starve signal/timeout supervision.
        for (index, source) in [&mut stdout as &mut dyn Read, &mut stderr]
            .into_iter()
            .enumerate()
        {
            if !pending[index].is_empty() || !pipes[index] {
                continue;
            }
            match source.read(&mut buffer) {
                Ok(0) => pipes[index] = false,
                Ok(n) => {
                    let bytes = &buffer[..n];
                    if index == 0 && capture && result.output_gap.is_none() {
                        if result.stdout.len() + n <= MAX_OUTPUT_BYTES {
                            result.stdout.extend_from_slice(bytes);
                        } else {
                            result.stdout.clear();
                            result.output_gap = Some("too_large");
                        }
                    }
                    pending[index].extend_from_slice(bytes);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => {
                    pipes[index] = false;
                    result.output_gap = Some("unreadable");
                }
            }
        }
        for index in 0..2 {
            if pending[index].is_empty() {
                continue;
            }
            let fd = if index == 0 {
                libc::STDOUT_FILENO
            } else {
                libc::STDERR_FILENO
            };
            let mut ready = libc::pollfd {
                fd,
                events: libc::POLLOUT,
                revents: 0,
            };
            if !sink_failed[index] && unsafe { libc::poll(&mut ready, 1, 0) } > 0 {
                // POSIX pipes guarantee atomic writes up to at least 512 bytes. Poll and
                // small raw writes avoid write_all's uninterruptible retry/backpressure,
                // without changing shared inherited descriptor flags (including stdin).
                let bytes = &pending[index][offsets[index]..];
                let count = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len().min(512)) };
                if count > 0 {
                    offsets[index] += count as usize;
                    if index == 1 {
                        result.stderr_terminated = pending[index][offsets[index] - 1] == b'\n';
                    }
                } else if count < 0
                    && !matches!(
                        io::Error::last_os_error().kind(),
                        io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                    )
                {
                    sink_failed[index] = true;
                    result.output_gap = Some("unreadable");
                }
            }
            if sink_failed[index] || offsets[index] == pending[index].len() {
                pending[index].clear();
                offsets[index] = 0;
            }
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(found) => status = found,
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    result.code = 1;
                    result.outcome = json!({"kind":"unknown"});
                    result.output_gap = Some("unreadable");
                    return result;
                }
            }
        }
        if status.is_some() && !pipes.iter().any(|p| *p) && pending.iter().all(Vec::is_empty) {
            break;
        }
        let now = Instant::now();
        if stopping.is_none() && (signals.received() != 0 || deadline.is_some_and(|d| now >= d)) {
            timed_out = signals.received() == 0;
            stopping = Some(now);
            if status.is_none() {
                let signal = if timed_out {
                    libc::SIGTERM
                } else {
                    signals.received()
                };
                unsafe {
                    libc::kill(child.id() as i32, signal);
                }
            }
        }
        if stopping.is_some_and(|started| now.duration_since(started) >= Duration::from_millis(200))
        {
            if status.is_none() {
                let _ = child.kill();
                status = child.wait().ok();
                continue; // one last nonblocking drain after reaping
            }
            if pipes.iter().any(|p| *p) || pending.iter().any(|p| !p.is_empty()) {
                result.output_gap = Some("unreadable");
            }
            result.streams_complete = false;
            // Descendants holding a pipe open do not extend cancellation indefinitely.
            break;
        }
        let mut fds = [
            libc::pollfd {
                fd: if pipes[0] && pending[0].is_empty() {
                    stdout.as_raw_fd()
                } else {
                    -1
                },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if pipes[1] && pending[1].is_empty() {
                    stderr.as_raw_fd()
                } else {
                    -1
                },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if pending[0].is_empty() {
                    -1
                } else {
                    libc::STDOUT_FILENO
                },
                events: libc::POLLOUT,
                revents: 0,
            },
            libc::pollfd {
                fd: if pending[1].is_empty() {
                    -1
                } else {
                    libc::STDERR_FILENO
                },
                events: libc::POLLOUT,
                revents: 0,
            },
        ];
        unsafe {
            libc::poll(fds.as_mut_ptr(), fds.len() as _, 5);
        }
    }
    if sink_failed.iter().any(|v| *v) {
        result.streams_complete = false;
    }
    if timed_out {
        result.code = 124;
        result.outcome = json!({"kind":"timed_out"});
    } else if let Some(status) = status {
        result.code = status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(0));
        result.outcome = if let Some(code) = status.code() {
            json!({"kind":"exited","exit_code":code})
        } else {
            json!({"kind":"signaled","signal":status.signal()})
        };
    } else {
        result.code = 1;
        result.outcome = json!({"kind":"unknown"});
    }
    result
}
