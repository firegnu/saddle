use anyhow::Result;
use saddle_plugin_sdk::{
    self as sdk, Context, Event, Plugin,
    protocol::AttentionItem,
    ratatui::{buffer::Buffer, layout::Rect, style::Style},
};
use serde_json::{Value, json};
#[derive(Default)]
struct Demo {
    initialized: bool,
    revision: u64,
    count: usize,
    opened: String,
    status: String,
}
impl Demo {
    fn publish(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.attention(
            (1..=self.count)
                .map(|n| AttentionItem {
                    id: format!("sample-{n}"),
                    title: format!("Demo item {n}"),
                    note: format!("Synthetic revision {}", self.revision),
                    action: "open".into(),
                    target: json!({"sample": n, "revision":self.revision}),
                })
                .collect(),
        )?;
        ctx.redraw();
        Ok(())
    }
}
impl Plugin for Demo {
    fn id(&self) -> &str {
        "demo.attention"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn event(&mut self, event: Event, ctx: &mut Context) -> Result<()> {
        match event {
            Event::Tick if !self.initialized => {
                self.initialized = true;
                self.count = 2;
                self.revision = 1;
                self.publish(ctx)?;
            }
            Event::AttentionOpen(target) => {
                // Recheck current synthetic state, just as a business plugin must re-read its source.
                let valid = target.target["sample"]
                    .as_u64()
                    .is_some_and(|n| n > 0 && n <= self.count as u64)
                    && target.target["revision"] == self.revision;
                self.opened = if valid {
                    format!("Opened: {}", target.item_id)
                } else {
                    "Item is no longer current".into()
                };
                ctx.redraw();
            }
            Event::AttentionPublished { status, .. } => {
                self.status = status;
                ctx.redraw();
            }
            Event::Input(v) if v["event"]["type"] == "key" && v["event"]["phase"] == "press" => {
                match v["event"]["code"]["char"].as_str() {
                    Some("u") => {
                        self.revision += 1;
                        self.count = 2;
                        self.publish(ctx)?;
                    }
                    Some("w") => {
                        self.count = 0;
                        self.publish(ctx)?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn render(&mut self, area: Rect, theme: &Value) -> Result<Buffer> {
        let mut out = Buffer::empty(area);
        let style = Style::default().fg(theme
            .get("text")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .map(|c| sdk::terminal_color(&c))
            .unwrap_or_default());
        for (y, line) in [
            "Attention demo — synthetic data only".into(),
            self.opened.clone(),
            format!("Items: {}  Revision: {}", self.count, self.revision),
            "u Update / restore items   w Withdraw all".into(),
            "Esc Close; then open Attention to select an item.".into(),
            format!("Snapshot: {}", self.status),
        ]
        .iter()
        .enumerate()
        .take(area.height as usize)
        {
            if area.width > 2 {
                out.set_stringn(2, y as u16, line, (area.width - 2) as usize, style);
            }
        }
        Ok(out)
    }
}
fn main() -> Result<()> {
    sdk::run(|| Box::<Demo>::default())
}
