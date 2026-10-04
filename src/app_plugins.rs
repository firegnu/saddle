use super::*;
impl App {
    pub(super) fn plugin_toast_event(&mut self, mouse: &crossterm::event::MouseEvent) -> bool {
        let Some((area, close)) = self.plugin_toast else {
            return false;
        };
        let point = (mouse.column, mouse.row).into();
        if !area.contains(point) {
            return false;
        }
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return true;
        }
        let Some(toast) = self.plugin_toast_shown.clone() else {
            return true;
        };
        let live = self.plugins.notices.lock().unwrap().items.iter().any(|n| {
            n.plugin == toast.plugin
                && n.session == toast.session
                && n.notification_id == toast.notification_id
                && n.expires > Instant::now()
        });
        if !live {
            return true;
        }
        if close.contains(point) {
            self.plugins.notices.lock().unwrap().items.retain(|n| {
                !(n.plugin == toast.plugin
                    && n.session == toast.session
                    && n.notification_id == toast.notification_id)
            });
        } else if toast.target.is_some() {
            if self.plugin_ui_busy()
                || self
                    .plugin_overlay
                    .as_ref()
                    .is_some_and(|o| o.id != toast.plugin)
            {
                self.panel.message =
                    "Close the current dialog before opening this notification".into();
            } else if self.plugins.open_notification(&toast) {
                self.plugins.notices.lock().unwrap().items.retain(|n| {
                    !(n.plugin == toast.plugin
                        && n.session == toast.session
                        && n.notification_id == toast.notification_id)
                });
                self.open_plugin_view_with_context(&toast.plugin, false);
            }
        }
        true
    }
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
        &mut self,
        frame: &mut ratatui::Frame,
        workspace: Rect,
    ) -> Option<(Rect, Rect)> {
        let notices = self.plugins.notices.lock().unwrap();
        self.plugin_toast_shown = notices.items.front().cloned();
        let toast = self.plugin_toast_shown.as_ref()?;
        let area = crate::terminals::toast_area(workspace)?;
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

pub(super) struct Overlay {
    pub(super) id: String,
    panel: crate::plugins::Panel,
    back: Focus,
    back_pane: u64,
    area: Rect,
    pressed_close: bool,
    notice: String,
}
fn overlay_area(panes: Panes) -> Rect {
    let workspace = panes.agents.union(panes.viewer).union(panes.status);
    let width = ((u32::from(workspace.width) * 4 / 5) as u16)
        .max(1)
        .min(workspace.width);
    let height = ((u32::from(workspace.height) * 4 / 5) as u16)
        .max(1)
        .min(workspace.height);
    Rect::new(
        workspace.x + (workspace.width - width) / 2,
        workspace.y + (workspace.height - height) / 2,
        width,
        height,
    )
}
impl App {
    pub(super) fn plugin_ui_busy(&self) -> bool {
        self.settings.is_some()
            || self.plugin_page.is_some()
            || self.telemetry.is_some()
            || self.closing.is_some()
            || self.placement.is_some()
            || self.search.is_some()
            || self.attention.is_some()
            || self.panel.confirm.is_some()
            || self.new_agent.as_ref().is_some_and(|f| f.visible)
    }
    pub(super) fn open_plugin_attention(&mut self, target: &crate::attention::Target) {
        let crate::attention::Target::Plugin { plugin, .. } = target else {
            return;
        };
        if self.plugin_ui_busy() || self.plugin_overlay.is_some() {
            self.panel.message = "Close the current dialog before opening Attention".into();
            return;
        }
        if !self.plugins.open_attention(target) {
            self.panel.message =
                "Attention item changed or its source is unavailable; reopen Attention".into();
            return;
        }
        self.open_plugin_view_with_context(plugin, false);
    }
    pub(super) fn open_plugin_view(&mut self, id: &str) {
        self.open_plugin_view_with_context(id, true);
    }
    fn open_plugin_view_with_context(&mut self, id: &str, context: bool) {
        if self.plugin_overlay.is_some() || self.plugins.state(id) != "Running" {
            return;
        }
        if context {
            self.plugins
                .view_context(id, self.focused_agent_cwd(self.focus));
        }
        self.pointer.cancel();
        let existing = self
            .viewer
            .tabs
            .iter()
            .flat_map(|t| &t.panes)
            .any(|p| p.plugin_id() == Some(id));
        if existing || self.plugins.placement(id) == saddle_plugin_protocol::Placement::Workspace {
            self.viewer.open_plugin(id);
            self.focus = Focus::Viewer;
        } else {
            self.plugin_overlay = Some(Overlay {
                id: id.into(),
                panel: crate::plugins::Panel::unavailable(id),
                back: self.focus,
                back_pane: self.viewer.active_pane().id,
                area: Rect::default(),
                pressed_close: false,
                notice: String::new(),
            });
            self.native_mouse = false;
        }
    }
    fn close_plugin_overlay(&mut self, agents: bool) {
        if let Some(overlay) = self.plugin_overlay.take() {
            self.focus = if agents || self.viewer.get(overlay.back_pane).is_none() {
                Focus::Agents
            } else {
                overlay.back
            };
            if self.focus == Focus::Viewer {
                self.viewer.focus(overlay.back_pane);
            }
        }
        self.pointer.cancel();
        self.native_mouse = false;
    }
    pub(super) fn sync_plugins(&mut self, panes: Panes, focused: bool) {
        for id in self.plugins.take_view_closes() {
            if self.plugin_overlay.as_ref().is_some_and(|o| o.id == id) {
                self.close_plugin_overlay(false);
            } else if self.viewer.active_pane().plugin_id() == Some(id.as_str()) {
                let pane = self.viewer.active_pane().id;
                if let Err(e) = self.viewer.close_pane(pane) {
                    self.panel.message = e.to_string();
                }
            }
        }
        let overlay = self.plugin_overlay.as_ref().map(|o| {
            (
                o.id.as_str(),
                ui::inner(overlay_area(panes)),
                self.plugin_palette.is_none() && self.telemetry.is_none(),
            )
        });
        let picture = self.plugins.sync_with_overlay(
            &mut self.viewer,
            panes.viewer,
            focused && overlay.is_none(),
            overlay,
        );
        if let (Some(o), Some(p)) = (&mut self.plugin_overlay, picture) {
            o.panel = p;
        }
    }
    /// Opens the Telemetry page a plugin asked for from its current input, over the view that
    /// asked; closing the page returns there. The filter stays opaque to the host.
    pub(super) fn telemetry_open_tick(&mut self) {
        for request in self.plugins.take_telemetry_opens() {
            let source_open = self.plugin_palette.is_none()
                && !self.plugin_ui_busy()
                && match &self.plugin_overlay {
                    Some(o) => o.id == request.plugin,
                    None => {
                        self.focus == Focus::Viewer
                            && self.viewer.active_pane().plugin_id()
                                == Some(request.plugin.as_str())
                    }
                };
            if !source_open || !self.plugins.telemetry_open_current(&request) {
                self.plugins.telemetry_open_result(
                    &request,
                    "cancelled",
                    "Source view changed or another dialog is open",
                );
                continue;
            }
            let filter = request.filter.clone();
            self.pointer.cancel();
            self.open_telemetry(
                self.focus,
                Some(crate::telemetry::BindingFilter {
                    kind: filter.kind,
                    scope: filter.scope,
                    key: filter.key,
                    run: filter.run,
                }),
                Some(request.name.clone()),
            );
            self.plugins.telemetry_open_result(&request, "opened", "");
        }
    }
    pub(super) fn draw_plugin_overlay(&mut self, frame: &mut ratatui::Frame, panes: Panes) {
        let Some(o) = &mut self.plugin_overlay else {
            return;
        };
        o.area = overlay_area(panes);
        let escape = if o.panel.picture.as_ref().is_some_and(|p| p.escape_input) {
            "Esc Back"
        } else {
            "Esc Close"
        };
        // Keep the workspace visible as a subdued backdrop to a screen-centered
        // dialog, rather than replacing the agent's terminal region.
        let backdrop = frame.area();
        frame.buffer_mut().set_style(
            backdrop,
            ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::DIM),
        );
        frame.render_widget(ratatui::widgets::Clear, o.area);
        frame.render_widget(
            self.config
                .colors
                .block(format!(" {} · {escape} ", o.panel.name), false)
                .style(self.config.colors.base())
                .title_style(self.config.colors.base()),
            o.area,
        );
        let mut content = o.panel.clone();
        if content.interactive {
            // Transient input feedback belongs to host chrome, not over the
            // plugin's first content row. Failure placeholders remain in place.
            content.note.clear();
        }
        content.draw(frame, ui::inner(o.area), self.plugin_palette.is_none());
        if o.area.width >= 3 && o.area.height > 0 {
            frame.render_widget(
                ratatui::widgets::Paragraph::new("×"),
                Rect::new(o.area.right() - 2, o.area.y, 1, 1),
            );
        }
        let notices = [o.notice.as_str(), o.panel.note.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        ui::status_bar(
            &self.config.colors,
            frame,
            panes.status,
            &o.panel.name,
            &if notices.is_empty() {
                format!("{escape}  Ctrl-] Agents")
            } else {
                format!("{escape}  Ctrl-] Agents  · {notices}")
            },
        );
    }
    pub(super) fn plugin_overlay_event(&mut self, event: &Event) {
        use crossterm::event::KeyModifiers;
        if let Event::Key(k) = event {
            if k.kind != KeyEventKind::Release {
                if k.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(k.code, KeyCode::Char(']' | '5'))
                {
                    self.close_plugin_overlay(true);
                    return;
                }
                let escape_input = self.plugin_overlay.as_ref().is_some_and(|o| {
                    o.panel.interactive && o.panel.picture.as_ref().is_some_and(|p| p.escape_input)
                });
                if k.code == KeyCode::Esc && k.modifiers.is_empty() && !escape_input {
                    self.close_plugin_overlay(false);
                    return;
                }
            }
            if k.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(k.code, KeyCode::Char(']' | '5'))
            {
                return;
            }
        }
        // Keep notifications usable without forwarding their mouse events into the plugin.
        if let Event::Mouse(m) = event {
            if matches!(m.kind, MouseEventKind::Up(_))
                && self
                    .plugin_toast
                    .is_some_and(|(r, _)| r.contains((m.column, m.row).into()))
                && let Some(o) = &mut self.plugin_overlay
            {
                o.pressed_close = false;
            }
            if self.plugin_toast_event(m) {
                return;
            }
        }
        let Some(o) = &mut self.plugin_overlay else {
            return;
        };
        let input = match event {
            Event::Key(k) => Some(crate::plugins::key(*k)),
            Event::Paste(text) if text.len() <= saddle_plugin_protocol::MAX_PASTE => {
                Some(serde_json::json!({"type":"paste","text":text}))
            }
            Event::Paste(_) => {
                o.notice = "Paste too large".into();
                None
            }
            Event::Mouse(m) => {
                let close = o.area.width >= 3
                    && o.area.height > 0
                    && m.row == o.area.y
                    && m.column == o.area.right() - 2;
                if m.kind == MouseEventKind::Down(MouseButton::Left) {
                    o.pressed_close = close;
                }
                if m.kind == MouseEventKind::Up(MouseButton::Left)
                    && std::mem::take(&mut o.pressed_close)
                    && close
                {
                    self.close_plugin_overlay(false);
                    return;
                }
                let area = ui::inner(o.area);
                area.contains((m.column, m.row).into())
                    .then(|| crate::plugins::mouse(*m, area))
            }
            Event::Resize(_, _) => {
                o.pressed_close = false;
                None
            }
            _ => None,
        };
        if let Some(input) = input {
            self.plugins.input(&o.panel, input);
        }
    }
}

impl App {
    fn plugin_palette_items(&self) -> Vec<crate::plugins::palette::Item> {
        let mut opened: std::collections::BTreeSet<String> = self
            .viewer
            .tabs
            .iter()
            .flat_map(|t| &t.panes)
            .filter_map(|p| p.plugin_id().map(str::to_owned))
            .collect();
        if let Some(o) = &self.plugin_overlay {
            opened.insert(o.id.clone());
        }
        let source_plugin = self
            .plugin_palette
            .as_ref()
            .and_then(|p| p.placement)
            .filter(|(_, place)| *place != Place::Tab)
            .and_then(|(pane, _)| self.viewer.get(pane))
            .and_then(|p| p.plugin_id());
        self.plugins
            .palette_items(&opened)
            .into_iter()
            .filter(|item| Some(item.id.as_str()) != source_plugin)
            .collect()
    }
    pub(super) fn update_plugin_palette(&mut self) {
        if self.plugin_palette.is_none() {
            return;
        }
        let items = self.plugin_palette_items();
        if let Some(palette) = &mut self.plugin_palette {
            palette.update(items);
        }
    }
    /// Opens the pinned plugin as the launcher would. When it cannot open now, the launcher
    /// opens on that plugin to explain why; nothing is enabled, started or restarted.
    pub(super) fn open_pinned(&mut self) {
        let Some(item) = self.plugins.pinned_item() else {
            self.panel.message = "No pinned plugin. Pin one in Settings → Plugins.".into();
            return;
        };
        if self.plugin_ui_busy() {
            return;
        }
        self.pointer.cancel();
        if item.action().is_none() {
            self.plugin_palette = Some(crate::plugins::palette::Palette::selecting(&item.id));
            self.update_plugin_palette();
            self.native_mouse = false;
        } else if self.plugin_overlay.as_ref().is_none_or(|o| o.id != item.id) {
            self.close_plugin_overlay(false);
            self.open_plugin_view(&item.id);
        }
    }
    pub(super) fn plugin_launcher_event(&mut self, event: &Event) -> bool {
        if self.plugin_ui_busy() {
            self.plugin_entry_press = None;
            return false;
        }
        let Event::Mouse(m) = event else {
            self.plugin_entry_press = None;
            return false;
        };
        // The fixed Plugins entry and the user's pinned entry; each acts on its own release.
        let point = (m.column, m.row).into();
        let over = [self.hits.plugins, self.hits.pinned]
            .into_iter()
            .find(|r| r.contains(point));
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) if over.is_some() => {
                self.plugin_entry_press = over;
                self.pointer.cancel();
                return true;
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(pressed) = self.plugin_entry_press.take() {
                    if over == Some(pressed) && pressed == self.hits.plugins {
                        self.plugin_palette = Some(Default::default());
                        self.update_plugin_palette();
                        self.native_mouse = false;
                    } else if over == Some(pressed) && pressed == self.hits.pinned {
                        self.open_pinned();
                    }
                    return true;
                }
            }
            MouseEventKind::Moved | MouseEventKind::Drag(MouseButton::Left) => {}
            _ => {
                self.plugin_entry_press = None;
            }
        }
        false
    }
    pub(super) fn plugin_palette_outcome(&mut self, outcome: crate::plugins::palette::Outcome) {
        use crate::plugins::palette::Outcome;
        match outcome {
            Outcome::Stay => {}
            Outcome::Close => {
                self.plugin_palette = None;
            }
            Outcome::Manage => {
                let selected = self
                    .plugin_palette
                    .as_ref()
                    .and_then(|p| p.selected_id())
                    .map(str::to_owned);
                self.plugin_palette = None;
                self.close_plugin_overlay(false);
                self.open_settings(self.focus);
                let mut page = crate::plugins::ui::Page::default();
                if let Some(id) = selected {
                    page.select_plugin(&id, &self.plugins);
                }
                self.plugin_page = Some(page);
            }
            Outcome::Open(item) => {
                if !self.plugin_palette_items().contains(&item) || item.action().is_none() {
                    return;
                }
                let placement = self.plugin_palette.take().and_then(|p| p.placement);
                if let Some((anchor, place)) = placement {
                    let cwd = self
                        .viewer
                        .get(anchor)
                        .and_then(|p| p.source_cwd().map(str::to_owned));
                    match self.viewer.place_plugin(anchor, place, &item.id) {
                        Ok(_) => {
                            if !item.opened {
                                self.plugins.view_context(&item.id, cwd);
                            }
                            self.focus = Focus::Viewer;
                            self.pointer.cancel();
                            self.native_mouse = false;
                        }
                        Err(error) => self.panel.message = error.to_string(),
                    }
                    return;
                }
                if self
                    .plugin_overlay
                    .as_ref()
                    .is_some_and(|o| o.id == item.id)
                {
                    return;
                }
                self.close_plugin_overlay(false);
                self.open_plugin_view(&item.id);
            }
        }
    }
}
