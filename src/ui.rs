use crate::{
    agents::{Panel, group},
    corral::Agent,
    input::Focus,
    layout::Panes,
    pty::Session,
    theme as t,
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
    let screen_area = frame.area();
    frame
        .buffer_mut()
        .set_style(screen_area, crate::theme::base());
    for area in [view.panes.agents, view.panes.queue, view.panes.tabs] {
        frame
            .buffer_mut()
            .set_style(area, Style::default().bg(Color::Reset));
    }
    let mut hits = draw_agents(frame, panel, &view);
    hits.queue_rows = view
        .queue
        .draw(frame, view.panes.queue, view.focus == Focus::Queue);
    let title = view
        .showing
        .map(|name| format!("Viewer · {name}"))
        .unwrap_or_else(|| "Viewer".into());
    draw_terminal(
        frame,
        view.panes.viewer,
        &title,
        view.focus == Focus::Viewer,
        view.viewer,
        view.viewer_note,
        view.showing.is_some(),
    );
    let queue_modal = view.focus == Focus::Queue && view.queue.overlay_open();
    if queue_modal {
        view.queue.draw_overlay(frame);
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
            crate::theme::block(" 停止 agent ", true)
                .style(Style::default().bg(crate::theme::OVERLAY))
                .border_style(Style::default().fg(crate::theme::DANGER)),
            modal,
        );
        let content = inner(modal);
        let (body, buttons) = crate::buttons::draw(
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
            "停止 {name}？\n实例：{}\n当前活动：{}\n\n这会停止该 agent，不只是断开 Viewer。\n仅 y 或确认停止按钮执行；其他键取消。",
            agent.and_then(|a| a.instance.as_deref()).unwrap_or("未知"),
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
    view.pointer.paint(frame, &controls);
    if !view.panes.tabs.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " Agents ",
                    Style::default().fg(if !view.panes.agents.is_empty() {
                        crate::theme::FOCUS
                    } else {
                        crate::theme::MUTED
                    }),
                ),
                Span::styled(
                    " Queue ",
                    Style::default().fg(if !view.panes.queue.is_empty() {
                        crate::theme::FOCUS
                    } else {
                        crate::theme::MUTED
                    }),
                ),
            ])),
            view.panes.tabs,
        );
    }
    let (mut target, mut help) = match view.focus {
        Focus::Agents => (
            "Agents".to_string(),
            " ↑↓ 选择  ↵ 接入  r 回复  PgUp/Dn 滚动  Tab 队列  q 退出",
        ),
        Focus::Queue => (
            "Queue".to_string(),
            " ↑↓ 选择  Enter 详情  c 项目  a 新增  ? 帮助  Ctrl-] Agents",
        ),
        Focus::Viewer => (
            view.showing.unwrap_or("Viewer · 未连接").to_string(),
            " 按键发送到终端  Ctrl-] 返回 Agents",
        ),
    };
    if queue_modal {
        match &view.queue.page {
            crate::queue::Page::Add { body_focus, .. } => {
                target = format!("新增 · {}", if *body_focus { "正文" } else { "标题" });
                help = " Tab 切字段  Ctrl-S 保存  Esc 取消";
            }
            crate::queue::Page::Projects => {
                target = "项目选择".into();
                help = " ↑↓ 选择  Enter 切换  e 手动目录  Esc 取消";
            }
            crate::queue::Page::Project(_) => {
                target = "项目目录".into();
                help = " Enter 应用  Ctrl-U 清空  Esc 取消";
            }
            _ => {
                target = "Queue · 详情/反馈".into();
                help = " PgUp/PgDn 滚动  Esc 返回  Ctrl-] Agents";
            }
        }
    }
    if panel.confirm.is_some() {
        target = "停止确认".into();
        help = " y 确认停止  其他键取消";
    }
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" 输入 ▸ {target} "),
                Style::default()
                    .fg(crate::theme::BG)
                    .bg(crate::theme::FOCUS),
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
                Style::default().fg(crate::theme::MUTED),
            ),
        ])),
        view.panes.status,
    );
    hits
}

pub fn inner(area: Rect) -> Rect {
    Block::default().borders(Borders::ALL).inner(area)
}
fn border(title: &str, focused: bool) -> Block<'static> {
    crate::theme::block(title.to_string(), focused)
}
fn draw_terminal(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    session: Option<&Session>,
    note: &str,
    connected: bool,
) {
    let block = border(title, focused).title_top(
        Line::styled(
            if connected {
                " ◉ 已连接 "
            } else {
                " 未连接 "
            },
            Style::default().fg(if connected { t::CONNECTED } else { t::MUTED }),
        )
        .right_aligned(),
    );
    frame.render_widget(block, area);
    let area = inner(area);
    if let Some(session) = session {
        let screen = session.screen.lock().unwrap();
        let cursor = screen.render(area, frame.buffer_mut());
        if focused && let Some(cursor) = cursor {
            frame.set_cursor_position(cursor);
        }
    } else {
        frame.render_widget(Paragraph::new(note).wrap(Default::default()), area);
    }
}

struct Row {
    line: Line<'static>,
    name: Option<String>,
    headline: bool,
}
fn draw_agents(frame: &mut Frame, panel: &mut Panel, view: &View<'_>) -> Hits {
    let area = view.panes.agents;
    if area.is_empty() {
        return Hits::default();
    }
    let focused = view.focus == Focus::Agents;
    let block = border(" Agents ", focused).title_bottom(Line::styled(
        format!(" {} agents ", panel.agents.len()),
        Style::default().fg(t::DIM),
    ));
    frame.render_widget(block, area);
    let inside = inner(area);
    if inside.is_empty() {
        return Hits::default();
    }
    use crate::buttons::{self, Button};
    use crossterm::event::KeyCode as K;
    let selected = panel.selected.is_some();
    let connected = selected && panel.selected.as_deref() == view.showing;
    let (content, buttons) = buttons::draw_compact(
        frame,
        inside,
        &[
            Button::new(
                if connected { "Attached" } else { "Attach ↵" },
                K::Enter,
                selected && !connected,
            ),
            Button::new(
                if panel.show_reply {
                    "Hide r"
                } else {
                    "Reply r"
                },
                K::Char('r'),
                selected,
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
            Paragraph::new("暂无 agent").style(Style::default().fg(t::MUTED)),
            list,
        );
    }
    if rows.len() > usize::from(list.height) && list.height > 0 {
        scrollbar(
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
            border(&format!(" Agents · {context}↑{above} ↓{below} "), focused),
            area,
        );
    }
    if detail_height > 0 {
        let heading = format!("─ Last reply · {}", panel.selected.as_deref().unwrap_or(""));
        frame.render_widget(
            Paragraph::new(heading).style(Style::default().fg(t::BRIGHT)),
            Rect::new(content.x, list.bottom(), content.width, 1),
        );
        hits.reply = Rect::new(
            content.x,
            list.bottom() + 1,
            content.width.saturating_sub(1),
            detail_height - 1,
        );
        let lines = reply_lines(view.reply, usize::from(hits.reply.width));
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
fn scrollbar(frame: &mut Frame, area: Rect, len: usize, top: usize) {
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(t::MUTED))
            .track_style(Style::default().fg(t::BORDER)),
        area,
        &mut ScrollbarState::new(len)
            .viewport_content_length(usize::from(area.height))
            .position(top),
    );
}
fn agent_rows(
    panel: &Panel,
    showing: Option<&str>,
    width: usize,
    now: f64,
    focused: bool,
) -> Vec<Row> {
    let ordered = panel.ordered(now);
    let mut rows = Vec::new();
    let mut previous = None;
    for (index, a) in ordered.iter().enumerate() {
        let prefix = group(&a.name);
        if previous != Some(prefix) {
            rows.push(Row {
                line: Line::styled(
                    if prefix.is_empty() { "agents/" } else { prefix }.to_string(),
                    Style::default()
                        .fg(t::CONNECTED)
                        .add_modifier(Modifier::BOLD),
                ),
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
            Style::default().bg(t::SELECTED)
        } else {
            Style::default()
        };
        let (icon, state, color) = state(a, panel, now);
        let wide = width >= 46;
        let name_width = 8;
        let (brand_label, brand_color) = agent_brand(a.kind.as_deref().unwrap_or(""));
        let name = a.name.strip_prefix(prefix).unwrap_or(&a.name);
        let mut spans = vec![
            Span::styled(
                if selected { "▎" } else { " " },
                Style::default().fg(if focused { t::FOCUS } else { t::MUTED }),
            ),
            Span::styled(
                if last { "└─ " } else { "├─ " },
                Style::default().fg(t::DIM),
            ),
            Span::styled(format!("{icon} "), Style::default().fg(color)),
            Span::styled(
                pad(&clip(name, name_width), name_width),
                Style::default().fg(if selected { t::BRIGHT } else { t::TEXT }),
            ),
        ];
        if wide {
            spans.push(Span::styled(
                format!(" {} ", pad(&clip(&brand_label, 8), 8)),
                Style::default().fg(brand_color),
            ));
        }
        spans.push(Span::styled(
            format!(" {state}"),
            Style::default().fg(color),
        ));
        let badge = if showing == Some(&a.name) {
            "◉"
        } else if panel.unread.contains(&a.name) {
            "新"
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
                t::CONNECTED
            } else if !badge.is_empty() {
                t::UNREAD
            } else {
                t::MUTED
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
                    "{} {} · ATT {} · VIA {}",
                    a.kind.as_deref().unwrap_or("—"),
                    a.instance.as_deref().unwrap_or("—"),
                    a.attached,
                    a.last_input_source.as_deref().unwrap_or("—")
                ),
                t::WORKING,
            ),
            (short_path(a.cwd.as_deref().unwrap_or("—")), t::MUTED),
            (a.title.as_deref().unwrap_or("—").to_owned(), t::TEXT),
        ];
        if name.width() > name_width {
            details.insert(0, (name.to_owned(), t::TEXT));
        }
        if matches!(a.state.as_deref(), Some("working" | "blocked")) {
            details.push((
                format!(
                    "DOING {} · {}",
                    a.last_tool.as_deref().unwrap_or("thinking"),
                    seconds(a.turn_started.map(|v| now - v))
                ),
                color,
            ));
        }
        if let Some(error) = &a.error {
            details.push((format!("ERROR {error}"), t::DANGER));
        }
        if a.incompatible {
            details.push((
                format!("Incompatible protocol {}", a.proto.unwrap_or(0)),
                t::DANGER,
            ));
        }
        for (detail, color) in details {
            for line in reply_lines(&detail, width.saturating_sub(6)) {
                rows.push(Row {
                    line: Line::from(vec![
                        Span::styled(
                            if last { "      " } else { " │    " },
                            Style::default().fg(t::DIM),
                        ),
                        Span::styled(line.to_string(), Style::default().fg(color)),
                    ])
                    .style(style),
                    name: Some(a.name.clone()),
                    headline: false,
                });
            }
        }
    }
    rows
}
// Text approximations of brand marks; no icon font or terminal image protocol required.
fn agent_brand(kind: &str) -> (String, Color) {
    let (mark, color) = match kind.to_ascii_lowercase().as_str() {
        "claude" => ("✳", Color::Rgb(0xd9, 0x77, 0x57)),
        "codex" => (">_", Color::Rgb(0xff, 0xff, 0xff)),
        "pi" => ("π", Color::Rgb(0xff, 0xff, 0xff)),
        "omp" => ("π", Color::Rgb(0xa8, 0x55, 0xf7)),
        _ => return (kind.to_owned(), t::MUTED),
    };
    (format!("{mark} {kind}"), color)
}

fn short_path(path: &str) -> String {
    let parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() > 2 {
        format!("…/{}", parts[parts.len() - 2..].join("/"))
    } else {
        path.to_owned()
    }
}

fn state(a: &Agent, panel: &Panel, now: f64) -> (&'static str, &'static str, Color) {
    if a.error.is_some() || a.incompatible {
        return ("!", "异常", t::DANGER);
    }
    if panel.suspect(a, now) {
        return ("▲", "无进展", t::WARNING);
    }
    if a.starting {
        return ("◌", "启动中", t::MUTED);
    }
    match a.state.as_deref() {
        Some("working") => (
            ["◐", "◓", "◑", "◒"][(now * 3.0) as usize % 4],
            "工作中",
            t::WORKING,
        ),
        Some("blocked") => ("◆", "待处理", t::BLOCKED),
        Some("idle") => ("○", "空闲", t::MUTED),
        Some("starting") => ("◌", "启动中", t::MUTED),
        _ => ("·", "未知", t::MUTED),
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
fn reply_lines(text: &str, width: usize) -> Vec<Line<'static>> {
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
            Style::default().fg(Color::Yellow)
        } else if raw.starts_with('#') {
            Style::default()
                .fg(Color::Cyan)
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
