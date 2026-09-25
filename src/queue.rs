use crate::drover::{Operation, Request, Snapshot, Task};
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
    pub scroll: usize,
    pub page: Page,
    pub message: String,
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
                    "保存 ^S",
                    K::Char('s'),
                    !self.busy && self.read_error.is_none(),
                ),
                B::new("取消 Esc", K::Esc, !self.busy),
            ],
            Page::Project(_) => vec![
                B::new("应用 Enter", K::Enter, !self.busy),
                B::new("取消 Esc", K::Esc, true),
            ],
            Page::Projects => vec![
                B::new("打开 Enter", K::Enter, !self.projects.is_empty()),
                B::new("刷新 r", K::Char('r'), true),
                B::new("目录 e", K::Char('e'), true),
                B::new("返回 Esc", K::Esc, true),
            ],
            _ => {
                let ready = !self.busy && self.snapshot.is_some() && self.read_error.is_none();
                let mut buttons = vec![
                    B::new("项目 c", K::Char('c'), !self.busy),
                    B::new("刷新 r", K::Char('r'), !self.busy),
                ];
                if matches!(self.page, Page::List) {
                    buttons.push(B::new(
                        "详情 Enter",
                        K::Enter,
                        ready && !self.tasks().is_empty(),
                    ));
                } else {
                    buttons.push(B::new("返回 Esc", K::Esc, true));
                }
                buttons.extend([
                    B::new("新增 a", K::Char('a'), ready),
                    B::new("放行 g", K::Char('g'), ready),
                    B::new("下一件 n", K::Char('n'), ready),
                    B::new(
                        if self.snapshot.as_ref().is_some_and(|s| s.paused) {
                            "恢复 p"
                        } else {
                            "暂停 p"
                        },
                        K::Char('p'),
                        ready,
                    ),
                    B::new(
                        if self.snapshot.as_ref().is_some_and(|s| s.mode.r#loop) {
                            "关闭循环 l"
                        } else {
                            "开启循环 l"
                        },
                        K::Char('l'),
                        ready,
                    ),
                    B::new("帮助 ?", K::Char('?'), true),
                ]);
                buttons
            }
        }
    }
    pub fn tasks(&self) -> Vec<(&'static str, &Task)> {
        let Some(s) = &self.snapshot else {
            return Vec::new();
        };
        s.current
            .iter()
            .map(|t| ("当前", t))
            .chain(s.awaiting.iter().map(|t| ("待放行", t)))
            .chain(s.pending.iter().map(|t| ("待办", t)))
            .chain(s.history.iter().rev().map(|t| ("历史", t)))
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
                self.message = text.clone();
                if matches!(operation, Operation::Go | Operation::Next) {
                    self.page = Page::Feedback(text);
                    self.scroll = 0;
                } else if matches!(operation, Operation::Add { .. }) {
                    self.page = Page::List;
                }
            }
            Err(error) => {
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
                        self.message = "标题不能为空".into();
                        return None;
                    }
                    self.busy = true;
                    self.message = "正在新增…".into();
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
                    self.message = "等待队列数据，操作未执行".into();
                    return None;
                };
                let operation = match c {
                    'g' => Operation::Go,
                    'n' => Operation::Next,
                    'p' => Operation::Pause(!snapshot.paused),
                    _ => Operation::Loop(!snapshot.mode.r#loop),
                };
                self.busy = true;
                self.message = "正在执行…".into();
                return Some(Request::Run(operation));
            }
            _ => {}
        }
        None
    }
}

impl Panel {
    pub fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        focused: bool,
    ) -> Vec<(u16, usize)> {
        use ratatui::{
            layout::Rect,
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Paragraph, Wrap},
        };
        let block = Block::bordered()
            .title("Queue")
            .border_style(Style::default().fg(if focused {
                Color::Cyan
            } else {
                Color::DarkGray
            }));
        let inside = block.inner(area);
        self.buttons.clear();
        self.fields.clear();
        self.project_rows.clear();
        frame.render_widget(block, area);
        if inside.height < 2 || inside.width == 0 {
            return Vec::new();
        }
        let mode = self
            .snapshot
            .as_ref()
            .map(|s| {
                format!(
                    "{} · loop {}",
                    if s.paused {
                        "Paused"
                    } else if s.mode.gate {
                        "Manual"
                    } else {
                        "Auto"
                    },
                    if s.mode.r#loop { "on" } else { "off" }
                )
            })
            .unwrap_or_else(|| "正在读取队列…".into());
        let mode = if self.read_error.is_some() {
            "读取失败".to_string()
        } else {
            mode
        };
        frame.render_widget(
            Paragraph::new(mode).style(Style::default().fg(Color::Cyan)),
            Rect::new(inside.x, inside.y, inside.width, 1),
        );
        let project = Rect::new(inside.x, inside.y + 1, inside.width, 1);
        if inside.height > 2 {
            let name = std::path::Path::new(&self.project)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            frame.render_widget(
                Paragraph::new(format!("项目：{name} · {}", self.project)),
                project,
            );
        }
        let (content, buttons) = crate::buttons::draw(
            frame,
            Rect {
                height: inside.height.saturating_sub(1),
                ..inside
            },
            &self.controls(),
        );
        self.buttons = buttons;
        let body = Rect::new(
            content.x,
            content.y + 2,
            content.width,
            content.height.saturating_sub(2),
        );
        let footer = Rect::new(inside.x, inside.bottom() - 1, inside.width, 1);
        let footer_text = if self.message.is_empty() {
            "滚轮选择 · PgUp/PgDn 滚动"
        } else {
            &self.message
        };
        frame.render_widget(
            Paragraph::new(clean(footer_text)).style(Style::default().fg(Color::Yellow)),
            footer,
        );
        let mut hits = Vec::new();
        match &self.page {
            Page::List => {
                if let Some(error) = &self.read_error {
                    let text = format!(
                        "{error}\n\n检查 queue.cwd，或点击项目按钮切换。\nPgUp/PgDn 滚动完整错误。\n\n当前目录：{}",
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
                            "队列为空 · a 新增任务"
                        } else {
                            "正在通过公开 CLI 读取任务…"
                        })
                        .wrap(Wrap { trim: false }),
                        body,
                    );
                    return hits;
                }
                let mut rows = Vec::new();
                let mut section = "";
                for (index, (group, task)) in tasks.iter().enumerate() {
                    if section != *group {
                        rows.push((
                            None,
                            Line::styled(group.to_string(), Style::default().fg(Color::Cyan)),
                        ));
                        section = group;
                    }
                    let style = if index == self.selected {
                        Style::default()
                            .bg(Color::Indexed(237))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let label = format!(
                        "{}{} {}{}",
                        if index == self.selected { "▎" } else { " " },
                        task.id.as_deref().unwrap_or("·"),
                        task.title,
                        task.status
                            .as_ref()
                            .map(|s| format!(" [{s}]"))
                            .unwrap_or_default()
                    );
                    rows.push((Some(index), Line::styled(clean(&label), style)));
                }
                let selected_row = rows
                    .iter()
                    .position(|(i, _)| *i == Some(self.selected))
                    .unwrap_or(0);
                let height = usize::from(body.height);
                self.top = self
                    .top
                    .max((selected_row + 1).saturating_sub(height))
                    .min(selected_row)
                    .min(rows.len().saturating_sub(height));
                for (i, (index, line)) in rows.iter().skip(self.top).take(height).enumerate() {
                    let y = body.y + i as u16;
                    frame.render_widget(
                        Paragraph::new(line.clone()),
                        Rect::new(body.x, y, body.width, 1),
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
                            .end_symbol(None),
                        body,
                        &mut ScrollbarState::new(rows.len() - height).position(self.top),
                    );
                }
            }
            Page::Projects => {
                let mut lines = vec![(None, "选择项目 · 点击或 Enter 打开".to_string())];
                if let Some(error) = &self.registry_error {
                    lines.extend(
                        wrap_text(error, body.width)
                            .into_iter()
                            .map(|line| (None, line.to_string())),
                    );
                }
                for (index, path) in self.projects.iter().enumerate() {
                    let name = std::path::Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    lines.push((
                        Some(index),
                        format!(
                            "{} {}",
                            if index == self.project_selected {
                                "▎"
                            } else {
                                " "
                            },
                            name
                        ),
                    ));
                    lines.extend(
                        wrap_text(path, body.width.saturating_sub(2))
                            .into_iter()
                            .map(|line| (Some(index), format!("  {line}"))),
                    );
                }
                if self.projects.is_empty() {
                    lines.push((None, "未登记项目 · 点击目录手动指定".into()));
                }
                let selected = lines
                    .iter()
                    .position(|(i, _)| *i == Some(self.project_selected))
                    .unwrap_or(0);
                let top = selected.saturating_sub(usize::from(body.height.saturating_sub(2)));
                for (offset, (index, line)) in lines
                    .iter()
                    .skip(top)
                    .take(usize::from(body.height))
                    .enumerate()
                {
                    let row = Rect::new(body.x, body.y + offset as u16, body.width, 1);
                    let style = if *index == Some(self.project_selected) {
                        Style::default().bg(Color::Indexed(237))
                    } else {
                        Style::default()
                    };
                    frame.render_widget(Paragraph::new(line.as_str()).style(style), row);
                    if let Some(index) = index {
                        self.project_rows.push((row, *index));
                    }
                }
            }
            Page::Project(path) => {
                let field = Block::bordered().title("项目目录 · Enter 应用 · Esc 取消");
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
                    frame.render_widget(Paragraph::new("输入已接入 drover 的目录。Ctrl-U 清空。\n仅本次运行生效；长期默认请设置 queue.cwd。").wrap(Wrap { trim: false }), Rect::new(body.x, body.y + 3, body.width, body.height - 3));
                }
            }
            Page::Add {
                title,
                body: text,
                body_focus,
            } => {
                if body.height < 5 {
                    frame.render_widget(Paragraph::new("请增大窗口以编辑任务"), body);
                    return hits;
                }
                let title_area = Rect::new(body.x, body.y, body.width, 3);
                let text_area = Rect::new(body.x, body.y + 3, body.width, body.height - 3);
                self.fields = vec![(title_area, false), (text_area, true)];
                let title_block =
                    Block::bordered()
                        .title("标题")
                        .border_style(Style::default().fg(if !body_focus {
                            Color::Cyan
                        } else {
                            Color::DarkGray
                        }));
                let text_block = Block::bordered()
                    .title("正文 · Tab 切换 · Ctrl-S 保存 · Esc 取消")
                    .border_style(Style::default().fg(if *body_focus {
                        Color::Cyan
                    } else {
                        Color::DarkGray
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
                    Page::Detail=>self.tasks().get(self.selected).map(|(group,t)|format!("{} · {} {}\n\n{}{}",group,t.id.as_deref().unwrap_or(""),t.title,t.body,t.reason.as_ref().map(|r|format!("\n\n原因：{r}")).unwrap_or_default())).unwrap_or_else(||"任务已移出队列".into()),
                    Page::Help=>"Queue 原生看板\n点击底部按钮执行操作\nc：已登记项目；e：手动目录（项目页）\n↑↓ / j k：选择任务或项目\nEnter：任务详情；Esc：列表\nPgUp/PgDn：滚动详情/反馈\nr：刷新；g：核对并放行\nn：发送下一件\np：暂停/恢复；l：循环开/关\na：新增任务（原生表单）\n新增时 Tab 切字段、Ctrl-S 提交\nq / Ctrl-]：回 Agents\n\n只调用公开 drover CLI。\n操作不会启动外部看板。".into(),
                    Page::Feedback(text)=>text.clone(),
                    _=>unreachable!(),
                };
                let wrapped = wrap_text(&text, body.width);
                self.scroll = self
                    .scroll
                    .min(wrapped.len().saturating_sub(usize::from(body.height)));
                let paragraph = Paragraph::new(wrapped);
                frame.render_widget(
                    paragraph.scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                    body,
                );
            }
        }
        // Keep an unobtrusive native indicator visible while a public CLI runs.
        if self.busy {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "执行中…",
                    Style::default().fg(Color::Yellow),
                ))),
                footer,
            );
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
