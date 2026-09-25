use crate::drover::{Operation, Request, Snapshot, Task};
use crate::theme::{self, Theme};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Default)]
pub enum Page {
    #[default]
    List,
    Detail,
    Help,
    Feedback(String),
    Projects,
    Project(String),
    Add {
        title: String,
        body: String,
        body_focus: bool,
    },
}
#[derive(Default)]
pub struct Panel {
    pub(crate) buttons: Vec<crate::buttons::Hit>,
    pub(crate) fields: Vec<(ratatui::layout::Rect, bool)>,
    pub projects: Vec<String>,
    pub project_selected: usize,
    pub registry_error: Option<String>,
    pub project_rows: Vec<(ratatui::layout::Rect, usize)>,
    pub project: String,
    pub read_error: Option<String>,
    pub snapshot: Option<Snapshot>,
    pub selected: usize,
    pub top: usize,
    pub(crate) manual_scroll: bool,
    pub(crate) list_area: ratatui::layout::Rect,
    pub scroll: usize,
    pub page: Page,
    pub message: String,
    pub(crate) message_failed: bool,
    pub busy: bool,
}
impl Panel {
    pub fn click(&mut self, column: u16, row: u16) -> Option<Request> {
        let point = (column, row).into();
        if let Some(hit) = self.buttons.iter().find(|hit| hit.area.contains(point)) {
            return self.key(hit.key);
        }
        if self.busy {
            return None;
        }
        if let Some((_, index)) = self
            .project_rows
            .iter()
            .find(|(area, _)| area.contains(point))
        {
            return self.projects.get(*index).cloned().map(Request::Project);
        }
        if let Page::Add { body_focus, .. } = &mut self.page
            && let Some((_, body)) = self.fields.iter().find(|(area, _)| area.contains(point))
        {
            *body_focus = *body;
        }
        None
    }

    fn controls(&self) -> Vec<crate::buttons::Button<'static>> {
        use crate::buttons::Button as B;
        use KeyCode as K;
        match self.page {
            Page::Add { .. } => vec![
                B::control(
                    "Save ^s",
                    K::Char('s'),
                    !self.busy && self.read_error.is_none(),
                )
                .primary(),
                B::new("Cancel Esc", K::Esc, !self.busy),
            ],
            Page::Project(_) => vec![
                B::new("Apply ↵", K::Enter, !self.busy),
                B::new("Cancel Esc", K::Esc, true),
            ],
            Page::Projects => vec![
                B::new("Open ↵", K::Enter, !self.projects.is_empty()),
                B::new("Refresh r", K::Char('r'), true),
                B::new("Path e", K::Char('e'), true),
                B::new("Back Esc", K::Esc, true),
            ],
            _ => vec![B::new("Back Esc", K::Esc, true)],
        }
    }
    pub fn tasks(&self) -> Vec<(&'static str, &Task)> {
        let Some(s) = &self.snapshot else {
            return Vec::new();
        };
        s.current
            .iter()
            .map(|t| ("Current", t))
            .chain(s.awaiting.iter().map(|t| ("Awaiting", t)))
            .chain(s.pending.iter().map(|t| ("Pending", t)))
            .chain(s.history.iter().rev().map(|t| ("History", t)))
            .collect()
    }
    pub fn absorb(&mut self, snapshot: Snapshot) {
        self.read_error = None;
        let old = self
            .tasks()
            .get(self.selected)
            .map(|(_, t)| (t.id.clone(), t.title.clone()));
        self.snapshot = Some(snapshot);
        let tasks = self.tasks();
        self.selected = old
            .and_then(|old| {
                tasks
                    .iter()
                    .position(|(_, t)| (t.id.clone(), t.title.clone()) == old)
            })
            .unwrap_or(self.selected)
            .min(tasks.len().saturating_sub(1));
    }
    pub fn select(&mut self, index: usize) {
        self.selected = index.min(self.tasks().len().saturating_sub(1));
        self.scroll = 0;
        self.manual_scroll = false;
    }
    pub fn wheel(&mut self, column: u16, row: u16, delta: isize) {
        if !self.overlay_open() && self.list_area.contains((column, row).into()) {
            if self.read_error.is_some() {
                self.scroll = self.scroll.saturating_add_signed(delta);
            } else {
                self.top = self.top.saturating_add_signed(delta);
                self.manual_scroll = true;
            }
        }
    }
    pub fn paste(&mut self, text: &str) {
        if self.busy {
            return;
        }
        if let Page::Project(path) = &mut self.page {
            path.extend(text.chars().filter(|c| !c.is_control()));
            return;
        }
        if let Page::Add {
            title,
            body,
            body_focus,
        } = &mut self.page
        {
            let field = if *body_focus { body } else { title };
            field.extend(
                text.chars()
                    .filter(|c| !c.is_control() || (*body_focus && *c == '\n')),
            );
        }
    }
    pub fn complete(&mut self, operation: &Operation, result: anyhow::Result<String>) {
        self.busy = false;
        match result {
            Ok(text) => {
                self.message_failed = false;
                self.message = text.clone();
                if matches!(operation, Operation::Go | Operation::Next) {
                    self.page = Page::Feedback(text);
                    self.scroll = 0;
                } else if matches!(operation, Operation::Add { .. }) {
                    self.page = Page::List;
                }
            }
            Err(error) => {
                self.message_failed = true;
                self.message = format!("{error:#}");
                if !matches!(self.page, Page::Add { .. }) {
                    self.page = Page::Feedback(self.message.clone());
                    self.scroll = 0;
                }
            }
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> Option<Request> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if matches!(self.page, Page::Projects) {
            match key.code {
                KeyCode::Esc => self.page = Page::List,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.project_selected = self.project_selected.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.project_selected =
                        (self.project_selected + 1).min(self.projects.len().saturating_sub(1))
                }
                KeyCode::Enter => {
                    return self
                        .projects
                        .get(self.project_selected)
                        .cloned()
                        .map(Request::Project);
                }
                KeyCode::Char('e') => self.page = Page::Project(self.project.clone()),
                KeyCode::Char('r') => return Some(Request::Projects),
                _ => {}
            }
            return None;
        }
        if let Page::Project(path) = &mut self.page {
            match key.code {
                KeyCode::Esc => self.page = Page::List,
                KeyCode::Enter => {
                    if !path.trim().is_empty() {
                        return Some(Request::Project(path.clone()));
                    }
                }
                KeyCode::Backspace => {
                    path.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => path.clear(),
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    path.push(c)
                }
                _ => {}
            }
            return None;
        }
        if let Page::Add {
            title,
            body,
            body_focus,
        } = &mut self.page
        {
            if self.busy {
                return None;
            }
            match key.code {
                KeyCode::Esc => self.page = Page::List,
                KeyCode::Tab | KeyCode::BackTab => *body_focus = !*body_focus,
                KeyCode::Enter if !*body_focus => *body_focus = true,
                KeyCode::Enter => body.push('\n'),
                KeyCode::Backspace => {
                    if *body_focus {
                        body.pop();
                    } else {
                        title.pop();
                    }
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.read_error.is_some() {
                        return None;
                    }
                    if title.trim().is_empty() {
                        self.message_failed = true;
                        self.message = "Title is required".into();
                        return None;
                    }
                    self.busy = true;
                    self.message_failed = false;
                    self.message = "Adding task…".into();
                    return Some(Request::Run(Operation::Add {
                        title: title.clone(),
                        body: body.clone(),
                    }));
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    if *body_focus {
                        body.push(c);
                    } else {
                        title.push(c);
                    }
                }
                _ => {}
            }
            return None;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        match key.code {
            KeyCode::Esc => {
                self.message.clear();
                self.page = Page::List;
                self.scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if matches!(self.page, Page::List) && self.read_error.is_none() {
                    self.select(self.selected.saturating_sub(1));
                } else {
                    self.scroll = self.scroll.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if matches!(self.page, Page::List) && self.read_error.is_none() {
                    self.select(self.selected + 1);
                } else {
                    self.scroll = self.scroll.saturating_add(1);
                }
            }
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
            KeyCode::Enter if !self.tasks().is_empty() && self.read_error.is_none() => {
                self.page = Page::Detail;
                self.scroll = 0;
            }
            KeyCode::Char('?' | 'h') => {
                self.page = Page::Help;
                self.scroll = 0;
            }
            KeyCode::Char('c') if !self.busy => {
                self.page = Page::Projects;
                self.scroll = 0;
                return Some(Request::Projects);
            }
            KeyCode::Char('a')
                if !self.busy && self.read_error.is_none() && self.snapshot.is_some() =>
            {
                self.page = Page::Add {
                    title: String::new(),
                    body: String::new(),
                    body_focus: false,
                };
                self.message.clear();
            }
            KeyCode::Char('r') => return Some(Request::Refresh),
            KeyCode::Char(c @ ('g' | 'n' | 'p' | 'l')) if !self.busy => {
                if self.read_error.is_some() {
                    return None;
                }
                let Some(snapshot) = &self.snapshot else {
                    self.message_failed = true;
                    self.message = "Waiting for queue data; action not sent".into();
                    return None;
                };
                let operation = match c {
                    'g' => Operation::Go,
                    'n' => Operation::Next,
                    'p' => Operation::Pause(!snapshot.paused),
                    _ => Operation::Loop(!snapshot.mode.r#loop),
                };
                self.busy = true;
                self.message_failed = false;
                self.message = "Running action…".into();
                return Some(Request::Run(operation));
            }
            _ => {}
        }
        None
    }
}

impl Panel {
    fn message_color(&self, t: &Theme) -> ratatui::style::Color {
        if self.busy {
            t.agent_working
        } else if self.message_failed {
            t.agent_error
        } else {
            t.agent_idle
        }
    }
    pub fn overlay_open(&self) -> bool {
        !matches!(self.page, Page::List)
    }
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        focused: bool,
    ) -> Vec<(u16, usize)> {
        use crate::{
            buttons::{self, Button as B},
            ui,
        };
        use KeyCode as K;
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::{Line, Span},
            widgets::Paragraph,
        };
        self.buttons.clear();
        self.fields.clear();
        self.project_rows.clear();
        self.list_area = Rect::default();
        if area.is_empty() {
            return Vec::new();
        }
        let mut block = t.block(" Queue ", focused).title_top(
            Line::styled(
                format!(" {} tasks ", self.tasks().len()),
                Style::default().fg(t.muted),
            )
            .right_aligned(),
        );
        if self.overlay_open() && !focused {
            block = block.title_bottom(Line::styled(
                " Click Queue to resume ",
                Style::default().fg(t.focus),
            ));
        }
        let inside = block.inner(area);
        frame.render_widget(block, area);
        if inside.height < 3 || inside.width == 0 {
            return Vec::new();
        }
        let ready = !self.overlay_open()
            && !self.busy
            && self.snapshot.is_some()
            && self.read_error.is_none();
        let name = std::path::Path::new(&self.project)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let inline = inside.width >= 40;
        let controls_width = 23.min(inside.width);
        let controls = Rect::new(
            inside.right() - controls_width,
            inside.y + u16::from(!inline),
            controls_width,
            inside.height.saturating_sub(u16::from(!inline)).min(1),
        );
        let (_, project_hits) = buttons::draw_compact_top(
            t,
            frame,
            controls,
            &[
                B::new(
                    "Refresh r",
                    K::Char('r'),
                    !self.busy && !self.overlay_open(),
                ),
                B::new(
                    "Project c",
                    K::Char('c'),
                    !self.busy && !self.overlay_open(),
                ),
            ],
        );
        self.buttons.extend(project_hits);
        let name_width = if inline {
            inside.width.saturating_sub(controls_width + 1)
        } else {
            inside.width
        };
        frame.render_widget(
            Paragraph::new(ui::clip(&name, usize::from(name_width)))
                .style(Style::default().fg(t.bright)),
            Rect::new(inside.x, inside.y, name_width, 1),
        );
        if inline {
            frame.render_widget(
                Paragraph::new(ui::clip(&self.project, usize::from(inside.width)))
                    .style(Style::default().fg(t.dim)),
                Rect::new(inside.x, inside.y + 1, inside.width, 1),
            );
        }
        let header_height = 3;
        let emphasis = |color| Style::default().fg(color).add_modifier(Modifier::BOLD);
        let mode = if self.read_error.is_some() {
            Line::styled("Read failed", emphasis(t.agent_error))
        } else if let Some(s) = &self.snapshot {
            let (state, color) = if s.paused {
                ("Paused", t.agent_blocked)
            } else if s.current.is_some() {
                ("Running", t.agent_working)
            } else if s.awaiting.is_some() {
                ("Awaiting", t.agent_blocked)
            } else if !s.pending.is_empty() {
                ("Ready", t.agent_idle)
            } else {
                ("Idle", t.agent_idle)
            };
            Line::from(vec![
                Span::styled(
                    if s.mode.gate { "Manual" } else { "Auto" },
                    Style::default().fg(t.muted),
                ),
                Span::raw(" · "),
                Span::styled(state, emphasis(color)),
                Span::raw(" · "),
                Span::styled(
                    if s.mode.r#loop { "Loop on" } else { "Loop off" },
                    if s.mode.r#loop {
                        emphasis(t.agent_idle)
                    } else {
                        Style::default().fg(t.dim)
                    },
                ),
            ])
        } else {
            Line::styled("Loading tasks…", emphasis(t.agent_starting))
        };
        frame.render_widget(
            Paragraph::new(mode),
            Rect::new(
                inside.x,
                inside.y + (header_height - 1).min(inside.height - 1),
                inside.width,
                1,
            ),
        );
        let used = header_height.min(inside.height);
        let remaining = Rect::new(
            inside.x,
            inside.y + used,
            inside.width,
            inside.height - used,
        );
        let (remaining, action_hits) = if self.read_error.is_some() {
            (remaining, Vec::new())
        } else {
            buttons::draw_compact_top(
                t,
                frame,
                remaining,
                &[
                    B::new("Go g", K::Char('g'), ready).primary(),
                    B::new("Next n", K::Char('n'), ready),
                    B::new(
                        if self.snapshot.as_ref().is_some_and(|s| s.paused) {
                            "Resume p"
                        } else {
                            "Pause p"
                        },
                        K::Char('p'),
                        ready,
                    ),
                    B::new("Loop l", K::Char('l'), ready),
                ],
            )
        };
        self.buttons.extend(action_hits);
        let (mut body, task_hits) = buttons::draw_compact(
            t,
            frame,
            remaining,
            &[
                B::new("Details ↵", K::Enter, ready && !self.tasks().is_empty()),
                B::new("Add a", K::Char('a'), ready),
                B::new("Help ?", K::Char('?'), !self.overlay_open()),
            ],
        );
        self.buttons.extend(task_hits);
        if body.height > 0 {
            frame.render_widget(
                Paragraph::new(if self.busy {
                    "─ Running action…"
                } else {
                    "─ Tasks ─────────────"
                })
                .style(Style::default().fg(if self.busy {
                    t.agent_working
                } else {
                    t.border
                })),
                Rect::new(body.x, body.y, body.width, 1),
            );
            body.y += 1;
            body.height -= 1;
        }
        let hits = self.draw_page(t, frame, body, focused, true);
        if self.overlay_open() {
            Vec::new()
        } else {
            hits
        }
    }
    pub fn draw_overlay(&mut self, t: &Theme, frame: &mut ratatui::Frame) {
        if !self.overlay_open() {
            return;
        }
        use crate::buttons;
        use ratatui::{
            layout::Rect,
            style::Style,
            widgets::{Clear, Paragraph},
        };
        let title = match self.page {
            Page::Projects => " Projects ",
            Page::Project(_) => " Project path ",
            Page::Add { .. } => " Add task ",
            Page::Detail => " Task details ",
            Page::Help => " Help ",
            Page::Feedback(_) => " Action result ",
            Page::List => return,
        };
        let height = if matches!(self.page, Page::Projects | Page::Project(_)) {
            20
        } else {
            28
        };
        let area = theme::centered(frame.area(), 76, height);
        frame.render_widget(Clear, area);
        let block = t.block(title, true).style(t.base().bg(t.overlay));
        let inside = block.inner(area);
        frame.render_widget(block, area);
        self.buttons.clear();
        self.fields.clear();
        self.project_rows.clear();
        let (mut body, hits) = buttons::draw_compact(t, frame, inside, &self.controls());
        self.buttons = hits;
        if !self.message.is_empty() && body.height > 3 {
            let lines = wrap_text(&self.message, body.width);
            let height = (lines.len() as u16).min(body.height / 3).max(1);
            frame.render_widget(
                Paragraph::new(lines).style(Style::default().fg(self.message_color(t))),
                Rect::new(body.x, body.bottom() - height, body.width, height),
            );
            body.height -= height;
        }
        self.draw_page(t, frame, body, true, false);
    }
    fn draw_page(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        mut body: ratatui::layout::Rect,
        focused: bool,
        list: bool,
    ) -> Vec<(u16, usize)> {
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Paragraph, Wrap},
        };
        let mut hits = Vec::new();
        let page = if list { &Page::List } else { &self.page };
        match page {
            Page::List => {
                self.list_area = body;
                if let Some(error) = &self.read_error {
                    let text = format!(
                        "{error}\n\nCheck queue.cwd or choose Project.\nScroll to read the full error.\n\nDirectory: {}",
                        self.project
                    );
                    let lines = wrap_text(&text, body.width);
                    self.scroll = self
                        .scroll
                        .min(lines.len().saturating_sub(usize::from(body.height)));
                    frame.render_widget(
                        Paragraph::new(lines)
                            .scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                        body,
                    );
                    return hits;
                }
                let tasks = self.tasks();
                if tasks.is_empty() {
                    frame.render_widget(
                        Paragraph::new(if self.snapshot.is_some() {
                            "No active tasks · Add a\n\nHistory · 0\nNo history yet"
                        } else {
                            "Loading tasks…"
                        })
                        .wrap(Wrap { trim: false }),
                        body,
                    );
                    return hits;
                }
                let text_width = body.width.saturating_sub(1);
                let mut rows = Vec::new();
                if let Some(s) = &self.snapshot
                    && s.current.is_none()
                    && s.awaiting.is_none()
                    && s.pending.is_empty()
                {
                    rows.push((
                        None,
                        Line::styled("No active tasks · Add a", Style::default().fg(t.muted)),
                    ));
                    rows.push((None, Line::raw("")));
                }
                let mut section = "";
                for (index, (group, task)) in tasks.iter().enumerate() {
                    if section != *group {
                        // History mixes outcomes, so only its rows carry status colors.
                        let color = match *group {
                            "Current" => t.agent_working,
                            "Awaiting" => t.agent_blocked,
                            "Pending" => t.agent_starting,
                            _ => t.muted,
                        };
                        rows.push((
                            None,
                            Line::styled(
                                format!(
                                    "{group} {}",
                                    tasks.iter().filter(|(g, _)| g == group).count()
                                ),
                                Style::default().fg(color).add_modifier(Modifier::BOLD),
                            ),
                        ));
                        section = group;
                    }
                    let style = if index == self.selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let (status, color) = match *group {
                        "Current" => ("Running", t.agent_working),
                        "Awaiting" => ("Awaiting", t.agent_blocked),
                        "Pending" => ("Pending", t.agent_starting),
                        _ => match task.status.as_deref() {
                            Some("done") => ("Done", t.agent_idle),
                            Some("failed") => ("Failed", t.agent_error),
                            Some("dropped" | "drop") => ("Dropped", t.agent_stalled),
                            Some(s) => (s, t.muted),
                            None => ("—", t.dim),
                        },
                    };
                    let status_width = unicode_width::UnicodeWidthStr::width(status) + 1;
                    let id = task.id.as_deref().unwrap_or("·");
                    let id_width = unicode_width::UnicodeWidthStr::width(id).min(8);
                    let title_width =
                        usize::from(text_width).saturating_sub(id_width + status_width + 3);
                    let spans = vec![
                        Span::styled(
                            if index == self.selected { "▎" } else { " " },
                            Style::default().fg(if focused { t.focus } else { t.muted }),
                        ),
                        Span::styled(crate::ui::clip(id, id_width), Style::default().fg(t.muted)),
                        Span::raw(" "),
                        Span::raw(crate::ui::pad(
                            &crate::ui::clip(&task.title, title_width),
                            title_width,
                        )),
                        Span::styled(
                            format!(" {status}"),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                    ];
                    rows.push((Some(index), Line::from(spans).style(style)));
                }
                let history_start = tasks.iter().position(|(group, _)| *group == "History");
                let history_total = tasks.len() - history_start.unwrap_or(tasks.len());
                let history_footer = history_start.filter(|_| body.height > 1).map(|_| {
                    body.height -= 1;
                    Rect::new(body.x, body.bottom(), body.width, 1)
                });
                let selected_row = rows
                    .iter()
                    .position(|(i, _)| *i == Some(self.selected))
                    .unwrap_or(0);
                let height = usize::from(body.height);
                if !self.manual_scroll {
                    self.top = self
                        .top
                        .max((selected_row + 1).saturating_sub(height))
                        .min(selected_row);
                }
                self.top = self.top.min(rows.len().saturating_sub(height));
                if let Some(footer) = history_footer {
                    let start = history_start.unwrap();
                    let total = history_total;
                    let visible: Vec<_> = rows
                        .iter()
                        .skip(self.top)
                        .take(height)
                        .filter_map(|(index, _)| {
                            index.filter(|i| *i >= start).map(|i| i - start + 1)
                        })
                        .collect();
                    let label = match (visible.first(), visible.last()) {
                        (Some(first), Some(last)) => format!(
                            " History {first}–{last}/{total}{} ",
                            if *last == total { " · End" } else { "" }
                        ),
                        _ => format!(" History {total} below "),
                    };
                    frame.render_widget(
                        Block::new()
                            .borders(ratatui::widgets::Borders::TOP)
                            .border_style(Style::default().fg(t.border))
                            .title(Line::styled(label, Style::default().fg(t.muted))),
                        footer,
                    );
                }
                for (i, (index, line)) in rows.iter().skip(self.top).take(height).enumerate() {
                    let y = body.y + i as u16;
                    frame.render_widget(
                        Paragraph::new(line.clone()).style(line.style),
                        Rect::new(body.x, y, text_width, 1),
                    );
                    if let Some(index) = index {
                        hits.push((y, *index));
                    }
                }
                if rows.len() > height && height > 0 {
                    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
                    frame.render_stateful_widget(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(None)
                            .end_symbol(None)
                            .thumb_style(Style::default().fg(t.muted))
                            .track_style(Style::default().fg(t.border)),
                        body,
                        &mut ScrollbarState::new(rows.len().saturating_sub(height) + 1)
                            .viewport_content_length(height)
                            .position(self.top),
                    );
                }
            }
            Page::Projects => {
                let mut lines = Vec::new();
                if let Some(error) = &self.registry_error {
                    lines.extend(
                        wrap_text(error, body.width)
                            .into_iter()
                            .map(|l| (None, l.to_string())),
                    );
                }
                for (index, path) in self.projects.iter().enumerate() {
                    let name = std::path::Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let name = crate::ui::clip(&name, 16);
                    let label = format!(
                        "{} {} {}",
                        if index == self.project_selected {
                            "▎"
                        } else {
                            " "
                        },
                        crate::ui::pad(&name, 16),
                        crate::ui::clip(path, usize::from(body.width).saturating_sub(20))
                    );
                    lines.push((Some(index), label));
                }
                if self.projects.is_empty() {
                    lines.push((None, "No registered projects · Choose Path".into()));
                }
                let footer_height = body.height.min(3);
                let height = usize::from(body.height - footer_height);
                let selected = lines
                    .iter()
                    .position(|(i, _)| *i == Some(self.project_selected))
                    .unwrap_or(0);
                let top = selected.saturating_sub(height.saturating_sub(1));
                for (offset, (index, line)) in lines.iter().skip(top).take(height).enumerate() {
                    let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
                    frame.render_widget(
                        Paragraph::new(line.as_str()).style(
                            if *index == Some(self.project_selected) {
                                Style::default().bg(t.selected)
                            } else {
                                Style::default()
                            },
                        ),
                        row,
                    );
                    if let Some(index) = index {
                        self.project_rows.push((row, *index));
                    }
                }
                let path = self
                    .projects
                    .get(self.project_selected)
                    .map(String::as_str)
                    .unwrap_or(&self.project);
                frame.render_widget(
                    Paragraph::new(format!("{}\nClick / Enter to open · e Set path", path))
                        .wrap(Wrap { trim: false })
                        .style(Style::default().fg(t.muted)),
                    Rect::new(
                        body.x,
                        body.bottom() - footer_height,
                        body.width,
                        footer_height,
                    ),
                );
            }
            Page::Project(path) => {
                let field = Block::bordered().title("Project path · Enter Apply · Esc Cancel");
                let field_area = Rect::new(body.x, body.y, body.width, body.height.min(3));
                let inner = field.inner(field_area);
                frame.render_widget(field, field_area);
                let width = unicode_width::UnicodeWidthStr::width(path.as_str()) as u16;
                frame.render_widget(
                    Paragraph::new(path.as_str())
                        .scroll((0, width.saturating_sub(inner.width.saturating_sub(1)))),
                    inner,
                );
                if focused && !inner.is_empty() {
                    frame.set_cursor_position((inner.x + width.min(inner.width - 1), inner.y));
                }
                if body.height > 3 {
                    frame.render_widget(Paragraph::new("Enter a project registered with drover. Ctrl-U clears.\nFor this session only; set queue.cwd for a default.").wrap(Wrap { trim: false }), Rect::new(body.x, body.y + 3, body.width, body.height - 3));
                }
            }
            Page::Add {
                title,
                body: text,
                body_focus,
            } => {
                if body.height < 5 {
                    frame.render_widget(Paragraph::new("Enlarge the window to edit a task"), body);
                    return hits;
                }
                let title_area = Rect::new(body.x, body.y, body.width, 3);
                let text_area = Rect::new(body.x, body.y + 3, body.width, body.height - 3);
                self.fields = vec![(title_area, false), (text_area, true)];
                let title_block = Block::bordered().title("Title").border_style(
                    Style::default().fg(if !body_focus { t.focus } else { t.border }),
                );
                let text_block = Block::bordered()
                    .title("Body · Tab Switch · Ctrl-S Save · Esc Cancel")
                    .border_style(Style::default().fg(if *body_focus {
                        t.focus
                    } else {
                        t.border
                    }));
                let title_inner = title_block.inner(title_area);
                let text_inner = text_block.inner(text_area);
                frame.render_widget(title_block, title_area);
                frame.render_widget(text_block, text_area);
                let title_width = unicode_width::UnicodeWidthStr::width(title.as_str()) as u16;
                frame.render_widget(
                    Paragraph::new(title.as_str()).scroll((
                        0,
                        title_width.saturating_sub(title_inner.width.saturating_sub(1)),
                    )),
                    title_inner,
                );
                let wrapped = wrap_text(text, text_inner.width);
                let lines = wrapped.len();
                let paragraph = Paragraph::new(wrapped);
                let scroll = lines.saturating_sub(usize::from(text_inner.height));
                frame.render_widget(
                    paragraph.scroll((scroll.min(u16::MAX as usize) as u16, 0)),
                    text_inner,
                );
                if focused && !self.busy {
                    if *body_focus && !text_inner.is_empty() {
                        let last_width = unicode_width::UnicodeWidthStr::width(
                            text.rsplit('\n').next().unwrap_or(""),
                        );
                        frame.set_cursor_position((
                            text_inner.x + (last_width % usize::from(text_inner.width)) as u16,
                            text_inner.y
                                + (lines.saturating_sub(1 + scroll) as u16)
                                    .min(text_inner.height - 1),
                        ));
                    } else if !title_inner.is_empty() {
                        frame.set_cursor_position((
                            title_inner.x + title_width.min(title_inner.width - 1),
                            title_inner.y,
                        ));
                    }
                }
            }
            _ => {
                let text=match &self.page {
                    Page::Detail=>self.tasks().get(self.selected).map(|(group,t)|format!("{} · {} {}\n\n{}{}",group,t.id.as_deref().unwrap_or(""),t.title,t.body,t.reason.as_ref().map(|r|format!("\n\nReason: {r}")).unwrap_or_default())).unwrap_or_else(||"Task no longer in queue".into()),
                    Page::Help=>"Queue help\nTop actions control the project; bottom actions control tasks.\nc: Projects; e: Set path (in Projects)\nWheel / trackpad: Scroll the task list\nUp/Down / j k: Select task or project\nEnter: Details; Esc: Back\nPgUp/PgDn: Scroll details / results\nr: Refresh; g: Check and release\nn: Send next task\np: Pause / Resume; l: Toggle loop\na: Add task\nTab: Switch field; Ctrl-S: Save\nq / Ctrl-]: Return to Agents\n\nGo / Next / Pause / Loop apply to the project,\nregardless of the selected history task.".into(),
                    Page::Feedback(text)=>text.clone(),
                    _=>unreachable!(),
                };
                let wrapped = wrap_text(&text, body.width);
                self.scroll = self
                    .scroll
                    .min(wrapped.len().saturating_sub(usize::from(body.height)));
                let paragraph =
                    Paragraph::new(wrapped).style(if matches!(self.page, Page::Feedback(_)) {
                        Style::default().fg(self.message_color(t))
                    } else {
                        Style::default()
                    });
                frame.render_widget(
                    paragraph.scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                    body,
                );
            }
        }
        hits
    }
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

fn wrap_text(text: &str, width: u16) -> Vec<ratatui::text::Line<'static>> {
    use unicode_width::UnicodeWidthChar;
    if width == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for c in clean(text).replace('\t', "    ").chars() {
        if c == '\n' {
            rows.push(ratatui::text::Line::raw(std::mem::take(&mut line)));
            used = 0;
            continue;
        }
        let w = c.width().unwrap_or(0);
        if used + w > usize::from(width) && !line.is_empty() {
            rows.push(ratatui::text::Line::raw(std::mem::take(&mut line)));
            used = 0;
        }
        line.push(c);
        used += w;
    }
    rows.push(ratatui::text::Line::raw(line));
    rows
}
