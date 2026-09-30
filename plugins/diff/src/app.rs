use crate::{
    git::{self, Mode, Snapshot},
    view::{self, Document},
};
use anyhow::Result;
use saddle_plugin_sdk::{
    Context, Event, Plugin,
    ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
    },
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const INTERVAL: Duration = Duration::from_millis(750);
type ScanResult = Result<Option<(Arc<Snapshot>, Document)>>;
struct Job {
    generation: u64,
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<ScanResult>,
}
#[derive(Default, Clone, Copy)]
enum Layout {
    #[default]
    Auto,
    Unified,
    Split,
}
impl Layout {
    fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Unified => "Unified",
            Self::Split => "Split",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::Auto => Self::Unified,
            Self::Unified => Self::Split,
            Self::Split => Self::Auto,
        }
    }
}
pub struct DiffPlugin {
    cwd: Option<PathBuf>,
    mode: Mode,
    snapshot: Option<Arc<Snapshot>>,
    document: Document,
    generation: u64,
    job: Option<Job>,
    last: Option<Instant>,
    visible: bool,
    error: Option<String>,
    top: usize,
    horizontal: usize,
    layout: Layout,
    split: bool,
    sidebar: bool,
    size: Rect,
    sidebar_width: u16,
    sidebar_start: usize,
    pressed: Option<(PathBuf, u64, u64)>,
}
impl Default for DiffPlugin {
    fn default() -> Self {
        Self::new(None)
    }
}
impl DiffPlugin {
    pub fn new(cwd: Option<PathBuf>) -> Self {
        Self {
            cwd,
            mode: Mode::All,
            snapshot: None,
            document: Document::default(),
            generation: 0,
            job: None,
            last: None,
            visible: false,
            error: None,
            top: 0,
            horizontal: 0,
            layout: Layout::Auto,
            split: false,
            sidebar: true,
            size: Rect::default(),
            sidebar_width: 0,
            sidebar_start: 0,
            pressed: None,
        }
    }
    fn cancel(&mut self) {
        self.generation += 1;
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.last = None;
        self.pressed = None;
    }
    fn reset(&mut self) {
        self.cancel();
        self.snapshot = None;
        self.document = Document::default();
        self.top = 0;
        self.horizontal = 0;
        self.error = None;
    }
    fn tick(&mut self, ctx: &mut Context) {
        if self.job.as_ref().is_some_and(|j| j.handle.is_finished()) {
            let job = self.job.take().unwrap();
            let result = job
                .handle
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("Diff worker stopped")));
            if self.visible && job.generation == self.generation {
                self.last = Some(Instant::now());
                match result {
                    Ok(next) => {
                        let recovered = self.error.take().is_some();
                        if let Some((snapshot, document)) = next {
                            if let Some(old) = &self.snapshot {
                                self.top = view::restore(
                                    self.document.rows(self.split),
                                    old,
                                    self.top,
                                    document.rows(self.split),
                                    &snapshot,
                                );
                            }
                            self.snapshot = Some(snapshot);
                            self.document = document;
                            ctx.redraw();
                        } else if recovered {
                            ctx.redraw();
                        }
                    }
                    Err(e) => {
                        let error = e.to_string();
                        if self.error.as_ref() != Some(&error) {
                            self.error = Some(error);
                            ctx.redraw();
                        }
                    }
                }
            }
        }
        if self.visible
            && self.job.is_none()
            && self.last.is_none_or(|t| t.elapsed() >= INTERVAL)
            && let Some(cwd) = self.cwd.clone()
        {
            let (mode, previous) = (self.mode, self.snapshot.clone());
            let cancel = Arc::new(AtomicBool::new(false));
            let worker_cancel = cancel.clone();
            let handle = thread::spawn(move || {
                let snapshot = git::scan(&cwd, mode, &worker_cancel)?;
                if previous.as_deref() == Some(&snapshot) {
                    return Ok(None);
                }
                let doc = Document::build(&snapshot);
                Ok(Some((Arc::new(snapshot), doc)))
            });
            self.job = Some(Job {
                generation: self.generation,
                cancel,
                handle,
            });
        }
    }
    fn current_file(&self) -> usize {
        self.document
            .rows(self.split)
            .get(self.top)
            .map_or(0, |r| r.file)
    }
    fn jump_file(&mut self, index: usize) {
        if let Some(top) = self
            .document
            .rows(self.split)
            .iter()
            .position(|r| r.header && r.file == index)
        {
            self.top = top;
        }
    }
    fn scroll(&mut self, delta: isize) {
        self.top = self
            .top
            .saturating_add_signed(delta)
            .min(self.document.rows(self.split).len().saturating_sub(1));
    }
    fn jump_hunk(&mut self, forward: bool) {
        let rows = self.document.rows(self.split);
        let target = if forward {
            rows.iter()
                .enumerate()
                .find(|(i, r)| *i > self.top && r.hunk)
                .map(|(i, _)| i)
        } else {
            rows.iter()
                .enumerate()
                .rev()
                .find(|(i, r)| *i < self.top && r.hunk)
                .map(|(i, _)| i)
        };
        if let Some(top) = target {
            self.top = top;
        }
    }
    fn mouse_file(&self, e: &Value) -> Option<usize> {
        let x = e["x"].as_u64()?;
        let y = e["y"].as_u64()?;
        if x >= self.sidebar_width.saturating_sub(1).into()
            || y < 3
            || y >= self.size.height.saturating_sub(1).into()
        {
            return None;
        }
        let index = self.sidebar_start + y as usize - 3;
        self.snapshot.as_ref()?.files.get(index).map(|_| index)
    }
    fn input(&mut self, v: Value, ctx: &mut Context) -> Result<()> {
        let e = &v["event"];
        if e["type"] == "key" && e["phase"] != "release" {
            if e["modifiers"]
                .as_array()
                .is_some_and(|m| m.iter().any(|m| m == "ctrl" || m == "alt" || m == "super"))
            {
                return Ok(());
            }
            let key = e["code"]["char"]
                .as_str()
                .or(e["code"]["name"].as_str())
                .unwrap_or("");
            match key {
                "1" | "2" | "3" => {
                    let mode = match key {
                        "2" => Mode::Unstaged,
                        "3" => Mode::Staged,
                        _ => Mode::All,
                    };
                    if mode != self.mode {
                        self.mode = mode;
                        self.reset();
                    }
                }
                "down" | "j" => self.scroll(1),
                "up" | "k" => self.scroll(-1),
                "page_down" | " " => {
                    self.scroll(self.size.height.saturating_sub(4).max(1) as isize)
                }
                "page_up" => self.scroll(-(self.size.height.saturating_sub(4).max(1) as isize)),
                "home" | "g" => self.top = 0,
                "end" | "G" => self.top = self.document.rows(self.split).len().saturating_sub(1),
                "right" | "l" => self.horizontal = self.horizontal.saturating_add(8),
                "left" | "h" => self.horizontal = self.horizontal.saturating_sub(8),
                "]" => self.jump_file(self.current_file() + 1),
                "[" => self.jump_file(self.current_file().saturating_sub(1)),
                "n" => self.jump_hunk(true),
                "p" => self.jump_hunk(false),
                "v" => self.layout = self.layout.next(),
                "f" => self.sidebar = !self.sidebar,
                "r" => self.cancel(),
                "esc" => ctx.close_view()?,
                _ => return Ok(()),
            }
            ctx.redraw();
        } else if e["type"] == "mouse" {
            if e["action"] == "scroll" {
                self.scroll(e["dy"].as_i64().unwrap_or(0).signum() as isize * 3);
                ctx.redraw();
            } else if e["button"] == "left" {
                let identity = (
                    v["frame_id"].as_u64().unwrap_or(0),
                    v["size_revision"].as_u64().unwrap_or(0),
                );
                let target = self
                    .mouse_file(e)
                    .map(|i| (i, self.snapshot.as_ref().unwrap().files[i].path.clone()));
                if e["action"] == "down" {
                    self.pressed = target.map(|(_, path)| (path, identity.0, identity.1));
                } else if e["action"] == "up" {
                    let pressed = self.pressed.take();
                    if let Some((index, path)) = target
                        && pressed == Some((path, identity.0, identity.1))
                    {
                        self.jump_file(index);
                        ctx.redraw();
                    }
                }
            }
        }
        Ok(())
    }
}
impl Drop for DiffPlugin {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
            let _ = job.handle.join();
        }
    }
}
impl Plugin for DiffPlugin {
    fn id(&self) -> &str {
        "diff"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn event(&mut self, event: Event, ctx: &mut Context) -> Result<()> {
        match event {
            Event::Opened(value) => {
                // Moving/focusing a plugin pane can have no source directory.
                // Only a concrete new source replaces the repository being read.
                if let Some(cwd) = value["cwd"].as_str().map(PathBuf::from)
                    && self.cwd.as_ref() != Some(&cwd)
                {
                    self.cwd = Some(cwd);
                    self.reset();
                }
                ctx.redraw();
            }
            Event::Closed => {
                self.visible = false;
                self.cancel();
            }
            Event::Focus(false) => self.pressed = None,
            Event::Tick => self.tick(ctx),
            Event::Input(value) => self.input(value, ctx)?,
            _ => {}
        }
        Ok(())
    }
    fn render(&mut self, area: Rect, theme: &Value) -> Result<Buffer> {
        self.visible = true;
        self.size = area;
        let mut buffer = Buffer::empty(area);
        let color = |name: &str| {
            theme
                .get(name)
                .cloned()
                .and_then(|c| serde_json::from_value(c).ok())
                .map(|c| saddle_plugin_sdk::terminal_color(&c))
                .unwrap_or(Color::Reset)
        };
        buffer.set_style(
            area,
            Style::default().fg(color("text")).bg(color("background")),
        );
        if area.width == 0 || area.height == 0 {
            return Ok(buffer);
        }
        self.sidebar_width = if self.sidebar && area.width >= 85 {
            30
        } else {
            0
        };
        let width = area.width.saturating_sub(self.sidebar_width);
        let split = match self.layout {
            Layout::Auto => width >= 100,
            Layout::Unified => false,
            Layout::Split => width >= 30,
        };
        if split != self.split {
            if let Some(s) = &self.snapshot {
                self.top = view::restore(
                    self.document.rows(self.split),
                    s,
                    self.top,
                    self.document.rows(split),
                    s,
                );
            }
            self.split = split;
        }
        let status = if self.error.is_some() {
            "STALE / error"
        } else if self.snapshot.is_none() {
            "Loading"
        } else {
            "Live"
        };
        let count = self.snapshot.as_ref().map_or(0, |s| s.files.len());
        view::put(
            &mut buffer,
            0,
            0,
            area.width,
            &format!(
                "{}  ·  {count} files  ·  {status}     1 All   2 Unstaged   3 Staged",
                self.mode.label()
            ),
            Style::default().add_modifier(Modifier::BOLD),
        );
        if area.height > 1 {
            let root = self
                .snapshot
                .as_ref()
                .map(|s| &s.root)
                .or(self.cwd.as_ref());
            let text = root
                .map(|p| view::display_path(p))
                .unwrap_or_else(|| "Open Diff from an agent or terminal in a Git worktree.".into());
            view::put(
                &mut buffer,
                0,
                1,
                area.width,
                &text,
                Style::default().fg(Color::DarkGray),
            );
        }
        if area.height > 2
            && let Some(error) = &self.error
        {
            view::put(
                &mut buffer,
                0,
                2,
                area.width,
                error,
                Style::default().fg(Color::Red),
            );
        }
        let height = area.height.saturating_sub(4);
        self.top = self
            .top
            .min(self.document.rows(split).len().saturating_sub(1));
        if self.sidebar_width > 0
            && let Some(s) = &self.snapshot
        {
            for y in 0..height {
                view::put(
                    &mut buffer,
                    self.sidebar_width - 1,
                    y + 3,
                    1,
                    "│",
                    Style::default().fg(Color::DarkGray),
                );
            }
            let current = self.current_file();
            self.sidebar_start = current.saturating_sub(usize::from(height) / 2);
            for (y, (i, file)) in s
                .files
                .iter()
                .enumerate()
                .skip(self.sidebar_start)
                .take(height.into())
                .enumerate()
            {
                let style = if i == current {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                view::put(
                    &mut buffer,
                    0,
                    y as u16 + 3,
                    self.sidebar_width - 2,
                    &format!("{} {}", file.status, view::display_path(&file.path)),
                    style,
                );
            }
        }
        for (y, row) in self
            .document
            .rows(split)
            .iter()
            .skip(self.top)
            .take(height.into())
            .enumerate()
        {
            view::draw_row(
                &mut buffer,
                Rect::new(self.sidebar_width, 3 + y as u16, width, 1),
                row,
                split,
                self.horizontal,
            );
        }
        if count == 0 && self.snapshot.is_some() && height > 0 {
            view::put(
                &mut buffer,
                self.sidebar_width,
                3,
                width,
                "No changes in this mode.",
                Style::default(),
            );
        }
        if area.height >= 4 {
            let footer = format!(
                "↑↓ scroll · [] files · n/p hunks · v {} · f files · r refresh · Esc close   {}/{}",
                self.layout.label(),
                self.top + 1,
                self.document.rows(split).len().max(1)
            );
            view::put(
                &mut buffer,
                0,
                area.height - 1,
                area.width,
                &footer,
                Style::default().fg(Color::DarkGray),
            );
        }
        Ok(buffer)
    }
}
