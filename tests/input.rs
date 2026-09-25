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
    assert_eq!(focus.route(key(K::Char('c'), M::CONTROL)), Route::Terminal);
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
