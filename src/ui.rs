use crate::{
    agents::{Panel, Status, group},
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
    pub buttons: Vec<crate::buttons::Hit>,
    pub terminal: Vec<crate::terminals::Hit>,
    pub agents: Vec<(u16, String)>,
    pub list: Rect,
    pub reply: Rect,
    pub plugins: Rect,
}

pub struct View<'a> {
    pub colors: &'a Theme,
    pub panes: Panes,
    pub focus: Focus,
    pub showing: Option<&'a str>,
    /// Agents this saddle is displaying in any pane, in every tab.
    pub local: &'a [String],
    pub viewer: Option<&'a Session>,
    pub projects: &'a [String],
    pub viewer_note: &'a str,
    pub reply: &'a str,
    pub now: f64,
    pub pointer: &'a crate::buttons::Pointer,
}

pub fn draw(frame: &mut Frame, panel: &mut Panel, view: View<'_>) -> Hits {
    draw_workspace(frame, panel, view, None)
}

pub struct Workspace<'a> {
    pub terminals: &'a crate::terminals::Terminals,
    pub mascot: &'a mut crate::mascot::Mascot,
    pub mascot_enabled: bool,
    pub placement: Option<&'a crate::placement::Placement>,
    pub search: Option<&'a mut crate::search::Search>,
    pub form: Option<&'a mut crate::launch::Form>,
    pub program: &'a str,
    pub modal: bool,
    pub attention: Attention<'a>,
    pub settings: Option<&'a mut crate::settings::Settings>,
}
/// The Attention entry's items and, while open, its popup.
pub struct Attention<'a> {
    pub items: &'a [crate::attention::Item],
    pub loading: bool,
    pub popup: Option<&'a mut crate::attention::Popup>,
}
pub fn draw_workspace(
    frame: &mut Frame,
    panel: &mut Panel,
    view: View<'_>,
    workspace: Option<Workspace<'_>>,
) -> Hits {
    let (
        terminals,
        mascot,
        placement,
        mut search,
        mut form,
        program,
        modal,
        attention,
        mut settings,
    ) = match workspace {
        Some(w) => (
            Some(w.terminals),
            if w.mascot_enabled {
                Some(w.mascot)
            } else {
                w.mascot.hide();
                None
            },
            w.placement,
            w.search,
            w.form,
            w.program,
            w.modal,
            w.attention,
            w.settings,
        ),
        None => (
            None,
            None,
            None,
            None,
            None,
            "corral",
            false,
            Attention {
                items: &[],
                loading: false,
                popup: None,
            },
            None,
        ),
    };
    let t = view.colors;
    let screen_area = frame.area();
    frame.buffer_mut().set_style(screen_area, t.base());
    let header = agents_header(view.panes.agents);
    let show_plugins = terminals.is_some() && inner(view.panes.agents).height >= 6;
    let action_width = SETTINGS.width()
        + if show_plugins {
            "Plugins".width() + 2
        } else {
            0
        };
    let action_row = if usize::from(header.width)
        >= format!("Agents · {}", panel.agents.len()).width() + 2 + action_width
    {
        0
    } else {
        2
    };
    let separate_actions = show_plugins && usize::from(header.width) < action_width;
    let settings_row = action_row + u16::from(separate_actions);
    let header_rows = (settings_row + 1).max(2);
    let right_aligned = |row: u16, width: u16| {
        let width = width.min(header.width);
        Rect::new(
            header.right() - width,
            header.y.saturating_add(row),
            width,
            1,
        )
        .intersection(inner(view.panes.agents))
    };
    let mut hits = draw_agents(frame, panel, &view, header_rows);
    if show_plugins {
        let mut rect = right_aligned(action_row, 7);
        if !separate_actions {
            rect.x -= SETTINGS.width() as u16 + 2;
        }
        let hovered = view.pointer.hover.is_some_and(|point| rect.contains(point));
        frame.render_widget(
            Paragraph::new("Plugins").style(
                Style::default()
                    .fg(if hovered { t.bright } else { t.agents_text })
                    .remove_modifier(Modifier::BOLD),
            ),
            rect,
        );
        hits.plugins = rect;
    }
    let attention_row = Rect {
        y: header.y.saturating_add(1),
        ..header
    }
    .intersection(inner(view.panes.agents));
    let area = crate::attention::entry(t, frame, attention_row, attention.items, attention.loading);
    let settings_area = right_aligned(settings_row, SETTINGS.width() as u16);
    if !settings_area.is_empty() && settings_area.width == SETTINGS.width() as u16 {
        frame.render_widget(
            Paragraph::new(SETTINGS).style(if settings.is_some() {
                Style::default().fg(t.focus).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(t.agents_text)
                    .remove_modifier(Modifier::BOLD)
            }),
            settings_area,
        );
        hits.buttons.push(crate::buttons::Hit {
            area: settings_area,
            danger: false,
            key: crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char(','),
                crossterm::event::KeyModifiers::NONE,
            ),
        });
    }
    if !area.is_empty() {
        hits.buttons.push(crate::buttons::Hit {
            area,
            danger: false,
            key: crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('a'),
                crossterm::event::KeyModifiers::NONE,
            ),
        });
    }
    let mut attention_popup = attention
        .popup
        .map(|popup| (popup, attention.items, attention.loading));
    let title = pane_title(view.showing, &panel.agents);
    if let Some(terminals) = terminals {
        hits.terminal = crate::terminals::draw(
            t,
            frame,
            view.panes.viewer,
            terminals,
            view.focus == Focus::Viewer && form.is_none() && placement.is_none() && !modal,
            &panel.agents,
        );
        if let Some(mascot) = mascot {
            if let (Some(area), Some(name)) = (
                terminals.mascot_area(view.panes.viewer, &hits.terminal),
                terminals.mascot_target(),
            ) {
                let agent = panel.agents.iter().find(|a| a.name == name);
                let status = agent.map_or(Status::Unknown, |a| panel.status(a, view.now));
                mascot.draw(
                    frame,
                    area,
                    (name, agent.and_then(|a| a.instance.as_deref())),
                    status,
                    view.now,
                );
            } else {
                mascot.hide();
            }
        }
    } else {
        draw_terminal(frame, view.panes.viewer, &title, &view);
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
        let (body, buttons) = crate::buttons::draw_compact(
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
    }
    if let Some(form) = form.as_mut() {
        hits.buttons = form.draw(t, frame, program, view.projects);
        hits.agents.clear();
    }
    if panel.confirm.is_some() || form.is_some() {
        hits.terminal.clear();
    }
    if let (Some(terminals), Some(placement)) = (terminals, placement) {
        // Only the popup stays clickable; it replaces the tab and pane controls.
        hits.terminal = crate::placement::draw(
            t,
            frame,
            view.panes.viewer,
            terminals,
            placement,
            &panel.agents,
        );
        hits.buttons.clear();
        hits.agents.clear();
    }
    if let Some(search) = search.as_mut() {
        // Only the popup stays clickable; its rows are kept by the search itself.
        hits.buttons = search.draw(t, frame, &panel.agents, |name| {
            terminals.is_some_and(|t| t.find(name).is_some())
        });
        hits.terminal.clear();
        hits.agents.clear();
    }
    if let Some((popup, items, loading)) = attention_popup.as_mut() {
        // Only the popup stays clickable; its rows are kept by the popup itself.
        hits.buttons = popup.draw(t, frame, items, *loading);
        hits.terminal.clear();
        hits.agents.clear();
    }
    if let Some(settings) = settings.as_mut() {
        // Only the popup stays clickable; its field rows are kept by the settings itself.
        hits.buttons = settings.draw(t, frame);
        hits.terminal.clear();
        hits.agents.clear();
    }
    let controls: Vec<_> = hits
        .buttons
        .iter()
        .cloned()
        .map(|h| (Focus::Agents, h))
        .chain(
            hits.terminal
                .iter()
                .map(|(hit, _)| (Focus::Viewer, hit.clone())),
        )
        .collect();
    view.pointer.paint(t, frame, &controls);
    // Tab bodies and their close targets share one visual frame, but keep separate hits.
    if let Some(point) = view.pointer.hover {
        use crate::terminals::Control;
        for pair in hits.terminal.windows(2) {
            let [
                (body, Control::Tab(id)),
                (close, Control::CloseTab(close_id)),
            ] = pair
            else {
                continue;
            };
            let area = body.area.union(close.area);
            if id == close_id && area.contains(point) {
                // Extend the actual hover/pressed colour already painted by Pointer.
                let color = frame.buffer_mut()[(point.x, point.y)].fg;
                frame
                    .buffer_mut()
                    .set_style(area, Style::default().fg(color));
            }
        }
    }
    let (mut target, mut help) = match view.focus {
        Focus::Agents => (
            "Agents".to_string(),
            " ↑↓ Select  ↵ Attach  / Search  a Attention  n New  z Fold  , Settings  Tab Viewer  q Quit",
        ),
        Focus::Viewer => (
            view.showing
                .or_else(|| {
                    terminals
                        .filter(|t| t.active_pane().viewer.shell.is_some())
                        .map(|_| "Terminal")
                })
                .unwrap_or("Viewer · disconnected")
                .to_string(),
            " Keys go to terminal  Ctrl-] Agents",
        ),
    };
    if let Some(form) = &form {
        target = format!("New agent · {}", form.label());
        help = " Tab/Shift-Tab Field  ←→ Home/End Move  Backspace/Delete Erase  Ctrl-U Clear";
    } else if let Some(placement) = placement {
        (target, help) = if placement.place.is_some() {
            (
                "Open agent".into(),
                " ↑↓ Select  Enter Open  Esc Cancel  Ctrl-] Agents",
            )
        } else {
            (
                "Split pane".into(),
                " ←→↑↓ Choose side  Esc Cancel  Ctrl-] Agents",
            )
        };
    }
    if search.is_some() {
        target = "Search agents".into();
        help = " Type to filter  ↑↓ Select  Enter Open  Esc Cancel";
    }
    if attention_popup.is_some() {
        target = "Attention".into();
        help = " ↑↓ Select  Enter Open  Esc Cancel";
    }
    if let Some(settings) = &settings {
        target = "Settings".into();
        help = if settings.conflict() {
            " k Keep my edits  d Discard my edits  Esc Back"
        } else {
            " Tab/↑↓ Field  F1-F3 Page  Ctrl-D Default  Ctrl-U Clear  Ctrl-S Save  Esc Cancel"
        };
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
                if panel.confirm.is_some()
                    || form.is_some()
                    || placement.is_some()
                    || search.is_some()
                    || attention_popup.is_some()
                    || settings.is_some()
                    || panel.message.is_empty()
                {
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
/// A terminal pane's title: the shown agent's public role label, or Viewer when empty.
pub fn pane_title(showing: Option<&str>, agents: &[Agent]) -> String {
    let Some(name) = showing else {
        return "Viewer".into();
    };
    let role = agents.iter().find(|a| a.name == name).and_then(Agent::role);
    let role = match role {
        Some(crate::corral::Role::Controller) => "Controller",
        Some(crate::corral::Role::Regular) => "Regular",
        Some(crate::corral::Role::Implementer) => "Implementer",
        Some(crate::corral::Role::Reviewer) => "Reviewer",
        None => "Agent",
    };
    format!("{role} · {name}")
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
/// The Agents header row, inside the border and its one-column side padding.
fn agents_header(area: Rect) -> Rect {
    let inside = inner(area);
    Rect::new(
        inside.x + 1.min(inside.width),
        inside.y,
        inside.width.saturating_sub(2),
        inside.height.min(1),
    )
}
const SETTINGS: &str = "Settings";
/// One bottom-bar control: key and label, whether it acts, and whether it is destructive.
/// `lit` shows a state in normal text without making the control clickable.
struct Control {
    key: &'static str,
    label: &'static str,
    code: crossterm::event::KeyCode,
    enabled: bool,
    danger: bool,
    lit: bool,
}
impl Control {
    fn width(&self) -> u16 {
        (self.key.width() + usize::from(!self.key.is_empty()) + self.label.width()) as u16
    }
}
/// Places controls left to right, with two-column gaps, then one, then wrapping onto rows.
fn bar_rows(controls: &[Control], width: u16) -> Vec<Vec<(u16, usize)>> {
    for gap in [2, 1] {
        let total: u16 = controls.iter().map(Control::width).sum::<u16>()
            + gap * controls.len().saturating_sub(1) as u16;
        if total <= width {
            let mut x = 0;
            return vec![
                controls
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let at = x;
                        x += c.width() + gap;
                        (at, i)
                    })
                    .collect(),
            ];
        }
    }
    let mut rows = vec![Vec::new()];
    let mut x = 0;
    for (i, c) in controls.iter().enumerate() {
        if x > 0 && x + c.width() > width {
            rows.push(Vec::new());
            x = 0;
        }
        rows.last_mut().unwrap().push((x, i));
        x += c.width() + 1;
    }
    rows
}
fn draw_agents(frame: &mut Frame, panel: &mut Panel, view: &View<'_>, header_rows: u16) -> Hits {
    let t = view.colors;
    let area = view.panes.agents;
    if area.is_empty() {
        return Hits::default();
    }
    let focused = view.focus == Focus::Agents;
    frame
        .buffer_mut()
        .set_style(area, Style::default().fg(t.agents_text).bg(t.agents_bg));
    let block = t
        .block("", focused)
        .border_style(Style::default().fg(if focused { t.focus } else { t.agents_border }));
    frame.render_widget(block.clone(), area);
    let inside = inner(area);
    let content = agents_header(area);
    let content = Rect {
        height: inside.height,
        ..content
    };
    if content.is_empty() {
        return Hits::default();
    }
    let rule = |frame: &mut Frame, y: u16| {
        frame.render_widget(
            Paragraph::new("─".repeat(usize::from(content.width)))
                .style(Style::default().fg(t.agents_rule)),
            Rect::new(content.x, y, content.width, 1),
        );
    };
    frame.render_widget(
        Paragraph::new(format!("Agents · {}", panel.agents.len())).style(
            Style::default()
                .fg(t.agents_text)
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(content.x, content.y, content.width, 1),
    );
    // The list starts after the shared title/actions, status and any wrapped actions.
    if content.height > header_rows {
        rule(frame, content.y + header_rows);
    }
    use crossterm::event::KeyCode as K;
    let selected = panel.selected.is_some();
    let connected = selected && panel.selected.as_deref() == view.showing;
    let folded = panel.folded();
    let controls = [
        if connected {
            Control {
                key: "",
                label: "[Attached]",
                code: K::Enter,
                enabled: false,
                danger: false,
                lit: true,
            }
        } else {
            Control {
                key: "↵",
                label: "Attach",
                code: K::Enter,
                enabled: selected,
                danger: false,
                lit: false,
            }
        },
        Control {
            key: "/",
            label: "Search",
            code: K::Char('/'),
            enabled: true,
            danger: false,
            lit: false,
        },
        Control {
            key: "n",
            label: "New",
            code: K::Char('n'),
            enabled: true,
            danger: false,
            lit: false,
        },
        Control {
            key: "s",
            label: if panel.by_name { "Name" } else { "Sort" },
            code: K::Char('s'),
            enabled: true,
            danger: false,
            lit: false,
        },
        Control {
            key: if panel.stopping { "" } else { "x" },
            label: if panel.stopping { "Stopping" } else { "Stop" },
            code: K::Char('x'),
            enabled: selected && !panel.stopping,
            danger: true,
            lit: false,
        },
        Control {
            key: "z",
            label: if folded { "Expand" } else { "Fold" },
            code: K::Char('z'),
            enabled: true,
            danger: false,
            lit: false,
        },
    ];
    let bar = bar_rows(&controls, content.width);
    let bar_height = bar.len() as u16;
    let mut hits = Hits::default();
    // Header, entries, rule, at least one list row, rule and the bar; with less room the
    // list keeps the space.
    let body_top = content.y + (header_rows + 1).min(content.height);
    let mut body_bottom = content.bottom();
    if content.height > header_rows + 2 + bar_height {
        body_bottom = content.bottom() - bar_height - 1;
        rule(frame, body_bottom);
        for (row, placed) in bar.iter().enumerate() {
            let y = body_bottom + 1 + row as u16;
            for &(x, i) in placed {
                let c = &controls[i];
                let (key, label) = if c.lit {
                    let text = Style::default().fg(t.agents_text);
                    (text, text)
                } else if !c.enabled {
                    let dim = Style::default().fg(t.agents_dimmer);
                    (dim, dim)
                } else if c.danger {
                    let red = Style::default().fg(t.agents_red);
                    (red.add_modifier(Modifier::BOLD), red)
                } else {
                    (
                        Style::default()
                            .fg(t.agents_accent)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(t.agents_text),
                    )
                };
                let mut spans = Vec::new();
                if !c.key.is_empty() {
                    spans.push(Span::styled(c.key, key));
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(c.label, label));
                let rect = Rect::new(content.x + x, y, c.width(), 1).intersection(content);
                frame.render_widget(Paragraph::new(Line::from(spans)), rect);
                if c.enabled {
                    hits.buttons.push(crate::buttons::Hit {
                        area: rect,
                        danger: c.danger,
                        key: crossterm::event::KeyEvent::new(
                            c.code,
                            crossterm::event::KeyModifiers::NONE,
                        ),
                    });
                }
            }
        }
    }
    let body_height = body_bottom.saturating_sub(body_top);
    let detail_height = if panel.show_reply && selected && body_height >= 6 {
        (body_height / 3).max(3)
    } else {
        0
    };
    let list = Rect::new(
        content.x,
        body_top,
        content.width,
        body_height - detail_height,
    );
    hits.list = list;
    let rows = agent_rows(t, panel, view.local, inside.width, view.now);
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
            Paragraph::new("No agents").style(Style::default().fg(t.agents_dim)),
            list,
        );
    }
    if rows.len() > usize::from(list.height) && list.height > 0 {
        // The right padding column carries the scrollbar.
        scrollbar(
            t,
            frame,
            Rect {
                width: list.width + 1,
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
        // Keep the repository visible when its heading has scrolled above the viewport.
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
                Style::default().fg(t.agents_dim),
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
        hits.reply = Rect::new(content.x, list.bottom() + 1, list.width, detail_height - 1);
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
                    width: list.width + 1,
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
fn width_of(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}
/// Cuts spans to `width` display columns, ending with … when anything was dropped.
pub(crate) fn clip_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Span<'static>> {
    if width_of(&spans) <= width {
        return spans;
    }
    let mut result = Vec::new();
    let mut used = 0;
    for span in spans {
        let room = width.saturating_sub(used);
        let text = clip(&span.content, room);
        used += text.width();
        let cut = text != span.content;
        result.push(Span::styled(text, span.style));
        if cut {
            break;
        }
    }
    result
}
/// Rows of every group and agent. `panel_width` is the Agents panel's inner width; the row
/// area is two columns narrower (side padding), and entries indent after their gutter.
fn agent_rows(t: &Theme, panel: &Panel, local: &[String], panel_width: u16, now: f64) -> Vec<Row> {
    let width = usize::from(panel_width.saturating_sub(2));
    // Entry text after the gutter and its padding; expanded lines indent two more.
    let content = width.saturating_sub(2);
    let info = content.saturating_sub(2);
    let brand_width = if panel_width >= 50 { 8 } else { 2 };
    let ordered = panel.ordered(now);
    // The effort column exists only when some agent carries a known delegated effort label.
    let effort_column = ordered.iter().any(|a| a.effort().is_some());
    // Dot, then name, agent, effort (2), state (9) and time (5), one column apart; the name
    // gives way.
    let name_width = content
        .saturating_sub(2 + brand_width + 1 + if effort_column { 3 } else { 0 } + 9 + 1 + 5 + 1);
    let folded = panel.folded();
    let mut rows = Vec::new();
    let mut previous = None;
    for (index, a) in ordered.iter().enumerate() {
        let prefix = group(&a.name);
        if previous != Some(prefix) {
            if previous.is_some() {
                rows.push(Row {
                    line: Line::default(),
                    name: None,
                    headline: false,
                });
            }
            let count = ordered[index..]
                .iter()
                .take_while(|agent| group(&agent.name) == prefix)
                .count();
            let count = format!("({count})");
            let title = clip(
                if prefix.is_empty() { "agents/" } else { prefix },
                width.saturating_sub(count.width() + 3),
            );
            let fill = width.saturating_sub(title.width() + count.width() + 2);
            rows.push(Row {
                line: Line::from(vec![
                    Span::styled(
                        title,
                        Style::default()
                            .fg(t.agents_accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled("─".repeat(fill), Style::default().fg(t.agents_faint)),
                    Span::raw(" "),
                    Span::styled(count, Style::default().fg(t.agents_dim)),
                ]),
                name: None,
                headline: false,
            });
            previous = Some(prefix);
        }
        let selected = panel.selected.as_deref() == Some(&a.name);
        let status = panel.status(a, now);
        let look = look(t, status, now);
        let short = a.name.strip_prefix(prefix).unwrap_or(&a.name);
        let exited = status == Status::Exited;
        let mut lines = Vec::new();
        // R1: status dot, name, agent, state and time in fixed columns.
        let (brand_label, brand_color) = agent_brand(t, a.kind.as_deref().unwrap_or(""));
        let brand = if brand_width == 8 {
            brand_label
        } else {
            brand_label.split(' ').next().unwrap_or("").to_owned()
        };
        let mut state = Vec::new();
        state.push(Span::styled(
            look.label,
            Style::default().fg(look.color).add_modifier(Modifier::BOLD),
        ));
        let state_width = width_of(&state);
        state.push(Span::raw(" ".repeat(9usize.saturating_sub(state_width))));
        let here = local.iter().any(|name| name == &a.name);
        let (mark, mark_color) = if here {
            ("⦿", t.agents_green)
        } else if panel.unread.contains(&a.name) {
            ("•", t.unread)
        } else {
            ("", t.agents_text)
        };
        let time = short_time(a.last_output.map(|v| now - v));
        let time_color = if here {
            t.agents_green
        } else if status == Status::Working {
            t.agents_blue
        } else if exited {
            t.agents_faint
        } else {
            t.agents_text
        };
        let gap = if mark.is_empty() || mark.width() + 1 + time.width() > 5 {
            ""
        } else {
            " "
        };
        let used = mark.width() + gap.width() + time.width();
        let mut first = vec![
            Span::styled(look.dot, Style::default().fg(look.color)),
            Span::raw(" "),
            Span::styled(
                pad(&clip(short, name_width), name_width),
                Style::default()
                    .fg(if exited {
                        t.agents_faint
                    } else {
                        t.agents_text
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                pad(&clip(&brand, brand_width), brand_width),
                Style::default()
                    .fg(brand_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ];
        if effort_column {
            first.extend(effort_bars(t, a.effort()));
            first.push(Span::raw(" "));
        }
        first.extend(state);
        first.push(Span::raw(" ".repeat(1 + 5usize.saturating_sub(used))));
        first.push(Span::styled(mark, Style::default().fg(mark_color)));
        first.push(Span::styled(
            format!("{gap}{time}"),
            Style::default().fg(time_color),
        ));
        lines.push(first);
        if !folded || selected {
            let indent = |spans: Vec<Span<'static>>| {
                let mut line = vec![Span::raw("  ")];
                line.extend(clip_spans(spans, info));
                line
            };
            // R2: the title unless it merely repeats the group.
            if let Some(title) = a
                .title
                .as_deref()
                .filter(|title| !title.trim().is_empty() && *title != prefix.trim_end_matches('/'))
            {
                lines.push(indent(vec![Span::styled(
                    title.to_owned(),
                    Style::default().fg(t.agents_text),
                )]));
            }
            // R3: what the agent is doing or needs, from public fields only.
            for (label, text, duration) in activity(a, status) {
                let tail = duration
                    .then(|| format!(" · {}", short_time(a.turn_started.map(|v| now - v))))
                    .filter(|tail| label.width() + 2 + tail.width() <= info);
                let room = info.saturating_sub(
                    label.width() + 1 + tail.as_deref().map_or(0, UnicodeWidthStr::width),
                );
                let mut spans = vec![Span::styled(
                    format!("{label} {}", clip(&text, room)),
                    Style::default().fg(look.color),
                )];
                if let Some(tail) = tail {
                    spans.push(Span::styled(tail, Style::default().fg(t.agents_dim)));
                }
                lines.push(indent(spans));
            }
            // R4 (and R4b): branch and base on the left, uncommitted changes on the right.
            if let Some(cwd) = &a.cwd {
                let (left, diff) = git_parts(t, panel.git.get(cwd), short, info);
                let (left_width, diff_width) = (width_of(&left), width_of(&diff));
                if diff.is_empty() {
                    lines.push(indent(left));
                } else if left_width + 1 + diff_width <= info {
                    let mut line = left;
                    line.push(Span::raw(" ".repeat(info - left_width - diff_width)));
                    line.extend(diff);
                    lines.push(indent(line));
                } else {
                    lines.push(indent(left));
                    let mut line = vec![Span::raw(" ".repeat(info.saturating_sub(diff_width)))];
                    line.extend(diff);
                    lines.push(indent(line));
                }
            }
            // R5: the directory.
            lines.push(indent(vec![Span::styled(
                agent_path(a.cwd.as_deref().unwrap_or("—"), prefix, info),
                Style::default().fg(t.agents_dim),
            )]));
            // R6: instance, connections and source.
            let metadata_color = if selected {
                t.agents_text
            } else {
                t.agents_dim
            };
            let metadata = Style::default().fg(metadata_color);
            let mut identity = Vec::new();
            // Narrow panels keep only marks; a type without one is named here instead.
            if brand_width < 8 && a.kind.as_deref().is_some_and(|kind| brand == kind) {
                identity.push(Span::styled(
                    format!("{} · ", a.kind.as_deref().unwrap_or("—")),
                    metadata,
                ));
            }
            identity.extend([
                Span::styled(
                    format!(
                        "{} · ",
                        a.instance
                            .as_deref()
                            .unwrap_or("—")
                            .chars()
                            .take(6)
                            .collect::<String>()
                    ),
                    metadata,
                ),
                Span::styled(
                    format!("ATT {}", a.attached),
                    Style::default().fg(if a.attached > 0 {
                        t.agents_green
                    } else {
                        metadata_color
                    }),
                ),
                Span::styled(
                    format!(" · VIA {}", a.last_input_source.as_deref().unwrap_or("—")),
                    metadata,
                ),
            ]);
            lines.push(indent(identity));
        }
        let background = if selected {
            Style::default().bg(t.agent_selected)
        } else {
            Style::default()
        };
        for (i, spans) in lines.into_iter().enumerate() {
            let mut line = vec![
                Span::styled(
                    if selected { "┃" } else { "│" },
                    Style::default()
                        .fg(if selected {
                            t.agents_accent
                        } else {
                            t.agents_faint
                        })
                        .bg(t.agents_bg),
                ),
                Span::raw(" "),
            ];
            line.extend(spans);
            rows.push(Row {
                line: Line::from(line).style(background),
                name: Some(a.name.clone()),
                headline: i == 0,
            });
        }
    }
    rows
}
/// Status dot, state label and its color in the Agents palette.
struct Look {
    dot: &'static str,
    label: &'static str,
    color: Color,
}
fn look(t: &Theme, status: Status, now: f64) -> Look {
    let (dot, label, color) = match status {
        Status::Waiting => ("?", "waiting", t.agents_yellow),
        Status::Error => ("!", "error", t.agents_red),
        Status::Stalled => ("▲", "stalled", t.agent_stalled),
        Status::Working => (
            ["◐", "◓", "◑", "◒"][(now * 1000.0 / 360.0) as usize % 4],
            "working",
            t.agents_blue,
        ),
        Status::Starting => ("◌", "starting", t.agent_starting),
        Status::Unknown => ("·", "unknown", t.agents_dim),
        Status::Idle => ("○", "idle", t.agents_green),
        Status::Exited => ("✕", "exited", t.agents_faint),
    };
    Look { dot, label, color }
}
/// Activity lines: label, text, and whether the turn's duration follows. Waiting has no
/// public summary of the question, so it says so plainly instead of guessing one.
fn activity(a: &Agent, status: Status) -> Vec<(&'static str, String, bool)> {
    let mut lines = Vec::new();
    match status {
        Status::Waiting => lines.push(("ASK", "waiting for input".to_owned(), true)),
        Status::Working | Status::Stalled => lines.push((
            "DOING",
            a.last_tool.as_deref().unwrap_or("thinking").to_owned(),
            true,
        )),
        _ => {}
    }
    if let Some(error) = &a.error {
        lines.push(("ERR", error.clone(), false));
    }
    if a.incompatible {
        lines.push((
            "ERR",
            format!("incompatible protocol {}", a.proto.unwrap_or(0)),
            false,
        ));
    }
    lines
}
// Text approximations of brand marks; no icon font or terminal image protocol required.
fn agent_brand(t: &Theme, kind: &str) -> (String, Color) {
    let (mark, color) = match kind.to_ascii_lowercase().as_str() {
        "claude" => ("✳", t.claude),
        "codex" => (">_", t.codex),
        "pi" => ("π", t.pi),
        "omp" => ("π", t.omp),
        _ => return (kind.to_owned(), t.agents_dim),
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

/// Git state of the agent's directory as (branch and base, changes). A missing entry is still
/// loading, `None` is unavailable, and fields Git could not determine show — rather than zero.
/// `ahead` counts commits HEAD has beyond its base, so it reads ↑. The branch is left out
/// when it repeats the agent's name, and shortened so the base stays within `width`.
fn git_parts(
    t: &Theme,
    git: Option<&Option<Summary>>,
    name: &str,
    width: usize,
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let dim = Style::default().fg(t.agents_dim);
    let Some(git) = git else {
        return (vec![Span::styled("git …", dim)], Vec::new());
    };
    let Some(s) = git else {
        return (vec![Span::styled("git unavailable", dim)], Vec::new());
    };
    let head = match &s.head {
        Head::Branch(branch) if branch == name => String::new(),
        Head::Branch(branch) => branch.clone(),
        Head::Detached => "HEAD detached".into(),
        Head::Unknown => "—".into(),
    };
    let mut base = vec![match &s.ahead {
        Some((count, _)) => Span::styled(
            format!(" ↑{count}"),
            Style::default().fg(if *count > 0 {
                t.agents_yellow
            } else {
                t.agents_faint
            }),
        ),
        None => Span::styled(" ↑—", Style::default().fg(t.agents_faint)),
    }];
    if let Some((_, name)) = &s.ahead {
        base.push(Span::styled(format!(" {name}"), dim));
    }
    let mut left = vec![Span::styled("⎇", dim)];
    if !head.is_empty() {
        let room = width.saturating_sub(2 + width_of(&base)).max(1);
        left.push(Span::styled(
            format!(" {}", clip(&head, room)),
            Style::default().fg(t.agents_branch),
        ));
    }
    left.extend(base);
    let mut diff = Vec::new();
    match &s.changes {
        Some(changes) => {
            diff.push(Span::styled(
                format!("+{}", changes.added),
                Style::default().fg(t.agents_green),
            ));
            diff.push(Span::styled(
                format!(" -{}", changes.deleted),
                Style::default().fg(t.agents_red),
            ));
            if changes.binary > 0 {
                diff.push(Span::styled(format!(" {} binary", changes.binary), dim));
            }
        }
        None => diff.push(Span::styled("+— -—", dim)),
    }
    let untracked = s.untracked.map_or("—".into(), |n| n.to_string());
    diff.push(Span::styled(format!(" ?{untracked}"), dim));
    (left, diff)
}

/// The full directory; when its last level repeats the group, only the parent's last level.
/// Anything too wide loses leading levels, then leading characters, keeping the end.
fn agent_path(path: &str, prefix: &str, width: usize) -> String {
    let mut parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    let trailing = if !prefix.is_empty() && parts.last() == Some(&prefix.trim_end_matches('/')) {
        parts.pop();
        "/"
    } else {
        ""
    };
    let mut start = if trailing.is_empty() {
        0
    } else {
        parts.len().saturating_sub(1)
    };
    let text = |start: usize| {
        let joined = parts[start..].join("/");
        if start > 0 {
            format!("…/{joined}{trailing}")
        } else if path.starts_with('/') {
            format!("/{joined}{trailing}")
        } else {
            format!("{joined}{trailing}")
        }
    };
    if parts.is_empty() {
        return clip(path, width);
    }
    while text(start).width() > width && start + 1 < parts.len() {
        start += 1;
    }
    let result = text(start);
    if result.width() <= width {
        return result;
    }
    // Keep the end of the last level.
    let mut kept = String::new();
    let mut used = 1;
    for c in result.chars().rev() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        kept.insert(0, c);
        used += w;
    }
    format!("…{kept}")
}

/// Ages fit the five-column time slot beside a connection mark: at most four columns, so
/// hours round once past ten, then days and years take over.
fn short_time(value: Option<f64>) -> String {
    match value {
        Some(s) if s >= 1000.0 * 86400.0 => format!("{}y", (s / (365.0 * 86400.0)) as u64),
        Some(s) if s >= 100.0 * 3600.0 => format!("{}d", (s / 86400.0) as u64),
        Some(s) if s >= 36000.0 => format!("{}h", (s / 3600.0) as u64),
        _ => seconds(value),
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

pub(crate) fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

pub(crate) fn wrap_text(text: &str, width: u16) -> Vec<ratatui::text::Line<'static>> {
    use unicode_width::UnicodeWidthChar;
    if width == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for c in clean(text).replace('\t', "    ").chars() {
        if c == '\n' {
            rows.push(ratatui::text::Line::raw(std::mem::take(&mut line)));
            used = 0;
            continue;
        }
        let w = c.width().unwrap_or(0);
        if used + w > usize::from(width) && !line.is_empty() {
            rows.push(ratatui::text::Line::raw(std::mem::take(&mut line)));
            used = 0;
        }
        line.push(c);
        used += w;
    }
    rows.push(ratatui::text::Line::raw(line));
    rows
}
