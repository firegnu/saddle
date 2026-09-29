//! Settings: view, edit and save the existing config file from inside saddle. Edits stay a
//! draft until Save; Save writes only the edited keys, keeping the rest of the file as it is.
//! Its Diagnostics page only reads.
use crate::{
    buttons::{self, Button},
    config::Config,
    launch::edit::Input,
    theme::{Theme, color_name, parse_color},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use unicode_width::UnicodeWidthStr;

pub const TITLE: &str = " Settings ";
const LABEL: usize = 18;
/// The Task notifications choices; the key names no config setting, as Drover keeps it.
const NOTIFICATIONS: &str = "notifications";
const SYSTEM: &str = "System";
const IN_SADDLE: &str = "In saddle";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Colors,
    Advanced,
    Diagnostics,
}
const PAGES: [(Page, &str, u8); 4] = [
    (Page::General, "General F1", 1),
    (Page::Colors, "Colors F2", 2),
    (Page::Advanced, "Advanced F3", 3),
    (Page::Diagnostics, "Diagnostics F4", 4),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Columns,
    Millis,
    Project,
    Command,
    Color,
    /// Task notifications: System or In saddle, read from and saved to Drover.
    Channel,
}
struct Field {
    /// The config key, dotted below a table (`queue.cwd`, `colors.bg`).
    key: String,
    label: &'static str,
    page: Page,
    group: &'static str,
    kind: Kind,
    /// Applies from the next start; saving does not change the running saddle.
    restart: bool,
}

/// Color groups in display order.
const GROUPS: [&str; 4] = ["Interface", "Agents panel", "Status", "Agent types"];
fn group(name: &str) -> &'static str {
    match name {
        "agent_selected" => GROUPS[1],
        _ if name.starts_with("agents_") => GROUPS[1],
        _ if name.starts_with("agent_") => GROUPS[2],
        "claude" | "codex" | "pi" | "omp" => GROUPS[3],
        _ => GROUPS[0],
    }
}

fn fields() -> Vec<Field> {
    let field = |key: &str, label, page, kind, restart| Field {
        key: key.into(),
        label,
        page,
        group: "",
        kind,
        restart,
    };
    let mut list = vec![
        field(
            "left_width",
            "Sidebar width",
            Page::General,
            Kind::Columns,
            false,
        ),
        field(
            "refresh_ms",
            "Refresh interval",
            Page::General,
            Kind::Millis,
            true,
        ),
        field(
            "queue.cwd",
            "Initial project",
            Page::General,
            Kind::Project,
            true,
        ),
        field(
            NOTIFICATIONS,
            "Task notifications",
            Page::General,
            Kind::Channel,
            false,
        ),
    ];
    let mut colors: Vec<_> = Theme::default()
        .named_mut()
        .into_iter()
        .map(|(name, _)| Field {
            key: format!("colors.{name}"),
            label: name,
            page: Page::Colors,
            group: group(name),
            kind: Kind::Color,
            restart: false,
        })
        .collect();
    colors.sort_by_key(|f| GROUPS.iter().position(|g| *g == f.group));
    list.extend(colors);
    list.extend([
        field(
            "corral",
            "corral command",
            Page::Advanced,
            Kind::Command,
            true,
        ),
        field(
            "queue.drover",
            "drover command",
            Page::Advanced,
            Kind::Command,
            true,
        ),
    ]);
    list
}

/// A setting as the form shows it; an omitted initial project is empty (Automatic).
fn value(config: &Config, field: &Field) -> String {
    match field.key.as_str() {
        "left_width" => config.left_width.to_string(),
        "refresh_ms" => config.refresh_ms.to_string(),
        "queue.cwd" => config.queue.cwd.clone().unwrap_or_default(),
        "corral" => config.corral.clone(),
        "queue.drover" => config.queue.drover.clone(),
        NOTIFICATIONS => String::new(),
        key => color(&config.colors, &key["colors.".len()..]).map_or_else(String::new, color_name),
    }
}
fn color(theme: &Theme, name: &str) -> Option<Color> {
    let mut theme = theme.clone();
    theme
        .named_mut()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
}

pub enum Outcome {
    Stay,
    Cancel,
    /// Written: the saved configuration and the labels of saved settings that need a restart.
    Saved(Box<Config>, Vec<&'static str>),
    /// Ask Drover to turn its system notifications on or off; any config edits are already
    /// written (and should apply now). Settings waits for `channel_saved`.
    SetChannel(Option<Box<Config>>, bool),
    /// Everything saved: close with this message.
    Done(String),
    /// Diagnostics opened or Refresh pressed: check again and answer with `diagnose`.
    Diagnose,
    /// Copy this diagnostics summary; answer with `copied`.
    Copy(String),
}

/// Drover's notification preference as Settings knows it.
enum Remote {
    Loading,
    Ready,
    Failed(String),
}

pub struct Settings {
    path: PathBuf,
    truecolor: bool,
    fields: Vec<Field>,
    defaults: Vec<String>,
    /// The file as last read (None: no file yet). Save refuses when the disk no longer matches.
    base: Option<String>,
    /// The values in `base`, with defaults for omitted settings.
    saved: Vec<String>,
    inputs: Vec<Input>,
    page: Page,
    selected: usize,
    top: usize,
    message: String,
    error: bool,
    conflict: bool,
    /// The file cannot be read or is invalid: nothing to edit until it is reloaded.
    broken: Option<String>,
    /// Whether the failed reload was keeping the draft; retrying keeps that choice.
    keeping: bool,
    /// Field rows as last drawn with their input areas, for clicks.
    rows: Vec<(Rect, Rect, usize)>,
    remote: Remote,
    /// Waiting for Drover to save Task notifications.
    saving: bool,
    /// Restart labels of config settings this Save already wrote, while Drover is still asked.
    written: Option<Vec<&'static str>>,
    /// The Task notifications choices as last drawn, for clicks.
    choices: Vec<(Rect, &'static str)>,
    /// What Diagnostics shows; None until the first check is asked for.
    report: Option<crate::diagnostics::Report>,
    /// The first Diagnostics line shown.
    report_top: u16,
}

impl Settings {
    pub fn open(path: PathBuf, truecolor: bool) -> Self {
        let fields = fields();
        let mut defaults: Vec<_> = fields
            .iter()
            .map(|f| value(&Config::default(), f))
            .collect();
        let inputs = defaults.iter().map(|v| Input::new(v.clone())).collect();
        let saved = defaults.clone();
        // Drover's own default: its system notifications are on.
        defaults[channel(&fields)] = SYSTEM.into();
        let mut settings = Self {
            path,
            truecolor,
            inputs,
            saved,
            defaults,
            fields,
            base: None,
            page: Page::General,
            selected: 0,
            top: 0,
            message: String::new(),
            error: false,
            conflict: false,
            broken: None,
            keeping: false,
            rows: Vec::new(),
            remote: Remote::Loading,
            saving: false,
            written: None,
            choices: Vec::new(),
            report: None,
            report_top: 0,
        };
        settings.reload(false);
        settings
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// The draft value of a setting, by config key.
    pub fn value(&self, key: &str) -> Option<&str> {
        let i = self.fields.iter().position(|f| f.key == key)?;
        Some(&self.inputs[i].text)
    }
    /// A new Diagnostics report; its background checks follow in `checked`.
    pub fn diagnose(&mut self, report: crate::diagnostics::Report) {
        self.report = Some(report);
        self.message.clear();
        self.error = false;
    }
    pub fn checked(&mut self, checks: crate::diagnostics::Checks) {
        if let Some(report) = &mut self.report {
            report.checks = Some(checks);
        }
    }
    pub fn copied(&mut self, result: Result<(), String>) {
        (self.message, self.error) = match result {
            Ok(()) => ("Copied the diagnostics summary.".into(), false),
            Err(error) => (format!("Copy failed: {error}"), true),
        };
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn conflict(&self) -> bool {
        self.conflict
    }
    /// Drover's answer to the reading asked for when Settings opened: whether its system
    /// notifications are enabled, or why that is unknown.
    pub fn channel_status(&mut self, result: Result<bool, String>) {
        if self.saving {
            return;
        }
        let i = channel(&self.fields);
        match result {
            Ok(system) => {
                let drafted =
                    matches!(self.remote, Remote::Ready) && self.inputs[i].text != self.saved[i];
                self.saved[i] = choice(system).into();
                if !drafted {
                    self.inputs[i] = Input::new(self.saved[i].clone());
                }
                self.remote = Remote::Ready;
            }
            Err(error) => {
                self.saved[i].clear();
                self.inputs[i] = Input::new(String::new());
                self.remote = Remote::Failed(error);
            }
        }
    }
    /// Drover's answer to `SetChannel`. Success closes Settings; a failure keeps the choice as
    /// a draft and says what was and was not saved.
    pub fn channel_saved(&mut self, result: Result<bool, String>) -> Outcome {
        self.saving = false;
        match result {
            Ok(system) => {
                let applied = format!(
                    "Task notifications: {}; Drover applies it at its next notification check.",
                    choice(system)
                );
                Outcome::Done(match self.written.take() {
                    Some(restart) if !restart.is_empty() => format!(
                        "Settings saved; restart saddle to apply: {}. {applied}",
                        restart.join(", ")
                    ),
                    _ => format!("Settings saved. {applied}"),
                })
            }
            Err(error) => {
                let saved = if self.written.is_some() {
                    "Config saved. "
                } else {
                    ""
                };
                self.fail(format!("{saved}Task notifications not saved: {error}"))
            }
        }
    }
    fn edited(&self) -> Vec<usize> {
        (0..self.fields.len())
            .filter(|&i| self.inputs[i].text != self.saved[i])
            .collect()
    }
    fn read(&self) -> anyhow::Result<Option<String>> {
        match fs::read_to_string(&self.path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(anyhow::Error::new(error).context(format!("reading {}", self.path.display())))
            }
        }
    }
    /// Rereads the file. `keep` keeps edited values as the draft on top of it; otherwise the
    /// draft becomes the file's values.
    fn reload(&mut self, keep: bool) {
        let loaded = self.read().and_then(|text| {
            let config = Config::parse(text.as_deref().unwrap_or(""))
                .map_err(|e| e.context(format!("invalid config {}", self.path.display())))?;
            Ok((text, config))
        });
        let (text, config) = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                self.broken = Some(format!("{error:#}"));
                self.keeping = keep;
                self.conflict = false;
                return;
            }
        };
        let edited = if keep { self.edited() } else { Vec::new() };
        let drover = std::mem::take(&mut self.saved[channel(&self.fields)]);
        self.saved = self.fields.iter().map(|f| value(&config, f)).collect();
        self.saved[channel(&self.fields)] = drover;
        for (i, input) in self.inputs.iter_mut().enumerate() {
            if !edited.contains(&i) {
                *input = Input::new(self.saved[i].clone());
            }
        }
        self.base = text;
        self.broken = None;
        self.conflict = false;
        self.error = false;
        self.message = if keep && !edited.is_empty() {
            "Reloaded; your edits are kept as an unsaved draft.".into()
        } else {
            String::new()
        };
    }
    fn fail(&mut self, message: String) -> Outcome {
        self.message = message;
        self.error = true;
        Outcome::Stay
    }
    fn select(&mut self, index: usize) {
        self.selected = index;
        self.page = self.fields[index].page;
    }
    fn page_fields(&self) -> Vec<usize> {
        (0..self.fields.len())
            .filter(|&i| self.fields[i].page == self.page)
            .collect()
    }
    fn step(&mut self, delta: isize) {
        let list = self.page_fields();
        let at = list.iter().position(|&i| i == self.selected).unwrap_or(0);
        let next = (at as isize + delta).rem_euclid(list.len() as isize) as usize;
        self.selected = list[next];
    }
    fn show(&mut self, page: Page) {
        self.page = page;
        self.selected = self.page_fields()[0];
        self.top = 0;
    }

    pub fn key(&mut self, key: KeyEvent) -> Outcome {
        if self.saving {
            return Outcome::Stay;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && matches!(key.code, KeyCode::Char(']' | '5')) {
            return Outcome::Cancel;
        }
        if self.conflict {
            match key.code {
                KeyCode::Char('k') => self.reload(true),
                KeyCode::Char('d') => self.reload(false),
                KeyCode::Esc => {
                    self.conflict = false;
                    self.message = "Not saved: the file changed on disk.".into();
                }
                _ => {}
            }
            return Outcome::Stay;
        }
        if key.code == KeyCode::Esc {
            return Outcome::Cancel;
        }
        // Diagnostics stays reachable while the config file cannot be used.
        if self.page == Page::Diagnostics || key.code == KeyCode::F(4) {
            return self.diagnostics_key(key);
        }
        if self.broken.is_some() {
            if ctrl && key.code == KeyCode::Char('r') {
                self.reload(self.keeping);
            }
            return Outcome::Stay;
        }
        if let KeyCode::F(n @ 1..=3) = key.code {
            self.show(PAGES[usize::from(n - 1)].0);
            return Outcome::Stay;
        }
        let choosing = self.fields[self.selected].kind == Kind::Channel;
        // Nothing to choose from until Drover answers; moving on and saving still work.
        let allowed = matches!(self.remote, Remote::Ready)
            || (ctrl && key.code == KeyCode::Char('s'))
            || matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab
            );
        if choosing && !allowed {
            return Outcome::Stay;
        }
        if ctrl {
            match key.code {
                KeyCode::Char('s') => return self.save(),
                KeyCode::Char('u') if choosing => {}
                KeyCode::Char('d') => {
                    self.inputs[self.selected] = Input::new(self.defaults[self.selected].clone())
                }
                KeyCode::Char('u') => self.inputs[self.selected].clear(),
                _ => {}
            }
            return Outcome::Stay;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return Outcome::Stay;
        }
        match key.code {
            KeyCode::Up | KeyCode::BackTab => self.step(-1),
            KeyCode::Down | KeyCode::Tab => self.step(1),
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') | KeyCode::Enter if choosing => {
                let other = if self.inputs[self.selected].text == SYSTEM {
                    IN_SADDLE
                } else {
                    SYSTEM
                };
                self.inputs[self.selected] = Input::new(other.into());
            }
            _ if choosing => {}
            code => self.inputs[self.selected].key(code, false),
        }
        Outcome::Stay
    }
    /// Diagnostics: Refresh, Copy summary, Close and scrolling; F1–F3 go back to the settings.
    fn diagnostics_key(&mut self, key: KeyEvent) -> Outcome {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return Outcome::Stay;
        }
        match key.code {
            KeyCode::F(4) | KeyCode::Char('r') => {
                self.page = Page::Diagnostics;
                self.report_top = 0;
                return Outcome::Diagnose;
            }
            KeyCode::F(n @ 1..=3) => self.show(PAGES[usize::from(n - 1)].0),
            KeyCode::Char('c') => {
                if let Some(report) = &self.report {
                    return Outcome::Copy(report.summary());
                }
            }
            KeyCode::Up => self.report_top = self.report_top.saturating_sub(1),
            KeyCode::Down => self.report_top = self.report_top.saturating_add(1),
            _ => {}
        }
        Outcome::Stay
    }
    pub fn paste(&mut self, text: &str) {
        if self.page != Page::Diagnostics
            && !self.conflict
            && self.broken.is_none()
            && !self.saving
            && self.fields[self.selected].kind != Kind::Channel
        {
            self.inputs[self.selected].insert(text, false);
        }
    }
    pub fn click(&mut self, point: Position) {
        if self.conflict || self.broken.is_some() || self.saving || self.page == Page::Diagnostics {
            return;
        }
        if let Some(&(_, choice)) = self.choices.iter().find(|(area, _)| area.contains(point)) {
            let i = channel(&self.fields);
            self.selected = i;
            self.inputs[i] = Input::new(choice.into());
            return;
        }
        if let Some(&(_, input, i)) = self.rows.iter().find(|(row, _, _)| row.contains(point)) {
            self.selected = i;
            if input.contains(point) {
                self.inputs[i].click(point);
            }
        }
    }
    pub fn scroll(&mut self, down: bool) {
        if self.page == Page::Diagnostics {
            self.report_top = if down {
                self.report_top.saturating_add(1)
            } else {
                self.report_top.saturating_sub(1)
            };
        } else if !self.conflict && self.broken.is_none() && !self.saving {
            self.step(if down { 1 } else { -1 });
        }
    }

    /// Checks and writes the edited settings. Nothing is written, and the draft stays, when a
    /// value is invalid, the file changed on disk, or writing fails. A changed Task
    /// notifications choice goes to Drover after the file is written.
    fn save(&mut self) -> Outcome {
        let mut edited = self.edited();
        if edited.is_empty() {
            return Outcome::Cancel;
        }
        let i = channel(&self.fields);
        let set = edited.contains(&i).then(|| self.inputs[i].text == SYSTEM);
        edited.retain(|&e| e != i);
        let config = if edited.is_empty() {
            None
        } else {
            match self.write_config(&edited) {
                Ok(written) => Some(written),
                Err(outcome) => return outcome,
            }
        };
        let Some(system) = set else {
            let (config, restart) = config.expect("edited config settings were written");
            return Outcome::Saved(config, restart);
        };
        let config = config.map(|(config, restart)| {
            // Saved now, whatever Drover answers.
            for &e in &edited {
                self.saved[e] = self.inputs[e].text.clone();
            }
            self.written = Some(restart);
            config
        });
        self.saving = true;
        self.error = false;
        self.message = "Saving Task notifications in Drover…".into();
        Outcome::SetChannel(config, system)
    }
    fn write_config(
        &mut self,
        edited: &[usize],
    ) -> Result<(Box<Config>, Vec<&'static str>), Outcome> {
        for &i in edited {
            let (field, text) = (&self.fields[i], self.inputs[i].text.trim());
            let problem = match field.kind {
                Kind::Columns => text
                    .parse::<u16>()
                    .err()
                    .map(|_| format!("{} must be a whole number of columns", field.label)),
                Kind::Millis => text
                    .parse::<u64>()
                    .err()
                    .map(|_| format!("{} must be a whole number of milliseconds", field.label)),
                Kind::Color => parse_color(text)
                    .err()
                    .map(|e| format!("{}: {e}", field.label)),
                Kind::Project | Kind::Command | Kind::Channel => None,
            };
            if let Some(problem) = problem {
                self.select(i);
                return Err(self.fail(problem));
            }
        }
        match self.read() {
            Ok(disk) if disk == self.base => {}
            Ok(_) => {
                self.conflict = true;
                self.message = "The config file changed on disk; nothing was saved.".into();
                self.error = true;
                return Err(Outcome::Stay);
            }
            Err(error) => return Err(self.fail(format!("Not saved: {error:#}"))),
        }
        let base = self.base.as_deref().unwrap_or("");
        let mut document: toml_edit::DocumentMut = match base.parse() {
            Ok(document) => document,
            Err(error) => return Err(self.fail(format!("Not saved: {error}"))),
        };
        for &i in edited {
            if let Err(error) = apply(&mut document, &self.fields[i], &self.inputs[i].text) {
                return Err(self.fail(format!("Not saved: {error:#}")));
            }
        }
        let text = document.to_string();
        let config = match Config::parse(&text) {
            Ok(config) => config,
            Err(error) => return Err(self.fail(format!("Not saved: {error:#}"))),
        };
        if let Err(error) = write(&self.path, &text) {
            return Err(self.fail(format!(
                "Not saved: writing {}: {error}",
                self.path.display()
            )));
        }
        self.base = Some(text);
        let restart = edited
            .iter()
            .filter(|&&i| self.fields[i].restart)
            .map(|&i| self.fields[i].label)
            .collect();
        Ok((Box::new(config), restart))
    }

    /// The colors as drafted, where valid, for the preview and swatches.
    fn theme(&self) -> Theme {
        let mut theme = Theme::default();
        for (name, color) in theme.named_mut() {
            let i = self
                .fields
                .iter()
                .position(|f| f.key.strip_prefix("colors.") == Some(name))
                .unwrap();
            if let Ok(c) = parse_color(self.inputs[i].text.trim()).or(parse_color(&self.saved[i])) {
                *color = c;
            }
        }
        theme.for_terminal(self.truecolor)
    }

    /// Draws the popup centred on the screen; returns its tabs and buttons.
    pub fn draw(&mut self, t: &Theme, frame: &mut Frame) -> Vec<buttons::Hit> {
        // Colors needs a tall list; the other pages and notices stay compact.
        let diagnostics = self.page == Page::Diagnostics;
        let height = if diagnostics {
            40
        } else if self.conflict || self.broken.is_some() {
            18
        } else if self.page == Page::Colors {
            34
        } else if self.page == Page::General {
            15
        } else {
            13
        };
        let area = crate::theme::centered(frame.area(), 76, height);
        frame.render_widget(Clear, area);
        frame.render_widget(t.block(TITLE, true).style(t.base().bg(t.overlay)), area);
        let inside = crate::ui::inner(area);
        let inside = Rect {
            x: inside.x + 1.min(inside.width),
            width: inside.width.saturating_sub(2),
            ..inside
        };
        self.rows.clear();
        let bar = if diagnostics {
            vec![
                Button::new("Refresh r", KeyCode::Char('r'), true).primary(),
                Button::new("Copy summary c", KeyCode::Char('c'), self.report.is_some()),
                Button::new("Close Esc", KeyCode::Esc, true),
            ]
        } else if self.conflict {
            vec![
                Button::new("Back Esc", KeyCode::Esc, true),
                Button::new("Keep my edits k", KeyCode::Char('k'), true).primary(),
                Button::new("Discard my edits d", KeyCode::Char('d'), true).danger(),
            ]
        } else if self.broken.is_some() {
            vec![
                Button::new("Diagnostics F4", KeyCode::F(4), true),
                Button::new("Cancel Esc", KeyCode::Esc, true),
                Button::control("Reload Ctrl-R", KeyCode::Char('r'), true),
            ]
        } else {
            let custom = self.inputs[self.selected].text != self.defaults[self.selected]
                && (self.fields[self.selected].kind != Kind::Channel
                    || matches!(self.remote, Remote::Ready));
            vec![
                Button::control("Default Ctrl-D", KeyCode::Char('d'), custom && !self.saving),
                Button::new("Cancel Esc", KeyCode::Esc, !self.saving),
                Button::control("Save Ctrl-S", KeyCode::Char('s'), !self.saving).primary(),
            ]
        };
        let (mut body, mut hits) = buttons::draw_compact(t, frame, inside, &bar);
        if body.is_empty() {
            return hits;
        }
        // The file path, clipped at the left so its name stays visible.
        let path = shown(&self.path);
        let room = usize::from(body.width).saturating_sub(8);
        let path = if path.width() > room {
            let tail: String = path
                .chars()
                .rev()
                .scan(1, |w, c| {
                    *w += c.to_string().width();
                    (*w <= room).then_some(c)
                })
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!("…{tail}")
        } else {
            path
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Config: ", Style::default().fg(t.muted)),
                Span::styled(path, Style::default().fg(t.text)),
            ])),
            Rect { height: 1, ..body },
        );
        shrink_top(&mut body, 1);
        // The message line sits above the buttons.
        if body.height > 1 && !self.message.is_empty() {
            frame.render_widget(
                Paragraph::new(crate::ui::clip(&self.message, usize::from(body.width)))
                    .style(Style::default().fg(if self.error { t.danger } else { t.muted })),
                Rect::new(body.x, body.bottom() - 1, body.width, 1),
            );
            body.height -= 1;
        }
        if let Some(error) = self.broken.as_ref().filter(|_| !diagnostics) {
            let kept = if self.keeping && !self.edited().is_empty() {
                "\nYour unsaved edits are kept and stay on top of the file after Reload; Cancel drops them."
            } else {
                ""
            };
            frame.render_widget(
                Paragraph::new(format!(
                    "\nThe config file cannot be used:\n{error}\n\nFix it outside saddle, then Reload.{kept}"
                ))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(t.text)),
                body,
            );
            return hits;
        }
        if self.conflict && !diagnostics {
            let edited: Vec<_> = self
                .edited()
                .into_iter()
                .map(|i| self.fields[i].label)
                .collect();
            frame.render_widget(
                Paragraph::new(format!(
                    "\nThe config file changed on disk after Settings read it. Saving now would overwrite that change, so nothing was saved.\n\nYour unsaved edits: {}\n\nKeep my edits: reload the file and keep these edits as a draft; Save again to write them.\nDiscard my edits: reload the file and drop the draft.\nBack: return to the draft without reloading.",
                    if edited.is_empty() { "none".into() } else { edited.join(", ") }
                ))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(t.text)),
                body,
            );
            return hits;
        }
        // Page tabs, outlined when there is room.
        let draw_tabs = if body.height >= 7 {
            buttons::draw_outlined_top
        } else {
            buttons::draw_compact_top
        };
        let tabs: Vec<_> = PAGES
            .iter()
            .map(|&(page, label, n)| {
                let button = Button::new(label, KeyCode::F(n), true);
                if page == self.page {
                    button.primary()
                } else {
                    button
                }
            })
            .collect();
        let (rest, tab_hits) = draw_tabs(t, frame, body, &tabs);
        for hit in &tab_hits {
            let current = PAGES
                .iter()
                .any(|&(page, _, n)| page == self.page && hit.key.code == KeyCode::F(n));
            frame.buffer_mut().set_style(
                hit.area,
                if current {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.muted)
                },
            );
        }
        hits.extend(tab_hits);
        body = rest;
        shrink_top(&mut body, 1);
        if diagnostics {
            self.draw_report(t, frame, body);
            return hits;
        }
        let preview = self.theme();
        // The Colors page keeps a small preview under its list when there is room.
        if self.page == Page::Colors && body.height >= 12 {
            let height = 5;
            draw_preview(
                t,
                &preview,
                frame,
                Rect::new(body.x, body.bottom() - height, body.width, height),
            );
            body.height -= height + 1;
        }
        self.draw_fields(t, &preview, frame, body);
        hits
    }

    fn draw_fields(&mut self, t: &Theme, preview: &Theme, frame: &mut Frame, area: Rect) {
        // Rows: group headings (Colors only), blank lines between groups, fields, and the note
        // under Task notifications.
        let mut rows: Vec<Option<Result<usize, &str>>> = Vec::new();
        let mut group = "";
        for i in self.page_fields() {
            if self.fields[i].group != group {
                if !rows.is_empty() {
                    rows.push(None);
                }
                group = self.fields[i].group;
                rows.push(Some(Err(group)));
            }
            rows.push(Some(Ok(i)));
            if self.fields[i].kind == Kind::Channel {
                rows.push(Some(Err(NOTIFICATIONS)));
            }
        }
        self.choices.clear();
        // One input width for the page, leaving room for units and restart notes.
        let page = self.page_fields();
        let unit = page
            .iter()
            .map(|&i| unit(self.fields[i].kind).width())
            .max()
            .unwrap_or(0);
        let swatch = if self.page == Page::Colors { 3 } else { 0 };
        let room = usize::from(area.width).saturating_sub(1 + LABEL + 1 + swatch + 2 + unit);
        let note = if page.iter().any(|&i| self.fields[i].restart) {
            [" Restart required", " Restart", ""]
                .into_iter()
                .find(|n| room >= n.width() + 12)
                .unwrap_or("")
        } else {
            ""
        };
        let width = room.saturating_sub(note.width()).min(40) as u16;
        let height = usize::from(area.height);
        let at = rows
            .iter()
            .position(|r| *r == Some(Ok(self.selected)))
            .unwrap_or(0);
        // Keep the selected field, and its heading when possible, in view.
        self.top = self
            .top
            .min(at.saturating_sub(usize::from(
                at > 0 && rows[at - 1].is_some_and(|r| r.is_err()),
            )))
            .max((at + 1).saturating_sub(height))
            .min(rows.len().saturating_sub(height));
        for (offset, row) in rows.iter().skip(self.top).take(height).enumerate() {
            let rect = Rect::new(area.x, area.y + offset as u16, area.width, 1);
            match row {
                None => {}
                Some(Err(NOTIFICATIONS)) => self.draw_channel_note(t, frame, rect),
                Some(Err(heading)) => frame.render_widget(
                    Paragraph::new(*heading)
                        .style(Style::default().fg(t.bright).add_modifier(Modifier::BOLD)),
                    rect,
                ),
                Some(Ok(i)) => self.draw_field(t, preview, frame, rect, *i, width, note),
            }
        }
        if rows.len() > height && height > 0 {
            let mut state =
                ratatui::widgets::ScrollbarState::new(rows.len() - height + 1).position(self.top);
            frame.render_stateful_widget(
                ratatui::widgets::Scrollbar::new(
                    ratatui::widgets::ScrollbarOrientation::VerticalRight,
                )
                .begin_symbol(None)
                .end_symbol(None)
                .style(Style::default().fg(t.dim)),
                Rect {
                    width: area.width + 1,
                    ..area
                },
                &mut state,
            );
        }
    }

    /// The Diagnostics groups; long values wrap under their value column.
    fn draw_report(&mut self, t: &Theme, frame: &mut Frame, area: Rect) {
        use crate::diagnostics::{Row, Tone};
        let Some(report) = &self.report else {
            frame.render_widget(
                Paragraph::new("Checking…").style(Style::default().fg(t.muted)),
                area,
            );
            return;
        };
        let room = area.width.saturating_sub(LABEL as u16 + 1).max(1);
        let mut lines = Vec::new();
        for row in report.rows() {
            match row {
                Row::Heading(heading) => {
                    if !lines.is_empty() {
                        lines.push(Line::default());
                    }
                    lines.push(Line::styled(
                        heading,
                        Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
                    ));
                }
                Row::Item(label, value, tone) => {
                    let style = Style::default().fg(match tone {
                        Tone::Good => t.text,
                        Tone::Bad => t.danger,
                        Tone::Unknown => t.muted,
                    });
                    let label = format!("  {} ", crate::ui::pad(label, LABEL - 2));
                    for (n, part) in crate::queue::wrap_text(&value, room)
                        .into_iter()
                        .enumerate()
                    {
                        let lead = if n == 0 {
                            label.clone()
                        } else {
                            " ".repeat(label.width())
                        };
                        let mut spans = vec![Span::styled(lead, Style::default().fg(t.muted))];
                        spans.extend(part.spans.into_iter().map(|span| span.style(style)));
                        lines.push(Line::from(spans));
                    }
                }
            }
        }
        let last = (lines.len() as u16).saturating_sub(area.height);
        self.report_top = self.report_top.min(last);
        frame.render_widget(Paragraph::new(lines).scroll((self.report_top, 0)), area);
    }

    /// What Task notifications controls, or why it cannot be chosen.
    fn draw_channel_note(&self, t: &Theme, frame: &mut Frame, row: Rect) {
        let (text, color) = match &self.remote {
            Remote::Loading => ("Reading Drover's setting…".to_owned(), t.muted),
            Remote::Ready => (
                "Drover tasks across projects, for this user".to_owned(),
                t.muted,
            ),
            Remote::Failed(error) => (format!("Unavailable: {error}"), t.danger),
        };
        let indent = (LABEL + 2) as u16;
        let area = Rect {
            x: row.x + indent.min(row.width),
            width: row.width.saturating_sub(indent),
            ..row
        };
        frame.render_widget(
            Paragraph::new(crate::ui::clip(&text, usize::from(area.width)))
                .style(Style::default().fg(color)),
            area,
        );
    }
    /// `[ System | In saddle ]`, the draft choice marked; dim until Drover answers.
    fn draw_choice(&mut self, t: &Theme, frame: &mut Frame, at: Rect, i: usize, selected: bool) {
        let ready = matches!(self.remote, Remote::Ready);
        let bracket = Style::default().fg(if selected { t.focus } else { t.border });
        let mut spans = vec![Span::styled("[ ", bracket)];
        let mut x = at.x + 2;
        for (n, option) in [SYSTEM, IN_SADDLE].into_iter().enumerate() {
            if n > 0 {
                spans.push(Span::styled(" | ", Style::default().fg(t.border)));
                x += 3;
            }
            let chosen = ready && self.inputs[i].text == option;
            spans.push(Span::styled(
                option,
                if chosen {
                    Style::default()
                        .fg(t.bright)
                        .bg(t.selected)
                        .add_modifier(Modifier::BOLD)
                } else if ready {
                    Style::default().fg(t.text)
                } else {
                    Style::default().fg(t.dim)
                },
            ));
            let area = Rect::new(x, at.y, option.width() as u16, 1).intersection(at);
            if ready {
                self.choices.push((area, option));
            }
            x += option.width() as u16;
        }
        spans.push(Span::styled(" ]", bracket));
        frame.render_widget(Paragraph::new(Line::from(spans)), at);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_field(
        &mut self,
        t: &Theme,
        preview: &Theme,
        frame: &mut Frame,
        row: Rect,
        i: usize,
        width: u16,
        note: &str,
    ) {
        let field = &self.fields[i];
        let selected = i == self.selected;
        let mut spans = vec![
            Span::styled(
                if self.inputs[i].text != self.saved[i] {
                    "•"
                } else {
                    " "
                },
                Style::default().fg(t.unread),
            ),
            Span::styled(
                crate::ui::pad(&crate::ui::clip(field.label, LABEL), LABEL),
                if selected {
                    Style::default().fg(t.focus).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.text)
                },
            ),
            Span::raw(" "),
        ];
        if field.kind == Kind::Color {
            let name = &field.key["colors.".len()..];
            spans.push(match parse_color(self.inputs[i].text.trim()) {
                Ok(_) => Span::styled(
                    "██",
                    Style::default().fg(color(preview, name).unwrap_or_default()),
                ),
                Err(_) => Span::styled("??", Style::default().fg(t.danger)),
            });
            spans.push(Span::raw(" "));
        }
        let unit = unit(field.kind);
        let note = if field.restart { note } else { "" };
        let used = spans.iter().map(|s| s.content.width()).sum::<usize>() as u16;
        if field.kind == Kind::Channel {
            frame.render_widget(Paragraph::new(Line::from(spans)), row);
            let at = Rect::new(row.x + used, row.y, row.width.saturating_sub(used), 1);
            self.draw_choice(t, frame, at, i, selected);
            self.rows.push((row, Rect::default(), i));
            return;
        }
        let bracket = Style::default().fg(if selected { t.focus } else { t.border });
        spans.push(Span::styled("[", bracket));
        frame.render_widget(Paragraph::new(Line::from(spans)), row);
        let input = Rect::new(row.x + used + 1, row.y, width, 1).intersection(row);
        let placeholder = if field.kind == Kind::Project {
            "Automatic"
        } else {
            ""
        };
        self.inputs[i].draw(frame, input, selected, placeholder, t);
        let after = Rect::new(
            input.right(),
            row.y,
            row.right().saturating_sub(input.right()),
            1,
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("]", bracket),
                Span::styled(unit, Style::default().fg(t.muted)),
                Span::styled(note.to_owned(), Style::default().fg(t.muted)),
            ])),
            after,
        );
        self.rows.push((row, input, i));
    }
}

fn channel(fields: &[Field]) -> usize {
    fields
        .iter()
        .position(|f| f.kind == Kind::Channel)
        .expect("Task notifications is a field")
}
fn choice(system: bool) -> &'static str {
    if system { SYSTEM } else { IN_SADDLE }
}

fn unit(kind: Kind) -> &'static str {
    if kind == Kind::Millis { " ms" } else { "" }
}

fn shrink_top(area: &mut Rect, rows: u16) {
    let rows = rows.min(area.height);
    area.y += rows;
    area.height -= rows;
}

/// A few sample lines in the drafted colors: the Agents panel, a selected agent, a dialog and
/// the shared status colors.
fn draw_preview(t: &Theme, p: &Theme, frame: &mut Frame, area: Rect) {
    let s = |text: &'static str, fg: Color| Span::styled(text, Style::default().fg(fg));
    let lines = [
        (
            p.agents_bg,
            vec![
                Span::styled(
                    " Agents · 3 ",
                    Style::default()
                        .fg(p.agents_text)
                        .add_modifier(Modifier::BOLD),
                ),
                s("dev/ ", p.agents_accent),
                s("◓ working ", p.agents_blue),
                s("○ idle ", p.agents_green),
                s("? waiting ", p.agents_yellow),
                s("! error ", p.agents_red),
                s("↑0", p.agents_faint),
            ],
        ),
        (
            p.agent_selected,
            vec![
                s(" ┃ ", p.agents_accent),
                s("main ", p.agents_text),
                s("✳ claude ", p.claude),
                s(">_ codex ", p.codex),
                s("pi ", p.pi),
                s("omp ", p.omp),
                s("⎇ main ", p.agents_branch),
                s("…/src ", p.agents_dim),
                s("ATT 0", p.agents_dimmer),
            ],
        ),
        (
            p.overlay,
            vec![
                Span::styled(" Input ▸ ", Style::default().fg(p.input_text).bg(p.focus)),
                s(" Text ", p.text),
                s("Bright ", p.bright),
                s("Muted ", p.muted),
                s("Dim ", p.dim),
                s("◉ ", p.connected),
                s("● ", p.working),
                s("• ", p.unread),
                s("‹Save› ", p.focus),
                s("‹Stop›", p.danger),
            ],
        ),
        (
            p.bg,
            vec![
                s(" Running ", p.agent_working),
                s("Done ", p.agent_idle),
                s("Awaiting ", p.agent_blocked),
                s("Dropped ", p.agent_stalled),
                s("Failed ", p.agent_error),
                s("Pending ", p.agent_starting),
                s("`code` ", p.reply_code),
                s("# Heading", p.reply_heading),
            ],
        ),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Preview",
                Style::default().fg(t.bright).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  unsaved colors; the rest of saddle changes on Save",
                Style::default().fg(t.muted),
            ),
        ])),
        Rect { height: 1, ..area },
    );
    for (offset, (background, spans)) in lines.into_iter().enumerate() {
        let y = area.y + 1 + offset as u16;
        if y >= area.bottom() {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(crate::ui::clip_spans(
                spans,
                usize::from(area.width),
            )))
            .style(Style::default().bg(background)),
            Rect::new(area.x, y, area.width, 1),
        );
    }
}

/// Sets one setting in the document, keeping the key's place and the value's comment.
fn apply(document: &mut toml_edit::DocumentMut, field: &Field, text: &str) -> anyhow::Result<()> {
    use toml_edit::{Item, Value};
    let value = match field.kind {
        Kind::Columns | Kind::Millis => Some(Value::from(text.trim().parse::<i64>()?)),
        Kind::Color => Some(Value::from(text.trim())),
        Kind::Project if text.trim().is_empty() => None,
        Kind::Project | Kind::Command => Some(Value::from(text)),
        Kind::Channel => return Ok(()),
    };
    let (table, key) = match field.key.split_once('.') {
        Some((table, key)) => {
            let item = document
                .as_table_mut()
                .entry(table)
                .or_insert(toml_edit::table());
            let table = item
                .as_table_like_mut()
                .ok_or_else(|| anyhow::anyhow!("[{table}] is not a table"))?;
            (table, key)
        }
        None => (
            document.as_table_mut() as &mut dyn toml_edit::TableLike,
            field.key.as_str(),
        ),
    };
    match (value, table.get_mut(key)) {
        (None, _) => {
            table.remove(key);
        }
        (Some(mut value), Some(item)) => {
            if let Some(old) = item.as_value() {
                *value.decor_mut() = old.decor().clone();
            }
            *item = Item::Value(value);
        }
        (Some(value), None) => {
            table.insert(key, Item::Value(value));
        }
    }
    Ok(())
}

/// Replaces the file (through a symlink, keeping its permissions), creating missing folders. A
/// link whose target cannot be resolved is left alone rather than replaced by a file.
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    let target = match fs::canonicalize(path) {
        Ok(target) => target,
        Err(_) if path.is_symlink() => {
            return Err(std::io::Error::other(
                "it is a symbolic link whose target cannot be resolved; the link is kept",
            ));
        }
        Err(_) => path.to_path_buf(),
    };
    let dir = match target.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    fs::create_dir_all(dir)?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(text.as_bytes())?;
    if let Ok(metadata) = fs::metadata(&target) {
        file.as_file().set_permissions(metadata.permissions())?;
    }
    file.persist(&target).map_err(|e| e.error)?;
    Ok(())
}

/// The path with the home folder shown as `~`.
fn shown(path: &Path) -> String {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .and_then(|home| path.strip_prefix(home).ok())
        .map_or_else(
            || path.display().to_string(),
            |rest| format!("~/{}", rest.display()),
        )
}
