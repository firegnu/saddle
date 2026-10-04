//! Search the host's existing agent, plugin and settings destinations.
use crate::{
    buttons::{self, Button},
    corral::Agent,
    launch::edit::Input,
    plugins::palette::Item,
    settings::{PAGES, Page},
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
pub const TITLE: &str = " Search ";

pub struct Search {
    input: Input,
    selected: Option<Outcome>,
    plugins: Vec<Item>,
    activation_changed: bool,
    /// Candidate targets as last drawn; the app rechecks a plugin before navigating.
    rows: Vec<(Rect, Outcome)>,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            input: Input::new(String::new()),
            selected: None,
            plugins: Vec::new(),
            activation_changed: false,
            rows: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Cancel,
    Open(String),
    Settings(Page),
    Plugin(Item),
}

/// The last component of the agent's working directory.
fn project(agent: &Agent) -> &str {
    let cwd = agent.cwd.as_deref().unwrap_or("").trim_end_matches('/');
    cwd.rsplit('/').next().unwrap_or(cwd)
}

impl Search {
    pub fn update_plugins(&mut self, plugins: Vec<Item>) {
        if let Some(Outcome::Plugin(selected)) = &self.selected
            && !plugins.contains(selected)
        {
            self.activation_changed = true;
        }
        self.plugins = plugins;
    }
    fn index(&self, entries: &[(Outcome, String)]) -> usize {
        entries
            .iter()
            .position(|(target, _)| match (&self.selected, target) {
                (Some(Outcome::Plugin(old)), Outcome::Plugin(new)) => old.id == new.id,
                (Some(old), new) => old == new,
                _ => false,
            })
            .unwrap_or(0)
    }

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
    fn entries(&self, agents: &[Agent]) -> Vec<(Outcome, String)> {
        let mut entries: Vec<_> = self
            .matches(agents)
            .into_iter()
            .map(|agent| {
                (
                    Outcome::Open(agent.name.clone()),
                    format!("Agent · {}  {}", agent.name, project(agent)),
                )
            })
            .collect();
        let query = self.input.text.trim().to_lowercase();
        entries.extend(
            self.plugins
                .iter()
                .filter(|item| {
                    item.title.to_lowercase().contains(&query)
                        || item.id.to_lowercase().contains(&query)
                })
                .map(|item| {
                    (
                        Outcome::Plugin(item.clone()),
                        format!("Plugin · {}", item.title),
                    )
                }),
        );
        entries.extend(PAGES.iter().filter_map(|&(page, label, _)| {
            let label = format!("Settings › {label}");
            label
                .to_lowercase()
                .contains(&query)
                .then_some((Outcome::Settings(page), label))
        }));
        entries
    }
    pub fn key(&mut self, key: KeyEvent, agents: &[Agent]) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.code == KeyCode::Esc || (ctrl && matches!(key.code, KeyCode::Char(']' | '5'))) {
            return Outcome::Cancel;
        }
        let entries = self.entries(agents);
        let index = self.index(&entries);
        let last = entries.len().saturating_sub(1);
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                let next = if key.code == KeyCode::Up {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(last)
                };
                self.selected = entries.get(next).map(|(target, _)| target.clone());
            }
            KeyCode::Enter => {
                if self.activation_changed {
                    return Outcome::Stay;
                }
                return entries
                    .get(index)
                    .map_or(Outcome::Stay, |(target, _)| target.clone());
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
            self.selected = None;
        }
    }
    pub fn scroll(&mut self, down: bool, agents: &[Agent]) {
        let code = if down { KeyCode::Down } else { KeyCode::Up };
        self.key(KeyEvent::new(code, KeyModifiers::NONE), agents);
    }
    pub fn click(&self, point: Position) -> Option<Outcome> {
        if self.activation_changed {
            return None;
        }
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
        let matches = self.entries(agents);
        let longest = matches
            .iter()
            .map(|(_, label)| label.width())
            .max()
            .unwrap_or(0);
        let width = (longest as u16 + 32).clamp(44, 90);
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
            let block = t.block(" Project, agent, plugin or settings ", false);
            let inside = block.inner(input);
            frame.render_widget(block, input);
            inside
        } else {
            input
        };
        self.input
            .draw(frame, inside, true, "Type to find an entry", t);
        body.y += input.height;
        body.height -= input.height;
        // Keep one row for the selected plugin's explanation above Cancel.
        let list = Rect {
            height: body.height.saturating_sub(1),
            ..body
        };
        self.rows.clear();
        let selected = self.index(&matches);
        self.selected = matches.get(selected).map(|(target, _)| target.clone());
        self.activation_changed = false;
        if body.height > 0
            && let Some(Outcome::Plugin(item)) = &self.selected
        {
            frame.render_widget(
                Paragraph::new(crate::ui::clip(
                    &item.explanation().replace('\n', " · "),
                    body.width as usize,
                ))
                .style(Style::default().fg(t.muted)),
                Rect::new(body.x, body.bottom() - 1, body.width, 1),
            );
        }
        if matches.is_empty() {
            let query = self.input.text.trim();
            let room = usize::from(list.width).saturating_sub("No entries match “”.".width());
            let empty = format!("No entries match “{}”.", crate::ui::clip(query, room));
            frame.render_widget(
                Paragraph::new(empty).style(Style::default().fg(t.muted)),
                list,
            );
            return hits;
        }
        let top = selected.saturating_sub(usize::from(list.height).saturating_sub(1));
        for (offset, (index, (target, label))) in matches
            .iter()
            .enumerate()
            .skip(top)
            .take(usize::from(list.height))
            .enumerate()
        {
            let row = Rect::new(list.x, list.y + offset as u16, list.width, 1);
            let tag = match target {
                Outcome::Open(name) if open(name) => "Open".to_owned(),
                Outcome::Open(_) => "Attach".to_owned(),
                Outcome::Plugin(item) => {
                    format!("{} · {}", item.status(), item.action().unwrap_or("Manage"))
                }
                _ => "Open".to_owned(),
            };
            let room = usize::from(row.width).saturating_sub(tag.width() + 1);
            let name = crate::ui::clip(label, room);
            let text_width = name.width();
            let chosen = index == selected;
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
                    Span::raw(" ".repeat(room.saturating_sub(text_width))),
                    Span::styled(tag, Style::default().fg(t.connected)),
                ]))
                .style(if chosen {
                    Style::default().bg(t.selected)
                } else {
                    Style::default()
                }),
                row,
            );
            self.rows.push((row, target.clone()));
        }
        hits
    }
}
