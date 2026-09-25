use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

pub struct Button<'a> {
    pub label: &'a str,
    pub key: KeyEvent,
    pub enabled: bool,
}
impl<'a> Button<'a> {
    pub fn new(label: &'a str, code: KeyCode, enabled: bool) -> Self {
        Self {
            label,
            key: KeyEvent::new(code, KeyModifiers::NONE),
            enabled,
        }
    }
    pub fn control(label: &'a str, code: KeyCode, enabled: bool) -> Self {
        Self {
            label,
            key: KeyEvent::new(code, KeyModifiers::CONTROL),
            enabled,
        }
    }
}
#[derive(Clone)]
pub struct Hit {
    pub area: Rect,
    pub key: KeyEvent,
}

/// Draw a wrapping button bar at the bottom, returning the remaining content area.
pub fn draw(frame: &mut Frame, area: Rect, buttons: &[Button<'_>]) -> (Rect, Vec<Hit>) {
    if area.width == 0 || area.height < 3 {
        return (area, Vec::new());
    }
    let mut placements = Vec::new();
    let (mut x, mut y) = (0, 0);
    for button in buttons {
        let width = (button.label.width() as u16 + 2).min(area.width);
        if x > 0 && x + width > area.width {
            x = 0;
            y += 1;
        }
        placements.push((Rect::new(x, y, width, 1), button));
        x += width + 1;
    }
    let height = (y + 1).min(area.height.saturating_sub(2));
    let top = area.bottom() - height;
    let mut hits = Vec::new();
    for (relative, button) in placements {
        if relative.y >= height {
            break;
        }
        let rect = Rect::new(area.x + relative.x, top + relative.y, relative.width, 1);
        let style = if button.enabled {
            Style::default().fg(Color::White).bg(Color::Indexed(24))
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(format!("[{}]", button.label)).style(style),
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
        Rect {
            height: area.height - height,
            ..area
        },
        hits,
    )
}
