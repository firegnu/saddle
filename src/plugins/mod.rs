pub mod registry;
pub mod runtime;
pub mod ui;
use anyhow::{Context, Result, ensure};
use registry::{Manifest, Registry};
use runtime::{Notices, Runtime, Snapshot};
use saddle_plugin_protocol::Message;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Clone)]
pub struct Panel {
    pub id: String,
    pub name: String,
    pub state: String,
    pub note: String,
    pub picture: Option<Arc<runtime::Picture>>,
    pub interactive: bool,
}
impl Panel {
    pub fn unavailable(id: &str) -> Self {
        Self {
            id: id.into(),
            name: id.into(),
            state: "Unavailable".into(),
            note: "Plugin unavailable · Close".into(),
            picture: None,
            interactive: false,
        }
    }
    pub fn draw(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        if let Some(p) = &self.picture
            && p.buffer.area.width == area.width
            && p.buffer.area.height == area.height
        {
            let truecolor = crate::theme::truecolor(std::env::var("COLORTERM").ok().as_deref());
            for y in 0..area.height {
                for x in 0..area.width {
                    let mut c = p.buffer[(x, y)].clone();
                    if !truecolor {
                        c.fg = crate::theme::nearest_256(c.fg);
                        c.bg = crate::theme::nearest_256(c.bg);
                    }
                    frame.buffer_mut()[(area.x + x, area.y + y)] = c;
                }
            }
        }
        if !self.interactive || !self.note.is_empty() {
            frame.render_widget(
                ratatui::widgets::Paragraph::new(format!("{} · {}", self.state, self.note))
                    .wrap(Default::default()),
                area,
            );
        }
    }
}
struct Running {
    runtime: Runtime,
    manifest: Manifest,
    size: Option<(u16, u16)>,
    revision: u64,
    focused: bool,
    enabled: bool,
    theme: serde_json::Value,
}
pub struct Manager {
    pub registry: Registry,
    pub notices: Notices,
    running: BTreeMap<String, Running>,
    restart: BTreeSet<String>,
    errors: BTreeMap<String, String>,
    input: u64,
    catalog: BTreeMap<String, (Manifest, PathBuf)>,
    theme: serde_json::Value,
}
impl Manager {
    pub fn open(path: PathBuf) -> Self {
        let registry = Registry::open(path);
        let mut this = Self {
            registry,
            notices: Notices::default(),
            running: BTreeMap::new(),
            restart: BTreeSet::new(),
            errors: BTreeMap::new(),
            input: 0,
            catalog: BTreeMap::new(),
            theme: serde_json::Value::Null,
        };
        this.catalog();
        let ids: Vec<_> = this
            .registry
            .entries
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.id.clone())
            .collect();
        for id in ids {
            this.start(&id);
        }
        this
    }
    fn start(&mut self, id: &str) {
        let result = (|| -> Result<()> {
            let e = self
                .registry
                .entries
                .iter()
                .find(|e| e.id == id)
                .context("registration missing")?;
            let manifest = Manifest::read(&e.directory)?;
            ensure!(manifest.id == id, "manifest identity changed");
            let runtime =
                Runtime::with_notices(&e.directory, manifest.clone(), self.notices.clone());
            self.running.insert(
                id.into(),
                Running {
                    runtime,
                    manifest,
                    size: None,
                    revision: 0,
                    focused: false,
                    enabled: true,
                    theme: serde_json::Value::Null,
                },
            );
            Ok(())
        })();
        if let Err(e) = result {
            self.errors.insert(id.into(), format!("{e:#}"));
        } else {
            self.errors.remove(id);
        }
    }
    pub fn state(&self, id: &str) -> String {
        if let Some(r) = self.running.get(id) {
            r.runtime.state()
        } else if self.errors.contains_key(id) {
            "Unavailable".into()
        } else if self.registry.entries.iter().any(|e| e.id == id) {
            "Disabled".into()
        } else {
            "Unavailable".into()
        }
    }
    pub fn enabled_here(&self, id: &str) -> bool {
        self.running.get(id).is_some_and(|r| r.enabled) || self.errors.contains_key(id)
    }
    pub fn stopped(&self, id: &str) -> bool {
        self.running.get(id).is_none_or(|r| r.runtime.reaped())
    }
    fn catalog(&mut self) {
        self.catalog.clear();
        for e in &self.registry.entries {
            if let Ok(m) = Manifest::read(&e.directory)
                && m.id == e.id
                && let Ok(p) = m.program(&e.directory)
            {
                self.catalog.insert(e.id.clone(), (m, p));
            }
        }
    }
    pub fn refresh(&mut self) -> Result<()> {
        self.registry.refresh()?;
        self.catalog();
        Ok(())
    }
    pub fn manifest(&self, id: &str) -> Result<Manifest> {
        self.catalog
            .get(id)
            .map(|(m, _)| m.clone())
            .context("plugin unavailable")
    }
    pub fn program(&self, id: &str) -> Option<&Path> {
        self.catalog.get(id).map(|(_, p)| p.as_path())
    }
    pub fn theme(&mut self, t: &crate::theme::Theme) {
        self.theme = json!({"text":saddle_plugin_sdk::color(t.text),"muted":saddle_plugin_sdk::color(t.muted),"background":saddle_plugin_sdk::color(t.bg),"accent":saddle_plugin_sdk::color(t.focus),"error":saddle_plugin_sdk::color(t.danger)});
    }

    pub fn enable(&mut self, id: &str) -> Result<()> {
        ensure!(self.stopped(id), "plugin already running; use Restart");
        self.registry.enabled(id, true)?;
        self.running.remove(id);
        self.start(id);
        Ok(())
    }
    pub fn disable(&mut self, id: &str) -> Result<()> {
        self.registry.enabled(id, false)?;
        self.restart.remove(id);
        self.errors.remove(id);
        if let Some(r) = self.running.get_mut(id) {
            r.enabled = false;
            r.runtime.stop();
        }
        self.notices.lock().unwrap().clear(id);
        Ok(())
    }
    pub fn restart(&mut self, id: &str) -> Result<()> {
        ensure!(self.enabled_here(id), "plugin is disabled");
        ensure!(self.state(id) != "Stopping", "already stopping");
        if let Some(r) = self.running.get(id) {
            r.runtime.stop();
        }
        self.restart.insert(id.into());
        Ok(())
    }
    pub fn remove(&mut self, id: &str) -> Result<()> {
        ensure!(
            self.stopped(id) && !self.enabled_here(id),
            "disable and wait for process exit first"
        );
        self.registry.remove(id)?;
        self.running.remove(id);
        self.errors.remove(id);
        Ok(())
    }
    pub fn add(&mut self, dir: &Path, preview: &Manifest) -> Result<()> {
        self.registry.add(dir, preview)?;
        self.catalog();
        Ok(())
    }
    pub fn tick(&mut self) {
        let ready: Vec<_> = self
            .restart
            .iter()
            .filter(|id| self.stopped(id))
            .cloned()
            .collect();
        for id in ready {
            self.restart.remove(&id);
            self.running.remove(&id);
            self.start(&id);
        }
        self.notices.lock().unwrap().expire();
    }
    pub fn sync(
        &mut self,
        terminals: &mut crate::terminals::Terminals,
        area: ratatui::layout::Rect,
        focused: bool,
    ) {
        self.tick();
        let visible = terminals.rects(area);
        let active = terminals.active_pane().id;
        let panes: Vec<_> = terminals
            .tabs
            .iter()
            .flat_map(|t| &t.panes)
            .filter_map(|p| p.plugin_id().map(|id| (p.id, id.to_owned())))
            .collect();
        let open: BTreeSet<_> = panes.iter().map(|(_, id)| id.as_str()).collect();
        for (id, r) in &mut self.running {
            if !open.contains(id.as_str()) && r.size.is_some() {
                r.runtime
                    .send(Message::event("panel.close", json!({"panel":"main"})));
                r.size = None;
                r.focused = false;
            }
        }
        for (pane, id) in panes {
            let mut view = Panel::unavailable(&id);
            if let Some(r) = self.running.get_mut(&id) {
                let rect = visible
                    .iter()
                    .find(|(p, _)| *p == pane)
                    .map(|(_, r)| crate::ui::inner(*r));
                let s = r.runtime.snapshot();
                view.name = r.manifest.name.clone();
                view.state = s.state.clone();
                view.note = s.note.clone();
                view.picture = s.picture;
                view.interactive = s.interactive;
                if s.state == "Running" {
                    if r.theme != self.theme {
                        r.theme = self.theme.clone();
                        r.runtime.send(Message::event("theme", self.theme.clone()));
                    }
                    if let Some(rect) = rect {
                        let size = (rect.width, rect.height);
                        if r.size != Some(size) {
                            let name = if r.size.is_some() {
                                "panel.resize"
                            } else {
                                "panel.open"
                            };
                            r.revision += 1;
                            r.runtime.send(Message::event(name,json!({"panel":"main","cols":size.0,"rows_count":size.1,"size_revision":r.revision})));
                            r.size = Some(size);
                            view.interactive = false;
                        }
                    }
                    let focus = focused && active == pane && rect.is_some();
                    if r.focused != focus {
                        r.focused = focus;
                        r.runtime.send(Message::event(
                            "panel.focus",
                            json!({"panel":"main","focused":focus}),
                        ));
                    }
                }
                view.interactive &= view
                    .picture
                    .as_ref()
                    .is_some_and(|p| p.revision == r.revision);
                if view.state == "Disabled" {
                    view.note = "Plugin disabled · Open Plugins / Close".into();
                }
            } else if let Some(error) = self.errors.get(&id) {
                view.note = error.clone();
            } else if self.registry.entries.iter().any(|e| e.id == id) {
                view.state = "Disabled".into();
                view.note = "Plugin disabled · Open Plugins / Close".into();
            }
            terminals.get_mut(pane).unwrap().plugin = Some(view);
        }
    }
    pub fn input(&mut self, panel: &Panel, event: serde_json::Value) -> bool {
        if !panel.interactive {
            return false;
        }
        let Some(pic) = &panel.picture else {
            return false;
        };
        let Some(r) = self.running.get_mut(&panel.id) else {
            return false;
        };
        if r.runtime.state() != "Running" {
            return false;
        }
        if !r.focused {
            r.focused = true;
            r.runtime.send(Message::event(
                "panel.focus",
                json!({"panel":"main","focused":true}),
            ));
        }
        self.input += 1;
        r.runtime.send(Message::event("input",json!({"panel":"main","input_id":self.input,"frame_id":pic.frame_id,"size_revision":pic.revision,"event":event})))
    }
    pub fn note(&self, id: &str) -> String {
        self.snapshot(id)
            .map(|s| s.note)
            .or_else(|| self.errors.get(id).cloned())
            .unwrap_or_default()
    }
    pub fn snapshot(&self, id: &str) -> Option<Snapshot> {
        self.running.get(id).map(|r| r.runtime.snapshot())
    }
}

pub fn key(k: crossterm::event::KeyEvent) -> serde_json::Value {
    use crossterm::event::KeyCode;
    let code = match k.code {
        KeyCode::Char(c) => json!({"char":c.to_string()}),
        KeyCode::F(n) => json!({"name":"function","number":n}),
        KeyCode::Media(m) => json!({"name":"media","key":match m {
            crossterm::event::MediaKeyCode::Play=>"play",crossterm::event::MediaKeyCode::Pause=>"pause",crossterm::event::MediaKeyCode::PlayPause=>"play_pause",crossterm::event::MediaKeyCode::Reverse=>"reverse",crossterm::event::MediaKeyCode::Stop=>"stop",crossterm::event::MediaKeyCode::FastForward=>"fast_forward",crossterm::event::MediaKeyCode::Rewind=>"rewind",crossterm::event::MediaKeyCode::TrackNext=>"track_next",crossterm::event::MediaKeyCode::TrackPrevious=>"track_previous",crossterm::event::MediaKeyCode::Record=>"record",crossterm::event::MediaKeyCode::LowerVolume=>"lower_volume",crossterm::event::MediaKeyCode::RaiseVolume=>"raise_volume",crossterm::event::MediaKeyCode::MuteVolume=>"mute_volume"
        }}),
        KeyCode::Modifier(m) => json!({"name":"modifier","key":match m {
            crossterm::event::ModifierKeyCode::LeftShift=>"left_shift",crossterm::event::ModifierKeyCode::LeftControl=>"left_control",crossterm::event::ModifierKeyCode::LeftAlt=>"left_alt",crossterm::event::ModifierKeyCode::LeftSuper=>"left_super",crossterm::event::ModifierKeyCode::LeftHyper=>"left_hyper",crossterm::event::ModifierKeyCode::LeftMeta=>"left_meta",crossterm::event::ModifierKeyCode::RightShift=>"right_shift",crossterm::event::ModifierKeyCode::RightControl=>"right_control",crossterm::event::ModifierKeyCode::RightAlt=>"right_alt",crossterm::event::ModifierKeyCode::RightSuper=>"right_super",crossterm::event::ModifierKeyCode::RightHyper=>"right_hyper",crossterm::event::ModifierKeyCode::RightMeta=>"right_meta",crossterm::event::ModifierKeyCode::IsoLevel3Shift=>"iso_level3_shift",crossterm::event::ModifierKeyCode::IsoLevel5Shift=>"iso_level5_shift"
        }}),
        other => json!({"name":match other {
            KeyCode::Enter=>"enter",KeyCode::Esc=>"esc",KeyCode::Backspace=>"backspace",KeyCode::Left=>"left",KeyCode::Right=>"right",KeyCode::Up=>"up",KeyCode::Down=>"down",KeyCode::Home=>"home",KeyCode::End=>"end",KeyCode::PageUp=>"page_up",KeyCode::PageDown=>"page_down",KeyCode::Tab=>"tab",KeyCode::BackTab=>"back_tab",KeyCode::Delete=>"delete",KeyCode::Insert=>"insert",KeyCode::Null=>"null",KeyCode::CapsLock=>"caps_lock",KeyCode::ScrollLock=>"scroll_lock",KeyCode::NumLock=>"num_lock",KeyCode::PrintScreen=>"print_screen",KeyCode::Pause=>"pause",KeyCode::Menu=>"menu",KeyCode::KeypadBegin=>"keypad_begin",_=>unreachable!()
        }}),
    };
    json!({"type":"key","code":code,"modifiers":mods(k.modifiers),"phase":format!("{:?}",k.kind).to_lowercase()})
}
fn mods(m: crossterm::event::KeyModifiers) -> Vec<&'static str> {
    use crossterm::event::KeyModifiers as M;
    [
        (M::SHIFT, "shift"),
        (M::CONTROL, "control"),
        (M::ALT, "alt"),
        (M::SUPER, "super"),
        (M::HYPER, "hyper"),
        (M::META, "meta"),
    ]
    .into_iter()
    .filter_map(|(flag, name)| m.contains(flag).then_some(name))
    .collect()
}
pub fn mouse(m: crossterm::event::MouseEvent, area: ratatui::layout::Rect) -> serde_json::Value {
    use crossterm::event::MouseEventKind as K;
    let (action, button, dx, dy) = match m.kind {
        K::Down(b) => ("down", Some(b), 0, 0),
        K::Up(b) => ("up", Some(b), 0, 0),
        K::Drag(b) => ("drag", Some(b), 0, 0),
        K::Moved => ("move", None, 0, 0),
        K::ScrollUp => ("scroll", None, 0, -1),
        K::ScrollDown => ("scroll", None, 0, 1),
        K::ScrollLeft => ("scroll", None, -1, 0),
        K::ScrollRight => ("scroll", None, 1, 0),
    };
    json!({"type":"mouse","action":action,"button":button.map(|b|format!("{b:?}").to_lowercase()),"x":m.column-area.x,"y":m.row-area.y,"dx":dx,"dy":dy,"modifiers":mods(m.modifiers)})
}

impl Drop for Manager {
    fn drop(&mut self) {
        for r in self.running.values() {
            r.runtime.stop();
        }
    }
}
