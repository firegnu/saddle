use crate::config::Config;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panes {
    pub agents: Rect,
    pub queue: Rect,
    pub viewer: Rect,
}
impl Panes {
    pub fn new(area: Rect, config: &Config) -> Self {
        let left = config.left_width.min(area.width / 2);
        let top = ((area.height as f64 * config.left_split).round() as u16).min(area.height);
        Self {
            agents: Rect::new(area.x, area.y, left, top),
            queue: Rect::new(area.x, area.y + top, left, area.height - top),
            viewer: Rect::new(area.x + left, area.y, area.width - left, area.height),
        }
    }
}
