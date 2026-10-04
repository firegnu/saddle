//! Quick switch: filter the public agent list by project or name, then open the chosen agent.
use crate::{
    buttons::{self, Button},
    corral::Agent,
    launch::edit::Input,
    theme::Theme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

const ROWS: usize = 12;
pub const TITLE: &str = " Search agents ";

pub struct Search {
    input: Input,
    pub selected: usize,
    /// Candidate rows as last drawn, so a click opens the name it was shown on.
    rows: Vec<(Rect, String)>,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            input: Input::new(String::new()),
            selected: 0,
            rows: Vec::new(),
        }
    }
}
pub enum Outcome {
    Stay,
    Cancel,
    Open(String),
}

/// The last component of the agent's working directory.
fn project(agent: &Agent) -> &str {
    let cwd = agent.cwd.as_deref().unwrap_or("").trim_end_matches('/');
    cwd.rsplit('/').next().unwrap_or(cwd)
}

impl Search {
    /// Agents whose name or project contains the query, ignoring case, by name.
    pub fn matches<'a>(&self, agents: &'a [Agent]) -> Vec<&'a Agent> {
        let query = self.input.text.trim().to_lowercase();
        let mut list: Vec<_> = agents
            .iter()
            .filter(|a| {
                a.name.to_lowercase().contains(&query) || project(a).to_lowercase().contains(&query)
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }
    pub fn key(&mut self, key: KeyEvent, agents: &[Agent]) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.code == KeyCode::Esc || (ctrl && matches!(key.code, KeyCode::Char(']' | '5'))) {
            return Outcome::Cancel;
        }
        let last = self.matches(agents).len().saturating_sub(1);
        match key.code {
            KeyCode::Up => self.selected = self.selected.min(last).saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Enter => {
                return self
                    .matches(agents)
                    .get(self.selected.min(last))
                    .map_or(Outcome::Stay, |a| Outcome::Open(a.name.clone()));
            }
            KeyCode::Char('u') if ctrl => self.edit(|input| input.clear()),
            _ if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) => {}
            code => self.edit(|input| input.key(code, false)),
        }
        Outcome::Stay
    }
    pub fn paste(&mut self, text: &str) {
        self.edit(|input| input.insert(text, false));
    }
    fn edit(&mut self, change: impl FnOnce(&mut Input)) {
        let old = self.input.text.clone();
        change(&mut self.input);
        if self.input.text != old {
            self.selected = 0;
        }
    }
    pub fn scroll(&mut self, down: bool, agents: &[Agent]) {
        let code = if down { KeyCode::Down } else { KeyCode::Up };
        self.key(KeyEvent::new(code, KeyModifiers::NONE), agents);
    }
    pub fn click(&self, point: Position) -> Option<String> {
        self.rows
            .iter()
            .find(|(row, _)| row.contains(point))
            .map(|(_, name)| name.clone())
    }
    /// Draws the popup centred on the screen; returns its Cancel button.
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        agents: &[Agent],
        open: impl Fn(&str) -> bool,
    ) -> Vec<buttons::Hit> {
        let matches = self.matches(agents);
        let longest = matches
            .iter()
            .map(|a| a.name.width() + project(a).width())
            .max()
            .unwrap_or(0);
        let width = (longest as u16 + 14).clamp(44, 72);
        let height = matches.len().clamp(1, ROWS) as u16 + 7;
        let area = crate::theme::centered(frame.area(), width, height);
        frame.render_widget(Clear, area);
        frame.render_widget(t.block(TITLE, true).style(t.base().bg(t.overlay)), area);
        let (mut body, hits) = buttons::draw_compact(
            t,
            frame,
            crate::ui::inner(area),
            &[Button::new("Cancel Esc", KeyCode::Esc, true)],
        );
        // A bordered input when there is room, otherwise a single line. The popup already has
        // the thick focus frame, so the input keeps a thin one and shows focus by its cursor.
        let boxed = body.height >= 5;
        let input = Rect {
            height: if boxed { 3 } else { 1 }.min(body.height),
            ..body
        };
        let inside = if boxed {
            let block = t.block(" Project or name ", false);
            let inside = block.inner(input);
            frame.render_widget(block, input);
            inside
        } else {
            input
        };
        self.input
            .draw(frame, inside, true, "Type a project or agent name", t);
        body.y += input.height;
        body.height -= input.height;
        // Keep one blank row above Cancel.
        let list = Rect {
            height: body.height.saturating_sub(1),
            ..body
        };
        self.rows.clear();
        if matches.is_empty() {
            // An empty list and a query that matches nothing are different situations.
            let empty = if agents.is_empty() {
                "No agents to search.".to_owned()
            } else {
                let query = self.input.text.trim();
                let room = usize::from(list.width).saturating_sub("No agents match “”.".width());
                format!("No agents match “{}”.", crate::ui::clip(query, room))
            };
            frame.render_widget(
                Paragraph::new(empty).style(Style::default().fg(t.muted)),
                list,
            );
            return hits;
        }
        self.selected = self.selected.min(matches.len() - 1);
        let top = self
            .selected
            .saturating_sub(usize::from(list.height).saturating_sub(1));
        for (offset, (index, agent)) in matches
            .iter()
            .enumerate()
            .skip(top)
            .take(usize::from(list.height))
            .enumerate()
        {
            let row = Rect::new(list.x, list.y + offset as u16, list.width, 1);
            let tag = if open(&agent.name) { "Open " } else { "" };
            let room = usize::from(row.width).saturating_sub(tag.width() + 1);
            let name = crate::ui::clip(&agent.name, room);
            let project = crate::ui::clip(project(agent), room.saturating_sub(name.width() + 2));
            let text = format!(" {name}  {project}");
            let chosen = index == self.selected;
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(if chosen { "›" } else { " " }),
                    Span::styled(
                        name,
                        if chosen {
                            Style::default().add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::raw("  "),
                    Span::styled(project, Style::default().fg(t.muted)),
                    Span::raw(" ".repeat((room + 1).saturating_sub(text.width()))),
                    Span::styled(tag, Style::default().fg(t.connected)),
                ]))
                .style(if chosen {
                    Style::default().bg(t.selected)
                } else {
                    Style::default()
                }),
                row,
            );
            self.rows.push((row, agent.name.clone()));
        }
        hits
    }
}
