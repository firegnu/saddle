use super::*;
impl App {
    pub(super) fn plugin_input(&mut self, event: serde_json::Value) -> bool {
        if self.viewer.active_pane().plugin_id().is_none() {
            return false;
        }
        if let Some(panel) = self.viewer.active_pane().plugin.clone() {
            self.plugins.input(&panel, event);
        }
        true
    }
    pub(super) fn draw_plugin_toast(
        &self,
        frame: &mut ratatui::Frame,
        workspace: Rect,
    ) -> Option<(Rect, Rect)> {
        let notices = self.plugins.notices.lock().unwrap();
        let toast = notices.items.front()?;
        if workspace.width < 12 || workspace.height < 4 {
            return None;
        }
        let area = Rect::new(
            workspace.right().saturating_sub(workspace.width.min(48)),
            workspace.bottom().saturating_sub(4),
            workspace.width.min(48),
            4,
        );
        frame.render_widget(ratatui::widgets::Clear, area);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(toast.text.as_str())
                .wrap(Default::default())
                .block(self.config.colors.block(format!(" {} ", toast.name), false))
                .style(self.config.colors.base()),
            area,
        );
        let close = Rect::new(area.right() - 2, area.y, 1, 1);
        frame.render_widget(ratatui::widgets::Paragraph::new("×"), close);
        Some((area, close))
    }
}
