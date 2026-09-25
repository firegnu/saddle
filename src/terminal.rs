use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    vte::ansi::{Processor, Rgb},
};
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone, Copy)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }
    fn screen_lines(&self) -> usize {
        usize::from(self.rows.max(1))
    }
    fn columns(&self) -> usize {
        usize::from(self.cols.max(2))
    }
}
#[derive(Clone)]
pub struct Events(Sender<Event>);
impl EventListener for Events {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event);
    }
}
pub struct Screen {
    pub term: Term<Events>,
    parser: Processor,
    events: Receiver<Event>,
}
impl Screen {
    pub fn new(size: Size) -> Self {
        let (tx, events) = mpsc::channel();
        Self {
            term: Term::new(Default::default(), &size, Events(tx)),
            parser: Processor::new(),
            events,
        }
    }
    pub fn process(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.parser.advance(&mut self.term, bytes);
        let mut replies = Vec::new();
        for event in self.events.try_iter() {
            let reply = match event {
                Event::PtyWrite(text) => text,
                Event::TextAreaSizeRequest(format) => {
                    format(alacritty_terminal::event::WindowSize {
                        num_lines: self.term.screen_lines() as u16,
                        num_cols: self.term.columns() as u16,
                        cell_width: 0,
                        cell_height: 0,
                    })
                }
                Event::ColorRequest(index, format) => format(default_rgb(index)),
                _ => continue,
            };
            replies.extend_from_slice(reply.as_bytes());
        }
        replies
    }
    pub fn resize(&mut self, size: Size) {
        self.term.resize(size);
    }
}

fn default_rgb(index: usize) -> Rgb {
    let (r, g, b) = match index {
        0..=15 => [
            (0, 0, 0),
            (205, 0, 0),
            (0, 205, 0),
            (205, 205, 0),
            (0, 0, 238),
            (205, 0, 205),
            (0, 205, 205),
            (229, 229, 229),
            (127, 127, 127),
            (255, 0, 0),
            (0, 255, 0),
            (255, 255, 0),
            (92, 92, 255),
            (255, 0, 255),
            (0, 255, 255),
            (255, 255, 255),
        ][index],
        16..=231 => {
            let n = index - 16;
            let c = [0, 95, 135, 175, 215, 255];
            (c[n / 36], c[n / 6 % 6], c[n % 6])
        }
        232..=255 => {
            let v = 8 + (index as u8 - 232) * 10;
            (v, v, v)
        }
        257 => (0, 0, 0),
        _ => (229, 229, 229),
    };
    Rgb { r, g, b }
}

impl Screen {
    pub fn render(
        &self,
        area: ratatui::layout::Rect,
        buffer: &mut ratatui::buffer::Buffer,
    ) -> Option<(u16, u16)> {
        use alacritty_terminal::{
            index::{Column, Line},
            term::{TermMode, cell::Flags},
        };
        use ratatui::style::{Modifier, Style};
        let rows = area.height.min(self.term.screen_lines() as u16);
        let cols = area.width.min(self.term.columns() as u16);
        for y in 0..rows {
            for x in 0..cols {
                let cell = &self.term.grid()[Line(i32::from(y))][Column(usize::from(x))];
                if cell
                    .flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    continue;
                }
                let mut style = Style::default()
                    .fg(self.color(cell.fg))
                    .bg(self.color(cell.bg));
                for (flag, modifier) in [
                    (Flags::BOLD, Modifier::BOLD),
                    (Flags::DIM, Modifier::DIM),
                    (Flags::ITALIC, Modifier::ITALIC),
                    (Flags::ALL_UNDERLINES, Modifier::UNDERLINED),
                    (Flags::INVERSE, Modifier::REVERSED),
                    (Flags::HIDDEN, Modifier::HIDDEN),
                    (Flags::STRIKEOUT, Modifier::CROSSED_OUT),
                ] {
                    if cell.flags.intersects(flag) {
                        style = style.add_modifier(modifier);
                    }
                }
                let mut symbol = cell.c.to_string();
                if let Some(chars) = cell.zerowidth() {
                    symbol.extend(chars);
                }
                if cell.flags.contains(Flags::WIDE_CHAR) && x + 1 >= cols {
                    symbol = " ".into();
                }
                buffer[(area.x + x, area.y + y)]
                    .set_symbol(&symbol)
                    .set_style(style);
            }
        }
        let point = self.term.grid().cursor.point;
        if self.term.mode().contains(TermMode::SHOW_CURSOR)
            && point.line.0 >= 0
            && point.line.0 < i32::from(rows)
            && point.column.0 < usize::from(cols)
        {
            Some((area.x + point.column.0 as u16, area.y + point.line.0 as u16))
        } else {
            None
        }
    }
    fn color(&self, color: alacritty_terminal::vte::ansi::Color) -> ratatui::style::Color {
        use alacritty_terminal::vte::ansi::Color as A;
        use ratatui::style::Color as R;
        let index = match color {
            A::Spec(rgb) => return R::Rgb(rgb.r, rgb.g, rgb.b),
            A::Indexed(index) => usize::from(index),
            A::Named(name) => name as usize,
        };
        if let Some(rgb) = self.term.colors()[index] {
            return R::Rgb(rgb.r, rgb.g, rgb.b);
        }
        if index <= 255 {
            R::Indexed(index as u8)
        } else {
            R::Reset
        }
    }
}
