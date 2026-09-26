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

pub fn draw_open(t: &Theme, frame: &mut Frame, name: &str) -> Vec<buttons::Hit> {
    let area = crate::theme::centered(frame.area(), 64, 16);
    frame.render_widget(Clear, area);
    frame.render_widget(
        t.block(" Open agent ", true).style(t.base().bg(t.overlay)),
        area,
    );
    let labels: Vec<_> = Place::ALL
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{} {}", p.label(), i + 1))
        .collect();
    let mut buttons: Vec<_> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| Button::new(label, KeyCode::Char(char::from(b'1' + i as u8)), true))
        .collect();
    buttons.push(Button::new("Cancel Esc", KeyCode::Esc, true));
    let (body, hits) = buttons::draw(t, frame, crate::ui::inner(area), &buttons);
    frame.render_widget(
        Paragraph::new(format!(
            "{name}\nChoose where to open. Already open agents jump to their existing pane."
        ))
        .wrap(Default::default()),
        body,
    );
    hits
}

pub struct Form {
    pub fields: [String; 4],
    pub field: usize,
    pub place: usize,
    pub busy: Option<Ticket>,
    pub error: String,
    pub visible: bool,
    pub preview_top: u16,
    pub field_hits: Vec<(Rect, usize)>,
}
impl Form {
    pub fn new(project: String) -> Self {
        Self {
            fields: [project, String::new(), String::new(), String::new()],
            field: 0,
            place: 1,
            busy: None,
            error: String::new(),
            visible: true,
            preview_top: 0,
            field_hits: Vec::new(),
        }
    }
    pub fn args(&self) -> Result<Vec<String>> {
        let [cwd, name, command, prompt] = &self.fields;
        if cwd.trim().is_empty() {
            bail!("Directory is required");
        }
        if name.trim().is_empty() || name.starts_with('-') || name.chars().any(char::is_whitespace)
        {
            bail!("Name is required; use a name without spaces or a leading '-'");
        }
        let words = shell_words::split(command).context("Command has an unclosed quote")?;
        if words.first().is_none_or(|w| w.is_empty()) {
            bail!("Command is required");
        }
        if self.fields.iter().any(|s| s.contains('\0')) {
            bail!("Fields cannot contain NUL bytes");
        }
        let mut args = vec![
            "start".into(),
            name.clone(),
            "--cwd".into(),
            expand_home(cwd).to_string_lossy().into_owned(),
        ];
        if !prompt.is_empty() {
            args.extend(["--prompt".into(), prompt.clone()]);
        }
        args.push("--".into());
        args.extend(words);
        Ok(args)
    }
    pub fn paste(&mut self, text: &str) {
        if self.busy.is_none() && self.field < 4 {
            self.fields[self.field].extend(
                text.chars()
                    .filter(|c| !c.is_control() || (self.field == 3 && matches!(c, '\n' | '\t'))),
            );
        }
    }
    /// Returns true only for explicit submission; tab/enter in text never launches an agent.
    pub fn key(&mut self, key: KeyEvent, projects: &[String]) -> bool {
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char(']' | '5')))
        {
            self.visible = false;
            return false;
        }
        match key.code {
            KeyCode::PageUp => {
                self.preview_top = self.preview_top.saturating_sub(5);
                return false;
            }
            KeyCode::PageDown => {
                self.preview_top = self.preview_top.saturating_add(5);
                return false;
            }
            _ => {}
        }
        if self.busy.is_some() {
            return false;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => return true,
                KeyCode::Char('u') if self.field < 4 => self.fields[self.field].clear(),
                KeyCode::Char('p') if !projects.is_empty() => {
                    let next = projects
                        .iter()
                        .position(|p| p == &self.fields[0])
                        .map_or(0, |i| (i + 1) % projects.len());
                    self.fields[0] = projects[next].clone();
                    self.field = 0;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Tab => self.field = (self.field + 1) % 5,
                KeyCode::BackTab => self.field = (self.field + 4) % 5,
                KeyCode::Left if self.field == 4 => self.place = (self.place + 5) % 6,
                KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') if self.field == 4 => {
                    self.place = (self.place + 1) % 6
                }
                KeyCode::Backspace if self.field < 4 => {
                    self.fields[self.field].pop();
                }
                KeyCode::Enter if self.field == 3 => self.fields[3].push('\n'),
                KeyCode::Char(c) if self.field < 4 => self.fields[self.field].push(c),
                _ => {}
            }
        }
        false
    }
    pub fn draw(&mut self, t: &Theme, frame: &mut Frame, program: &str) -> Vec<buttons::Hit> {
        let area = crate::theme::centered(frame.area(), 104, 32);
        frame.render_widget(Clear, area);
        frame.render_widget(
            t.block(" New agent ", true).style(t.base().bg(t.overlay)),
            area,
        );
        let (body, hits) = buttons::draw_compact(
            t,
            frame,
            crate::ui::inner(area),
            &[
                Button::control(
                    if self.busy.is_some() {
                        "Starting…"
                    } else {
                        "Start Ctrl-S"
                    },
                    KeyCode::Char('s'),
                    self.busy.is_none(),
                )
                .primary(),
                Button::control(
                    "Next project Ctrl-P",
                    KeyCode::Char('p'),
                    self.busy.is_none(),
                ),
                Button::new("Back Esc", KeyCode::Esc, true),
            ],
        );
        self.field_hits.clear();
        let mut y = body.y;
        // Keep the selected field reachable even in short terminals.
        let start = if body.height < 13 {
            self.field.min(4)
        } else {
            0
        };
        for i in start..5 {
            if y + 2 > body.bottom() {
                break;
            }
            let label = [
                "Directory",
                "Name",
                "Command (arguments with quotes; no shell expansion)",
                "First message (optional)",
                "Open in (Left/Right or click to change)",
            ][i];
            frame.render_widget(
                Paragraph::new(label).style(Style::default().fg(if self.field == i {
                    t.focus
                } else {
                    t.muted
                })),
                Rect::new(body.x, y, body.width, 1),
            );
            y += 1;
            let height = if i == 3 && body.height >= 18 {
                3.min(body.bottom() - y)
            } else {
                1
            };
            let rect = Rect::new(body.x, y, body.width, height);
            let value = if i == 4 {
                Place::ALL[self.place].label()
            } else {
                &self.fields[i]
            };
            let lines = crate::queue::wrap_text(value, body.width);
            let offset = if self.field == i {
                lines
                    .len()
                    .saturating_sub(height as usize)
                    .min(u16::MAX as usize) as u16
            } else {
                0
            };
            frame.render_widget(
                Paragraph::new(lines)
                    .scroll((offset, 0))
                    .style(Style::default().fg(t.text).bg(if self.field == i {
                        t.selected
                    } else {
                        t.overlay
                    })),
                rect,
            );
            if self.busy.is_none() {
                self.field_hits.push((rect, i));
            }
            y += height;
        }
        let args = self.args();
        let preview = args
            .as_ref()
            .map(|args| {
                shell_words::join(std::iter::once(program).chain(args.iter().map(String::as_str)))
            })
            .unwrap_or_else(|e| e.to_string());
        let text = format!(
            "{}\nPreview · PgUp/PgDn scroll · Tab field · Ctrl-U clear\n{}",
            self.error, preview
        );
        let lines = crate::queue::wrap_text(&text, body.width);
        let height = body.bottom().saturating_sub(y);
        self.preview_top = self.preview_top.min(
            lines
                .len()
                .saturating_sub(height as usize)
                .min(u16::MAX as usize) as u16,
        );
        frame.render_widget(
            Paragraph::new(lines)
                .scroll((self.preview_top, 0))
                .style(Style::default().fg(if self.error.is_empty() {
                    t.muted
                } else {
                    t.danger
                })),
            Rect::new(body.x, y, body.width, height),
        );
        hits
    }
}
