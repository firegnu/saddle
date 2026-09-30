//! In-saddle task notifications: a short prompt when a task newly awaits release, while Drover's
//! public preference says its own system notifications are off. A prompt only shows; the task
//! stays in Attention until it is really released.
use crate::{
    attention::{Target, project_name},
    drover::{Preference, Snapshot, Task},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use unicode_width::UnicodeWidthStr;

/// How long a prompt stays up.
pub const SHOWN: Duration = Duration::from_secs(5);

/// Drover's opaque public notification identity. Git references never enter this key.
pub type Identity = String;
pub fn identity(_project: &str, task: &Task) -> Option<Identity> {
    task.notification_key.clone().filter(|key| !key.is_empty())
}

pub struct Toast {
    /// The tasks it announces, with their `project · id` labels.
    pub targets: Vec<(Target, String)>,
    since: Instant,
}
impl Toast {
    pub fn text(&self) -> String {
        match &self.targets[..] {
            [(_, label)] => format!("{label} ready for review"),
            targets => format!("{} tasks ready for review", targets.len()),
        }
    }
}

/// Decides when to prompt. Every start, first In saddle reading and preference change starts
/// over with a baseline: whatever each project already awaits is taken as known, not prompted.
#[derive(Default)]
pub struct Notifier {
    /// Drover's preference as last read; `None` while unknown or unreadable.
    preference: Option<Preference>,
    /// The revision the baselines belong to, and the projects that have one.
    revision: Option<u64>,
    based: HashSet<String>,
    /// Runs already prompted or baselined in this saddle run; only ever grows.
    known: HashSet<Identity>,
    toast: Option<Toast>,
}
impl Notifier {
    /// A new reading of Drover's preference; `None` when it could not be read.
    pub fn preference(&mut self, preference: Option<Preference>) {
        if let Some(p) = preference
            && self.revision != Some(p.revision)
        {
            self.revision = Some(p.revision);
            self.based.clear();
        }
        self.preference = preference;
    }
    /// Projects no longer registered lose their baseline.
    pub fn registry(&mut self, projects: &[String]) {
        self.based.retain(|p| projects.contains(p));
    }
    /// A project's successful snapshot. Prompts only when In saddle is confirmed, the project
    /// already has a baseline and the awaiting run is new.
    pub fn snapshot(&mut self, project: &str, snapshot: &Snapshot, now: Instant) {
        if self.preference.is_none_or(|p| p.system_enabled) {
            return;
        }
        let first = self.based.insert(project.to_owned());
        let Some(task) = &snapshot.awaiting else {
            return;
        };
        let Some(identity) = identity(project, task) else {
            return;
        };
        if !self.known.insert(identity) || first {
            return;
        }
        let entry = (
            Target::task(project, task),
            format!(
                "{} · {}",
                project_name(project),
                task.id.as_deref().unwrap_or_default()
            ),
        );
        match self.toast.as_mut().filter(|t| now < t.since + SHOWN) {
            Some(toast) => {
                toast.targets.push(entry);
                toast.since = now;
            }
            None => {
                self.toast = Some(Toast {
                    targets: vec![entry],
                    since: now,
                })
            }
        }
    }
    pub fn visible(&self, now: Instant) -> Option<&Toast> {
        self.toast.as_ref().filter(|t| now < t.since + SHOWN)
    }
    /// Closes the prompt, returning what it announced.
    pub fn dismiss(&mut self) -> Option<Toast> {
        self.toast.take()
    }
}

/// Draws the prompt at the bottom right of `area`; returns its whole area and its close mark.
pub fn draw(t: &Theme, frame: &mut Frame, area: Rect, toast: &Toast) -> Option<(Rect, Rect)> {
    let text = toast.text();
    let close = " × ";
    let width = (text.width() + close.width() + 5) as u16;
    let width = width.min(area.width.saturating_sub(2));
    if width < 12 || area.height < 5 {
        return None;
    }
    let rect = Rect::new(area.right() - 1 - width, area.bottom() - 1 - 3, width, 3);
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::bordered()
            .border_style(Style::default().fg(t.agent_blocked))
            .style(t.base().bg(t.overlay)),
        rect,
    );
    let inner = crate::ui::inner(rect);
    let room = usize::from(inner.width).saturating_sub(close.width() + 3);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" → ", Style::default().fg(t.agent_blocked)),
            Span::styled(
                crate::ui::clip(&text, room),
                Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
            ),
        ])),
        inner,
    );
    let mark = Rect::new(
        inner.right() - close.width() as u16,
        inner.y,
        close.width() as u16,
        1,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(close, Style::default().fg(t.muted))),
        mark,
    );
    Some((rect, mark))
}
