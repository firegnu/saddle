//! Process plugin SDK. Call `run` before any business code or threads.
use anyhow::{Result, ensure};
use protocol::{Color, Frame, Message, Modifier, Span};
pub use ratatui;
use ratatui::{
    buffer::{Buffer, CellWidth},
    layout::Rect,
    style::{Color as RColor, Modifier as RModifier},
};
pub use saddle_plugin_protocol as protocol;
use serde_json::{Value, json};
use std::io::{BufReader, Write};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn color(c: RColor) -> Color {
    match c {
        RColor::Reset => Color::default(),
        RColor::Indexed(indexed) => Color::Indexed { indexed },
        RColor::Rgb(r, g, b) => Color::Rgb { rgb: [r, g, b] },
        other => Color::Indexed {
            indexed: match other {
                RColor::Black => 0,
                RColor::Red => 1,
                RColor::Green => 2,
                RColor::Yellow => 3,
                RColor::Blue => 4,
                RColor::Magenta => 5,
                RColor::Cyan => 6,
                RColor::Gray => 7,
                RColor::DarkGray => 8,
                RColor::LightRed => 9,
                RColor::LightGreen => 10,
                RColor::LightYellow => 11,
                RColor::LightBlue => 12,
                RColor::LightMagenta => 13,
                RColor::LightCyan => 14,
                _ => 15,
            },
        },
    }
}
pub fn terminal_color(c: &Color) -> RColor {
    match c {
        Color::Default(_) => RColor::Reset,
        Color::Indexed { indexed } => RColor::Indexed(*indexed),
        Color::Rgb { rgb: [r, g, b] } => RColor::Rgb(*r, *g, *b),
    }
}
fn modifiers() -> [(Modifier, RModifier); 6] {
    [
        (Modifier::Bold, RModifier::BOLD),
        (Modifier::Dim, RModifier::DIM),
        (Modifier::Italic, RModifier::ITALIC),
        (Modifier::Underlined, RModifier::UNDERLINED),
        (Modifier::Reversed, RModifier::REVERSED),
        (Modifier::CrossedOut, RModifier::CROSSED_OUT),
    ]
}
pub fn frame(buffer: &Buffer, size_revision: u64, frame_id: u64) -> Result<Frame> {
    let area = buffer.area;
    ensure!(
        usize::from(area.width) * usize::from(area.height) <= protocol::MAX_CELLS,
        "frame_too_large"
    );
    let mut rows = Vec::new();
    for y in area.y..area.bottom() {
        let mut row: Vec<Span> = Vec::new();
        let mut x = area.x;
        while x < area.right() {
            let cell = &buffer[(x, y)];
            let text = cell.symbol();
            ensure!(text.graphemes(true).count() == 1, "unsupported_glyph");
            let width = text.width();
            ensure!(
                (1..=2).contains(&width) && usize::from(cell.cell_width()) == width,
                "unsupported_glyph"
            );
            ensure!(
                usize::from(x) + width <= usize::from(area.right()),
                "unsupported_glyph"
            );
            let mut mods = Vec::new();
            let mut known = RModifier::empty();
            for (m, r) in modifiers() {
                if cell.modifier.contains(r) {
                    mods.push(m);
                    known |= r;
                }
            }
            ensure!(known == cell.modifier, "unsupported_style");
            let span = Span {
                text: text.into(),
                fg: color(cell.fg),
                bg: color(cell.bg),
                modifiers: mods,
            };
            if let Some(last) = row.last_mut().filter(|last| {
                last.fg == span.fg && last.bg == span.bg && last.modifiers == span.modifiers
            }) {
                last.text.push_str(text)
            } else {
                row.push(span)
            }
            x += width as u16;
        }
        rows.push(row);
    }
    let f = Frame {
        panel: "main".into(),
        size_revision,
        frame_id,
        cols: area.width,
        rows_count: area.height,
        rows,
    };
    f.validate()?;
    Ok(f)
}
pub fn buffer(frame: &Frame) -> Result<Buffer> {
    frame.validate()?;
    let mut out = Buffer::empty(Rect::new(0, 0, frame.cols, frame.rows_count));
    for (y, row) in frame.rows.iter().enumerate() {
        let mut x = 0;
        for span in row {
            let mut style = ratatui::style::Style::default()
                .fg(terminal_color(&span.fg))
                .bg(terminal_color(&span.bg));
            for m in &span.modifiers {
                style =
                    style.add_modifier(modifiers().into_iter().find(|(p, _)| p == m).unwrap().1);
            }
            for g in span.text.graphemes(true) {
                out[(x, y as u16)].set_symbol(g).set_style(style);
                let w = g.width() as u16;
                if w == 2 {
                    out[(x + 1, y as u16)].set_symbol(" ").set_style(style);
                }
                x += w;
            }
        }
    }
    Ok(out)
}

/// Business callbacks run on the plugin event thread; move long jobs to your own workers.
/// Input is structured protocol data, never terminal escape sequences.
pub trait Plugin {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn event(&mut self, event: Event, context: &mut Context) -> Result<()>;
    fn render(&mut self, area: Rect, theme: &Value) -> Result<Buffer>;
}
#[derive(Debug)]
pub enum Event {
    Tick,
    Input(Value),
    Focus(bool),
    Closed,
    Notification { id: u64, status: String },
    AttentionOpen(protocol::AttentionOpen),
    AttentionPublished { id: u64, status: String },
}
pub struct Context {
    requests: Vec<(u64, String)>,
    next: u64,
    dirty: bool,
    reserved_keys: Vec<String>,
    attention: Option<(u64, protocol::AttentionSnapshot)>,
}
impl Context {
    /// Keys owned by the host for this view; these never become plugin input.
    pub fn reserved_keys(&self) -> &[String] {
        &self.reserved_keys
    }

    pub fn redraw(&mut self) {
        self.dirty = true;
    }

    /// Replace this plugin's current Attention items. Empty explicitly withdraws them.
    pub fn attention(&mut self, items: Vec<protocol::AttentionItem>) -> Result<u64> {
        let snapshot = protocol::AttentionSnapshot { items };
        snapshot.validate()?;
        self.next += 1;
        self.attention = Some((self.next, snapshot));
        Ok(self.next)
    }

    pub fn notify(&mut self, text: impl Into<String>) -> Result<u64> {
        ensure!(self.requests.len() < 16, "too many notifications");
        let text = text.into();
        ensure!(
            text.len() <= 1024 && !text.chars().any(char::is_control),
            "invalid notification text"
        );
        self.next += 1;
        self.requests.push((self.next, text));
        Ok(self.next)
    }
}
/// Must be called before any other threads: duplicates protocol fds with CLOEXEC,
/// then gives business code null stdin and log stderr as stdout.
pub fn run(factory: impl FnOnce() -> Box<dyn Plugin>) -> Result<()> {
    use std::os::fd::FromRawFd;
    let (input, mut output) = unsafe {
        let input = libc::fcntl(0, libc::F_DUPFD_CLOEXEC, 3);
        ensure!(input >= 0, "duplicate protocol input failed");
        let output = libc::fcntl(1, libc::F_DUPFD_CLOEXEC, 3);
        if output < 0 {
            libc::close(input);
            anyhow::bail!("duplicate protocol output failed")
        }
        let input = std::fs::File::from_raw_fd(input);
        let output = std::fs::File::from_raw_fd(output);
        let null = std::fs::File::open("/dev/null")?;
        use std::os::fd::AsRawFd;
        ensure!(
            libc::dup2(null.as_raw_fd(), 0) >= 0 && libc::dup2(2, 1) >= 0,
            "redirect plugin stdio failed"
        );
        (input, output)
    };
    let mut plugin = factory();
    let mut reader = BufReader::new(input);
    let mut context = Context {
        requests: Vec::new(),
        next: 0,
        dirty: false,
        reserved_keys: Vec::new(),
        attention: None,
    };
    let mut ready = false;
    let mut highest = 0;
    let mut request_id = 0;
    let mut pending = std::collections::BTreeMap::new();
    let mut attention_pending: Option<(u64, u64, std::time::Instant)> = None;
    let mut size = None;
    let mut frame_id = 0;
    let mut focused = false;
    let mut theme = Value::Null;
    loop {
        use std::os::fd::AsRawFd;
        let mut ready_fd = libc::pollfd {
            fd: reader.get_ref().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let readable =
            !reader.buffer().is_empty() || unsafe { libc::poll(&mut ready_fd, 1, 50) } > 0;
        if ready {
            plugin.event(Event::Tick, &mut context)?;
        }
        let mut redraw = std::mem::take(&mut context.dirty);
        if readable {
            let Some(message) = protocol::read(&mut reader)? else {
                break;
            };
            match message {
                Message::Request { id, method, params } => {
                    ensure!(id > highest, "duplicate request");
                    highest = id;
                    let response = match method.as_str() {
                        "initialize" => {
                            ensure!(
                                !ready
                                    && params["protocol_major"] == 1
                                    && params["width_profile"] == protocol::PROFILE,
                                "incompatible handshake"
                            );
                            context.reserved_keys = params["reserved_keys"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|v| v.as_str().map(str::to_owned))
                                .collect();
                            ready = true;
                            theme = params["theme"].clone();
                            Message::response(
                                id,
                                json!({"id":plugin.id(),"version":plugin.version(),"protocol_major":1,"capabilities":protocol::CAPABILITIES.iter().filter(|c| params["capabilities"].as_array().is_some_and(|caps| caps.iter().any(|v| v == **c))).collect::<Vec<_>>(),"width_profile":protocol::PROFILE}),
                            )
                        }
                        "ping" if ready => Message::response(id, json!({})),
                        "shutdown" if ready => {
                            output
                                .write_all(&protocol::encode(&Message::response(id, json!({})))?)?;
                            break;
                        }
                        _ => Message::error(id, "unsupported", "unsupported request"),
                    };
                    output.write_all(&protocol::encode(&response)?)?;
                }
                Message::Response { id, result, error } => {
                    ensure!(id <= request_id, "unknown response");
                    if attention_pending.is_some_and(|(request, _, _)| request == id) {
                        let (_, published, _) = attention_pending.take().unwrap();
                        plugin.event(
                            Event::AttentionPublished {
                                id: published,
                                status: result
                                    .and_then(|v| v["status"].as_str().map(str::to_owned))
                                    .unwrap_or_else(|| error.map_or("unknown".into(), |e| e.code)),
                            },
                            &mut context,
                        )?;
                    } else if let Some((notification, _)) = pending.remove(&id) {
                        plugin.event(
                            Event::Notification {
                                id: notification,
                                status: result
                                    .and_then(|v| v["status"].as_str().map(str::to_owned))
                                    .unwrap_or_else(|| error.map_or("unknown".into(), |e| e.code)),
                            },
                            &mut context,
                        )?;
                    }
                }
                Message::Event { name, data } => {
                    ensure!(ready, "event before handshake");
                    match name.as_str() {
                        "attention.open" => {
                            plugin.event(
                                Event::AttentionOpen(serde_json::from_value(data)?),
                                &mut context,
                            )?;
                            redraw = true;
                        }
                        "panel.open" | "panel.resize" => {
                            if let Some(keys) = data["reserved_keys"].as_array() {
                                context.reserved_keys = keys
                                    .iter()
                                    .filter_map(|v| v.as_str().map(str::to_owned))
                                    .collect();
                            }
                            let cols = data["cols"]
                                .as_u64()
                                .filter(|x| *x <= 65535)
                                .ok_or_else(|| anyhow::anyhow!("invalid cols"))?
                                as u16;
                            let rows = data["rows_count"]
                                .as_u64()
                                .filter(|x| *x <= 65535)
                                .ok_or_else(|| anyhow::anyhow!("invalid rows"))?
                                as u16;
                            size = Some((
                                cols,
                                rows,
                                data["size_revision"]
                                    .as_u64()
                                    .ok_or_else(|| anyhow::anyhow!("missing revision"))?,
                            ));
                            redraw = true;
                        }
                        "panel.close" => {
                            size = None;
                            focused = false;
                            plugin.event(Event::Closed, &mut context)?;
                        }
                        "panel.focus" => {
                            focused = data["focused"]
                                .as_bool()
                                .ok_or_else(|| anyhow::anyhow!("invalid focus"))?;
                            plugin.event(Event::Focus(focused), &mut context)?;
                        }
                        "theme" => {
                            theme = data;
                            redraw = true;
                        }
                        "input" => {
                            if focused
                                && data["frame_id"] == frame_id
                                && size.is_some_and(|(_, _, r)| data["size_revision"] == r)
                            {
                                plugin.event(Event::Input(data), &mut context)?;
                            } else {
                                output.write_all(&protocol::encode(&Message::event("input.rejected",json!({"panel":"main","input_id":data["input_id"],"reason":"stale_frame"})))?)?;
                            }
                        }
                        _ if name.starts_with("optional.") => {}
                        _ => anyhow::bail!("unknown event {name}"),
                    }
                }
            }
        }
        redraw |= std::mem::take(&mut context.dirty);
        for (id, text) in context.requests.drain(..) {
            if pending.len() >= 16 {
                plugin.event(
                    Event::Notification {
                        id,
                        status: "limited".into(),
                    },
                    &mut Context {
                        requests: vec![],
                        next: context.next,
                        dirty: false,
                        reserved_keys: context.reserved_keys.clone(),
                        attention: None,
                    },
                )?;
                continue;
            }
            request_id += 1;
            pending.insert(request_id, (id, std::time::Instant::now()));
            output.write_all(&protocol::encode(&Message::request(
                request_id,
                "notify",
                json!({"notification_id":id,"text":text,"level":"info"}),
            ))?)?;
        }
        if attention_pending.is_some_and(|(_, _, sent)| sent.elapsed().as_secs() >= 5) {
            let (_, id, _) = attention_pending.take().unwrap();
            plugin.event(
                Event::AttentionPublished {
                    id,
                    status: "unknown".into(),
                },
                &mut context,
            )?;
        }
        if ready
            && attention_pending.is_none()
            && let Some((id, snapshot)) = context.attention.take()
        {
            request_id += 1;
            output.write_all(&protocol::encode(&Message::request(
                request_id,
                "attention.replace",
                serde_json::to_value(snapshot)?,
            ))?)?;
            attention_pending = Some((request_id, id, std::time::Instant::now()));
        }
        let expired: Vec<_> = pending
            .iter()
            .filter(|(_, (_, t))| t.elapsed().as_secs() >= 5)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            let (id, _) = pending.remove(&id).unwrap();
            plugin.event(
                Event::Notification {
                    id,
                    status: "unknown".into(),
                },
                &mut context,
            )?;
            redraw = true;
        }
        if redraw
            && let Some((cols, rows, revision)) = size
            && cols > 0
            && rows > 0
        {
            frame_id += 1;
            let result = (|| {
                ensure!(
                    usize::from(cols) * usize::from(rows) <= protocol::MAX_CELLS,
                    "frame_too_large"
                );
                let buf = plugin.render(Rect::new(0, 0, cols, rows), &theme)?;
                ensure!(buf.area == Rect::new(0, 0, cols, rows), "render_failed");
                let f = frame(&buf, revision, frame_id)?;
                protocol::encode(&Message::event("panel.frame", serde_json::to_value(f)?))
            })();
            let bytes=result.unwrap_or_else(|e:anyhow::Error| {let reason=e.to_string();let code=if reason.contains("too large")||reason=="frame_too_large"{"frame_too_large"}else if reason.contains("unsupported_style"){"unsupported_style"}else if reason.contains("glyph"){"unsupported_glyph"}else{"render_failed"};protocol::encode(&Message::event("panel.error",json!({"panel":"main","size_revision":revision,"code":code,"message":reason.chars().take(200).collect::<String>()}))).expect("bounded error")});
            output.write_all(&bytes)?;
        }
        output.flush()?;
    }
    Ok(())
}
