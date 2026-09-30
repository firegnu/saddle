use crate::config::Config;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panes {
    pub agents: Rect,
    pub viewer: Rect,
    pub status: Rect,
}
impl Panes {
    pub fn new(area: Rect, config: &Config) -> Self {
        let height = area.height.saturating_sub(1);
        let limit = if area.width < 100 {
            34
        } else if area.width < 140 {
            44
        } else {
            config.left_width
        };
        let left = config.left_width.min(limit).min(area.width / 2);
        Self {
            agents: Rect::new(area.x, area.y, left, height),
            viewer: Rect::new(area.x + left, area.y, area.width - left, height),
            status: Rect::new(area.x, area.y + height, area.width, area.height.min(1)),
        }
    }
}
