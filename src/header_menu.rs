use crate::theme::Theme;
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Close,
    Settings,
}

const ITEMS: [(&str, Outcome); 1] = [("Settings", Outcome::Settings)];

#[derive(Default)]
pub(crate) struct Menu {
    selected: usize,
    pressed: Option<usize>,
    area: Rect,
    rows: Vec<(Rect, usize)>,
}
impl Menu {
    fn activate(&self, index: usize) -> Outcome {
        ITEMS[index].1
    }
    pub fn event(&mut self, event: &Event) -> Outcome {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.pressed = None;
                match key.code {
                    KeyCode::Esc => return Outcome::Close,
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down | KeyCode::Tab => {
                        self.selected = (self.selected + 1) % ITEMS.len()
                    }
                    KeyCode::Enter => return self.activate(self.selected),
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                let point = (mouse.column, mouse.row).into();
                let row = self
                    .rows
                    .iter()
                    .find(|(r, _)| r.contains(point))
                    .map(|(_, i)| *i);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if !self.area.contains(point) {
                            return Outcome::Close;
                        }
                        self.pressed = row;
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if let Some(pressed) = self.pressed.take()
                            && row == Some(pressed)
                        {
                            return self.activate(pressed);
                        }
                    }
                    MouseEventKind::Moved | MouseEventKind::Drag(MouseButton::Left) => {
                        if let Some(row) = row {
                            self.selected = row;
                        }
                        if self.pressed != row {
                            self.pressed = None;
                        }
                    }
                    _ => {}
                }
            }
            // The old row coordinates are no longer safe to activate.
            Event::Resize(_, _) => return Outcome::Close,
            _ => {}
        }
        Outcome::Stay
    }

    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        bounds: Rect,
        anchor: Rect,
        updates: bool,
    ) {
        let bounds = bounds.intersection(frame.area());
        let width = 16.min(bounds.width);
        let height = 5.min(bounds.height);
        self.area = Rect::new(
            anchor
                .right()
                .saturating_sub(width)
                .clamp(bounds.x, bounds.right() - width),
            anchor.bottom().min(bounds.bottom() - height).max(bounds.y),
            width,
            height,
        );
        frame.render_widget(Clear, self.area);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(t.agents_bg).fg(t.agents_border));
        let inner = block.inner(self.area);
        frame.render_widget(block, self.area);
        self.rows.clear();
        let top = self
            .selected
            .saturating_sub(usize::from(inner.height.saturating_sub(1)));
        for (row, (i, (label, _))) in ITEMS
            .iter()
            .enumerate()
            .skip(top)
            .take(inner.height as usize)
            .enumerate()
        {
            let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
            let mut text = vec![Span::raw(format!(" {label}"))];
            if updates {
                text.push(Span::styled(" ●", Style::default().fg(t.unread)));
            }
            frame.render_widget(
                Paragraph::new(Line::from(text)).style(Style::default().fg(t.agents_text).bg(
                    if i == self.selected {
                        t.agent_selected
                    } else {
                        t.agents_bg
                    },
                )),
                rect,
            );
            self.rows.push((rect, i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers, MouseEvent};
    use ratatui::{Terminal, backend::TestBackend};

    fn mouse(kind: MouseEventKind, rect: Rect) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn menu_requires_same_row_release_and_cancels_outside_or_on_resize() {
        let mut menu = Menu::default();
        let mut terminal = Terminal::new(TestBackend::new(52, 24)).unwrap();
        terminal
            .draw(|f| {
                menu.draw(
                    &Theme::default(),
                    f,
                    f.area(),
                    Rect::new(47, 1, 3, 1),
                    false,
                )
            })
            .unwrap();
        let first = menu.rows[0].0;
        let last = menu.rows[0].0;
        assert_eq!(
            menu.event(&mouse(MouseEventKind::Down(MouseButton::Left), first)),
            Outcome::Stay
        );
        assert_eq!(
            menu.event(&mouse(
                MouseEventKind::Up(MouseButton::Left),
                Rect::new(0, 20, 1, 1)
            )),
            Outcome::Stay
        );
        assert_eq!(
            menu.event(&mouse(MouseEventKind::Down(MouseButton::Left), last)),
            Outcome::Stay
        );
        assert_eq!(
            menu.event(&mouse(MouseEventKind::Up(MouseButton::Left), last)),
            Outcome::Settings
        );
        assert_eq!(
            menu.event(&mouse(
                MouseEventKind::Down(MouseButton::Left),
                Rect::new(0, 20, 1, 1)
            )),
            Outcome::Close
        );
        assert_eq!(menu.event(&Event::Resize(20, 10)), Outcome::Close);
    }

    #[test]
    fn menu_stays_in_bounds_and_keeps_settings_update_notice() {
        for (width, height) in [(52, 24), (20, 10), (12, 3)] {
            let mut menu = Menu::default();
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for _ in 0..2 {
                menu.event(&Event::Key(KeyEvent::new(
                    KeyCode::Down,
                    KeyModifiers::NONE,
                )));
            }
            terminal
                .draw(|f| {
                    menu.draw(
                        &Theme::default(),
                        f,
                        f.area(),
                        Rect::new(width - 3, 1, 3, 1),
                        true,
                    )
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert_eq!(menu.area.intersection(buffer.area), menu.area);
            let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
            assert!(text.contains("Settings"), "{text}");
            if width >= 16 {
                assert!(text.contains('●'));
            }
            assert_eq!(
                menu.event(&Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::NONE
                ))),
                Outcome::Settings
            );
        }
    }
}
