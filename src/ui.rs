use crate::{
    agents::{Panel, group},
    corral::{Agent, Effort},
    git::{Head, Summary},
    input::Focus,
    layout::Panes,
    pty::Session,
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Default)]
pub struct Hits {
    pub(crate) buttons: Vec<crate::buttons::Hit>,
    pub agents: Vec<(u16, String)>,
    pub list: Rect,
    pub reply: Rect,
    pub queue_rows: Vec<(u16, usize)>,
}

pub struct View<'a> {
    pub colors: &'a Theme,
    pub panes: Panes,
    pub focus: Focus,
    pub showing: Option<&'a str>,
    pub viewer: Option<&'a Session>,
    pub queue: &'a mut crate::queue::Panel,
    pub viewer_note: &'a str,
    pub reply: &'a str,
    pub now: f64,
    pub pointer: &'a crate::buttons::Pointer,
}

pub fn draw(frame: &mut Frame, panel: &mut Panel, view: View<'_>) -> Hits {
    let t = view.colors;
    let screen_area = frame.area();
    frame.buffer_mut().set_style(screen_area, t.base());
    let mut hits = draw_agents(frame, panel, &view);
    hits.queue_rows = view
        .queue
        .draw(t, frame, view.panes.queue, view.focus == Focus::Queue);
    let title = view
        .showing
        .map(|name| format!("Viewer · {name}"))
        .unwrap_or_else(|| "Viewer".into());
    draw_terminal(frame, view.panes.viewer, &title, &view);
    let queue_modal = view.focus == Focus::Queue && view.queue.overlay_open();
    if queue_modal {
        view.queue.draw_overlay(t, frame);
        hits.buttons.clear();
        hits.agents.clear();
        hits.queue_rows.clear();
    }
    if !queue_modal && view.queue.overlay_open() {
        view.queue.buttons.clear();
        view.queue.fields.clear();
        view.queue.project_rows.clear();
    }
    if let Some(name) = &panel.confirm {
        let modal = crate::theme::centered(frame.area(), 64, 12);
        frame.render_widget(ratatui::widgets::Clear, modal);
        frame.render_widget(
            t.block(" Stop agent ", true)
                .style(t.base().bg(t.overlay))
                .border_style(Style::default().fg(t.danger)),
            modal,
        );
        let content = inner(modal);
        let (body, buttons) = crate::buttons::draw(
            t,
            frame,
            content,
            &[
                crate::buttons::Button::new("Cancel Esc", crossterm::event::KeyCode::Esc, true),
                crate::buttons::Button::new("Stop y", crossterm::event::KeyCode::Char('y'), true)
                    .danger(),
            ],
        );
        let agent = panel.agents.iter().find(|a| &a.name == name);
        let text = format!(
            "Stop {name}?\nInstance: {}\nActivity: {}\n\nThis stops the agent, not just the Viewer connection.\nPress y or click Stop to confirm; any other key cancels.",
            agent
                .and_then(|a| a.instance.as_deref())
                .unwrap_or("unknown"),
            agent.and_then(|a| a.last_tool.as_deref()).unwrap_or("—")
        );
        frame.render_widget(Paragraph::new(text).wrap(Default::default()), body);
        hits.buttons = buttons;
        hits.agents.clear();
        hits.queue_rows.clear();
        view.queue.buttons.clear();
        view.queue.fields.clear();
        view.queue.project_rows.clear();
    }
    let controls: Vec<_> = hits
        .buttons
        .iter()
        .cloned()
        .map(|h| (Focus::Agents, h))
        .chain(
            view.queue
                .buttons
                .iter()
                .cloned()
                .map(|h| (Focus::Queue, h)),
        )
        .collect();
    view.pointer.paint(t, frame, &controls);
    if !view.panes.tabs.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " Agents ",
                    Style::default().fg(if !view.panes.agents.is_empty() {
                        t.focus
                    } else {
                        t.muted
                    }),
                ),
                Span::styled(
                    " Queue ",
                    Style::default().fg(if !view.panes.queue.is_empty() {
                        t.focus
                    } else {
                        t.muted
                    }),
                ),
            ])),
            view.panes.tabs,
        );
    }
    let (mut target, mut help) = match view.focus {
        Focus::Agents => (
            "Agents".to_string(),
            " ↑↓ Select  ↵ Attach  PgUp/Dn Scroll  Tab Queue  q Quit",
        ),
        Focus::Queue => (
            "Queue".to_string(),
            " ↑↓ Select  Enter Details  c Projects  a Add  A All pending  ? Help  Ctrl-] Agents",
        ),
        Focus::Viewer => (
            view.showing.unwrap_or("Viewer · disconnected").to_string(),
            " Keys go to terminal  Ctrl-] Agents",
        ),
    };
    if queue_modal {
        match &view.queue.page {
            crate::queue::Page::Add { body_focus, .. }
            | crate::queue::Page::Edit { body_focus, .. } => {
                target = format!(
                    "{} · {}",
                    if matches!(view.queue.page, crate::queue::Page::Edit { .. }) {
                        "Edit"
                    } else {
                        "Add"
                    },
                    if *body_focus { "Body" } else { "Title" }
                );
                help = " Tab Switch  Ctrl-S Save  Esc Cancel";
            }
            crate::queue::Page::Projects => {
                target = "Projects".into();
                help = " ↑↓ Select  Enter Open  e Path  Esc Cancel";
            }
            crate::queue::Page::Project(_) => {
                target = "Project path".into();
                help = " Enter Apply  Ctrl-U Clear  Esc Cancel";
            }
            crate::queue::Page::Delete { .. } => {
                target = "Confirm delete".into();
                help = " y Delete  Esc Cancel  PgUp/PgDn Scroll";
            }
            crate::queue::Page::AllPending => {
                target = "All pending".into();
                help = " Wheel / PgUp/PgDn Scroll  r Refresh  Esc Back  Ctrl-] Agents";
            }
            _ => {
                target = "Queue · Details / Result".into();
                help = " Wheel / PgUp/PgDn Scroll  Esc Back  Ctrl-] Agents";
            }
        }
    }
    if view.focus == Focus::Queue && matches!(view.queue.page, crate::queue::Page::Detail(_)) {
        target = "Queue · Task details".into();
        help = " ↑↓ / Wheel Scroll  PgUp/PgDn Page  Esc Back  Ctrl-] Agents";
    }
    if panel.confirm.is_some() {
        target = "Confirm stop".into();
        help = " y Stop  Any other key cancels";
    }
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" Input ▸ {target} "),
                Style::default().fg(t.input_text).bg(t.focus),
            ),
            Span::styled(
                if panel.confirm.is_some() || queue_modal {
                    help
                } else if view.focus == Focus::Queue && !view.queue.message.is_empty() {
                    &view.queue.message
                } else if panel.message.is_empty() {
                    help
                } else {
                    &panel.message
                },
                Style::default().fg(t.muted),
            ),
        ])),
        view.panes.status,
    );
    hits
}

pub fn inner(area: Rect) -> Rect {
    Block::default().borders(Borders::ALL).inner(area)
}
fn border(t: &Theme, title: &str, focused: bool) -> Block<'static> {
    t.block(title.to_string(), focused)
}
fn draw_terminal(frame: &mut Frame, area: Rect, title: &str, view: &View<'_>) {
    let t = view.colors;
    let focused = view.focus == Focus::Viewer;
    let connected = view.showing.is_some();
    let block = border(t, title, focused).title_top(
        Line::styled(
            if connected {
                " ◉ connected "
            } else {
                " disconnected "
            },
            Style::default().fg(if connected { t.connected } else { t.muted }),
        )
        .right_aligned(),
    );
    frame.render_widget(block, area);
    let area = inner(area);
    if let Some(session) = view.viewer {
        let screen = session.screen.lock().unwrap();
        let cursor = screen.render(area, frame.buffer_mut());
        if focused && let Some(cursor) = cursor {
            frame.set_cursor_position(cursor);
        }
    } else {
        frame.render_widget(
            Paragraph::new(view.viewer_note).wrap(Default::default()),
            area,
        );
    }
}

struct Row {
    line: Line<'static>,
    name: Option<String>,
    headline: bool,
}
fn draw_agents(frame: &mut Frame, panel: &mut Panel, view: &View<'_>) -> Hits {
    let t = view.colors;
    let area = view.panes.agents;
    if area.is_empty() {
        return Hits::default();
    }
    let focused = view.focus == Focus::Agents;
    let title = format!(" Agents · {} ", panel.agents.len());
    let block = border(t, &title, focused)
        .title_style(Style::default().fg(t.text).add_modifier(Modifier::BOLD));
    frame.render_widget(block.clone(), area);
    let inside = inner(area);
    if inside.is_empty() {
        return Hits::default();
    }
    use crate::buttons::{self, Button};
    use crossterm::event::KeyCode as K;
    let selected = panel.selected.is_some();
    let connected = selected && panel.selected.as_deref() == view.showing;
    let (content, buttons) = buttons::draw_compact(
        t,
        frame,
        inside,
        &[
            Button::new(
                if connected { "Attached" } else { "Attach ↵" },
                K::Enter,
                selected && !connected,
            ),
            Button::new(
                if panel.by_state { "Name s" } else { "Sort s" },
                K::Char('s'),
                true,
            ),
            Button::new(
                if panel.stopping { "Stopping" } else { "Stop x" },
                K::Char('x'),
                selected && !panel.stopping,
            )
            .danger(),
        ],
    );
    let detail_height = if panel.show_reply && selected && content.height >= 6 {
        (content.height / 3).max(3)
    } else {
        0
    };
    let list = Rect::new(
        content.x,
        content.y,
        content.width.saturating_sub(1),
        content.height - detail_height,
    );
    let mut hits = Hits {
        buttons,
        list,
        ..Default::default()
    };
    let rows = agent_rows(
        t,
        panel,
        view.showing,
        usize::from(list.width),
        view.now,
        focused,
    );
    let selected_rows: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.name.is_some() && r.name == panel.selected)
        .map(|(i, _)| i)
        .collect();
    if panel.follow
        && let (Some(first), Some(last)) = (selected_rows.first(), selected_rows.last())
    {
        panel.top = panel
            .top
            .max((last + 1).saturating_sub(usize::from(list.height)))
            .min(*first);
    }
    panel.top = panel
        .top
        .min(rows.len().saturating_sub(usize::from(list.height)));
    for (offset, row) in rows
        .iter()
        .skip(panel.top)
        .take(usize::from(list.height))
        .enumerate()
    {
        let y = list.y + offset as u16;
        frame.render_widget(
            Paragraph::new(row.line.clone()).style(row.line.style),
            Rect::new(list.x, y, list.width, 1),
        );
        if let Some(name) = &row.name {
            hits.agents.push((y, name.clone()));
        }
    }
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No agents").style(Style::default().fg(t.muted)),
            list,
        );
    }
    if rows.len() > usize::from(list.height) && list.height > 0 {
        scrollbar(
            t,
            frame,
            Rect {
                width: inside.width,
                ..list
            },
            rows.len(),
            panel.top,
        );
        let above = rows[..panel.top].iter().filter(|r| r.headline).count();
        let below = rows
            .iter()
            .skip(panel.top + usize::from(list.height))
            .filter(|r| r.headline)
            .count();
        // Keep the repository visible when its root has scrolled above the viewport.
        let context = rows[panel.top]
            .name
            .as_deref()
            .map(|name| {
                let prefix = group(name);
                format!("{} · ", if prefix.is_empty() { "agents/" } else { prefix })
            })
            .unwrap_or_default();
        frame.render_widget(
            block.title_bottom(Line::styled(
                format!(" {context}↑{above} ↓{below} "),
                Style::default().fg(t.muted),
            )),
            area,
        );
    }
    if detail_height > 0 {
        let heading = format!("─ Last reply · {}", panel.selected.as_deref().unwrap_or(""));
        frame.render_widget(
            Paragraph::new(heading).style(Style::default().fg(t.bright)),
            Rect::new(content.x, list.bottom(), content.width, 1),
        );
        hits.reply = Rect::new(
            content.x,
            list.bottom() + 1,
            content.width.saturating_sub(1),
            detail_height - 1,
        );
        let lines = reply_lines(t, view.reply, usize::from(hits.reply.width));
        panel.reply_top = panel
            .reply_top
            .min(lines.len().saturating_sub(usize::from(hits.reply.height)));
        frame.render_widget(
            Paragraph::new(lines.clone())
                .scroll((panel.reply_top.min(u16::MAX as usize) as u16, 0)),
            hits.reply,
        );
        if lines.len() > usize::from(hits.reply.height) {
            scrollbar(
                t,
                frame,
                Rect {
                    width: content.width,
                    ..hits.reply
                },
                lines.len(),
                panel.reply_top,
            );
        }
    }
    hits
}
fn scrollbar(t: &Theme, frame: &mut Frame, area: Rect, len: usize, top: usize) {
    // Ratatui's position range must match viewport offsets, not the number of rendered rows.
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(t.muted))
            .track_style(Style::default().fg(t.border)),
        area,
        &mut ScrollbarState::new(len.saturating_sub(usize::from(area.height)) + 1)
            .viewport_content_length(usize::from(area.height))
            .position(top),
    );
}
fn agent_rows(
    t: &Theme,
    panel: &Panel,
    showing: Option<&str>,
    width: usize,
    now: f64,
    focused: bool,
) -> Vec<Row> {
    let ordered = panel.ordered(now);
    let mut rows = Vec::new();
    let mut previous = None;
    let wide = width >= 46;
    let mut name_width = 8;
    // The effort column exists only when some agent carries a known delegated effort label.
    let effort_column = ordered.iter().any(|a| a.effort().is_some());
    for (index, a) in ordered.iter().enumerate() {
        let prefix = group(&a.name);
        if previous != Some(prefix) {
            if wide {
                let longest = ordered[index..]
                    .iter()
                    .take_while(|agent| group(&agent.name) == prefix)
                    .map(|agent| {
                        agent
                            .name
                            .strip_prefix(prefix)
                            .unwrap_or(&agent.name)
                            .width()
                    })
                    .max()
                    .unwrap_or(8);
                // Reserve tree/status icon, type, effort, state and the right-hand activity badge.
                let reserved = if effort_column { 36 } else { 33 };
                name_width = longest.clamp(8, width.saturating_sub(reserved).max(8));
            }
            rows.push(Row {
                line: {
                    let count = ordered[index..]
                        .iter()
                        .take_while(|agent| group(&agent.name) == prefix)
                        .count();
                    let count = format!("({count})");
                    let name_width = width.saturating_sub(count.width() + 1);
                    let name = clip(
                        if prefix.is_empty() { "agents/" } else { prefix },
                        name_width,
                    );
                    Line::from(vec![
                        Span::styled(
                            pad(&name, width.saturating_sub(count.width())),
                            Style::default()
                                .fg(t.connected)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(count, Style::default().fg(t.muted)),
                    ])
                },
                name: None,
                headline: false,
            });
            previous = Some(prefix);
        }
        let last = ordered
            .get(index + 1)
            .is_none_or(|next| group(&next.name) != prefix);
        let selected = panel.selected.as_deref() == Some(&a.name);
        let style = if selected {
            Style::default().bg(t.agent_selected)
        } else {
            Style::default()
        };
        let tree_color = if selected { t.muted } else { t.dim };
        let (icon, state, color) = state(t, a, panel, now);
        let (brand_label, brand_color) = agent_brand(t, a.kind.as_deref().unwrap_or(""));
        let name = a.name.strip_prefix(prefix).unwrap_or(&a.name);
        let mut spans = vec![
            Span::styled(
                if selected { "▎" } else { " " },
                Style::default().fg(if focused { t.focus } else { t.muted }),
            ),
            Span::styled(
                if last { "└─ " } else { "├─ " },
                Style::default().fg(tree_color),
            ),
            Span::styled(format!("{icon} "), Style::default().fg(color)),
            Span::styled(
                pad(&clip(name, name_width), name_width),
                Style::default().fg(if selected { t.bright } else { t.text }),
            ),
        ];
        if wide {
            spans.push(Span::styled(
                format!(" {} ", pad(&clip(&brand_label, 8), 8)),
                Style::default().fg(brand_color).add_modifier(
                    if a.kind
                        .as_deref()
                        .is_some_and(|kind| kind.eq_ignore_ascii_case("codex"))
                    {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    },
                ),
            ));
        }
        if effort_column {
            spans.push(Span::raw(" "));
            spans.extend(effort_bars(t, a.effort()));
        }
        spans.push(Span::styled(
            format!(" {state}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        let badge = if showing == Some(&a.name) {
            "◉"
        } else if panel.unread.contains(&a.name) {
            "new"
        } else {
            ""
        };
        let suffix = format!(" {badge} {}", seconds(a.last_output.map(|v| now - v)));
        let used: usize = spans.iter().map(|s| s.content.width()).sum();
        spans.push(Span::raw(
            " ".repeat(width.saturating_sub(used + suffix.width())),
        ));
        spans.push(Span::styled(
            suffix,
            Style::default().fg(if showing == Some(&a.name) {
                t.connected
            } else if !badge.is_empty() {
                t.unread
            } else {
                t.muted
            }),
        ));
        rows.push(Row {
            line: Line::from(spans).style(style),
            name: Some(a.name.clone()),
            headline: true,
        });
        let mut details = vec![
            (
                format!(
                    "{}{} · ATT {} · VIA {}",
                    if wide && brand_label.width() <= 8 {
                        String::new()
                    } else {
                        format!("{} ", a.kind.as_deref().unwrap_or("—"))
                    },
                    a.instance
                        .as_deref()
                        .unwrap_or("—")
                        .chars()
                        .take(6)
                        .collect::<String>(),
                    a.attached,
                    a.last_input_source.as_deref().unwrap_or("—")
                ),
                Style::default().fg(t.working),
            ),
            (
                short_path(a.cwd.as_deref().unwrap_or("—")),
                Style::default().fg(t.muted).add_modifier(Modifier::DIM),
            ),
            (
                a.title.as_deref().unwrap_or("—").to_owned(),
                Style::default().fg(t.text),
            ),
        ];
        if name.width() > name_width {
            details.insert(0, (name.to_owned(), Style::default().fg(t.text)));
        }
        // The Git line goes right below the path, which sits just before the title here.
        let path_index = details.len() - 2;
        if matches!(a.state.as_deref(), Some("working" | "blocked")) {
            details.push((
                format!(
                    "DOING {} · {}",
                    a.last_tool.as_deref().unwrap_or("thinking"),
                    seconds(a.turn_started.map(|v| now - v))
                ),
                Style::default().fg(color),
            ));
        }
        if let Some(error) = &a.error {
            details.push((format!("ERROR {error}"), Style::default().fg(t.danger)));
        }
        if a.incompatible {
            details.push((
                format!("Incompatible protocol {}", a.proto.unwrap_or(0)),
                Style::default().fg(t.danger),
            ));
        }
        let mut lines = Vec::new();
        for (i, (detail, detail_style)) in details.into_iter().enumerate() {
            lines.extend(
                reply_lines(t, &detail, width.saturating_sub(6))
                    .into_iter()
                    .map(|line| vec![Span::styled(line.to_string(), detail_style)]),
            );
            if i == path_index
                && let Some(cwd) = &a.cwd
            {
                lines.extend(wrap_spans(
                    git_spans(t, panel.git.get(cwd)),
                    width.saturating_sub(6),
                ));
            }
        }
        for spans in lines {
            let mut line = vec![Span::styled(
                if last { "      " } else { " │    " },
                Style::default().fg(tree_color),
            )];
            line.extend(spans);
            rows.push(Row {
                line: Line::from(line).style(style),
                name: Some(a.name.clone()),
                headline: false,
            });
        }
        if index + 1 < ordered.len() {
            rows.push(Row {
                line: Line::styled(if last { "" } else { " │" }, Style::default().fg(t.dim)),
                name: None,
                headline: false,
            });
        }
    }
    rows
}
// Text approximations of brand marks; no icon font or terminal image protocol required.
fn agent_brand(t: &Theme, kind: &str) -> (String, Color) {
    let (mark, color) = match kind.to_ascii_lowercase().as_str() {
        "claude" => ("✳", t.claude),
        "codex" => (">_", t.codex),
        "pi" => ("π", t.pi),
        "omp" => ("π", t.omp),
        _ => return (kind.to_owned(), t.muted),
    };
    (format!("{mark} {kind}"), color)
}

// Pack the first two bars into one braille cell to keep the staircase compact. Each tier has
// its own shape and theme color; neither changes with selection. Unknown stays blank.
fn effort_bars(t: &Theme, effort: Option<Effort>) -> Vec<Span<'static>> {
    let (first, last, color) = match effort {
        Some(Effort::Medium) => ("⣄", "⡀", t.agent_idle),
        Some(Effort::High) => ("⣴", "⡀", t.agent_working),
        Some(Effort::Xhigh) => ("⣴", "⡇", t.agent_starting),
        None => return vec![Span::raw("  ")],
    };
    vec![
        Span::styled(first, Style::default().fg(color)),
        Span::styled(
            last,
            Style::default().fg(if effort == Some(Effort::Xhigh) {
                color
            } else {
                t.dim
            }),
        ),
    ]
}

// Git state of the agent's directory: a missing entry is still loading, `None` is unavailable,
// and fields Git could not determine show — rather than zero.
fn git_spans(t: &Theme, git: Option<&Option<Summary>>) -> Vec<Span<'static>> {
    let muted = Style::default().fg(t.muted);
    let Some(git) = git else {
        return vec![Span::styled("git …", muted)];
    };
    let Some(s) = git else {
        return vec![Span::styled("git unavailable", muted)];
    };
    let head = match &s.head {
        Head::Branch(branch) => branch.as_str(),
        Head::Detached => "HEAD detached",
        Head::Unknown => "—",
    };
    let ahead = s
        .ahead
        .as_ref()
        .map_or("C—".into(), |(count, base)| format!("C{count}({base})"));
    let mut spans = vec![Span::styled(format!("{head} · {ahead} · "), muted)];
    match &s.changes {
        Some(changes) => {
            spans.push(Span::styled(
                format!("+{}", changes.added),
                Style::default().fg(t.agent_idle),
            ));
            spans.push(Span::styled(" ", muted));
            spans.push(Span::styled(
                format!("-{}", changes.deleted),
                Style::default().fg(t.agent_error),
            ));
            if changes.binary > 0 {
                spans.push(Span::styled(format!(" · {} binary", changes.binary), muted));
            }
        }
        None => spans.push(Span::styled("+— -—", muted)),
    }
    let untracked = s.untracked.map_or("—".into(), |n| n.to_string());
    spans.push(Span::styled(format!(" · ?{untracked}"), muted));
    spans
}

// Wraps styled spans by display width, keeping each character's style.
fn wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Vec<Span<'static>>> {
    let mut lines = vec![Vec::new()];
    let mut used = 0;
    for span in spans {
        let mut piece = String::new();
        for c in span.content.chars() {
            let w = c.width().unwrap_or(0);
            if used + w > width && used > 0 {
                if !piece.is_empty() {
                    lines
                        .last_mut()
                        .unwrap()
                        .push(Span::styled(std::mem::take(&mut piece), span.style));
                }
                lines.push(Vec::new());
                used = 0;
            }
            piece.push(c);
            used += w;
        }
        if !piece.is_empty() {
            lines
                .last_mut()
                .unwrap()
                .push(Span::styled(piece, span.style));
        }
    }
    lines
}

fn short_path(path: &str) -> String {
    let parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() > 2 {
        format!("…/{}", parts[parts.len() - 2..].join("/"))
    } else {
        path.to_owned()
    }
}

fn state(t: &Theme, a: &Agent, panel: &Panel, now: f64) -> (&'static str, &'static str, Color) {
    if a.error.is_some() || a.incompatible {
        return ("!", "error", t.agent_error);
    }
    if panel.suspect(a, now) {
        return ("▲", "stalled", t.agent_stalled);
    }
    if a.starting {
        return ("◌", "starting", t.agent_starting);
    }
    match a.state.as_deref() {
        Some("working") => (
            ["◐", "◓", "◑", "◒"][(now * 3.0) as usize % 4],
            "working",
            t.agent_working,
        ),
        Some("blocked") => ("◆", "blocked", t.agent_blocked),
        Some("idle") => ("○", "idle", t.agent_idle),
        Some("starting") => ("◌", "starting", t.agent_starting),
        _ => ("·", "unknown", t.muted),
    }
}
fn seconds(value: Option<f64>) -> String {
    value
        .map(|s| {
            if s >= 3600.0 {
                format!("{:.1}h", s / 3600.0)
            } else if s >= 60.0 {
                format!("{}m", (s / 60.0) as u64)
            } else {
                format!("{}s", s.max(0.0) as u64)
            }
        })
        .unwrap_or_else(|| "—".into())
}
pub(crate) fn pad(text: &str, width: usize) -> String {
    format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
}
pub(crate) fn clip(text: &str, width: usize) -> String {
    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
    if clean.width() <= width {
        return clean;
    }
    if width == 0 {
        return String::new();
    }
    let (mut used, mut result) = (0, String::new());
    for c in clean.chars() {
        let w = c.width().unwrap_or(0);
        if used + w >= width {
            break;
        }
        result.push(c);
        used += w;
    }
    result.push('…');
    result
}
fn reply_lines(t: &Theme, text: &str, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut code = false;
    for raw in text.lines() {
        if raw.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        let style = if code {
            Style::default().fg(t.reply_code)
        } else if raw.starts_with('#') {
            Style::default()
                .fg(t.reply_heading)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let mut line = String::new();
        let mut used = 0;
        for c in raw
            .replace('\t', "    ")
            .chars()
            .filter(|c| !c.is_control())
        {
            let w = c.width().unwrap_or(0);
            if used + w > width && !line.is_empty() {
                result.push(Line::styled(std::mem::take(&mut line), style));
                used = 0;
            }
            line.push(c);
            used += w;
        }
        result.push(Line::styled(line, style));
    }
    result
}
