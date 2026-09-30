use super::*;

impl Panel {
    pub fn links_key(&self) -> Option<crate::links::Key> {
        let detail = self.content.as_ref()?;
        let body = self
            .live(detail)
            .map(|(_, t)| t.body.clone())
            .unwrap_or_else(|| detail.task.body.clone());
        let range = detail.data.as_ref().and_then(|d| {
            if d.task.status.as_deref() != Some("running") || d.evidence.git.state != "available" {
                return None;
            }
            let end = d.evidence.git.fields.get("head_sha")?.as_str()?;
            Some((d.task.start.clone()?, end.into()))
        });
        Some(crate::links::Key {
            project: self.project.clone(),
            seq: detail.seq,
            body,
            range,
        })
    }
    pub fn tick_links(&mut self) {
        if self.view == View::Links
            && matches!(self.page, Page::List)
            && self.read_error.is_none()
            && let Some(key) = self.links_key()
        {
            self.links.sync(key);
            self.links.tick();
        }
    }
    pub fn reading_link(&self) -> bool {
        self.view == View::Links && self.links.reading.is_some() && matches!(self.page, Page::List)
    }
    pub(super) fn draw_links(
        &mut self,
        t: &Theme,
        frame: &mut ratatui::Frame,
        mut area: ratatui::layout::Rect,
    ) {
        use ratatui::{
            layout::Rect,
            style::{Modifier, Style},
            text::Line,
            widgets::Paragraph,
        };
        self.links.rows.clear();
        if let Some(reading) = &mut self.links.reading {
            let (body, hits) = crate::buttons::draw_compact_top(
                t,
                frame,
                area,
                &[crate::buttons::Button::new("Back Esc", KeyCode::Esc, true)],
            );
            self.buttons.extend(hits);
            area = body;
            let mut lines = wrap_text(&reading.title, area.width);
            lines.push(Line::raw(""));
            if reading.commits.is_empty() {
                lines.extend(wrap_text(&reading.text, area.width));
            } else {
                lines.extend(wrap_text(
                    "Recorded range only; commits are not necessarily owned by this task.",
                    area.width,
                ));
                let mut selected_row = 0;
                let mut positions = Vec::new();
                for (i, (sha, subject)) in reading.commits.iter().enumerate() {
                    if i == reading.selected {
                        selected_row = lines.len();
                    }
                    let start = lines.len();
                    lines.extend(wrap_text(
                        &format!(
                            "{} {} {subject}",
                            if i == reading.selected { "›" } else { " " },
                            &sha[..12.min(sha.len())]
                        ),
                        area.width,
                    ));
                    positions.push((start, lines.len(), i));
                }
                if selected_row < reading.scroll {
                    reading.scroll = selected_row;
                }
                if selected_row >= reading.scroll + area.height as usize {
                    reading.scroll =
                        selected_row.saturating_sub(area.height.saturating_sub(1) as usize);
                }
                self.links.rows = positions
                    .into_iter()
                    .filter_map(|(start, end, index)| {
                        let first = start.max(reading.scroll);
                        let last = end.min(reading.scroll + area.height as usize);
                        (first < last).then(|| {
                            (
                                Rect::new(
                                    area.x,
                                    area.y + (first - reading.scroll) as u16,
                                    area.width,
                                    (last - first) as u16,
                                ),
                                index,
                            )
                        })
                    })
                    .collect();
            }
            reading.scroll = reading
                .scroll
                .min(lines.len().saturating_sub(area.height as usize));
            frame.render_widget(
                Paragraph::new(
                    lines
                        .into_iter()
                        .skip(reading.scroll)
                        .take(area.height as usize)
                        .collect::<Vec<_>>(),
                ),
                area,
            );
            return;
        }
        let mut lines = Vec::new();
        let mut positions = Vec::new();
        let mut group = "";
        for (index, link) in self.links.entries.iter().enumerate() {
            if group != link.group() {
                group = link.group();
                lines.push(Line::styled(
                    group,
                    Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
                ));
            }
            let start = lines.len();
            let marker = if self.links.selected == index {
                "›"
            } else {
                " "
            };
            let label = match &link.target {
                crate::links::Target::Agent {
                    instance: Some(instance),
                    ..
                } => format!("{} | instance={instance}", link.label),
                _ => link.label.clone(),
            };
            let style = Style::default().fg(if self.links.selected == index {
                t.focus
            } else {
                t.text
            });
            lines.extend(
                wrap_text(&format!("{marker} {label}"), area.width)
                    .into_iter()
                    .map(|l| l.style(style)),
            );
            lines.extend(
                wrap_text(&format!("  From: {}", link.sources.join(", ")), area.width)
                    .into_iter()
                    .map(|l| l.style(Style::default().fg(t.muted))),
            );
            if let Some(note) = &link.note {
                lines.extend(
                    wrap_text(note, area.width)
                        .into_iter()
                        .map(|l| l.style(Style::default().fg(t.agent_blocked))),
                );
            }
            positions.push((start, lines.len(), index));
        }
        if self.links.entries.is_empty() && self.links.message.is_empty() {
            lines.push(Line::raw("No explicit links"));
        }
        let mut notices = Vec::new();
        if !self.links.message.is_empty() {
            notices.push(self.links.message.clone());
        }
        if self.detail_key().is_some()
            && let Some(detail) = &self.content
        {
            if let Some(error) = &detail.error {
                notices.push(format!(
                    "{}\n{error}",
                    if detail.data.is_some() {
                        "Recorded range may be stale; details refresh failed:"
                    } else {
                        "Recorded range unavailable:"
                    }
                ));
            } else if detail.data.is_none() {
                notices.push("Loading recorded range…".into());
            }
        }
        if !notices.is_empty() {
            let notice = wrap_text(&notices.join("\n"), area.width);
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
        if let Some((start, end, _)) = positions.get(self.links.selected) {
            if *start < self.links.top {
                self.links.top = *start;
            }
            if *end > self.links.top + height {
                self.links.top = end.saturating_sub(height).min(*start);
            }
        }
        let top = self.links.top.min(lines.len().saturating_sub(height));
        for (start, end, index) in positions {
            let first = start.max(top);
            let last = end.min(top + height);
            if first < last {
                self.links.rows.push((
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
