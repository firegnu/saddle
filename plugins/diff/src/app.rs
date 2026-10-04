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
use unicode_width::UnicodeWidthStr;

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
        let (fg, muted, accent) = (color("text"), color("muted"), color("accent"));
        let muted_style = Style::default().fg(muted);
        let (status, status_style) = if self.error.is_some() {
            (
                "STALE / error",
                Style::default()
                    .fg(color("error"))
                    .add_modifier(Modifier::BOLD),
            )
        } else if self.snapshot.is_none() {
            ("Loading", muted_style)
        } else {
            ("Live", Style::default().fg(fg))
        };
        let count = self.snapshot.as_ref().map_or(0, |s| s.files.len());
        // Modes on the left with the current one as a solid bold block; the file count and
        // status sit at the right edge. A narrow pane gives up, in order, the other modes'
        // names, the file count, then the other modes' keys; the current mode and the status
        // are kept.
        let summary = format!("{count} files  ·  ");
        let modes = [Mode::All, Mode::Unstaged, Mode::Staged];
        let labels = |level: u8| -> Vec<(Mode, String)> {
            modes
                .into_iter()
                .enumerate()
                .filter(|&(_, m)| level < 2 || m == self.mode)
                .map(|(i, m)| {
                    let label = if level == 0 || m == self.mode {
                        format!(" {} {} ", i + 1, m.label())
                    } else {
                        format!(" {} ", i + 1)
                    };
                    (m, label)
                })
                .collect()
        };
        let fits = |level: u8, count: bool| {
            let modes = labels(level)
                .iter()
                .map(|(_, l)| l.width() + 1)
                .sum::<usize>();
            let right = status.width() + if count { summary.width() } else { 0 };
            modes + right <= usize::from(area.width)
        };
        let (level, show_count) = [(0, true), (1, true), (1, false), (2, false)]
            .into_iter()
            .find(|&(level, count)| fits(level, count))
            .unwrap_or((2, false));
        let mut x = 0;
        for (mode, label) in labels(level) {
            let style = if mode == self.mode {
                Style::default()
                    .fg(fg)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                muted_style
            };
            view::put(
                &mut buffer,
                x,
                0,
                area.width.saturating_sub(x),
                &label,
                style,
            );
            x = x.saturating_add(label.width() as u16 + 1);
        }
        // When even the current mode and status do not fit, the status starts after the mode.
        let status_x = area.width.saturating_sub(status.width() as u16).max(x);
        if show_count {
            let summary_x = status_x - summary.width() as u16;
            view::put(
                &mut buffer,
                summary_x,
                0,
                summary.width() as u16,
                &summary,
                muted_style,
            );
        }
        view::put(
            &mut buffer,
            status_x,
            0,
            area.width.saturating_sub(status_x),
            status,
            status_style,
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
            view::put(&mut buffer, 0, 1, area.width, &text, muted_style);
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
                Style::default().fg(color("error")),
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
                    muted_style,
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
                let (marker, style) = if i == current {
                    (
                        "▎",
                        Style::default().fg(accent).add_modifier(Modifier::BOLD),
                    )
                } else {
                    (" ", muted_style)
                };
                view::put(
                    &mut buffer,
                    0,
                    y as u16 + 3,
                    self.sidebar_width - 2,
                    &format!("{marker}{} {}", file.status, view::display_path(&file.path)),
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
                (color("background"), muted, fg),
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
            let y = area.height - 1;
            // The row position keeps its own slot at the right edge.
            let position = format!(
                "{}/{}",
                self.top + 1,
                self.document.rows(split).len().max(1)
            );
            let position_x = area.width.saturating_sub(position.width() as u16);
            view::put(
                &mut buffer,
                position_x,
                y,
                area.width,
                &position,
                muted_style,
            );
            let layout = match self.layout {
                Layout::Split if !split => "Split layout (narrow)".into(),
                layout => format!("{} layout", layout.label()),
            };
            let list = if self.sidebar && self.sidebar_width == 0 {
                "list (narrow)"
            } else {
                "file list"
            };
            // (key, action, drop rank): a narrow pane drops whole items, highest rank first.
            let items = [
                ("↑↓", "scroll", 1),
                ("[ ]", "prev/next file", 2),
                ("n/p", "hunk", 5),
                ("v", layout.as_str(), 4),
                ("f", list, 3),
                ("r", "refresh", 6),
                ("Esc", "close", 0),
            ];
            let room = usize::from(position_x.saturating_sub(2));
            let mut keep = [false; 7];
            let mut used = 0;
            let mut order: Vec<usize> = (0..items.len()).collect();
            order.sort_by_key(|&i| items[i].2);
            for i in order {
                let (key, action, _) = items[i];
                let w = key.width() + 1 + action.width() + if used > 0 { 2 } else { 0 };
                if used + w <= room {
                    used += w;
                    keep[i] = true;
                }
            }
            let mut x = 0;
            for (i, (key, action, _)) in items.into_iter().enumerate() {
                if !keep[i] {
                    continue;
                }
                view::put(
                    &mut buffer,
                    x,
                    y,
                    key.width() as u16,
                    key,
                    Style::default().fg(fg),
                );
                x += key.width() as u16 + 1;
                view::put(
                    &mut buffer,
                    x,
                    y,
                    action.width() as u16,
                    action,
                    muted_style,
                );
                x += action.width() as u16 + 2;
            }
        }
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::FileDiff;
    use serde_json::json;

    const BG: Color = Color::Rgb(10, 20, 30);
    const TEXT: Color = Color::Rgb(220, 220, 220);
    const MUTED: Color = Color::Rgb(120, 120, 120);
    const ACCENT: Color = Color::Rgb(240, 200, 60);
    const ERROR: Color = Color::Rgb(230, 80, 80);

    fn theme() -> Value {
        let c = saddle_plugin_sdk::color;
        json!({"text":c(TEXT),"muted":c(MUTED),"background":c(BG),"accent":c(ACCENT),"error":c(ERROR)})
    }
    fn plugin() -> DiffPlugin {
        let snapshot = Snapshot {
            root: "/tmp/synthetic-repo".into(),
            files: vec![
                FileDiff {
                    path: "src/a.rs".into(),
                    status: "M".into(),
                    patch: "@@ -1,2 +1,2 @@\n-fn old() {}\n+fn new() {}\n common()\n".into(),
                },
                FileDiff {
                    path: "notes.txt".into(),
                    status: "?".into(),
                    patch: "@@ -0,0 +1 @@\n+second\n".into(),
                },
            ],
        };
        let mut p = DiffPlugin::new(Some("/tmp/synthetic-repo".into()));
        p.document = Document::build(&snapshot);
        p.snapshot = Some(Arc::new(snapshot));
        p
    }
    fn lines(b: &Buffer) -> Vec<String> {
        (0..b.area.height)
            .map(|y| {
                (0..b.area.width)
                    .map(|x| b[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }
    fn find(b: &Buffer, text: &str) -> Option<(u16, u16)> {
        lines(b).iter().enumerate().find_map(|(y, l)| {
            l.find(text)
                .map(|i| (l[..i].chars().count() as u16, y as u16))
        })
    }
    fn render(p: &mut DiffPlugin, w: u16, h: u16) -> Buffer {
        p.render(Rect::new(0, 0, w, h), &theme()).unwrap()
    }

    fn long_text_plugin() -> DiffPlugin {
        let mut p = plugin();
        let snapshot = Arc::make_mut(p.snapshot.as_mut().unwrap());
        snapshot.root = format!("/tmp/{}repo", "synthetic-directory/".repeat(10)).into();
        snapshot.files[0].path = format!("src/{}file.txt", "nested/".repeat(20)).into();
        snapshot.files[0].patch = format!("@@ -0,0 +1 @@\n+{}END_OF_LINE\n", "界".repeat(100));
        p.document = Document::build(snapshot);
        p.layout = Layout::Unified;
        p
    }

    #[test]
    fn long_paths_and_code_keep_chrome_visible_while_scrolling() {
        for width in [70, 120] {
            let mut p = long_text_plugin();
            let before = render(&mut p, width, 12);
            assert!(lines(&before)[1].starts_with("/tmp/synthetic-directory/"));
            assert!(find(&before, "src/nested/").is_some());
            assert!(find(&before, "界").is_some());
            assert!(find(&before, "END_OF_LINE").is_none());
            p.horizontal = 200; // Display columns, not UTF-8 bytes or character count.
            let after = render(&mut p, width, 12);
            assert!(find(&after, "END_OF_LINE").is_some());
            let before_rows = lines(&before);
            let after_rows = lines(&after);
            assert_eq!(&before_rows[..3], &after_rows[..3]);
            assert_eq!(before_rows[11], after_rows[11]);
            assert!(after_rows[0].contains("1 All") && after_rows[0].contains("Live"));
            assert!(after_rows[11].contains("Esc close"));
            saddle_plugin_sdk::frame(&before, 1, 1).unwrap();
            saddle_plugin_sdk::frame(&after, 1, 1).unwrap();
        }
    }

    #[test]
    fn header_body_and_footer_use_theme_roles_and_keep_states_distinct() {
        let mut p = plugin();
        let b = render(&mut p, 120, 12);
        let cell = |b: &Buffer, text: &str| b[find(b, text).unwrap()].clone();
        // Current mode: solid bold block; other modes stay muted in their original order.
        let all = cell(&b, "1 All");
        assert!(all.modifier.contains(Modifier::BOLD | Modifier::REVERSED));
        assert_eq!(cell(&b, "2 Unstaged").fg, MUTED);
        assert!(find(&b, "1 All").unwrap().0 < find(&b, "2 Unstaged").unwrap().0);
        assert!(find(&b, "2 Unstaged").unwrap().0 < find(&b, "3 Staged").unwrap().0);
        assert_eq!(cell(&b, "/tmp/synthetic-repo").fg, MUTED);
        // Unchanged code sits on the theme background, not the terminal default.
        let (x, y) = find(&b, "common").unwrap();
        assert_eq!(b[(x - 1, y)].bg, BG);
        let current = cell(&b, "▎M src/a.rs");
        assert_eq!((current.fg, current.modifier), (ACCENT, Modifier::BOLD));
        assert!(lines(&b)[11].ends_with("1/10"));
        assert!(lines(&b)[11].contains("[ ] prev/next file"));
        assert!(lines(&b)[11].contains("f file list"));

        p.mode = Mode::Staged;
        p.error = Some("fatal: not a git repository".into());
        let b = render(&mut p, 90, 10);
        assert!(cell(&b, "3 Staged").modifier.contains(Modifier::REVERSED));
        assert!(!cell(&b, "1 All").modifier.contains(Modifier::REVERSED));
        assert_eq!(cell(&b, "STALE / error").fg, ERROR);
        assert_eq!(cell(&b, "fatal").fg, ERROR);
        // Narrow error pane: the current mode and the error status both stay; the file count
        // and the other mode names give way first.
        let b = render(&mut p, 34, 10);
        println!("|{}|", lines(&b)[0]);
        let current = find(&b, "3 Staged").expect("current mode stays visible");
        assert!(b[current].modifier.contains(Modifier::REVERSED));
        assert_eq!(cell(&b, "STALE / error").fg, ERROR);
        assert!(find(&b, "1").unwrap().0 < find(&b, "2").unwrap().0);
        assert!(find(&b, "2").unwrap().0 < current.0);

        // Narrow: the list preference is still on and `f` still toggles it, so it is shown
        // as narrow rather than disabled; Esc and the position stay visible.
        let mut p = plugin();
        let b = render(&mut p, 70, 12);
        let footer = &lines(&b)[11];
        assert!(footer.contains("f list (narrow)") && footer.contains("Esc close"));
        assert!(footer.ends_with("1/10"));
        assert!(find(&b, "Live").is_some());
        for (w, h) in (1..130).flat_map(|w| (1..7).map(move |h| (w, h))) {
            saddle_plugin_sdk::frame(&render(&mut p, w, h), 1, 1).unwrap();
        }
    }

    /// Unchanged code on the default theme (Reset background, not inferable) and on explicit
    /// light and mid-gray backgrounds. Prints each syntax color's contrast: `cargo test -p saddle-diff-plugin
    /// light_theme -- --nocapture`.
    #[test]
    fn light_theme_keeps_unchanged_code_readable() {
        fn linear(c: Color) -> [f64; 3] {
            let Color::Rgb(r, g, b) = c else {
                unreachable!()
            };
            [r, g, b].map(|v| {
                let v = f64::from(v) / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            })
        }
        fn lum(c: Color) -> f64 {
            let [r, g, b] = linear(c);
            0.2126 * r + 0.7152 * g + 0.0722 * b
        }
        // CIELAB (D65), for the perceived difference between two syntax classes.
        fn lab(c: Color) -> [f64; 3] {
            let [r, g, b] = linear(c);
            let f = |t: f64| {
                if t > 0.008856 {
                    t.cbrt()
                } else {
                    7.787 * t + 16.0 / 116.0
                }
            };
            let x = f((0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047);
            let y = f(0.2126 * r + 0.7152 * g + 0.0722 * b);
            let z = f((0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883);
            [116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z)]
        }
        let contrast = |a: Color, b: Color| {
            let (x, y) = (lum(a), lum(b));
            (x.max(y) + 0.05) / (x.min(y) + 0.05)
        };
        let c = saddle_plugin_sdk::color;
        let dune = json!({"text":c(Color::Reset),"muted":c(Color::Gray),"background":c(Color::Reset),"accent":c(Color::Yellow),"error":c(Color::Red)});
        let light_bg = Color::Rgb(0xf6, 0xf6, 0xf4);
        let light_text = Color::Rgb(0x24, 0x29, 0x2f);
        let light_accent = Color::Rgb(0x09, 0x69, 0xda);
        let light_theme = |bg: Color| json!({"text":c(light_text),"muted":c(Color::Rgb(0x6e,0x77,0x81)),"background":c(bg),"accent":c(light_accent),"error":c(Color::Rgb(0xcf,0x22,0x2e))});
        // A mid-gray background is still light, but too dark for 7:1 against any color.
        let mid_bg = Color::Rgb(0x90, 0x90, 0x90);
        let snapshot = Snapshot {
            root: "/tmp/synthetic-repo".into(),
            files: vec![FileDiff {
                path: "src/config.rs".into(),
                status: "M".into(),
                patch: "@@ -1,7 +1,7 @@\n // Read the limit from the config file.\n use std::fs;\n-const LIMIT: u32 = 10;\n+const LIMIT: u32 = 20;\n pub fn load(path: &str) -> String {\n     let text = fs::read_to_string(path).unwrap_or_default();\n     format!(\"{text} {}\", 42)\n"
                    .into(),
            }],
        };
        let mut p = DiffPlugin::new(Some("/tmp/synthetic-repo".into()));
        p.document = Document::build(&snapshot);
        p.snapshot = Some(Arc::new(snapshot));
        let mut base = (0, 0.0);
        // Smallest CIELAB difference (ΔE76) between two syntax classes, to catch classes merging.
        let closest = |seen: &[(Color, String)]| {
            let mut min = (f64::MAX, String::new());
            for (i, (a, ta)) in seen.iter().enumerate() {
                for (b, tb) in &seen[i + 1..] {
                    let (a, b) = (lab(*a), lab(*b));
                    let d = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
                    if d < min.0 {
                        min = (d, format!("{ta} / {tb}"));
                    }
                }
            }
            println!("  closest classes: ΔE {:.1} ({})", min.0, min.1);
            min.0
        };
        for (name, theme, bg) in [
            ("default (Dune)", dune, None),
            ("explicit light", light_theme(light_bg), Some(light_bg)),
            ("explicit mid gray", light_theme(mid_bg), Some(mid_bg)),
        ] {
            let b = p.render(Rect::new(0, 0, 100, 14), &theme).unwrap();
            println!("--- {name}");
            for l in lines(&b) {
                println!("|{l}|");
            }
            // Code cells of unchanged rows: right of the 8-column number gutter, theme background.
            let mut seen: Vec<(Color, String)> = vec![];
            for y in 3..b.area.height - 1 {
                let gutter: String = lines(&b)[y as usize].chars().skip(30).take(5).collect();
                if gutter.trim().parse::<usize>().is_err() {
                    continue;
                }
                for x in 38..b.area.width {
                    let cell = &b[(x, y)];
                    let changed = [Color::Rgb(52, 28, 30), Color::Rgb(23, 48, 35)];
                    if cell.symbol().trim().is_empty() || changed.contains(&cell.bg) {
                        continue;
                    }
                    match seen.iter_mut().find(|(fg, _)| *fg == cell.fg) {
                        Some((_, text)) => text.push_str(cell.symbol()),
                        None => seen.push((cell.fg, cell.symbol().into())),
                    }
                }
            }
            for (fg, text) in &seen {
                let cell = b[find(&b, "use std").unwrap()].clone();
                match (fg, cell.bg) {
                    (Color::Rgb(..), Color::Rgb(..)) => {
                        println!("  fg={fg:?} contrast={:.2} {text}", contrast(*fg, cell.bg))
                    }
                    _ => println!("  fg={fg:?} bg={:?}: not inferable {text}", cell.bg),
                }
            }
            let distance = closest(&seen);
            if let Some(bg) = bg {
                for (fg, text) in &seen {
                    assert!(
                        contrast(*fg, bg) >= 4.5,
                        "{fg:?} on {bg:?} is unreadable: {text}"
                    );
                }
                // Syntax classes stay as distinct as on the default theme.
                assert_eq!(seen.len(), base.0, "syntax classes merge on {bg:?}");
                assert!(distance >= base.1 / 2.0, "syntax classes merge on {bg:?}");
                // Changed rows keep their red/green backgrounds and word emphasis.
                let (x, y) = find(&b, "20").unwrap();
                assert_eq!(b[(x, y)].bg, Color::Rgb(23, 48, 35));
                assert!(
                    b[(x, y)]
                        .modifier
                        .contains(Modifier::BOLD | Modifier::UNDERLINED)
                );
                assert_eq!(b[find(&b, "10").unwrap()].bg, Color::Rgb(52, 28, 30));
                // Current mode and current file cues stay.
                assert!(
                    b[find(&b, "1 All").unwrap()]
                        .modifier
                        .contains(Modifier::REVERSED)
                );
                let current = b[find(&b, "▎M src/config.rs").unwrap()].clone();
                assert_eq!(
                    (current.fg, current.modifier),
                    (light_accent, Modifier::BOLD)
                );
            } else {
                // Reset background: syntax colors are left exactly as highlighted.
                let highlighted: Vec<_> = p
                    .document
                    .unified
                    .iter()
                    .flat_map(|r| r.left.iter().chain(&r.right))
                    .flat_map(|l| l.spans.iter().map(|(_, s)| s.fg))
                    .collect();
                assert!(seen.iter().all(|(fg, _)| highlighted.contains(&Some(*fg))));
                base = (seen.len(), distance);
            }
        }
    }

    /// Prints synthetic frames for review: `cargo test -p saddle-diff-plugin dump -- --nocapture`.
    #[test]
    fn dump_synthetic_frames() {
        let scenes: Vec<(&str, DiffPlugin, u16, u16)> = vec![
            ("wide, file list", plugin(), 120, 12),
            ("narrow, file list preference on", plugin(), 70, 12),
            (
                "error",
                {
                    let mut p = plugin();
                    p.error = Some("fatal: not a git repository".into());
                    p
                },
                90,
                10,
            ),
            ("loading", DiffPlugin::new(Some("/tmp/x".into())), 60, 6),
            ("tiny", plugin(), 34, 8),
            ("long paths and code", long_text_plugin(), 70, 12),
            (
                "long code scrolled",
                {
                    let mut p = long_text_plugin();
                    p.horizontal = 200;
                    p
                },
                70,
                12,
            ),
        ];
        for (name, mut p, w, h) in scenes {
            let b = render(&mut p, w, h);
            println!("--- {name} ({w}x{h})");
            for l in lines(&b) {
                println!("|{l}|");
            }
            let probe = |label: &str, at: Option<(u16, u16)>| {
                if let Some((x, y)) = at {
                    let c = &b[(x, y)];
                    println!(
                        "  {label}: fg={:?} bg={:?} mod={:?}",
                        c.fg, c.bg, c.modifier
                    );
                }
            };
            probe("mode All", find(&b, "All"));
            probe("mode Unstaged", find(&b, "Unstaged"));
            probe(
                "status",
                find(&b, "Live")
                    .or(find(&b, "STALE"))
                    .or(find(&b, "Loading")),
            );
            probe("path", find(&b, "/tmp/"));
            probe("error", find(&b, "fatal"));
            probe(
                "unchanged code",
                find(&b, "common").map(|(x, y)| (x.saturating_sub(1), y)),
            );
            probe("current file", find(&b, "M src/a.rs"));
        }
    }
}
