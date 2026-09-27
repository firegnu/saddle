use crate::{pty::Session, terminal::Size};
use anyhow::Result;
use std::thread::{self, JoinHandle};

pub struct Shell {
    pub program: String,
    pub cwd: String,
    pub state: &'static str,
    pub exit_code: Option<u32>,
    pub env: Vec<(String, String)>,
}

pub struct Viewer {
    pub session: Option<Session>,
    pub showing: Option<String>,
    pub note: String,
    pub shell: Option<Shell>,
    pub agent_instance: Option<String>,
    closing: bool,
    pending: Option<String>,
    generation: u64,
    corral: String,
    spawning: Option<(u64, String, JoinHandle<Result<Session>>)>,
}
impl Viewer {
    pub fn new(corral: String) -> Self {
        Self {
            session: None,
            showing: None,
            pending: None,
            generation: 0,
            corral,
            spawning: None,
            shell: None,
            agent_instance: None,
            closing: false,
            note: "Select an agent on the left, then press Enter or click.".into(),
        }
    }
    pub fn select(&mut self, name: String) -> Result<()> {
        self.closing = false;
        if self.showing.as_ref() == Some(&name)
            && self
                .session
                .as_ref()
                .is_some_and(|session| !session.is_stopping())
        {
            return Ok(());
        }
        self.cancel_pending();
        self.pending = Some(name);
        if let Some(session) = &mut self.session {
            session.interrupt()?;
        }
        Ok(())
    }
    pub fn disappeared(&mut self, names: &[&str]) -> Result<()> {
        if self.shell.is_some() {
            return Ok(());
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|name| !names.contains(&name.as_str()))
        {
            self.cancel_pending();
        }
        if self
            .showing
            .as_ref()
            .is_some_and(|name| !names.contains(&name.as_str()))
            && let Some(session) = &mut self.session
        {
            session.interrupt()?;
        }
        Ok(())
    }
    pub fn target(&self) -> Option<&str> {
        self.pending.as_deref().or(self.showing.as_deref())
    }
    /// Invalidate pending work without disconnecting the currently displayed session.
    pub fn cancel_pending(&mut self) {
        self.pending = None;
        self.generation += 1;
    }
    pub fn close(&mut self) -> Result<()> {
        self.closing = true;
        self.cancel_pending();
        if let Some(session) = &mut self.session {
            session.interrupt()?;
        }
        Ok(())
    }
    pub fn closed(&self) -> bool {
        self.session.is_none() && self.spawning.is_none() && self.pending.is_none()
    }
    pub fn tick(&mut self, size: Size) -> Result<()> {
        self.tick_visible(Some(size))
    }
    pub fn tick_visible(&mut self, size: Option<Size>) -> Result<()> {
        if let Some(session) = &mut self.session {
            if session.poll_exit()? {
                if let Some(shell) = &mut self.shell {
                    shell.state = "exited";
                    shell.exit_code = session.exit_code();
                    self.note = format!("Terminal exited ({})", shell.exit_code.unwrap_or(0));
                }
                if self.shell.is_none() || self.closing {
                    self.session = None;
                }
                if let Some(name) = self.showing.take() {
                    self.note =
                        format!("{name} attach exited. Select an agent on the left to reconnect.");
                }
            } else if let Some(size) = size {
                session.resize(size)?;
            }
        }
        if self
            .spawning
            .as_ref()
            .is_some_and(|(_, _, worker)| worker.is_finished())
        {
            let (generation, name, worker) = self.spawning.take().unwrap();
            let shell_spawn = name.is_empty();
            let current = generation == self.generation
                && !self.closing
                && if shell_spawn {
                    self.shell.as_ref().is_some_and(|s| s.state == "starting")
                } else {
                    self.pending.as_ref() == Some(&name)
                };
            match worker.join().expect("attach worker panicked") {
                Ok(mut session) => {
                    if current {
                        self.pending = None;
                        if shell_spawn {
                            self.shell.as_mut().unwrap().state = "running";
                        } else {
                            self.shell = None;
                            self.showing = Some(name);
                        }
                    } else {
                        // Keep ownership until the stale child exits, without naming it
                        // as the current agent or allowing it to receive input.
                        session.interrupt()?;
                    }
                    self.session = Some(session);
                }
                Err(error) if current => {
                    self.pending = None;
                    if shell_spawn {
                        self.shell.as_mut().unwrap().state = "failed";
                    }
                    self.note = format!("terminal {name}: {error:#}");
                }
                Err(_) => {}
            }
        }
        if self.session.is_none()
            && self.spawning.is_none()
            && let Some(name) = &self.pending
        {
            let name = name.clone();
            let command = vec![self.corral.clone(), "attach".into(), name.clone()];
            let instance = self.agent_instance.clone();
            let checked_name = name.clone();
            let program = self.corral.clone();
            let size = size.unwrap_or(Size { rows: 24, cols: 80 });
            self.spawning = Some((
                self.generation,
                name,
                thread::spawn(move || {
                    if let Some(instance) = instance {
                        let status = crate::corral::Client { program }.json(
                            &["status", &checked_name],
                            std::time::Duration::from_secs(15),
                            &std::sync::atomic::AtomicBool::new(false),
                        )?;
                        anyhow::ensure!(
                            status["instance"].as_str() == Some(&instance),
                            "agent identity changed before attach"
                        );
                        anyhow::ensure!(
                            status["attached"].as_u64().unwrap_or(0) == 0,
                            "agent attached elsewhere"
                        );
                    }
                    Session::spawn(&command, None, size)
                }),
            ));
        }
        if self.session.is_none()
            && self.spawning.is_none()
            && !self.closing
            && let Some(shell) = self.shell.as_ref().filter(|s| s.state == "starting")
        {
            let command = vec![shell.program.clone(), "-i".into()];
            let cwd = std::path::PathBuf::from(&shell.cwd);
            let env = shell.env.clone();
            let size = size.unwrap_or(Size { rows: 24, cols: 80 });
            self.spawning = Some((
                self.generation,
                String::new(),
                thread::spawn(move || Session::spawn_shell(&command, &cwd, size, &env)),
            ));
        }
        Ok(())
    }
    pub fn start_shell(&mut self, shell: Shell) {
        self.cancel_pending();
        self.closing = false;
        self.shell = Some(shell);
        self.note = "Starting terminal…".into();
    }
    pub fn shell_live(&self) -> bool {
        self.shell
            .as_ref()
            .is_some_and(|s| matches!(s.state, "starting" | "running"))
    }
    pub fn state(&self) -> &'static str {
        if let Some(shell) = &self.shell {
            return shell.state;
        }
        if self.pending.is_some() || self.spawning.is_some() {
            "attaching"
        } else if self.session.as_ref().is_some_and(Session::running) && self.showing.is_some() {
            "running"
        } else {
            "disconnected"
        }
    }
}
impl Drop for Viewer {
    fn drop(&mut self) {
        // A spawn already in flight still owns its PTY; join and detach it on exit.
        if let Some((_, _, worker)) = self.spawning.take() {
            let _ = worker.join();
        }
    }
}
