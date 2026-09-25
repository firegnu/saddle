use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType},
};

pub const BG: Color = Color::Rgb(20, 22, 25);
pub const OVERLAY: Color = Color::Rgb(27, 30, 35);
pub const SELECTED: Color = Color::Rgb(35, 41, 49);
pub const BORDER: Color = Color::Rgb(52, 58, 67);
pub const TEXT: Color = Color::Rgb(213, 216, 221);
pub const BRIGHT: Color = Color::Rgb(244, 245, 247);
pub const MUTED: Color = Color::Rgb(139, 146, 156);
pub const DIM: Color = Color::Rgb(90, 97, 107);
pub const FOCUS: Color = Color::Rgb(226, 178, 98);
pub const CONNECTED: Color = Color::Rgb(98, 195, 192);
pub const WORKING: Color = Color::Rgb(111, 166, 227);
pub const BLOCKED: Color = Color::Rgb(232, 150, 79);
pub const WARNING: Color = Color::Rgb(210, 193, 96);
pub const SUCCESS: Color = Color::Rgb(134, 192, 122);
pub const DANGER: Color = Color::Rgb(226, 106, 106);
pub const UNREAD: Color = Color::Rgb(190, 150, 230);
pub const BUTTON: Color = Color::Rgb(38, 44, 52);
pub const HOVER: Color = Color::Rgb(52, 59, 70);

pub fn base() -> Style {
    Style::default().fg(TEXT).bg(BG)
}
pub fn block(title: impl Into<ratatui::text::Line<'static>>, focused: bool) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Plain
        })
        .border_style(Style::default().fg(if focused { FOCUS } else { BORDER }))
}
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
