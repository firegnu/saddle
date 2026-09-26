//! Editing state local to the New-agent form. Positions are UTF-8 boundaries.
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Style,
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthChar;

pub(super) struct Input {
    pub text: String,
    pub cursor: usize,
    area: Rect,
    top: usize,
    left: usize,
}

impl Input {
    pub fn new(text: String) -> Self {
        Self {
            cursor: text.len(),
            text,
            area: Rect::default(),
            top: 0,
            left: 0,
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn click(&mut self, point: Position) {
        if self.area.is_empty() {
            return;
        }
        let row = self.top
            + point
                .y
                .saturating_sub(self.area.y)
                .min(self.area.height - 1) as usize;
        let column =
            self.left + point.x.saturating_sub(self.area.x).min(self.area.width - 1) as usize;
        let mut start = 0;
        for (i, line) in self.text.split('\n').enumerate() {
            if i == row {
                self.cursor = start + byte_at_column(line, column);
                return;
            }
            start += line.len() + 1;
        }
        self.cursor = self.text.len();
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        placeholder: &str,
        t: &crate::theme::Theme,
    ) {
        self.area = area;
        if area.is_empty() {
            return;
        }
        let row = self.text[..self.cursor]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        let column = width(&self.text[self.start()..self.cursor]);
        if focused {
            self.top = self
                .top
                .min(row)
                .max(row.saturating_sub(area.height as usize - 1));
            self.left = self
                .left
                .min(column)
                .max(column.saturating_sub(area.width as usize - 1));
        }
        if self.text.is_empty() {
            frame.render_widget(
                Paragraph::new(placeholder).style(Style::default().fg(t.dim)),
                area,
            );
        } else {
            let lines: Vec<String> = self
                .text
                .split('\n')
                .skip(self.top)
                .take(area.height as usize)
                .map(|line| {
                    let mut visible = String::new();
                    let mut column = 0;
                    for c in line.chars() {
                        let w = char_width(c);
                        if column >= self.left && column + w <= self.left + area.width as usize {
                            if visible.is_empty() {
                                visible.push_str(&" ".repeat(column - self.left));
                            }
                            if c == '\t' {
                                visible.push_str("    ");
                            } else {
                                visible.push(c);
                            }
                        }
                        column += w;
                    }
                    visible
                })
                .collect();
            frame.render_widget(
                Paragraph::new(lines.join("\n")).style(Style::default().fg(t.text)),
                area,
            );
        }
        if focused {
            frame.set_cursor_position((
                area.x + (column - self.left) as u16,
                area.y + (row - self.top) as u16,
            ));
        }
    }

    pub fn insert(&mut self, text: &str, multiline: bool) {
        let text: String = text
            .chars()
            .filter(|c| !c.is_control() || (multiline && matches!(c, '\n' | '\t')))
            .collect();
        self.text.insert_str(self.cursor, &text);
        self.cursor += text.len();
    }

    fn start(&self) -> usize {
        self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1)
    }

    fn end(&self) -> usize {
        self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |i| self.cursor + i)
    }

    pub fn key(&mut self, key: KeyCode, multiline: bool) {
        match key {
            KeyCode::Left => {
                self.cursor = self.text[..self.cursor]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(i, _)| i);
            }
            KeyCode::Right => {
                self.cursor += self.text[self.cursor..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
            }
            KeyCode::Home => self.cursor = self.start(),
            KeyCode::End => self.cursor = self.end(),
            KeyCode::Backspace if self.cursor > 0 => {
                let end = self.cursor;
                self.key(KeyCode::Left, multiline);
                self.text.drain(self.cursor..end);
            }
            KeyCode::Delete if self.cursor < self.text.len() => {
                self.text.remove(self.cursor);
            }
            KeyCode::Up | KeyCode::Down if multiline => {
                let start = self.start();
                let column = width(&self.text[start..self.cursor]);
                let target = if key == KeyCode::Up && start > 0 {
                    Some(self.text[..start - 1].rfind('\n').map_or(0, |i| i + 1))
                } else if key == KeyCode::Down && self.end() < self.text.len() {
                    Some(self.end() + 1)
                } else {
                    None
                };
                if let Some(target) = target {
                    self.cursor = target
                        + byte_at_column(self.text[target..].split('\n').next().unwrap(), column);
                }
            }
            KeyCode::Enter if multiline => self.insert("\n", true),
            KeyCode::Char(c) => self.insert(&c.to_string(), multiline),
            _ => {}
        }
    }
}

fn char_width(c: char) -> usize {
    if c == '\t' { 4 } else { c.width().unwrap_or(0) }
}

fn width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

fn byte_at_column(text: &str, column: usize) -> usize {
    let mut x = 0;
    for (i, c) in text.char_indices() {
        x += char_width(c);
        if x > column {
            return i;
        }
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn scrolled_multiline_click_and_cursor_share_cell_coordinates() {
        let mut input = Input::new("first\nsecond\n123456中尾".into());
        let mut terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
        terminal
            .draw(|frame| {
                input.draw(
                    frame,
                    frame.area(),
                    true,
                    "",
                    &crate::theme::Theme::default(),
                )
            })
            .unwrap();
        assert_eq!(terminal.backend().cursor_position(), Position::new(7, 1));
        // Last line is horizontally scrolled by three cells. 中 starts at x=3.
        input.click(Position::new(4, 1));
        input.insert("文", true);
        assert_eq!(input.text, "first\nsecond\n123456文中尾");
        input.key(KeyCode::Home, true);
        input.key(KeyCode::Backspace, true);
        assert_eq!(input.text, "first\nsecond123456文中尾");
        input.key(KeyCode::Enter, true);
        assert_eq!(input.text, "first\nsecond\n123456文中尾");
    }
}
