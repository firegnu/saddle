//! Attention: what needs a person across agents and every registered project, from public
//! snapshots only. Nothing here answers, releases or otherwise acts on a target.
use crate::{
    agents::{Panel, Status},
    buttons::{self, Button},
    drover::{Snapshot, Survey, Task},
    theme::Theme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

pub const TITLE: &str = " Attention ";
const ROWS: usize = 14;

/// What an entry opens; also its identity for selection and Mark seen.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Agent(String),
    /// A task by project and public identity: its id, or its text when unnumbered.
    Task {
        project: String,
        id: Option<String>,
        title: String,
        body: String,
    },
    /// A project whose read failed; opening shows its Tasks.
    Project(String),
    /// A failed source with nothing to open.
    Source(String),
}
impl Target {
    fn task(project: &str, task: &Task) -> Self {
        Self::Task {
            project: project.into(),
            id: task.id.clone(),
            title: task.title.clone(),
            body: task.body.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Waiting,
    Error,
    Awaiting,
    Failed,
    ReadFailed,
    Reply,
}
#[derive(Clone, Debug)]
pub struct Item {
    pub target: Target,
    pub kind: Kind,
    pub label: String,
    /// Public detail shown after the reason: an error text or a task title.
    pub note: String,
}
impl Item {
    pub fn needs(&self) -> bool {
        self.kind != Kind::Reply
    }
    /// Only a historical failure can be marked seen; current states leave on their own.
    pub fn seeable(&self) -> bool {
        self.kind == Kind::Failed
    }
    fn reason(&self) -> &'static str {
        match self.kind {
            Kind::Waiting => "Waiting for input",
            Kind::Error => "Error",
            Kind::Awaiting => "Awaiting release",
            Kind::Failed => "Failed",
            Kind::ReadFailed => "Read failed",
            Kind::Reply => "New reply",
        }
    }
    fn look(&self, t: &Theme) -> (&'static str, Color) {
        match self.kind {
            Kind::Waiting => ("?", t.agent_blocked),
            Kind::Error | Kind::Failed | Kind::ReadFailed => ("!", t.agent_error),
            Kind::Awaiting => ("→", t.agent_blocked),
            Kind::Reply => ("•", t.unread),
        }
    }
}

/// A project's latest public snapshot, or why it could not be read.
pub type ProjectState = Result<Box<Snapshot>, String>;
/// Source state for Attention. Seen marks last for this run only.
#[derive(Default)]
pub struct Board {
    /// Registered projects and their latest snapshot: `None` until the first read returns.
    pub projects: Vec<(String, Option<ProjectState>)>,
    pub registry: Option<Result<(), String>>,
    pub agents_loaded: bool,
    pub corral_error: Option<String>,
    pub seen: HashSet<Target>,
}
impl Board {
    pub fn absorb(&mut self, update: Survey) {
        match update {
            Survey::Projects(Ok(list)) => {
                let mut old = std::mem::take(&mut self.projects);
                self.projects = list
                    .into_iter()
                    .map(|p| {
                        let state = old
                            .iter_mut()
                            .find(|(o, _)| *o == p)
                            .and_then(|(_, s)| s.take());
                        (p, state)
                    })
                    .collect();
                self.registry = Some(Ok(()));
            }
            Survey::Projects(Err(error)) => {
                // Without the registry no project is read; old results would be stale.
                self.projects.clear();
                self.registry = Some(Err(error));
            }
            Survey::Snapshot(project, result) => {
                if let Some((_, state)) = self.projects.iter_mut().find(|(p, _)| *p == project) {
                    *state = Some(result);
                }
            }
        }
    }
    /// Some source has not answered yet.
    pub fn loading(&self) -> bool {
        !self.agents_loaded
            || self.registry.is_none()
            || self.projects.iter().any(|(_, s)| s.is_none())
    }
    /// Needs attention first, then new replies; one row per agent, its need before its reply.
    pub fn items(&self, panel: &Panel, now: f64) -> Vec<Item> {
        let mut items = Vec::new();
        for (wanted, kind) in [
            (Status::Waiting, Kind::Waiting),
            (Status::Error, Kind::Error),
        ] {
            for a in panel
                .agents
                .iter()
                .filter(|a| panel.status(a, now) == wanted)
            {
                let incompatible = a
                    .incompatible
                    .then(|| format!("incompatible protocol {}", a.proto.unwrap_or(0)));
                items.push(Item {
                    target: Target::Agent(a.name.clone()),
                    kind,
                    label: a.name.clone(),
                    note: a.error.clone().or(incompatible).unwrap_or_default(),
                });
            }
        }
        for (project, state) in &self.projects {
            let name = project_name(project);
            match state {
                Some(Ok(s)) => {
                    let failed = s
                        .history
                        .iter()
                        .rev()
                        .filter(|t| t.status.as_deref() == Some("failed"))
                        .map(|t| (Kind::Failed, t));
                    for (kind, task) in s.awaiting.iter().map(|t| (Kind::Awaiting, t)).chain(failed)
                    {
                        let target = Target::task(project, task);
                        if kind == Kind::Failed && self.seen.contains(&target) {
                            continue;
                        }
                        items.push(Item {
                            target,
                            kind,
                            label: format!("{name} · {}", task.id.as_deref().unwrap_or("—")),
                            note: task.title.clone(),
                        });
                    }
                }
                Some(Err(error)) => items.push(Item {
                    target: Target::Project(project.clone()),
                    kind: Kind::ReadFailed,
                    label: name.to_owned(),
                    note: error.clone(),
                }),
                None => {}
            }
        }
        let sources = [
            ("corral", self.corral_error.as_ref()),
            (
                "~/.drover/projects",
                self.registry.as_ref().and_then(|r| r.as_ref().err()),
            ),
        ];
        for (source, error) in sources {
            if let Some(error) = error {
                items.push(Item {
                    target: Target::Source(source.into()),
                    kind: Kind::ReadFailed,
                    label: source.into(),
                    note: error.clone(),
                });
            }
        }
        let mut unread: Vec<_> = panel
            .agents
            .iter()
            .filter(|a| panel.unread.contains(&a.name))
            .collect();
        unread.sort_by(|a, b| a.name.cmp(&b.name));
        for a in unread {
            let target = Target::Agent(a.name.clone());
            if let Some(item) = items.iter_mut().find(|i| i.target == target) {
                item.note = format!("new reply{}", sep(&item.note));
                continue;
            }
            items.push(Item {
                target,
                kind: Kind::Reply,
                label: a.name.clone(),
                note: String::new(),
            });
        }
        items
    }
}
fn sep(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!(" · {text}")
    }
}
fn project_name(path: &str) -> &str {
    let path = path.trim_end_matches('/');
    path.rsplit('/').next().unwrap_or(path)
}

/// The fixed Agents entry: the item count, faint at zero, with a mark while still loading.
pub fn entry(t: &Theme, frame: &mut Frame, area: Rect, items: &[Item], loading: bool) -> Rect {
    if area.is_empty() {
        return Rect::default();
    }
    let count = items.len();
    let color = if items.iter().any(Item::needs) {
        t.agents_yellow
    } else if count > 0 {
        t.unread
    } else {
        t.agents_dimmer
    };
    let mut spans = vec![
        Span::styled(
            "Attention",
            Style::default().fg(if count > 0 {
                t.agents_text
            } else {
                t.agents_dim
            }),
        ),
        Span::styled(" · ", Style::default().fg(t.agents_dim)),
    ];
    if loading && count == 0 {
        spans.push(Span::styled("…", Style::default().fg(t.agents_dim)));
    } else {
        spans.push(Span::styled(
            count.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        if loading {
            spans.push(Span::styled(" loading…", Style::default().fg(t.agents_dim)));
        }
    }
    let width = spans.iter().map(|s| s.content.width()).sum::<usize>() as u16;
    let area = Rect {
        width: width.min(area.width),
        ..area
    };
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    area
}

#[derive(Default)]
pub struct Popup {
    /// The selected entry by identity, and its last position for when it leaves the list.
    selected: Option<Target>,
    index: usize,
    /// Entry rows as last drawn, so a click opens what it was shown on.
    rows: Vec<(Rect, Target)>,
}
pub enum Outcome {
    Stay,
    Cancel,
    Open(Target),
    Seen(Target),
}
impl Popup {
    fn current(&mut self, items: &[Item]) -> Option<usize> {
        if items.is_empty() {
            return None;
        }
        let index = self
            .selected
            .as_ref()
            .and_then(|s| items.iter().position(|i| &i.target == s))
            .unwrap_or(self.index.min(items.len() - 1));
        self.index = index;
        self.selected = Some(items[index].target.clone());
        Some(index)
    }
    fn step(&mut self, items: &[Item], down: bool) {
        if let Some(index) = self.current(items) {
            self.index = if down {
                (index + 1).min(items.len() - 1)
            } else {
                index.saturating_sub(1)
            };
            self.selected = Some(items[self.index].target.clone());
        }
    }
    pub fn key(&mut self, key: KeyEvent, items: &[Item]) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.code == KeyCode::Esc || (ctrl && matches!(key.code, KeyCode::Char(']' | '5'))) {
            return Outcome::Cancel;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.step(items, false),
            KeyCode::Down | KeyCode::Char('j') => self.step(items, true),
            KeyCode::Enter => {
                if let Some(index) = self.current(items)
                    && !matches!(items[index].target, Target::Source(_))
                {
                    return Outcome::Open(items[index].target.clone());
                }
            }
            KeyCode::Char('m') => {
                if let Some(index) = self.current(items)
                    && items[index].seeable()
                {
                    return Outcome::Seen(items[index].target.clone());
                }
            }
            _ => {}
        }
        Outcome::Stay
    }
    pub fn scroll(&mut self, down: bool, items: &[Item]) {
        self.step(items, down);
    }
    pub fn click(&self, point: Position) -> Option<Target> {
        self.rows
            .iter()
            .find(|(row, _)| row.contains(point))
            .map(|(_, target)| target.clone())
            .filter(|target| !matches!(target, Target::Source(_)))
    }
    /// Draws the popup centred on the screen; returns its buttons.
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        items: &[Item],
        loading: bool,
    ) -> Vec<buttons::Hit> {
        let selected = self.current(items);
        // Group headings, entries, a blank line between groups and a loading note.
        let mut lines: Vec<(Line<'static>, Option<usize>)> = Vec::new();
        let label_width = items
            .iter()
            .map(|i| i.label.width())
            .max()
            .unwrap_or(0)
            .min(28);
        let heading = |text: &'static str| {
            Line::from(Span::styled(
                text,
                Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
            ))
        };
        for (needs, title) in [(true, "Needs attention"), (false, "New replies")] {
            let group: Vec<_> = (0..items.len())
                .filter(|&i| items[i].needs() == needs)
                .collect();
            if group.is_empty() {
                continue;
            }
            if !lines.is_empty() {
                lines.push((Line::default(), None));
            }
            lines.push((heading(title), None));
            for i in group {
                let item = &items[i];
                let (mark, color) = item.look(t);
                lines.push((
                    Line::from(vec![
                        Span::styled(format!(" {mark} "), Style::default().fg(color)),
                        Span::raw(crate::ui::pad(
                            &crate::ui::clip(&item.label, label_width),
                            label_width,
                        )),
                        Span::raw("  "),
                        Span::styled(item.reason(), Style::default().fg(color)),
                        Span::styled(sep(&item.note), Style::default().fg(t.muted)),
                    ]),
                    Some(i),
                ));
            }
        }
        if items.is_empty() && !loading {
            lines.push((
                Line::from(Span::styled(
                    "Nothing needs attention.",
                    Style::default().fg(t.muted),
                )),
                None,
            ));
        }
        if loading {
            lines.push((
                Line::from(Span::styled("Loading…", Style::default().fg(t.muted))),
                None,
            ));
        }
        let longest = lines.iter().map(|(l, _)| l.width()).max().unwrap_or(0);
        let width = (longest as u16 + 4).clamp(44, 76);
        let height = lines.len().clamp(1, ROWS) as u16 + 5;
        let area = crate::theme::centered(frame.area(), width, height);
        frame.render_widget(Clear, area);
        frame.render_widget(t.block(TITLE, true).style(t.base().bg(t.overlay)), area);
        let seeable = selected.is_some_and(|i| items[i].seeable());
        let (body, hits) = buttons::draw_compact(
            t,
            frame,
            crate::ui::inner(area),
            &[
                Button::new("Mark seen m", KeyCode::Char('m'), seeable),
                Button::new("Cancel Esc", KeyCode::Esc, true),
            ],
        );
        // Keep one blank row above the buttons.
        let list = Rect {
            height: body.height.saturating_sub(1),
            ..body
        };
        self.rows.clear();
        let at = selected.and_then(|s| lines.iter().position(|(_, i)| *i == Some(s)));
        let top = at.map_or(0, |at| {
            at.saturating_sub(usize::from(list.height).saturating_sub(1))
        });
        for (offset, (line, index)) in lines
            .into_iter()
            .skip(top)
            .take(usize::from(list.height))
            .enumerate()
        {
            let row = Rect::new(list.x, list.y + offset as u16, list.width, 1);
            let style = if index.is_some() && index == selected {
                Style::default().bg(t.selected)
            } else {
                Style::default()
            };
            let line = Line::from(crate::ui::clip_spans(line.spans, usize::from(row.width)));
            frame.render_widget(Paragraph::new(line).style(style), row);
            if let Some(i) = index {
                self.rows.push((row, items[i].target.clone()));
            }
        }
        hits
    }
}
