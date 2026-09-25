use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use saddle::input::{Focus, Route};
fn key(code: K, modifiers: M) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}
#[test]
fn escape_focus_does_not_steal_inner_terminal_keys() {
    let mut focus = Focus::Viewer;
    for code in [K::Char('q'), K::Char('x'), K::Tab, K::Enter] {
        assert_eq!(focus.route(key(code, M::NONE)), Route::Terminal);
    }
    assert_eq!(focus.route(key(K::Char(']'), M::CONTROL)), Route::Ignore);
    assert_eq!(focus, Focus::Agents);
    assert_eq!(focus.route(key(K::Tab, M::NONE)), Route::Ignore);
    assert_eq!(focus, Focus::Queue);
    assert_eq!(focus.route(key(K::Char('c'), M::CONTROL)), Route::Queue);
    focus.route(key(K::Char(']'), M::CONTROL));
    focus.route(key(K::BackTab, M::SHIFT));
    assert_eq!(focus, Focus::Viewer);
    focus.route(key(K::Char(']'), M::CONTROL));
    assert_eq!(focus.route(key(K::Char('q'), M::NONE)), Route::Quit);
}

#[test]
fn terminal_keys_preserve_utf8_control_alt_and_cursor_modes() {
    use saddle::input::encode_key;
    assert_eq!(
        encode_key(key(K::Char('中'), M::NONE), false),
        "中".as_bytes()
    );
    assert_eq!(encode_key(key(K::Char('c'), M::CONTROL), false), b"\x03");
    assert_eq!(encode_key(key(K::Char('x'), M::ALT), false), b"\x1bx");
    assert_eq!(encode_key(key(K::Up, M::NONE), false), b"\x1b[A");
    assert_eq!(encode_key(key(K::Up, M::NONE), true), b"\x1bOA");
    assert_eq!(encode_key(key(K::Left, M::CONTROL), true), b"\x1b[1;5D");
    assert_eq!(encode_key(key(K::F(5), M::SHIFT), false), b"\x1b[15;2~");
}

#[test]
fn mouse_coordinates_are_local_and_paste_obeys_inner_terminal_mode() {
    use alacritty_terminal::term::TermMode as T;
    use crossterm::event::{MouseButton as B, MouseEvent, MouseEventKind as E};
    use ratatui::layout::Rect;
    use saddle::input::{encode_mouse, encode_paste};
    let area = Rect::new(53, 1, 66, 38);
    let event = MouseEvent {
        kind: E::Down(B::Left),
        column: 55,
        row: 3,
        modifiers: M::NONE,
    };
    assert_eq!(
        encode_mouse(event, area, T::MOUSE_REPORT_CLICK | T::SGR_MOUSE),
        b"\x1b[<0;3;3M"
    );
    assert_eq!(
        encode_mouse(
            MouseEvent {
                kind: E::Up(B::Left),
                ..event
            },
            area,
            T::MOUSE_REPORT_CLICK | T::SGR_MOUSE
        ),
        b"\x1b[<0;3;3m"
    );
    assert!(encode_mouse(event, area, T::empty()).is_empty());
    assert!(
        encode_mouse(
            MouseEvent { column: 0, ..event },
            area,
            T::MOUSE_REPORT_CLICK
        )
        .is_empty()
    );
    assert!(
        encode_mouse(
            MouseEvent {
                kind: E::Moved,
                ..event
            },
            area,
            T::MOUSE_DRAG | T::SGR_MOUSE
        )
        .is_empty()
    );
    assert_eq!(
        encode_paste("中文\ntext", true),
        "\x1b[200~中文\ntext\x1b[201~".as_bytes()
    );
    assert_eq!(encode_paste("plain", false), b"plain");
}

#[test]
fn crossterm_legacy_ctrl_bracket_alias_returns_focus_without_sending_a_digit() {
    // Crossterm decodes the legacy 0x1d byte as Ctrl-5, not Ctrl-].
    let mut focus = Focus::Viewer;
    assert_eq!(focus.route(key(K::Char('5'), M::CONTROL)), Route::Ignore);
    assert_eq!(focus, Focus::Agents);
}

#[test]
fn legacy_control_digit_aliases_keep_their_original_bytes() {
    use saddle::input::encode_key;
    for (digit, byte) in [('4', 0x1c), ('6', 0x1e), ('7', 0x1f)] {
        assert_eq!(encode_key(key(K::Char(digit), M::CONTROL), false), [byte]);
    }
}

#[test]
fn modified_enter_is_distinct_from_submit_for_multiline_prompts() {
    use saddle::input::encode_key;
    assert_eq!(encode_key(key(K::Enter, M::SHIFT), false), b"\x1b[13;2u");
    assert_eq!(encode_key(key(K::Enter, M::CONTROL), false), b"\x1b[13;5u");
    assert_eq!(encode_key(key(K::Enter, M::ALT), false), b"\x1b\r");
}

#[test]
fn queue_controls_are_native_and_never_routed_to_a_pty() {
    let mut focus = Focus::Queue;
    assert_ne!(focus.route(key(K::Char('p'), M::NONE)), Route::Terminal);
}
