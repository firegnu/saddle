use anyhow::{Context, Result, bail};
use std::{
    ffi::OsString,
    io::Read,
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub fn run(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Output> {
    run_without_env(program, args, cwd, &[], timeout, cancel)
}
/// Like `run`, but the child does not inherit the environment variables named in `remove`.
pub fn run_without_env(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    remove: &[OsString],
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Output> {
    let mut command = Command::new(program);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for name in remove {
        command.env_remove(name);
    }
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("starting {} {}", program, args.join(" ")))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    for (is_stdout, mut pipe) in [
        (true, Box::new(stdout) as Box<dyn Read + Send>),
        (false, Box::new(stderr)),
    ] {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
            let _ = tx.send((is_stdout, result));
        });
    }
    drop(tx);
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("{} cancelled or timed out", args.join(" "));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let mut output = Vec::new();
    let mut errors = Vec::new();
    for _ in 0..2 {
        let (is_stdout, bytes) = rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .context("command output timed out")?;
        if is_stdout {
            output = bytes?;
        } else {
            errors = bytes?;
        }
    }
    Ok(Output {
        status,
        stdout: output,
        stderr: errors,
    })
}
