mod capture;
pub mod cli;
pub mod core;
pub mod navigation;
pub mod palette;
pub mod registry;
pub mod resources;
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
    pub fn draw(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, focused: bool) {
        if let Some(p) = &self.picture
            && p.buffer.area.width == area.width
            && p.buffer.area.height == area.height
        {
            if focused
                && self.interactive
                && self.note.is_empty()
                && let Some([x, y]) = p.cursor
            {
                frame.set_cursor_position((area.x + x, area.y + y));
            }
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
    core_catalog: core::Catalog,
    running: BTreeMap<String, Running>,
    restart: BTreeSet<String>,
    errors: BTreeMap<String, String>,
    input: u64,
    catalog: BTreeMap<String, (Manifest, PathBuf)>,
    theme: serde_json::Value,
    resources: resources::Resources,
    resource_status: BTreeMap<String, Result<resources::Status, String>>,
    receipts: BTreeMap<String, resources::Receipt>,
}
impl Manager {
    pub fn open(path: PathBuf) -> Self {
        Self::with_core(path, &[])
    }
    pub fn with_core(path: PathBuf, core_catalog: core::Catalog) -> Self {
        Self::with_resources(path, core_catalog, resources::Resources::from_environment())
    }
    pub fn with_resources(
        path: PathBuf,
        core_catalog: core::Catalog,
        resources: resources::Resources,
    ) -> Self {
        let registry = core::registry(path, core_catalog);
        let mut this = Self {
            registry,
            core_catalog,
            notices: Notices::default(),
            running: BTreeMap::new(),
            restart: BTreeSet::new(),
            errors: BTreeMap::new(),
            input: 0,
            catalog: BTreeMap::new(),
            theme: serde_json::Value::Null,
            resources,
            resource_status: BTreeMap::new(),
            receipts: BTreeMap::new(),
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
        // Startup only upgrades enabled built-ins' own, unmodified outdated resources.
        for plugin in core_catalog {
            let m = plugin.manifest();
            if core::state(&this.registry, m.id) == core::State::Enabled {
                let receipt = this.resources.apply(m, resources::Trigger::Startup, true);
                if receipt.notable() {
                    this.receipts.insert(m.id.into(), receipt);
                }
            }
        }
        this.refresh_resources();
        this
    }
    pub fn core_catalog(&self) -> core::Catalog {
        self.core_catalog
    }
    pub fn core_state(&self, id: &str) -> Option<core::State> {
        self.core_catalog
            .iter()
            .any(|p| p.manifest().id == id)
            .then(|| core::state(&self.registry, id))
    }
    /// The switch is saved first; resources follow only after it succeeded. Disable keeps
    /// installed resources.
    pub fn core_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        self.registry.core_enabled(id, enabled)?;
        self.receipts.remove(id);
        if enabled {
            self.apply_resources(id, resources::Trigger::Enable, true);
        }
        Ok(())
    }
    pub fn core_sync(&mut self, id: &str) -> Result<()> {
        self.refresh()?;
        ensure!(
            self.core_state(id) == Some(core::State::Enabled),
            "enable the plugin before syncing resources"
        );
        self.apply_resources(id, resources::Trigger::Sync, true);
        Ok(())
    }
    pub fn core_remove_resources(&mut self, id: &str) -> Result<()> {
        self.refresh()?;
        ensure!(
            self.core_state(id) == Some(core::State::Disabled),
            "disable the plugin before removing resources"
        );
        self.apply_resources(id, resources::Trigger::Remove, false);
        Ok(())
    }
    fn apply_resources(&mut self, id: &str, trigger: resources::Trigger, enabled: bool) {
        if let Some(plugin) = self.core_catalog.iter().find(|p| p.manifest().id == id) {
            let receipt = self.resources.apply(plugin.manifest(), trigger, enabled);
            self.receipts.insert(id.into(), receipt);
        }
        self.refresh_resources();
    }
    /// Read-only classification for display; recomputed on Refresh and after actions.
    fn refresh_resources(&mut self) {
        self.resource_status = self
            .core_catalog
            .iter()
            .map(|p| (p.manifest().id.into(), self.resources.status(p.manifest())))
            .collect();
    }
    pub fn core_resources(&self, id: &str) -> Option<&Result<resources::Status, String>> {
        self.resource_status.get(id)
    }
    pub fn core_receipt(&self, id: &str) -> Option<&resources::Receipt> {
        self.receipts.get(id)
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
        self.refresh_resources();
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
    pub fn attention_items(&self) -> Vec<crate::attention::Item> {
        use crate::attention::{Item, Kind, Target};
        let mut items = Vec::new();
        for entry in &self.registry.entries {
            let running = self.running.get(&entry.id);
            let manifest = running
                .map(|r| &r.manifest)
                .or_else(|| self.catalog.get(&entry.id).map(|(m, _)| m));
            let Some(m) =
                manifest.filter(|m| m.required_capabilities.iter().any(|c| c == "attention.v1"))
            else {
                continue;
            };
            if !self.enabled_here(&entry.id) {
                continue;
            }
            let snapshot = running.map(|r| r.runtime.snapshot());
            if let Some(s) = snapshot
                .as_ref()
                .filter(|s| s.state == "Running" && !self.restart.contains(&entry.id))
                && let Some((revision, snapshot)) = &s.attention
            {
                items.extend(snapshot.items.iter().map(|i| Item {
                    target: Target::Plugin {
                        plugin: entry.id.clone(),
                        session: s.session,
                        revision: *revision,
                        item: i.id.clone(),
                    },
                    kind: Kind::Plugin,
                    label: i.title.clone(),
                    note: if i.note.is_empty() {
                        m.name.clone()
                    } else {
                        format!("{} · {}", m.name, i.note)
                    },
                }));
            } else {
                items.push(Item {
                    target: Target::Source(format!("plugin:{}", entry.id)),
                    kind: Kind::Unavailable,
                    label: m.name.clone(),
                    note: if self.restart.contains(&entry.id) {
                        "Restarting".into()
                    } else if let Some(s) = snapshot {
                        if s.state == "Running" {
                            "Waiting for snapshot".into()
                        } else {
                            format!("{} · {}", s.state, s.note)
                        }
                    } else {
                        "Unavailable".into()
                    },
                });
            }
        }
        items
    }
    pub fn open_attention(&self, target: &crate::attention::Target) -> bool {
        let crate::attention::Target::Plugin {
            plugin,
            session,
            revision,
            item,
        } = target
        else {
            return false;
        };
        !self.restart.contains(plugin)
            && self
                .running
                .get(plugin)
                .is_some_and(|r| r.enabled && r.runtime.open_attention(*session, *revision, item))
    }
    pub fn palette_items(&self, opened: &BTreeSet<String>) -> Vec<palette::Item> {
        // A conflicting external registration keeps the ID; that one stays reachable here.
        let builtin: Vec<_> = self
            .core_catalog
            .iter()
            .map(|p| p.manifest())
            .filter(|m| self.core_state(m.id) != Some(core::State::Conflict))
            .map(|m| palette::Item {
                id: m.id.into(),
                title: m.name.into(),
                state: format!(
                    "Built-in · {}",
                    if self.core_state(m.id) == Some(core::State::Enabled) {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ),
                note: String::new(),
                has_view: false,
                opened: false,
                pid: None,
                builtin: true,
            })
            .collect();
        let mut items: Vec<_> = self
            .registry
            .entries
            .iter()
            .map(|e| {
                let running = self.running.get(&e.id);
                let manifest = running
                    .map(|r| &r.manifest)
                    .or_else(|| self.catalog.get(&e.id).map(|(m, _)| m));
                let snapshot = running.map(|r| r.runtime.snapshot());
                let state = if self.restart.contains(&e.id) {
                    "Restarting".into()
                } else if running.is_some_and(|r| !r.enabled) {
                    if self.stopped(&e.id) {
                        "Disabled".into()
                    } else {
                        "Stopping".into()
                    }
                } else if manifest.is_none() {
                    "Unavailable".into()
                } else {
                    snapshot
                        .as_ref()
                        .map(|s| s.state.clone())
                        .unwrap_or_else(|| self.state(&e.id))
                };
                palette::Item {
                    id: e.id.clone(),
                    title: manifest
                        .map(|m| {
                            m.action
                                .as_ref()
                                .map(|a| a.title.clone())
                                .unwrap_or_else(|| m.name.clone())
                        })
                        .unwrap_or_else(|| e.id.clone()),
                    state,
                    note: snapshot
                        .as_ref()
                        .map(|s| s.note.clone())
                        .or_else(|| self.errors.get(&e.id).cloned())
                        .unwrap_or_else(|| {
                            if manifest.is_none() {
                                "Plugin files or manifest are unavailable.".into()
                            } else {
                                String::new()
                            }
                        }),
                    has_view: manifest
                        .is_some_and(|m| m.required_capabilities.iter().any(|c| c == "panel.v1")),
                    opened: opened.contains(&e.id),
                    pid: snapshot.and_then(|s| s.pid),
                    builtin: false,
                }
            })
            .collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        builtin.into_iter().chain(items).collect()
    }
    pub fn placement(&self, id: &str) -> saddle_plugin_protocol::Placement {
        self.running
            .get(id)
            .and_then(|r| r.manifest.view.as_ref())
            .map_or(saddle_plugin_protocol::Placement::Workspace, |v| {
                v.placement
            })
    }
    pub fn sync(
        &mut self,
        terminals: &mut crate::terminals::Terminals,
        area: ratatui::layout::Rect,
        focused: bool,
    ) {
        self.sync_with_overlay(terminals, area, focused, None);
    }
    pub fn sync_with_overlay(
        &mut self,
        terminals: &mut crate::terminals::Terminals,
        area: ratatui::layout::Rect,
        focused: bool,
        overlay: Option<(&str, ratatui::layout::Rect, bool)>,
    ) -> Option<Panel> {
        self.tick();
        let visible = terminals.rects(area);
        let active = terminals.active_pane().id;
        let panes: Vec<_> = terminals
            .tabs
            .iter()
            .flat_map(|t| &t.panes)
            .filter_map(|p| p.plugin_id().map(|id| (p.id, id.to_owned())))
            .collect();
        let mut open: BTreeSet<_> = panes.iter().map(|(_, id)| id.as_str()).collect();
        if let Some((id, _, _)) = overlay {
            open.insert(id);
        }
        for (id, r) in &mut self.running {
            if !open.contains(id.as_str()) && r.size.is_some() {
                r.runtime
                    .send(Message::event("panel.close", json!({"panel":"main"})));
                r.size = None;
                r.focused = false;
            }
        }
        for (pane, id) in panes {
            let rect = visible
                .iter()
                .find(|(p, _)| *p == pane)
                .map(|(_, r)| crate::ui::inner(*r));
            let view = self.sync_panel(
                &id,
                rect,
                focused && active == pane && rect.is_some(),
                false,
            );
            terminals.get_mut(pane).unwrap().plugin = Some(view);
        }
        overlay.map(|(id, rect, focus)| self.sync_panel(id, Some(rect), focus, true))
    }
    fn sync_panel(
        &mut self,
        id: &str,
        rect: Option<ratatui::layout::Rect>,
        focused: bool,
        overlay: bool,
    ) -> Panel {
        let mut view = Panel::unavailable(id);
        if let Some(r) = self.running.get_mut(id) {
            let s = r.runtime.snapshot();
            view.name = r.manifest.name.clone();
            view.state = s.state.clone();
            view.note = s.note;
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
                        r.runtime.send(Message::event(name,json!({"panel":"main","cols":size.0,"rows_count":size.1,"size_revision":r.revision,"reserved_keys":if overlay {vec!["ctrl+]","esc"]} else {vec!["ctrl+]"]}})));
                        r.size = Some(size);
                        view.interactive = false;
                    }
                }
                if r.focused != focused {
                    r.focused = focused;
                    r.runtime.send(Message::event(
                        "panel.focus",
                        json!({"panel":"main","focused":focused}),
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
        } else if let Some(error) = self.errors.get(id) {
            view.note = error.clone();
        } else if self.registry.entries.iter().any(|e| e.id == id) {
            view.state = "Disabled".into();
            view.note = "Plugin disabled · Open Plugins / Close".into();
        }
        view
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
        if r.runtime.state() != "Running" || pic.revision != r.revision {
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
    pub fn invoke_command(
        &self,
        id: &str,
        request: &str,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<u64> {
        let r = self
            .running
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("plugin is not enabled"))?;
        anyhow::ensure!(
            r.enabled
                && r.manifest
                    .required_capabilities
                    .iter()
                    .any(|c| c == "command.v1"),
            "plugin does not provide commands"
        );
        r.runtime.invoke(request, method, params)
    }
    pub fn command_result(
        &self,
        id: &str,
        session: u64,
        request: &str,
    ) -> Option<serde_json::Value> {
        match self.running.get(id) {
            Some(r) if r.enabled && r.runtime.snapshot().session == session => {
                r.runtime.command_result(session, request)
            }
            _ => Some(
                json!({"ok":false,"error":{"code":"result_unknown","message":"Plugin was disabled or restarted; check state before retrying"}}),
            ),
        }
    }
    pub fn take_view_closes(&self) -> Vec<String> {
        self.running
            .iter()
            .filter_map(|(id, r)| {
                let input = r.runtime.take_close()?;
                let s = r.runtime.snapshot();
                (r.enabled && s.state == "Running" && s.input_id == input).then(|| id.clone())
            })
            .collect()
    }
    pub fn open_notification(&self, toast: &runtime::Toast) -> bool {
        let Some(r) = self.running.get(&toast.plugin) else {
            return false;
        };
        let s = r.runtime.snapshot();
        let Some(target) = &toast.target else {
            return false;
        };
        r.enabled
            && toast.expires > std::time::Instant::now()
            && self.notices.lock().unwrap().items.iter().any(|n| {
                n.plugin == toast.plugin
                    && n.session == toast.session
                    && n.notification_id == toast.notification_id
            })
            && s.state == "Running"
            && s.session == toast.session
            && r.runtime.send(Message::event(
                "notification.open",
                serde_json::to_value(target).expect("validated target"),
            ))
    }
    pub fn take_navigation(&self) -> Vec<runtime::Navigation> {
        self.running
            .values()
            .filter_map(|r| r.runtime.take_navigation())
            .collect()
    }
    pub fn take_telemetry_opens(&self) -> Vec<runtime::TelemetryOpen> {
        self.running
            .values()
            .filter_map(|r| r.runtime.take_telemetry())
            .collect()
    }
    /// The source is still the running session that sent it, with no newer input since.
    pub fn telemetry_open_current(&self, request: &runtime::TelemetryOpen) -> bool {
        self.running.get(&request.plugin).is_some_and(|r| {
            let s = r.runtime.snapshot();
            r.enabled
                && s.session == request.session
                && s.state == "Running"
                && s.input_id == request.input_id
        })
    }
    pub fn telemetry_open_result(
        &self,
        request: &runtime::TelemetryOpen,
        status: &str,
        message: &str,
    ) {
        if let Some(r) = self.running.get(&request.plugin)
            && r.runtime.snapshot().session == request.session
        {
            r.runtime.send(Message::response(
                request.request_id,
                json!({"status":status,"message":message}),
            ));
        }
    }
    pub fn navigation_current(&self, request: &runtime::Navigation) -> bool {
        self.running.get(&request.plugin).is_some_and(|r| {
            let s = r.runtime.snapshot();
            r.enabled
                && s.session == request.session
                && s.state == "Running"
                && s.input_id == request.input_id
        })
    }
    pub fn navigation_result(&self, request: &runtime::Navigation, status: &str, message: &str) {
        if let Some(r) = self.running.get(&request.plugin)
            && r.runtime.snapshot().session == request.session
        {
            r.runtime.send(Message::response(
                request.request_id,
                json!({"status":status,"message":message}),
            ));
        }
    }
    pub fn view_context(&self, id: &str, cwd: Option<String>) {
        if let Some(r) = self.running.get(id)
            && r.manifest
                .required_capabilities
                .iter()
                .any(|c| c == "view.context.v1")
        {
            r.runtime
                .send(Message::event("optional.view_context", json!({"cwd": cwd})));
        }
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
