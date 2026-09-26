use crate::{
    buttons::{self, Button},
    config::expand_home,
    terminals::{Place, Ticket},
    theme::Theme,
};
use anyhow::{Context, Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph},
};

#[path = "launch_edit.rs"]
mod edit;

const PROJECT: usize = 5;
const CODEX: usize = 6;
const CLAUDE: usize = 7;
const ADVANCED: usize = 8;
const PREVIEW: usize = 9;

pub struct Form {
    fields: [edit::Input; 4],
    field: usize,
    pub place: usize,
    pub busy: Option<Ticket>,
    pub error: String,
    pub visible: bool,
    preview_top: u16,
    field_hits: Vec<(Rect, usize)>,
    advanced: bool,
    edit_path: bool,
    manual_name: bool,
    agent: usize,
    choosing_project: bool,
    project_index: usize,
    project_hits: Vec<(Rect, usize)>,
    scroll_start: usize,
}
impl Form {
    pub fn new(project: String) -> Self {
        let mut form = Self {
            fields: [project, String::new(), "codex".into(), String::new()].map(edit::Input::new),
            field: PROJECT,
            place: 1,
            busy: None,
            error: String::new(),
            visible: true,
            preview_top: 0,
            field_hits: Vec::new(),
            advanced: false,
            edit_path: false,
            manual_name: false,
            agent: 0,
            choosing_project: false,
            project_index: 0,
            project_hits: Vec::new(),
            scroll_start: 0,
        };
        form.suggest_name();
        form
    }
    fn suggest_name(&mut self) {
        if self.manual_name {
            return;
        }
        let path = expand_home(&self.fields[0].text);
        let project = path.file_name().unwrap_or_default().to_string_lossy();
        let name: String = project
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let name = name.trim_matches('-');
        let name = if name.is_empty() { "project" } else { name };
        self.fields[1] = edit::Input::new(format!("{name}/{}", ["codex", "claude"][self.agent]));
    }
    fn invalid(&self, field: usize) -> Option<String> {
        let value = &self.fields[field].text;
        if value.contains('\0') {
            return Some("NUL bytes are not allowed".into());
        }
        match field {
            0 if value.trim().is_empty() => Some("Directory is required".into()),
            1 if value.trim().is_empty()
                || value.starts_with('-')
                || value.chars().any(char::is_whitespace) =>
            {
                Some("Name needs text, no spaces or leading '-'".into())
            }
            2 => match shell_words::split(value) {
                Err(_) => Some("Command has an unclosed quote".into()),
                Ok(words) if words.first().is_none_or(|w| w.is_empty()) => {
                    Some("Command is required".into())
                }
                _ => None,
            },
            _ => None,
        }
    }
    pub fn reveal_invalid(&mut self) {
        if let Some(field) = (0..4).find(|i| self.invalid(*i).is_some()) {
            self.field = field;
            if field == 0 {
                self.edit_path = true;
            }
            if field >= 2 {
                self.advanced = true;
            }
        }
    }
    pub fn args(&self) -> Result<Vec<String>> {
        for field in 0..4 {
            if let Some(error) = self.invalid(field) {
                bail!("{error}");
            }
        }
        let [cwd, name, command, prompt] = self.fields.each_ref().map(|f| &f.text);
        let words = shell_words::split(command).context("Command has an unclosed quote")?;
        let mut args = vec![
            "start".into(),
            name.clone(),
            "--cwd".into(),
            expand_home(cwd).to_string_lossy().into_owned(),
        ];
        if !self.manual_name {
            args.push("--unique".into());
        }
        if !prompt.is_empty() {
            args.extend(["--prompt".into(), prompt.clone()]);
        }
        args.push("--".into());
        args.extend(words);
        Ok(args)
    }
    fn edited(&mut self, old: &str) {
        if self.field < 4 && old != self.fields[self.field].text {
            if self.field == 1 {
                self.manual_name = true;
            }
            if self.field == 0 {
                self.suggest_name();
            }
            self.error.clear();
        }
    }
    pub fn paste(&mut self, text: &str) {
        if self.busy.is_none() && !self.choosing_project && self.field < 4 {
            let old = self.fields[self.field].text.clone();
            self.fields[self.field].insert(text, self.field == 3);
            self.edited(&old);
        }
    }
    fn focus_order(&self) -> Vec<usize> {
        let mut order = vec![PROJECT];
        if self.edit_path {
            order.push(0);
        }
        order.extend([CODEX, CLAUDE, 1, ADVANCED]);
        if self.advanced {
            order.extend([2, 3, 4, PREVIEW]);
        }
        order
    }
    fn move_focus(&mut self, back: bool) {
        let order = self.focus_order();
        let i = order.iter().position(|f| *f == self.field).unwrap_or(0);
        self.field = order[(i + if back { order.len() - 1 } else { 1 }) % order.len()];
    }
    fn choose_agent(&mut self, agent: usize) {
        self.agent = agent;
        self.field = CODEX + agent;
        self.fields[2] = edit::Input::new(["codex", "claude"][agent].into());
        self.suggest_name();
        self.error.clear();
    }
    fn select_project(&mut self, projects: &[String]) {
        if let Some(project) = projects.get(self.project_index) {
            self.fields[0] = edit::Input::new(project.clone());
            self.suggest_name();
            self.error.clear();
        }
        self.choosing_project = false;
        self.field = PROJECT;
    }
    pub fn label(&self) -> &str {
        if self.choosing_project {
            return "Choose project";
        }
        [
            "Directory",
            "Name",
            "Command",
            "First message",
            "Open in",
            "Project",
            "Codex",
            "Claude",
            "Advanced",
            "Preview",
        ][self.field]
    }
    /// Only the Create button / Ctrl-S submits; editing and navigation never do.
    pub fn key(&mut self, key: KeyEvent, projects: &[String]) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.code == KeyCode::Esc || (ctrl && matches!(key.code, KeyCode::Char(']' | '5'))) {
            if self.choosing_project && key.code == KeyCode::Esc {
                self.choosing_project = false;
            } else {
                self.choosing_project = false;
                self.visible = false;
            }
            return false;
        }
        if self.busy.is_some() {
            return false;
        }
        if ctrl && key.code == KeyCode::Char('e') {
            self.choosing_project = false;
            self.edit_path = true;
            self.field = 0;
            return false;
        }
        if self.choosing_project {
            match key.code {
                KeyCode::Up | KeyCode::BackTab => {
                    self.project_index = self.project_index.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Tab => {
                    self.project_index =
                        (self.project_index + 1).min(projects.len().saturating_sub(1))
                }
                KeyCode::Enter => self.select_project(projects),
                _ => {}
            }
            return false;
        }
        if ctrl {
            match key.code {
                KeyCode::Char('s') => return true,
                KeyCode::Char('p') => {
                    self.project_index = projects
                        .iter()
                        .position(|p| p == &self.fields[0].text)
                        .unwrap_or(0);
                    self.choosing_project = true;
                    self.field = PROJECT;
                }
                KeyCode::Char('u') if self.field < 4 => {
                    let old = self.fields[self.field].text.clone();
                    self.fields[self.field].clear();
                    self.edited(&old);
                }
                _ => {}
            }
            return false;
        }
        // Alt/Super combinations must not insert their printable key.
        if key
            .modifiers
            .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return false;
        }
        match key.code {
            KeyCode::Tab => self.move_focus(false),
            KeyCode::BackTab => self.move_focus(true),
            KeyCode::F(2) => self.choose_agent(0),
            KeyCode::F(3) => self.choose_agent(1),
            KeyCode::F(4) => {
                self.advanced = !self.advanced;
                self.field = ADVANCED;
            }
            KeyCode::F(5) if self.advanced => {
                self.place = (self.place + 1) % 6;
                self.field = 4;
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.field == PROJECT => {
                return self.key(
                    KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
                    projects,
                );
            }
            KeyCode::Left | KeyCode::Right if matches!(self.field, CODEX | CLAUDE) => {
                self.choose_agent(1 - self.agent)
            }
            KeyCode::Enter | KeyCode::Char(' ') if matches!(self.field, CODEX | CLAUDE) => {
                self.choose_agent(self.field - CODEX)
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.field == ADVANCED => {
                self.advanced = !self.advanced;
            }
            KeyCode::Left if self.field == 4 => self.place = (self.place + 5) % 6,
            KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') if self.field == 4 => {
                self.place = (self.place + 1) % 6
            }
            KeyCode::PageUp | KeyCode::PageDown if self.advanced => {
                self.field = PREVIEW;
                self.preview_top = if key.code == KeyCode::PageUp {
                    self.preview_top.saturating_sub(3)
                } else {
                    self.preview_top.saturating_add(3)
                };
            }
            _ if self.field < 4 => {
                let old = self.fields[self.field].text.clone();
                self.fields[self.field].key(key.code, self.field == 3);
                self.edited(&old);
            }
            _ => {}
        }
        false
    }
    pub fn click(&mut self, point: ratatui::layout::Position, projects: &[String]) {
        if self.busy.is_some() {
            return;
        }
        if self.choosing_project {
            if let Some((_, i)) = self.project_hits.iter().find(|(r, _)| r.contains(point)) {
                self.project_index = *i;
                self.select_project(projects);
            }
        } else if let Some((_, field)) = self.field_hits.iter().find(|(r, _)| r.contains(point)) {
            self.field = *field;
            if *field < 4 {
                self.fields[*field].click(point);
            }
        }
    }
    pub fn scroll(&mut self, down: bool) {
        if self.busy.is_some() {
            return;
        }
        if self.choosing_project {
            self.project_index = if down {
                self.project_index.saturating_add(1)
            } else {
                self.project_index.saturating_sub(1)
            };
        } else if self.field == PREVIEW {
            self.preview_top = if down {
                self.preview_top.saturating_add(3)
            } else {
                self.preview_top.saturating_sub(3)
            };
        } else {
            self.move_focus(!down);
        }
    }
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        program: &str,
        projects: &[String],
    ) -> Vec<buttons::Hit> {
        let area = crate::theme::centered(frame.area(), 104, if self.advanced { 40 } else { 24 });
        frame.render_widget(Clear, area);
        frame.render_widget(
            t.block(" New agent ", true).style(t.base().bg(t.overlay)),
            area,
        );
        let enabled = self.busy.is_none();
        let (mut body, mut hits) = buttons::draw_compact(
            t,
            frame,
            crate::ui::inner(area),
            &[
                Button::control(
                    if enabled {
                        "Create agent Ctrl-S"
                    } else {
                        "Starting…"
                    },
                    KeyCode::Char('s'),
                    enabled && !self.choosing_project,
                )
                .primary(),
                Button::new(
                    if self.choosing_project {
                        "Back Esc"
                    } else {
                        "Cancel Esc"
                    },
                    KeyCode::Esc,
                    true,
                ),
            ],
        );
        self.field_hits.clear();
        self.project_hits.clear();
        if !self.error.is_empty() && body.height > 0 {
            let lines = crate::queue::wrap_text(&self.error, body.width);
            let height = (lines.len() as u16).min(3).min(body.height / 2);
            let offset = lines
                .len()
                .saturating_sub(height as usize)
                .min(u16::MAX as usize) as u16;
            frame.render_widget(
                Paragraph::new(lines)
                    .scroll((offset, 0))
                    .style(Style::default().fg(t.danger)),
                Rect::new(body.x, body.y, body.width, height),
            );
            body.y += height;
            body.height -= height;
        }
        if self.choosing_project {
            self.project_index = self.project_index.min(projects.len().saturating_sub(1));
            let (mut list, controls) = buttons::draw_outlined_top(
                t,
                frame,
                body,
                &[Button::control(
                    "Edit path Ctrl-E",
                    KeyCode::Char('e'),
                    true,
                )],
            );
            hits.extend(controls);
            if list.height > 0 {
                frame.render_widget(
                    Paragraph::new("Choose project · ↑↓ select · Enter confirm")
                        .style(Style::default().fg(t.muted)),
                    Rect::new(list.x, list.y, list.width, 1),
                );
                list.y += 1;
                list.height -= 1;
            }
            if projects.is_empty() {
                frame.render_widget(
                    Paragraph::new("No registered projects. Use Edit path."),
                    list,
                );
            } else {
                let top = self
                    .project_index
                    .saturating_sub(list.height.saturating_sub(1) as usize);
                for (i, project) in projects
                    .iter()
                    .enumerate()
                    .skip(top)
                    .take(list.height as usize)
                {
                    let rect = Rect::new(list.x, list.y + (i - top) as u16, list.width, 1);
                    let name = std::path::Path::new(project)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    frame.render_widget(
                        Paragraph::new(format!(
                            "{} {name} · {project}",
                            if i == self.project_index { "▸" } else { " " }
                        ))
                        .style(Style::default().fg(t.text).bg(
                            if i == self.project_index {
                                t.selected
                            } else {
                                t.overlay
                            },
                        )),
                        rect,
                    );
                    self.project_hits.push((rect, i));
                }
            }
            return hits;
        }
        // Outlined buttons take three rows.
        let mut sections = vec![(PROJECT, 5)];
        if self.edit_path {
            sections.push((0, 3));
        }
        sections.extend([(CODEX, 4), (1, 4), (ADVANCED, 3)]);
        if self.advanced {
            sections.extend([(2, 4), (3, 6), (4, 4), (PREVIEW, 4)]);
        }
        let focus = sections
            .iter()
            .position(|(id, _)| *id == self.field || (*id == CODEX && self.field == CLAUDE))
            .unwrap_or(0);
        self.scroll_start = self.scroll_start.min(focus);
        while self.scroll_start < focus
            && sections[self.scroll_start..=focus]
                .iter()
                .map(|(_, h)| h)
                .sum::<u16>()
                > body.height
        {
            self.scroll_start += 1;
        }
        if sections.iter().map(|(_, h)| h).sum::<u16>() <= body.height {
            self.scroll_start = 0;
        }
        let mut y = body.y;
        for (id, height) in sections.into_iter().skip(self.scroll_start) {
            let remaining = body.bottom().saturating_sub(y);
            if remaining == 0 {
                break;
            }
            let rect = Rect::new(
                body.x,
                y,
                body.width,
                if id == PREVIEW {
                    remaining
                } else {
                    height.min(remaining)
                },
            );
            match id {
                PROJECT => {
                    let path = &self.fields[0].text;
                    let name = std::path::Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let label = format!(
                        "Project: {} ▾ Ctrl-P",
                        if name.is_empty() {
                            "Choose project"
                        } else {
                            &name
                        }
                    );
                    let (rest, controls) = buttons::draw_outlined_top(
                        t,
                        frame,
                        rect,
                        &[
                            Button::control(&label, KeyCode::Char('p'), enabled),
                            Button::control("Edit path Ctrl-E", KeyCode::Char('e'), enabled),
                        ],
                    );
                    hits.extend(controls);
                    frame.render_widget(
                        Paragraph::new(if path.is_empty() {
                            "Directory is required — choose a project or edit path"
                        } else {
                            path
                        })
                        .style(Style::default().fg(if path.is_empty() {
                            t.danger
                        } else {
                            t.muted
                        })),
                        rest,
                    );
                }
                CODEX => {
                    let command = &self.fields[2].text;
                    let codex = if command == "codex" {
                        "● Codex"
                    } else {
                        "○ Codex"
                    };
                    let claude = if command == "claude" {
                        "● Claude"
                    } else {
                        "○ Claude"
                    };
                    let label = if command == "codex" || command == "claude" {
                        "Agent"
                    } else {
                        "Agent · Custom command (Advanced)"
                    };
                    frame.render_widget(
                        Paragraph::new(label).style(Style::default().fg(t.muted)),
                        Rect::new(rect.x, rect.y, rect.width, 1),
                    );
                    if rect.height > 1 {
                        let (_, controls) = buttons::draw_outlined_top(
                            t,
                            frame,
                            Rect::new(rect.x, rect.y + 1, rect.width, rect.height - 1),
                            &[
                                Button::new(codex, KeyCode::F(2), enabled),
                                Button::new(claude, KeyCode::F(3), enabled),
                            ],
                        );
                        hits.extend(controls);
                    }
                }
                ADVANCED => {
                    let (_, controls) = buttons::draw_outlined_top(
                        t,
                        frame,
                        rect,
                        &[Button::new(
                            if self.advanced {
                                "▾ Advanced F4"
                            } else {
                                "▸ Advanced F4"
                            },
                            KeyCode::F(4),
                            enabled,
                        )],
                    );
                    hits.extend(controls);
                }
                4 => {
                    let label = format!("Open in: {} (←/→)", Place::ALL[self.place].label());
                    let (_, controls) = buttons::draw_outlined_top(
                        t,
                        frame,
                        rect,
                        &[Button::new(&label, KeyCode::F(5), enabled)],
                    );
                    hits.extend(controls);
                }
                PREVIEW => {
                    let args = self.args();
                    let preview = args
                        .map(|args| {
                            shell_words::join(
                                std::iter::once(program).chain(args.iter().map(String::as_str)),
                            )
                        })
                        .unwrap_or_else(|e| e.to_string());
                    let lines = crate::queue::wrap_text(&preview, rect.width.saturating_sub(2));
                    let inner_height = rect.height.saturating_sub(2);
                    self.preview_top = self.preview_top.min(
                        lines
                            .len()
                            .saturating_sub(inner_height as usize)
                            .min(u16::MAX as usize) as u16,
                    );
                    frame.render_widget(
                        Paragraph::new(lines)
                            .scroll((self.preview_top, 0))
                            .block(t.block(" Preview · PgUp/PgDn / Wheel ", self.field == PREVIEW))
                            .style(Style::default().fg(t.muted)),
                        rect,
                    );
                    self.field_hits.push((rect, PREVIEW));
                }
                0..=3 => {
                    let hint = match id {
                        1 => {
                            if self.manual_name {
                                "Exact name · edit freely"
                            } else {
                                "Suggested name · edit freely"
                            }
                        }
                        2 => "Quoted arguments; no shell expansion",
                        3 => "Enter newline · ↑↓ move · paste multiple lines",
                        _ => "",
                    };
                    let input_height = rect
                        .height
                        .saturating_sub(u16::from(!hint.is_empty() && rect.height > 3));
                    let box_rect = Rect::new(rect.x, rect.y, rect.width, input_height);
                    let error = self.invalid(id);
                    let label = ["Directory", "Name", "Command", "First message (optional)"][id];
                    let title = error
                        .as_ref()
                        .map_or(format!(" {label} "), |error| format!(" {label} · {error} "));
                    let focused = self.field == id && enabled;
                    let mut block = t.block(title, focused);
                    if error.is_some() {
                        block = block.border_style(Style::default().fg(t.danger));
                    }
                    let inside = block.inner(box_rect);
                    frame.render_widget(block, box_rect);
                    self.fields[id].draw(
                        frame,
                        inside,
                        focused,
                        [
                            "/path/to/project",
                            "project/my-agent",
                            "codex --model 'model name'",
                            "Describe the first task…",
                        ][id],
                        t,
                    );
                    if enabled {
                        self.field_hits.push((box_rect, id));
                    }
                    if input_height < rect.height {
                        frame.render_widget(
                            Paragraph::new(hint).style(Style::default().fg(t.muted)),
                            Rect::new(rect.x, rect.y + input_height, rect.width, 1),
                        );
                    }
                }
                _ => unreachable!(),
            }
            // Give keyboard-focused non-text controls the same visible emphasis.
            if self.field >= 4 && (id == self.field || (id == CODEX && self.field == CLAUDE)) {
                for hit in hits.iter().filter(|h| rect.contains(h.area.as_position())) {
                    let selected = id != CODEX
                        || hit.key.code == KeyCode::F(if self.field == CODEX { 2 } else { 3 });
                    if selected {
                        // Outline and label turn to the focus colour; the frame stays unfilled.
                        frame
                            .buffer_mut()
                            .set_style(hit.area, Style::default().fg(t.focus));
                    }
                }
            }
            y += rect.height;
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_new_and_queue_use_the_same_bottom_cancel() {
        use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
        fn text(buffer: &Buffer, area: Rect) -> String {
            (area.y..area.bottom())
                .map(|y| {
                    (area.x..area.right())
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        let t = Theme::default();
        let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
        let mut hits = Vec::new();
        let terminals = crate::terminals::Terminals::new("unused-fake-corral".into());
        let placement = crate::placement::Placement {
            pane: terminals.active_pane().id,
            place: None,
            selected: 0,
            pressed: None,
        };
        terminal
            .draw(|frame| {
                hits = crate::placement::draw(&t, frame, frame.area(), &terminals, &placement, &[])
                    .into_iter()
                    .map(|(hit, _)| hit)
                    .collect()
            })
            .unwrap();
        let cancel = hits.iter().find(|h| h.key.code == KeyCode::Esc).unwrap();
        assert_eq!(
            cancel.area.height, 1,
            "Cancel must use the existing compact toolbar"
        );
        let buffer = terminal.backend().buffer();
        assert_eq!(text(buffer, cancel.area), "‹Cancel Esc›");
        let style: Vec<_> = (cancel.area.x..cancel.area.right())
            .map(|x| buffer[(x, cancel.area.y)].clone())
            .collect();
        let mut form = Form::new("/tmp/demo".into());
        let mut queue = crate::queue::Panel::default();
        queue.page = crate::queue::Page::Project("/tmp/demo".into());
        for new in [true, false] {
            terminal
                .draw(|frame| {
                    if new {
                        hits = form.draw(&t, frame, "corral", &[]);
                    } else {
                        queue.draw_overlay(&t, frame);
                        hits = queue.buttons.clone();
                    }
                })
                .unwrap();
            let cancel = hits.iter().find(|h| h.key.code == KeyCode::Esc).unwrap();
            let buffer = terminal.backend().buffer();
            println!(
                "{} synthetic render:\n{}",
                if new { "NEW" } else { "QUEUE Project path" },
                text(buffer, buffer.area)
            );
            assert_eq!(cancel.area.height, 1);
            assert_eq!(
                cancel.area.bottom(),
                crate::theme::centered(
                    buffer.area,
                    if new { 104 } else { 76 },
                    if new { 24 } else { 20 }
                )
                .bottom()
                    - 1
            );
            let cells: Vec<_> = (cancel.area.x..cancel.area.right())
                .map(|x| buffer[(x, cancel.area.y)].clone())
                .collect();
            assert_eq!(cells, style, "same label, border, text and shortcut colors");
        }
        form.key(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &[],
        );
        terminal
            .draw(|frame| hits = form.draw(&t, frame, "corral", &[]))
            .unwrap();
        let back: Vec<_> = hits.iter().filter(|h| h.key.code == KeyCode::Esc).collect();
        assert_eq!(back.len(), 1, "project picker has one bottom return action");
        let buffer = terminal.backend().buffer();
        assert_eq!(text(buffer, back[0].area), "‹Back Esc›");
        assert_eq!(
            back[0].area.bottom(),
            crate::theme::centered(buffer.area, 104, 24).bottom() - 1
        );
    }

    #[test]
    fn default_project_and_codex_can_create_without_typing_a_command() {
        let form = Form::new("/tmp/demo".into());
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "demo/codex",
                "--cwd",
                "/tmp/demo",
                "--unique",
                "--",
                "codex"
            ]
        );
    }

    #[test]
    fn text_edits_at_the_cursor_without_corrupting_wide_characters() {
        let mut form = Form::new(String::new());
        form.field = 0;
        form.paste("a中b");
        form.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &[]);
        form.paste("文");
        assert_eq!(form.fields[0].text, "a中文b");
        form.key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), &[]);
        form.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &[]);
        form.key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE), &[]);
        assert_eq!(form.fields[0].text, "ab");
        form.key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), &[]);
        form.paste("首");
        form.key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), &[]);
        form.paste("尾");
        assert_eq!(form.fields[0].text, "首ab尾");
    }

    fn press(form: &mut Form, code: KeyCode) {
        assert!(!form.key(KeyEvent::new(code, KeyModifiers::NONE), &[]));
    }

    #[test]
    fn choices_update_suggestions_but_keep_manual_names_and_advanced_values() {
        let mut form = Form::new("/tmp/first".into());
        let projects = vec!["/tmp/first".into(), "/tmp/second project".into()];
        form.key(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &projects,
        );
        form.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &projects);
        form.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &projects);
        press(&mut form, KeyCode::F(3));
        assert_eq!(
            form.args().unwrap(),
            [
                "start",
                "second-project/claude",
                "--cwd",
                "/tmp/second project",
                "--unique",
                "--",
                "claude"
            ]
        );
        press(&mut form, KeyCode::Tab); // Name
        form.key(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &[],
        );
        form.paste("mine");
        press(&mut form, KeyCode::F(2));
        assert_eq!(form.args().unwrap()[1], "mine");
        assert!(!form.args().unwrap().iter().any(|a| a == "--unique"));
        press(&mut form, KeyCode::F(4));
        press(&mut form, KeyCode::Tab); // Command
        form.key(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &[],
        );
        form.paste("codex --model 'test model'");
        press(&mut form, KeyCode::Tab); // Message
        form.paste("hello\n世界");
        press(&mut form, KeyCode::Home);
        form.paste("你好，");
        press(&mut form, KeyCode::Up);
        press(&mut form, KeyCode::End);
        form.paste("!");
        press(&mut form, KeyCode::Tab); // Place
        press(&mut form, KeyCode::Right);
        let args = form.args().unwrap();
        assert_eq!(
            args,
            [
                "start",
                "mine",
                "--cwd",
                "/tmp/second project",
                "--prompt",
                "hello!\n你好，世界",
                "--",
                "codex",
                "--model",
                "test model"
            ]
        );
        press(&mut form, KeyCode::F(4)); // Collapse
        press(&mut form, KeyCode::Tab);
        press(&mut form, KeyCode::BackTab);
        press(&mut form, KeyCode::F(4)); // Expand
        assert_eq!(form.args().unwrap(), args);
        assert_eq!(form.place, 2);
        press(&mut form, KeyCode::Esc);
        assert!(!form.visible);
        assert_eq!(form.args().unwrap(), args);
    }

    #[test]
    fn short_form_keeps_focused_inputs_and_footer_reachable() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut form = Form::new("/tmp/demo".into());
        let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
        press(&mut form, KeyCode::F(4));
        form.key(
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
            &[],
        );
        let mut visited = Vec::new();
        for _ in 0..10 {
            terminal
                .draw(|frame| {
                    let hits = form.draw(&Theme::default(), frame, "corral", &[]);
                    assert!(hits.iter().any(|h| h.key.code == KeyCode::Char('s')));
                    assert!(hits.iter().any(|h| h.key.code == KeyCode::Esc));
                })
                .unwrap();
            if form.field < 4 {
                assert!(
                    form.field_hits
                        .iter()
                        .any(|(r, i)| *i == form.field && r.height >= 3)
                );
                let cursor = terminal.backend().cursor_position();
                assert!(cursor.x < 40 && cursor.y < 11);
                visited.push(form.field);
            }
            press(&mut form, KeyCode::Tab);
        }
        assert_eq!(visited, [0, 1, 2, 3]);
    }

    #[test]
    fn inner_controls_are_unfilled_outlines_matching_their_targets() {
        use ratatui::{Terminal, backend::TestBackend, style::Color};
        let t = Theme::default();
        let mut form = Form::new("/tmp/demo".into());
        press(&mut form, KeyCode::F(4));
        form.key(
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
            &[],
        );
        press(&mut form, KeyCode::F(2));
        let mut terminal = Terminal::new(TestBackend::new(106, 42)).unwrap();
        let mut hits = Vec::new();
        terminal
            .draw(|frame| hits = form.draw(&t, frame, "corral", &[]))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..42)
            .map(|y| (0..106).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        println!("{}", rows.join("\n"));
        let inner: Vec<_> = hits
            .iter()
            .filter(|h| h.area.height == 3)
            .map(|h| h.key)
            .collect();
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        let plain = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(
            inner,
            [
                ctrl('p'),
                ctrl('e'),
                plain(KeyCode::F(2)),
                plain(KeyCode::F(3)),
                plain(KeyCode::F(4)),
                plain(KeyCode::F(5)),
            ],
            "every inner control, all fully visible"
        );
        assert_eq!(
            hits.len(),
            inner.len() + 2,
            "Create and Cancel stay compact"
        );
        for hit in hits.iter().filter(|h| h.area.height == 3) {
            let a = hit.area;
            assert_eq!(buffer[(a.x, a.y)].symbol(), "╭");
            assert_eq!(buffer[(a.right() - 1, a.bottom() - 1)].symbol(), "╯");
            // Codex has keyboard focus: focus colour, still no fill.
            let focused = hit.key.code == KeyCode::F(2);
            assert_eq!(
                buffer[(a.x, a.y)].fg,
                if focused { t.focus } else { t.muted }
            );
            for y in a.y..a.bottom() {
                for x in a.x..a.right() {
                    assert_eq!(buffer[(x, y)].bg, Color::Reset);
                }
            }
            for other in hits.iter().filter(|o| o.area != a) {
                assert!(!a.intersects(other.area));
            }
        }
        assert!(rows.iter().any(|r| r.contains("│ ● Codex │ │ ○ Claude │")));

        form.key(ctrl('p'), &[]);
        terminal
            .draw(|frame| hits = form.draw(&t, frame, "corral", &["/tmp/demo".into()]))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let edit = hits.iter().find(|h| h.key == ctrl('e')).unwrap();
        assert_eq!(edit.area.height, 3);
        assert_eq!(buffer[(edit.area.x, edit.area.y)].symbol(), "╭");
        assert!(form.project_hits[0].0.y >= edit.area.bottom());
    }
}
