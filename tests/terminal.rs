use alacritty_terminal::{
    index::{Column, Line},
    term::{TermMode, cell::Flags},
    vte::ansi::{Color, Rgb},
};
use saddle::terminal::{Screen, Size};
#[test]
fn screen_preserves_split_utf8_color_modes_and_replies_to_terminal_queries() {
    let mut screen = Screen::new(Size { rows: 6, cols: 20 });
    let mut replies = Vec::new();
    for byte in
        "\x1b[2J\x1b[H\x1b[38;2;12;34;56m中\x1b[38;5;196mA\x1b[?1002h\x1b[?1006h\x1b[?2004h\x1b[6n"
            .as_bytes()
    {
        replies.extend(screen.process(&[*byte]));
    }
    let grid = screen.term.grid();
    assert_eq!(grid[Line(0)][Column(0)].c, '中');
    assert!(
        grid[Line(0)][Column(1)]
            .flags
            .contains(Flags::WIDE_CHAR_SPACER)
    );
    assert_eq!(
        grid[Line(0)][Column(0)].fg,
        Color::Spec(Rgb {
            r: 12,
            g: 34,
            b: 56
        })
    );
    assert_eq!(grid[Line(0)][Column(2)].fg, Color::Indexed(196));
    assert_eq!(replies, b"\x1b[1;4R");
    assert!(
        screen
            .term
            .mode()
            .contains(TermMode::MOUSE_DRAG | TermMode::SGR_MOUSE | TermMode::BRACKETED_PASTE)
    );
}

#[test]
fn terminal_rendering_offsets_cells_styles_and_cursor_inside_its_pane() {
    use ratatui::{buffer::Buffer, layout::Rect, style::Color as C};
    let mut screen = Screen::new(Size { rows: 4, cols: 10 });
    screen.process("\x1b[38;2;12;34;56m中\x1b[38;5;196mA".as_bytes());
    let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 10));
    let cursor = screen.render(Rect::new(5, 2, 10, 4), &mut buffer);
    assert_eq!(buffer[(5, 2)].symbol(), "中");
    assert_eq!(buffer[(5, 2)].fg, C::Rgb(12, 34, 56));
    assert_eq!(buffer[(7, 2)].symbol(), "A");
    assert_eq!(buffer[(7, 2)].fg, C::Indexed(196));
    assert_eq!(buffer[(4, 2)].symbol(), " ");
    assert_eq!(cursor, Some((8, 2)));
    screen.process(b"\x1b[?25l");
    assert_eq!(screen.render(Rect::new(5, 2, 10, 4), &mut buffer), None);
}

fn history_rows(screen: &Screen, rows: u16, cols: u16) -> Vec<(String, String)> {
    use ratatui::{buffer::Buffer, layout::Rect, style::Modifier};
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, rows));
    screen.render(Rect::new(0, 0, cols, rows), &mut buffer);
    (0..rows)
        .map(|y| {
            let text: String = (0..cols).map(|x| buffer[(x, y)].symbol()).collect();
            let marked: String = (0..cols)
                .filter(|x| buffer[(*x, y)].modifier.contains(Modifier::REVERSED))
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            (text.trim_end().to_owned(), marked)
        })
        .collect()
}

fn press(screen: &mut Screen, code: crossterm::event::KeyCode) {
    use crossterm::event::{KeyEvent, KeyModifiers};
    screen.history_key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn history_scrolls_back_and_new_output_does_not_pull_the_view_to_the_bottom() {
    use crossterm::event::KeyCode;
    let mut screen = Screen::new(Size { rows: 4, cols: 20 });
    for i in 0..30 {
        screen.process(format!("line {i}\r\n").as_bytes());
    }
    let text = |screen: &Screen| -> Vec<String> {
        history_rows(screen, 4, 20)
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    };
    assert_eq!(text(&screen), ["line 27", "line 28", "line 29", ""]);
    screen.enter_history();
    assert!(screen.in_history());
    press(&mut screen, KeyCode::PageUp);
    assert_eq!(text(&screen), ["line 23", "line 24", "line 25", "line 26"]);
    // The agent keeps writing; the lines being read stay where they are.
    screen.process(b"line 30\r\nline 31\r\n");
    assert_eq!(text(&screen), ["line 23", "line 24", "line 25", "line 26"]);
    press(&mut screen, KeyCode::Up);
    assert_eq!(text(&screen)[0], "line 22");
    press(&mut screen, KeyCode::Down);
    press(&mut screen, KeyCode::PageDown);
    assert_eq!(text(&screen)[0], "line 27");
    screen.history_scroll(2);
    assert_eq!(text(&screen)[0], "line 25");
    // Esc returns to the live screen.
    press(&mut screen, KeyCode::Esc);
    assert!(!screen.in_history());
    assert_eq!(text(&screen), ["line 29", "line 30", "line 31", ""]);
}

#[test]
fn history_search_jumps_between_matches_and_esc_leaves_the_query_first() {
    use crossterm::event::KeyCode;
    let mut screen = Screen::new(Size { rows: 4, cols: 20 });
    for i in 0..30 {
        let line = match i {
            5 => "Error A".to_owned(),
            20 => "boom error B".to_owned(),
            _ => format!("ok {i}"),
        };
        screen.process(format!("{line}\r\n").as_bytes());
    }
    let marked = |screen: &Screen| -> Vec<String> {
        history_rows(screen, 4, 20)
            .into_iter()
            .filter(|(_, m)| !m.is_empty())
            .map(|(t, m)| format!("{t}|{m}"))
            .collect()
    };
    screen.enter_history();
    press(&mut screen, KeyCode::Char('/'));
    for c in "error".chars() {
        press(&mut screen, KeyCode::Char(c));
    }
    // While typing, keys edit the query instead of moving the view.
    press(&mut screen, KeyCode::Char('n'));
    press(&mut screen, KeyCode::Backspace);
    assert_eq!(screen.history().unwrap().input.as_deref(), Some("error"));
    press(&mut screen, KeyCode::Enter);
    // The newest match above the view bottom comes first, selected for copying.
    assert_eq!(marked(&screen), ["boom error B|error"]);
    assert_eq!(screen.selected_text().as_deref(), Some("error"));
    press(&mut screen, KeyCode::Char('n'));
    assert_eq!(marked(&screen), ["Error A|Error"]);
    press(&mut screen, KeyCode::Char('n'));
    assert_eq!(marked(&screen), ["boom error B|error"], "wraps around");
    press(&mut screen, KeyCode::Char('N'));
    assert_eq!(marked(&screen), ["Error A|Error"]);
    press(&mut screen, KeyCode::Char('N'));
    assert_eq!(marked(&screen), ["boom error B|error"]);

    press(&mut screen, KeyCode::Char('/'));
    press(&mut screen, KeyCode::Char('x'));
    press(&mut screen, KeyCode::Esc);
    assert!(screen.in_history(), "Esc in the query returns to history");
    assert_eq!(screen.history().unwrap().input, None);
    press(&mut screen, KeyCode::Char('/'));
    for c in "missing".chars() {
        press(&mut screen, KeyCode::Char(c));
    }
    press(&mut screen, KeyCode::Enter);
    assert_eq!(screen.history().unwrap().status, "no match");
    press(&mut screen, KeyCode::Esc);
    assert!(!screen.in_history());
    assert_eq!(screen.selected_text(), None);
}

#[test]
fn history_drag_selects_text_across_lines_for_copying() {
    let mut screen = Screen::new(Size { rows: 4, cols: 20 });
    screen.process(b"first line\r\nsecond line\r\nthird\r\n");
    screen.enter_history();
    screen.history_select((6, 0), true);
    screen.history_select((5, 1), false);
    assert_eq!(screen.selected_text().as_deref(), Some("line\nsecond"));
    // Dragging back before the press keeps the pressed cell.
    screen.history_select((2, 2), true);
    screen.history_select((0, 1), false);
    assert_eq!(screen.selected_text().as_deref(), Some("second line\nthi"));
    // A plain click clears the selection.
    screen.history_select((1, 0), true);
    assert_eq!(screen.selected_text(), None);
    screen.history_paste("ignored outside a query");
    assert_eq!(screen.history().unwrap().input, None);
}
