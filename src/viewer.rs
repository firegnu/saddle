use crate::{pty::Session, terminal::Size};
use anyhow::Result;
use std::thread::{self, JoinHandle};

pub struct Viewer {
    pub session: Option<Session>,
    pub showing: Option<String>,
    pub note: String,
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
            note: "Select an agent on the left, then press Enter or click.".into(),
        }
    }
    pub fn select(&mut self, name: String) -> Result<()> {
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
                self.session = None;
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
            let current = generation == self.generation && self.pending.as_ref() == Some(&name);
            match worker.join().expect("attach worker panicked") {
                Ok(mut session) => {
                    if current {
                        self.pending = None;
                        self.showing = Some(name);
                    } else {
                        // Keep ownership until the stale child exits, without naming it
                        // as the current agent or allowing it to receive input.
                        session.interrupt()?;
                    }
                    self.session = Some(session);
                }
                Err(error) if current => {
                    self.pending = None;
                    self.note = format!("attach {name}: {error:#}");
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
            let size = size.unwrap_or(Size { rows: 24, cols: 80 });
            self.spawning = Some((
                self.generation,
                name,
                thread::spawn(move || Session::spawn(&command, None, size)),
            ));
        }
        Ok(())
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
