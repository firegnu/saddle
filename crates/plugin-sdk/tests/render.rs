use saddle_plugin_sdk::{
    self as sdk,
    ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style},
    },
};
#[test]
fn buffer_roundtrip_preserves_unicode_and_colors_without_ansi() {
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    b.set_string(
        0,
        0,
        "中文e\u{301}👩‍💻",
        Style::default().fg(Color::Rgb(10, 20, 30)),
    );
    let f = sdk::frame(&b, 1, 1).unwrap();
    let restored = sdk::buffer(&f).unwrap();
    assert_eq!(restored[(0, 0)].symbol(), "中");
    assert_eq!(restored[(4, 0)].symbol(), "e\u{301}");
    assert_eq!(restored[(5, 0)].symbol(), "👩‍💻");
    assert_eq!(restored[(5, 0)].fg, Color::Rgb(10, 20, 30));
    b[(9, 0)].set_symbol("中");
    assert!(sdk::frame(&b, 1, 2).is_err());
}
