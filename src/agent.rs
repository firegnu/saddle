//! Headless, one-shot Corral invocation. Telemetry is composed here, never in Corral.
mod metadata;
mod process;

use crate::telemetry::{self, BodyInput, Capture, Observation, OperationInput, Store};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    trace_id: String,
    dispatch_id: String,
    #[serde(default)]
    basis_event_ids: Vec<String>,
    decision_event_id: Option<String>,
    previous_brief_event_id: Option<String>,
    send_kind: Option<String>,
}

struct Invocation<'a> {
    program: OsString,
    business: &'a [OsString],
    context: Option<Context>,
    brief: Option<PathBuf>,
    timeout: Option<Duration>,
}

fn nonempty(s: &str) -> bool {
    !s.is_empty() && !s.contains('\0')
}

impl<'a> Invocation<'a> {
    fn parse(args: &'a [OsString]) -> Result<Self, &'static str> {
        let separator = args
            .iter()
            .position(|arg| arg == "--")
            .ok_or("missing_separator")?;
        let mut program = None;
        let mut context_file = None;
        let mut brief = None;
        let mut timeout = None;
        let mut seen = std::collections::HashSet::new();
        let mut options = args[..separator].iter();
        while let Some(option) = options.next() {
            let option = option.to_str().ok_or("invalid_option")?;
            if !seen.insert(option) {
                return Err("duplicate_option");
            }
            let value = options.next().ok_or("missing_option_value")?;
            match option {
                "--corral" if !value.is_empty() => program = Some(value.clone()),
                "--timeout-ms" => {
                    let millis = value
                        .to_str()
                        .and_then(|v| v.parse::<u64>().ok())
                        .filter(|v| *v > 0 && *v <= 86_400_000)
                        .ok_or("invalid_timeout")?;
                    timeout = Some(Duration::from_millis(millis));
                }
                "--record-context" | "--brief-file" => {
                    let path = PathBuf::from(value);
                    if !path.is_absolute() {
                        return Err("absolute_path_required");
                    }
                    if option == "--record-context" {
                        context_file = Some(path);
                    } else {
                        if path.to_str().is_none() {
                            return Err("invalid_brief_path");
                        }
                        brief = Some(path);
                    }
                }
                _ => return Err("invalid_option"),
            }
        }
        let business = &args[separator + 1..];
        let kind = business
            .first()
            .and_then(|v| v.to_str())
            .ok_or("missing_command")?;
        if !matches!(kind, "start" | "send" | "reply") {
            return Err("unsupported_command");
        }
        if kind == "reply" && brief.is_some() {
            return Err("reply_has_no_brief");
        }
        let context: Option<Context> = context_file
            .map(|path| {
                let mut file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                    .open(path)
                    .map_err(|_| "unreadable_context")?;
                if !file.metadata().map_err(|_| "unreadable_context")?.is_file() {
                    return Err("invalid_context_file");
                }
                let mut bytes = vec![];
                (&mut file)
                    .take(65537)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "unreadable_context")?;
                if bytes.len() > 65536 {
                    return Err("context_too_large");
                }
                serde_json::from_slice(&bytes).map_err(|_| "invalid_context")
            })
            .transpose()?;
        if let Some(c) = &context {
            if c.schema_version != 1
                || !nonempty(&c.trace_id)
                || !nonempty(&c.dispatch_id)
                || c.basis_event_ids
                    .iter()
                    .chain(c.decision_event_id.iter())
                    .chain(c.previous_brief_event_id.iter())
                    .any(|v| !nonempty(v))
                || c.basis_event_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != c.basis_event_ids.len()
            {
                return Err("invalid_context");
            }
            if (kind == "send"
                && !matches!(
                    c.send_kind.as_deref(),
                    Some("initial" | "followup" | "rework")
                ))
                || (kind != "send" && c.send_kind.is_some())
                || (kind == "reply"
                    && (!c.basis_event_ids.is_empty()
                        || c.decision_event_id.is_some()
                        || c.previous_brief_event_id.is_some()))
                || (c.previous_brief_event_id.is_some() && brief.is_none())
            {
                return Err("invalid_context_combination");
            }
        }
        Ok(Self {
            program: match program {
                Some(p) => p,
                None => crate::agent_program::configured()
                    .map_err(|_| "invalid_agent_program_config")?
                    .into_os_string(),
            },
            business,
            context,
            brief,
            timeout,
        })
    }
    fn kind(&self) -> &str {
        self.business[0].to_str().expect("validated command")
    }
    fn operation(&self) -> OperationInput {
        let c = self.context.as_ref().expect("recording requested");
        OperationInput {
            schema_version: c.schema_version,
            trace_id: c.trace_id.clone(),
            dispatch_id: c.dispatch_id.clone(),
            kind: format!("agent.{}", self.kind()),
            producer: "saddle.agent".into(),
            basis_event_ids: c.basis_event_ids.clone(),
            decision_event_id: c.decision_event_id.clone(),
            previous_brief_event_id: c.previous_brief_event_id.clone(),
        }
    }
}

fn now() -> Value {
    json!(
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("UTC timestamp")
    )
}
fn gap(payload: &mut Value, role: &str, reason: &str) {
    payload["gaps"]
        .as_array_mut()
        .expect("gaps array")
        .push(json!({"role":role,"reason":reason}));
}
fn body(
    bytes: &[u8],
    role: &str,
    payload: &mut Value,
    files: &mut Vec<tempfile::NamedTempFile>,
) -> Vec<BodyInput> {
    if bytes.len() as u64 > telemetry::MAX_BODY_BYTES {
        gap(payload, role, "too_large");
        return vec![];
    }
    let file = tempfile::NamedTempFile::new().and_then(|mut f| {
        f.write_all(bytes)?;
        Ok(f)
    });
    match file {
        Ok(file) => {
            let path = file.path().to_path_buf();
            files.push(file);
            vec![BodyInput {
                role: role.into(),
                path,
            }]
        }
        Err(_) => {
            gap(payload, role, "write_failed");
            vec![]
        }
    }
}

fn prepare(invocation: &Invocation<'_>, store: &Store) -> telemetry::Result<Capture> {
    let mut files = vec![];
    let begin = metadata::begin(invocation, &mut files)?;
    store.prepare_operation(invocation.operation(), begin, invocation.brief.clone())
}

fn end(
    kind: &str,
    outcome: Value,
    output: &[u8],
    output_gap: Option<&str>,
    files: &mut Vec<tempfile::NamedTempFile>,
) -> Observation {
    let parsed = output_gap
        .is_none()
        .then(|| {
            serde_json::from_slice::<Value>(output)
                .ok()
                .filter(Value::is_object)
        })
        .flatten();
    let mut payload = json!({"outcome":outcome,"gaps":[]});
    let keys: &[&str] = if kind == "reply" {
        &["name", "instance", "at"]
    } else {
        &[
            "name",
            "instance",
            "at",
            "confirmed",
            "pending",
            "merged_with_draft",
        ]
    };
    for key in keys {
        let value = parsed.as_ref().and_then(|v| v.get(key));
        let valid = value.filter(|v| match *key {
            "at" => v.is_number(),
            "confirmed" | "pending" | "merged_with_draft" => v.is_boolean(),
            _ => v.as_str().is_some_and(nonempty),
        });
        payload[key] = valid.cloned().unwrap_or(Value::Null);
        if valid.is_none() {
            gap(
                &mut payload,
                key,
                output_gap.unwrap_or(if value.is_some_and(|v| !v.is_null()) {
                    "unrecognized"
                } else {
                    "not_available"
                }),
            );
        }
    }
    let mut bodies = vec![];
    if kind == "reply" {
        payload["association"] = json!("not_proven");
        if let Some(text) = parsed.as_ref().and_then(|v| v["text"].as_str()) {
            bodies = body(text.as_bytes(), "reply", &mut payload, files);
        } else {
            gap(&mut payload, "reply", output_gap.unwrap_or("unrecognized"));
        }
    }
    Observation {
        observed_at: now(),
        payload,
        bodies,
    }
}

fn start_boundary(call_id: &str) -> String {
    format!(
        "saddle-telemetry: {}\n",
        json!({"schema_version":1,"call_id":call_id,"final":false})
    )
}

fn rejected(
    code: &str,
    call_id: &str,
    signals: Option<&process::Signals>,
    deadline: Option<Instant>,
) -> i32 {
    let receipt = json!({"schema_version":1,"call_id":call_id,"final":true,"executed":false,"operation_id":null,"begin":"not_requested","end":"not_requested","error":code,"gaps":[]});
    process::write_receipt(
        format!("{}saddle-telemetry: {receipt}\n", start_boundary(call_id)).as_bytes(),
        signals,
        deadline,
    );
    125
}

pub fn run(args: &[OsString]) -> i32 {
    let started = Instant::now();
    if args == [OsString::from("--help")] {
        println!(
            "saddle agent [--corral PROGRAM] [--timeout-ms MS] [--record-context /abs/context.json] [--brief-file /abs/task.md] -- start|send|reply <Corral arguments>"
        );
        return 0;
    }
    // This transport boundary is independent of optional persisted operations. Never pass
    // the ID to Corral, including when telemetry is disabled or invocation is rejected.
    let call_id = uuid::Uuid::new_v4().to_string();
    let signals = process::Signals::install().ok();
    let invocation = match Invocation::parse(args) {
        Ok(v) => v,
        Err(e) => return rejected(e, &call_id, signals.as_ref(), None),
    };
    let deadline = invocation.timeout.map(|d| started + d);
    let Some(signals) = signals else {
        return rejected("signal_setup_failed", &call_id, None, deadline);
    };
    // Do not start a telemetry write whose 100 ms lock budget exceeds the total deadline.
    // As with Store, disk IO/fsync is not a hard real-time guarantee; the caller owns the outer timeout.
    let can_record = || {
        signals.received() == 0
            && deadline.is_none_or(|d| {
                d.saturating_duration_since(Instant::now()) >= Duration::from_millis(100)
            })
    };
    let mut receipt = json!({"schema_version":1,"call_id":call_id,"final":true,"executed":false,"operation_id":null,"begin":"not_requested","end":"not_requested","gaps":[]});
    let mut capture = None;
    let mut store = None;
    if invocation.context.is_some() {
        // Only explicit context/association errors refuse execution. Observation/storage errors
        // after this point are gaps and must not prevent or repeat a business action.
        let checked = Store::from_environment().and_then(|s| {
            s.check_operation(&invocation.operation())?;
            Ok(s)
        });
        match checked {
            Err(e) if e.status == "invalid" => {
                return rejected(
                    "invalid_context_or_association",
                    &call_id,
                    Some(&signals),
                    deadline,
                );
            }
            Err(e) => {
                receipt["begin"] = json!(e.status);
                receipt["end"] = json!(e.status);
            }
            Ok(s) => {
                if can_record() {
                    match prepare(&invocation, &s) {
                        Ok(c) => {
                            receipt["operation_id"] = json!(c.operation_id());
                            receipt["gaps"] = json!(c.gaps());
                            receipt["begin"] = if can_record() {
                                match s.record_begin(&c) {
                                    Ok(v) => v["status"].clone(),
                                    Err(e) => json!(e.status),
                                }
                            } else {
                                json!("unavailable")
                            };
                            capture = Some(c);
                            store = Some(s);
                        }
                        Err(e) => {
                            receipt["begin"] = json!(if e.status == "disabled" {
                                "disabled"
                            } else {
                                "unavailable"
                            });
                            receipt["end"] = receipt["begin"].clone();
                            gap(
                                &mut receipt,
                                "capture",
                                if e.status == "invalid" {
                                    "unrecognized"
                                } else {
                                    "write_failed"
                                },
                            );
                        }
                    }
                } else {
                    receipt["begin"] = json!("unavailable");
                    receipt["end"] = json!("unavailable");
                    gap(&mut receipt, "capture", "not_available");
                }
            }
        }
    }
    let boundary = start_boundary(&call_id);
    let result = process::run(
        &invocation,
        capture.is_some(),
        &signals,
        deadline,
        boundary.as_bytes(),
    );
    receipt["executed"] = json!(result.executed);
    receipt["outcome"] = result.outcome.clone();
    if let Some(reason) = result.output_gap {
        gap(&mut receipt, "output", reason);
    }
    if let (Some(s), Some(c)) = (&store, &capture) {
        if can_record() {
            let mut files = vec![];
            let observation = end(
                invocation.kind(),
                result.outcome,
                &result.stdout,
                result.output_gap,
                &mut files,
            );
            receipt["gaps"]
                .as_array_mut()
                .unwrap()
                .extend(observation.payload["gaps"].as_array().unwrap().clone());
            receipt["end"] = if can_record() {
                match s.record_end_before(c, observation, deadline) {
                    Ok(v) => v["status"].clone(),
                    Err(e) => json!(e.status),
                }
            } else {
                json!("unavailable")
            };
        } else {
            receipt["end"] = json!("unavailable");
            gap(&mut receipt, "end", "not_available");
        }
    }
    // Interrupted/incomplete output cannot carry a trustworthy terminal receipt.
    // In particular, never block cancellation trying to write into a full stderr pipe.
    if !result.streams_complete {
        return result.code;
    }
    // The relay owns the start boundary after spawn. Pre-spawn failures have no business
    // output, so emit the same ordered pair here (or a partial pair on cancellation).
    let separator = if !result.executed {
        boundary.as_str()
    } else if result.stderr_terminated {
        ""
    } else {
        "\n"
    };
    process::write_receipt(
        format!("{separator}saddle-telemetry: {receipt}\n").as_bytes(),
        Some(&signals),
        deadline,
    );
    result.code
}
