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
    pub(crate) fn lines(
        &self,
        t: &Theme,
        live: Option<(&'static str, &Task)>,
        queried: bool,
        width: usize,
    ) -> Vec<Line<'static>> {
        let mut out = Out {
            t,
            width,
            rows: Vec::new(),
        };
        let Some(data) = &self.data else {
            let (group, task) = live.unwrap_or((self.group, &self.task));
            let (status, color) = crate::queue::task_status(t, group, task.status.as_deref());
            out.header(task.id.as_deref(), status, color, None, &task.title);
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
                return out.rows;
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
            return out.rows;
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
        out.overview(task, &data.reports);
        out.reports(&data.reports);
        out.records(task);
        for (index, run) in task.previous_runs.iter().enumerate() {
            out.heading(&format!("Previous run {}", index + 1));
            out.records(run);
        }
        out.evidence(&data.evidence);
        out.body(&task.body);
        out.rows
    }
}

struct Out<'a> {
    t: &'a Theme,
    width: usize,
    rows: Vec<Line<'static>>,
}
impl Out<'_> {
    fn overview(&mut self, task: &Task, reports: &crate::telemetry::Reports) {
        self.heading("本次关键节点");
        let missing = match reports.state.as_str() {
            "available" => "未记录（不代表未执行）",
            "not_recorded" => "本次未记录遥测；结果未知",
            _ => "遥测查询不可用；结果未知",
        };
        for (stage, label) in [
            ("agent.send", "消息交付"),
            ("route", "路由建议"),
            ("agent.start", "Agent 启动"),
            ("agent.reply", "Agent 回复"),
        ] {
            let value = reports
                .observations
                .iter()
                .find(|n| n.stage == stage)
                .filter(|_| reports.state == "available")
                .map(|n| {
                    format!(
                        "{} · seq {} · {} · 共{}条",
                        n.status, n.seq, n.recorded_at, n.count
                    )
                })
                .unwrap_or_else(|| missing.into());
            self.field(label, &[(value, self.t.text)]);
        }
        for (kind, label) in [("review.recorded", "审查"), ("controller.note", "收尾")] {
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
                            .map(|v| format!("主控声明：{v}"))
                            .unwrap_or_else(|| "已有主控报告（非成功判定）".into()),
                        r.seq,
                        r.recorded_at,
                        if r.text.is_none() {
                            " · 正文不可用"
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
                "人工提交",
                &task.t1,
                "尚未提交",
                matches!(task.status.as_deref(), Some("awaiting_release" | "done")),
            ),
            (
                "人工验收",
                &task.t2,
                "尚未验收",
                task.status.as_deref() == Some("done"),
            ),
        ] {
            let text = at
                .as_ref()
                .and_then(|v| v.as_f64())
                .map(|v| format!("已记录 · {}", clock(v)))
                .unwrap_or_else(|| {
                    if at.is_some() {
                        "记录时间不可识别".into()
                    } else if recorded {
                        "已完成此流转（时间未记录）".into()
                    } else {
                        empty.into()
                    }
                });
            self.field(label, &[(text, self.t.text)]);
        }
        self.note("各类仅显示最新记录，不代表整阶段成功；审查/收尾是主控声明。完整报告见下方，完整链路见遥测。");
        let count: usize = reports.observations.iter().map(|n| n.count).sum();
        if count > reports.observations.len() || reports.entries.len() > 2 {
            self.note("含多条事件/报告：不同委派或迟到事件可能交错，请在遥测核对完整顺序。");
        }
        let next = match task.status.as_deref() {
            Some("running") => {
                "核对下方审查和收尾报告；确认后手工 Submit for review。记录不足时先查看主控。"
            }
            Some("awaiting_release") => {
                "等待人工验收；通过后 Accept，需要返工则 Return to pending。"
            }
            Some("done") => "任务已验收，无需继续推进。",
            Some("pending") => "任务待派发；若有退回，先核对下方历史原因。",
            _ => "按任务当前状态处理，不根据遥测记录自动推进。",
        };
        self.field("下一步", &[(next.into(), self.t.focus)]);
    }

    fn reports(&mut self, reports: &crate::telemetry::Reports) {
        self.heading("Controller reports · this run");
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
        self.note("All recorded reports below; later reports may revise earlier ones. Full chain: Telemetry.");
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
            self.field("Event", &[(report.event_id.clone(), self.t.text)]);
            self.field(
                "Dispatch",
                &[(
                    report
                        .dispatch_id
                        .clone()
                        .unwrap_or_else(|| "trace-level".into()),
                    self.t.text,
                )],
            );
        }
        if let Some(trace) = &reports.trace_id {
            self.field("Trace", &[(trace.clone(), self.t.text)]);
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
        let width = self.width.max(1);
        let indent = indent.min(width / 2);
        let cells: Vec<(char, Style)> = spans
            .iter()
            .flat_map(|span| {
                crate::queue::clean(&span.content)
                    .replace('\t', "    ")
                    .chars()
                    .map(|c| (c, span.style))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (number, paragraph) in cells.split(|(c, _)| *c == '\n').enumerate() {
            let mut rest = paragraph;
            let mut lead = if number == 0 { 0 } else { indent };
            loop {
                let mut used = lead;
                let fit = rest
                    .iter()
                    .position(|(c, _)| {
                        used += c.width().unwrap_or(0);
                        used > width
                    })
                    .unwrap_or(rest.len());
                let (end, next) = if fit == rest.len() {
                    (fit, fit)
                } else {
                    match rest[..=fit].iter().rposition(|(c, _)| *c == ' ') {
                        Some(space) if space > 0 => (space, space + 1),
                        _ => (fit.max(1), fit.max(1)),
                    }
                };
                let mut row = vec![Span::raw(" ".repeat(lead))];
                for (c, style) in &rest[..end] {
                    match row.last_mut() {
                        Some(span) if span.style == *style => span.content.to_mut().push(*c),
                        _ => row.push(Span::styled(c.to_string(), *style)),
                    }
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
        const LABEL: usize = 13;
        let mut spans = vec![Span::styled(
            format!("  {} ", crate::ui::pad(label, LABEL)),
            Style::default().fg(self.t.muted),
        )];
        spans.extend(
            value
                .iter()
                .map(|(text, color)| Span::styled(text.clone(), Style::default().fg(*color))),
        );
        self.push(if self.width >= 44 { LABEL + 3 } else { 4 }, spans);
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
                {"stage":"route","count":3,"seq":9,"kind":"route.begin","recorded_at":"NOW","status":"已记录开始；结果未知"}],
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
        assert!(text.contains("本次关键节点"), "{text}");
        assert!(text.contains("已记录开始；结果未知"), "{text}");
        assert!(text.contains("主控声明：failed"), "{text}");
        assert!(
            text.contains("尚未提交") && text.contains("尚未验收"),
            "{text}"
        );
        assert!(text.find("本次关键节点").unwrap() < text.find("Run records").unwrap());
        assert!(text.contains("OLD RETURN REASON"), "{text}");
        assert_eq!(
            view.data.as_ref().unwrap().task.status.as_deref(),
            Some("running")
        );
    }
}
