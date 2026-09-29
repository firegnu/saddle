use super::*;

impl Panel {
    /// The selected task's explicit project root and task number; `None` for an unnumbered
    /// task, whose records are never guessed from its name, time or files.
    pub fn dispatch_key(&self, program: &str) -> Option<crate::dispatch::Key> {
        let detail = self.content.as_ref()?;
        Some(crate::dispatch::Key {
            program: program.into(),
            project: self.project.clone(),
            task: detail.task.id.clone()?,
            seq: detail.seq,
        })
    }
    /// Reads the Dispatch records only while that view shows them.
    pub fn tick_dispatch(&mut self, program: &str) {
        if self.view == View::Dispatch
            && matches!(self.page, Page::List)
            && self.read_error.is_none()
            && let Some(key) = self.dispatch_key(program)
        {
            self.dispatch.sync(key);
            self.dispatch.tick();
        }
    }
    pub fn reading_dispatch(&self) -> bool {
        self.view == View::Dispatch
            && self.dispatch.reading.is_some()
            && matches!(self.page, Page::List)
    }
    pub(super) fn draw_dispatch(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        mut area: ratatui::layout::Rect,
    ) {
        use crate::dispatch::Status;
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::Line,
            widgets::Paragraph,
        };
        self.dispatch.rows.clear();
        if let Some(reading) = &mut self.dispatch.reading {
            let (body, hits) = crate::buttons::draw_compact_top(
                t,
                frame,
                area,
                &[crate::buttons::Button::new("Back Esc", KeyCode::Esc, true)],
            );
            self.buttons.extend(hits);
            let mut lines: Vec<_> = wrap_text(&reading.title, body.width)
                .into_iter()
                .map(|l| l.style(Style::default().fg(t.bright).add_modifier(Modifier::BOLD)))
                .collect();
            lines.push(Line::raw(""));
            lines.extend(wrap_text(&reading.text, body.width));
            reading.scroll = reading
                .scroll
                .min(lines.len().saturating_sub(body.height as usize));
            frame.render_widget(
                Paragraph::new(
                    lines
                        .into_iter()
                        .skip(reading.scroll)
                        .take(body.height as usize)
                        .collect::<Vec<_>>(),
                ),
                body,
            );
            return;
        }
        let task = self.content.as_ref().and_then(|c| c.task.id.clone());
        let state = &self.dispatch;
        let message = match (&task, &state.status) {
            (None, _) => Some((
                "Unnumbered task: dispatch records are looked up by explicit task number only."
                    .to_owned(),
                t.muted,
            )),
            (_, Status::Loading) => Some(("Reading dispatch records…".into(), t.agent_starting)),
            (_, Status::Missing(program)) => Some((
                format!(
                    "dispatch-log not found ({program}). Dispatch records are optional; the other views and task actions are unaffected. Set queue.dispatch_log in the config to the dlog command."
                ),
                t.agent_blocked,
            )),
            (_, Status::Failed(error)) => Some((
                format!("Could not read dispatch records: {error}\nRefresh r to read again."),
                t.agent_error,
            )),
            (_, Status::Unsupported(error)) => Some((
                format!("Dispatch records are in a format this saddle does not support: {error}"),
                t.agent_error,
            )),
            (Some(task), Status::Loaded) if state.records.is_empty() => Some((
                format!("No dispatch records for {task} in this project."),
                t.muted,
            )),
            (Some(_), Status::Loaded) => None,
        };
        let mut lines = Vec::new();
        let mut positions = Vec::new();
        if let Some((text, color)) = message {
            lines.extend(
                wrap_text(&text, area.width)
                    .into_iter()
                    .map(|l| l.style(Style::default().fg(color))),
            );
        } else {
            lines.extend(
                wrap_text(
                    "Recorded by dispatch-log. Snapshots are copies saved at dispatch time, not the current files (see Links).",
                    area.width,
                )
                .into_iter()
                .map(|l| l.style(Style::default().fg(t.muted))),
            );
            let mut index = 0;
            for record in &state.records {
                lines.push(Line::raw(""));
                lines.extend(
                    wrap_text(&record.heading, area.width).into_iter().map(|l| {
                        l.style(Style::default().fg(t.bright).add_modifier(Modifier::BOLD))
                    }),
                );
                if let Some(error) = &record.error {
                    lines.extend(
                        wrap_text(&format!("  {error}"), area.width)
                            .into_iter()
                            .map(|l| l.style(Style::default().fg(t.agent_error))),
                    );
                }
                if record.error.is_none() && record.entries.is_empty() {
                    lines.push(Line::styled(
                        "  No steps recorded",
                        Style::default().fg(t.muted),
                    ));
                }
                for entry in &record.entries {
                    let chosen = state.selected == index;
                    let start = lines.len();
                    lines.extend(
                        wrap_text(
                            &format!("{} {}", if chosen { "›" } else { " " }, entry.summary),
                            area.width,
                        )
                        .into_iter()
                        .map(|l| {
                            l.style(Style::default().fg(if chosen { t.focus } else { t.text }))
                        }),
                    );
                    positions.push((start, lines.len(), index));
                    index += 1;
                }
            }
        }
        if !state.notice.is_empty() {
            let notice = wrap_text(&state.notice, area.width);
            let height = notice.len().min(area.height as usize) as u16;
            frame.render_widget(
                Paragraph::new(notice).style(Style::default().fg(t.agent_blocked)),
                Rect::new(
                    area.x,
                    area.bottom().saturating_sub(height),
                    area.width,
                    height,
                ),
            );
            area.height = area.height.saturating_sub(height);
        }
        let height = area.height as usize;
        let state = &mut self.dispatch;
        if let Some((start, end, _)) = positions.get(state.selected) {
            if *start < state.top {
                state.top = *start;
            }
            if *end > state.top + height {
                state.top = end.saturating_sub(height).min(*start);
            }
        }
        let top = state.top.min(lines.len().saturating_sub(height));
        for (start, end, index) in positions {
            let first = start.max(top);
            let last = end.min(top + height);
            if first < last {
                state.rows.push((
                    Rect::new(
                        area.x,
                        area.y + (first - top) as u16,
                        area.width,
                        (last - first) as u16,
                    ),
                    index,
                ));
            }
        }
        frame.render_widget(
            Paragraph::new(lines.into_iter().skip(top).take(height).collect::<Vec<_>>()),
            area,
        );
    }
}
