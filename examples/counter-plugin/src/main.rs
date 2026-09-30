use anyhow::Result;
use saddle_plugin_sdk::{
    self as sdk, Context, Event, Plugin,
    ratatui::{buffer::Buffer, layout::Rect, style::Style},
};
use serde_json::Value;
#[derive(Default)]
struct Counter {
    clicks: u64,
    pressed: Option<(u64, u64)>,
    button: Rect,
    notice: String,
}
impl Plugin for Counter {
    fn id(&self) -> &str {
        "demo.counter"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn event(&mut self, event: Event, ctx: &mut Context) -> Result<()> {
        let mut increment = false;
        match event {
            Event::Input(v) => {
                let e = &v["event"];
                if e["type"] == "key"
                    && e["code"]["name"] == "enter"
                    && e["phase"] == "press"
                    && e["modifiers"].as_array().is_some_and(Vec::is_empty)
                {
                    increment = true;
                }
                if e["type"] == "mouse" && e["button"] == "left" {
                    let inside = e["x"]
                        .as_u64()
                        .zip(e["y"].as_u64())
                        .is_some_and(|(x, y)| self.button.contains((x as u16, y as u16).into()));
                    let identity = (
                        v["frame_id"].as_u64().unwrap_or(0),
                        v["size_revision"].as_u64().unwrap_or(0),
                    );
                    if e["action"] == "down" {
                        self.pressed = inside.then_some(identity)
                    }
                    if e["action"] == "up" {
                        increment = inside && self.pressed.take() == Some(identity);
                    }
                }
            }
            Event::Focus(false) | Event::Closed => self.pressed = None,
            Event::Notification { status, .. } => {
                let notice = match status.as_str() {
                    "accepted" | "duplicate" => "",
                    "limited" => "Notification rate limited",
                    _ => "Notification result unknown",
                };
                if self.notice != notice {
                    self.notice = notice.into();
                    ctx.redraw();
                }
            }
            _ => {}
        }
        if increment {
            self.clicks += 1;
            ctx.notify(format!("Count: {}", self.clicks))?;
            ctx.redraw();
        }
        Ok(())
    }
    fn render(&mut self, area: Rect, theme: &Value) -> Result<Buffer> {
        let mut buffer = Buffer::empty(area);
        let text = theme
            .get("text")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .map(|c| sdk::terminal_color(&c))
            .unwrap_or_default();
        let style = Style::default().fg(text);
        let lines = [
            String::new(),
            format!("Clicks: {}", self.clicks),
            String::new(),
            "[ Increment ]".into(),
            String::new(),
            "Enter or click to increment.".into(),
            "Close keeps count.".into(),
            "Disable or restart resets it.".into(),
            self.notice.clone(),
        ];
        self.button = Rect::new(
            2,
            3,
            13.min(area.width.saturating_sub(2)),
            u16::from(area.height > 3),
        );
        for (y, line) in lines.iter().enumerate().take(area.height as usize) {
            if area.width > 2 {
                buffer.set_stringn(2, y as u16, line, usize::from(area.width - 2), style);
            }
        }
        Ok(buffer)
    }
}
fn main() -> Result<()> {
    sdk::run(|| Box::<Counter>::default())
}
