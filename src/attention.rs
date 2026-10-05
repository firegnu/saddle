//! Agent Attention. Opening an item never changes its business state.
use crate::{
    agents::{Panel, Status},
    buttons::{self, Button},
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
use unicode_width::UnicodeWidthStr;

pub const TITLE: &str = " Attention ";
const ROWS: usize = 14;

/// What an entry opens; also its identity for selection.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Agent(String),
    /// A failed source with nothing to open.
    Source(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Waiting,
    Error,
    ReadFailed,
    Reply,
}
#[derive(Clone, Debug)]
pub struct Item {
    pub target: Target,
    pub kind: Kind,
    pub label: String,
    /// Public detail shown after the reason: an error text or reply notice.
    pub note: String,
}
impl Item {
    pub fn needs(&self) -> bool {
        self.kind != Kind::Reply
    }
    fn reason(&self) -> &'static str {
        match self.kind {
            Kind::Waiting => "Waiting for input",
            Kind::Error => "Error",
            Kind::ReadFailed => "Read failed",
            Kind::Reply => "New reply",
        }
    }
    fn look(&self, t: &Theme) -> (&'static str, Color) {
        match self.kind {
            Kind::Waiting => ("?", t.agent_blocked),
            Kind::Error | Kind::ReadFailed => ("!", t.agent_error),
            Kind::Reply => ("•", t.unread),
        }
    }
}

/// Source state for agent Attention.
#[derive(Default)]
pub struct Board {
    pub agents_loaded: bool,
    pub corral_error: Option<String>,
}
impl Board {
    /// Some source has not answered yet.
    pub fn loading(&self) -> bool {
        !self.agents_loaded
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
        let sources = [("corral", self.corral_error.as_ref())];
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
            Style::default().fg(if count > 0 { color } else { t.agents_dim }),
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
                // Selection is the marker and row fill; a failed source has nothing to open,
                // so its name stays muted and plain while its reason stays readable.
                let chosen = selected == Some(i);
                let label = if matches!(item.target, Target::Source(_)) {
                    Style::default().fg(t.muted)
                } else if chosen {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push((
                    Line::from(vec![
                        Span::raw(if chosen { "›" } else { " " }),
                        Span::styled(format!("{mark} "), Style::default().fg(color)),
                        Span::styled(
                            crate::ui::pad(&crate::ui::clip(&item.label, label_width), label_width),
                            label,
                        ),
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
        let (body, hits) = buttons::draw_compact(
            t,
            frame,
            crate::ui::inner(area),
            &[Button::new("Cancel Esc", KeyCode::Esc, true)],
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
