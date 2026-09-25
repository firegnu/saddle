use crate::terminal::{Screen, Size};
use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

pub struct Session {
    pub screen: Arc<Mutex<Screen>>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    input: SyncSender<Vec<u8>>,
    size: Size,
    stopping: Option<Instant>,
    exited: bool,
}
impl Session {
    pub fn spawn(command: &[String], cwd: Option<&Path>, size: Size) -> Result<Self> {
        let program = command.first().context("empty terminal command")?;
        let pair = native_pty_system().openpty(pty_size(size))?;
        let mut cmd = CommandBuilder::new(program);
        cmd.args(&command[1..]);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(cwd) = cwd {
            cmd.cwd(cwd);
        }
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("starting {program}"))?;
        drop(pair.slave);
        let screen = Arc::new(Mutex::new(Screen::new(size)));
        let (input, rx) = mpsc::sync_channel::<Vec<u8>>(64);
        thread::spawn(move || {
            while let Ok(bytes) = rx.recv() {
                if writer
                    .write_all(&bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let output = screen.clone();
        let reply = input.clone();
        thread::spawn(move || {
            let mut bytes = [0; 4096];
            loop {
                match reader.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => {
                        let replies = output.lock().unwrap().process(&bytes[..n]);
                        if !replies.is_empty() && reply.send(replies).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            screen,
            master: pair.master,
            child,
            input,
            size,
            stopping: None,
            exited: false,
        })
    }
    pub fn send(&self, bytes: Vec<u8>) -> Result<()> {
        if !bytes.is_empty() {
            self.input
                .try_send(bytes)
                .context("terminal input busy or closed")?;
        }
        Ok(())
    }
    pub fn resize(&mut self, size: Size) -> Result<()> {
        if self.size.rows != size.rows || self.size.cols != size.cols {
            // Resize the parser before the child can redraw for the new size.
            self.screen.lock().unwrap().resize(size);
            self.master.resize(pty_size(size))?;
            self.size = size;
        }
        Ok(())
    }
    pub fn interrupt(&mut self) -> Result<()> {
        if self.stopping.is_none() && !self.poll_exit()? {
            self.signal(libc::SIGINT);
            self.stopping = Some(Instant::now());
        }
        Ok(())
    }
    fn signal(&self, signal: i32) {
        if let Some(pid) = self.child.process_id() {
            // Only this PTY child, never a corral agent or a name/path-based process group.
            unsafe {
                libc::kill(pid as i32, signal);
            }
        }
    }
    pub fn poll_exit(&mut self) -> Result<bool> {
        if self.exited {
            return Ok(true);
        }
        if self.child.try_wait()?.is_some() {
            self.exited = true;
            return Ok(true);
        }
        if self
            .stopping
            .is_some_and(|start| start.elapsed() >= Duration::from_secs(3))
        {
            self.signal(libc::SIGKILL);
        }
        Ok(false)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.interrupt();
        while !self.poll_exit().unwrap_or(true) {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
fn pty_size(size: Size) -> PtySize {
    PtySize {
        rows: size.rows.max(1),
        cols: size.cols.max(2),
        pixel_width: 0,
        pixel_height: 0,
    }
}
