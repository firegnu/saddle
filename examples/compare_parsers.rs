//! Reproducible synthetic prototype; never launches or attaches to an agent.
use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    index::{Column, Line},
    term::cell::Flags,
    vte::ansi::{Color, Processor, Rgb},
};
use std::sync::{Arc, Mutex};

struct Size;
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        6
    }
    fn screen_lines(&self) -> usize {
        6
    }
    fn columns(&self) -> usize {
        20
    }
}
#[derive(Clone, Default)]
struct Replies(Arc<Mutex<Vec<String>>>);
impl EventListener for Replies {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            self.0.lock().unwrap().push(text);
        }
    }
}
fn main() {
    let replies = Replies::default();
    let mut alacritty = Term::new(Default::default(), &Size, replies.clone());
    let mut processor: Processor = Processor::new();
    let mut vt = vt100::Parser::new(6, 20, 0);
    let stream = "\x1b[2J\x1b[H\x1b[38;2;12;34;56m中\x1b[38;5;196mA\x1b[0m\r\nline2\r\nline3\x1b[2;4r\x1b[4;1Hbottom\nscroll\x1b[r\x1b[5;7H\x1b[?1002h\x1b[?1006h\x1b[?2004h\x1b[6n";
    for byte in stream.as_bytes() {
        processor.advance(&mut alacritty, &[*byte]);
        vt.process(&[*byte]);
    }
    let screen = vt.screen();
    for row in 0..6 {
        for col in 0..20 {
            let a = &alacritty.grid()[Line(row)][Column(col)];
            let v = screen.cell(row as u16, col as u16).unwrap();
            if !a.flags.contains(Flags::WIDE_CHAR_SPACER) {
                assert_eq!(
                    a.c.to_string().trim_end(),
                    v.contents().trim_end(),
                    "cell {row},{col}"
                );
            }
        }
    }
    assert_eq!(
        alacritty.grid()[Line(0)][Column(0)].fg,
        Color::Spec(Rgb {
            r: 12,
            g: 34,
            b: 56
        })
    );
    assert_eq!(
        screen.cell(0, 0).unwrap().fgcolor(),
        vt100::Color::Rgb(12, 34, 56)
    );
    assert_eq!(alacritty.grid()[Line(0)][Column(2)].fg, Color::Indexed(196));
    assert_eq!(screen.cell(0, 2).unwrap().fgcolor(), vt100::Color::Idx(196));
    assert_eq!(screen.cursor_position(), (4, 6));
    assert_eq!(alacritty.grid().cursor.point.line, Line(4));
    assert_eq!(alacritty.grid().cursor.point.column, Column(6));
    assert!(screen.bracketed_paste());
    assert_eq!(&*replies.0.lock().unwrap(), &["\x1b[5;7R"]);
    println!(
        "Both parsers: matching grid, RGB/indexed color, CJK, cursor, clear and scroll region; chunked UTF-8 OK."
    );
    println!(
        "Alacritty: cursor query -> ESC[5;7R via Event::PtyWrite; vt100 exposes screen state, no reply callback."
    );
    println!("Synthetic only: real Claude Code/drover appearance and feel remain unverified.");
}
