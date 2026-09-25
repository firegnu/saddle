use crate::config::Config;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panes {
    pub agents: Rect,
    pub queue: Rect,
    pub viewer: Rect,
    pub status: Rect,
    pub tabs: Rect,
}
impl Panes {
    pub fn new(area: Rect, config: &Config) -> Self {
        Self::with_queue(area, config, false)
    }
    pub fn with_queue(area: Rect, config: &Config, queue: bool) -> Self {
        let height = area.height.saturating_sub(1);
        let narrow = area.width < 100;
        let limit = if narrow {
            34
        } else if area.width < 140 {
            44
        } else {
            config.left_width
        };
        let left = config.left_width.min(limit).min(area.width / 2);
        let mut result = Self {
            agents: Rect::default(),
            queue: Rect::default(),
            viewer: Rect::new(area.x + left, area.y, area.width - left, height),
            status: Rect::new(area.x, area.y + height, area.width, area.height.min(1)),
            tabs: Rect::default(),
        };
        if narrow {
            result.tabs = Rect::new(area.x, area.y, left, height.min(1));
            let content = Rect::new(
                area.x,
                area.y + height.min(1),
                left,
                height.saturating_sub(1),
            );
            if queue {
                result.queue = content;
            } else {
                result.agents = content;
            }
        } else {
            let top = ((height as f64 * config.left_split).round() as u16).min(height);
            result.agents = Rect::new(area.x, area.y, left, top);
            result.queue = Rect::new(area.x, area.y + top, left, height - top);
        }
        result
    }
}
