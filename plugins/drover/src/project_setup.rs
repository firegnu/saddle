//! Tasks' project form; all effects are explicit and run in the plugin's command worker.
use crate::{
    api,
    buttons::{self, Button as B},
    launch_edit::Input,
    theme::Theme,
};
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Wrap},
};
use serde_json::{Value, json};

#[derive(PartialEq)]
enum Mode {
    Form,
    Directories,
    Agents,
}
pub struct Setup {
    worker: api::Worker,
    pub path: Input,
    name: Input,
    agent: String,
    settings: bool,
    focus: usize,
    mode: Mode,
    choices: Vec<String>,
    selected: usize,
    info: Option<Value>,
    message: String,
    pending: Option<&'static str>,
    pub closed: bool,
    pub opened: Option<(String, String)>,
    fields: Vec<(Rect, usize)>,
    rows: Vec<(Rect, usize)>,
}
impl Setup {
    pub fn new(corral: String, path: String, settings: bool) -> Self {
        let mut s = Self {
            worker: api::Worker::start(corral),
            path: Input::new(path),
            name: Input::new(String::new()),
            agent: String::new(),
            settings,
            focus: 0,
            mode: Mode::Form,
            choices: vec![],
            selected: 0,
            info: None,
            message: String::new(),
            pending: None,
            closed: false,
            opened: None,
            fields: vec![],
            rows: vec![],
        };
        s.check();
        s
    }
    fn request(&mut self, method: &'static str, mut params: Value) {
        let path = crate::config::expand_home(&self.path.text);
        if !path.is_absolute() {
            self.message = "Choose an absolute project directory.".into();
            return;
        }
        params["project"] = json!(path);
        match self.worker.request(method.into(), method.into(), params) {
            Ok(()) => {
                self.pending = Some(method);
                self.message = "Reading…".into();
            }
            Err(e) => self.message = format!("Unavailable: {e:#}"),
        }
    }
    fn check(&mut self) {
        self.info = None;
        self.request("project-info", json!({}));
    }
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok((method, value)) = self.worker.results.try_recv() {
            changed = true;
            self.pending = None;
            if value["ok"] != true {
                self.message = format!("Could not complete: {}", value["error"]);
                continue;
            }
            self.message.clear();
            match method.as_str() {
                "project-info" => {
                    self.path =
                        Input::new(value["project"].as_str().unwrap_or(&self.path.text).into());
                    self.agent = value["main_agent"].as_str().unwrap_or("").into();
                    if self.name.text.is_empty() {
                        let name = std::path::Path::new(&self.path.text)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();
                        self.name = Input::new(
                            name.chars()
                                .map(|c| {
                                    if c.is_ascii_lowercase() || c.is_ascii_digit() {
                                        c
                                    } else {
                                        '-'
                                    }
                                })
                                .collect(),
                        );
                    }
                    self.info = Some(value);
                }
                "project-save" => {
                    self.opened = Some((
                        value["project"].as_str().unwrap_or(&self.path.text).into(),
                        value["warning"]
                            .as_str()
                            .unwrap_or("Project saved; no task dispatched.")
                            .into(),
                    ));
                }
                "project-browse" => {
                    self.path =
                        Input::new(value["project"].as_str().unwrap_or(&self.path.text).into());
                    self.info = None;
                    self.choices = std::path::Path::new(&self.path.text)
                        .parent()
                        .map(|p| p.display().to_string())
                        .into_iter()
                        .chain(
                            value["directories"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|v| v.as_str().map(str::to_owned)),
                        )
                        .collect();
                    self.selected = 0;
                    self.mode = Mode::Directories;
                }
                "project-agents" => {
                    self.choices = vec![String::new()];
                    self.choices.extend(
                        value["agents"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v.as_str().map(str::to_owned)),
                    );
                    if !self.agent.is_empty() && !self.choices.contains(&self.agent) {
                        self.choices.push(self.agent.clone());
                    }
                    self.selected = self
                        .choices
                        .iter()
                        .position(|a| a == &self.agent)
                        .unwrap_or(0);
                    self.mode = Mode::Agents;
                }
                _ => {}
            }
        }
        changed
    }
    fn state(&self) -> &str {
        self.info
            .as_ref()
            .and_then(|v| v["state"].as_str())
            .unwrap_or("unchecked")
    }
    fn editable_agent(&self) -> bool {
        self.state() == "new" || (self.settings && self.state() == "registered")
    }
    pub fn paste(&mut self, text: &str) {
        if self.pending.is_some() || self.mode != Mode::Form {
            return;
        }
        if self.focus == 0 && !self.settings {
            self.path.insert(text, false);
            self.info = None;
            self.name.clear();
        }
        if self.focus == 1 && self.state() == "new" {
            self.name.insert(text, false);
        }
    }
    pub fn key(&mut self, key: KeyEvent) {
        if self.pending.is_some() {
            return;
        }
        if self.mode != Mode::Form {
            match key.code {
                K::Esc => {
                    self.mode = Mode::Form;
                    if self.info.is_none() {
                        self.check();
                    }
                }
                K::Up => self.selected = self.selected.saturating_sub(1),
                K::Down => {
                    self.selected = (self.selected + 1).min(self.choices.len().saturating_sub(1))
                }
                K::Enter => {
                    if let Some(choice) = self.choices.get(self.selected).cloned() {
                        if self.mode == Mode::Agents {
                            self.agent = choice;
                            self.mode = Mode::Form;
                        } else {
                            self.path = Input::new(choice);
                            self.request("project-browse", json!({}));
                        }
                    }
                }
                K::F(3) if self.mode == Mode::Directories => {
                    self.mode = Mode::Form;
                    self.name.clear();
                    self.check();
                }
                _ => {}
            }
            return;
        }
        match key.code {
            K::Esc => self.closed = true,
            K::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.focus == 0 && !self.settings {
                    self.path.clear();
                    self.info = None;
                    self.name.clear();
                } else if self.focus == 1 && self.state() == "new" {
                    self.name.clear();
                }
            }
            K::Tab => self.focus = (self.focus + 1) % 3,
            K::BackTab => self.focus = (self.focus + 2) % 3,
            K::F(2) if !self.settings => self.request("project-browse", json!({})),
            K::F(3) => self.check(),
            K::F(5) if self.editable_agent() => self.agent.clear(),
            K::F(4) if self.editable_agent() => self.request("project-agents", json!({})),
            K::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => self.save(),
            K::Enter if self.focus == 0 => self.check(),
            K::Enter if self.focus == 2 && self.editable_agent() => {
                self.request("project-agents", json!({}))
            }
            _ if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if self.focus == 0 && !self.settings {
                    let before = self.path.text.clone();
                    self.path.key(key.code, false);
                    if self.path.text != before {
                        self.info = None;
                        self.name.clear();
                    }
                } else if self.focus == 1 && self.state() == "new" {
                    self.name.key(key.code, false);
                }
            }
            _ => {}
        }
    }
    fn save(&mut self) {
        if self.state() == "registered" && !self.settings {
            self.opened = Some((
                self.path.text.clone(),
                "Already added; opened existing project.".into(),
            ));
        } else if matches!(self.state(), "new" | "existing" | "registered") {
            self.request("project-save", json!({"name":self.name.text,"main_agent":self.agent,"token":self.info.as_ref().unwrap()["token"]}));
            if self.pending.is_some() {
                self.message = "Saving…".into();
            }
        }
    }
    pub fn click(&mut self, x: u16, y: u16) {
        if self.pending.is_some() {
            return;
        }
        if let Some((_, focus)) = self.fields.iter().find(|(r, _)| r.contains((x, y).into())) {
            self.focus = *focus;
            if *focus == 0 {
                self.path.click((x, y).into());
            }
            if *focus == 1 {
                self.name.click((x, y).into());
            }
            if *focus == 2 {
                self.key(KeyEvent::new(K::F(4), KeyModifiers::NONE));
            }
        } else if let Some((_, i)) = self.rows.iter().find(|(r, _)| r.contains((x, y).into())) {
            self.selected = *i;
            self.key(KeyEvent::new(K::Enter, KeyModifiers::NONE));
        }
    }
    pub fn draw(&mut self, t: &Theme, f: &mut Frame, area: Rect) -> Vec<buttons::Hit> {
        let title = match self.mode {
            Mode::Directories => "Choose directory",
            Mode::Agents => "Choose default receiver",
            Mode::Form if self.settings => "Project settings",
            Mode::Form => "Add project",
        };
        let area = crate::ui::dialog(t, f, area, title, 96, 26);
        let ready = self.pending.is_none();
        let controls = if self.mode == Mode::Directories {
            vec![
                B::new("Use directory F3", K::F(3), ready),
                B::new("Back Esc", K::Esc, ready),
            ]
        } else if self.mode == Mode::Agents {
            vec![
                B::new("Select Enter", K::Enter, ready),
                B::new("Cancel Esc", K::Esc, ready),
            ]
        } else {
            vec![
                B::control(
                    if self.state() == "registered" && !self.settings {
                        "Open ^s"
                    } else if self.state() == "existing" {
                        "Reuse & add ^s"
                    } else {
                        "Save ^s"
                    },
                    K::Char('s'),
                    ready && matches!(self.state(), "new" | "existing" | "registered"),
                )
                .primary(),
                B::new("Cancel Esc", K::Esc, ready),
            ]
        };
        let (body, mut hits) = buttons::draw_compact(t, f, area, &controls);
        self.fields.clear();
        self.rows.clear();
        if body.height < 5 {
            return hits;
        }
        if self.mode != Mode::Form {
            let label = if self.mode == Mode::Directories {
                format!("Directory: {}", self.path.text)
            } else {
                "Default receiver · choose an existing agent (optional)".into()
            };
            f.render_widget(
                Paragraph::new(safe(&label)),
                Rect::new(body.x, body.y + 1, body.width, 1),
            );
            let height = usize::from(body.height.saturating_sub(4));
            let top = self.selected.saturating_sub(height.saturating_sub(1));
            for (row, (i, choice)) in self
                .choices
                .iter()
                .enumerate()
                .skip(top)
                .take(height)
                .enumerate()
            {
                let r = Rect::new(body.x, body.y + 2 + row as u16, body.width, 1);
                let label = if choice.is_empty() {
                    "Not specified (manual handoff)".into()
                } else {
                    safe(choice)
                };
                f.render_widget(
                    Paragraph::new(format!(
                        "{} {label}",
                        if i == self.selected { ">" } else { " " }
                    ))
                    .style(if i == self.selected {
                        Style::default().bg(t.selected)
                    } else {
                        Style::default()
                    }),
                    r,
                );
                self.rows.push((r, i));
            }
        } else {
            let fields = [
                ("Directory", 0),
                ("Project short name", 1),
                ("Default receiver", 2),
            ];
            let mut remaining = Rect {
                height: body.height.saturating_sub(2),
                ..body
            };
            for (label, i) in fields {
                let y = remaining.y;
                if remaining.height < 3 {
                    break;
                }
                let editable = match i {
                    0 => !self.settings,
                    1 => self.state() == "new",
                    _ => self.editable_agent(),
                };
                let field = t.block(
                    format!(" {label}{} ", if editable { "" } else { " (read-only)" }),
                    self.focus == i && ready && editable,
                );
                let field_area = Rect::new(body.x, y, body.width, 3);
                let r = field.inner(field_area);
                f.render_widget(field, field_area);
                self.fields.push((r, i));
                match i {
                    0 => self.path.draw(
                        f,
                        r,
                        self.focus == 0 && ready && !self.settings,
                        "Absolute repository directory",
                        t,
                    ),
                    1 if self.state() == "new" => {
                        self.name
                            .draw(f, r, self.focus == 1 && ready, "lowercase-name", t)
                    }
                    1 => f.render_widget(
                        Paragraph::new(if matches!(self.state(), "registered" | "existing") {
                            "Existing configuration will be preserved"
                        } else {
                            "Check directory first"
                        }),
                        r,
                    ),
                    _ => {
                        f.render_widget(
                            Paragraph::new(if self.agent.is_empty() {
                                "Not specified (manual handoff)".into()
                            } else {
                                safe(&self.agent)
                            })
                            .style(
                                Style::default().fg(if self.focus == 2 { t.focus } else { t.text }),
                            ),
                            r,
                        )
                    }
                }
                remaining.y += 3;
                remaining.height -= 3;
                let controls = match i {
                    0 => vec![
                        B::new("Browse F2", K::F(2), ready && !self.settings),
                        B::new("Check / Reload F3", K::F(3), ready),
                    ],
                    2 => vec![
                        B::new("Choose agent F4", K::F(4), ready && self.editable_agent()),
                        B::new("No receiver F5", K::F(5), ready && self.editable_agent()),
                    ],
                    _ => vec![],
                };
                let (rest, row_hits) = buttons::draw_compact_top(t, f, remaining, &controls);
                remaining = rest;
                hits.extend(row_hits);
            }
            let state = match self.state() {
                "new" => {
                    "Not added to Tasks. Save initializes project configuration and task storage."
                }
                "existing" => {
                    "Existing Drover configuration found. Reuse it without resetting tasks or receiver."
                }
                "registered" => "Already added to Tasks.",
                "unavailable" => {
                    "Project unavailable; existing configuration will not be overwritten."
                }
                _ => "Choose a directory, then Check.",
            };
            if !remaining.is_empty() {
                let error = self
                    .info
                    .as_ref()
                    .and_then(|v| v["error"].as_str())
                    .unwrap_or("");
                f.render_widget(Paragraph::new(safe(&format!("{state}\n{error}\nNo task is dispatched. AGENTS.md remains manually maintained.\nTab: next field. Empty receiver means manual handoff."))).wrap(Wrap{trim:false}),remaining);
            }
        }
        f.render_widget(
            Paragraph::new(safe(&self.message)).wrap(Wrap { trim: false }),
            Rect::new(body.x, body.bottom().saturating_sub(2), body.width, 2),
        );
        hits
    }
}
pub fn safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != '\n' {
                '�'
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, layout::Position};

    #[test]
    fn compact_form_keeps_fields_and_actions_visible_and_clickable() {
        for (width, height) in [(48, 24), (80, 24), (160, 50)] {
            let mut setup = Setup {
                worker: api::Worker::start("unused-fake-corral".into()),
                path: Input::new("/tmp/example-project".into()),
                name: Input::new("example-project".into()),
                agent: "example/main".into(),
                settings: false,
                focus: 0,
                mode: Mode::Form,
                choices: vec![],
                selected: 0,
                info: Some(json!({"state": "new"})),
                message: "Not saved: example failure".into(),
                pending: None,
                closed: false,
                opened: None,
                fields: vec![],
                rows: vec![],
            };
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut hits = vec![];
            terminal
                .draw(|f| {
                    hits = setup.draw(&Theme::default(), f, f.area());
                })
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            for label in [
                "Add project",
                "Directory",
                "Project short name",
                "Default receiver",
                "Not saved:",
            ] {
                assert!(text.contains(label), "{width}x{height}: {label}");
            }
            assert_eq!(setup.fields.len(), 3);
            for key in [
                KeyEvent::from(K::F(2)),
                KeyEvent::from(K::F(3)),
                KeyEvent::from(K::F(4)),
                KeyEvent::from(K::F(5)),
                KeyEvent::new(K::Char('s'), KeyModifiers::CONTROL),
                KeyEvent::from(K::Esc),
            ] {
                let hit = hits
                    .iter()
                    .find(|h| h.key == key)
                    .expect("action has a click target");
                assert!(
                    terminal
                        .backend()
                        .buffer()
                        .area
                        .contains(Position::new(hit.area.x, hit.area.y))
                );
                assert!(
                    setup
                        .fields
                        .iter()
                        .all(|(field, _)| !field.intersects(hit.area))
                );
            }
            let name = setup.fields[1].0;
            setup.click(name.x, name.y);
            assert_eq!(setup.focus, 1);
        }
    }
}
