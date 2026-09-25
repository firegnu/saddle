use crate::{input::Focus, theme};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Default)]
pub enum Kind {
    #[default]
    Secondary,
    Primary,
    Danger,
}
pub struct Button<'a> {
    pub label: &'a str,
    pub key: KeyEvent,
    pub enabled: bool,
    pub kind: Kind,
}
impl<'a> Button<'a> {
    pub fn new(label: &'a str, code: KeyCode, enabled: bool) -> Self {
        Self {
            label,
            key: KeyEvent::new(code, KeyModifiers::NONE),
            enabled,
            kind: Kind::Secondary,
        }
    }
    pub fn control(label: &'a str, code: KeyCode, enabled: bool) -> Self {
        Self {
            key: KeyEvent::new(code, KeyModifiers::CONTROL),
            ..Self::new(label, code, enabled)
        }
    }
    pub fn primary(mut self) -> Self {
        self.kind = Kind::Primary;
        self
    }
    pub fn danger(mut self) -> Self {
        self.kind = Kind::Danger;
        self
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Hit {
    pub area: Rect,
    pub key: KeyEvent,
}

#[derive(Default)]
pub struct Pointer {
    pub hover: Option<Position>,
    pressed: Option<(Focus, Hit)>,
}
impl Pointer {
    pub fn cancel(&mut self) {
        self.pressed = None;
    }
    pub fn event(&mut self, event: MouseEvent, hits: &[(Focus, Hit)]) -> Option<(Focus, KeyEvent)> {
        let point = Position::new(event.column, event.row);
        self.hover = Some(point);
        let hit = hits.iter().find(|(_, h)| h.area.contains(point));
        if matches!(event.kind, MouseEventKind::Up(MouseButton::Left)) {
            let pressed = self.pressed.take();
            return pressed.filter(|p| Some(p) == hit).map(|(f, h)| (f, h.key));
        }
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
            self.pressed = hit.cloned();
        } else if self.pressed.as_ref() != hit {
            self.pressed = None;
        }
        None
    }
    pub fn paint(&self, frame: &mut Frame, hits: &[(Focus, Hit)]) {
        for (focus, hit) in hits {
            if self.hover.is_some_and(|point| hit.area.contains(point)) {
                let pressed = self
                    .pressed
                    .as_ref()
                    .is_some_and(|(f, h)| f == focus && h == hit);
                frame.buffer_mut().set_style(
                    hit.area,
                    if pressed {
                        Style::default().bg(theme::BRIGHT).fg(theme::BG)
                    } else {
                        Style::default().bg(theme::HOVER).fg(theme::BRIGHT)
                    },
                );
            }
        }
    }
}

/// Compact wrapping toolbar. Top and bottom variants share placement and hit geometry.
pub fn draw(frame: &mut Frame, area: Rect, buttons: &[Button<'_>]) -> (Rect, Vec<Hit>) {
    draw_bar(frame, area, buttons, false)
}
pub fn draw_top(frame: &mut Frame, area: Rect, buttons: &[Button<'_>]) -> (Rect, Vec<Hit>) {
    draw_bar(frame, area, buttons, true)
}
fn draw_bar(frame: &mut Frame, area: Rect, buttons: &[Button<'_>], top: bool) -> (Rect, Vec<Hit>) {
    if area.is_empty() {
        return (area, Vec::new());
    }
    let mut placements = Vec::new();
    let (mut x, mut y) = (0, 0);
    for button in buttons {
        let width = (button.label.width() as u16 + 3).min(area.width);
        if x > 0 && x + width > area.width {
            x = 0;
            y += 1;
        }
        placements.push((Rect::new(x, y, width, 1), button));
        x += width + 1;
    }
    let height = if buttons.is_empty() {
        0
    } else {
        (y + 1).min(area.height)
    };
    let start = if top { area.y } else { area.bottom() - height };
    let mut hits = Vec::new();
    for (relative, button) in placements {
        if relative.y >= height {
            break;
        }
        let rect = Rect::new(area.x + relative.x, start + relative.y, relative.width, 1);
        let style = if !button.enabled {
            Style::default().fg(theme::DIM)
        } else {
            match button.kind {
                Kind::Secondary => Style::default().fg(theme::TEXT).bg(theme::BUTTON),
                Kind::Primary => Style::default().fg(theme::BG).bg(theme::FOCUS),
                Kind::Danger => Style::default()
                    .fg(theme::DANGER)
                    .bg(ratatui::style::Color::Rgb(55, 30, 32)),
            }
        };
        let (label, key) = button.label.rsplit_once(' ').unwrap_or((button.label, ""));
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(format!(" {label}  ")),
                Span::styled(
                    key,
                    if matches!(button.kind, Kind::Secondary) && button.enabled {
                        Style::default().fg(theme::MUTED)
                    } else {
                        style
                    },
                ),
                Span::raw(" "),
            ]))
            .style(style),
            rect,
        );
        if button.enabled {
            hits.push(Hit {
                area: rect,
                key: button.key,
            });
        }
    }
    (
        Rect::new(
            area.x,
            area.y + if top { height } else { 0 },
            area.width,
            area.height - height,
        ),
        hits,
    )
}
