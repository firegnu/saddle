//! Direct plugin entries, separate from lifecycle management and plugin rendering.
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{Frame, layout::Rect, style::Style, widgets::Paragraph};
use saddle_plugin_protocol::Placement;
use unicode_width::UnicodeWidthChar;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub plugin: String,
    pub action: String,
    pub title: String,
    pub placement: Placement,
    pub state: String,
}
#[derive(Default)]
pub struct Bar {
    pub items: Vec<Entry>,
    pub focused: bool,
    selected: usize,
    top: usize,
    hits: Vec<(Rect, Entry)>,
    pressed: Option<Entry>,
}
impl Bar {
    pub fn update(&mut self, items: Vec<Entry>) {
        if self.items != items {
            let selected = self
                .items
                .get(self.selected)
                .map(|e| (&e.plugin, &e.action));
            self.selected = selected
                .and_then(|id| items.iter().position(|e| (&e.plugin, &e.action) == id))
                .unwrap_or(0);
            self.items = items;
            self.pressed = None;
        }
        if self.items.is_empty() {
            self.focused = false;
        }
    }
    fn rows(&self, width: u16) -> Vec<(usize, String)> {
        if width == 0 {
            return Vec::new();
        }
        let mut rows = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let mut text = if item.state == "Running" {
                item.title.clone()
            } else {
                format!("{} · {}", item.title, item.state)
            };
            if i == 0 && !self.focused {
                text.push_str("  F6");
            }
            let mut line = String::new();
            let mut columns = 0;
            for c in text.chars() {
                let w = c.width().unwrap_or(0) as u16;
                if columns + w > width && !line.is_empty() {
                    rows.push((i, std::mem::take(&mut line)));
                    columns = 0;
                }
                line.push(c);
                columns += w;
            }
            if !line.is_empty() {
                rows.push((i, line));
            }
        }
        rows
    }
    pub fn height(&self, available: u16, width: u16) -> u16 {
        (self.rows(width).len().min(3) as u16).min(available.saturating_sub(5))
    }
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        self.hits.clear();
        if area.is_empty() {
            return;
        }
        let rows = self.rows(area.width);
        if self.focused
            && let Some(first) = rows.iter().position(|(i, _)| *i == self.selected)
        {
            self.top = self
                .top
                .min(first)
                .max((first + 1).saturating_sub(area.height as usize));
        }
        self.top = self
            .top
            .min(rows.len().saturating_sub(area.height as usize));
        for (row, (index, text)) in rows
            .iter()
            .skip(self.top)
            .take(area.height as usize)
            .enumerate()
        {
            let item = &self.items[*index];
            let rect = Rect::new(area.x, area.y + row as u16, area.width, 1);
            frame.render_widget(
                Paragraph::new(text.as_str()).style(Style::default().fg(
                    if self.focused && *index == self.selected {
                        theme.focus
                    } else {
                        theme.agents_text
                    },
                )),
                rect,
            );
            self.hits.push((rect, item.clone()));
        }
    }
    /// Returns whether the event belongs to this bar, and an explicitly activated entry.
    pub fn event(&mut self, event: &Event, agents_focused: bool) -> (bool, Option<Entry>) {
        if !agents_focused {
            self.focused = false;
        }
        match event {
            Event::Key(k) if agents_focused && k.kind != KeyEventKind::Release => {
                self.pressed = None;
                if k.code == KeyCode::F(6) && k.modifiers.is_empty() && !self.items.is_empty() {
                    self.focused = !self.focused;
                    return (true, None);
                }
                if !self.focused {
                    return (false, None);
                }
                match k.code {
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down => {
                        self.selected = (self.selected + 1).min(self.items.len().saturating_sub(1))
                    }
                    KeyCode::Enter if k.modifiers.is_empty() => {
                        return (
                            true,
                            self.items
                                .get(self.selected)
                                .filter(|e| e.state == "Running")
                                .cloned(),
                        );
                    }
                    KeyCode::Esc => self.focused = false,
                    _ => {
                        self.focused = false;
                        return (false, None);
                    }
                }
                (true, None)
            }
            Event::Mouse(m) => {
                let hit = self
                    .hits
                    .iter()
                    .find(|(r, _)| r.contains((m.column, m.row).into()))
                    .map(|(_, e)| e.clone());
                let captured = self.pressed.is_some();
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) => self.pressed = hit.clone(),
                    MouseEventKind::Up(MouseButton::Left) => {
                        let old = self.pressed.take();
                        let activated =
                            old.filter(|e| Some(e) == hit.as_ref() && e.state == "Running");
                        return (captured || hit.is_some(), activated);
                    }
                    MouseEventKind::ScrollUp if hit.is_some() => {
                        self.pressed = None;
                        self.focused = false;
                        self.top = self.top.saturating_sub(1);
                    }
                    MouseEventKind::ScrollDown if hit.is_some() => {
                        self.pressed = None;
                        self.focused = false;
                        self.top += 1;
                    }
                    _ => {}
                }
                (hit.is_some() || captured, None)
            }
            _ => (false, None),
        }
    }
    pub fn cancel(&mut self) {
        self.pressed = None;
        self.focused = false;
    }
}
