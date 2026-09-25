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

/// The Tasks area's detail page for one task, fixed when it opens.
pub struct TaskDetail {
    /// Unique per opening, so a reopened page never takes an older result.
    pub(crate) seq: u64,
    /// Where the list had the task, and its list copy, when the page opened.
    pub(crate) group: &'static str,
    pub(crate) task: Task,
    /// The last successful `drover show`; kept, and labelled stale, when a refresh fails.
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
            seq: OPENED.fetch_add(1, Ordering::Relaxed),
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
    /// `live` is where the list has the task now; `queried` whether `drover show` covers it.
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
                "From the queue list; drover show covers started tasks only."
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
        let group = match task.location.as_str() {
            "current" => "Current",
            "awaiting" => "Awaiting",
            _ => "History",
        };
        let (status, color) = crate::queue::task_status(t, group, Some(&task.status));
        let current = task.location == "current";
        out.header(
            Some(&task.id),
            status,
            color,
            Some(match data.timing.elapsed_seconds {
                Some(seconds) if current => format!("{} so far", duration(seconds)),
                Some(seconds) => duration(seconds),
                None => "elapsed unknown".into(),
            }),
            &task.title,
        );
        match &self.error {
            Some(error) => {
                out.line(vec![Span::styled(
                    format!("Refresh failed: {error}"),
                    Style::default().fg(t.agent_error),
                )]);
                out.line(vec![Span::styled(
                    format!(
                        "Showing stale details from {}. Retrying every 5s.",
                        time_of_day(data.observed_at)
                    ),
                    Style::default().fg(t.agent_blocked),
                )]);
            }
            None => out.note(&format!(
                "Updated {} · refreshes every 5s",
                time_of_day(data.observed_at)
            )),
        }
        let attention = &data.attention;
        match attention.state.as_str() {
            "awaiting_release" => out.line(vec![Span::styled(
                "Awaiting release",
                bold(t.agent_blocked),
            )]),
            "suggested" => out.line(vec![Span::styled(
                "Suggested attention: main agent idle with unmet checks (inferred)",
                bold(t.agent_stalled),
            )]),
            _ => {}
        }
        for warning in &data.warnings {
            let text = match warning.code.as_str() {
                "snapshot_changed" => "Inconsistent snapshot: changed during read",
                "snapshot_verification_unavailable" => "Snapshot not verified",
                code => code,
            };
            out.line(vec![Span::styled(
                format!(
                    "{text} ({}); the next refresh retries",
                    warning.sources.join(", ")
                ),
                Style::default().fg(t.agent_blocked),
            )]);
        }

        out.checks(data);
        out.last_check(data);
        out.progress(data);
        out.route(data);
        out.records(data);
        out.rows
    }
}

struct Out<'a> {
    t: &'a Theme,
    width: usize,
    rows: Vec<Line<'static>>,
}
impl Out<'_> {
    /// Completion checks recomputed now, or why history has none.
    fn checks(&mut self, data: &Detail) {
        let t = self.t;
        let task = &data.task;
        let completion = &data.completion;
        self.heading(if completion.scope == "current_repository" {
            "Completion checks · recomputed now"
        } else {
            "Completion checks"
        });
        match &completion.rows {
            Some(rows) => {
                if task.location == "awaiting" {
                    self.note("The recorded done stands; recomputed checks do not change it.");
                }
                for row in rows {
                    let (mark, state, color) = match row.state.as_str() {
                        "met" => ("✓", "met", t.agent_idle),
                        "unmet" => ("✗", "unmet", t.agent_blocked),
                        "unavailable" => ("?", "unavailable", t.muted),
                        "not_applicable" => ("–", "not applicable", t.dim),
                        "not_run" => ("·", "not run", t.muted),
                        other => ("·", other, t.muted),
                    };
                    let mut spans = vec![
                        Span::styled(format!("  {mark} "), bold(color)),
                        Span::raw(capitalized(&phrase(&row.id))),
                        Span::raw("  "),
                        Span::styled(state.to_owned(), bold(color)),
                    ];
                    if !matches!(row.state.as_str(), "met" | "unmet") {
                        spans.push(Span::styled(
                            format!(" · {}", phrase(&row.reason)),
                            Style::default().fg(t.muted),
                        ));
                    }
                    self.push(4, spans);
                    self.text(4, &row.why, Style::default().fg(t.text));
                }
            }
            None => {
                self.push(
                    4,
                    vec![Span::styled(
                        format!(
                            "  Not saved · {}",
                            completion
                                .unavailable_reason
                                .as_deref()
                                .map(phrase)
                                .unwrap_or_else(|| "not recorded".into())
                        ),
                        Style::default().fg(t.text),
                    )],
                );
                self.note("  Current checks are not applied to past tasks.");
            }
        }
    }
    /// The last check-command record, with what it does not prove.
    fn last_check(&mut self, data: &Detail) {
        let t = self.t;
        let check = &data.last_check;
        self.heading("Last check");
        let (label, color) = match check.status.as_str() {
            "valid" => ("Valid", t.agent_idle),
            "stale" => ("Stale", t.agent_blocked),
            "missing" => ("Missing", t.muted),
            "invalid" => ("Invalid", t.agent_error),
            "unavailable" => ("Unavailable", t.muted),
            other => (other, t.muted),
        };
        let mut reasons: Vec<String> = check.stale_reasons.iter().map(|r| phrase(r)).collect();
        reasons.extend(check.unavailable_reason.as_deref().map(phrase));
        let mut spans = vec![Span::styled(format!("  {label}"), bold(color))];
        if !reasons.is_empty() {
            spans.push(Span::styled(
                format!(" · {}", reasons.join(", ")),
                Style::default().fg(t.muted),
            ));
        }
        self.push(4, spans);
        if let Some(record) = &check.record {
            let (result, color) = match record.ok {
                Some(true) => ("passed", t.agent_idle),
                Some(false) => ("failed", t.agent_error),
                None => ("result unknown", t.muted),
            };
            self.field(
                "Result",
                &[
                    (result.into(), color),
                    (format!(" · {}", clock(record.t)), t.muted),
                ],
            );
            self.field("Command", &[(record.cmd.clone(), t.text)]);
            self.text(4, &record.why, Style::default().fg(t.text));
        }
        match check.status.as_str() {
            "valid" => {
                self.note("  Last saved result for this task, main and command; not run now.")
            }
            "missing" | "invalid" => {
                self.note("  No valid record; this does not mean it never ran.")
            }
            _ => {}
        }
        if check.scope == "retained_sample" {
            self.note("  Retained sample, not the check at completion.");
        }
    }
    /// Git range and main progress, with unknown values explained.
    fn progress(&mut self, data: &Detail) {
        let t = self.t;
        let task = &data.task;
        let current = task.location == "current";
        let git = &data.git;
        let unknown = |key: &str| match git.unavailable_reasons.get(key) {
            Some(reason) => format!("unknown · {}", phrase(reason)),
            None => "unknown".into(),
        };
        self.heading("Progress");
        match task.location.as_str() {
            "current" => {}
            "awaiting" => self.field(
                "Waiting",
                &[(
                    data.timing
                        .release_wait_seconds
                        .map(|s| format!("{} so far", duration(s)))
                        .unwrap_or_else(|| "unknown".into()),
                    t.text,
                )],
            ),
            _ => self.field(
                "Release wait",
                &[(
                    data.timing
                        .release_wait_seconds
                        .map(duration)
                        .unwrap_or_else(|| "unknown".into()),
                    t.text,
                )],
            ),
        }
        self.field(
            "Commits",
            &[(
                git.range_commits
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| unknown("range_commits")),
                t.text,
            )],
        );
        self.note(if current {
            "  Counts start..HEAD, not only this task's commits."
        } else {
            "  Counts start..end HEAD, not only this task's commits."
        });
        self.field(
            "Main",
            &[(
                git.main_commits_since_start
                    .map(|n| format!("+{n} since start"))
                    .unwrap_or_else(|| unknown("main_commits_since_start")),
                t.text,
            )],
        );
        self.field(
            "Main tip",
            &[(
                git.main_tip_committed_at
                    .map(|at| format!("committed {} (main now)", clock(at)))
                    .unwrap_or_else(|| unknown("main_tip_committed_at")),
                t.text,
            )],
        );
        let sha = |value: &Option<String>, missing: &str| {
            value
                .as_deref()
                .map(|s| s.chars().take(7).collect())
                .unwrap_or_else(|| missing.to_owned())
        };
        self.field(
            "Start",
            &[(
                format!(
                    "HEAD {} · main {}",
                    sha(&git.start_head, "not recorded"),
                    sha(&git.start_main, "not recorded")
                ),
                t.text,
            )],
        );
        if !current {
            self.field(
                "End HEAD",
                &[(
                    format!("{} (at done; not main)", sha(&git.end_head, "not recorded")),
                    t.text,
                )],
            );
        }
        self.field(
            "Now",
            &[(
                format!(
                    "HEAD {} · main {}",
                    sha(&git.observed_head, "unavailable"),
                    sha(&git.observed_main, "unavailable")
                ),
                t.text,
            )],
        );
    }
    /// Routing from the current task file, hold, and the attention inference.
    fn route(&mut self, data: &Detail) {
        let t = self.t;
        let attention = &data.attention;
        self.heading("Route, hold & attention");
        let route = match &data.routing {
            None => "none (no routed task file)".to_owned(),
            Some(routing) => format!(
                "{} · {}{} · {}",
                routing.tier,
                if routing.cross {
                    "cross review"
                } else {
                    "no cross review"
                },
                if routing.overridden {
                    " · overridden"
                } else {
                    ""
                },
                if routing.source == "task_file_now" {
                    "from the current task file".to_owned()
                } else {
                    format!("from {}", phrase(&routing.source))
                }
            ),
        };
        self.field("Route", &[(route, t.text)]);
        let hold = &data.hold;
        self.field(
            "Hold",
            &[(
                format!(
                    "{}{}",
                    match hold.enabled {
                        Some(true) => "On · stop after done".to_owned(),
                        Some(false) => "Off".to_owned(),
                        None => format!(
                            "Unknown · {}",
                            hold.unavailable_reason
                                .as_deref()
                                .map(phrase)
                                .unwrap_or_else(|| "not recorded".into())
                        ),
                    },
                    if hold.scope == "task_end_events" {
                        " (at task end)"
                    } else {
                        ""
                    }
                ),
                t.text,
            )],
        );
        self.field(
            "Attention",
            &[(
                format!(
                    "{} · {}{}",
                    capitalized(&phrase(&attention.state)),
                    phrase(&attention.reason),
                    if attention.inference && attention.state == "suggested" {
                        " (inferred)"
                    } else {
                        ""
                    }
                ),
                t.text,
            )],
        );
        if !attention.unmet_rows.is_empty() {
            let rows: Vec<_> = attention.unmet_rows.iter().map(|r| phrase(r)).collect();
            self.field("Unmet", &[(rows.join(", "), t.text)]);
        }
        if let Some(agent) = &attention.agent {
            let known = |value: &Option<String>| value.clone().unwrap_or_else(|| "unknown".into());
            let mut text = format!(
                "{} · {} · via {}",
                known(&agent.name),
                known(&agent.state),
                known(&agent.last_input_source)
            );
            if let Some(idle) = agent.idle_for {
                text += &format!(" · idle {}", duration(idle));
            }
            self.field("Main agent", &[(text, t.text)]);
        }
    }
    /// Recorded start, finish and release times, then the body.
    fn records(&mut self, data: &Detail) {
        let t = self.t;
        let task = &data.task;
        let current = task.location == "current";
        self.heading("Records");
        let at = |value: Option<f64>| value.map(clock).unwrap_or_else(|| "not recorded".into());
        self.field("Started", &[(at(data.timing.started_at), t.text)]);
        if !current {
            let label = if task.status == "dropped" {
                "Dropped"
            } else {
                "Done"
            };
            self.field(label, &[(at(data.timing.ended_at), t.text)]);
        }
        if task.location == "awaiting" {
            self.field("Released", &[("not yet".into(), t.text)]);
        } else if task.location == "history" && task.status == "done" {
            self.field(
                "Released",
                &[(
                    data.timing
                        .released_at
                        .map(clock)
                        .unwrap_or_else(|| "no release recorded".into()),
                    t.text,
                )],
            );
        }
        if let Some(reason) = &task.reason {
            self.field("Reason", &[(reason.clone(), t.text)]);
        }
        self.body(&task.body);
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
        self.line(vec![Span::styled(text.to_owned(), bold(self.t.text))]);
    }
    /// A labelled value whose wrapped rows line up under the value.
    fn field(&mut self, label: &str, value: &[(String, Color)]) {
        const LABEL: usize = 13;
        let mut spans = vec![Span::styled(
            format!("  {}", crate::ui::pad(label, LABEL)),
            Style::default().fg(self.t.muted),
        )];
        spans.extend(
            value
                .iter()
                .map(|(text, color)| Span::styled(text.clone(), Style::default().fg(*color))),
        );
        self.push(if self.width >= 44 { LABEL + 2 } else { 4 }, spans);
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
fn phrase(code: &str) -> String {
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
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
fn duration(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {}s", s / 60, s % 60),
        3600..86400 => format!("{}h {}m", s / 3600, s % 3600 / 60),
        _ => format!("{}d {}h", s / 86400, s % 86400 / 3600),
    }
}
/// Local date and time of a Unix timestamp.
fn clock(unix: f64) -> String {
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
fn local(unix: f64) -> Option<libc::tm> {
    let seconds = unix.floor() as libc::time_t;
    // SAFETY: localtime_r only writes the provided `tm`, which is plain data.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    (!unsafe { libc::localtime_r(&seconds, &mut tm) }.is_null()).then_some(tm)
}
