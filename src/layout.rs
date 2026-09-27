use crate::config::Config;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panes {
    pub agents: Rect,
    pub viewer: Rect,
    pub status: Rect,
    /// Where the Tasks popup opens: from the left edge, below the Agents top border that
    /// carries its entry, covering part of the Viewer. It is drawn only while open.
    pub tasks: Rect,
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
        let below = height.saturating_sub(1);
        // About 85% of the screen, but never so small that the list and text cannot sit side by side.
        let share = |n: u16| (u32::from(n) * 85 / 100) as u16;
        let width = share(area.width).max(area.width.min(64));
        let tall = share(area.height).max(below.min(20)).min(below);
        Self {
            agents: Rect::new(area.x, area.y, left, height),
            viewer: Rect::new(area.x + left, area.y, area.width - left, height),
            status: Rect::new(area.x, area.y + height, area.width, area.height.min(1)),
            tasks: Rect::new(area.x, area.y + height.min(1), width, tall),
        }
    }
}
