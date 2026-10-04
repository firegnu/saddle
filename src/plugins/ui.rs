//! Immediate plugin operations, independent of the Settings draft.
use super::{
    Manager,
    core::State,
    registry::{Entry, Manifest},
};
use crate::{
    buttons::{self, Button},
    launch::edit::Input,
    theme::Theme,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use unicode_width::UnicodeWidthStr;
#[derive(Default)]
pub struct Page {
    selected: usize,
    detail_scroll: usize,
    detail_area: Rect,
    focus: usize,
    pub message: String,
    /// `message` reports a failure rather than a result.
    error: bool,
    adding: Option<Adding>,
    hits: Vec<(Rect, usize)>,
    rows: Vec<(Rect, usize)>,
    tabs: Vec<(crate::input::Focus, buttons::Hit)>,
    pointer: buttons::Pointer,
}
struct Adding {
    input: Input,
    preview: Option<Manifest>,
    focus: usize,
    field: Rect,
}
pub enum Outcome {
    Stay,
    Back,
    Open(String),
    Page(crossterm::event::KeyEvent),
}
enum Row<'a> {
    Core(&'static saddle_core_plugin::Manifest),
    External(&'a Entry),
}
impl Row<'_> {
    fn id(&self) -> &str {
        match self {
            Row::Core(c) => c.id,
            Row::External(e) => &e.id,
        }
    }
}
/// Built-in rows first, then external registrations.
fn rows(m: &Manager) -> Vec<Row<'_>> {
    m.core_catalog()
        .iter()
        .map(|p| Row::Core(p.manifest()))
        .chain(m.registry.entries.iter().map(Row::External))
        .collect()
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Open,
    Toggle,
    Restart,
    Pin,
    Add,
    Remove,
    Sync,
    RemoveResources,
    Refresh,
    Back,
}
impl Page {
    pub fn select_plugin(&mut self, id: &str, m: &Manager) {
        let rows = rows(m);
        // An external registration holding a built-in ID is what the palette listed.
        self.detail_scroll = 0;
        self.selected = rows
            .iter()
            .position(|r| matches!(r, Row::External(e) if e.id == id))
            .or_else(|| rows.iter().position(|r| r.id() == id))
            .unwrap_or(0);
    }
    fn actions(&self, m: &Manager) -> Vec<(Action, &'static str, bool)> {
        let rows = rows(m);
        if let Some(Row::Core(c)) = rows.get(self.selected) {
            let state = m.core_state(c.id);
            let enabled = state == Some(State::Enabled);
            let recorded = m
                .core_resources(c.id)
                .and_then(|s| s.as_ref().ok())
                .is_some_and(|s| {
                    s.resources
                        .iter()
                        .flat_map(|r| &r.targets)
                        .any(|t| t.recorded())
                });
            return vec![
                (
                    Action::Toggle,
                    if enabled { "Disable" } else { "Enable" },
                    state != Some(State::Conflict),
                ),
                (
                    Action::Sync,
                    "Sync resources",
                    enabled && !c.resources.is_empty(),
                ),
                (
                    Action::RemoveResources,
                    "Remove resources",
                    state == Some(State::Disabled) && recorded,
                ),
                (Action::Add, "Add local…", m.registry.error.is_none()),
                (Action::Refresh, "Refresh", true),
                (Action::Back, "Back", true),
            ];
        }
        let id = rows.get(self.selected).map(Row::id);
        let state = id.map(|id| m.state(id)).unwrap_or_default();
        let enabled = id.is_some_and(|id| m.enabled_here(id));
        let stopped = id.is_none_or(|id| m.stopped(id));
        let pinned = id.is_some() && id == m.pinned();
        vec![
            (Action::Open, "Open panel", state == "Running"),
            (
                Action::Toggle,
                if enabled { "Disable" } else { "Enable" },
                id.is_some() && state != "Stopping" && (enabled || stopped),
            ),
            (
                Action::Restart,
                "Restart",
                enabled && matches!(state.as_str(), "Running" | "Unresponsive" | "Failed"),
            ),
            (
                Action::Pin,
                if pinned { "Unpin" } else { "Pin" },
                pinned || id.is_some_and(|id| m.pinnable(id)),
            ),
            (Action::Add, "Add local…", m.registry.error.is_none()),
            (
                Action::Remove,
                "Remove",
                id.is_some() && !enabled && stopped,
            ),
            (Action::Refresh, "Refresh", true),
            (Action::Back, "Back", true),
        ]
    }
    pub fn event(&mut self, event: Event, m: &mut Manager) -> Outcome {
        if let Event::Key(key) = &event
            && key.kind == KeyEventKind::Release
        {
            return Outcome::Stay;
        }
        if let Some(add) = &mut self.adding {
            match event {
                Event::Key(k) => match k.code {
                    KeyCode::Esc => {
                        self.adding = None;
                        self.message.clear();
                    }
                    KeyCode::Tab => add.focus = (add.focus + 1) % 4,
                    KeyCode::BackTab => add.focus = (add.focus + 3) % 4,
                    KeyCode::Enter if add.focus > 0 => {
                        let action = add.focus;
                        return self.add_action(action, m);
                    }
                    _ if add.focus == 0 => {
                        let old = add.input.text.clone();
                        add.input.key(k.code, false);
                        if old != add.input.text {
                            add.preview = None;
                        }
                    }
                    _ => {}
                },
                Event::Paste(s) if add.focus == 0 => {
                    if s.len() <= 65536 {
                        add.input.insert(&s, false);
                        add.preview = None;
                    }
                }
                Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                    let point = (mouse.column, mouse.row).into();
                    if add.field.contains(point) {
                        add.focus = 0;
                        add.input.click(point);
                    } else if let Some((_, i)) = self.hits.iter().find(|(r, _)| r.contains(point)) {
                        return self.add_action(*i, m);
                    }
                }
                _ => {}
            }
            return Outcome::Stay;
        }
        if let Event::Key(key) = &event {
            self.pointer.cancel();
            if matches!(key.code, KeyCode::F(1..=6)) {
                return Outcome::Page(*key);
            }
        }
        if let Event::Mouse(mouse) = &event {
            let captured = self.pointer.captured();
            if let Some((_, key)) = self.pointer.event(*mouse, &self.tabs) {
                return Outcome::Page(key);
            }
            if captured || self.pointer.captured() {
                return Outcome::Stay;
            }
        }
        let count = rows(m).len();
        self.selected = self.selected.min(count.saturating_sub(1));
        let stops = self.actions(m).len() + 1;
        self.focus = self.focus.min(stops - 1);
        let before = self.selected;
        let action = match event {
            Event::Key(k) => match k.code {
                KeyCode::Esc => return Outcome::Back,
                KeyCode::PageUp => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(5);
                    None
                }
                KeyCode::PageDown => {
                    self.detail_scroll = self.detail_scroll.saturating_add(5);
                    None
                }
                KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    None
                }
                KeyCode::Down => {
                    self.selected = (self.selected + 1).min(count.saturating_sub(1));
                    None
                }
                KeyCode::Tab => {
                    self.focus = (self.focus + 1) % stops;
                    None
                }
                KeyCode::BackTab => {
                    self.focus = (self.focus + stops - 1) % stops;
                    None
                }
                KeyCode::Enter if self.focus > 0 => Some(self.focus - 1),
                _ => None,
            },
            Event::Mouse(mouse)
                if self.detail_area.contains((mouse.column, mouse.row).into())
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                    ) =>
            {
                self.detail_scroll = if mouse.kind == MouseEventKind::ScrollDown {
                    self.detail_scroll.saturating_add(3)
                } else {
                    self.detail_scroll.saturating_sub(3)
                };
                None
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let p = (mouse.column, mouse.row).into();
                if let Some((_, i)) = self.rows.iter().find(|(r, _)| r.contains(p)) {
                    self.selected = *i;
                    self.focus = 0;
                    None
                } else {
                    self.hits
                        .iter()
                        .find(|(r, _)| r.contains(p))
                        .map(|(_, i)| *i)
                }
            }
            _ => None,
        };
        if self.selected != before {
            self.detail_scroll = 0;
        }
        let Some(action) = action else {
            return Outcome::Stay;
        };
        let Some(&(action, _, true)) = self.actions(m).get(action) else {
            return Outcome::Stay;
        };
        let (id, builtin) = rows(m)
            .get(self.selected)
            .map(|r| (r.id().to_owned(), matches!(r, Row::Core(_))))
            .unwrap_or_default();
        let result = match action {
            Action::Open => return Outcome::Open(id),
            Action::Toggle if builtin => {
                let enable = m.core_state(&id) != Some(State::Enabled);
                match m.core_enabled(&id, enable) {
                    Ok(()) if !enable => {
                        self.error = false;
                        self.message = "Disabled. Installed resources are kept; Remove resources deletes unmodified ones.".into();
                        return Outcome::Stay;
                    }
                    result => result,
                }
            }
            Action::Toggle => {
                if m.enabled_here(&id) {
                    m.disable(&id)
                } else {
                    m.enable(&id)
                }
            }
            Action::Restart => m.restart(&id),
            Action::Pin => {
                let pin = m.pinned() != Some(id.as_str());
                match m.pin(pin.then_some(id.as_str())) {
                    Ok(()) => {
                        self.error = false;
                        self.message = if pin {
                            "Pinned to the Agents header; press p in Agents to open it."
                        } else {
                            "Unpinned from the Agents header."
                        }
                        .into();
                        return Outcome::Stay;
                    }
                    result => result,
                }
            }
            Action::Add => {
                self.adding = Some(Adding {
                    input: Input::new(String::new()),
                    preview: None,
                    focus: 0,
                    field: Rect::default(),
                });
                self.message.clear();
                Ok(())
            }
            Action::Remove => m.remove(&id),
            Action::Sync => m.core_sync(&id),
            Action::RemoveResources => m.core_remove_resources(&id),
            Action::Refresh => m.refresh(),
            Action::Back => return Outcome::Back,
        };
        self.error = result.is_err();
        self.message = result.err().map(|e| format!("{e:#}")).unwrap_or_default();
        Outcome::Stay
    }
    fn add_action(&mut self, action: usize, m: &mut Manager) -> Outcome {
        let add = self.adding.as_mut().unwrap();
        match action {
            1 => match Manifest::read(&crate::config::expand_home(&add.input.text)) {
                Ok(manifest) => {
                    add.preview = Some(manifest);
                    self.message.clear();
                }
                Err(e) => {
                    add.preview = None;
                    self.error = true;
                    self.message = format!("{e:#}");
                }
            },
            2 => {
                if let Some(manifest) = &add.preview {
                    match m.add(&crate::config::expand_home(&add.input.text), manifest) {
                        Ok(()) => {
                            self.adding = None;
                            self.error = false;
                            self.message = "Added disabled. Enable to start it.".into();
                            self.selected = rows(m).len().saturating_sub(1);
                        }
                        Err(e) => {
                            self.error = true;
                            self.message = format!("{e:#}");
                        }
                    }
                }
            }
            3 => {
                self.adding = None;
                self.message.clear();
            }
            _ => {}
        }
        Outcome::Stay
    }
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        m: &Manager,
        settings: &crate::settings::Settings,
    ) {
        let rows = rows(m);
        let area = if self.adding.is_some() {
            crate::theme::centered(frame.area(), 80, 24)
        } else {
            crate::settings::page_area(frame.area())
        };
        frame.render_widget(Clear, area);
        frame.render_widget(
            t.block(
                if self.adding.is_some() {
                    " Add local plugin "
                } else {
                    crate::settings::TITLE
                },
                true,
            )
            .style(t.base().bg(t.overlay)),
            area,
        );
        let inside = crate::ui::inner(area);
        let inside = Rect {
            x: inside.x + 1.min(inside.width),
            width: inside.width.saturating_sub(2),
            ..inside
        };
        self.rows.clear();
        self.detail_area = Rect::default();
        self.hits.clear();
        self.tabs.clear();
        let choices: Vec<_> = if let Some(add) = &self.adding {
            vec![
                ("Read manifest", true),
                ("Add disabled", add.preview.is_some()),
                ("Cancel", true),
            ]
        } else {
            self.actions(m)
                .into_iter()
                .map(|(_, name, enabled)| (name, enabled))
                .collect()
        };
        let focus = self.adding.as_ref().map_or(self.focus, |a| a.focus);
        // Add local's main step is reading the manifest, then adding it; the management page
        // acts on whichever plugin is selected and has no single main action.
        let primary = self
            .adding
            .as_ref()
            .map(|a| if a.preview.is_some() { 1 } else { 0 });
        let buttons: Vec<_> = choices
            .iter()
            .enumerate()
            .map(|(i, (name, enabled))| {
                let b = Button::new(name, KeyCode::F(i as u8 + 1), *enabled);
                if primary == Some(i) { b.primary() } else { b }
            })
            .collect();
        let (body, hits) = buttons::draw_compact(t, frame, inside, &buttons);
        // F(i) only numbers the hit areas: these labels have no key, so none of their words is
        // drawn in the key colour.
        for hit in &hits {
            if let KeyCode::F(i) = hit.key.code
                && primary != Some(usize::from(i) - 1)
            {
                frame
                    .buffer_mut()
                    .set_style(label_area(hit.area), Style::default().fg(t.text));
            }
        }
        // Keyboard focus: underlined label in the focus colour, distinct from a primary action;
        // a disabled control keeps its dim colour but still shows where focus is.
        let bar = Rect::new(
            inside.x,
            body.bottom(),
            inside.width,
            inside.bottom().saturating_sub(body.bottom()),
        );
        if let Some((name, enabled)) = focus.checked_sub(1).and_then(|i| choices.get(i))
            && let Some(area) = drawn(frame, bar, name)
        {
            let buffer = frame.buffer_mut();
            if *enabled {
                buffer.set_style(area, Style::default().fg(t.focus));
            }
            // A cut button has no closing mark; its label runs to the last column.
            let cut = buffer[(area.right() - 1, area.y)].symbol() != "›";
            let label = label_area(area);
            buffer.set_style(
                Rect {
                    width: label.width + u16::from(cut),
                    ..label
                },
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            );
        }
        self.hits = hits
            .into_iter()
            .filter_map(|h| {
                if let KeyCode::F(i) = h.key.code {
                    Some((h.area, usize::from(i) - usize::from(self.adding.is_none())))
                } else {
                    None
                }
            })
            .collect();
        if let Some(add) = &mut self.adding {
            let field_area = Rect::new(body.x, body.y, body.width, body.height.min(3));
            let field = t.block(" Plugin directory ", add.focus == 0);
            add.field = field.inner(field_area);
            frame.render_widget(field, field_area);
            let text = Style::default().fg(t.text);
            let muted = Style::default().fg(t.muted);
            // The manifest read leads; its warning stays readable; the result or error follows.
            let mut lines = match &add.preview {
                Some(p) => vec![
                    Line::styled(
                        format!("{}  {} · {}", p.name, p.version, p.id),
                        text.add_modifier(Modifier::BOLD),
                    ),
                    Line::from(vec![
                        Span::styled("Program: ", muted),
                        Span::styled(
                            p.program(&crate::config::expand_home(&add.input.text))
                                .map(|p| p.display().to_string())
                                .unwrap_or_default(),
                            text,
                        ),
                    ]),
                    Line::default(),
                    Line::styled("Runs with your user permissions when enabled.", text),
                    Line::styled("Adding does not start the plugin.", muted),
                ],
                None => vec![Line::styled(
                    "Enter a directory, then Read manifest.",
                    muted,
                )],
            };
            lines.push(Line::default());
            let result = Style::default().fg(if self.error { t.danger } else { t.muted });
            lines.extend(
                self.message
                    .lines()
                    .map(|line| Line::styled(line.to_owned(), result)),
            );
            frame.render_widget(
                Paragraph::new(lines).wrap(Wrap { trim: false }),
                Rect::new(
                    body.x,
                    body.y + 4.min(body.height),
                    body.width,
                    body.height.saturating_sub(4),
                ),
            );
            add.input
                .draw(frame, add.field, add.focus == 0, "/path/to/plugin", t);
        } else {
            let (body, tabs) =
                settings.draw_header(t, frame, body, Some(crate::settings::Page::Plugins));
            self.tabs = tabs
                .into_iter()
                .map(|hit| (crate::input::Focus::Agents, hit))
                .collect();
            self.pointer.paint(t, frame, &self.tabs);
            let mut body = body;
            if body.height > 0 {
                frame.render_widget(
                    Paragraph::new("Changes apply immediately.")
                        .style(Style::default().fg(t.muted)),
                    Rect::new(body.x, body.y, body.width, 1),
                );
                body.y += 1;
                body.height -= 1;
            }
            if body.height > 0 {
                frame.render_widget(
                    Paragraph::new(
                        "↑↓ Select · Tab Focus · Enter Activate · PgUp/PgDn Details · Esc Back",
                    )
                    .style(Style::default().fg(t.muted)),
                    Rect::new(body.x, body.bottom() - 1, body.width, 1),
                );
                body.height -= 1;
            }
            let (message, error) = match &m.registry.error {
                Some(error) => (error.as_str(), true),
                None => (self.message.as_str(), self.error),
            };
            if !message.is_empty() && body.height > 2 {
                let height = (wrapped(message, body.width as usize) as u16)
                    .min(3)
                    .min(body.height - 1);
                frame.render_widget(
                    Paragraph::new(message)
                        .wrap(Wrap { trim: false })
                        .style(Style::default().fg(if error { t.danger } else { t.muted })),
                    Rect::new(body.x, body.bottom() - height, body.width, height),
                );
                body.height -= height;
            }
            let (list, detail) = if body.width >= 96 {
                let left = Rect::new(body.x, body.y, 38, body.height);
                frame.render_widget(
                    Block::new()
                        .borders(Borders::LEFT)
                        .border_style(Style::default().fg(t.border)),
                    Rect::new(left.right(), body.y, 1, body.height),
                );
                (
                    left,
                    Rect::new(left.right() + 2, body.y, body.width - 40, body.height),
                )
            } else {
                let height = (rows.len().min(7) as u16 + 2).min(body.height / 2);
                (
                    Rect { height, ..body },
                    Rect::new(body.x, body.y + height, body.width, body.height - height),
                )
            };
            let name_width = usize::from(list.width).saturating_sub(21).clamp(6, 24);
            if list.height > 0 {
                frame.render_widget(
                    Paragraph::new(format!(
                        "  {} On   Runtime",
                        crate::ui::pad("Plugin", name_width)
                    ))
                    .style(Style::default().fg(t.muted)),
                    Rect { height: 1, ..list },
                );
            }
            let count = usize::from(list.height.saturating_sub(2));
            let start = self.selected.saturating_sub(count.saturating_sub(1));
            for (offset, (i, entry)) in rows.iter().enumerate().skip(start).take(count).enumerate()
            {
                let r = Rect::new(list.x, list.y + 1 + offset as u16, list.width, 1);
                let (name, enabled, runtime) = match entry {
                    Row::Core(c) => (
                        c.name.to_owned(),
                        match m.core_state(c.id) {
                            Some(State::Enabled) => "Yes",
                            Some(State::Conflict) => "!",
                            _ => "No",
                        },
                        "Built-in".to_owned(),
                    ),
                    Row::External(e) => (
                        m.manifest(&e.id)
                            .map(|p| p.name)
                            .unwrap_or_else(|_| e.id.clone()),
                        if e.enabled { "Yes" } else { "No" },
                        m.state(&e.id),
                    ),
                };
                // Selection: background, weight and marker; the marker also shows list focus.
                let selected = i == self.selected;
                let style = if selected {
                    Style::default()
                        .fg(t.text)
                        .bg(t.agent_selected)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.text)
                };
                let failing = |bad: bool| if bad { style.fg(t.danger) } else { style };
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(
                            if selected { "›" } else { " " },
                            style.fg(if self.focus == 0 { t.focus } else { t.muted }),
                        ),
                        Span::raw(" "),
                        Span::raw(crate::ui::pad(
                            &crate::ui::clip(&name, name_width),
                            name_width,
                        )),
                        Span::raw(" "),
                        Span::styled(format!("{enabled:4}"), failing(enabled == "!")),
                        Span::raw(" "),
                        Span::styled(
                            runtime.clone(),
                            failing(matches!(
                                runtime.as_str(),
                                "Failed" | "Unresponsive" | "Unavailable"
                            )),
                        ),
                    ]))
                    .style(style),
                    r,
                );
                self.rows.push((r, i));
            }
            if rows.is_empty() && list.height > 1 {
                frame.render_widget(
                    Paragraph::new("No plugins registered").style(Style::default().fg(t.muted)),
                    Rect::new(list.x, list.y + 1, list.width, 1),
                );
            }
            if rows.len() > count && list.height > 1 {
                frame.render_widget(
                    Paragraph::new(format!(
                        "{}–{} / {}",
                        start + 1,
                        (start + count).min(rows.len()),
                        rows.len()
                    ))
                    .style(Style::default().fg(t.muted)),
                    Rect::new(list.x, list.bottom() - 1, list.width, 1),
                );
            }
            let (name, text) = match rows.get(self.selected) {
                Some(Row::Core(c)) => (c.name.to_owned(), core_detail(m, c)),
                Some(Row::External(e)) => (
                    m.manifest(&e.id)
                        .map(|p| p.name)
                        .unwrap_or_else(|_| e.id.clone()),
                    format!(
                        "{} · {}\nID: {}{}\n\nTechnical details\nDirectory\n{}\n\nProgram\n{}\n\n{}\n{}",
                        if e.enabled { "Enabled" } else { "Disabled" },
                        m.state(&e.id),
                        e.id,
                        if m.pinned() == Some(e.id.as_str()) {
                            "\nPinned to the Agents header."
                        } else if !m.pinnable(&e.id) {
                            "\nNo view to open; cannot be pinned."
                        } else {
                            ""
                        },
                        e.directory.display(),
                        m.program(&e.id)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "Unavailable".into()),
                        if e.enabled != m.enabled_here(&e.id) {
                            "Registry differs from this instance. Enable/Disable here explicitly."
                        } else {
                            "Remove deletes registration only; files are kept."
                        },
                        m.note(&e.id)
                    ),
                ),
                None => (
                    "Plugin details".into(),
                    "Add a local directory to begin.".into(),
                ),
            };
            let block = Block::new()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(t.border))
                .title(ratatui::text::Line::styled(
                    format!(" {name} "),
                    Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
                ));
            let viewport = block.inner(detail);
            frame.render_widget(block, detail);
            self.detail_area = viewport;
            let width = viewport.width.saturating_sub(1);
            let total = wrapped(&text, width as usize);
            self.detail_scroll = self
                .detail_scroll
                .min(total.saturating_sub(viewport.height as usize));
            let lines: Vec<_> = text
                .lines()
                .map(|line| {
                    ratatui::text::Line::styled(
                        line.to_owned(),
                        if matches!(
                            line,
                            "Resources"
                                | "Setup"
                                | "Technical details"
                                | "Directory"
                                | "Program"
                                | "Setup files"
                        ) {
                            Style::default().fg(t.muted).add_modifier(Modifier::BOLD)
                        } else if problem(line) {
                            Style::default().fg(t.danger)
                        } else {
                            Style::default().fg(t.text)
                        },
                    )
                })
                .collect();
            frame.render_widget(
                Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .scroll((self.detail_scroll.min(u16::MAX as usize) as u16, 0)),
                Rect { width, ..viewport },
            );
            if total > viewport.height as usize && !viewport.is_empty() {
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None)
                        .thumb_style(Style::default().fg(t.muted)),
                    viewport,
                    &mut ScrollbarState::new(total.saturating_sub(viewport.height as usize) + 1)
                        .position(self.detail_scroll),
                );
            }
        }
    }
}

/// The host renders only plugin-provided text and resolved paths; it never parses them.
fn core_detail(m: &Manager, c: &saddle_core_plugin::Manifest) -> String {
    let state = match m.core_state(c.id) {
        Some(State::Enabled) => "Enabled",
        Some(State::Conflict) => "Conflict",
        _ => "Disabled",
    };
    let mut lines = vec![
        format!("Built-in · {state}"),
        format!("ID: {}", c.id),
        String::new(),
        "Resources".into(),
    ];
    if state == "Conflict" {
        lines.push("An external plugin uses this ID; built-in commands are refused.".into());
    }
    let status = m.core_resources(c.id).and_then(|s| match s {
        Ok(s) => Some(s),
        Err(e) => {
            lines.push(format!("Resources unavailable: {e}"));
            None
        }
    });
    for r in status.iter().flat_map(|s| &s.resources) {
        let kind = if r.kind == "agent_skill" {
            "Skill"
        } else {
            "Resource"
        };
        match &r.error {
            Some(e) => lines.push(format!("{kind}: {} — not installable ({e})", r.name)),
            None => {
                lines.push(format!("{kind}: {}", r.name));
                for t in &r.targets {
                    lines.push(format!("  {:13} {}", t.label, t.text()));
                }
            }
        }
    }
    let result = m.core_receipt(c.id).map(|r| r.summary());
    if !c.setup_note.is_empty() || !c.setup_files.is_empty() {
        lines.push(String::new());
        lines.push("Setup".into());
        lines.extend(c.setup_note.lines().map(str::to_owned));
        if !c.setup_files.is_empty() {
            lines.push(String::new());
            lines.push("Setup files".into());
        }
        for (i, f) in c.setup_files.iter().enumerate() {
            let paths = status
                .and_then(|s| s.setup_files.get(i))
                .map(|s| s.paths.as_slice())
                .unwrap_or_default();
            if paths.is_empty() {
                lines.push(format!(
                    "{}: not installed (see skill status above)",
                    f.label
                ));
            }
            lines.extend(
                paths
                    .iter()
                    .map(|p| format!("{}: {}", f.label, p.display())),
            );
        }
    }
    if let Some(result) = result {
        lines.push(String::new());
        lines.push(result);
    }
    lines.join("\n")
}
/// Problems the host itself wrote into the details, drawn in the error role.
fn problem(line: &str) -> bool {
    line.starts_with("Resources unavailable: ")
        || line.starts_with("An external plugin uses this ID")
        || line.contains(" — not installable (")
}
/// A compact button's label, inside its ‹ › marks.
fn label_area(button: Rect) -> Rect {
    Rect {
        x: button.x + 1.min(button.width),
        width: button.width.saturating_sub(2),
        ..button
    }
}
/// Where the compact button `‹label›` was drawn in `area`; disabled buttons have no hit area.
/// A button wider than `area` is drawn cut on a row of its own, so its visible start counts.
fn drawn(frame: &mut Frame, area: Rect, label: &str) -> Option<Rect> {
    let want: Vec<String> = format!("‹{label}›")
        .chars()
        .map(String::from)
        .take(usize::from(area.width))
        .collect();
    let width = want.len() as u16;
    let buffer = frame.buffer_mut();
    (area.top()..area.bottom()).find_map(|y| {
        (area.left()..area.right().saturating_sub(width.saturating_sub(1)))
            .find(|&x| {
                want.iter()
                    .zip(x..)
                    .all(|(c, x)| buffer[(x, y)].symbol() == c)
            })
            .map(|x| Rect::new(x, y, width, 1))
    })
}
/// Rows a word-wrapped paragraph needs, plus one spare so the result line is never cut.
fn wrapped(text: &str, width: usize) -> usize {
    // Long unbroken runs lose up to one column per row to double-width characters.
    let (width, run) = (width.max(2), width.max(2) - 1);
    let rows = |line: &str| {
        let (mut rows, mut used) = (1, 0);
        for word in line.split(' ') {
            let w = word.width();
            if used + usize::from(used > 0) + w <= width {
                used += usize::from(used > 0) + w;
                continue;
            }
            rows += usize::from(used > 0);
            rows += w.saturating_sub(1) / run;
            used = w - w.saturating_sub(1) / run * run;
        }
        rows
    };
    text.lines().map(rows).sum::<usize>() + 1
}
