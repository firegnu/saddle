//! Host-owned plugin launcher. Selecting never changes plugin lifecycle.
use crate::{launch::edit::Input, theme::Theme};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub state: String,
    pub note: String,
    pub has_view: bool,
    pub opened: bool,
    pub pid: Option<u32>,
    /// Built-in plugins have no process or view; Enter opens their management row.
    pub builtin: bool,
}
impl Item {
    pub fn action(&self) -> Option<&'static str> {
        (self.state == "Running" && self.has_view).then_some(if self.opened {
            "Switch"
        } else {
            "Open"
        })
    }
    pub fn status(&self) -> &str {
        if self.state == "Running" {
            if self.opened {
                "View open"
            } else {
                "Background"
            }
        } else {
            &self.state
        }
    }
    pub fn explanation(&self) -> String {
        if self.builtin {
            return "Built-in plugin · Manage settings and resources.".into();
        }
        let guidance: String = match self.state.as_str() {
            "Running" if !self.has_view => "Background-only plugin; no view to open.".into(),
            "Running" => String::new(),
            "Starting" => "Waiting for plugin startup.".into(),
            "Stopping" => "Waiting for the plugin process to exit.".into(),
            "Restarting" => "Waiting for restart; another instance will not be started.".into(),
            "Disabled" => "Enable this plugin in Manage plugins to use it.".into(),
            "Unresponsive" => {
                "Plugin is not responding. Open Manage plugins to restart or disable.".into()
            }
            "Failed" => "Open Manage plugins to restart or disable.".into(),
            _ => "Plugin unavailable. Open Manage plugins for details.".into(),
        };
        if !self.note.is_empty() && self.state != "Running" {
            format!("{guidance}\n{}", self.note)
        } else {
            guidance
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    Manage,
    Open(Item),
}
#[derive(Clone, PartialEq, Eq)]
enum Target {
    Row(String),
    Open(Item),
    Manage,
    ManageItem(String),
    Close,
}
pub struct Palette {
    pub(crate) placement: Option<(u64, crate::terminals::Place)>,
    input: Input,
    items: Vec<Item>,
    selected: Option<String>,
    top: usize,
    focus: usize, // search/list, Manage, Close
    field: Rect,
    list: Rect,
    hits: Vec<(Rect, Target)>,
    pressed: Option<Target>,
    activation_changed: bool,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            placement: None,
            input: Input::new(String::new()),
            items: vec![],
            selected: None,
            top: 0,
            focus: 0,
            field: Rect::default(),
            list: Rect::default(),
            hits: vec![],
            pressed: None,
            activation_changed: false,
        }
    }
}
impl Palette {
    pub fn for_placement(pane: u64, place: crate::terminals::Place) -> Self {
        Self {
            placement: Some((pane, place)),
            ..Self::default()
        }
    }
    fn action(&self, item: &Item) -> Option<&'static str> {
        if item.builtin {
            return Some("Manage");
        }
        item.action().map(|action| {
            if self.placement.is_some() && item.opened {
                "Move"
            } else {
                action
            }
        })
    }
    pub fn update(&mut self, items: Vec<Item>) {
        if self.items != items {
            self.pressed = None;
        }
        if self.selected().is_some_and(|old| !items.contains(old)) {
            self.activation_changed = true;
        }
        self.items = items;
        let matches = self.matches();
        if !matches
            .iter()
            .any(|i| Some(&i.id) == self.selected.as_ref())
        {
            self.selected = matches.first().map(|i| i.id.clone());
        }
    }
    fn matches(&self) -> Vec<&Item> {
        let q = self.input.text.trim().to_lowercase();
        self.items
            .iter()
            .filter(|i| i.title.to_lowercase().contains(&q) || i.id.to_lowercase().contains(&q))
            .collect()
    }
    pub fn selected_id(&self) -> Option<&str> {
        self.selected.as_deref()
    }
    fn selected(&self) -> Option<&Item> {
        self.items
            .iter()
            .find(|i| Some(&i.id) == self.selected.as_ref())
    }
    fn move_selection(&mut self, down: bool) {
        let rows = self.matches();
        let at = rows
            .iter()
            .position(|i| Some(&i.id) == self.selected.as_ref())
            .unwrap_or(0);
        let at = if down {
            (at + 1).min(rows.len().saturating_sub(1))
        } else {
            at.saturating_sub(1)
        };
        self.selected = rows.get(at).map(|i| i.id.clone());
    }
    fn activate(&self) -> Outcome {
        if self.activation_changed {
            return Outcome::Stay;
        }
        match self.selected() {
            Some(i) if i.builtin => Outcome::Manage,
            Some(i) if i.action().is_some() => Outcome::Open(i.clone()),
            _ => Outcome::Stay,
        }
    }
    pub fn event(&mut self, event: &Event) -> Outcome {
        match event {
            Event::Key(k) => {
                self.pressed = None;
                if k.kind != KeyEventKind::Press {
                    return Outcome::Stay;
                }
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                if k.code == KeyCode::Esc || ctrl && matches!(k.code, KeyCode::Char(']' | '5')) {
                    return Outcome::Close;
                }
                match k.code {
                    KeyCode::Tab => self.focus = (self.focus + 1) % 3,
                    KeyCode::BackTab => self.focus = (self.focus + 2) % 3,
                    KeyCode::Enter if k.modifiers.is_empty() => {
                        return match self.focus {
                            1 => Outcome::Manage,
                            2 => Outcome::Close,
                            _ => self.activate(),
                        };
                    }
                    KeyCode::Up => {
                        self.focus = 0;
                        self.move_selection(false);
                    }
                    KeyCode::Down => {
                        self.focus = 0;
                        self.move_selection(true);
                    }
                    _ if self.focus == 0 => {
                        let old = self.input.text.clone();
                        if ctrl && k.code == KeyCode::Char('u') {
                            self.input.clear();
                        } else if !k.modifiers.intersects(
                            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                        ) && match k.code {
                            KeyCode::Char(c) => self.input.text.len() + c.len_utf8() <= 4096,
                            _ => true,
                        } {
                            self.input.key(k.code, false);
                        }
                        if self.input.text != old {
                            self.selected = self.matches().first().map(|i| i.id.clone());
                            self.top = 0;
                        }
                    }
                    _ => {}
                }
            }
            Event::Paste(s) if self.focus == 0 && self.input.text.len() + s.len() <= 4096 => {
                self.pressed = None;
                self.input.insert(s, false);
                self.selected = self.matches().first().map(|i| i.id.clone());
                self.top = 0;
            }
            Event::Mouse(m) => {
                let point = (m.column, m.row).into();
                let hit = self
                    .hits
                    .iter()
                    .find(|(r, _)| r.contains(point))
                    .map(|(_, t)| t.clone());
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.pressed = hit;
                        if self.field.contains(point) {
                            self.focus = 0;
                            self.input.click(point);
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some(target) =
                            self.pressed.take().filter(|t| Some(t) == hit.as_ref())
                        {
                            match target {
                                Target::Row(id) => {
                                    self.selected = Some(id);
                                    self.focus = 0;
                                }
                                Target::Open(item)
                                    if self.items.contains(&item) && item.action().is_some() =>
                                {
                                    return Outcome::Open(item);
                                }
                                Target::Manage => return Outcome::Manage,
                                Target::ManageItem(id)
                                    if self.items.iter().any(|i| i.id == id && i.builtin) =>
                                {
                                    self.selected = Some(id);
                                    return Outcome::Manage;
                                }
                                Target::Close => return Outcome::Close,
                                _ => {}
                            }
                        }
                    }
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                        if self.list.contains(point) =>
                    {
                        self.pressed = None;
                        self.focus = 0;
                        self.move_selection(m.kind == MouseEventKind::ScrollDown);
                    }
                    _ => {}
                }
            }
            Event::Resize(_, _) => self.pressed = None,
            _ => {}
        }
        Outcome::Stay
    }
    pub fn draw(&mut self, frame: &mut Frame, theme: &Theme) {
        let rows: Vec<_> = self.matches().into_iter().cloned().collect();
        let explanation = self
            .selected()
            .map(|item| {
                if !item.note.is_empty() && item.state != "Running" {
                    format!("{} · {}", item.title, item.note)
                } else {
                    let text = item.explanation();
                    if text.is_empty() {
                        format!("{} · Open the plugin view.", item.title)
                    } else {
                        text
                    }
                }
            })
            .unwrap_or_default();
        let height = 9 + rows.len().clamp(1, 12) as u16;
        let area = crate::theme::centered(frame.area(), 78, height);
        self.hits.clear();
        self.activation_changed = false;
        frame.render_widget(Clear, area);
        let title = self.placement.map_or_else(
            || " Plugins ".to_owned(),
            |(_, place)| format!(" Plugins · {} ", place.label()),
        );
        let show_position = area.width as usize >= title.width() + 18;
        frame.render_widget(
            theme
                .block(title, true)
                .style(theme.base().bg(theme.overlay)),
            area,
        );
        let inside = crate::ui::inner(area);
        let inside = inside.inner(Margin::new(u16::from(inside.width >= 38), 0));
        let search_height = if inside.height >= 8 { 3 } else { 1 }.min(inside.height);
        let search = Rect::new(inside.x, inside.y, inside.width, search_height);
        self.field = if search_height == 3 {
            frame.render_widget(theme.block("", false), search);
            crate::ui::inner(search)
        } else {
            search
        };
        self.input
            .draw(frame, self.field, self.focus == 0, "Search plugins", theme);
        let footer_rows = 2.min(inside.height.saturating_sub(search_height));
        let detail_rows = u16::from(inside.height >= search_height + footer_rows + 3);
        let header_rows = u16::from(inside.height >= search_height + footer_rows + detail_rows + 2);
        self.list = Rect::new(
            inside.x,
            inside.y + search_height + header_rows,
            inside.width,
            inside
                .height
                .saturating_sub(search_height + header_rows + footer_rows + detail_rows),
        )
        .intersection(inside);
        // Shared column widths keep every status and action on the same edge.
        let action_width = if inside.width >= 38 { 8 } else { 0 };
        let status_width = if inside.width >= 24 { 12 } else { 0 };
        let gaps = u16::from(action_width > 0) * 2 + u16::from(status_width > 0) * 2;
        let name_width = inside
            .width
            .saturating_sub(action_width + status_width + gaps);
        let status_x = inside.x + name_width + u16::from(status_width > 0) * 2;
        let action_x = inside.right().saturating_sub(action_width);
        if header_rows > 0 {
            let y = self.list.y - 1;
            for (text, x, width) in [
                ("Plugin", inside.x, name_width),
                ("Status", status_x, status_width),
                ("Action", action_x, action_width),
            ] {
                frame.render_widget(
                    Paragraph::new(text).style(theme.base().fg(theme.dim)),
                    Rect::new(x, y, width, 1),
                );
            }
        }
        if let Some(at) = rows
            .iter()
            .position(|i| Some(&i.id) == self.selected.as_ref())
        {
            self.top = self
                .top
                .min(at)
                .max((at + 1).saturating_sub(self.list.height as usize));
        }
        if show_position && area.height > 0 {
            let end = (self.top + self.list.height as usize).min(rows.len());
            let position = if rows.is_empty() {
                "0 / 0".to_owned()
            } else {
                format!("{}–{} / {}", (self.top + 1).min(end), end, rows.len())
            };
            frame.render_widget(
                Paragraph::new(position)
                    .alignment(ratatui::layout::Alignment::Right)
                    .style(theme.base().fg(theme.muted)),
                Rect::new(area.right() - 18, area.y, 15, 1),
            );
        }
        if rows.is_empty() {
            frame.render_widget(
                Paragraph::new(if self.items.is_empty() {
                    if self.placement.is_some() {
                        "No plugins available here."
                    } else {
                        "No plugins registered."
                    }
                } else {
                    "No matching plugins"
                })
                .wrap(Wrap { trim: false }),
                self.list,
            );
        }
        for (n, item) in rows
            .iter()
            .skip(self.top)
            .take(self.list.height as usize)
            .enumerate()
        {
            let rect = Rect::new(self.list.x, self.list.y + n as u16, self.list.width, 1);
            let selected = Some(&item.id) == self.selected.as_ref();
            let style = theme.base().bg(if selected {
                theme.agent_selected
            } else {
                theme.overlay
            });
            frame.render_widget(
                Paragraph::new(" ".repeat(rect.width as usize)).style(style),
                rect,
            );
            let disabled = item.state == "Disabled";
            let title_style = if selected {
                style.add_modifier(Modifier::BOLD)
            } else if disabled {
                style.fg(theme.dim)
            } else {
                style
            };
            let suffix = if item.builtin && name_width >= 23 {
                " · Built-in"
            } else {
                ""
            };
            let title = format!("{} {}", if selected { "›" } else { " " }, item.title);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        crate::ui::clip(
                            &title,
                            name_width.saturating_sub(suffix.width() as u16) as usize,
                        ),
                        title_style,
                    ),
                    Span::styled(suffix, style.fg(theme.dim)),
                ])),
                Rect::new(rect.x, rect.y, name_width, 1),
            );
            let status = Rect::new(status_x, rect.y, status_width, 1);
            frame.render_widget(
                Paragraph::new(crate::ui::clip(item.status(), status_width as usize)).style(
                    style.fg(
                        if matches!(
                            item.state.as_str(),
                            "Failed" | "Unavailable" | "Unresponsive"
                        ) {
                            theme.danger
                        } else if disabled {
                            theme.dim
                        } else {
                            theme.muted
                        },
                    ),
                ),
                status,
            );
            let row_hit = Rect::new(rect.x, rect.y, rect.width.saturating_sub(action_width), 1);
            self.hits.push((row_hit, Target::Row(item.id.clone())));
            if action_width > 0 {
                let action_area = Rect::new(rect.right() - action_width, rect.y, action_width, 1);
                frame.render_widget(
                    Paragraph::new(
                        self.action(item)
                            .map(|action| format!("‹{action}›"))
                            .unwrap_or_else(|| "—".into()),
                    )
                    .style(style.fg(
                        if item.action().is_some() || item.builtin {
                            theme.focus
                        } else {
                            theme.dim
                        },
                    )),
                    action_area,
                );
                self.hits.push((
                    action_area,
                    if item.builtin {
                        Target::ManageItem(item.id.clone())
                    } else if item.action().is_some() {
                        Target::Open(item.clone())
                    } else {
                        Target::Row(item.id.clone())
                    },
                ));
            }
        }
        if detail_rows > 0 {
            frame.render_widget(
                Paragraph::new(crate::ui::clip(&explanation, inside.width as usize))
                    .style(theme.base().fg(theme.muted)),
                Rect::new(inside.x, self.list.bottom(), inside.width, detail_rows)
                    .intersection(inside),
            );
        }
        if footer_rows > 0 {
            let footer = Rect::new(inside.x, inside.bottom() - footer_rows, inside.width, 1);
            let manage_width = 16.min(footer.width);
            let manage = Rect::new(footer.x, footer.y, manage_width, 1);
            frame.render_widget(
                Paragraph::new("‹Manage plugins›").style(theme.base().fg(if self.focus == 1 {
                    theme.focus
                } else {
                    theme.muted
                })),
                manage,
            );
            self.hits.push((manage, Target::Manage));
            if footer.width >= 24 {
                let close = Rect::new(footer.right() - 7, footer.y, 7, 1);
                frame.render_widget(
                    Paragraph::new("‹Close›").style(theme.base().fg(if self.focus == 2 {
                        theme.focus
                    } else {
                        theme.muted
                    })),
                    close,
                );
                self.hits.push((close, Target::Close));
            }
        }
        if footer_rows == 2 {
            let action = match self.focus {
                1 => Some("Manage"),
                2 => Some("Close"),
                _ => self.selected().and_then(|item| self.action(item)),
            };
            let hints = if let Some(action) = action {
                vec![
                    format!("↑↓ Select · Enter {action} · Tab Controls · Esc Close"),
                    format!("↑↓ · Enter {action} · Esc"),
                    "Tab · Esc".into(),
                ]
            } else {
                vec![
                    "↑↓ Select · Tab Controls · Esc Close".into(),
                    "↑↓ · Tab Controls · Esc".into(),
                    "Tab · Esc".into(),
                ]
            };
            let hint = hints.iter().find(|h| h.width() <= inside.width as usize);
            frame.render_widget(
                Paragraph::new(hint.map(String::as_str).unwrap_or("Esc"))
                    .style(Style::default().fg(theme.muted)),
                Rect::new(inside.x, inside.bottom() - 1, inside.width, 1),
            );
        }
    }
}
