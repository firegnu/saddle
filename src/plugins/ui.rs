//! Immediate plugin operations, independent of the Settings draft.
use super::{Manager, registry::Manifest};
use crate::{
    buttons::{self, Button},
    launch::edit::Input,
    theme::Theme,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph, Wrap},
};
#[derive(Default)]
pub struct Page {
    selected: usize,
    focus: usize,
    pub message: String,
    adding: Option<Adding>,
    hits: Vec<(Rect, usize)>,
    rows: Vec<(Rect, usize)>,
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
}
impl Page {
    pub fn select_plugin(&mut self, id: &str, m: &Manager) {
        self.selected = m
            .registry
            .entries
            .iter()
            .position(|e| e.id == id)
            .unwrap_or(0);
    }
    fn actions(&self, m: &Manager) -> Vec<(&'static str, bool)> {
        let id = m.registry.entries.get(self.selected).map(|e| e.id.as_str());
        let state = id.map(|id| m.state(id)).unwrap_or_default();
        let enabled = id.is_some_and(|id| m.enabled_here(id));
        let stopped = id.is_none_or(|id| m.stopped(id));
        vec![
            ("Open panel", state == "Running"),
            (
                if enabled { "Disable" } else { "Enable" },
                id.is_some() && state != "Stopping" && (enabled || stopped),
            ),
            (
                "Restart",
                enabled && matches!(state.as_str(), "Running" | "Unresponsive" | "Failed"),
            ),
            ("Add local…", m.registry.error.is_none()),
            ("Remove", id.is_some() && !enabled && stopped),
            ("Refresh", true),
            ("Back", true),
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
        self.selected = self
            .selected
            .min(m.registry.entries.len().saturating_sub(1));
        let action = match event {
            Event::Key(k) => match k.code {
                KeyCode::Esc => return Outcome::Back,
                KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    None
                }
                KeyCode::Down => {
                    self.selected =
                        (self.selected + 1).min(m.registry.entries.len().saturating_sub(1));
                    None
                }
                KeyCode::Tab => {
                    self.focus = (self.focus + 1) % 8;
                    None
                }
                KeyCode::BackTab => {
                    self.focus = (self.focus + 7) % 8;
                    None
                }
                KeyCode::Enter if self.focus > 0 => Some(self.focus - 1),
                _ => None,
            },
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
        let Some(action) = action else {
            return Outcome::Stay;
        };
        if !self.actions(m).get(action).is_some_and(|(_, ok)| *ok) {
            return Outcome::Stay;
        }
        let id = m
            .registry
            .entries
            .get(self.selected)
            .map(|e| e.id.clone())
            .unwrap_or_default();
        let result = match action {
            0 => return Outcome::Open(id),
            1 => {
                if m.enabled_here(&id) {
                    m.disable(&id)
                } else {
                    m.enable(&id)
                }
            }
            2 => m.restart(&id),
            3 => {
                self.adding = Some(Adding {
                    input: Input::new(String::new()),
                    preview: None,
                    focus: 0,
                    field: Rect::default(),
                });
                self.message.clear();
                Ok(())
            }
            4 => m.remove(&id),
            5 => m.refresh(),
            _ => return Outcome::Back,
        };
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
                    self.message = format!("{e:#}");
                }
            },
            2 => {
                if let Some(manifest) = &add.preview {
                    match m.add(&crate::config::expand_home(&add.input.text), manifest) {
                        Ok(()) => {
                            self.adding = None;
                            self.message = "Added disabled. Enable to start it.".into();
                            self.selected = m.registry.entries.len().saturating_sub(1);
                        }
                        Err(e) => self.message = format!("{e:#}"),
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
    pub fn draw(&mut self, t: &Theme, frame: &mut Frame, m: &Manager) {
        // Keep the existing details/footer room; only reserve rows for actual entries.
        let height = if self.adding.is_some() {
            28
        } else {
            16 + m.registry.entries.len().clamp(1, 12) as u16
        };
        let area = crate::theme::centered(frame.area(), 80, height);
        frame.render_widget(Clear, area);
        frame.render_widget(
            t.block(
                if self.adding.is_some() {
                    " Add local plugin "
                } else {
                    " Settings · Plugins "
                },
                true,
            )
            .style(t.base().bg(t.overlay)),
            area,
        );
        let inside = crate::ui::inner(area);
        self.rows.clear();
        self.hits.clear();
        let choices: Vec<_> = if let Some(add) = &self.adding {
            vec![
                ("Read manifest", true),
                ("Add disabled", add.preview.is_some()),
                ("Cancel", true),
            ]
        } else {
            self.actions(m)
        };
        let focus = self.adding.as_ref().map_or(self.focus, |a| a.focus);
        let buttons: Vec<_> = choices
            .iter()
            .enumerate()
            .map(|(i, (name, enabled))| {
                let b = Button::new(name, KeyCode::F(i as u8 + 1), *enabled);
                if focus == i + 1 { b.primary() } else { b }
            })
            .collect();
        let (body, hits) = buttons::draw_compact(t, frame, inside, &buttons);
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
            add.field = Rect::new(body.x, body.y.saturating_add(2), body.width, 1);
            add.input
                .draw(frame, add.field, add.focus == 0, "Plugin directory", t);
            let preview=add.preview.as_ref().map(|p|format!("{}  {} · {}\nProgram: {}\n\nRuns with your user permissions when enabled.\nAdding does not start the plugin.",p.name,p.version,p.id,p.program(&crate::config::expand_home(&add.input.text)).map(|p|p.display().to_string()).unwrap_or_default())).unwrap_or_else(||"Enter a directory, then Read manifest.".into());
            frame.render_widget(
                Paragraph::new(format!("Directory\n\n\n\n{preview}\n\n{}", self.message))
                    .wrap(Wrap { trim: false }),
                body,
            );
            add.input
                .draw(frame, add.field, add.focus == 0, "Plugin directory", t);
        } else {
            let header =
                "Changes here apply immediately.\n\n  Name                     Enabled  Runtime";
            frame.render_widget(Paragraph::new(header), body);
            let count = usize::from(body.height.saturating_sub(13)).max(1);
            let start = self.selected.saturating_sub(count - 1);
            for (row, (i, e)) in m
                .registry
                .entries
                .iter()
                .enumerate()
                .skip(start)
                .take(count)
                .enumerate()
            {
                let r = Rect::new(body.x, body.y + 3 + row as u16, body.width, 1);
                if r.y >= body.bottom() {
                    break;
                }
                let name = m
                    .manifest(&e.id)
                    .map(|p| p.name)
                    .unwrap_or_else(|_| e.id.clone());
                let text = format!(
                    "{} {:24} {:8} {}",
                    if i == self.selected { ">" } else { " " },
                    crate::ui::clip(&name, 24),
                    if e.enabled { "Yes" } else { "No" },
                    m.state(&e.id)
                );
                frame.render_widget(
                    Paragraph::new(text).style(Style::default().fg(if i == self.selected {
                        t.focus
                    } else {
                        t.text
                    })),
                    r,
                );
                self.rows.push((r, i));
            }
            let detail = m
                .registry
                .entries
                .get(self.selected)
                .map(|e| {
                    format!(
                        "ID: {}\nDirectory: {}\nProgram: {}\n{}\n{}",
                        e.id,
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
                    )
                })
                .unwrap_or_else(|| "No plugins registered. Add a local directory to begin.".into());
            let top = body.y + 3 + count as u16;
            let r = Rect::new(
                body.x,
                top.min(body.bottom()),
                body.width,
                body.bottom().saturating_sub(top),
            );
            frame.render_widget(
                Paragraph::new(format!(
                    "\n{detail}\n{}\n↑↓ Select · Tab Focus · Enter Activate · Esc Back",
                    m.registry.error.as_deref().unwrap_or(&self.message)
                ))
                .wrap(Wrap { trim: false }),
                r,
            );
        }
    }
}
