//! Host Telemetry page (docs/遥测查询与Drover接入设计.md §2–§4): a read-only view of
//! `telemetry::Store` — trace list → trace detail → full body. Each read runs on its own thread
//! and comes back with its token; a layer keeps only the token it is waiting for, so a result
//! for a layer that was left or queried again is dropped. Reads happen only on open, filter,
//! paging and refresh. Text from storage is shown with control characters made visible.
use crate::{
    buttons::{Hit, Pointer},
    input::Focus,
    launch::edit::Input,
    telemetry::{BindingFilter, EventQuery, Store},
    theme::Theme,
    ui::clip,
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use serde_json::Value;
use std::{
    fmt::Write as _,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Events per page; later pages keep the first page's upper bound.
const PAGE: i64 = 100;
/// Windows at least this wide put the event detail beside the timeline.
const WIDE: u16 = 140;

pub enum Outcome {
    Stay,
    Close,
}

#[derive(Clone)]
struct Failure {
    code: String,
    message: String,
}
impl From<crate::telemetry::Error> for Failure {
    fn from(e: crate::telemetry::Error) -> Self {
        Self {
            code: e.code.into(),
            message: e.message,
        }
    }
}
impl Failure {
    fn text(&self) -> String {
        format!("{}: {}", inert(&self.code), inert(&self.message))
    }
}

enum Data {
    Value(Value),
    Bytes(Vec<u8>),
}
struct Reply {
    token: u64,
    at: String,
    result: Result<Data, Failure>,
}

/// One read: waiting for `token`, or its result and when it was queried.
enum Load<T> {
    Pending(u64),
    Done {
        at: String,
        result: Result<T, Failure>,
    },
}
impl<T> Load<T> {
    fn waits(&self, token: u64) -> bool {
        matches!(self, Load::Pending(t) if *t == token)
    }
    fn ready(&self) -> Option<&T> {
        match self {
            Load::Done { result: Ok(v), .. } => Some(v),
            _ => None,
        }
    }
    fn at(&self) -> &str {
        match self {
            Load::Done { at, .. } => at,
            Load::Pending(_) => "",
        }
    }
}

struct Form {
    fields: [Input; 4],
    focus: usize,
    areas: [Rect; 4],
    error: String,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Event,
    Ops,
    Intervals,
}
struct Detail {
    trace: Value,
    summary: Load<Value>,
    /// The selected dispatch's ID; None is All. Kept by ID so a summary being read again
    /// cannot turn it into All or another dispatch.
    dispatch: Option<String>,
    events: Vec<Value>,
    upper: Option<i64>,
    next_after: i64,
    more: bool,
    page: Option<Load<Value>>,
    selected: usize,
    top: usize,
    panel: Panel,
    panel_top: usize,
    picking: Option<usize>,
    message: String,
    reader: Option<Reader>,
    /// A `carried_from` target of the selected event, outside the loaded events: its event
    /// ID and the read of where it is.
    target: Option<(String, Load<Value>)>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Text,
    Hex,
    Lossy,
}
struct Reader {
    seq: String,
    label: String,
    body: Value,
    load: Load<Vec<u8>>,
    utf8: bool,
    view: View,
    display: String,
    wrap: bool,
    top: usize,
    left: usize,
    rows: Vec<(usize, usize)>,
    laid_out: Option<(u16, bool, View)>,
    height: usize,
}

pub struct Page {
    store: Result<Arc<Store>, Failure>,
    sender: Sender<Reply>,
    receiver: Receiver<Reply>,
    next: u64,
    settings: Load<Value>,
    filter: Option<BindingFilter>,
    source: Option<String>,
    list: Load<Value>,
    selected: usize,
    top: usize,
    form: Option<Form>,
    detail: Option<Detail>,
    rows: Vec<(Rect, usize)>,
    list_area: Rect,
    panel_area: Rect,
    list_height: usize,
    panel_height: usize,
    controls: Vec<(Focus, Hit)>,
    pointer: Pointer,
}

impl Page {
    /// Opens the list, optionally narrowed to an opaque binding (`source` names who asked).
    /// Nothing is created when no telemetry was ever recorded.
    pub fn open(
        store: crate::telemetry::Result<Store>,
        filter: Option<BindingFilter>,
        source: Option<String>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut page = Self {
            store: store.map(Arc::new).map_err(Failure::from),
            sender,
            receiver,
            next: 0,
            settings: Load::Pending(0),
            filter,
            source,
            list: Load::Pending(0),
            selected: 0,
            top: 0,
            form: None,
            detail: None,
            rows: Vec::new(),
            list_area: Rect::default(),
            panel_area: Rect::default(),
            list_height: 1,
            panel_height: 1,
            controls: Vec::new(),
            pointer: Pointer::default(),
        };
        page.read_settings();
        page.read_list();
        page
    }
    /// Takes the results that arrived; anything no layer waits for is dropped.
    pub fn poll(&mut self) {
        while let Ok(reply) = self.receiver.try_recv() {
            self.absorb(reply);
        }
        self.sync_target();
    }
    /// Whether a read is still running for what is open.
    pub fn loading(&self) -> bool {
        let detail = self.detail.as_ref().is_some_and(|d| {
            matches!(d.summary, Load::Pending(_))
                || matches!(d.page, Some(Load::Pending(_)))
                || matches!(d.target, Some((_, Load::Pending(_))))
                || d.reader
                    .as_ref()
                    .is_some_and(|r| matches!(r.load, Load::Pending(_)))
        });
        matches!(self.settings, Load::Pending(_)) || matches!(self.list, Load::Pending(_)) || detail
    }
    /// The status bar text while the page has input.
    pub fn status(&self) -> &'static str {
        if self.detail.is_some() || self.form.is_some() {
            " Input ▸ Telemetry · Esc Back"
        } else {
            " Input ▸ Telemetry · Esc Close"
        }
    }

    fn start(
        &mut self,
        job: impl FnOnce(&Store) -> crate::telemetry::Result<Data> + Send + 'static,
    ) -> u64 {
        self.next += 1;
        let token = self.next;
        let sender = self.sender.clone();
        match &self.store {
            Ok(store) => {
                let store = store.clone();
                thread::spawn(move || {
                    let at = clock();
                    let result = job(&store).map_err(Failure::from);
                    let _ = sender.send(Reply { token, at, result });
                });
            }
            Err(failure) => {
                let _ = sender.send(Reply {
                    token,
                    at: clock(),
                    result: Err(failure.clone()),
                });
            }
        }
        token
    }
    fn read_settings(&mut self) {
        self.settings = Load::Pending(self.start(|s| s.settings().map(Data::Value)));
    }
    fn read_list(&mut self) {
        let filter = self.filter.as_ref().map(copy);
        self.list =
            Load::Pending(self.start(move |s| s.list_bound(filter.as_ref()).map(Data::Value)));
    }
    fn read_summary(&mut self) {
        let Some(id) = self
            .detail
            .as_ref()
            .and_then(|d| d.trace["trace_id"].as_str())
            .map(str::to_owned)
        else {
            return;
        };
        let token = self.start(move |s| s.show(&id).map(Data::Value));
        self.detail.as_mut().unwrap().summary = Load::Pending(token);
    }
    /// The first page under a new upper bound.
    fn read_events(&mut self) {
        let Some(d) = &mut self.detail else {
            return;
        };
        d.events.clear();
        d.upper = None;
        d.next_after = 0;
        d.more = false;
        self.read_page();
    }
    fn read_page(&mut self) {
        let Some(d) = &self.detail else {
            return;
        };
        let query = EventQuery {
            trace_id: d.trace["trace_id"].as_str().map(str::to_owned),
            dispatch_id: d.dispatch_id().map(str::to_owned),
            after_seq: d.next_after,
            upper_seq: d.upper,
            limit: PAGE,
        };
        let token = self.start(move |s| s.events(&query).map(Data::Value));
        self.detail.as_mut().unwrap().page = Some(Load::Pending(token));
    }
    fn read_body(&mut self) {
        let Some(hash) = self
            .detail
            .as_ref()
            .and_then(|d| d.reader.as_ref())
            .and_then(|r| r.body["sha256"].as_str())
            .map(str::to_owned)
        else {
            return;
        };
        let token = self.start(move |s| s.body(&hash).map(Data::Bytes));
        if let Some(reader) = self.detail.as_mut().and_then(|d| d.reader.as_mut()) {
            reader.load = Load::Pending(token);
        }
    }

    fn absorb(&mut self, reply: Reply) {
        let Reply { token, at, result } = reply;
        let value = |result: Result<Data, Failure>| {
            result.map(|data| match data {
                Data::Value(v) => v,
                Data::Bytes(_) => Value::Null,
            })
        };
        if self.settings.waits(token) {
            self.settings = Load::Done {
                at,
                result: value(result),
            };
            return;
        }
        if self.list.waits(token) {
            let mut result = value(result);
            // Newest first, as the list is read top-down.
            if let Ok(list) = &mut result
                && let Some(traces) = list["traces"].as_array_mut()
            {
                traces.reverse();
            }
            self.list = Load::Done { at, result };
            self.selected = self.selected.min(self.traces().len().saturating_sub(1));
            return;
        }
        let Some(d) = &mut self.detail else {
            return;
        };
        if d.summary.waits(token) {
            d.summary = Load::Done {
                at,
                result: value(result),
            };
            return;
        }
        if d.page.as_ref().is_some_and(|p| p.waits(token)) {
            match value(result) {
                Ok(page) => {
                    d.events
                        .extend(page["events"].as_array().cloned().unwrap_or_default());
                    d.upper = d.upper.or(page["upper_seq"].as_i64());
                    d.next_after = page["next_after_seq"].as_i64().unwrap_or(d.next_after);
                    d.more = page["has_more"] == true;
                    d.page = None;
                }
                Err(failure) => {
                    d.page = Some(Load::Done {
                        at,
                        result: Err(failure),
                    })
                }
            }
            return;
        }
        if let Some((_, target)) = &mut d.target
            && target.waits(token)
        {
            *target = Load::Done {
                at,
                result: value(result),
            };
            return;
        }
        if let Some(reader) = &mut d.reader
            && reader.load.waits(token)
        {
            let result = result.map(|data| match data {
                Data::Bytes(b) => b,
                Data::Value(_) => Vec::new(),
            });
            if let Ok(bytes) = &result {
                reader.utf8 = std::str::from_utf8(bytes).is_ok();
                reader.view = if reader.utf8 { View::Text } else { View::Hex };
                reader.display = std::str::from_utf8(bytes)
                    .map(escape_body)
                    .unwrap_or_default();
                reader.laid_out = None;
            }
            reader.load = Load::Done { at, result };
        }
    }

    fn traces(&self) -> &[Value] {
        self.list
            .ready()
            .and_then(|l| l["traces"].as_array())
            .map_or(&[], Vec::as_slice)
    }

    pub fn event(&mut self, event: &Event) -> Outcome {
        let outcome = self.handle(event);
        self.sync_target();
        outcome
    }
    /// Reads where the selected event's `carried_from` target lives, only for a target
    /// outside the loaded events and only when the selection points at a new one.
    fn sync_target(&mut self) {
        let Some(d) = &self.detail else {
            return;
        };
        let wanted = d.events.get(d.selected).and_then(|event| {
            event["links"]
                .as_array()?
                .iter()
                .filter(|l| l["relation"] == "carried_from")
                .map(target_id)
                .find(|id| !d.events.iter().any(|e| e["event_id"] == *id))
                .map(str::to_owned)
        });
        if d.target.as_ref().map(|(id, _)| id) == wanted.as_ref() {
            return;
        }
        let token = wanted
            .clone()
            .map(|id| self.start(move |s| s.show(&id).map(Data::Value)));
        self.detail.as_mut().unwrap().target = wanted.zip(token.map(Load::Pending));
    }
    fn handle(&mut self, event: &Event) -> Outcome {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.pointer.cancel();
                self.key(*key)
            }
            Event::Paste(text) => {
                if let Some(form) = &mut self.form
                    && text.len() <= 65536
                {
                    form.fields[form.focus].insert(text, false);
                }
                Outcome::Stay
            }
            Event::Mouse(mouse) => {
                let captured = self.pointer.captured();
                if let Some((_, key)) = self.pointer.event(*mouse, &self.controls) {
                    return self.key(key);
                }
                if captured || self.pointer.captured() {
                    return Outcome::Stay;
                }
                let point = Position::new(mouse.column, mouse.row);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => self.click(point),
                    MouseEventKind::ScrollUp => self.scroll(point, -1),
                    MouseEventKind::ScrollDown => self.scroll(point, 1),
                    _ => {}
                }
                Outcome::Stay
            }
            _ => Outcome::Stay,
        }
    }
    fn click(&mut self, point: Position) {
        if let Some(form) = &mut self.form {
            if let Some(i) = form.areas.iter().position(|a| a.contains(point)) {
                form.focus = i;
                form.fields[i].click(point);
            }
            return;
        }
        let Some(&(_, i)) = self.rows.iter().find(|(r, _)| r.contains(point)) else {
            return;
        };
        match &mut self.detail {
            None => self.selected = i,
            Some(d) if d.reader.is_some() => {}
            Some(d) => match &mut d.picking {
                Some(choice) => *choice = i,
                None => {
                    if d.selected != i {
                        d.panel_top = 0;
                    }
                    d.selected = i;
                }
            },
        }
    }
    fn scroll(&mut self, point: Position, delta: isize) {
        let code = if delta < 0 {
            KeyCode::Up
        } else {
            KeyCode::Down
        };
        if let Some(reader) = self.detail.as_mut().and_then(|d| d.reader.as_mut()) {
            reader.top = reader.top.saturating_add_signed(delta * 3);
            return;
        }
        if let Some(d) = &mut self.detail
            && d.picking.is_none()
            && self.panel_area.contains(point)
        {
            d.panel_top = d.panel_top.saturating_add_signed(delta);
            return;
        }
        if self.list_area.contains(point) || self.panel_area.contains(point) {
            self.key(KeyEvent::new(code, KeyModifiers::NONE));
        }
    }

    fn key(&mut self, key: KeyEvent) -> Outcome {
        if self.detail.is_some() {
            return self.detail_key(key);
        }
        if let Some(form) = &mut self.form {
            match key.code {
                KeyCode::Esc => self.form = None,
                KeyCode::Tab => form.focus = (form.focus + 1) % 4,
                KeyCode::BackTab => form.focus = (form.focus + 3) % 4,
                KeyCode::Enter => {
                    let [kind, scope, key, run] = form.fields.each_ref().map(|f| f.text.clone());
                    // The same rule as the Store's binding: kind, scope and key, run optional.
                    if kind.is_empty() || scope.is_empty() || key.is_empty() {
                        form.error = "kind, scope and key are required".into();
                    } else {
                        self.filter = Some(BindingFilter {
                            kind,
                            scope,
                            key,
                            run: (!run.is_empty()).then_some(run),
                        });
                        self.source = None;
                        self.form = None;
                        self.selected = 0;
                        self.read_list();
                    }
                }
                code => {
                    form.fields[form.focus].key(code, false);
                    form.error.clear();
                }
            }
            return Outcome::Stay;
        }
        let count = self.traces().len();
        match key.code {
            KeyCode::Esc => return Outcome::Close,
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(count.saturating_sub(1))
            }
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(self.list_height),
            KeyCode::PageDown => {
                self.selected = (self.selected + self.list_height).min(count.saturating_sub(1))
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = count.saturating_sub(1),
            KeyCode::Enter => {
                if let Some(trace) = self.traces().get(self.selected).cloned() {
                    self.detail = Some(Detail {
                        trace,
                        summary: Load::Pending(0),
                        dispatch: None,
                        events: Vec::new(),
                        upper: None,
                        next_after: 0,
                        more: false,
                        page: None,
                        selected: 0,
                        top: 0,
                        panel: Panel::Event,
                        panel_top: 0,
                        picking: None,
                        message: String::new(),
                        reader: None,
                        target: None,
                    });
                    self.read_summary();
                    self.read_events();
                }
            }
            KeyCode::Char('f') => {
                let f = self.filter.as_ref();
                let value = |get: fn(&BindingFilter) -> Option<&String>| {
                    Input::new(f.and_then(get).cloned().unwrap_or_default())
                };
                self.form = Some(Form {
                    fields: [
                        value(|f| Some(&f.kind)),
                        value(|f| Some(&f.scope)),
                        value(|f| Some(&f.key)),
                        value(|f| f.run.as_ref()),
                    ],
                    focus: 0,
                    areas: [Rect::default(); 4],
                    error: String::new(),
                });
            }
            KeyCode::Char('F') => {
                self.filter = None;
                self.source = None;
                self.selected = 0;
                self.read_list();
            }
            KeyCode::Char('r') => {
                self.read_settings();
                self.read_list();
            }
            _ => {}
        }
        Outcome::Stay
    }

    fn detail_key(&mut self, key: KeyEvent) -> Outcome {
        let d = self.detail.as_mut().unwrap();
        if let Some(reader) = &mut d.reader {
            let page = reader.height.max(1);
            match key.code {
                KeyCode::Esc => d.reader = None,
                KeyCode::Up | KeyCode::Char('k') => reader.top = reader.top.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => reader.top += 1,
                KeyCode::PageUp => reader.top = reader.top.saturating_sub(page),
                KeyCode::PageDown | KeyCode::Char(' ') => reader.top += page,
                KeyCode::Home | KeyCode::Char('g') => reader.top = 0,
                KeyCode::End | KeyCode::Char('G') => reader.top = usize::MAX,
                KeyCode::Left => reader.left = reader.left.saturating_sub(8),
                KeyCode::Right if !reader.wrap => reader.left += 8,
                KeyCode::Char('w') => {
                    reader.wrap = !reader.wrap;
                    reader.left = 0;
                }
                KeyCode::Char('x') => reader.toggle_view(),
                KeyCode::Char('r') => self.read_body(),
                _ => {}
            }
            return Outcome::Stay;
        }
        if let Some(choice) = &mut d.picking {
            let count = d
                .events
                .get(d.selected)
                .and_then(|e| e["bodies"].as_array())
                .map_or(0, Vec::len);
            match key.code {
                KeyCode::Esc => d.picking = None,
                KeyCode::Up | KeyCode::Char('k') => *choice = choice.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    *choice = (*choice + 1).min(count.saturating_sub(1))
                }
                KeyCode::Enter => {
                    let choice = *choice;
                    d.picking = None;
                    self.open_body(choice);
                }
                _ => {}
            }
            return Outcome::Stay;
        }
        d.message.clear();
        let before = d.selected;
        let count = d.events.len();
        let scrolls = d.panel != Panel::Event;
        let step = if scrolls {
            self.panel_height
        } else {
            self.list_height
        };
        let (position, last) = if scrolls {
            (&mut d.panel_top, usize::MAX)
        } else {
            (&mut d.selected, count.saturating_sub(1))
        };
        match key.code {
            KeyCode::Esc => {
                self.detail = None;
                return Outcome::Stay;
            }
            KeyCode::Up | KeyCode::Char('k') => *position = position.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => *position = (*position + 1).min(last),
            KeyCode::PageUp => *position = position.saturating_sub(step.max(1)),
            KeyCode::PageDown => *position = (*position + step.max(1)).min(last),
            KeyCode::Home => *position = 0,
            KeyCode::End => *position = last,
            KeyCode::Enter => {
                let Some(event) = d.events.get(d.selected) else {
                    return Outcome::Stay;
                };
                match event["bodies"].as_array().map_or(0, Vec::len) {
                    0 => {
                        let gaps = gaps(event);
                        d.message = if gaps.is_empty() {
                            "This event has no body.".into()
                        } else {
                            format!("Not captured: {gaps}")
                        };
                    }
                    1 => self.open_body(0),
                    _ => d.picking = Some(0),
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                let Some(ids) = d.summary.ready().map(|s| {
                    dispatches(&s["record"])
                        .iter()
                        .filter_map(|x| x["dispatch_id"].as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                }) else {
                    d.message = "Dispatches are not loaded yet.".into();
                    return Outcome::Stay;
                };
                // Position 0 is All, then each dispatch.
                let count = ids.len() + 1;
                let at = d
                    .dispatch
                    .as_ref()
                    .and_then(|id| ids.iter().position(|x| x == id))
                    .map_or(0, |i| i + 1);
                let next = if key.code == KeyCode::Tab {
                    (at + 1) % count
                } else {
                    (at + count - 1) % count
                };
                d.dispatch = next.checked_sub(1).map(|i| ids[i].clone());
                d.selected = 0;
                d.top = 0;
                self.read_events();
            }
            KeyCode::Char('o') => {
                d.panel = if d.panel == Panel::Ops {
                    Panel::Event
                } else {
                    Panel::Ops
                };
                d.panel_top = 0;
            }
            KeyCode::Char('i') => {
                d.panel = if d.panel == Panel::Intervals {
                    Panel::Event
                } else {
                    Panel::Intervals
                };
                d.panel_top = 0;
            }
            KeyCode::Char('n') if d.more && d.page.is_none() => self.read_page(),
            KeyCode::Char('r') => {
                d.target = None;
                self.read_summary();
                self.read_events();
            }
            // The detail panel scrolls on its own; the event selection stays.
            KeyCode::Char('J') => d.panel_top += 1,
            KeyCode::Char('K') => d.panel_top = d.panel_top.saturating_sub(1),
            _ => {}
        }
        if let Some(d) = &mut self.detail
            && !scrolls
            && d.selected != before
        {
            d.panel_top = 0;
        }
        Outcome::Stay
    }
    fn open_body(&mut self, index: usize) {
        let d = self.detail.as_mut().unwrap();
        let Some(event) = d.events.get(d.selected) else {
            return;
        };
        let Some(body) = event["bodies"].get(index).cloned() else {
            return;
        };
        d.reader = Some(Reader {
            seq: text(&event["seq"]),
            label: body_label(event, &body),
            body,
            load: Load::Pending(0),
            utf8: true,
            view: View::Text,
            display: String::new(),
            wrap: true,
            top: 0,
            left: 0,
            rows: Vec::new(),
            laid_out: None,
            height: 1,
        });
        self.read_body();
    }

    pub fn draw(&mut self, t: &Theme, frame: &mut Frame, area: Rect) {
        self.rows.clear();
        self.controls.clear();
        self.list_area = Rect::default();
        self.panel_area = Rect::default();
        frame.render_widget(Clear, area);
        let label = |trace: &Value| clip(&inert(trace["label"].as_str().unwrap_or("")), 40);
        let (title, right) = match &self.detail {
            None => (" Telemetry ".to_string(), format!(" {} ", self.recording())),
            Some(d) => match &d.reader {
                None => (format!(" Telemetry ▸ {} ", label(&d.trace)), String::new()),
                Some(r) => (
                    format!(
                        " Telemetry ▸ {} ▸ seq {} ▸ {} ",
                        label(&d.trace),
                        r.seq,
                        r.label
                    ),
                    String::new(),
                ),
            },
        };
        let block = t
            .block(title, true)
            .title_top(Line::from(right).right_aligned())
            .style(t.base().bg(t.overlay));
        frame.render_widget(block, area);
        let inside = crate::ui::inner(area);
        let inside = Rect {
            x: inside.x + 1.min(inside.width),
            width: inside.width.saturating_sub(2),
            ..inside
        };
        if inside.is_empty() {
            return;
        }
        let help = self.help();
        let help_rows = bar_rows(&help, inside.width).len() as u16;
        let body = Rect {
            height: inside.height.saturating_sub(help_rows),
            ..inside
        };
        let bar = Rect {
            y: body.bottom(),
            height: inside.height - body.height,
            ..inside
        };
        match &self.detail {
            None => self.draw_list(t, frame, body),
            Some(d) if d.reader.is_some() => self.draw_reader(t, frame, body),
            Some(_) => self.draw_detail(t, frame, body),
        }
        self.draw_help(t, frame, bar, &help);
        self.pointer.paint(t, frame, &self.controls);
    }
    fn recording(&self) -> String {
        match &self.settings {
            Load::Pending(_) => "Recording: …".into(),
            Load::Done { result: Err(_), .. } => "Recording: Unknown".into(),
            Load::Done { result: Ok(s), .. } if s["initialized"] != true => {
                "Recording: Off (not initialized)".into()
            }
            Load::Done { result: Ok(s), .. } => format!(
                "Recording: {} (global)",
                if s["enabled"] == true { "On" } else { "Off" }
            ),
        }
    }
    fn help(&self) -> Vec<(&'static str, &'static str, Option<KeyCode>)> {
        use KeyCode as K;
        let Some(d) = &self.detail else {
            if self.form.is_some() {
                return vec![
                    ("Tab", "Field", Some(K::Tab)),
                    ("↵", "Apply", Some(K::Enter)),
                    ("Esc", "Cancel", Some(K::Esc)),
                ];
            }
            return vec![
                ("↑↓", "Select", None),
                ("↵", "Open", Some(K::Enter)),
                ("f", "Filter", Some(K::Char('f'))),
                ("F", "Clear", Some(K::Char('F'))),
                ("r", "Refresh", Some(K::Char('r'))),
                ("Esc", "Close", Some(K::Esc)),
            ];
        };
        if let Some(r) = &d.reader {
            let other = match (r.utf8, r.view) {
                (true, View::Text) => "Hex",
                (true, _) => "Text",
                (false, View::Hex) => "Lossy",
                (false, _) => "Hex",
            };
            let mut keys = vec![
                ("↑↓ PgUp PgDn", "Scroll", None),
                ("g/G", "Top/End", None),
                (
                    "w",
                    if r.wrap { "No wrap" } else { "Wrap" },
                    Some(K::Char('w')),
                ),
                ("x", other, Some(K::Char('x'))),
            ];
            if !r.wrap {
                keys.push(("←→", "Pan", None));
            }
            if matches!(r.load, Load::Done { result: Err(_), .. }) {
                keys.push(("r", "Retry", Some(K::Char('r'))));
            }
            keys.push(("Esc", "Back", Some(K::Esc)));
            return keys;
        }
        if d.picking.is_some() {
            return vec![
                ("↑↓", "Select", None),
                ("↵", "Read", Some(K::Enter)),
                ("Esc", "Cancel", Some(K::Esc)),
            ];
        }
        vec![
            ("↑↓", "Select", None),
            ("↵", "Read", Some(K::Enter)),
            ("Tab", "Dispatch", Some(K::Tab)),
            ("J/K", "Detail", None),
            ("o", "Ops", Some(K::Char('o'))),
            ("i", "Intervals", Some(K::Char('i'))),
            ("n", "More", Some(K::Char('n'))),
            ("r", "Refresh", Some(K::Char('r'))),
            ("Esc", "Back", Some(K::Esc)),
        ]
    }
    fn draw_help(
        &mut self,
        t: &Theme,
        frame: &mut Frame,
        area: Rect,
        help: &[(&'static str, &'static str, Option<KeyCode>)],
    ) {
        for (row, items) in bar_rows(help, area.width).into_iter().enumerate() {
            let y = area.y + row as u16;
            for (x, i) in items {
                let (key, label, code) = help[i];
                let rect = Rect::new(area.x + x, y, (key.width() + 1 + label.width()) as u16, 1)
                    .intersection(area);
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(key, Style::default().fg(t.focus)),
                        Span::raw(" "),
                        Span::styled(label, Style::default().fg(t.text)),
                    ])),
                    rect,
                );
                if let Some(code) = code {
                    self.controls.push((
                        Focus::Agents,
                        Hit {
                            area: rect,
                            danger: false,
                            key: KeyEvent::new(code, KeyModifiers::NONE),
                        },
                    ));
                }
            }
        }
    }

    fn draw_list(&mut self, t: &Theme, frame: &mut Frame, area: Rect) {
        let width = usize::from(area.width);
        let filter = match &self.filter {
            None => "Filter: all traces".to_string(),
            Some(f) => {
                let mut text = format!(
                    "Filter: kind={} scope={} key={}",
                    inert(&f.kind),
                    inert(&f.scope),
                    inert(&f.key)
                );
                if let Some(run) = &f.run {
                    let _ = write!(text, " run={}", inert(run));
                }
                if let Some(source) = &self.source {
                    let _ = write!(text, " (from {})", inert(source));
                }
                text
            }
        };
        let count = match &self.list {
            Load::Pending(_) => "Loading…".to_string(),
            Load::Done { result: Ok(l), .. } if l["initialized"] == true => {
                let n = self.traces().len();
                format!("{n} trace{} · r Refresh", if n == 1 { "" } else { "s" })
            }
            Load::Done { .. } => "r Refresh".to_string(),
        };
        put(
            frame,
            area,
            0,
            sides(&filter, t.text, &count, t.muted, width),
        );
        put(frame, area, 1, rule(t, "", width));
        // The selected trace (or the filter form) takes the bottom four rows.
        let footer = 4.min(area.height.saturating_sub(2));
        let list = Rect {
            y: area.y + 2.min(area.height),
            height: area.height.saturating_sub(2 + footer),
            ..area
        };
        self.list_area = list;
        self.list_height = usize::from(list.height).max(1);
        let message = |frame: &mut Frame, lines: Vec<Line<'static>>| {
            for (i, line) in lines.into_iter().enumerate() {
                put(frame, list, i as u16, line);
            }
        };
        match &self.list {
            Load::Pending(_) => message(frame, vec![Line::raw("Loading…")]),
            Load::Done { result: Err(f), .. } => message(
                frame,
                vec![
                    Line::styled(f.text(), Style::default().fg(t.danger)),
                    Line::raw("Telemetry could not be read; this is not an empty result. r Retry"),
                ],
            ),
            Load::Done { result: Ok(l), .. } if l["initialized"] != true => message(
                frame,
                vec![Line::raw(
                    "Telemetry is not initialized — nothing recorded yet. No files were created.",
                )],
            ),
            Load::Done { .. } if self.traces().is_empty() => message(
                frame,
                vec![Line::raw(if self.filter.is_some() {
                    "No traces match this filter."
                } else {
                    "No traces recorded."
                })],
            ),
            Load::Done { .. } => {
                let height = self.list_height;
                self.top = follow(self.top, self.selected, height);
                let wide = area.width >= 120;
                let traces = self.traces().to_vec();
                for (row, (i, trace)) in traces
                    .iter()
                    .enumerate()
                    .skip(self.top)
                    .take(height)
                    .enumerate()
                {
                    let chosen = i == self.selected;
                    let (left, right) = trace_row(trace, wide);
                    let left = format!("{}{left}", if chosen { "▸ " } else { "  " });
                    let color = if chosen { t.focus } else { t.text };
                    put(
                        frame,
                        list,
                        row as u16,
                        sides(&left, color, &right, t.muted, width),
                    );
                    self.rows
                        .push((Rect::new(list.x, list.y + row as u16, list.width, 1), i));
                }
            }
        }
        let bottom = Rect {
            y: list.bottom(),
            height: footer,
            ..area
        };
        if let Some(form) = &mut self.form {
            put(frame, bottom, 0, rule(t, "filter", width));
            let cell = bottom.width / 4;
            for (i, name) in ["kind", "scope", "key", "run"].iter().enumerate() {
                let x = bottom.x + cell * i as u16;
                let y = bottom.y + 1;
                if y >= bottom.bottom() {
                    break;
                }
                frame.render_widget(
                    Paragraph::new(*name).style(Style::default().fg(t.muted)),
                    Rect::new(x, y, 6.min(cell), 1),
                );
                let field = Rect::new(x + 6, y, cell.saturating_sub(7), 1);
                form.areas[i] = field;
                frame
                    .buffer_mut()
                    .set_style(field, Style::default().bg(t.selected));
                form.fields[i].draw(
                    frame,
                    field,
                    form.focus == i,
                    if i == 3 { "any" } else { "" },
                    t,
                );
            }
            put(
                frame,
                bottom,
                2,
                if form.error.is_empty() {
                    Line::styled(
                        "kind, scope and key must match exactly; run is optional.",
                        Style::default().fg(t.muted),
                    )
                } else {
                    Line::styled(form.error.clone(), Style::default().fg(t.danger))
                },
            );
            return;
        }
        put(frame, bottom, 0, rule(t, "selected", width));
        let Some(trace) = self.traces().get(self.selected) else {
            return;
        };
        let binding = &trace["binding"];
        let lines = [
            format!("label  {}", inert(trace["label"].as_str().unwrap_or(""))),
            if binding.is_object() {
                format!(
                    "scope  {}   key {}   run {}",
                    text(&binding["scope"]),
                    text(&binding["key"]),
                    text(&binding["run"])
                )
            } else {
                format!("{} · no binding", text(&trace["origin"]))
            },
            format!(
                "coverage_start {} · trace recording {}",
                text(&trace["coverage_start"]),
                if trace["capture_enabled"] == true {
                    "on"
                } else {
                    "paused"
                }
            ),
        ];
        for (i, line) in lines.into_iter().enumerate() {
            put(frame, bottom, i as u16 + 1, Line::raw(clip(&line, width)));
        }
    }

    fn draw_detail(&mut self, t: &Theme, frame: &mut Frame, area: Rect) {
        let width = usize::from(area.width);
        let d = self.detail.as_mut().unwrap();
        let summary = d.summary.ready().map(|s| &s["record"]);
        let at = d.summary.at().to_owned();
        // Rows 0–2: the current state as one show(trace) read.
        let (now, counts) = match &d.summary {
            Load::Pending(_) => ("Now: Loading…".to_string(), String::new()),
            Load::Done { result: Err(f), .. } => {
                (format!("Now: {} · r Retry", f.text()), String::new())
            }
            Load::Done { result: Ok(_), .. } => match summary.filter(|s| s.is_object()) {
                None => ("Now: trace not found".to_string(), String::new()),
                Some(s) => (
                    format!(
                        "Now (queried {at}) · {} · {}",
                        binding_text(s),
                        text(&s["registration"])
                    ),
                    counts_text(s),
                ),
            },
        };
        let failed = matches!(d.summary, Load::Done { result: Err(_), .. });
        put(
            frame,
            area,
            0,
            Line::styled(
                clip(&now, width),
                Style::default().fg(if failed { t.danger } else { t.text }),
            ),
        );
        put(
            frame,
            area,
            1,
            sides(&counts, t.text, "o Ops", t.muted, width),
        );
        let mut spans = vec![Span::raw("Dispatch: ")];
        let known = summary.map(dispatches).unwrap_or_default();
        let names = std::iter::once("All".to_string()).chain(known.iter().map(|x| {
            format!(
                "{} {}",
                dispatch_kind(x["kind"].as_str().unwrap_or("")),
                short(&x["dispatch_id"], 4)
            )
        }));
        for (i, name) in names.enumerate() {
            if i == 1 {
                spans.push(Span::styled("  ◂ Tab ▸  ", Style::default().fg(t.muted)));
            } else if i > 1 {
                spans.push(Span::raw(" · "));
            }
            let current = match i.checked_sub(1) {
                None => d.dispatch.is_none(),
                Some(i) => {
                    d.dispatch.is_some()
                        && known[i]["dispatch_id"].as_str() == d.dispatch.as_deref()
                }
            };
            spans.push(if current {
                Span::styled(
                    name,
                    Style::default().fg(t.focus).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(name)
            });
        }
        put(frame, area, 2, Line::from(spans));

        // The panel: event detail, current operations, intervals or a body choice.
        d.selected = d.selected.min(d.events.len().saturating_sub(1));
        let operations = summary.and_then(|s| s["operations"].as_array());
        let loaded_all = d.dispatch.is_none() && !d.more && d.page.is_none();
        let (title, lines) = if d.picking.is_some() {
            let event = d.events.get(d.selected);
            let bodies = event
                .and_then(|e| e["bodies"].as_array())
                .cloned()
                .unwrap_or_default();
            let lines = bodies
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    format!(
                        "{}{}  {} {} B",
                        if Some(i) == d.picking { "▸ " } else { "  " },
                        body_label(event.unwrap(), b),
                        text(&b["role"]),
                        grouped(b["bytes"].as_u64().unwrap_or(0))
                    )
                })
                .collect();
            ("Choose a body".to_string(), lines)
        } else {
            match d.panel {
                Panel::Ops => (
                    format!("Operations now (queried {at})"),
                    match operations {
                        None => vec![now.clone()],
                        Some(ops) if ops.is_empty() => vec!["No operations recorded.".into()],
                        Some(ops) => ops
                            .iter()
                            .map(|op| op_line(op, &d.events, loaded_all))
                            .collect(),
                    },
                ),
                Panel::Intervals => (
                    format!("Recording intervals now (queried {at})"),
                    match summary.filter(|s| s.is_object()) {
                        None => vec![now.clone()],
                        Some(s) => intervals(s),
                    },
                ),
                Panel::Event => {
                    let mut lines = Vec::new();
                    let selected = known
                        .iter()
                        .find(|x| x["dispatch_id"].as_str() == d.dispatch.as_deref());
                    if selected.is_none()
                        && let Some(id) = &d.dispatch
                    {
                        lines.push(format!("dispatch {} · loading…", inert(id)));
                    }
                    if let Some(x) = selected {
                        lines.push(format!(
                            "dispatch {} · {}",
                            text(&x["dispatch_id"]),
                            text(&x["kind"])
                        ));
                        lines.push(format!(
                            "parent {} · created {}",
                            x["parent_dispatch_id"]
                                .as_str()
                                .map_or("none".into(), inert),
                            text(&x["created_at"])
                        ));
                    }
                    let event = d.events.get(d.selected);
                    match event {
                        Some(event) => lines.extend(event_lines(
                            event,
                            &d.events,
                            operations,
                            &at,
                            d.target.as_ref(),
                        )),
                        None if d.dispatch.is_none() => lines.push("No event selected.".into()),
                        None => {}
                    }
                    let seq = event.map(|e| format!("seq {}", text(&e["seq"])));
                    (seq.unwrap_or_default(), lines)
                }
            }
        };
        // The fixed event set and the detail panel.
        let rest = Rect {
            y: area.y + 3.min(area.height),
            height: area.height.saturating_sub(3),
            ..area
        };
        let wide = area.width >= WIDE;
        let (timeline, panel) = if wide {
            let left = area.width * 3 / 5;
            (
                Rect {
                    width: left,
                    ..rest
                },
                Rect {
                    x: rest.x + left + 1,
                    width: rest.width.saturating_sub(left + 1),
                    ..rest
                },
            )
        } else {
            // The event detail gets the rows it needs, leaving the timeline at least six.
            let needed: usize = lines
                .iter()
                .map(|l| crate::ui::wrap_text(l, rest.width).len())
                .sum::<usize>()
                + 1
                + usize::from(!d.message.is_empty());
            let bottom = if d.panel == Panel::Event && d.picking.is_none() {
                (needed as u16)
                    .min(rest.height.saturating_sub(6))
                    .max(4.min(rest.height))
            } else {
                rest.height / 2
            };
            (
                Rect {
                    height: rest.height.saturating_sub(bottom),
                    ..rest
                },
                Rect {
                    y: rest.y + rest.height.saturating_sub(bottom),
                    height: bottom,
                    ..rest
                },
            )
        };
        let tw = usize::from(timeline.width);
        let bound = match (&d.upper, &d.page) {
            (Some(upper), _) => format!("Events seq ≤ {upper}"),
            (None, Some(Load::Done { result: Err(_), .. })) => "Events".into(),
            (None, _) => "Events (loading…)".into(),
        };
        put(frame, timeline, 0, rule(t, &bound, tw));
        put(
            frame,
            timeline,
            1,
            Line::styled(
                clip("  seq recorded  disp kind                   src notes", tw),
                Style::default().fg(t.muted),
            ),
        );
        let list = Rect {
            y: timeline.y + 2.min(timeline.height),
            height: timeline.height.saturating_sub(2),
            ..timeline
        };
        self.list_area = list;
        let footer = match &d.page {
            Some(Load::Pending(_)) if !d.events.is_empty() => {
                Some(Line::raw("── loading next page… ──"))
            }
            Some(Load::Done { result: Err(f), .. }) => Some(Line::styled(
                format!("── {} · r Refresh ──", f.text()),
                Style::default().fg(t.danger),
            )),
            None if d.more => Some(Line::styled(
                format!("── n: next page ({PAGE} per page) ──"),
                Style::default().fg(t.muted),
            )),
            _ => None,
        };
        let height = usize::from(list.height).saturating_sub(usize::from(footer.is_some()));
        self.list_height = height.max(1);
        d.top = follow(d.top, d.selected, height.max(1));
        if d.events.is_empty() {
            let text = match &d.page {
                Some(Load::Pending(_)) => "Loading…",
                Some(Load::Done { .. }) => "",
                None if d.dispatch.is_some() => "No events recorded for this dispatch.",
                None => "No events recorded for this trace.",
            };
            put(frame, list, 0, Line::raw(text));
        }
        let mut shown = 0;
        for (row, (i, event)) in d
            .events
            .iter()
            .enumerate()
            .skip(d.top)
            .take(height)
            .enumerate()
        {
            let chosen = i == d.selected && d.picking.is_none();
            let line = format!(
                "{}{:>5} {:9} {:4} {:22} {}   {}",
                if chosen { "▸ " } else { "  " },
                text(&event["seq"]),
                clock_of(&event["recorded_at"]),
                if event["dispatch_id"].is_null() {
                    "·".into()
                } else {
                    short(&event["dispatch_id"], 4)
                },
                clip(&inert(event["kind"].as_str().unwrap_or("")), 22),
                source_letter(event),
                notes(event)
            );
            put(
                frame,
                list,
                row as u16,
                Line::styled(
                    clip(&line, tw),
                    Style::default().fg(if chosen { t.focus } else { t.text }),
                ),
            );
            self.rows
                .push((Rect::new(list.x, list.y + row as u16, list.width, 1), i));
            shown = row + 1;
        }
        if let Some(footer) = footer {
            put(
                frame,
                list,
                shown.max(usize::from(d.events.is_empty())) as u16,
                footer,
            );
        }

        let pw = usize::from(panel.width);
        let body = Rect {
            y: panel.y + 1.min(panel.height),
            height: panel
                .height
                .saturating_sub(1 + u16::from(!d.message.is_empty())),
            ..panel
        };
        self.panel_area = body;
        self.panel_height = usize::from(body.height).max(1);
        let rows: Vec<Line<'static>> = lines
            .iter()
            .flat_map(|l| crate::ui::wrap_text(l, body.width))
            .collect();
        let height = usize::from(body.height);
        let skip = if d.picking.is_some() {
            0
        } else {
            d.panel_top = d.panel_top.min(rows.len().saturating_sub(height));
            d.panel_top
        };
        // A detail longer than the panel says which part shows and how to reach the rest.
        let title = if d.picking.is_none() && rows.len() > height {
            format!(
                "{title} · {}–{} / {} J/K",
                skip + 1,
                (skip + height).min(rows.len()),
                rows.len()
            )
        } else {
            title
        };
        put(frame, panel, 0, rule(t, &title, pw));
        for (row, line) in rows
            .into_iter()
            .skip(skip)
            .take(usize::from(body.height))
            .enumerate()
        {
            put(frame, body, row as u16, line);
            if d.picking.is_some() {
                self.rows
                    .push((Rect::new(body.x, body.y + row as u16, body.width, 1), row));
            }
        }
        if !d.message.is_empty() {
            put(
                frame,
                panel,
                panel.height.saturating_sub(1),
                Line::styled(clip(&d.message, pw), Style::default().fg(t.focus)),
            );
        }
    }

    fn draw_reader(&mut self, t: &Theme, frame: &mut Frame, area: Rect) {
        let width = usize::from(area.width);
        let r = self
            .detail
            .as_mut()
            .and_then(|d| d.reader.as_mut())
            .unwrap();
        let hash = r.body["sha256"].as_str().unwrap_or("");
        let size = r.body["bytes"].as_u64().unwrap_or(0);
        let short_hash = if hash.len() > 8 {
            format!("{}…{}", &hash[..4], &hash[hash.len() - 3..])
        } else {
            inert(hash)
        };
        let content = Rect {
            y: area.y + 2.min(area.height),
            height: area.height.saturating_sub(2),
            ..area
        };
        self.list_area = content;
        r.height = usize::from(content.height).max(1);
        let bytes = match &r.load {
            Load::Pending(_) => {
                put(
                    frame,
                    area,
                    0,
                    Line::raw(format!("sha256 {short_hash} · {} bytes", grouped(size))),
                );
                put(frame, area, 1, rule(t, "", width));
                put(frame, content, 0, Line::raw("Loading…"));
                return;
            }
            Load::Done { result: Err(f), .. } => {
                put(
                    frame,
                    area,
                    0,
                    Line::raw(format!("sha256 {short_hash} · {} bytes", grouped(size))),
                );
                put(frame, area, 1, rule(t, "", width));
                put(
                    frame,
                    content,
                    0,
                    Line::styled(f.text(), Style::default().fg(t.danger)),
                );
                put(
                    frame,
                    content,
                    1,
                    Line::raw("No body text is shown — this is not an empty body. r Retry"),
                );
                return;
            }
            Load::Done {
                result: Ok(bytes), ..
            } => bytes,
        };
        let kind = match r.view {
            View::Text => "UTF-8",
            View::Hex if r.utf8 => "UTF-8 · hex",
            View::Hex => "not UTF-8 · hex",
            View::Lossy => "not UTF-8 · lossy (replacement characters)",
        };
        let meta = format!(
            "sha256 {short_hash} · {} bytes · hash verified · {kind}",
            grouped(bytes.len() as u64)
        );
        put(frame, area, 1, rule(t, "", width));
        if bytes.is_empty() {
            put(frame, area, 0, Line::raw(clip(&meta, width)));
            put(
                frame,
                content,
                0,
                Line::raw("Empty body (0 bytes, hash verified)"),
            );
            return;
        }
        let height = usize::from(content.height);
        let total = if r.view == View::Hex {
            bytes.len().div_ceil(16)
        } else {
            let key = (content.width, r.wrap, r.view);
            if r.laid_out != Some(key) {
                r.rows = layout(&r.display, usize::from(content.width), r.wrap);
                r.laid_out = Some(key);
            }
            r.rows.len()
        };
        r.top = r.top.min(total.saturating_sub(height));
        for row in 0..height.min(total - r.top) {
            let index = r.top + row;
            let line = if r.view == View::Hex {
                hex_row(bytes, index)
            } else {
                let (start, end) = r.rows[index];
                columns(&r.display[start..end], r.left, usize::from(content.width))
            };
            put(frame, content, row as u16, Line::raw(line));
        }
        let lines = format!(
            "lines {}–{} / {}",
            r.top + 1,
            (r.top + height).min(total),
            total
        );
        put(frame, area, 0, sides(&meta, t.text, &lines, t.focus, width));
    }
}

impl Detail {
    fn dispatch_id(&self) -> Option<&str> {
        self.dispatch.as_deref()
    }
}
impl Reader {
    /// UTF-8 bodies switch between text and hex; others between hex and a marked lossy view.
    fn toggle_view(&mut self) {
        let Load::Done {
            result: Ok(bytes), ..
        } = &self.load
        else {
            return;
        };
        self.view = match (self.utf8, self.view) {
            (true, View::Text) => View::Hex,
            (true, _) => View::Text,
            (false, View::Hex) => {
                self.display = escape_body(&String::from_utf8_lossy(bytes));
                View::Lossy
            }
            (false, _) => View::Hex,
        };
        self.laid_out = None;
        self.top = 0;
    }
}

fn copy(f: &BindingFilter) -> BindingFilter {
    BindingFilter {
        kind: f.kind.clone(),
        scope: f.scope.clone(),
        key: f.key.clone(),
        run: f.run.clone(),
    }
}
fn dispatches(record: &Value) -> Vec<Value> {
    record["dispatches"].as_array().cloned().unwrap_or_default()
}
fn dispatch_kind(kind: &str) -> String {
    match kind {
        "controller_handoff" => "handoff".into(),
        other => inert(other),
    }
}

/// One-line foreign text made inert: every control character, newline included, is shown
/// as a visible escape and never reaches the terminal.
pub fn inert(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => escape(c, &mut out),
            c => out.push(c),
        }
    }
    out
}
/// Body text: newlines kept, tabs as spaces, other control characters visible.
fn escape_body(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push('\n'),
            '\t' => out.push_str("    "),
            c if c.is_control() => escape(c, &mut out),
            c => out.push(c),
        }
    }
    out
}
fn escape(c: char, out: &mut String) {
    let n = u32::from(c);
    if n <= 0xff {
        let _ = write!(out, "\\x{n:02x}");
    } else {
        let _ = write!(out, "\\u{{{n:x}}}");
    }
}
fn text(value: &Value) -> String {
    match value {
        Value::Null => "—".into(),
        Value::String(s) => inert(s),
        other => inert(&other.to_string()),
    }
}
fn short(value: &Value, n: usize) -> String {
    value.as_str().map_or("—".into(), |s| {
        inert(&s.chars().take(n).collect::<String>())
    })
}
fn clock() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!("{:02}:{:02}:{:02}Z", now.hour(), now.minute(), now.second())
}
/// `HH:MM:SSZ` of a stored UTC timestamp.
fn clock_of(value: &Value) -> String {
    match value.as_str() {
        Some(s) if s.len() >= 19 && s.is_char_boundary(19) => format!("{}Z", &s[11..19]),
        _ => text(value),
    }
}
fn trace_row(trace: &Value, wide: bool) -> (String, String) {
    let created = trace["coverage_start"].as_str().unwrap_or("");
    let time = if created.len() >= 19 && created.is_char_boundary(19) {
        if wide {
            format!("{}Z", created[..19].replace('T', " "))
        } else {
            format!("{}Z", created[5..16].replace('T', " "))
        }
    } else {
        inert(created)
    };
    let origin = trace["origin"].as_str().unwrap_or("");
    let binding = &trace["binding"];
    let main = if binding.is_object() {
        if wide {
            format!(
                "{} · {} · {} · run {}",
                text(&binding["kind"]),
                text(&binding["scope"]),
                text(&binding["key"]),
                text(&binding["run"])
            )
        } else {
            format!(
                "{} · {} · run {}",
                text(&binding["kind"]),
                text(&binding["key"]),
                short(&binding["run"], 6)
            )
        }
    } else {
        inert(trace["label"].as_str().unwrap_or(""))
    };
    let paused = trace["capture_enabled"] != true;
    let right = match (binding.is_object(), paused) {
        (true, false) => text(&trace["registration"]),
        (true, true) => format!("{} · trace paused", text(&trace["registration"])),
        (false, true) => "trace paused".into(),
        (false, false) => String::new(),
    };
    (format!("{time}  {:6}  {main}", inert(origin)), right)
}
fn binding_text(record: &Value) -> String {
    let b = &record["binding"];
    if b.is_object() {
        format!(
            "{} · {} · run {}",
            text(&b["kind"]),
            text(&b["key"]),
            short(&b["run"], 6)
        )
    } else {
        format!("{} · no binding", text(&record["origin"]))
    }
}
fn counts_text(record: &Value) -> String {
    let intervals = record["recording_intervals"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let off = intervals.iter().filter(|i| i["enabled"] != true).count();
    let on = intervals.last().is_some_and(|i| i["enabled"] == true);
    let gaps = record["known_gaps"].as_array().map_or(0, Vec::len);
    let ops = record["operations"].as_array().cloned().unwrap_or_default();
    let count = |field: &str| ops.iter().filter(|o| o[field] == true).count();
    let unknown = ops.iter().filter(|o| !o["unknown"].is_null()).count();
    let mut parts = Vec::new();
    for (name, n) in [
        ("no begin", count("begin_missing")),
        ("no end", count("end_missing")),
        ("unknown", unknown),
    ] {
        if n > 0 {
            parts.push(format!("{name} {n}"));
        }
    }
    format!(
        "recording {} ({off} off interval{}) · gaps {gaps} · ops {}{}",
        if on { "on" } else { "off" },
        if off == 1 { "" } else { "s" },
        ops.len(),
        if parts.is_empty() {
            String::new()
        } else {
            format!(": {}", parts.join(", "))
        }
    )
}
fn source_letter(event: &Value) -> &'static str {
    match event["evidence_kind"].as_str() {
        Some("execution_observed") => "E",
        Some("system_control") => "S",
        Some("controller_statement") => "C",
        Some("plugin_statement") => "P",
        _ => "?",
    }
}
fn source_text(event: &Value) -> String {
    match event["evidence_kind"].as_str() {
        Some("execution_observed") => "observed by Saddle".into(),
        Some("system_control") => "system control".into(),
        _ => format!("declared by {} (unverified)", text(&event["producer"])),
    }
}
fn gaps(event: &Value) -> String {
    event["payload"]["gaps"]
        .as_array()
        .map(|gaps| {
            gaps.iter()
                .map(|g| format!("{}:{}", text(&g["role"]), text(&g["reason"])))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}
fn notes(event: &Value) -> String {
    let mut parts = Vec::new();
    if event["late_submission"] == true {
        parts.push("late".to_string());
    }
    for body in event["bodies"].as_array().into_iter().flatten() {
        parts.push(format!(
            "{} {}",
            text(&body["role"]),
            size(body["bytes"].as_u64().unwrap_or(0))
        ));
    }
    for gap in event["payload"]["gaps"].as_array().into_iter().flatten() {
        parts.push(format!(
            "gap {}:{}",
            text(&gap["role"]),
            text(&gap["reason"])
        ));
    }
    if let Some(kind) = event["payload"]["outcome"]["kind"].as_str() {
        parts.push(inert(kind));
    }
    parts.join(" · ")
}
/// What a body is, kept apart: the brief snapshot, the message actually sent, declared text.
fn body_label(event: &Value, body: &Value) -> String {
    match (
        event["kind"].as_str().unwrap_or(""),
        body["role"].as_str().unwrap_or(""),
    ) {
        ("brief.snapshot", "brief") => "任务书快照".into(),
        ("agent.send.begin", "message") => "实际发送内容".into(),
        ("requirement.recorded", "text") => "需求原文（声明来源）".into(),
        ("authorization.recorded", "text") => "授权原文（声明来源）".into(),
        ("proposal.recorded", "text") => "提案原文（声明来源）".into(),
        (_, role) => format!("{} body", inert(role)),
    }
}
fn event_lines(
    event: &Value,
    events: &[Value],
    operations: Option<&Vec<Value>>,
    at: &str,
    resolved: Option<&(String, Load<Value>)>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut head = format!("{} · {}", text(&event["kind"]), source_text(event));
    let operation = event["operation_id"]
        .as_str()
        .and_then(|id| operations.and_then(|ops| ops.iter().find(|o| o["operation_id"] == id)));
    if event["operation_id"].is_string() {
        let kind = operation.map_or(String::new(), |o| format!(" {}", text(&o["kind"])));
        let _ = write!(head, " · op {}{kind}", short(&event["operation_id"], 4));
    }
    lines.push(head);
    if event["source_description"].is_string() {
        lines.push(text(&event["source_description"]));
    }
    lines.push(format!(
        "event {} · trace {} · dispatch {}",
        text(&event["event_id"]),
        text(&event["trace_id"]),
        text(&event["dispatch_id"])
    ));
    let payload = &event["payload"];
    // The recorded payload as stored; gaps are listed under Not captured.
    if let Some(fields) = payload.as_object() {
        for (key, value) in fields.iter().filter(|(k, _)| *k != "gaps") {
            fields_of(key, value, &mut lines);
        }
    }
    for body in event["bodies"].as_array().into_iter().flatten() {
        let label = body_label(event, body);
        if event["kind"] == "brief.snapshot" {
            lines.push(format!("{label}  {}", text(&payload["absolute_path"])));
        } else {
            lines.push(label);
        }
        lines.push(format!(
            "body {} · {} B · sha256 {}… · ↵ Read",
            text(&body["role"]),
            grouped(body["bytes"].as_u64().unwrap_or(0)),
            short(&body["sha256"], 8)
        ));
    }
    let gaps = gaps(event);
    if !gaps.is_empty() {
        lines.push(format!("Not captured: {gaps}"));
    }
    // The operation as the current summary has it, not as of the event list's bound.
    if let Some(op) = operation {
        lines.push(format!(
            "op {} now: {} (queried {at})",
            short(&op["operation_id"], 4),
            op_state(op)
        ));
    }
    if event["late_submission"] == true {
        let declared = event["payload"]
            .get("declared_at")
            .filter(|v| !v.is_null())
            .unwrap_or(&event["observed_at"]);
        lines.push(format!("late · {}", text(&event["submission_notice"])));
        lines.push(format!(
            "declared {} · recorded {}",
            text(declared),
            text(&event["recorded_at"])
        ));
    } else {
        lines.push(format!(
            "recorded {} · source time {}",
            text(&event["recorded_at"]),
            text(&event["observed_at"])
        ));
    }
    for link in event["links"].as_array().into_iter().flatten() {
        let target = link["target_event_id"].as_str().unwrap_or("");
        let found = events.iter().find(|e| e["event_id"] == target);
        lines.push(match found {
            Some(e) => format!(
                "link {} → seq {} {}",
                text(&link["relation"]),
                text(&e["seq"]),
                text(&e["kind"])
            ),
            None => {
                let head = format!(
                    "link {} → event {}",
                    text(&link["relation"]),
                    short(&link["target_event_id"], 8)
                );
                match resolved
                    .filter(|(id, _)| link["relation"] == "carried_from" && id == target_id(link))
                {
                    Some((_, Load::Pending(_))) => format!("{head} · reading target trace…"),
                    Some((_, Load::Done { result: Err(f), .. })) => {
                        format!("{head} · target trace not read ({})", f.text())
                    }
                    Some((_, Load::Done { result: Ok(v), .. })) if v["record"].is_object() => {
                        let r = &v["record"];
                        format!(
                            "link {} → trace {} seq {} {}",
                            text(&link["relation"]),
                            short(&r["trace_id"], 8),
                            text(&r["seq"]),
                            text(&r["kind"])
                        )
                    }
                    Some(_) => format!("{head} · target trace not read (not initialized)"),
                    None => format!("{head} (not in loaded events)"),
                }
            }
        });
    }
    lines
}
/// One payload field per line; nested objects as dotted keys, arrays as compact JSON.
fn fields_of(key: &str, value: &Value, lines: &mut Vec<String>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (k, v) in map {
                fields_of(&format!("{key}.{k}"), v, lines);
            }
        }
        Value::String(s) => lines.push(format!("  {}: {}", inert(key), inert(s))),
        other => lines.push(format!("  {}: {}", inert(key), inert(&other.to_string()))),
    }
}
fn target_id(link: &Value) -> &str {
    link["target_event_id"].as_str().unwrap_or("")
}
fn op_state(op: &Value) -> String {
    let mut parts = Vec::new();
    if op["begin_missing"] == true {
        parts.push("no begin".to_string());
    }
    parts.push(if op["end_missing"] == true {
        "no end".into()
    } else {
        "ended".into()
    });
    if !op["unknown"].is_null() {
        parts.push(text(&op["unknown"]));
    }
    parts.join(" · ")
}
/// An operation's begin/end as the current summary has them, with seq when that event is in
/// the loaded set; never presented as part of the fixed event list.
fn op_line(op: &Value, events: &[Value], loaded_all: bool) -> String {
    let phase = |missing: &str, id: &str| {
        if op[missing] == true {
            return "missing".to_string();
        }
        let id = op[id].as_str().unwrap_or("");
        match events.iter().find(|e| e["event_id"] == id) {
            Some(e) => format!("seq {}", text(&e["seq"])),
            None if loaded_all => format!("{} (beyond event list)", clip(&inert(id), 8)),
            None => format!("{} (not in loaded events)", clip(&inert(id), 8)),
        }
    };
    let mut line = format!(
        "{} {} {}  begin {} · {}",
        short(&op["operation_id"], 4),
        text(&op["kind"]),
        short(&op["dispatch_id"], 4),
        phase("begin_missing", "begin_event_id"),
        if op["end_missing"] == true {
            "no end".to_string()
        } else {
            format!("end {}", phase("end_missing", "end_event_id"))
        }
    );
    if !op["unknown"].is_null() {
        let _ = write!(line, " · {}", text(&op["unknown"]));
    }
    line
}
fn intervals(record: &Value) -> Vec<String> {
    let mut lines: Vec<String> = record["recording_intervals"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| {
            format!(
                "{} → {}  {}",
                text(&i["start"]),
                if i["end"].is_null() {
                    "now".into()
                } else {
                    text(&i["end"])
                },
                if i["enabled"] == true { "on" } else { "off" }
            )
        })
        .collect();
    lines.push(String::new());
    lines.push(text(&record["coverage_notice"]));
    let gaps = record["known_gaps"].as_array().cloned().unwrap_or_default();
    lines.push(format!("known gaps {}", gaps.len()));
    lines.extend(gaps.iter().map(|g| {
        format!(
            "  event {} · {}:{}",
            short(&g["event_id"], 8),
            text(&g["gap"]["role"]),
            text(&g["gap"]["reason"])
        )
    }));
    lines
}
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
fn size(n: u64) -> String {
    match n {
        n if n < 1024 => format!("{n} B"),
        n if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)),
    }
}
fn hex_row(bytes: &[u8], row: usize) -> String {
    let chunk = &bytes[row * 16..bytes.len().min(row * 16 + 16)];
    let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
    let ascii: String = chunk
        .iter()
        .map(|b| {
            if (0x20..0x7f).contains(b) {
                char::from(*b)
            } else {
                '.'
            }
        })
        .collect();
    format!("{:08x}  {:<47}  |{ascii}|", row * 16, hex.join(" "))
}
/// Display rows as byte ranges of `text`: lines, soft-wrapped to `width` when `wrap`.
fn layout(text: &str, width: usize, wrap: bool) -> Vec<(usize, usize)> {
    let mut rows = Vec::new();
    let (mut start, mut used) = (0, 0);
    for (i, c) in text.char_indices() {
        if c == '\n' {
            rows.push((start, i));
            start = i + 1;
            used = 0;
            continue;
        }
        let w = c.width().unwrap_or(0);
        if wrap && used + w > width && i > start {
            rows.push((start, i));
            start = i;
            used = 0;
        }
        used += w;
    }
    if start < text.len() || !text.ends_with('\n') {
        rows.push((start, text.len()));
    }
    rows
}
/// The part of one display row from column `left`, at most `width` columns.
fn columns(row: &str, left: usize, width: usize) -> String {
    let (mut x, mut out) = (0, String::new());
    for c in row.chars() {
        let w = c.width().unwrap_or(0);
        if x >= left && x + w <= left + width {
            out.push(c);
        }
        x += w;
        if x >= left + width {
            break;
        }
    }
    out
}
fn follow(top: usize, selected: usize, height: usize) -> usize {
    top.min(selected).max((selected + 1).saturating_sub(height))
}
fn put(frame: &mut Frame, area: Rect, row: u16, line: Line<'_>) {
    if row < area.height {
        frame.render_widget(
            Paragraph::new(line),
            Rect::new(area.x, area.y + row, area.width, 1),
        );
    }
}
fn rule(t: &Theme, title: &str, width: usize) -> Line<'static> {
    let head = if title.is_empty() {
        String::new()
    } else {
        format!("─ {title} ")
    };
    let head = clip(&head, width);
    let fill = width.saturating_sub(head.width());
    Line::styled(
        format!("{head}{}", "─".repeat(fill)),
        Style::default().fg(t.border),
    )
}
/// Left text and right-aligned text on one row; the left side gives way.
fn sides(
    left: &str,
    left_color: ratatui::style::Color,
    right: &str,
    right_color: ratatui::style::Color,
    width: usize,
) -> Line<'static> {
    let right = clip(right, width);
    let room = width.saturating_sub(right.width() + usize::from(!right.is_empty()));
    let left = clip(left, room);
    let gap = width.saturating_sub(left.width() + right.width());
    Line::from(vec![
        Span::styled(left, Style::default().fg(left_color)),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::default().fg(right_color)),
    ])
}
/// Help items left to right with two-column gaps, then one, then wrapping onto rows.
fn bar_rows(items: &[(&str, &str, Option<KeyCode>)], width: u16) -> Vec<Vec<(u16, usize)>> {
    let w =
        |(key, label, _): &(&str, &str, Option<KeyCode>)| (key.width() + 1 + label.width()) as u16;
    for gap in [2, 1] {
        let total: u16 =
            items.iter().map(w).sum::<u16>() + gap * items.len().saturating_sub(1) as u16;
        if total <= width {
            let mut x = 0;
            return vec![
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let at = x;
                        x += w(item) + gap;
                        (at, i)
                    })
                    .collect(),
            ];
        }
    }
    let mut rows = vec![Vec::new()];
    let mut x = 0;
    for (i, item) in items.iter().enumerate() {
        if x > 0 && x + w(item) > width {
            rows.push(Vec::new());
            x = 0;
        }
        rows.last_mut().unwrap().push((x, i));
        x += w(item) + 1;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(page: &mut Page) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while page.loading() && std::time::Instant::now() < deadline {
            page.poll();
            thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn results_for_a_left_or_requeried_layer_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let mut page = Page::open(Ok(Store::new(dir.path().join("none"))), None, None);
        wait(&mut page);
        let stale = |token| Reply {
            token,
            at: "00:00:00Z".into(),
            result: Ok(Data::Value(
                serde_json::json!({"initialized":true,"traces":[{"trace_id":"late"}]}),
            )),
        };
        // The list's first read is answered; a repeat of its token changes nothing.
        let first = page.next;
        page.absorb(stale(first));
        assert!(page.traces().is_empty());
        // A re-query supersedes the read before it, whichever answers last.
        page.key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        let (old, new) = (page.next - 2, page.next);
        assert!(page.list.waits(new));
        page.absorb(stale(old));
        assert!(page.list.waits(new));
        wait(&mut page);
        assert!(page.traces().is_empty());
        assert!(page.list.ready().is_some());
    }
}
