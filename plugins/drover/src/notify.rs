//! In-saddle task notifications: a short prompt when a task newly awaits release, while Drover's
//! public preference says its own system notifications are off. A prompt only shows; the task
//! stays in Attention until it is really released.
use crate::{
    attention::{Target, project_name},
    drover::{Preference, Snapshot, Task},
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

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
