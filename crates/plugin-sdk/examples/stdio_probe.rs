//! Test peer for the SDK's process boundary; not an installable product plugin.
use saddle_plugin_sdk::{
    self as sdk, Context, Event, Plugin,
    ratatui::{buffer::Buffer, layout::Rect, style::Style},
};
struct Probe;
impl Plugin for Probe {
    fn id(&self) -> &str {
        "test.stdio"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn event(&mut self, _: Event, _: &mut Context) -> anyhow::Result<()> {
        Ok(())
    }
    fn render(&mut self, area: Rect, _: &serde_json::Value) -> anyhow::Result<Buffer> {
        let mut b = Buffer::empty(area);
        b.set_stringn(0, 0, "OK", area.width as usize, Style::default());
        Ok(b)
    }
}
fn main() -> anyhow::Result<()> {
    sdk::run(|| {
        println!("business stdout became a log");
        let status = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("read line; test $? -ne 0 || exit 41; echo child stdout became a log")
            .status()
            .unwrap();
        assert!(status.success());
        Box::new(Probe)
    })
}
