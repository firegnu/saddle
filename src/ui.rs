use crate::{
    agents::{Panel, group},
    corral::Agent,
    input::Focus,
    layout::Panes,
    pty::Session,
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
    );
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
    let (target, help) = match view.focus {
        Focus::Agents => (
            "Agents".to_string(),
            " ↑↓ 选择  Enter 接入  r 回复  s 排序  x 停止  Tab 队列  q 退出",
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
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" 输入 ▸ {target} "),
                Style::default()
                    .fg(crate::theme::BG)
                    .bg(crate::theme::FOCUS),
            ),
            Span::styled(
                if panel.message.is_empty() {
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
) {
    frame.render_widget(border(title, focused), area);
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
    frame.render_widget(border("Agents", view.focus == Focus::Agents), area);
    let inside = inner(area);
    if inside.is_empty() {
        return Hits::default();
    }
    let help = "↑↓ 选择 · Tab 队列 · q 退出";
    let footer = Rect::new(inside.x, inside.bottom() - 1, inside.width, 1);
    frame.render_widget(
        Paragraph::new(if panel.message.is_empty() {
            help
        } else {
            &panel.message
        })
        .style(Style::default().fg(Color::Yellow)),
        footer,
    );
    use crate::buttons::{self, Button};
    use crossterm::event::KeyCode as K;
    let buttons = if panel.confirm.is_some() {
        vec![
            Button::new("确认停止 y", K::Char('y'), true),
            Button::new("取消 Esc", K::Esc, true),
        ]
    } else {
        vec![
            Button::new("接入 Enter", K::Enter, panel.selected.is_some()),
            Button::new(
                if panel.show_reply {
                    "收起 r"
                } else {
                    "回复 r"
                },
                K::Char('r'),
                panel.selected.is_some(),
            ),
            Button::new(
                if panel.by_state {
                    "项目排序 s"
                } else {
                    "状态排序 s"
                },
                K::Char('s'),
                true,
            ),
            Button::new("停止 x", K::Char('x'), panel.selected.is_some()),
        ]
    };
    let (content, buttons) = buttons::draw(
        frame,
        Rect {
            height: inside.height.saturating_sub(1),
            ..inside
        },
        &buttons,
    );
    let available = content.height;
    let list_height = if panel.show_reply && available >= 5 {
        (available / 2).max(3)
    } else {
        available
    };
    let list = Rect::new(
        inside.x,
        inside.y,
        inside.width.saturating_sub(1),
        list_height,
    );
    let mut hits = Hits {
        buttons,
        list,
        ..Default::default()
    };
    let rows = agent_rows(panel, view.showing, usize::from(list.width), view.now);
    let selected_rows: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.name.as_ref() == panel.selected.as_ref() && r.name.is_some())
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
            Paragraph::new(row.line.clone()),
            Rect::new(list.x, y, list.width, 1),
        );
        if let Some(name) = &row.name {
            hits.agents.push((y, name.clone()));
        }
    }
    if rows.is_empty() {
        frame.render_widget(Paragraph::new("(no agents)"), list);
    }
    if rows.len() > usize::from(list.height) && list.height > 0 {
        let mut scrollbar =
            ScrollbarState::new(rows.len().saturating_sub(usize::from(list.height)))
                .position(panel.top);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            Rect::new(inside.x, inside.y, inside.width, list.height),
            &mut scrollbar,
        );
        let above = rows[..panel.top].iter().filter(|r| r.headline).count();
        let below = rows
            .iter()
            .skip(panel.top + usize::from(list.height))
            .filter(|r| r.headline)
            .count();
        frame.render_widget(
            border(
                &format!("Agents · ↑ {above} ↓ {below}"),
                view.focus == Focus::Agents,
            ),
            area,
        );
    }
    if list_height < available {
        let y = list.bottom();
        frame.render_widget(
            Paragraph::new(format!(
                "─ {} · last reply",
                panel.selected.as_deref().unwrap_or("")
            ))
            .style(Style::default().fg(Color::Cyan)),
            Rect::new(inside.x, y, inside.width, 1),
        );
        hits.reply = Rect::new(
            inside.x,
            y + 1,
            inside.width.saturating_sub(1),
            available - list_height - 1,
        );
        let lines = reply_lines(view.reply, usize::from(hits.reply.width));
        panel.reply_top = panel
            .reply_top
            .min(lines.len().saturating_sub(usize::from(hits.reply.height)));
        frame.render_widget(
            Paragraph::new(
                lines
                    .iter()
                    .skip(panel.reply_top)
                    .take(usize::from(hits.reply.height))
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            hits.reply,
        );
        if lines.len() > usize::from(hits.reply.height) {
            let mut state =
                ScrollbarState::new(lines.len().saturating_sub(usize::from(hits.reply.height)))
                    .position(panel.reply_top);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                Rect {
                    width: inside.width,
                    ..hits.reply
                },
                &mut state,
            );
        }
    }
    hits
}

fn agent_rows(panel: &Panel, showing: Option<&str>, width: usize, now: f64) -> Vec<Row> {
    let ordered = panel.ordered(now);
    let cells: Vec<_> = ordered.iter().map(|a| cells(a, now)).collect();
    let heads = [
        "NAME", "KIND", "INST", "STATE", "DOING", "QUIET", "ATT", "SOURCE", "DIR", "TITLE",
    ];
    let mut widths: Vec<_> = heads.iter().map(|s| s.len()).collect();
    for row in &cells {
        for (i, s) in row.iter().enumerate() {
            widths[i] = widths[i].max(s.width());
        }
    }
    let table = widths.iter().sum::<usize>() + heads.len() - 1 + 4 <= width;
    let mut rows = Vec::new();
    if table {
        rows.push(Row {
            line: Line::styled(
                format!(
                    "    {}",
                    heads
                        .iter()
                        .enumerate()
                        .map(|(i, s)| pad(s, widths[i]))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                Style::default().fg(Color::DarkGray),
            ),
            name: None,
            headline: false,
        });
    }
    let mut previous_group = None;
    for (a, cells) in ordered.iter().zip(cells) {
        let prefix = group(&a.name);
        if previous_group != Some(prefix) {
            rows.push(Row {
                line: Line::styled(
                    if prefix.is_empty() {
                        "(no prefix)"
                    } else {
                        prefix
                    }
                    .to_owned(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                name: None,
                headline: false,
            });
            previous_group = Some(prefix);
        }
        let selected = panel.selected.as_deref() == Some(&a.name);
        let mut style = Style::default();
        if selected {
            style = style.bg(Color::Indexed(237));
        }
        let state_color = match a.state.as_deref() {
            Some("blocked") => Color::Red,
            Some("working") => Color::Yellow,
            Some("idle") => Color::Green,
            _ => Color::Gray,
        };
        let marker = if a.state.as_deref() == Some("blocked") {
            "!"
        } else if panel.suspect(a, now) {
            "?"
        } else if panel.unread.contains(&a.name) {
            "●"
        } else {
            " "
        };
        let lead = format!(
            "{}{}{marker} ",
            if selected { "▎" } else { " " },
            if showing == Some(&a.name) { "▶" } else { " " }
        );
        let summary = if table {
            cells
                .iter()
                .enumerate()
                .map(|(i, s)| pad(s, widths[i]))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            format!("{}  {}  {}", cells[0], cells[3], cells[5])
        };
        rows.push(Row {
            line: Line::from(vec![
                Span::styled(lead, style.fg(state_color)),
                Span::styled(
                    clip(&summary, width.saturating_sub(4)),
                    style.fg(state_color),
                ),
            ])
            .style(style),
            name: Some(a.name.clone()),
            headline: true,
        });
        if !table {
            let mut items = Vec::new();
            if !cells[4].is_empty() {
                items.push(cells[4].clone());
            }
            items.extend(
                [
                    format!("{} {}", cells[1], cells[2]).trim().to_string(),
                    format!("ATT {}", cells[6]),
                    format!("SOURCE {}", cells[7]),
                    format!("DIR {}", cells[8]),
                    cells[9].clone(),
                ]
                .into_iter()
                .filter(|s| !s.is_empty() && s != "SOURCE " && s != "DIR "),
            );
            for line in fold_items(&items, width.saturating_sub(4)) {
                rows.push(Row {
                    line: Line::styled(format!("    {line}"), style.fg(Color::Gray)),
                    name: Some(a.name.clone()),
                    headline: false,
                });
            }
        }
    }
    rows
}
fn cells(a: &Agent, now: f64) -> Vec<String> {
    let state = if a.starting {
        "(starting)".into()
    } else if a.incompatible {
        format!("(incompatible proto {})", a.proto.unwrap_or(0))
    } else if let Some(error) = &a.error {
        error.clone()
    } else {
        a.state.clone().unwrap_or_default()
    };
    let doing = if matches!(a.state.as_deref(), Some("working" | "blocked")) {
        let spin = if a.state.as_deref() == Some("working") {
            ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"][(now * 5.0) as usize % 10]
        } else {
            ""
        };
        format!(
            "{spin} {} {}",
            a.last_tool.as_deref().unwrap_or("thinking"),
            seconds(a.turn_started.map(|t| now - t))
        )
    } else {
        String::new()
    };
    vec![
        a.name
            .strip_prefix(group(&a.name))
            .unwrap_or(&a.name)
            .into(),
        a.kind.clone().unwrap_or_default(),
        a.instance
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(6)
            .collect(),
        state,
        doing,
        seconds(a.last_output.map(|t| now - t)),
        a.attached.to_string(),
        a.last_input_source.clone().unwrap_or_default(),
        short_dir(a.cwd.as_deref().unwrap_or("")),
        a.title.clone().unwrap_or_default(),
    ]
}
fn seconds(value: Option<f64>) -> String {
    value
        .map(|s| {
            if s >= 3600.0 {
                format!("{:.1}h", s / 3600.0)
            } else {
                format!("{}s", s.max(0.0) as u64)
            }
        })
        .unwrap_or_default()
}
fn short_dir(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = if !home.is_empty() && (path == home || path.starts_with(&format!("{home}/"))) {
        format!("~{}", &path[home.len()..])
    } else {
        path.into()
    };
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() > 3 {
        format!("…/{}", parts[parts.len() - 2..].join("/"))
    } else {
        path
    }
}
fn pad(text: &str, width: usize) -> String {
    format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
}
fn clip(text: &str, width: usize) -> String {
    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
    if clean.width() <= width {
        return clean;
    }
    if width == 0 {
        return String::new();
    }
    let mut used = 0;
    let mut result = String::new();
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
fn fold_items(items: &[String], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for item in items {
        let item = clip(item, width);
        if !current.is_empty() && current.width() + 3 + item.width() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str(" · ");
        }
        current.push_str(&item);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
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
