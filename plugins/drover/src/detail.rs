use crate::{
    drover::{Detail, Task},
    theme::Theme,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::sync::atomic::{AtomicU64, Ordering};
use unicode_width::UnicodeWidthChar;

static OPENED: AtomicU64 = AtomicU64::new(0);
/// A number no earlier opening of a detail or confirmation page has used.
pub(crate) fn opening() -> u64 {
    OPENED.fetch_add(1, Ordering::Relaxed)
}

/// The Tasks area's detail page for one task, fixed when it opens.
pub struct TaskDetail {
    /// Unique per opening, so a reopened page never takes an older result.
    pub(crate) seq: u64,
    /// Where the list had the task, and its list copy, when the page opened.
    pub(crate) group: &'static str,
    pub(crate) task: Task,
    /// The last successful task detail read; kept, and labelled stale, when a refresh fails.
    pub data: Option<Box<Detail>>,
    pub error: Option<String>,
    pub(crate) scroll: usize,
    /// Viewport height and furthest scroll from the last draw. Only keys clamp to them, so a
    /// refresh that briefly shortens the page does not lose the reading position.
    pub(crate) view: (usize, usize),
}
impl TaskDetail {
    pub(crate) fn new(group: &'static str, task: Task) -> Self {
        Self {
            seq: opening(),
            group,
            task,
            data: None,
            error: None,
            scroll: 0,
            view: (0, 0),
        }
    }
    pub(crate) fn scroll_by(&mut self, delta: isize) {
        self.scroll = self.scroll.saturating_add_signed(delta).min(self.view.1);
    }
    pub(crate) fn page(&self) -> isize {
        self.view.0.saturating_sub(1).max(1) as isize
    }
    /// `live` is where the list has the task now; `queried` whether task detail read covers it.
    #[cfg(test)]
    pub(crate) fn lines(
        &self,
        t: &Theme,
        live: Option<(&'static str, &Task)>,
        queried: bool,
        width: usize,
    ) -> Vec<Line<'static>> {
        self.lines_and_links(t, live, queried, width).0
    }
    /// The lines, and each drawn piece of the Telemetry link as (row, column, width).
    pub(crate) fn lines_and_links(
        &self,
        t: &Theme,
        live: Option<(&'static str, &Task)>,
        queried: bool,
        width: usize,
    ) -> (Vec<Line<'static>>, Vec<(usize, u16, u16)>) {
        let mut out = Out {
            t,
            width,
            rows: Vec::new(),
            links: Vec::new(),
        };
        let Some(data) = &self.data else {
            let (group, task) = live.unwrap_or((self.group, &self.task));
            let (status, color) = crate::queue::task_status(t, group, task.status.as_deref());
            out.header(task.id.as_deref(), status, color, None, &task.title);
            if queried && self.error.is_none() {
                out.banner(
                    "CHECKING STATUS",
                    "Waiting for current run details.",
                    t.agent_starting,
                );
            } else {
                out.banner(
                    "STATUS UNAVAILABLE",
                    "No current run details; check the controller.",
                    t.agent_blocked,
                );
            }
            if queried {
                match &self.error {
                    None => out.line(vec![Span::styled(
                        "Loading details…",
                        bold(t.agent_starting),
                    )]),
                    Some(error) => {
                        out.line(vec![Span::styled(
                            format!("Details unavailable: {error}"),
                            Style::default().fg(t.agent_error),
                        )]);
                        out.note("Retrying automatically every 5s.");
                    }
                }
                return (out.rows, out.links);
            }
            out.note(if task.id.is_some() {
                "From the queue list; task details are unavailable."
            } else {
                "Unnumbered task; showing queue list data only."
            });
            if live.is_none() {
                out.line(vec![Span::styled(
                    "No longer in the list; showing the copy from when details opened.",
                    Style::default().fg(t.agent_blocked),
                )]);
            }
            if let Some(reason) = &task.reason {
                out.field("Reason", &[(reason.clone(), t.text)]);
            }
            out.body(&task.body);
            return (out.rows, out.links);
        };
        let task = &data.task;
        let group = match task.status.as_deref() {
            Some("running") => "Current",
            Some("awaiting_release") => "Awaiting",
            Some("pending") => "Pending",
            _ => "History",
        };
        let (status, color) = crate::queue::task_status(t, group, task.status.as_deref());
        out.header(task.id.as_deref(), status, color, None, &task.title);
        out.status_banner(task, &data.reports, self.error.is_some());
        if let Some(error) = &self.error {
            out.line(vec![Span::styled(
                format!("Refresh failed: {error}"),
                Style::default().fg(t.agent_error),
            )]);
            out.note(&format!(
                "Showing stale details from {}. Retrying every 5s.",
                time_of_day(data.evidence.observed_at)
            ));
        } else {
            out.note(&format!(
                "Updated {} · refreshes every 5s",
                time_of_day(data.evidence.observed_at)
            ));
        }
        out.look_further(task, &data.reports);
        out.overview(task, &data.reports);
        out.reports(&data.reports);
        out.records(task);
        for (index, run) in task.previous_runs.iter().enumerate() {
            out.heading(&format!("Previous run {}", index + 1));
            out.records(run);
        }
        out.evidence(&data.evidence);
        out.body(&task.body);
        (out.rows, out.links)
    }
}

/// The Telemetry link inside Run details; drawn and clicked like the one beside the tabs.
pub(crate) const TELEMETRY_LINK: &str = "Open in Telemetry ↗";
// Wide enough for the longest fixed label, "Declared verdict".
const LABEL: usize = 16;

struct Out<'a> {
    t: &'a Theme,
    width: usize,
    rows: Vec<Line<'static>>,
    /// Each drawn piece of the Telemetry link: row, column and width.
    links: Vec<(usize, u16, u16)>,
}
impl Out<'_> {
    fn banner(&mut self, title: &str, action: &str, color: Color) {
        let first = self.rows.len();
        self.text(1, &format!(" {title}"), bold(color));
        self.text(1, &format!(" {action}"), Style::default().fg(self.t.bright));
        for row in &mut self.rows[first..] {
            row.style = Style::default().bg(self.t.agent_selected);
        }
    }

    fn status_banner(&mut self, task: &Task, reports: &crate::telemetry::Reports, stale: bool) {
        let closure = reports
            .entries
            .iter()
            .filter(|r| r.kind == "controller.note")
            .max_by_key(|r| r.seq);
        let readable_closure = reports.state == "available"
            && closure.is_some_and(|r| r.text.as_ref().is_some_and(|text| !text.trim().is_empty()));
        let (title, action, color) = if stale {
            (
                "STATUS UNAVAILABLE",
                "Refresh failed. Details below are stale; check before acting.",
                self.t.agent_blocked,
            )
        } else {
            match task.status.as_deref() {
                Some("awaiting_release") => (
                    "AWAITING YOUR ACCEPTANCE",
                    "Review the result, then Accept or Return to pending.",
                    self.t.focus,
                ),
                Some("done") => (
                    "ACCEPTED",
                    "This task has been accepted. No approval is pending.",
                    self.t.agent_idle,
                ),
                Some("running") if readable_closure => (
                    "YOUR REVIEW NEEDED",
                    "Controller closure report available. Check it before Submit for review.",
                    self.t.focus,
                ),
                Some("running") => (
                    "COMPLETION UNCONFIRMED",
                    "No readable closure report for this run. Check the controller.",
                    self.t.agent_blocked,
                ),
                Some("pending") => (
                    "PENDING DISPATCH",
                    "Review the task and any return reason before dispatching.",
                    self.t.agent_starting,
                ),
                Some("failed") => (
                    "FAILED",
                    "Check the recorded failure and reports below.",
                    self.t.agent_error,
                ),
                Some("dropped") => (
                    "DROPPED",
                    "This task was dropped. No approval is pending.",
                    self.t.muted,
                ),
                _ => (
                    "STATUS UNKNOWN",
                    "Check the task and controller before acting.",
                    self.t.agent_blocked,
                ),
            }
        };
        self.banner(title, action, color);
    }

    /// Where the rest of this page and the full chain are, from what was already read.
    fn look_further(&mut self, task: &Task, reports: &crate::telemetry::Reports) {
        self.heading("Look further");
        let spans = vec![
            self.label("Full chain"),
            Span::styled(TELEMETRY_LINK, bold(self.t.focus)),
            Span::styled(
                format!(
                    " · every run of {}",
                    task.id.as_deref().unwrap_or("this task")
                ),
                Style::default().fg(self.t.text),
            ),
        ];
        self.push_link(self.value_indent(), spans, Some(1));
        let recorded = match reports.state.as_str() {
            "available" if reports.entries.is_empty() => "None recorded for this run".into(),
            "available" => {
                let unreadable = reports.entries.iter().filter(|r| r.text.is_none()).count();
                format!(
                    "{} recorded below{} · scroll or PgDn",
                    reports.entries.len(),
                    match unreadable {
                        0 => String::new(),
                        1 => " (1 body unreadable)".into(),
                        n => format!(" ({n} bodies unreadable)"),
                    }
                )
            }
            "not_recorded" => "No trace for this run; unknown".into(),
            _ => "Unavailable; unknown".into(),
        };
        self.field("Reports", &[(recorded, self.t.text)]);
        let mut runs = match task.previous_runs.len() {
            0 => "This run below".to_owned(),
            1 => "This run + 1 previous run below".into(),
            n => format!("This run + {n} previous runs below"),
        };
        match task.return_history.len() {
            0 => {}
            1 => runs.push_str(" · 1 return this run"),
            n => runs.push_str(&format!(" · {n} returns this run")),
        }
        self.field("Run records", &[(runs, self.t.text)]);
    }

    fn overview(&mut self, task: &Task, reports: &crate::telemetry::Reports) {
        self.heading("Key events · this run");
        let next = match task.status.as_deref() {
            Some("running") => {
                "Check review and closure reports before Submit for review. If evidence is missing, check the controller."
            }
            Some("awaiting_release") => {
                "Awaiting manual acceptance. Accept if satisfied, or Return to pending for rework."
            }
            Some("done") => "Task accepted; no further transition needed.",
            Some("pending") => "Pending dispatch. Check any previous return reason below.",
            _ => "Follow the task status; telemetry does not advance it automatically.",
        };
        self.field("Next step", &[(next.into(), self.t.focus)]);
        let missing = match reports.state.as_str() {
            "available" => "Not recorded (may have occurred)",
            "not_recorded" => "No trace for this run; result unknown",
            _ => "Telemetry unavailable; result unknown",
        };
        for (kind, label) in [
            ("review.recorded", "Review"),
            ("controller.note", "Closure"),
        ] {
            let report = reports
                .entries
                .iter()
                .filter(|r| r.kind == kind)
                .max_by_key(|r| r.seq)
                .filter(|_| reports.state == "available");
            let value = report
                .map(|r| {
                    format!(
                        "{} · seq {} · {}{}",
                        r.verdict
                            .as_ref()
                            .map(|v| format!("Controller declared: {v}"))
                            .unwrap_or_else(|| "Report recorded (not proof of success)".into()),
                        r.seq,
                        r.recorded_at,
                        if r.text.is_none() {
                            " · Body unavailable"
                        } else {
                            ""
                        }
                    )
                })
                .unwrap_or_else(|| missing.into());
            self.field(label, &[(value, self.t.text)]);
        }
        for (label, at, empty, recorded) in [
            (
                "Submission",
                &task.t1,
                "Not submitted",
                matches!(task.status.as_deref(), Some("awaiting_release" | "done")),
            ),
            (
                "Acceptance",
                &task.t2,
                "Not accepted",
                task.status.as_deref() == Some("done"),
            ),
        ] {
            let text = at
                .as_ref()
                .and_then(|v| v.as_f64())
                .map(|v| format!("Recorded · {}", clock(v)))
                .unwrap_or_else(|| {
                    if at.is_some() {
                        "Recorded time unreadable".into()
                    } else if recorded {
                        "Transition completed (time not recorded)".into()
                    } else {
                        empty.into()
                    }
                });
            self.field(label, &[(text, self.t.text)]);
        }
        for (stage, label) in [
            ("agent.send", "Delivery"),
            ("route", "Routing"),
            ("agent.start", "Agent start"),
            ("agent.reply", "Agent reply"),
        ] {
            let value = reports
                .observations
                .iter()
                .find(|n| n.stage == stage)
                .filter(|_| reports.state == "available")
                .map(|n| {
                    format!(
                        "{} · seq {} · {} · {} events",
                        n.status, n.seq, n.recorded_at, n.count
                    )
                })
                .unwrap_or_else(|| missing.into());
            self.field(label, &[(value, self.t.text)]);
        }
        self.note("Latest event per category, not overall success. Review/closure are controller reports.");
        let count: usize = reports.observations.iter().map(|n| n.count).sum();
        if count > reports.observations.len() || reports.entries.len() > 2 {
            self.note("Multiple events/reports may span dispatches or arrive late. Check the full order in Telemetry.");
        }
    }

    fn reports(&mut self, reports: &crate::telemetry::Reports) {
        if reports.state == "available" {
            self.heading(&format!(
                "Controller reports · this run ({})",
                reports.entries.len()
            ));
        } else {
            self.heading("Controller reports · this run");
        }
        self.note("Recorded declarations, not task acceptance. Submit / Accept remain manual.");
        match reports.state.as_str() {
            "available" => {}
            "not_recorded" => {
                self.note("No telemetry trace recorded for this run; completion is unknown.");
                return;
            }
            _ => {
                self.note("Reports unavailable; completion is unknown.");
                if let Some(detail) = &reports.detail {
                    self.note(detail);
                }
                return;
            }
        }
        if reports.entries.is_empty() {
            self.note("No review or closure report recorded for this run.");
        }
        self.note("All recorded reports below; later reports may revise earlier ones.");
        for report in &reports.entries {
            let label = if report.kind == "review.recorded" {
                "Review"
            } else {
                "Closure"
            };
            self.heading(&format!(
                "{label} · seq {} · {}",
                report.seq, report.recorded_at
            ));
            if let Some(verdict) = &report.verdict {
                self.field("Declared verdict", &[(verdict.clone(), self.t.text)]);
            }
            for link in &report.links {
                self.value("Link", link);
            }
            match &report.text {
                Some(text) => self.text(0, text, Style::default().fg(self.t.text)),
                None => self.note(
                    report
                        .body_error
                        .as_deref()
                        .unwrap_or("Body unavailable; open Telemetry."),
                ),
            }
            // Identifiers for matching events in Telemetry: kept whole, shown as secondary.
            self.field(
                "Event",
                &[(
                    format!(
                        "{} · Dispatch {}",
                        report.event_id,
                        report.dispatch_id.as_deref().unwrap_or("trace-level")
                    ),
                    self.t.muted,
                )],
            );
        }
        if let Some(trace) = &reports.trace_id {
            self.field("Trace", &[(trace.clone(), self.t.muted)]);
        }
    }
    fn evidence(&mut self, evidence: &crate::drover::Evidence) {
        self.heading("Repository reference");
        self.note("Observed repository data; does not establish task ownership or control task transitions.");
        for (label, reference) in [("Git", &evidence.git), ("Last check", &evidence.last_check)] {
            self.heading(label);
            let color = match reference.state.as_str() {
                "failed" => self.t.agent_error,
                "stale" => self.t.agent_blocked,
                _ => self.t.text,
            };
            self.field("State", &[(reference.state.clone(), color)]);
            if label == "Git" && reference.state != "available" {
                self.note("Git data is incomplete or stale; empty results do not establish a clean repository.");
            }
            if label == "Last check" {
                self.note("Saved reference only; no check was run by this view.");
                if let Some(record) = reference.fields.get("record") {
                    let (result, color) = match record.get("ok").and_then(|v| v.as_bool()) {
                        Some(false) => ("failed", self.t.agent_error),
                        Some(true) => ("passed in saved record", self.t.text),
                        None => ("unknown", self.t.muted),
                    };
                    self.field("Saved result", &[(result.into(), color)]);
                }
            }
            for (key, value) in &reference.fields {
                self.value(&phrase(key), value);
            }
        }
    }
    fn records(&mut self, task: &Task) {
        self.heading("Run records");
        self.field(
            "Run",
            &[(
                task.run_id.clone().unwrap_or_else(|| "not recorded".into()),
                self.t.text,
            )],
        );
        for (label, value) in [
            ("Started", &task.t0),
            ("Submit / done", &task.t1),
            ("Accepted", &task.t2),
        ] {
            self.field(
                label,
                &[(
                    value
                        .as_ref()
                        .and_then(|v| v.as_f64())
                        .map(clock)
                        .unwrap_or_else(|| "not recorded".into()),
                    self.t.text,
                )],
            );
        }
        if let Some(submission) = &task.submission {
            self.heading("Submission / legacy completion");
            self.value("Recorded event", submission);
        }
        if let Some(record) = &task.completion_record {
            self.heading("Legacy manual completion");
            self.note(
                "Historical user confirmation; does not establish checks passed or acceptance.",
            );
            self.value("Saved record", record);
        }
        if !task.return_history.is_empty() {
            self.heading("Return history");
            self.note("Work stopped is the user's confirmation, not an observed agent state.");
            for record in &task.return_history {
                self.value("Returned", record);
            }
        }
        if let Some(reason) = &task.reason {
            self.field("Reason", &[(reason.clone(), self.t.text)]);
        }
    }
    /// Historical and reference payloads are shown as supplied; unknown fields carry no inference.
    fn value(&mut self, label: &str, value: &serde_json::Value) {
        if let serde_json::Value::Object(fields) = value {
            self.note(label);
            for (key, value) in fields {
                self.value(&phrase(key), value);
            }
            return;
        }
        let text = match value {
            serde_json::Value::Null => "unknown".into(),
            serde_json::Value::String(text) => text.clone(),
            _ => value.to_string(),
        };
        self.field(label, &[(text, self.t.text)]);
    }
    /// Adds one logical line, wrapped to the width at spaces where possible; continuation rows
    /// start `indent` columns in. Control characters are dropped, so text is never interpreted.
    fn push(&mut self, indent: usize, spans: Vec<Span<'static>>) {
        self.push_link(indent, spans, None);
    }
    /// Like `push`; where each wrapped piece of the span at `link` lands is recorded.
    fn push_link(&mut self, indent: usize, spans: Vec<Span<'static>>, link: Option<usize>) {
        let width = self.width.max(1);
        let indent = indent.min(width / 2);
        let cells: Vec<(char, Style, bool)> = spans
            .iter()
            .enumerate()
            .flat_map(|(index, span)| {
                crate::queue::clean(&span.content)
                    .replace('\t', "    ")
                    .chars()
                    .map(|c| (c, span.style, link == Some(index)))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (number, paragraph) in cells.split(|(c, _, _)| *c == '\n').enumerate() {
            let mut rest = paragraph;
            let mut lead = if number == 0 { 0 } else { indent };
            loop {
                let mut used = lead;
                let fit = rest
                    .iter()
                    .position(|(c, _, _)| {
                        used += c.width().unwrap_or(0);
                        used > width
                    })
                    .unwrap_or(rest.len());
                let (end, next) = if fit == rest.len() {
                    (fit, fit)
                } else {
                    match rest[..=fit].iter().rposition(|(c, _, _)| *c == ' ') {
                        Some(space) if space > 0 => (space, space + 1),
                        _ => (fit.max(1), fit.max(1)),
                    }
                };
                let mut row = vec![Span::raw(" ".repeat(lead))];
                let (mut column, mut piece) = (lead, None);
                for (c, style, linked) in &rest[..end] {
                    if *linked {
                        piece.get_or_insert(column);
                    } else if let Some(start) = piece.take() {
                        self.links
                            .push((self.rows.len(), start as u16, (column - start) as u16));
                    }
                    column += c.width().unwrap_or(0);
                    match row.last_mut() {
                        Some(span) if span.style == *style => span.content.to_mut().push(*c),
                        _ => row.push(Span::styled(c.to_string(), *style)),
                    }
                }
                if let Some(start) = piece {
                    self.links
                        .push((self.rows.len(), start as u16, (column - start) as u16));
                }
                self.rows.push(Line::from(row));
                rest = &rest[next..];
                if rest.is_empty() {
                    break;
                }
                lead = indent;
            }
        }
    }
    fn line(&mut self, spans: Vec<Span<'static>>) {
        self.push(0, spans);
    }
    fn note(&mut self, text: &str) {
        let indent = text.len() - text.trim_start().len();
        self.push(
            indent,
            vec![Span::styled(
                text.to_owned(),
                Style::default().fg(self.t.muted),
            )],
        );
    }
    fn text(&mut self, indent: usize, text: &str, style: Style) {
        self.push(
            indent,
            vec![Span::styled(format!("{}{text}", " ".repeat(indent)), style)],
        );
    }
    fn heading(&mut self, text: &str) {
        self.rows.push(Line::raw(""));
        self.line(vec![Span::styled(
            text.to_owned(),
            bold(self.t.reply_heading),
        )]);
    }
    /// A labelled value whose wrapped rows line up under the value.
    fn field(&mut self, label: &str, value: &[(String, Color)]) {
        let mut spans = vec![self.label(label)];
        spans.extend(
            value
                .iter()
                .map(|(text, color)| Span::styled(text.clone(), Style::default().fg(*color))),
        );
        self.push(self.value_indent(), spans);
    }
    fn label(&self, label: &str) -> Span<'static> {
        Span::styled(
            format!("  {} ", crate::ui::pad(label, LABEL)),
            Style::default().fg(self.t.muted),
        )
    }
    /// Where wrapped rows of a field value start.
    fn value_indent(&self) -> usize {
        if self.width >= 44 { LABEL + 3 } else { 4 }
    }
    fn header(
        &mut self,
        id: Option<&str>,
        status: &str,
        color: Color,
        elapsed: Option<String>,
        title: &str,
    ) {
        let mut spans = Vec::new();
        if let Some(id) = id {
            spans.push(Span::styled(id.to_owned(), bold(self.t.bright)));
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::styled(status.to_owned(), bold(color)));
        if let Some(elapsed) = elapsed {
            spans.push(Span::styled(
                format!(" · {elapsed}"),
                Style::default().fg(self.t.text),
            ));
        }
        self.line(spans);
        self.line(vec![Span::styled(
            title.to_owned(),
            Style::default().fg(self.t.bright),
        )]);
    }
    fn body(&mut self, body: &str) {
        self.heading("Body");
        if body.is_empty() {
            self.note("  Not recorded");
        } else {
            self.text(2, body, Style::default().fg(self.t.text));
        }
    }
}

fn bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
/// Contract codes as English words, clarified where the bare code would mislead.
pub(crate) fn phrase(code: &str) -> String {
    match code {
        "git_query_failed" => "Git query failed".into(),
        "observed_head_unavailable" => "current HEAD unavailable".into(),
        "completion_main_not_recorded" => "main at completion not recorded".into(),
        "check_not_configured" => "no check command configured".into(),
        "cache_for_other_task" => "cache belongs to another task".into(),
        "agent_self_turn" => "agent started its own turn".into(),
        _ => code.replace('_', " "),
    }
}
/// Local date and time of a Unix timestamp.
pub(crate) fn clock(unix: f64) -> String {
    local(unix)
        .map(|tm| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                tm.tm_year + 1900,
                tm.tm_mon + 1,
                tm.tm_mday,
                tm.tm_hour,
                tm.tm_min
            )
        })
        .unwrap_or_else(|| format!("{unix:.0}"))
}
fn time_of_day(unix: f64) -> String {
    local(unix)
        .map(|tm| format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec))
        .unwrap_or_else(|| format!("{unix:.0}"))
}
pub(crate) fn local(unix: f64) -> Option<libc::tm> {
    let seconds = unix.floor() as libc::time_t;
    // SAFETY: localtime_r only writes the provided `tm`, which is plain data.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    (!unsafe { libc::localtime_r(&seconds, &mut tm) }.is_null()).then_some(tm)
}

#[cfg(test)]
mod report_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn run_details_shows_reports_without_marking_the_task_complete() {
        let value = json!({"project":"/synthetic", "task":{"id":"T1","run_id":"r1","status":"running","title":"Example"},
        "evidence":{"scope":"repository_reference","controls_transition":false,"observed_at":0,
            "git":{"state":"available"},"last_check":{"state":"unknown"}},
        "reports":{"state":"available","trace_id":"trace-r1","entries":[
            {"event_id":"review-1","seq":10,"kind":"review.recorded","dispatch_id":"implementation-1","recorded_at":"2026-10-02T00:00:00Z","verdict":"passed","text":"Three examples verified: 5, 1, 0."},
            {"event_id":"closure-1","seq":11,"kind":"controller.note","dispatch_id":null,"recorded_at":"2026-10-02T00:00:01Z","verdict":null,"text":"Cleanup completed; not submitted.\nREPORT END"}
        ]}});
        let data: Detail = serde_json::from_value(value).unwrap();
        let mut view = TaskDetail::new("Current", data.task.clone());
        view.data = Some(Box::new(data));
        let text = view
            .lines(&Theme::default(), None, true, 120)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Controller reports"), "{text}");
        assert!(text.contains("Three examples verified: 5, 1, 0."), "{text}");
        assert!(text.contains("REPORT END"), "{text}");
        assert!(
            text.contains("Declared verdict") && text.contains("passed"),
            "{text}"
        );
        assert!(text.contains("not task acceptance"), "{text}");
        assert_eq!(
            view.data.as_ref().unwrap().task.status.as_deref(),
            Some("running")
        );
    }
    #[test]
    fn overview_puts_current_run_nodes_and_manual_steps_before_raw_records() {
        let data: Detail = serde_json::from_value(json!({"project":"/synthetic", "task":{
            "id":"T1","run_id":"r2","status":"running","title":"Example","t0":1,
            "previous_runs":[{"run_id":"r1","t1":2,"t2":3,"return_history":[{"reason":"OLD RETURN REASON"}]}]},
            "evidence":{"scope":"repository_reference","controls_transition":false,"observed_at":0,
                "git":{"state":"available"},"last_check":{"state":"unknown"}},
            "reports":{"state":"available","observations":[
                {"stage":"route","count":3,"seq":9,"kind":"route.begin","recorded_at":"NOW","status":"Start recorded; result unknown"}],
                "entries":[{"event_id":"r","seq":10,"kind":"review.recorded","dispatch_id":"d","recorded_at":"NOW","verdict":"failed","text":"NEEDS REWORK"}]}
        })).unwrap();
        let mut view = TaskDetail::new("Current", data.task.clone());
        view.data = Some(Box::new(data));
        let text = view
            .lines(&Theme::default(), None, true, 120)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Key events · this run"), "{text}");
        assert!(text.contains("Start recorded; result unknown"), "{text}");
        assert!(text.contains("Controller declared: failed"), "{text}");
        assert!(
            text.contains("Not submitted") && text.contains("Not accepted"),
            "{text}"
        );
        // The raw section heading, not the pointer to it under Look further.
        assert!(
            text.find("Key events · this run").unwrap() < text.find("\nRun records\n").unwrap()
        );
        assert!(text.contains("OLD RETURN REASON"), "{text}");
        assert_eq!(
            view.data.as_ref().unwrap().task.status.as_deref(),
            Some("running")
        );
    }
    #[test]
    fn look_further_counts_reports_by_read_state_and_tracks_the_wrapped_link() {
        let mut data: Detail = serde_json::from_value(json!({"project":"/synthetic", "task":{
            "id":"T1","run_id":"r1","status":"running","title":"Example"},
            "evidence":{"scope":"repository_reference","controls_transition":false,"observed_at":0,
                "git":{"state":"available"},"last_check":{"state":"unknown"}},
            "reports":{"state":"available"}
        }))
        .unwrap();
        let render = |data: &Detail, width| {
            let mut view = TaskDetail::new("Current", data.task.clone());
            view.data = Some(Box::new(data.clone()));
            view.lines_and_links(&Theme::default(), None, true, width)
        };
        let text = |rows: &[Line]| rows.iter().map(ToString::to_string).collect::<Vec<_>>();
        for (state, expected) in [
            ("available", "None recorded for this run"),
            ("not_recorded", "No trace for this run; unknown"),
            ("unavailable", "Unavailable; unknown"),
        ] {
            data.reports.state = state.into();
            let all = text(&render(&data, 120).0).join("\n");
            assert!(all.contains(expected), "{state}: {all}");
            assert!(!all.contains("0 recorded"), "{all}");
            assert!(all.contains("This run below"), "{all}");
            assert!(
                !all.contains("this run ·"),
                "no returns, none counted: {all}"
            );
        }
        // Narrow enough that the link itself wraps: each piece is tracked where it is drawn.
        let (rows, links) = render(&data, 30);
        let rows = text(&rows);
        assert!(links.len() >= 2, "{links:?}\n{}", rows.join("\n"));
        let link: String = links
            .iter()
            .map(|&(row, x, width)| {
                let cells: Vec<char> = rows[row].chars().collect();
                cells[usize::from(x)..usize::from(x + width)]
                    .iter()
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(link, "Open in Telemetry ↗");
        assert!(links.windows(2).all(|w| w[1].0 == w[0].0 + 1), "{links:?}");
    }
    #[test]
    fn headline_distinguishes_a_report_to_review_from_acceptance_and_unknown() {
        let mut data: Detail = serde_json::from_value(json!({"project":"/synthetic", "task":{
            "id":"T1","run_id":"r2","status":"running","title":"Example",
            "previous_runs":[{"run_id":"r1","status":"done","t2":3}]},
            "evidence":{"scope":"repository_reference","controls_transition":false,"observed_at":0,
                "git":{"state":"available"},"last_check":{"state":"unknown"}},
            "reports":{"state":"available","entries":[
                {"event_id":"c","seq":10,"kind":"controller.note","recorded_at":"NOW","text":"CLOSURE BODY"}]}
        })).unwrap();
        let render = |data: &Detail, error: Option<String>| {
            let mut view = TaskDetail::new("Current", data.task.clone());
            view.data = Some(Box::new(data.clone()));
            view.error = error;
            view.lines(&Theme::default(), None, true, 100)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        };
        let text = render(&data, None);
        assert!(text.contains("YOUR REVIEW NEEDED"), "{text}");
        assert!(text.find("YOUR REVIEW NEEDED").unwrap() < text.find("Key events").unwrap());
        assert!(text.contains("CLOSURE BODY"));
        assert_eq!(data.task.status.as_deref(), Some("running"));
        assert!(render(&data, Some("offline".into())).contains("STATUS UNAVAILABLE"));
        data.task.status = Some("awaiting_release".into());
        assert!(render(&data, None).contains("AWAITING YOUR ACCEPTANCE"));
        data.task.status = Some("done".into());
        assert!(render(&data, None).contains("ACCEPTED"));
        data.task.status = Some("running".into());
        data.reports.entries[0].text = None;
        assert!(render(&data, None).contains("COMPLETION UNCONFIRMED"));
        data.reports.entries[0].text = Some("Readable closure".into());
        data.reports.state = "unavailable".into();
        assert!(!render(&data, None).contains("YOUR REVIEW NEEDED"));
        data.reports.state = "available".into();
        data.reports.entries.clear();
        assert!(render(&data, None).contains("COMPLETION UNCONFIRMED"));
        data.reports.state = "unavailable".into();
        assert!(!render(&data, None).contains("YOUR REVIEW NEEDED"));
    }
}
