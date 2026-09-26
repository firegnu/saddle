//! Terminal placement: pick where a terminal goes (a side of a pane, or a new tab), then the
//! agent to show there. Nothing changes in the layout until an agent is picked.
use crate::{
    buttons::{self, Button},
    corral::Agent,
    terminals::{Control, Hit, Place, Terminals},
    theme::Theme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub struct Placement {
    /// The pane that started it; splits are relative to this pane.
    pub pane: u64,
    /// `None` while the side menu is open.
    pub place: Option<Place>,
    pub selected: usize,
    /// The candidate a pick is bound to when it starts (mouse press or Enter). A refresh can
    /// put another agent on the same row before the release, which must not open it.
    pub pressed: Option<String>,
}

const SIDES: [(&str, KeyCode, Place); 4] = [
    ("Left ←", KeyCode::Left, Place::Left),
    ("Right →", KeyCode::Right, Place::Right),
    ("Above ↑", KeyCode::Up, Place::Up),
    ("Below ↓", KeyCode::Down, Place::Down),
];
const ROWS: usize = 12;

/// The side chosen by an arrow key in the side menu.
pub fn side(code: KeyCode) -> Option<Place> {
    SIDES.iter().find(|(_, key, _)| *key == code).map(|s| s.2)
}

/// Candidate agents by name, each flagged when it is already open and would move.
pub fn candidates(
    agents: &[Agent],
    terminals: &Terminals,
    placement: &Placement,
) -> Vec<(String, bool)> {
    let mut list: Vec<_> = agents
        .iter()
        .filter_map(|a| {
            let open = terminals.find(&a.name);
            // A pane's own agent cannot be split beside itself.
            (placement.place == Some(Place::Tab) || open != Some(placement.pane))
                .then(|| (a.name.clone(), open.is_some()))
        })
        .collect();
    list.sort();
    list
}

pub fn draw(
    t: &Theme,
    frame: &mut Frame,
    viewer: Rect,
    terminals: &Terminals,
    placement: &Placement,
    agents: &[Agent],
) -> Vec<Hit> {
    let candidates = candidates(agents, terminals, placement);
    let (title, width, height) = match placement.place {
        None => {
            let name = terminals
                .get(placement.pane)
                .and_then(|p| p.viewer.showing.as_deref());
            (
                name.map_or(" Split pane ".into(), |n| format!(" Split {n} ")),
                26,
                9,
            )
        }
        Some(place) => {
            let longest = candidates.iter().map(|(n, _)| n.width()).max().unwrap_or(0);
            (
                match place {
                    Place::Tab => " Open agent in a new tab ",
                    Place::Left => " Open agent on the left ",
                    Place::Right => " Open agent on the right ",
                    Place::Up => " Open agent above ",
                    Place::Down => " Open agent below ",
                    Place::Current => " Open agent here ",
                }
                .into(),
                (longest as u16 + 16).clamp(32, 60),
                candidates.len().clamp(1, ROWS) as u16 + 4,
            )
        }
    };
    let screen = frame.area();
    let (width, height) = (width.min(screen.width), height.min(screen.height));
    // A new tab opens below the tab strip; a split opens over the pane's lower right corner,
    // keeping its Split control in view.
    let (x, y) = match terminals
        .rects(viewer)
        .into_iter()
        .find(|(id, _)| placement.place != Some(Place::Tab) && *id == placement.pane)
    {
        Some((_, pane)) => (
            pane.right().saturating_sub(width + 1),
            pane.bottom().saturating_sub(height + 1),
        ),
        None => (
            viewer.x,
            viewer.y + viewer.height.min(crate::terminals::STRIP),
        ),
    };
    let area = Rect::new(
        x.clamp(screen.x, screen.right() - width),
        y.clamp(screen.y, screen.bottom() - height),
        width,
        height,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(t.block(title, true).style(t.base().bg(t.overlay)), area);
    let (body, cancel) = buttons::draw_compact(
        t,
        frame,
        crate::ui::inner(area),
        &[Button::new("Cancel Esc", KeyCode::Esc, true)],
    );
    let mut hits: Vec<Hit> = cancel.into_iter().map(|h| (h, Control::Cancel)).collect();
    if placement.place.is_none() {
        let buttons: Vec<_> = SIDES
            .iter()
            .map(|(label, key, _)| Button::new(label, *key, true))
            .collect();
        let (_, sides) = buttons::draw_outlined_top(t, frame, body, &buttons);
        hits.extend(sides.into_iter().filter_map(|h| {
            let place = side(h.key.code)?;
            Some((h, Control::Side(place)))
        }));
        return hits;
    }
    // Keep one blank row above Cancel.
    let list = Rect {
        height: body.height.saturating_sub(1),
        ..body
    };
    if candidates.is_empty() {
        frame.render_widget(
            Paragraph::new("No agents to open here.").style(Style::default().fg(t.muted)),
            list,
        );
        return hits;
    }
    let selected = placement.selected.min(candidates.len() - 1);
    let rows = usize::from(list.height);
    let top = selected.saturating_sub(rows.saturating_sub(1));
    for (offset, (index, (name, open))) in candidates
        .iter()
        .enumerate()
        .skip(top)
        .take(rows)
        .enumerate()
    {
        let row = Rect::new(list.x, list.y + offset as u16, list.width, 1);
        let tag = if *open { "Move here " } else { "" };
        let room = usize::from(row.width).saturating_sub(tag.width() + 2);
        let name = crate::ui::clip(name, room);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(crate::ui::pad(&format!(" {name}"), room + 2)),
                Span::styled(tag, Style::default().fg(t.connected)),
            ]))
            .style(if index == selected {
                Style::default().bg(t.selected)
            } else {
                Style::default()
            }),
            row,
        );
        hits.push((
            buttons::Hit {
                area: row,
                danger: false,
                key: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            },
            Control::Pick(index),
        ));
    }
    hits
}
