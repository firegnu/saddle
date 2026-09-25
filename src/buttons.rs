use crate::{input::Focus, theme};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
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
    captured: bool,
}
impl Pointer {
    pub fn cancel(&mut self) {
        self.pressed = None;
        self.captured = false;
    }
    pub fn captured(&self) -> bool {
        self.captured
    }
    pub fn event(&mut self, event: MouseEvent, hits: &[(Focus, Hit)]) -> Option<(Focus, KeyEvent)> {
        let point = Position::new(event.column, event.row);
        self.hover = Some(point);
        let hit = hits.iter().find(|(_, h)| h.area.contains(point));
        if matches!(event.kind, MouseEventKind::Up(MouseButton::Left)) {
            self.captured = false;
            let pressed = self.pressed.take();
            return pressed.filter(|p| Some(p) == hit).map(|(f, h)| (f, h.key));
        }
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
            self.pressed = hit.cloned();
            self.captured = hit.is_some();
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
                let danger = frame.buffer_mut()[(hit.area.x, hit.area.y)].fg == theme::DANGER;
                frame.buffer_mut().set_style(
                    hit.area,
                    if pressed {
                        Style::default()
                            .bg(ratatui::style::Color::Reset)
                            .fg(if danger { theme::DANGER } else { theme::FOCUS })
                    } else {
                        Style::default()
                            .bg(ratatui::style::Color::Reset)
                            .fg(if danger { theme::DANGER } else { theme::BRIGHT })
                    },
                );
            }
        }
    }
}

/// Wrapping toolbars share placement and hit geometry.
pub fn draw(frame: &mut Frame, area: Rect, buttons: &[Button<'_>]) -> (Rect, Vec<Hit>) {
    draw_bar(frame, area, buttons, false, false)
}
pub fn draw_top(frame: &mut Frame, area: Rect, buttons: &[Button<'_>]) -> (Rect, Vec<Hit>) {
    draw_bar(frame, area, buttons, true, false)
}
pub fn draw_compact(frame: &mut Frame, area: Rect, buttons: &[Button<'_>]) -> (Rect, Vec<Hit>) {
    draw_bar(frame, area, buttons, false, true)
}
fn draw_bar(
    frame: &mut Frame,
    area: Rect,
    buttons: &[Button<'_>],
    top: bool,
    compact: bool,
) -> (Rect, Vec<Hit>) {
    let row_height = if compact { 1 } else { 3 };
    let padding = if compact { 2 } else { 4 };
    if area.height < row_height || area.width < padding + 1 {
        return (area, Vec::new());
    }
    let mut placements = Vec::new();
    let (mut x, mut y) = (0, 0);
    let total: usize = buttons
        .iter()
        .map(|b| b.label.width() + usize::from(padding))
        .sum::<usize>()
        + buttons.len().saturating_sub(1);
    let columns = if buttons.len() == 4 && total > usize::from(area.width) {
        2
    } else {
        buttons.len().max(1)
    };
    let mut in_row = 0;
    for button in buttons {
        let width = (button.label.width() as u16 + padding).min(area.width);
        if x > 0 && (x + width > area.width || in_row == columns) {
            x = 0;
            y += row_height;
            in_row = 0;
        }
        placements.push((Rect::new(x, y, width, row_height), button));
        x += width + 1;
        in_row += 1;
    }
    let height = if buttons.is_empty() {
        0
    } else {
        (y + row_height).min(area.height / row_height * row_height)
    };
    let start = if top { area.y } else { area.bottom() - height };
    let mut hits = Vec::new();
    for (relative, button) in placements {
        if relative.y + row_height > height {
            break;
        }
        let rect = Rect::new(
            area.x + relative.x,
            start + relative.y,
            relative.width,
            row_height,
        );
        let foreground = if !button.enabled {
            theme::DIM
        } else {
            match button.kind {
                Kind::Secondary => theme::TEXT,
                Kind::Primary => theme::FOCUS,
                Kind::Danger => theme::DANGER,
            }
        };
        let style = Style::default()
            .fg(foreground)
            .bg(ratatui::style::Color::Reset);
        let border = if !button.enabled {
            theme::DIM
        } else if matches!(button.kind, Kind::Secondary) {
            theme::BORDER
        } else {
            foreground
        };
        let (label, key) = button.label.rsplit_once(' ').unwrap_or((button.label, ""));
        let key_style = Style::default().fg(
            if button.enabled && matches!(button.kind, Kind::Secondary) {
                theme::MUTED
            } else {
                foreground
            },
        );
        if compact {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("‹", Style::default().fg(border)),
                    Span::raw(format!("{label}{}", if key.is_empty() { "" } else { " " })),
                    Span::styled(key, key_style),
                    Span::styled("›", Style::default().fg(border)),
                ]))
                .style(style),
                rect,
            );
        } else {
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .style(style)
                .border_style(Style::default().fg(border));
            let inner = block.inner(rect);
            frame.render_widget(block, rect);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(format!(" {label}{}", if key.is_empty() { "" } else { " " })),
                    Span::styled(key, key_style),
                    Span::raw(" "),
                ]))
                .style(style),
                inner,
            );
        }
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
