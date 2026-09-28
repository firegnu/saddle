//! In-memory tabs. Stable pane ids and revisions bind asynchronous work to its initiator.
use crate::{
    pty::Session,
    terminal::Size,
    viewer::{AgentMetadata, Viewer},
};
use anyhow::Result;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    Current,
    Tab,
    Left,
    Right,
    Up,
    Down,
}
impl Place {
    pub const ALL: [Self; 6] = [
        Self::Current,
        Self::Tab,
        Self::Left,
        Self::Right,
        Self::Up,
        Self::Down,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Current => "Current pane",
            Self::Tab => "New tab",
            Self::Left => "Split left",
            Self::Right => "Split right",
            Self::Up => "Split up",
            Self::Down => "Split down",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub pane: u64,
    pub revision: u64,
}
pub struct Pane {
    pub id: u64,
    pub viewer: Viewer,
    revision: u64,
    requested: Option<String>,
    reserved: bool,
    pub starting: bool,
    observed: bool,
    /// Target metadata during public start/status; the displayed session keeps its own.
    pub pending_agent: AgentMetadata,
}
impl Pane {
    pub fn input_session(&self) -> Option<&Session> {
        // A pending target may share a pane with a different, still-live session.
        if self.reserved || (self.viewer.showing.is_none() && self.viewer.shell.is_none()) {
            return None;
        }
        self.viewer.session.as_ref().filter(|s| s.running())
    }
    pub fn ticket(&self) -> Ticket {
        Ticket {
            pane: self.id,
            revision: self.revision,
        }
    }
    pub fn replacing(&self) -> bool {
        self.reserved || self.viewer.state() == "attaching"
    }
    pub fn requested(&self) -> Option<&str> {
        self.requested.as_deref()
    }
    /// The displayed session owns its directory during replacement. A new pane can
    /// inherit from its target while start/status or the PTY connection is pending.
    pub fn source_cwd(&self) -> Option<&str> {
        if self.viewer.showing.is_some() || self.viewer.shell.is_some() {
            self.viewer.metadata.cwd.as_deref()
        } else {
            self.pending_agent
                .cwd
                .as_deref()
                .or(self.viewer.target_metadata().cwd.as_deref())
        }
    }
}
#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum Node {
    Leaf(u64),
    Split {
        vertical: bool,
        first: Box<Node>,
        second: Box<Node>,
    },
}
impl Node {
    fn split(&mut self, target: u64, id: u64, place: Place) {
        match self {
            Self::Leaf(old) if *old == target => {
                let (a, b) = if matches!(place, Place::Left | Place::Up) {
                    (id, *old)
                } else {
                    (*old, id)
                };
                *self = Self::Split {
                    vertical: matches!(place, Place::Up | Place::Down),
                    first: Box::new(Self::Leaf(a)),
                    second: Box::new(Self::Leaf(b)),
                };
            }
            Self::Split { first, second, .. } => {
                first.split(target, id, place);
                second.split(target, id, place);
            }
            _ => {}
        }
    }
    fn remove(self, id: u64) -> Option<Self> {
        match self {
            Self::Leaf(old) => (old != id).then_some(Self::Leaf(old)),
            Self::Split {
                vertical,
                first,
                second,
            } => match (first.remove(id), second.remove(id)) {
                (Some(first), Some(second)) => Some(Self::Split {
                    vertical,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (first, second) => first.or(second),
            },
        }
    }
    fn rects(&self, area: Rect, active: u64, out: &mut Vec<(u64, Rect)>) {
        match self {
            Self::Leaf(id) => out.push((*id, area)),
            Self::Split {
                vertical,
                first,
                second,
            } => {
                // When a split cannot provide useful content, show only the active branch.
                let extent = if *vertical { area.height } else { area.width };
                let minimum = if *vertical { 6 } else { 12 };
                if extent < minimum {
                    if first.contains(active) {
                        first.rects(area, active, out);
                    } else {
                        second.rects(area, active, out);
                    }
                } else {
                    let half = extent / 2;
                    let (a, b) = if *vertical {
                        (
                            Rect {
                                height: half,
                                ..area
                            },
                            Rect::new(area.x, area.y + half, area.width, area.height - half),
                        )
                    } else {
                        (
                            Rect {
                                width: half,
                                ..area
                            },
                            Rect::new(area.x + half, area.y, area.width - half, area.height),
                        )
                    };
                    first.rects(a, active, out);
                    second.rects(b, active, out);
                }
            }
        }
    }
    fn contains(&self, id: u64) -> bool {
        match self {
            Self::Leaf(old) => *old == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }
}
pub struct Tab {
    pub id: u64,
    pub active: u64,
    pub panes: Vec<Pane>,
    tree: Node,
    /// A pane temporarily filling the tab. Display only: the tree and sessions are unchanged.
    zoomed: Option<u64>,
}
impl Tab {
    pub fn layout(&self) -> serde_json::Value {
        serde_json::to_value(&self.tree).unwrap()
    }
    /// The zoomed pane, while it is still the focused one of several.
    pub fn zoomed(&self) -> Option<u64> {
        self.zoomed
            .filter(|id| *id == self.active && self.panes.len() > 1)
    }
    /// Focus moving to another pane ends a zoom, so the input target always stays visible.
    fn activate(&mut self, id: u64) {
        if self.zoomed.is_some_and(|z| z != id) {
            self.zoomed = None;
        }
        self.active = id;
    }
}
pub struct Terminals {
    pub tabs: Vec<Tab>,
    pub active: u64,
    next_id: u64,
    corral: String,
    retiring: Vec<Viewer>,
}
impl Terminals {
    pub fn new(corral: String) -> Self {
        let mut this = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 0,
            corral,
            retiring: Vec::new(),
        };
        this.new_tab();
        this
    }
    fn pane(&mut self) -> Pane {
        self.next_id += 1;
        Pane {
            id: self.next_id,
            viewer: Viewer::new(self.corral.clone()),
            revision: 0,
            requested: None,
            reserved: false,
            starting: false,
            observed: false,
            pending_agent: AgentMetadata::default(),
        }
    }
    pub fn new_tab(&mut self) -> u64 {
        let pane = self.pane();
        let id = pane.id;
        self.tabs.push(Tab {
            id,
            active: id,
            tree: Node::Leaf(id),
            panes: vec![pane],
            zoomed: None,
        });
        self.active = id;
        id
    }
    pub fn tab(&self) -> &Tab {
        self.tabs.iter().find(|t| t.id == self.active).unwrap()
    }
    pub fn active_pane(&self) -> &Pane {
        let t = self.tab();
        t.panes.iter().find(|p| p.id == t.active).unwrap()
    }
    pub fn get(&self, id: u64) -> Option<&Pane> {
        self.tabs.iter().flat_map(|t| &t.panes).find(|p| p.id == id)
    }
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Pane> {
        self.tabs
            .iter_mut()
            .flat_map(|t| &mut t.panes)
            .find(|p| p.id == id)
    }
    pub fn focus(&mut self, id: u64) {
        if let Some(tab) = self
            .tabs
            .iter_mut()
            .find(|t| t.panes.iter().any(|p| p.id == id))
        {
            tab.activate(id);
            self.active = tab.id;
        }
    }
    /// Zooms pane `id` to fill its tab's terminal area, or restores the split if it is zoomed.
    /// A tab with a single pane has nothing to zoom.
    pub fn toggle_zoom(&mut self, id: u64) {
        let Some(tab) = self
            .tabs
            .iter_mut()
            .find(|t| t.panes.iter().any(|p| p.id == id))
        else {
            return;
        };
        tab.zoomed = if tab.zoomed() == Some(id) || tab.panes.len() < 2 {
            None
        } else {
            tab.activate(id);
            Some(id)
        };
    }
    pub fn find(&self, name: &str) -> Option<u64> {
        self.tabs
            .iter()
            .flat_map(|t| &t.panes)
            .find(|p| {
                p.requested.as_deref() == Some(name)
                    || p.viewer.target() == Some(name)
                    || p.viewer.showing.as_deref() == Some(name)
            })
            .map(|p| p.id)
    }
    pub fn activate_existing(&mut self, name: &str) -> Result<bool> {
        let Some(id) = self.claim(name)? else {
            return Ok(false);
        };
        self.focus(id);
        Ok(true)
    }
    /// The pane holding `name`, keeping it there instead of any pending replacement.
    fn claim(&mut self, name: &str) -> Result<Option<u64>> {
        let Some(id) = self.find(name) else {
            return Ok(None);
        };
        let pane = self.get_mut(id).unwrap();
        if (pane.starting && pane.viewer.showing.as_deref() == Some(name))
            || (pane.reserved && pane.requested.as_deref() != Some(name))
            || (pane.requested.is_none() && pane.viewer.target() != Some(name))
            || (pane.viewer.showing.as_deref() == Some(name)
                && pane
                    .viewer
                    .session
                    .as_ref()
                    .is_some_and(|session| session.is_stopping()))
        {
            // Reopening the displayed agent cancels an older replacement, including a start.
            pane.revision += 1;
            pane.requested = None;
            pane.reserved = false;
            pane.starting = false;
            pane.pending_agent = AgentMetadata::default();
            pane.viewer.cancel_pending();
            if pane.viewer.showing.as_deref() == Some(name) {
                pane.viewer.select(name.into())?;
            }
        }
        Ok(Some(id))
    }
    /// Shows `name` in a new tab or beside pane `anchor`. An agent already open elsewhere
    /// keeps its pane, so its session, output and pending request move with it; otherwise the
    /// returned ticket reserves a new pane. `place` is `Tab` or a split direction.
    pub fn place(&mut self, anchor: u64, place: Place, name: &str) -> Result<Option<Ticket>> {
        anyhow::ensure!(self.get(anchor).is_some(), "target pane disappeared");
        anyhow::ensure!(place != Place::Current, "placement requires a tab or split");
        if let Some(id) = self.claim(name)? {
            // A pane cannot split itself; it stays where it is.
            if id != anchor || place == Place::Tab {
                let pane = self.take(id).unwrap();
                self.insert(anchor, place, pane);
            }
            self.focus(id);
            return Ok(None);
        }
        Ok(Some(self.reserve_at(anchor, place, Some(name.into()))))
    }
    /// Detaches a pane from its tab, dropping the tab once it holds no panes.
    fn take(&mut self, id: u64) -> Option<Pane> {
        let index = self
            .tabs
            .iter()
            .position(|t| t.panes.iter().any(|p| p.id == id))?;
        let tab = &mut self.tabs[index];
        let at = tab.panes.iter().position(|p| p.id == id).unwrap();
        let pane = tab.panes.remove(at);
        if tab.panes.is_empty() {
            let old = self.tabs.remove(index).id;
            if self.active == old && !self.tabs.is_empty() {
                self.active = self.tabs[index.min(self.tabs.len() - 1)].id;
            }
        } else {
            tab.tree = std::mem::replace(&mut tab.tree, Node::Leaf(0))
                .remove(id)
                .unwrap();
            if tab.active == id {
                tab.activate(tab.panes[0].id);
            }
        }
        Some(pane)
    }
    fn insert(&mut self, anchor: u64, place: Place, pane: Pane) {
        let id = pane.id;
        if place == Place::Tab {
            self.next_id += 1;
            self.tabs.push(Tab {
                id: self.next_id,
                active: id,
                tree: Node::Leaf(id),
                panes: vec![pane],
                zoomed: None,
            });
        } else {
            let tab = self
                .tabs
                .iter_mut()
                .find(|t| t.panes.iter().any(|p| p.id == anchor))
                .unwrap();
            tab.tree.split(anchor, id, place);
            tab.panes.push(pane);
        }
        self.focus(id);
    }
    pub fn reserve(&mut self, place: Place, name: Option<String>) -> Ticket {
        self.reserve_at(self.tab().active, place, name)
    }
    pub(crate) fn reserve_at(&mut self, anchor: u64, place: Place, name: Option<String>) -> Ticket {
        let id = match place {
            Place::Current => anchor,
            Place::Tab => self.new_tab(),
            _ => {
                let pane = self.pane();
                let id = pane.id;
                self.insert(anchor, place, pane);
                id
            }
        };
        let pane = self.get_mut(id).unwrap();
        pane.revision += 1;
        pane.requested = name;
        pane.reserved = true;
        pane.starting = false;
        pane.pending_agent = AgentMetadata::default();
        pane.viewer.cancel_pending();
        Ticket {
            pane: id,
            revision: pane.revision,
        }
    }
    pub fn expect(&mut self, ticket: Ticket, name: String) {
        if self.valid(ticket) {
            let pane = self.get_mut(ticket.pane).unwrap();
            pane.requested = Some(name);
            pane.starting = false;
        }
    }
    pub fn valid(&self, ticket: Ticket) -> bool {
        self.get(ticket.pane)
            .is_some_and(|p| p.revision == ticket.revision)
    }
    pub fn complete(&mut self, ticket: Ticket, name: Option<String>) -> Result<bool> {
        if !self.valid(ticket) {
            return Ok(false);
        }
        let pane = self.get_mut(ticket.pane).unwrap();
        pane.requested = None;
        pane.reserved = false;
        pane.starting = false;
        let metadata = std::mem::take(&mut pane.pending_agent);
        if let Some(name) = name {
            pane.observed = false;
            pane.viewer.select_agent(name, metadata)?;
        }
        Ok(true)
    }
    pub fn focus_snapshot(&self) -> Vec<(u64, u64)> {
        let mut values: Vec<_> = self.tabs.iter().map(|t| (t.id, t.active)).collect();
        values.push((self.active, self.active_pane().id));
        values
    }
    pub fn restore_focus(&mut self, snapshot: &[(u64, u64)]) {
        for (id, pane) in snapshot {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == *id)
                && tab.panes.iter().any(|p| p.id == *pane)
            {
                tab.activate(*pane);
            }
        }
        if let Some((tab, pane)) = snapshot.last() {
            if self.get(*pane).is_some() {
                self.focus(*pane);
            } else if self.tabs.iter().any(|t| t.id == *tab) {
                self.active = *tab;
            }
        }
    }
    pub fn close_pane(&mut self, id: u64) -> Result<()> {
        let Some(mut pane) = self.take(id) else {
            return Ok(());
        };
        pane.viewer.close()?;
        self.retiring.push(pane.viewer);
        if self.tabs.is_empty() {
            self.new_tab();
        }
        Ok(())
    }
    pub fn close_tab(&mut self, id: u64) -> Result<()> {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return Ok(());
        };
        for mut pane in self.tabs.remove(index).panes {
            pane.viewer.close()?;
            self.retiring.push(pane.viewer);
        }
        if self.tabs.is_empty() {
            self.new_tab();
        } else if self.active == id {
            self.active = self.tabs[index.min(self.tabs.len() - 1)].id;
        }
        Ok(())
    }
    pub fn rects(&self, area: Rect) -> Vec<(u64, Rect)> {
        let mut result = Vec::new();
        // The tab strip; everything below belongs to the split tree.
        let area = Rect::new(
            area.x,
            area.y + area.height.min(STRIP),
            area.width,
            area.height.saturating_sub(STRIP),
        );
        match self.tab().zoomed() {
            Some(id) => result.push((id, area)),
            None => self.tab().tree.rects(area, self.tab().active, &mut result),
        }
        result
    }
    pub fn disappeared(&mut self, names: &[&str]) -> Result<()> {
        for pane in self.tabs.iter_mut().flat_map(|t| &mut t.panes) {
            if pane.viewer.target().is_some_and(|n| names.contains(&n)) {
                pane.observed = true;
            }
            if pane.observed {
                pane.viewer.disappeared(names)?;
            }
        }
        Ok(())
    }
    pub fn tick(&mut self, area: Rect) -> Result<()> {
        let rects = self.rects(area);
        for pane in self.tabs.iter_mut().flat_map(|t| &mut t.panes) {
            let size = rects.iter().find(|(id, _)| *id == pane.id).map(|(_, a)| {
                let a = crate::ui::inner(*a);
                Size {
                    rows: a.height.max(1),
                    cols: a.width.max(2),
                }
            });
            pane.viewer.tick_visible(size)?;
        }
        for viewer in &mut self.retiring {
            viewer.tick_visible(None)?;
        }
        self.retiring.retain(|v| !v.closed());
        Ok(())
    }
}

/// Rows taken by the outlined tab strip above the panes.
pub const STRIP: u16 = 3;
#[derive(Clone, Copy)]
pub enum Control {
    NewTab,
    Tab(u64),
    CloseTab(u64),
    ClosePane(u64),
    /// Zoom a split pane over its tab's terminal area, or restore it.
    Zoom(u64),
    /// Look back through a pane's output; Copy and Live act on that history view.
    History(u64),
    Copy(u64),
    Live(u64),
    Pane(u64),
    Previous,
    Next,
    /// Placement popup: open the side menu for a pane, pick a side, a candidate, or cancel.
    Split(u64),
    Side(Place),
    Pick(usize),
    Cancel,
}
pub type Hit = (crate::buttons::Hit, Control);
pub fn draw(
    t: &crate::theme::Theme,
    frame: &mut ratatui::Frame,
    area: Rect,
    terminals: &Terminals,
    focused: bool,
    agents: &[crate::corral::Agent],
) -> Vec<Hit> {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Paragraph},
    };
    let mut hits = Vec::new();
    let target = |area: Rect, control: Control| {
        (
            crate::buttons::Hit {
                area,
                danger: false,
                key: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
            },
            control,
        )
    };
    // Keep the rounded outlines, with no extra horizontal padding.
    if area.height >= STRIP {
        let mut x = area.x;
        let outline = |frame: &mut ratatui::Frame, x: u16, spans: Vec<Span<'static>>, border| {
            let width = spans.iter().map(Span::width).sum::<usize>() as u16 + 2;
            if x + width > area.right() {
                return None;
            }
            let rect = Rect::new(x, area.y, width, STRIP);
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border));
            frame.render_widget(Paragraph::new(Line::from(spans)).block(block), rect);
            Some(rect)
        };
        let label = Span::styled(" + ", Style::default().fg(t.text));
        if let Some(rect) = outline(frame, x, vec![label], t.border) {
            hits.push(target(rect, Control::NewTab));
            x = rect.right() + 1;
        }
        for (label, control) in [("‹ ", Control::Previous), ("› ", Control::Next)] {
            if x + 2 > area.right() {
                break;
            }
            let rect = Rect::new(x, area.y + 1, 2, 1);
            frame.render_widget(
                Paragraph::new(label).style(Style::default().fg(t.muted)),
                rect,
            );
            hits.push(target(rect, control));
            x += 2;
        }
        let available = area.right().saturating_sub(x) as usize;
        let labels: Vec<_> = terminals
            .tabs
            .iter()
            .map(|tab| {
                let pane = tab.panes.iter().find(|p| p.id == tab.active).unwrap();
                let name = pane
                    .requested
                    .as_deref()
                    .or(pane.viewer.target())
                    .unwrap_or(if pane.viewer.shell.is_some() {
                        "Terminal"
                    } else {
                        "Empty"
                    });
                crate::ui::clip(name, available.saturating_sub(4).min(20))
            })
            .collect();
        let active = terminals
            .tabs
            .iter()
            .position(|t| t.id == terminals.active)
            .unwrap();
        // Account for actual label widths so a long earlier name cannot hide the current tab.
        let mut start = active;
        let mut used = unicode_width::UnicodeWidthStr::width(labels[active].as_str()) + 4;
        while start > 0 {
            let previous = unicode_width::UnicodeWidthStr::width(labels[start - 1].as_str()) + 5;
            if used + previous > available {
                break;
            }
            used += previous;
            start -= 1;
        }
        for (tab, label) in terminals.tabs.iter().zip(labels).skip(start) {
            if label.is_empty() {
                break;
            }
            let current = tab.id == terminals.active;
            let name = Style::default().fg(t.text);
            let spans = vec![
                Span::styled(
                    label,
                    if current {
                        name.fg(t.bright).add_modifier(Modifier::BOLD)
                    } else {
                        name
                    },
                ),
                Span::styled(" ×", Style::default().fg(t.muted)),
            ];
            let border = if current { t.focus } else { t.border };
            let Some(rect) = outline(frame, x, spans, border) else {
                break;
            };
            // The close symbol and right boundary close; the label switches tabs.
            let close = Rect::new(rect.right() - 2, rect.y, 2, rect.height);
            hits.push(target(
                Rect {
                    width: rect.width - 2,
                    ..rect
                },
                Control::Tab(tab.id),
            ));
            hits.push(target(close, Control::CloseTab(tab.id)));
            x = rect.right() + 1;
        }
    }
    let mut button = |frame: &mut ratatui::Frame,
                      rect: Rect,
                      label: &str,
                      control: Control,
                      active: bool| {
        if rect.is_empty() {
            return;
        }
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(if active { t.focus } else { t.text })),
            rect,
        );
        hits.push(target(rect, control));
    };
    for (id, rect) in terminals.rects(area) {
        if rect.is_empty() {
            continue;
        }
        let pane = terminals.get(id).unwrap();
        let active = id == terminals.tab().active;
        let title = if let Some(shell) = &pane.viewer.shell {
            format!(
                " Terminal · {} · {} ",
                if shell.state == "exited" {
                    format!("exited {}", shell.exit_code.unwrap_or(0))
                } else {
                    shell.state.into()
                },
                shell.cwd
            )
        } else {
            crate::ui::pane_title(pane.viewer.showing.as_deref(), agents)
        };
        frame.render_widget(t.block(title.clone(), focused && active), rect);
        let inside = crate::ui::inner(rect);
        if let Some(session) = &pane.viewer.session {
            let cursor = session
                .screen
                .lock()
                .unwrap()
                .render(inside, frame.buffer_mut());
            if focused
                && active
                && pane.input_session().is_some()
                && let Some(cursor) = cursor
            {
                frame.set_cursor_position(cursor);
            }
        } else {
            frame.render_widget(
                Paragraph::new(
                    pane.requested
                        .as_ref()
                        .map(|n| {
                            format!(
                                "{} {n}…",
                                if pane.starting {
                                    "Starting"
                                } else {
                                    "Attaching"
                                }
                            )
                        })
                        .unwrap_or_else(|| pane.viewer.note.clone()),
                )
                .wrap(Default::default()),
                inside,
            );
        }
        let title_width = (unicode_width::UnicodeWidthStr::width(title.as_str()) as u16)
            .min(rect.width.saturating_sub(2));
        // Clicking a title focuses a pane without sending the gesture to its terminal.
        button(
            frame,
            Rect::new(rect.x + rect.width.min(1), rect.y, title_width, 1),
            &title,
            Control::Pane(id),
            active,
        );
        // The widest set that fits the bottom border, laid out from its right corner.
        let split = (" Split ▾ ", Control::Split(id));
        let close = (" Close pane ", Control::ClosePane(id));
        let cross = ("×", Control::ClosePane(id));
        // Zoom sits beside Split only when the tab has several panes.
        let zoom = (terminals.tab().panes.len() > 1).then(|| {
            (
                if terminals.tab().zoomed() == Some(id) {
                    " Restore "
                } else {
                    " Zoom "
                },
                Control::Zoom(id),
            )
        });
        // A pane with output offers its history; inside it, Copy and the way back replace it.
        let screen = pane
            .viewer
            .session
            .as_ref()
            .map(|s| s.screen.lock().unwrap());
        let history = screen.as_ref().and_then(|s| s.history());
        let entry = match (&screen, history) {
            (None, _) => vec![],
            (Some(_), None) => vec![Some((" History ", Control::History(id)))],
            (Some(_), Some(_)) => vec![
                Some((" Copy ", Control::Copy(id))),
                Some((" Live Esc ", Control::Live(id))),
            ],
        };
        let base = [
            vec![
                Some(split),
                zoom,
                Some(close),
                Some((" Close tab  ", Control::CloseTab(terminals.active))),
            ],
            vec![Some(split), zoom, Some(close)],
            vec![Some(split), zoom, Some(cross)],
            vec![zoom, Some(cross)],
            vec![Some(cross)],
        ];
        // Narrow panes shorten the other controls first and keep the history ones, with room
        // for the history state; only the smallest panes fall back to the controls alone.
        let reserve = if history.is_some() { 10 } else { 0 };
        let sets: Vec<(Vec<_>, u16)> = base
            .iter()
            .map(|set| {
                (
                    entry.iter().chain(set).cloned().collect::<Vec<_>>(),
                    reserve,
                )
            })
            .chain(base.iter().map(|set| (set.clone(), 0)))
            .map(|(set, reserve)| (set.into_iter().flatten().collect(), reserve))
            .collect();
        let width = |set: &Vec<(&str, Control)>| {
            set.iter()
                .map(|(label, _)| unicode_width::UnicodeWidthStr::width(*label) as u16)
                .sum::<u16>()
                + set.len() as u16
                + 1
        };
        if active
            && rect.height >= 2
            && let Some((set, _)) = sets
                .iter()
                .find(|(set, reserve)| width(set) + reserve <= rect.width)
        {
            let mut end = rect.right() - 1;
            if let Some(history) = history {
                // The history state on the left of the bottom border.
                let label = match &history.input {
                    Some(input) => format!(" /{input}▏"),
                    None if history.status.is_empty() => " History ".into(),
                    None => format!(" History · {} ", history.status),
                };
                let room = rect.width.saturating_sub(width(set) + 1);
                let label = crate::ui::clip(&label, usize::from(room));
                frame.render_widget(
                    Paragraph::new(label).style(Style::default().fg(t.focus)),
                    Rect::new(rect.x + 1, rect.bottom() - 1, room, 1),
                );
            }
            for (label, control) in set.iter().rev() {
                let w = unicode_width::UnicodeWidthStr::width(*label) as u16;
                button(
                    frame,
                    Rect::new(end - w, rect.bottom() - 1, w, 1),
                    label,
                    *control,
                    false,
                );
                end -= w + 1;
            }
        }
    }
    hits
}
impl Drop for Terminals {
    fn drop(&mut self) {
        for pane in self.tabs.iter_mut().flat_map(|t| &mut t.panes) {
            let _ = pane.viewer.close();
        }
        for viewer in &mut self.retiring {
            let _ = viewer.close();
        }
    }
}
