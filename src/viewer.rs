use crate::{pty::Session, terminal::Size};
use anyhow::Result;

pub struct Viewer {
    pub session: Option<Session>,
    pub showing: Option<String>,
    pub note: String,
    pending: Option<String>,
    corral: String,
}
impl Viewer {
    pub fn new(corral: String) -> Self {
        Self {
            session: None,
            showing: None,
            pending: None,
            corral,
            note: "在左边选一个 agent，按 Enter 或点击。".into(),
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
            self.pending = None;
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
    pub fn tick(&mut self, size: Size) -> Result<()> {
        if let Some(session) = &mut self.session {
            if session.poll_exit()? {
                self.session = None;
                self.note = format!(
                    "{} attach 已退出。在左边重新选择 agent。",
                    self.showing.take().unwrap_or_default()
                );
            } else {
                session.resize(size)?;
            }
        }
        if self.session.is_none()
            && let Some(name) = self.pending.take()
        {
            match Session::spawn(
                &[self.corral.clone(), "attach".into(), name.clone()],
                None,
                size,
            ) {
                Ok(session) => {
                    self.session = Some(session);
                    self.showing = Some(name);
                }
                Err(error) => {
                    self.note = format!("attach {name}: {error:#}");
                }
            }
        }
        Ok(())
    }
}
