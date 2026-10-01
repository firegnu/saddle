//! Immediate plugin operations, independent of the Settings draft.
use super::{
    Manager,
    core::State,
    registry::{Entry, Manifest},
};
use crate::{
    buttons::{self, Button},
    launch::edit::Input,
    theme::Theme,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;
#[derive(Default)]
pub struct Page {
    selected: usize,
    focus: usize,
    pub message: String,
    adding: Option<Adding>,
    hits: Vec<(Rect, usize)>,
    rows: Vec<(Rect, usize)>,
    tabs: Vec<(crate::input::Focus, buttons::Hit)>,
    pointer: buttons::Pointer,
}
struct Adding {
    input: Input,
    preview: Option<Manifest>,
    focus: usize,
    field: Rect,
}
pub enum Outcome {
    Stay,
    Back,
    Open(String),
    Page(crossterm::event::KeyEvent),
}
enum Row<'a> {
    Core(&'static saddle_core_plugin::Manifest),
    External(&'a Entry),
}
impl Row<'_> {
    fn id(&self) -> &str {
        match self {
            Row::Core(c) => c.id,
            Row::External(e) => &e.id,
        }
    }
}
/// Built-in rows first, then external registrations.
fn rows(m: &Manager) -> Vec<Row<'_>> {
    m.core_catalog()
        .iter()
        .map(|p| Row::Core(p.manifest()))
        .chain(m.registry.entries.iter().map(Row::External))
        .collect()
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Open,
    Toggle,
    Restart,
    Add,
    Remove,
    Sync,
    RemoveResources,
    Refresh,
    Back,
}
impl Page {
    pub fn select_plugin(&mut self, id: &str, m: &Manager) {
        let rows = rows(m);
        // An external registration holding a built-in ID is what the palette listed.
        self.selected = rows
            .iter()
            .position(|r| matches!(r, Row::External(e) if e.id == id))
            .or_else(|| rows.iter().position(|r| r.id() == id))
            .unwrap_or(0);
    }
    fn actions(&self, m: &Manager) -> Vec<(Action, &'static str, bool)> {
        let rows = rows(m);
        if let Some(Row::Core(c)) = rows.get(self.selected) {
            let state = m.core_state(c.id);
            let enabled = state == Some(State::Enabled);
            let recorded = m
                .core_resources(c.id)
                .and_then(|s| s.as_ref().ok())
                .is_some_and(|s| {
                    s.resources
                        .iter()
                        .flat_map(|r| &r.targets)
                        .any(|t| t.recorded())
                });
            return vec![
                (
                    Action::Toggle,
                    if enabled { "Disable" } else { "Enable" },
                    state != Some(State::Conflict),
                ),
                (
                    Action::Sync,
                    "Sync resources",
                    enabled && !c.resources.is_empty(),
                ),
                (
                    Action::RemoveResources,
                    "Remove resources",
                    state == Some(State::Disabled) && recorded,
                ),
                (Action::Add, "Add local…", m.registry.error.is_none()),
                (Action::Refresh, "Refresh", true),
                (Action::Back, "Back", true),
            ];
        }
        let id = rows.get(self.selected).map(Row::id);
        let state = id.map(|id| m.state(id)).unwrap_or_default();
        let enabled = id.is_some_and(|id| m.enabled_here(id));
        let stopped = id.is_none_or(|id| m.stopped(id));
        vec![
            (Action::Open, "Open panel", state == "Running"),
            (
                Action::Toggle,
                if enabled { "Disable" } else { "Enable" },
                id.is_some() && state != "Stopping" && (enabled || stopped),
            ),
            (
                Action::Restart,
                "Restart",
                enabled && matches!(state.as_str(), "Running" | "Unresponsive" | "Failed"),
            ),
            (Action::Add, "Add local…", m.registry.error.is_none()),
            (
                Action::Remove,
                "Remove",
                id.is_some() && !enabled && stopped,
            ),
            (Action::Refresh, "Refresh", true),
            (Action::Back, "Back", true),
        ]
    }
    pub fn event(&mut self, event: Event, m: &mut Manager) -> Outcome {
        if let Event::Key(key) = &event
            && key.kind == KeyEventKind::Release
        {
            return Outcome::Stay;
        }
        if let Some(add) = &mut self.adding {
            match event {
                Event::Key(k) => match k.code {
                    KeyCode::Esc => {
                        self.adding = None;
                        self.message.clear();
                    }
                    KeyCode::Tab => add.focus = (add.focus + 1) % 4,
                    KeyCode::BackTab => add.focus = (add.focus + 3) % 4,
                    KeyCode::Enter if add.focus > 0 => {
                        let action = add.focus;
                        return self.add_action(action, m);
                    }
                    _ if add.focus == 0 => {
                        let old = add.input.text.clone();
                        add.input.key(k.code, false);
                        if old != add.input.text {
                            add.preview = None;
                        }
                    }
                    _ => {}
                },
                Event::Paste(s) if add.focus == 0 => {
                    if s.len() <= 65536 {
                        add.input.insert(&s, false);
                        add.preview = None;
                    }
                }
                Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                    let point = (mouse.column, mouse.row).into();
                    if add.field.contains(point) {
                        add.focus = 0;
                        add.input.click(point);
                    } else if let Some((_, i)) = self.hits.iter().find(|(r, _)| r.contains(point)) {
                        return self.add_action(*i, m);
                    }
                }
                _ => {}
            }
            return Outcome::Stay;
        }
        if let Event::Key(key) = &event {
            self.pointer.cancel();
            if matches!(key.code, KeyCode::F(1..=5)) {
                return Outcome::Page(*key);
            }
        }
        if let Event::Mouse(mouse) = &event {
            let captured = self.pointer.captured();
            if let Some((_, key)) = self.pointer.event(*mouse, &self.tabs) {
                return Outcome::Page(key);
            }
            if captured || self.pointer.captured() {
                return Outcome::Stay;
            }
        }
        let count = rows(m).len();
        self.selected = self.selected.min(count.saturating_sub(1));
        let stops = self.actions(m).len() + 1;
        self.focus = self.focus.min(stops - 1);
        let action = match event {
            Event::Key(k) => match k.code {
                KeyCode::Esc => return Outcome::Back,
                KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    None
                }
                KeyCode::Down => {
                    self.selected = (self.selected + 1).min(count.saturating_sub(1));
                    None
                }
                KeyCode::Tab => {
                    self.focus = (self.focus + 1) % stops;
                    None
                }
                KeyCode::BackTab => {
                    self.focus = (self.focus + stops - 1) % stops;
                    None
                }
                KeyCode::Enter if self.focus > 0 => Some(self.focus - 1),
                _ => None,
            },
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let p = (mouse.column, mouse.row).into();
                if let Some((_, i)) = self.rows.iter().find(|(r, _)| r.contains(p)) {
                    self.selected = *i;
                    self.focus = 0;
                    None
                } else {
                    self.hits
                        .iter()
                        .find(|(r, _)| r.contains(p))
                        .map(|(_, i)| *i)
                }
            }
            _ => None,
        };
        let Some(action) = action else {
            return Outcome::Stay;
        };
        let Some(&(action, _, true)) = self.actions(m).get(action) else {
            return Outcome::Stay;
        };
        let (id, builtin) = rows(m)
            .get(self.selected)
            .map(|r| (r.id().to_owned(), matches!(r, Row::Core(_))))
            .unwrap_or_default();
        let result = match action {
            Action::Open => return Outcome::Open(id),
            Action::Toggle if builtin => {
                let enable = m.core_state(&id) != Some(State::Enabled);
                match m.core_enabled(&id, enable) {
                    Ok(()) if !enable => {
                        self.message = "Disabled. Installed resources are kept; Remove resources deletes unmodified ones.".into();
                        return Outcome::Stay;
                    }
                    result => result,
                }
            }
            Action::Toggle => {
                if m.enabled_here(&id) {
                    m.disable(&id)
                } else {
                    m.enable(&id)
                }
            }
            Action::Restart => m.restart(&id),
            Action::Add => {
                self.adding = Some(Adding {
                    input: Input::new(String::new()),
                    preview: None,
                    focus: 0,
                    field: Rect::default(),
                });
                self.message.clear();
                Ok(())
            }
            Action::Remove => m.remove(&id),
            Action::Sync => m.core_sync(&id),
            Action::RemoveResources => m.core_remove_resources(&id),
            Action::Refresh => m.refresh(),
            Action::Back => return Outcome::Back,
        };
        self.message = result.err().map(|e| format!("{e:#}")).unwrap_or_default();
        Outcome::Stay
    }
    fn add_action(&mut self, action: usize, m: &mut Manager) -> Outcome {
        let add = self.adding.as_mut().unwrap();
        match action {
            1 => match Manifest::read(&crate::config::expand_home(&add.input.text)) {
                Ok(manifest) => {
                    add.preview = Some(manifest);
                    self.message.clear();
                }
                Err(e) => {
                    add.preview = None;
                    self.message = format!("{e:#}");
                }
            },
            2 => {
                if let Some(manifest) = &add.preview {
                    match m.add(&crate::config::expand_home(&add.input.text), manifest) {
                        Ok(()) => {
                            self.adding = None;
                            self.message = "Added disabled. Enable to start it.".into();
                            self.selected = rows(m).len().saturating_sub(1);
                        }
                        Err(e) => self.message = format!("{e:#}"),
                    }
                }
            }
            3 => {
                self.adding = None;
                self.message.clear();
            }
            _ => {}
        }
        Outcome::Stay
    }
    pub fn draw(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        m: &Manager,
        settings: &crate::settings::Settings,
    ) {
        let rows = rows(m);
        let selected_core = match rows.get(self.selected) {
            Some(Row::Core(c)) if self.adding.is_none() => Some(*c),
            _ => None,
        };
        // Built-in details may need more than the external summary's five rows.
        let width = usize::from(frame.area().width.min(76).saturating_sub(4));
        let full = selected_core.map(|c| core_detail(m, c, true));
        let extra = full
            .as_ref()
            .map_or(0, |d| wrapped(d, width).saturating_sub(5)) as u16;
        // Its longer action labels can also wrap the button bar to another row.
        let bar = if full.is_some() {
            let labels: Vec<_> = self.actions(m).iter().map(|a| a.1).collect();
            bar_rows(&labels, width) as u16 - 1
        } else {
            0
        };
        // Keep the existing details/footer room; only reserve rows for actual entries.
        let height = if self.adding.is_some() {
            28
        } else {
            19 + rows.len().clamp(1, 12) as u16 + extra + bar
        };
        let area = crate::theme::centered(frame.area(), 76, height);
        frame.render_widget(Clear, area);
        frame.render_widget(
            t.block(
                if self.adding.is_some() {
                    " Add local plugin "
                } else {
                    crate::settings::TITLE
                },
                true,
            )
            .style(t.base().bg(t.overlay)),
            area,
        );
        let inside = crate::ui::inner(area);
        let inside = Rect {
            x: inside.x + 1.min(inside.width),
            width: inside.width.saturating_sub(2),
            ..inside
        };
        self.rows.clear();
        self.hits.clear();
        self.tabs.clear();
        let choices: Vec<_> = if let Some(add) = &self.adding {
            vec![
                ("Read manifest", true),
                ("Add disabled", add.preview.is_some()),
                ("Cancel", true),
            ]
        } else {
            self.actions(m)
                .into_iter()
                .map(|(_, name, enabled)| (name, enabled))
                .collect()
        };
        let focus = self.adding.as_ref().map_or(self.focus, |a| a.focus);
        let buttons: Vec<_> = choices
            .iter()
            .enumerate()
            .map(|(i, (name, enabled))| {
                let b = Button::new(name, KeyCode::F(i as u8 + 1), *enabled);
                if focus == i + 1 { b.primary() } else { b }
            })
            .collect();
        let (body, hits) = buttons::draw_compact(t, frame, inside, &buttons);
        self.hits = hits
            .into_iter()
            .filter_map(|h| {
                if let KeyCode::F(i) = h.key.code {
                    Some((h.area, usize::from(i) - usize::from(self.adding.is_none())))
                } else {
                    None
                }
            })
            .collect();
        if let Some(add) = &mut self.adding {
            add.field = Rect::new(body.x, body.y.saturating_add(2), body.width, 1);
            add.input
                .draw(frame, add.field, add.focus == 0, "Plugin directory", t);
            let preview=add.preview.as_ref().map(|p|format!("{}  {} · {}\nProgram: {}\n\nRuns with your user permissions when enabled.\nAdding does not start the plugin.",p.name,p.version,p.id,p.program(&crate::config::expand_home(&add.input.text)).map(|p|p.display().to_string()).unwrap_or_default())).unwrap_or_else(||"Enter a directory, then Read manifest.".into());
            frame.render_widget(
                Paragraph::new(format!("Directory\n\n\n\n{preview}\n\n{}", self.message))
                    .wrap(Wrap { trim: false }),
                body,
            );
            add.input
                .draw(frame, add.field, add.focus == 0, "Plugin directory", t);
        } else {
            let (body, tabs) =
                settings.draw_header(t, frame, body, Some(crate::settings::Page::Plugins));
            self.tabs = tabs
                .into_iter()
                .map(|hit| (crate::input::Focus::Agents, hit))
                .collect();
            self.pointer.paint(t, frame, &self.tabs);
            let header =
                "Changes here apply immediately.\n\n  Name                     Enabled  Runtime";
            frame.render_widget(Paragraph::new(header), body);
            let count = usize::from(body.height.saturating_sub(13 + extra)).max(1);
            let start = self.selected.saturating_sub(count - 1);
            for (row, (i, entry)) in rows.iter().enumerate().skip(start).take(count).enumerate() {
                let r = Rect::new(body.x, body.y + 3 + row as u16, body.width, 1);
                if r.y >= body.bottom() {
                    break;
                }
                let (name, enabled, runtime) = match entry {
                    Row::Core(c) => (
                        c.name.to_owned(),
                        match m.core_state(c.id) {
                            Some(State::Enabled) => "Yes",
                            Some(State::Conflict) => "Conflict",
                            _ => "No",
                        },
                        "Built-in".to_owned(),
                    ),
                    Row::External(e) => (
                        m.manifest(&e.id)
                            .map(|p| p.name)
                            .unwrap_or_else(|_| e.id.clone()),
                        if e.enabled { "Yes" } else { "No" },
                        m.state(&e.id),
                    ),
                };
                let text = format!(
                    "{} {:24} {:8} {}",
                    if i == self.selected { ">" } else { " " },
                    crate::ui::clip(&name, 24),
                    enabled,
                    runtime
                );
                frame.render_widget(
                    Paragraph::new(text).style(Style::default().fg(if i == self.selected {
                        t.focus
                    } else {
                        t.text
                    })),
                    r,
                );
                self.rows.push((r, i));
            }
            let top = body.y + 3 + count as u16;
            let r = Rect::new(
                body.x,
                top.min(body.bottom()),
                body.width,
                body.bottom().saturating_sub(top),
            );
            let detail = match rows.get(self.selected) {
                // Short windows keep the switch and resource summary; the long setup note
                // and paths stay available through the read-only status command.
                Some(Row::Core(c)) => full
                    .filter(|d| wrapped(d, usize::from(r.width)) + 3 <= usize::from(r.height))
                    .unwrap_or_else(|| core_detail(m, c, false)),
                Some(Row::External(e)) => format!(
                    "ID: {}\nDirectory: {}\nProgram: {}\n{}\n{}",
                    e.id,
                    e.directory.display(),
                    m.program(&e.id)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Unavailable".into()),
                    if e.enabled != m.enabled_here(&e.id) {
                        "Registry differs from this instance. Enable/Disable here explicitly."
                    } else {
                        "Remove deletes registration only; files are kept."
                    },
                    m.note(&e.id)
                ),
                None => "No plugins registered. Add a local directory to begin.".into(),
            };
            frame.render_widget(
                Paragraph::new(format!(
                    "\n{detail}\n{}\n↑↓ Select · Tab Focus · Enter Activate · Esc Back",
                    m.registry.error.as_deref().unwrap_or(&self.message)
                ))
                .wrap(Wrap { trim: false }),
                r,
            );
        }
    }
}

/// The host renders only plugin-provided text and resolved paths; it never parses them.
fn core_detail(m: &Manager, c: &saddle_core_plugin::Manifest, full: bool) -> String {
    let state = match m.core_state(c.id) {
        Some(State::Enabled) => "Enabled",
        Some(State::Conflict) => "Conflict",
        _ => "Disabled",
    };
    let mut lines = vec![format!("ID: {} · Built-in · {state}", c.id)];
    if state == "Conflict" {
        lines.push("An external plugin uses this ID; built-in commands are refused.".into());
    }
    let status = m.core_resources(c.id).and_then(|s| match s {
        Ok(s) => Some(s),
        Err(e) => {
            lines.push(format!("Resources unavailable: {e}"));
            None
        }
    });
    for r in status.iter().flat_map(|s| &s.resources) {
        let kind = if r.kind == "agent_skill" {
            "Skill"
        } else {
            "Resource"
        };
        match &r.error {
            Some(e) => lines.push(format!("{kind}: {} — not installable ({e})", r.name)),
            None => {
                lines.push(format!("{kind}: {}", r.name));
                for t in &r.targets {
                    lines.push(format!("  {:13} {}", t.label, t.text()));
                }
            }
        }
    }
    let result = m.core_receipt(c.id).map(|r| r.summary());
    if !full {
        lines.extend(result);
        if !c.setup_note.is_empty() || !c.setup_files.is_empty() {
            lines.push(format!(
                "Setup note and paths: saddle plugin status {}",
                c.id
            ));
        }
        return lines.join("\n");
    }
    if !c.setup_note.is_empty() || !c.setup_files.is_empty() {
        lines.push(String::new());
        lines.extend(c.setup_note.lines().map(str::to_owned));
        for (i, f) in c.setup_files.iter().enumerate() {
            let paths = status
                .and_then(|s| s.setup_files.get(i))
                .map(|s| s.paths.as_slice())
                .unwrap_or_default();
            if paths.is_empty() {
                lines.push(format!(
                    "{}: not installed (see skill status above)",
                    f.label
                ));
            }
            lines.extend(
                paths
                    .iter()
                    .map(|p| format!("{}: {}", f.label, p.display())),
            );
        }
    }
    if let Some(result) = result {
        lines.push(String::new());
        lines.push(result);
    }
    lines.join("\n")
}
/// Rows of the compact button bar (same greedy rule as `buttons::draw_compact`).
fn bar_rows(labels: &[&str], width: usize) -> usize {
    let (mut rows, mut x) = (1, 0);
    for label in labels {
        let w = (label.width() + 2).min(width);
        if x > 0 && x + w > width {
            rows += 1;
            x = 0;
        }
        x += w + 1;
    }
    rows
}
/// Rows a word-wrapped paragraph needs, plus one spare so the result line is never cut.
fn wrapped(text: &str, width: usize) -> usize {
    // Long unbroken runs lose up to one column per row to double-width characters.
    let (width, run) = (width.max(2), width.max(2) - 1);
    let rows = |line: &str| {
        let (mut rows, mut used) = (1, 0);
        for word in line.split(' ') {
            let w = word.width();
            if used + usize::from(used > 0) + w <= width {
                used += usize::from(used > 0) + w;
                continue;
            }
            rows += usize::from(used > 0);
            rows += w.saturating_sub(1) / run;
            used = w - w.saturating_sub(1) / run * run;
        }
        rows
    };
    text.lines().map(rows).sum::<usize>() + 1
}
