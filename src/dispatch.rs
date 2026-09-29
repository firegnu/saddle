//! Dispatch: one task's recorded dispatches, read only through dispatch-log's public `ls`,
//! `show` and `cat`. saddle never records, routes, sends or retries through the recorder, and
//! never reads its data directory. Material is labelled by who stated it; nothing missing is
//! filled in or guessed.
use crate::links::Job;
use serde_json::Value;
use std::{collections::HashSet, sync::atomic::AtomicBool, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(5);
/// The largest answer one read takes; a larger one is reported, never shown cut short.
const LIMIT: usize = 4 * 1024 * 1024;

/// The task a reading belongs to: its explicit project root and task number, from one opening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub program: String,
    pub project: String,
    pub task: String,
    pub seq: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Loading,
    /// The recorder command could not be found; carries the configured command.
    Missing(String),
    Failed(String),
    Unsupported(String),
    Loaded,
}
/// One recorded dispatch as the controller declared it, with its steps in recorded order.
pub struct Record {
    pub id: String,
    pub heading: String,
    /// This dispatch could not be read; the others still show.
    pub error: Option<String>,
    pub entries: Vec<Entry>,
}
#[derive(Clone)]
pub struct Entry {
    pub summary: String,
    /// What the event itself says, before its recorded texts.
    pub text: String,
    pub parts: Vec<Part>,
}
/// A recorded text: its content hash, or why it was not recorded.
#[derive(Clone)]
pub struct Part {
    pub label: String,
    pub content: Result<String, String>,
}
pub struct Reading {
    pub title: String,
    pub text: String,
    pub scroll: usize,
}
enum Output {
    Listing {
        records: Vec<Record>,
        skipped: usize,
    },
    Text(String),
}
#[derive(Default)]
pub struct State {
    pub key: Option<Key>,
    pub status: Status,
    pub records: Vec<Record>,
    /// Records the recorder returned for another project or task, which are not shown.
    pub notice: String,
    pub selected: usize,
    pub top: usize,
    pub reading: Option<Reading>,
    pub rows: Vec<(ratatui::layout::Rect, usize)>,
    job: Option<Job<Result<Output, Status>>>,
}
impl State {
    /// Reads `key`'s records once; another task or opening drops the old reading and results.
    pub fn sync(&mut self, key: Key) {
        if self.key.as_ref() == Some(&key) {
            return;
        }
        *self = Self::default();
        self.key = Some(key.clone());
        self.job = Some(Job::start(move |cancel| listing(&key, cancel)));
    }
    pub fn tick(&mut self) {
        let Some(result) = self.job.as_ref().and_then(|j| j.result.try_recv().ok()) else {
            return;
        };
        self.job = None;
        match result {
            Ok(Output::Listing { records, skipped }) => {
                self.records = records;
                self.status = Status::Loaded;
                if skipped > 0 {
                    self.notice = format!(
                        "{skipped} records returned for another project or task are not shown."
                    );
                }
            }
            Ok(Output::Text(text)) => {
                if let Some(reading) = &mut self.reading {
                    reading.text = text;
                }
            }
            Err(status) => self.status = status,
        }
    }
    pub fn entries(&self) -> Vec<&Entry> {
        self.records.iter().flat_map(|r| &r.entries).collect()
    }
    /// Opens the selected entry's full text in place; `back` returns to the same entry.
    pub fn open(&mut self) {
        if self.job.is_some() {
            return;
        }
        let (Some(entry), Some(key)) = (
            self.entries().get(self.selected).map(|e| (*e).clone()),
            self.key.clone(),
        ) else {
            return;
        };
        self.reading = Some(Reading {
            title: entry.summary.clone(),
            text: "Loading…".into(),
            scroll: 0,
        });
        self.job = Some(Job::start(move |cancel| {
            Ok(Output::Text(full(&key.program, &entry, cancel)))
        }));
    }
    pub fn back(&mut self) {
        self.job = None;
        self.reading = None;
    }
    pub fn scroll(&mut self, delta: isize) {
        if let Some(reading) = &mut self.reading {
            reading.scroll = reading.scroll.saturating_add_signed(delta);
        } else {
            self.selected = self
                .selected
                .saturating_add_signed(delta)
                .min(self.entries().len().saturating_sub(1));
        }
    }
}

fn read(program: &str, args: &[&str], cancel: &AtomicBool) -> Result<Vec<u8>, Status> {
    let output = crate::command::run_bounded(program, args, None, &[], TIMEOUT, cancel, LIMIT)
        .map_err(|error| {
            let missing = error.chain().any(|e| {
                e.downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
            });
            if missing {
                Status::Missing(program.into())
            } else {
                Status::Failed(format!("{error:#}"))
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(Status::Failed(if stderr.is_empty() {
            format!("dispatch-log {} exited with {}", args[0], output.status)
        } else {
            stderr
        }));
    }
    Ok(output.stdout)
}
fn unsupported(what: impl std::fmt::Display) -> Status {
    Status::Unsupported(what.to_string())
}
fn problem(status: Status) -> String {
    match status {
        Status::Missing(program) => format!("dispatch-log not found ({program})"),
        Status::Failed(error) => format!("Read failed: {error}"),
        Status::Unsupported(error) => format!("Unsupported format: {error}"),
        Status::Loading | Status::Loaded => String::new(),
    }
}
fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// A dispatch as `ls` and `show` declare it (format version 1).
struct Declared {
    id: String,
    project: String,
    task: String,
    kind: String,
    parent: Option<String>,
    created_at: String,
}
fn declared(value: &Value) -> Result<Declared, Status> {
    let id = text(value, "dispatch_id")
        .filter(|id| id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    match (
        value.get("schema_version").and_then(Value::as_u64),
        id,
        text(value, "project"),
        text(value, "task"),
    ) {
        (Some(1), Some(id), Some(project), Some(task)) => Ok(Declared {
            id: id.into(),
            project: project.into(),
            task: task.into(),
            kind: text(value, "kind").unwrap_or("unknown").into(),
            parent: text(value, "parent").map(str::to_owned),
            created_at: text(value, "created_at").unwrap_or("").into(),
        }),
        (Some(1), ..) => Err(unsupported("dispatch record lacks its id, project or task")),
        (version, ..) => Err(unsupported(format!(
            "dispatch record format version {}",
            version.map_or("missing".into(), |v| v.to_string())
        ))),
    }
}
fn listing(key: &Key, cancel: &AtomicBool) -> Result<Output, Status> {
    let stdout = read(
        &key.program,
        &["ls", "--project", &key.project, "--task", &key.task],
        cancel,
    )?;
    let rows: Value = serde_json::from_slice(&stdout)
        .map_err(|e| unsupported(format!("ls did not answer JSON: {e}")))?;
    let rows = rows
        .as_array()
        .ok_or_else(|| unsupported("ls did not answer a list"))?;
    // The recorder keys projects by their resolved path.
    let project = std::fs::canonicalize(&key.project)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| key.project.clone());
    let mut records = Vec::new();
    let mut skipped = 0;
    for row in rows {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Status::Failed("cancelled".into()));
        }
        let dispatch = declared(row)?;
        if dispatch.task != key.task || dispatch.project != project {
            skipped += 1;
            continue;
        }
        records.push(record(&key.program, dispatch, cancel));
    }
    Ok(Output::Listing { records, skipped })
}
fn short(id: &str) -> String {
    id.chars().take(8).collect()
}
fn record(program: &str, dispatch: Declared, cancel: &AtomicBool) -> Record {
    let kind = match dispatch.kind.as_str() {
        "initial" => "Initial dispatch".to_owned(),
        "redispatch" => "Redispatch".into(),
        "review" => "Review".into(),
        other => format!("Dispatch ({other})"),
    };
    let parent = dispatch
        .parent
        .as_deref()
        .map_or(String::new(), |p| format!(" of {}", short(p)));
    let mut record = Record {
        id: dispatch.id.clone(),
        heading: format!(
            "{kind}{parent} · {} · {}",
            time(&dispatch.created_at),
            short(&dispatch.id)
        ),
        error: None,
        entries: Vec::new(),
    };
    match read(program, &["show", &dispatch.id], cancel).and_then(|out| events(&dispatch, &out)) {
        Ok(events) => record.entries = entries(program, &events, cancel),
        Err(status) => record.error = Some(problem(status)),
    }
    record
}
fn events(dispatch: &Declared, stdout: &[u8]) -> Result<Vec<Value>, Status> {
    let shown: Value = serde_json::from_slice(stdout)
        .map_err(|e| unsupported(format!("show did not answer JSON: {e}")))?;
    let same = declared(&shown["dispatch"])?;
    if (same.id.as_str(), same.project.as_str(), same.task.as_str())
        != (&dispatch.id, &dispatch.project, &dispatch.task)
    {
        return Err(unsupported("show answered for another dispatch"));
    }
    let events = shown["events"]
        .as_array()
        .ok_or_else(|| unsupported("show has no event list"))?;
    for event in events {
        if event["schema_version"].as_u64() != Some(1)
            || text(event, "kind").is_none()
            || text(event, "observed_at").is_none()
            || !event["data"].is_object()
        {
            return Err(unsupported("event record not in format version 1"));
        }
        if text(event, "dispatch_id") != Some(&dispatch.id) {
            return Err(unsupported("show returned an event of another dispatch"));
        }
    }
    Ok(events.clone())
}
/// One entry per step: an intent with a result is the same step as that result.
fn entries(program: &str, events: &[Value], cancel: &AtomicBool) -> Vec<Entry> {
    let finished: HashSet<&str> = events
        .iter()
        .filter(|e| !text(e, "kind").unwrap_or("").ends_with(".intent"))
        .filter_map(|e| text(&e["data"], "operation_id"))
        .collect();
    events
        .iter()
        .filter_map(|event| {
            let kind = text(event, "kind").unwrap_or("");
            let intent_only = kind.ends_with(".intent");
            if intent_only
                && text(&event["data"], "operation_id").is_some_and(|op| finished.contains(op))
            {
                return None;
            }
            // The suggestion's model and tier lead the JEV entry's summary.
            let suggestion = (kind == "jev.route")
                .then(|| blob(&event["data"]["suggestion"]))
                .flatten()
                .map(|sha| {
                    read(program, &["cat", &sha], cancel)
                        .map_err(problem)
                        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
                });
            Some(describe(event, intent_only, suggestion))
        })
        .collect()
}
fn blob(value: &Value) -> Option<String> {
    text(value, "sha256")
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_owned)
}
fn part(label: impl Into<String>, value: &Value, missing: &str) -> Part {
    Part {
        label: label.into(),
        content: blob(value).ok_or_else(|| format!("Not recorded{missing}")),
    }
}
/// `2026-09-29T08:03:00.1+00:00` as `09-29 08:03`; anything else as given.
fn time(stamp: &str) -> String {
    match (stamp.get(5..10), stamp.get(11..16)) {
        (Some(day), Some(clock)) if stamp.as_bytes().get(10) == Some(&b'T') => {
            format!("{day} {clock}")
        }
        _ => stamp.into(),
    }
}
/// route.py's `{verdict, ...}` suggestion: a null verdict means JEV is uncertain; its other
/// fields (such as the tier's top `level`) are never shown as the verdict.
fn verdict(value: &Value) -> &str {
    match value.get("verdict") {
        Some(Value::String(verdict)) => verdict,
        Some(Value::Null) => "uncertain",
        _ => "not recorded",
    }
}
fn recorded(value: &Value) -> &str {
    value.as_str().unwrap_or("not recorded")
}
fn given(value: &Value, what: &str) -> String {
    value
        .as_str()
        .map_or_else(|| format!("{what} not given"), str::to_owned)
}
fn describe(event: &Value, intent_only: bool, suggestion: Option<Result<Value, String>>) -> Entry {
    let data = &event["data"];
    let kind = text(event, "kind").unwrap_or("");
    let observed = text(event, "observed_at").unwrap_or("");
    let mut lines = vec![format!("Recorded {observed} (dispatch-log clock)")];
    lines.push(
        if event["source"] == "controller_statement" {
            "Source: controller statement, not observed by a tool"
        } else {
            "Source: dispatch-log's observation of a public command"
        }
        .into(),
    );
    if event["association"]["source"] == "unassociated"
        || event["association"]["context_found"] == false
    {
        lines.push("Association: not linked to a recorded dispatch".into());
    }
    let mut parts = Vec::new();
    let outcome = || {
        if intent_only {
            "result unknown (intent only)".to_owned()
        } else if data["execution"] == "not_started" {
            "not started".into()
        } else {
            match data["exit_code"].as_i64() {
                Some(0) => "ok".into(),
                Some(code) => format!("exit {code}"),
                None => "exit unknown".into(),
            }
        }
    };
    let action = if kind == "corral.intent" {
        text(data, "action").unwrap_or("")
    } else {
        kind.strip_prefix("corral.").unwrap_or("")
    };
    let summary = if kind == "jev.route" || kind == "jev.intent" {
        let a_mode = data["mode"] == "A";
        lines.push(if a_mode {
            "Mode: A — route.py version not verified; only the suggestion is kept".into()
        } else {
            "Mode: B — parsed response JSON (not the HTTP bytes)".into()
        });
        if let Some(sha) = text(data, "route_sha256") {
            lines.push(format!("route.py sha256: {sha}"));
        }
        if let Some(error) = text(data, "error_type") {
            lines.push(format!("Error type: {error}"));
        }
        parts.push(part("JEV input (summary)", &data["summary"], ""));
        parts.push(part("JEV request JSON", &data["request"], ""));
        parts.push(part(
            "JEV full parsed response",
            &data["parsed_response"],
            if a_mode {
                ": mode A keeps only the suggestion"
            } else {
                ""
            },
        ));
        parts.push(part("JEV suggestion", &data["suggestion"], ""));
        let result = match (intent_only, suggestion) {
            (true, _) => format!("JEV route · {}", outcome()),
            (false, Some(Ok(s))) => format!(
                "JEV suggestion · {} · tier {} · cross review {} · impact {}",
                given(&s["model"], "model"),
                verdict(&s["tier"]),
                verdict(&s["cross_review"]),
                verdict(&s["impact"])
            ),
            (false, Some(Err(_))) => "JEV suggestion unreadable".into(),
            (false, None) => format!("JEV route · no suggestion recorded · {}", outcome()),
        };
        format!("{result}{}", if a_mode { " · mode A" } else { "" })
    } else if kind == "controller.decide" {
        lines.push(format!("Model: {}", recorded(&data["model"])));
        lines.push(format!("Effort: {}", recorded(&data["effort"])));
        lines.push(format!(
            "Verification budget: {}",
            recorded(&data["budget"])
        ));
        lines.push(format!("Reason: {}", recorded(&data["reason"])));
        lines.push("JEV's suggestion is its own entry; this is what the controller chose.".into());
        format!(
            "Controller decision · {} / {} · budget {}",
            given(&data["model"], "model"),
            given(&data["effort"], "effort"),
            given(&data["budget"], "budget")
        )
    } else if kind == "controller.note" {
        let note = text(data, "kind").unwrap_or("unknown");
        parts.push(part(
            format!("Controller {note} note"),
            &data["content"],
            "",
        ));
        format!("Controller {note} note")
    } else if action == "start" {
        let args = &data["agent_parameters"];
        let labels = &data["labels"];
        let choice = format!(
            "{} / {}",
            given(&args["model"], "model"),
            given(&args["effort"], "effort")
        );
        lines.push(format!(
            "Agent program: {}",
            recorded(&data["agent_program"])
        ));
        lines.push(format!("Working directory: {}", recorded(&data["cwd"])));
        lines.push(format!(
            "Explicit arguments: {choice} (defaults are not inferred)"
        ));
        let declared: Vec<_> = ["role", "model", "effort"]
            .into_iter()
            .filter_map(|k| text(labels, k).map(|v| format!("{k}={v}")))
            .collect();
        lines.push(format!(
            "Declared labels: {}",
            if declared.is_empty() {
                "none recorded".into()
            } else {
                declared.join(", ")
            }
        ));
        corral_result(&mut lines, data, intent_only);
        match data["task_file"].as_object() {
            None => parts.push(Part {
                label: "Task file snapshot".into(),
                content: Err("Not recorded: no task file was given to the recorder".into()),
            }),
            Some(_) => parts.push(snapshot(&data["task_file"])),
        }
        parts.push(part("Prompt given to corral start", &data["prompt"], ""));
        format!(
            "Start {} · {choice} · {}",
            text(&data["result"], "name").unwrap_or("agent name unknown"),
            outcome()
        )
    } else if action == "send" {
        let label = if data["kind"] == "rework" {
            "Rework send"
        } else {
            "Follow-up send"
        };
        let delivery = text(data, "delivery").unwrap_or("delivery unknown");
        if let Some(reason) = text(data, "missing_reason") {
            lines.push(format!("Name and text not recorded: {reason}"));
        }
        lines.push(match delivery {
            "queued" => "Delivery: queued (pending) — not proof of delivery".into(),
            other => format!("Delivery: {other}"),
        });
        if data["complete_input_known"] == false {
            lines.push(
                "Only the recorded text is known; the agent's complete input is unknown.".into(),
            );
        }
        corral_result(&mut lines, data, intent_only);
        parts.push(part("Sent text", &data["text"], ""));
        if data["task_file"].is_object() {
            parts.push(snapshot(&data["task_file"]));
        }
        format!(
            "{label} → {} · {delivery} · {}",
            recorded(&data["target_name"]),
            outcome()
        )
    } else if action == "reply" {
        let check = text(data, "instance_check").unwrap_or("unknown");
        corral_result(&mut lines, data, intent_only);
        lines.push(format!(
            "Instance check: {check} — the same instance does not prove which send it answers"
        ));
        lines.push(
            "Causal link: not proven; the dispatch it belongs to is the controller's declaration"
                .into(),
        );
        lines.push(
            "Test results in the reply are the implementer's report, not observed by saddle."
                .into(),
        );
        parts.push(part("Reply text", &data["reply"], ""));
        format!(
            "Reply from {} · instance {check} · {}",
            recorded(&data["target_name"]),
            outcome()
        )
    } else {
        lines.push("Not recognised by this saddle; the recorded data follows.".into());
        lines.push(serde_json::to_string_pretty(data).unwrap_or_default());
        format!("{kind} (not recognised)")
    };
    if let Some(gaps) = data["recording_gaps"].as_array().filter(|g| !g.is_empty()) {
        let gaps: Vec<_> = gaps
            .iter()
            .map(|g| {
                format!(
                    "{} ({})",
                    text(g, "stage").unwrap_or("?"),
                    text(g, "error_type").unwrap_or("?")
                )
            })
            .collect();
        lines.push(format!("Recording gaps: {}", gaps.join(", ")));
    }
    Entry {
        summary: format!("{} {summary}", time(observed)),
        text: lines.join("\n"),
        parts,
    }
}
fn corral_result(lines: &mut Vec<String>, data: &Value, intent_only: bool) {
    if intent_only {
        lines.push(
            "Result: unknown — only the intent was recorded; this does not mean it did not run."
                .into(),
        );
        return;
    }
    let result = |key: &str| data["result"][key].as_str().unwrap_or("not returned");
    // corral's `at` is a Unix time number; shown as given, apart from the recorder's clock.
    let at = match &data["result"]["at"] {
        Value::Number(at) => format!("{at} (Unix time from corral)"),
        Value::String(at) => format!("{at} (corral's time)"),
        _ => "not returned".into(),
    };
    lines.push(format!(
        "corral answered: name {}, instance {}, at {at}",
        result("name"),
        result("instance"),
    ));
    if data["response_status"] == "unparseable" {
        lines.push("corral's answer could not be parsed.".into());
    }
    if let Some(code) = data["exit_code"].as_i64() {
        lines.push(format!("Exit code: {code}"));
    }
    if let Some(error) = text(data, "error_type") {
        lines.push(format!("Error type: {error}"));
    }
}
fn snapshot(file: &Value) -> Part {
    part(
        format!(
            "Task file snapshot: {}, saved {} before the command — not proof the agent read this version; Links shows the current file",
            recorded(&file["path"]),
            recorded(&file["read_at"])
        ),
        &file["content"],
        ": the file could not be read before the command",
    )
}
/// The entry's own lines, then each recorded text read through `cat`.
fn full(program: &str, entry: &Entry, cancel: &AtomicBool) -> String {
    let mut out = entry.text.clone();
    for part in &entry.parts {
        out.push_str(&format!("\n\n── {} ──\n", part.label));
        match &part.content {
            Ok(sha) => match read(program, &["cat", sha], cancel) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(content) => out.push_str(&content),
                    Err(_) => out.push_str("Not UTF-8 text; not shown."),
                },
                Err(status) => out.push_str(&problem(status)),
            },
            Err(why) => out.push_str(why),
        }
    }
    out
}
