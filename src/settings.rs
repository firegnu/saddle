//! Settings: view, edit and save the existing config file from inside saddle. Edits stay a
//! draft until Save; Save writes only the edited keys, keeping the rest of the file as it is.
//! Colors follow the chosen theme unless overridden; choosing another theme drops the overrides.
//! Its Diagnostics page only reads. The telemetry recording switch on General is read from and
//! saved to the telemetry store, never to the file.
use crate::{
    buttons::{self, Button},
    config::Config,
    launch::edit::Input,
    mascot::{Display, Pet},
    telemetry::{SettingInput, Store},
    theme::{Preset, Theme, color_name, parse_color},
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
/// Normal Settings pages share the existing Plugins frame; forms and confirmations stay compact.
pub(crate) fn page_area(screen: Rect) -> Rect {
    crate::theme::centered(screen, 108, 34)
}
/// The `value` key of the telemetry recording switch; it lives in the telemetry store, not the file.
pub const RECORDING: &str = "telemetry:recording";
const LABEL: usize = 18;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Colors,
    Advanced,
    Diagnostics,
    Plugins,
    Updates,
}
pub(crate) const PAGES: [(Page, &str, u8); 6] = [
    (Page::General, "General", 1),
    (Page::Colors, "Colors", 2),
    (Page::Advanced, "Advanced", 3),
    (Page::Diagnostics, "Diagnostics", 4),
    (Page::Plugins, "Plugins", 5),
    (Page::Updates, "Updates", 6),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Columns,
    Millis,
    Bool,
    Command,
    Color,
    Theme,
    Pet,
    Display,
}
struct Field {
    /// The config key, dotted below a table (`colors.bg`).
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
        field("mascot_enabled", "Mascot", Page::General, Kind::Bool, false),
        field("mascot", "Pet", Page::General, Kind::Pet, false),
        field(
            "mascot_display",
            "Display",
            Page::General,
            Kind::Display,
            false,
        ),
        field(
            RECORDING,
            "Telemetry recording",
            Page::General,
            Kind::Bool,
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
    list.push(field("theme", "Theme", Page::Colors, Kind::Theme, false));
    list.extend(colors);
    list.extend([field(
        "corral",
        "corral command",
        Page::Advanced,
        Kind::Command,
        true,
    )]);
    list
}

/// A setting as the form shows it; an omitted initial project is empty (Automatic).
fn value(config: &Config, field: &Field) -> String {
    match field.key.as_str() {
        "left_width" => config.left_width.to_string(),
        "refresh_ms" => config.refresh_ms.to_string(),
        "mascot_enabled" => config.mascot_enabled.to_string(),
        "mascot" => config.mascot.name().into(),
        "mascot_display" => config.mascot_display.name().into(),
        "corral" => config.corral.clone(),
        "theme" => config.theme.name().into(),
        // Off until the store says otherwise, as on a first install.
        RECORDING => "false".into(),
        key => color(&config.colors, &key["colors.".len()..]).map_or_else(String::new, color_name),
    }
}
/// Whether a color follows the theme rather than its own `[colors]` key.
fn follows(config: &Config, field: &Field) -> bool {
    field.kind == Kind::Color && !config.overrides.contains_key(&field.key["colors.".len()..])
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
    Plugins,
    Stay,
    Cancel,
    /// Written: the saved configuration and the labels of saved settings that need a restart.
    Saved(Box<Config>, Vec<&'static str>),
    /// Diagnostics opened or Refresh pressed: check again and answer with `diagnose`.
    Diagnose,
    /// Copy this diagnostics summary; answer with `copied`.
    Copy(String),
    /// Only the telemetry recording switch was saved; `message` tells its state.
    Recorded,
    /// The config was written but the switch was not: apply it and keep Settings open.
    Applied(Box<Config>, Vec<&'static str>),
    /// Updates opened or Refresh pressed: check again now.
    CheckUpdates,
    /// Upgrade all pressed while offered.
    Upgrade,
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
    /// Colors in the draft that follow the theme; their inputs show the theme's value.
    follow: Vec<bool>,
    /// Colors in `base` that follow the theme.
    saved_follow: Vec<bool>,
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
    /// What Diagnostics shows; None until the first check is asked for.
    report: Option<crate::diagnostics::Report>,
    /// The first Diagnostics line shown.
    report_top: u16,
    updates: crate::updates::Page,
    updates_top: u16,
    /// Where the recording switch is stored; Err says why there is none.
    telemetry: Result<Store, String>,
    /// The switch's generation as last read; Err while its state is unknown, saying why.
    recording: Result<i64, String>,
    /// The conflict is about the recording switch rather than the file.
    recording_conflict: bool,
}

impl Settings {
    pub fn open(path: PathBuf, truecolor: bool) -> Self {
        let fields = fields();
        let defaults: Vec<_> = fields
            .iter()
            .map(|f| value(&Config::default(), f))
            .collect();
        let inputs = defaults.iter().map(|v| Input::new(v.clone())).collect();
        let saved = defaults.clone();
        let follow: Vec<_> = fields.iter().map(|f| f.kind == Kind::Color).collect();
        let mut settings = Self {
            path,
            truecolor,
            inputs,
            saved,
            saved_follow: follow.clone(),
            follow,
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
            report: None,
            report_top: 0,
            updates: Default::default(),
            updates_top: 0,
            telemetry: Err("no telemetry store".into()),
            recording: Err("no telemetry store".into()),
            recording_conflict: false,
        };
        settings.reload(false);
        settings
    }
    /// Shows and saves the recording switch of this store; reading never initializes it.
    pub fn with_telemetry(mut self, store: crate::telemetry::Result<Store>) -> Self {
        self.telemetry = store.map_err(|e| e.to_string());
        self.refresh_recording();
        self
    }
    /// Rereads the recording switch unless it has an unsaved edit, which keeps the state it was
    /// made against so Save can tell whether the store changed meanwhile.
    pub fn refresh_recording(&mut self) {
        let i = self.recording_index();
        if self.inputs[i].text == self.saved[i] {
            self.load_recording();
            self.inputs[i] = Input::new(self.saved[i].clone());
        }
    }
    fn recording_index(&self) -> usize {
        self.fields.iter().position(|f| f.key == RECORDING).unwrap()
    }
    /// The stored switch and its generation.
    fn read_recording(&self) -> Result<(bool, i64), String> {
        let store = self.telemetry.as_ref().map_err(Clone::clone)?;
        let read = store.settings().map_err(|e| e.to_string())?;
        read["enabled"]
            .as_bool()
            .zip(read["generation"].as_i64())
            .ok_or_else(|| "unreadable telemetry settings".into())
    }
    /// Takes the stored switch as its saved value; an unknown state is empty.
    fn load_recording(&mut self) {
        let i = self.recording_index();
        let read = self.read_recording();
        self.saved[i] = read
            .as_ref()
            .map_or_else(|_| String::new(), |(on, _)| on.to_string());
        self.recording = read.map(|(_, generation)| generation);
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
    /// What the Updates page shows now.
    pub fn set_updates(&mut self, page: crate::updates::Page) {
        self.updates = page;
    }
    /// Shows the Updates page; the caller checks again.
    pub fn open_updates(&mut self) {
        self.page = Page::Updates;
        self.message.clear();
    }
    pub fn updates_help(&self) -> Option<&'static str> {
        (self.page == Page::Updates).then_some(if self.updates.upgrade {
            " Esc Close  r Refresh  u Upgrade all  ↑↓ Scroll  F1-F6 Page"
        } else {
            " Esc Close  r Refresh  ↑↓ Scroll  F1-F6 Page"
        })
    }
    pub fn help(&self) -> &'static str {
        if self.conflict {
            " Esc Back  k Keep my edits  d Discard my edits"
        } else if let Some(help) = self.updates_help() {
            help
        } else if self.page == Page::Diagnostics {
            if self.report.is_some() {
                " Esc Close  r Refresh  c Copy summary  ↑↓ Scroll  F1-F6 Page"
            } else {
                " Esc Close  r Refresh  ↑↓ Scroll  F1-F6 Page"
            }
        } else if self.broken.is_some() {
            " Esc Cancel  Ctrl-R Reload  F4 Diagnostics  F5 Plugins  F6 Updates"
        } else {
            " Esc Cancel  Ctrl-S Save  F1-F6 Page  Tab/↑↓ Field  Ctrl-D Default  Ctrl-U Clear"
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
    fn edited(&self) -> Vec<usize> {
        (0..self.fields.len())
            .filter(|&i| self.changed(i))
            .collect()
    }
    /// Whether Save writes this setting: its value, or whether the color is overridden. An
    /// override equal to the theme's value is still an override.
    fn changed(&self, i: usize) -> bool {
        self.follow[i] != self.saved_follow[i]
            || (!self.follow[i] && self.inputs[i].text != self.saved[i])
    }
    fn theme_index(&self) -> usize {
        self.fields
            .iter()
            .position(|f| f.kind == Kind::Theme)
            .unwrap()
    }
    /// The drafted theme.
    fn preset(&self) -> Preset {
        Preset::parse(&self.inputs[self.theme_index()].text).unwrap_or_default()
    }
    /// Colors that follow the theme show the drafted theme's value.
    fn sync(&mut self) {
        let theme = self.preset().theme();
        for i in 0..self.fields.len() {
            if self.follow[i] {
                let shown = color(&theme, &self.fields[i].key["colors.".len()..])
                    .map_or_else(String::new, color_name);
                if self.inputs[i].text != shown {
                    self.inputs[i] = Input::new(shown);
                }
            }
        }
    }
    /// Another theme loads all of its colors into the draft and drops the color overrides;
    /// the current one changes nothing.
    fn choose(&mut self, preset: Preset) {
        if preset == self.preset() {
            return;
        }
        let i = self.theme_index();
        self.inputs[i] = Input::new(preset.name().into());
        for (follow, field) in self.follow.iter_mut().zip(&self.fields) {
            *follow = field.kind == Kind::Color;
        }
        self.sync();
        self.message = format!(
            "Colors replaced by the {} theme; earlier color overrides are cleared from this draft.",
            preset.label()
        );
        self.error = false;
    }
    fn cycle(&mut self, delta: isize) {
        let text = &self.inputs[self.selected].text;
        let next = match self.fields[self.selected].kind {
            Kind::Pet => Some(step(&Pet::ALL, Pet::parse(text).unwrap_or_default(), delta).name()),
            Kind::Display => Some(
                step(
                    &Display::ALL,
                    Display::parse(text).unwrap_or_default(),
                    delta,
                )
                .name(),
            ),
            _ => None,
        };
        if let Some(next) = next {
            self.inputs[self.selected] = Input::new(next.into());
            return;
        }
        let all = Preset::ALL;
        let at = all.iter().position(|&p| p == self.preset()).unwrap_or(0);
        self.choose(all[(at as isize + delta).rem_euclid(all.len() as isize) as usize]);
    }
    /// Edits the selected input; a color typed into stops following the theme.
    fn edit(&mut self, change: impl FnOnce(&mut Input)) {
        let i = self.selected;
        let before = self.inputs[i].text.clone();
        change(&mut self.inputs[i]);
        if self.inputs[i].text != before {
            self.follow[i] = false;
        }
    }
    /// Default: a color follows the theme again, the theme goes back to Dune.
    fn reset(&mut self) {
        let i = self.selected;
        match self.fields[i].kind {
            Kind::Theme => self.choose(Preset::Dune),
            Kind::Color => {
                self.follow[i] = true;
                self.sync();
            }
            _ => self.inputs[i] = Input::new(self.defaults[i].clone()),
        }
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
        self.saved = self.fields.iter().map(|f| value(&config, f)).collect();
        self.saved_follow = self.fields.iter().map(|f| follows(&config, f)).collect();
        self.load_recording();
        for (i, input) in self.inputs.iter_mut().enumerate() {
            if !edited.contains(&i) {
                *input = Input::new(self.saved[i].clone());
                self.follow[i] = self.saved_follow[i];
            }
        }
        self.sync();
        self.base = text;
        self.broken = None;
        self.conflict = false;
        self.recording_conflict = false;
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

    /// Navigate exactly as the existing page tab does, including special-page outcomes.
    pub fn open_page(&mut self, page: Page) -> Outcome {
        let (_, _, key) = PAGES.iter().find(|&&(p, _, _)| p == page).unwrap();
        self.key(KeyEvent::new(KeyCode::F(*key), KeyModifiers::NONE))
    }

    pub fn key(&mut self, key: KeyEvent) -> Outcome {
        if key.code == KeyCode::F(5) {
            return Outcome::Plugins;
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
                    self.message = if self.recording_conflict {
                        "Not saved: telemetry recording changed outside Settings.".into()
                    } else {
                        "Not saved: the file changed on disk.".into()
                    };
                    self.recording_conflict = false;
                }
                _ => {}
            }
            return Outcome::Stay;
        }
        if key.code == KeyCode::Esc {
            return Outcome::Cancel;
        }
        if key.code == KeyCode::F(6) {
            self.open_updates();
            return Outcome::CheckUpdates;
        }
        if self.page == Page::Updates {
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
            {
                return Outcome::Stay;
            }
            match key.code {
                KeyCode::F(n @ 1..=3) => self.show(PAGES[usize::from(n - 1)].0),
                KeyCode::F(4) => return self.diagnostics_key(key),
                KeyCode::Char('r') => return Outcome::CheckUpdates,
                KeyCode::Char('u') if self.updates.upgrade => return Outcome::Upgrade,
                KeyCode::Up => self.updates_top = self.updates_top.saturating_sub(1),
                KeyCode::Down => self.updates_top = self.updates_top.saturating_add(1),
                KeyCode::PageUp => self.updates_top = self.updates_top.saturating_sub(10),
                KeyCode::PageDown => self.updates_top = self.updates_top.saturating_add(10),
                _ => {}
            }
            return Outcome::Stay;
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
        if ctrl {
            match key.code {
                KeyCode::Char('s') => return self.save(),
                KeyCode::Char('d') => self.reset(),
                KeyCode::Char('u')
                    if !matches!(
                        self.fields[self.selected].kind,
                        Kind::Bool | Kind::Theme | Kind::Pet | Kind::Display
                    ) =>
                {
                    self.edit(Input::clear)
                }
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
            KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Left | KeyCode::Right
                if self.fields[self.selected].kind == Kind::Bool =>
            {
                self.toggle()
            }
            KeyCode::Left if choice(self.fields[self.selected].kind) => self.cycle(-1),
            KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Right
                if choice(self.fields[self.selected].kind) =>
            {
                self.cycle(1)
            }
            code if !matches!(
                self.fields[self.selected].kind,
                Kind::Bool | Kind::Theme | Kind::Pet | Kind::Display
            ) =>
            {
                self.edit(|input| input.key(code, false))
            }
            _ => {}
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
    fn toggle(&mut self) {
        self.inputs[self.selected] =
            Input::new((self.inputs[self.selected].text != "true").to_string());
    }
    pub fn paste(&mut self, text: &str) {
        if !matches!(self.page, Page::Diagnostics | Page::Updates)
            && !self.conflict
            && self.broken.is_none()
            && !matches!(
                self.fields[self.selected].kind,
                Kind::Bool | Kind::Theme | Kind::Pet | Kind::Display
            )
        {
            self.edit(|input| input.insert(text, false));
        }
    }
    pub fn click(&mut self, point: Position) {
        if self.conflict
            || self.broken.is_some()
            || matches!(self.page, Page::Diagnostics | Page::Updates)
        {
            return;
        }
        if let Some(&(_, input, i)) = self.rows.iter().find(|(row, _, _)| row.contains(point)) {
            self.selected = i;
            if self.fields[i].kind == Kind::Bool {
                self.toggle();
            } else if choice(self.fields[i].kind) {
                // ‹ chooses the previous one; elsewhere on the row, the next.
                self.cycle(if point.x == input.x { -1 } else { 1 });
            } else if input.contains(point) {
                self.inputs[i].click(point);
            }
        }
    }
    pub fn scroll(&mut self, down: bool) {
        if self.page == Page::Updates {
            self.updates_top = if down {
                self.updates_top.saturating_add(1)
            } else {
                self.updates_top.saturating_sub(1)
            };
        } else if self.page == Page::Diagnostics {
            self.report_top = if down {
                self.report_top.saturating_add(1)
            } else {
                self.report_top.saturating_sub(1)
            };
        } else if !self.conflict && self.broken.is_none() {
            self.step(if down { 1 } else { -1 });
        }
    }

    /// Save only edited settings, preserving the draft on validation or disk conflicts. The file
    /// and the recording switch are two separate commits: each part reports what it actually did,
    /// and whatever was not saved stays a draft.
    fn save(&mut self) -> Outcome {
        let edited = self.edited();
        if edited.is_empty() {
            return Outcome::Cancel;
        }
        let r = self.recording_index();
        let config: Vec<usize> = edited.iter().copied().filter(|&i| i != r).collect();
        // Everything that can refuse the whole save is checked before either part is written.
        let prepared = if config.is_empty() {
            None
        } else {
            match self.prepare_config(&config) {
                Ok(prepared) => Some(prepared),
                Err(outcome) => return outcome,
            }
        };
        let recording = if edited.contains(&r) {
            match self.save_recording() {
                Some(result) => Some(result),
                None => return Outcome::Stay,
            }
        } else {
            None
        };
        let Some((text, parsed)) = prepared else {
            return match recording {
                Some(Ok(note)) => {
                    (self.message, self.error) = (note, false);
                    Outcome::Recorded
                }
                Some(Err(problem)) => self.fail(problem),
                None => Outcome::Cancel,
            };
        };
        if let Err(error) = write(&self.path, &text) {
            let done = match &recording {
                Some(Ok(note) | Err(note)) => format!("{note} Config not saved"),
                None => "Not saved".into(),
            };
            return self.fail(format!("{done}: writing {}: {error}", self.path.display()));
        }
        self.base = Some(text);
        let restart: Vec<_> = config
            .iter()
            .filter(|&&i| self.fields[i].restart)
            .map(|&i| self.fields[i].label)
            .collect();
        match recording {
            Some(Err(problem)) => {
                // Settings stays open on the switch's draft; the written values are saved now.
                for &i in &config {
                    self.saved[i] = value(&parsed, &self.fields[i]);
                    self.saved_follow[i] = follows(&parsed, &self.fields[i]);
                    self.inputs[i] = Input::new(self.saved[i].clone());
                    self.follow[i] = self.saved_follow[i];
                }
                // Colors following a newly saved theme now match it too.
                for (i, field) in self.fields.iter().enumerate() {
                    if self.follow[i] && self.saved_follow[i] {
                        self.saved[i] = value(&parsed, field);
                    }
                }
                // Both parts' results lead; the restart note follows.
                let restart_note = if restart.is_empty() {
                    String::new()
                } else {
                    format!(" Restart saddle to apply: {}.", restart.join(", "))
                };
                self.fail(format!("Config saved. {problem}{restart_note}"));
                Outcome::Applied(Box::new(parsed), restart)
            }
            Some(Ok(note)) => {
                (self.message, self.error) = (note, false);
                Outcome::Saved(Box::new(parsed), restart)
            }
            None => {
                self.message.clear();
                Outcome::Saved(Box::new(parsed), restart)
            }
        }
    }
    /// Saves the switch against the state it was edited from. None: the store changed or could
    /// not be read when the switch was shown, and the conflict notice is up; nothing was written.
    fn save_recording(&mut self) -> Option<Result<String, String>> {
        let i = self.recording_index();
        let enabled = self.inputs[i].text == "true";
        let not_saved = |e: &dyn std::fmt::Display| format!("Telemetry recording not saved: {e}.");
        let store = match &self.telemetry {
            Ok(store) => store,
            Err(e) => return Some(Err(not_saved(e))),
        };
        let result = match &self.recording {
            Ok(generation) => store
                .set_recording_if(
                    *generation,
                    SettingInput {
                        schema_version: 1,
                        enabled,
                        actor: "saddle settings".into(),
                    },
                )
                .map_err(|e| (e.status == "conflict", e.to_string())),
            // The draft was made without seeing the state: show the state before writing over it.
            Err(_) => match self.read_recording() {
                Ok(_) => Err((true, String::new())),
                Err(e) => Err((false, e)),
            },
        };
        match result {
            Ok(_) => {
                // The compare-and-set committed exactly this change.
                self.saved[i] = enabled.to_string();
                self.recording = self.recording.as_ref().map(|g| g + 1).map_err(Clone::clone);
                Some(Ok(format!(
                    "Telemetry recording saved: {}.",
                    if enabled { "Enabled" } else { "Disabled" }
                )))
            }
            Err((true, _)) => {
                self.conflict = true;
                self.recording_conflict = true;
                self.error = true;
                self.message =
                    "Telemetry recording changed outside Settings; nothing was saved.".into();
                None
            }
            Err((false, e)) => Some(Err(not_saved(&e))),
        }
    }
    /// Validates the edited config fields against the file as read and builds the new text.
    fn prepare_config(&mut self, edited: &[usize]) -> Result<(String, Config), Outcome> {
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
                Kind::Color if self.follow[i] => None,
                Kind::Color => parse_color(text)
                    .err()
                    .map(|e| format!("{}: {e}", field.label)),
                Kind::Theme => Preset::parse(text).err(),
                Kind::Pet => Pet::parse(text).err(),
                Kind::Display => Display::parse(text).err(),
                Kind::Bool | Kind::Command => None,
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
            // A color following the theme has no key.
            let text = (!self.follow[i]).then_some(self.inputs[i].text.as_str());
            if let Err(error) = apply(&mut document, &self.fields[i], text) {
                return Err(self.fail(format!("Not saved: {error:#}")));
            }
        }
        let text = document.to_string();
        match Config::parse(&text) {
            Ok(config) => Ok((text, config)),
            Err(error) => Err(self.fail(format!("Not saved: {error:#}"))),
        }
    }

    /// The colors as drafted, where valid, over the drafted theme, for the preview and swatches.
    fn theme(&self) -> Theme {
        let mut theme = self.preset().theme();
        for (name, color) in theme.named_mut() {
            let i = self
                .fields
                .iter()
                .position(|f| f.key.strip_prefix("colors.") == Some(name))
                .unwrap();
            if let Ok(c) = parse_color(self.inputs[i].text.trim()) {
                *color = c;
            }
        }
        theme.for_terminal(self.truecolor)
    }

    /// Draws the popup centred on the screen; returns its tabs and buttons.
    pub fn draw(&mut self, t: &Theme, frame: &mut Frame) -> Vec<buttons::Hit> {
        let diagnostics = self.page == Page::Diagnostics;
        let updates = self.page == Page::Updates;
        let area = if !diagnostics && !updates && (self.conflict || self.broken.is_some()) {
            crate::theme::centered(frame.area(), 76, 18)
        } else {
            page_area(frame.area())
        };
        frame.render_widget(Clear, area);
        frame.render_widget(t.block(TITLE, true).style(t.base().bg(t.overlay)), area);
        let inside = crate::ui::inner(area);
        let inside = Rect {
            x: inside.x + 1.min(inside.width),
            width: inside.width.saturating_sub(2),
            ..inside
        };
        self.rows.clear();
        let bar = if updates {
            vec![
                Button::new("Refresh r", KeyCode::Char('r'), true),
                Button::new("Upgrade all u", KeyCode::Char('u'), self.updates.upgrade).primary(),
                Button::new("Close Esc", KeyCode::Esc, true),
            ]
        } else if diagnostics {
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
                Button::new("Plugins F5", KeyCode::F(5), true),
                Button::new("Updates F6", KeyCode::F(6), true),
                Button::new("Cancel Esc", KeyCode::Esc, true),
                Button::control("Reload Ctrl-R", KeyCode::Char('r'), true),
            ]
        } else {
            let i = self.selected;
            let custom = match self.fields[i].kind {
                Kind::Color => !self.follow[i],
                Kind::Theme => self.preset() != Preset::Dune,
                _ => self.inputs[i].text != self.defaults[i],
            };
            vec![
                Button::control("Default Ctrl-D", KeyCode::Char('d'), custom),
                Button::new("Cancel Esc", KeyCode::Esc, true),
                Button::control("Save Ctrl-S", KeyCode::Char('s'), true).primary(),
            ]
        };
        let (mut body, mut hits) = buttons::draw_compact(t, frame, inside, &bar);
        if body.is_empty() {
            return hits;
        }
        let show_tabs = diagnostics || updates || (!self.conflict && self.broken.is_none());
        let (rest, tabs) = self.draw_header(t, frame, body, show_tabs.then_some(self.page));
        body = rest;
        hits.extend(tabs);
        if updates {
            use crate::updates::{Row, Tone};
            let mut rows = Vec::new();
            for (n, row) in self.updates.rows.iter().enumerate() {
                match row {
                    // The first heading states the page's status; a blank row sets it apart.
                    Row::Heading(text) if n == 0 => {
                        rows.push(ReportRow::Heading(
                            text,
                            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
                        ));
                        rows.push(ReportRow::Blank);
                    }
                    Row::Heading(text) => rows.push(ReportRow::Heading(text, heading(t))),
                    Row::Item(label, value, tone) => rows.push(ReportRow::Item(
                        label,
                        value,
                        match tone {
                            Tone::Good => t.text,
                            Tone::Action => t.unread,
                            Tone::Bad => t.danger,
                            Tone::Unknown => t.muted,
                        },
                    )),
                }
            }
            let lines = report_lines(t, &rows, body.width);
            self.updates_top = self
                .updates_top
                .min(lines.len().saturating_sub(usize::from(body.height)) as u16);
            frame.render_widget(Paragraph::new(lines).scroll((self.updates_top, 0)), body);
            return hits;
        }
        // The message sits above the buttons; a long one wraps onto up to three rows.
        if body.height > 1 && !self.message.is_empty() {
            let mut lines = word_wrap(&self.message, usize::from(body.width));
            lines.truncate(usize::from((body.height - 1).min(3)));
            let height = lines.len() as u16;
            frame.render_widget(
                Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
                    .style(Style::default().fg(if self.error { t.danger } else { t.muted })),
                Rect::new(body.x, body.bottom() - height, body.width, height),
            );
            body.height -= height;
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
            let (changed, what) = if self.recording_conflict {
                (
                    "Telemetry recording changed outside Settings, or could not be read when Settings showed it. Saving now could overwrite a newer setting",
                    "the file and the recording state",
                )
            } else {
                (
                    "The config file changed on disk after Settings read it. Saving now would overwrite that change",
                    "the file",
                )
            };
            frame.render_widget(
                Paragraph::new(format!(
                    "\n{changed}, so nothing was saved.\n\nYour unsaved edits: {}\n\nKeep my edits: reload {what} and keep these edits as a draft; Save again to write them.\nDiscard my edits: reload {what} and drop the draft.\nBack: return to the draft without reloading.",
                    if edited.is_empty() { "none".into() } else { edited.join(", ") }
                ))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(t.text)),
                body,
            );
            return hits;
        }
        if diagnostics {
            self.draw_report(t, frame, body);
            return hits;
        }
        let preview = self.theme();
        // The Colors page keeps a small preview under its list when there is room.
        if self.page == Page::Colors && body.height >= 12 {
            let height = 4;
            draw_preview(
                t,
                &preview,
                frame,
                Rect::new(body.x, body.bottom() - height, body.width, height),
            );
            body.height -= height + 1;
        }
        // Only the active field needs an operation hint; units stay beside their values.
        let hint = match self.fields[self.selected].kind {
            Kind::Bool => "Space/Enter toggle",
            Kind::Theme | Kind::Pet | Kind::Display => "←/→ choose",
            _ => "",
        };
        if !hint.is_empty() && body.height > 2 {
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(t.muted)),
                Rect::new(body.x, body.bottom() - 1, body.width, 1),
            );
            body.height -= 1;
        }
        // General explains the recording switch under its fields when there is room.
        if self.page == Page::General {
            let mut notes = vec![
                Line::styled(
                    "On: only work explicitly chosen for recording may be recorded.",
                    Style::default().fg(t.muted),
                ),
                Line::styled(
                    "Off: no new recording; tasks and agents still run; history is kept.",
                    Style::default().fg(t.muted),
                ),
            ];
            if let Err(reason) = &self.recording {
                notes.push(Line::styled(
                    format!("Recording state unknown: {reason}"),
                    Style::default().fg(t.danger),
                ));
            }
            let (height, fields) = (notes.len() as u16, self.page_fields().len() as u16);
            if body.height >= fields + 1 + height {
                let lines = notes
                    .into_iter()
                    .map(|line| crate::ui::clip_spans(line.spans, usize::from(body.width)));
                frame.render_widget(
                    Paragraph::new(lines.map(Line::from).collect::<Vec<_>>()),
                    Rect::new(body.x, body.y + fields + 1, body.width, height),
                );
                body.height = fields;
            }
        }
        self.draw_fields(t, &preview, frame, body);
        hits
    }

    /// Shared Settings path and page navigation; no editable fields or input cursor.
    pub(crate) fn draw_header(
        &self,
        t: &Theme,
        frame: &mut Frame,
        mut body: Rect,
        active: Option<Page>,
    ) -> (Rect, Vec<buttons::Hit>) {
        if body.is_empty() {
            return (body, Vec::new());
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
        let Some(active) = active else {
            return (body, Vec::new());
        };
        // Compact tabs wrap when the dialog cannot fit all pages.
        let labels: Vec<_> = PAGES
            .iter()
            .map(|&(_, label, n)| format!("{label} F{n}"))
            .collect();
        let tabs: Vec<_> = PAGES
            .iter()
            .zip(&labels)
            .map(|(&(page, _, n), label)| {
                let button = Button::new(label, KeyCode::F(n), true);
                if page == active {
                    button.primary()
                } else {
                    button
                }
            })
            .collect();
        let (rest, tab_hits) = buttons::draw_compact_top(t, frame, body, &tabs);
        for hit in &tab_hits {
            let current = PAGES
                .iter()
                .any(|&(page, _, n)| page == active && hit.key.code == KeyCode::F(n));
            frame.buffer_mut().set_style(
                hit.area,
                if current {
                    Style::default()
                        .fg(t.text)
                        .bg(t.selected)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.muted)
                },
            );
        }
        body = rest;
        shrink_top(&mut body, 1);
        (body, tab_hits)
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
        }
        // One input width for the page, leaving room for units and restart notes.
        let page = self.page_fields();
        let unit = page
            .iter()
            .map(|&i| match self.fields[i].kind {
                Kind::Color => CUSTOM.width(),
                kind => unit(kind).width(),
            })
            .max()
            .unwrap_or(0);
        let swatch = if self.page == Page::Colors { 3 } else { 0 };
        // Labels keep one column; a longer one widens it for its page only.
        let label = page
            .iter()
            .map(|&i| self.fields[i].label.width())
            .fold(LABEL, usize::max);
        let room = usize::from(area.width).saturating_sub(1 + label + 1 + swatch + 2 + unit);
        let note = if page.iter().any(|&i| self.fields[i].restart) {
            [" Restart required", " Restart", ""]
                .into_iter()
                .find(|n| room >= n.width() + 12)
                .unwrap_or("")
        } else {
            ""
        };
        let max_width = if self.page == Page::General { 8 } else { 40 };
        let width = room.saturating_sub(note.width()).min(max_width) as u16;
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
                Some(Err(heading)) => frame.render_widget(
                    Paragraph::new(*heading)
                        .style(Style::default().fg(t.bright).add_modifier(Modifier::BOLD)),
                    rect,
                ),
                Some(Ok(i)) => self.draw_field(t, preview, frame, rect, *i, (label, width), note),
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
        let rows = report.rows();
        let rows: Vec<_> = rows
            .iter()
            .map(|row| match row {
                Row::Heading(text) => ReportRow::Heading(text, heading(t)),
                Row::Item(label, value, tone) => ReportRow::Item(
                    label,
                    value,
                    match tone {
                        Tone::Good => t.text,
                        Tone::Bad => t.danger,
                        Tone::Unknown => t.muted,
                    },
                ),
            })
            .collect();
        let lines = report_lines(t, &rows, area.width);
        let last = (lines.len() as u16).saturating_sub(area.height);
        self.report_top = self.report_top.min(last);
        frame.render_widget(Paragraph::new(lines).scroll((self.report_top, 0)), area);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_field(
        &mut self,
        t: &Theme,
        preview: &Theme,
        frame: &mut Frame,
        row: Rect,
        i: usize,
        (label, width): (usize, u16),
        note: &str,
    ) {
        let field = &self.fields[i];
        let selected = i == self.selected;
        let mut spans = vec![
            Span::styled(
                if self.changed(i) || self.inputs[i].text != self.saved[i] {
                    "•"
                } else {
                    " "
                },
                Style::default().fg(t.unread),
            ),
            Span::styled(
                crate::ui::pad(&crate::ui::clip(field.label, label), label),
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
        let note = if field.restart {
            note
        } else if field.kind == Kind::Color && !self.follow[i] {
            CUSTOM
        } else {
            ""
        };
        let bracket = Style::default().fg(if selected { t.focus } else { t.border });
        if choice(field.kind) {
            // ‹ › take the place of the brackets, lined up with the color inputs.
            let label = match field.kind {
                Kind::Theme => {
                    spans.push(Span::raw("   "));
                    self.preset().label()
                }
                Kind::Display => Display::parse(&self.inputs[i].text)
                    .unwrap_or_default()
                    .label(),
                _ => Pet::parse(&self.inputs[i].text).unwrap_or_default().label(),
            };
            let used = spans.iter().map(|s| s.content.width()).sum::<usize>() as u16;
            let text = format!("‹ {label} ›");
            let input = Rect::new(row.x + used, row.y, text.width() as u16, 1).intersection(row);
            spans.push(Span::styled(
                text,
                if selected {
                    Style::default().fg(t.focus).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.text)
                },
            ));
            spans.push(Span::styled(unit, Style::default().fg(t.muted)));
            frame.render_widget(Paragraph::new(Line::from(spans)), row);
            self.rows.push((row, input, i));
            return;
        }
        let used = spans.iter().map(|s| s.content.width()).sum::<usize>() as u16;
        spans.push(Span::styled("[", bracket));
        frame.render_widget(Paragraph::new(Line::from(spans)), row);
        let input = Rect::new(row.x + used + 1, row.y, width, 1).intersection(row);
        if field.kind == Kind::Bool {
            let label = match self.inputs[i].text.as_str() {
                "true" => "Enabled",
                "false" => "Disabled",
                // The recording switch whose stored state could not be read.
                _ => "Unknown",
            };
            frame.render_widget(
                Paragraph::new(label).style(Style::default().fg(if selected {
                    t.focus
                } else {
                    t.text
                })),
                input,
            );
        } else {
            self.inputs[i].draw(frame, input, selected, "", t);
        }
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

fn unit(kind: Kind) -> &'static str {
    match kind {
        Kind::Millis => " ms",
        _ => "",
    }
}
/// The choice `delta` places from `now`, wrapping around.
fn step<T: Copy + PartialEq>(all: &[T], now: T, delta: isize) -> T {
    let at = all.iter().position(|&c| c == now).unwrap_or(0);
    all[(at as isize + delta).rem_euclid(all.len() as isize) as usize]
}
/// A setting chosen from a fixed list with ‹ ›, not typed.
fn choice(kind: Kind) -> bool {
    matches!(kind, Kind::Theme | Kind::Pet | Kind::Display)
}
/// Marks a color set by its own `[colors]` key rather than the theme.
const CUSTOM: &str = " custom";

/// One row of a read-only report as drawn: Diagnostics and Updates share this layout.
enum ReportRow<'a> {
    Heading(&'a str, Style),
    /// Label, value and the value's state colour.
    Item(&'a str, &'a str, Color),
    Blank,
}
/// Report section headings rely on weight rather than the focus colour.
fn heading(t: &Theme) -> Style {
    Style::default().fg(t.bright).add_modifier(Modifier::BOLD)
}
/// Headings set apart by a blank row; labels in one quiet column with values wrapped under their
/// own column. A label wider than the column takes its own row so neither it nor its value is cut.
fn report_lines(t: &Theme, rows: &[ReportRow<'_>], width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width).max(1);
    let column = (LABEL - 2).min(width / 3);
    let lead = 2 + column + 1;
    let room = width.saturating_sub(lead).max(1);
    let label_style = Style::default().fg(t.muted);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut after_heading = false;
    for row in rows {
        match row {
            ReportRow::Heading(text, style) => {
                if !after_heading && lines.last().is_some_and(|line| line.width() > 0) {
                    lines.push(Line::default());
                }
                lines.extend(
                    word_wrap(text, width)
                        .into_iter()
                        .map(|line| Line::styled(line, *style)),
                );
            }
            ReportRow::Item(label, value, color) => {
                let mut parts = word_wrap(value, room);
                if parts.is_empty() {
                    parts.push(String::new());
                }
                let mut first = format!("  {} ", crate::ui::pad(label, column));
                if label.width() > column {
                    for part in word_wrap(label, width.saturating_sub(2).max(1)) {
                        lines.push(Line::styled(format!("  {part}"), label_style));
                    }
                    first = " ".repeat(lead);
                }
                for (n, part) in parts.into_iter().enumerate() {
                    let lead = if n == 0 {
                        first.clone()
                    } else {
                        " ".repeat(lead)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(lead, label_style),
                        Span::styled(part, Style::default().fg(*color)),
                    ]));
                }
            }
            ReportRow::Blank => lines.push(Line::default()),
        }
        after_heading = matches!(row, ReportRow::Heading(..));
    }
    lines
}

/// Rows of whole words; only a word wider than the row is split.
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match rows.last_mut() {
            Some(row) if row.width() + 1 + word.width() <= width => {
                row.push(' ');
                row.push_str(word);
            }
            _ if word.width() > width => rows.extend(
                crate::ui::wrap_text(word, width as u16)
                    .into_iter()
                    .map(|line| line.to_string()),
            ),
            _ => rows.push(word.into()),
        }
    }
    rows
}

fn shrink_top(area: &mut Rect, rows: u16) {
    let rows = rows.min(area.height);
    area.y += rows;
    area.height -= rows;
}

/// Two readable examples, separate from the real Save/Cancel controls.
fn draw_preview(t: &Theme, p: &Theme, frame: &mut Frame, area: Rect) {
    let s = |text: &'static str, fg: Color| Span::styled(text, Style::default().fg(fg));
    let lines = [
        (
            p.agents_bg,
            vec![
                s("Status  ", t.muted),
                s("● Working   ", p.agents_blue),
                s("○ Idle   ", p.agents_green),
                s("? Waiting   ", p.agents_yellow),
                s("! Error", p.agents_red),
            ],
        ),
        (
            p.bg,
            vec![
                s("Text    ", t.muted),
                s("Normal   ", p.text),
                s("Muted   ", p.muted),
                s("Code   ", p.reply_code),
                s("Heading", p.reply_heading),
            ],
        ),
    ];
    frame.render_widget(
        Paragraph::new("Preview · unsaved colors").style(Style::default().fg(t.muted)),
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

/// Sets one setting in the document, keeping the key's place and the value's comment; None
/// removes the key.
fn apply(
    document: &mut toml_edit::DocumentMut,
    field: &Field,
    text: Option<&str>,
) -> anyhow::Result<()> {
    use toml_edit::{Item, Value};
    let value = match text {
        None => None,
        Some(text) => Some(match field.kind {
            Kind::Columns | Kind::Millis => Value::from(text.trim().parse::<i64>()?),
            Kind::Bool => Value::from(text.parse::<bool>()?),
            Kind::Color | Kind::Theme | Kind::Pet | Kind::Display => Value::from(text.trim()),
            Kind::Command => Value::from(text),
        }),
    };
    let (table, key) = match field.key.split_once('.') {
        Some((table, key)) => {
            let item = match document.as_table_mut().entry(table) {
                toml_edit::Entry::Vacant(_) if value.is_none() => return Ok(()),
                entry => entry.or_insert(toml_edit::table()),
            };
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
            let kept = remove(table, key);
            // The table's last key is gone: its comment lines stay under the table header.
            if let Some(header) = field
                .key
                .split_once('.')
                .and_then(|(table, _)| document.get_mut(table))
                .and_then(Item::as_table_mut)
                .filter(|_| !kept.is_empty())
            {
                let decor = header.decor_mut();
                let old = decor.suffix().and_then(|s| s.as_str()).unwrap_or("");
                decor.set_suffix(format!("{old}\n{}", kept.trim_end_matches('\n')));
            }
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

/// Removes a key. Comment lines written above it stay in place: above the next key, or after
/// the one before when it was the last. Returns them when the table has no key left to keep them.
fn remove(table: &mut dyn toml_edit::TableLike, key: &str) -> String {
    use toml_edit::Item;
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_owned()).collect();
    let Some(at) = keys.iter().position(|k| k == key) else {
        return String::new();
    };
    let above = table
        .key(key)
        .and_then(|k| k.leaf_decor().prefix())
        .and_then(|p| p.as_str())
        .unwrap_or("");
    // Lines an earlier removal left after this key's value.
    let after = table
        .get(key)
        .and_then(Item::as_value)
        .and_then(|v| v.decor().suffix())
        .and_then(|s| s.as_str())
        .and_then(|s| s.split_once('\n'))
        .map_or("", |(_, rest)| rest);
    let kept: String = above
        .lines()
        .chain(after.lines())
        .filter(|line| line.trim_start().starts_with('#'))
        .map(|line| format!("{line}\n"))
        .collect();
    table.remove(key);
    if kept.is_empty() {
        return kept;
    }
    if let Some(mut next) = keys.get(at + 1).and_then(|k| table.key_mut(k)) {
        let decor = next.leaf_decor_mut();
        let old = decor.prefix().and_then(|p| p.as_str()).unwrap_or("");
        decor.set_prefix(format!("{kept}{old}"));
    } else if let Some(value) = at
        .checked_sub(1)
        .and_then(|before| table.get_mut(&keys[before]))
        .and_then(Item::as_value_mut)
    {
        let decor = value.decor_mut();
        let old = decor.suffix().and_then(|s| s.as_str()).unwrap_or("");
        decor.set_suffix(format!("{old}\n{}", kept.trim_end_matches('\n')));
    } else {
        return kept;
    }
    String::new()
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
