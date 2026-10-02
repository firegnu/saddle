//! Optional task telemetry through Saddle's public CLI, at the path the host gives every process
//! plugin in `SADDLE_HOST_BIN`. Drover never opens the telemetry store; what happens here is
//! reported apart from the task's own delivery and record.
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The binding kind Drover declares; scope is the canonical project root, key the task number.
pub const KIND: &str = "drover.task";
/// Trace and controller handoff must exist within this, before any delivery starts.
pub const IDENTITY_BUDGET: Duration = Duration::from_millis(300);
/// Finding the run's trace and appending a transition after Drover saved the task state.
pub const FOLLOW_BUDGET: Duration = Duration::from_secs(1);
/// Only Drover calls the host; Corral and the agent entry never receive it.
pub const HOST_VARIABLE: &str = "SADDLE_HOST_BIN";
mod reports;
pub use reports::{Report, Reports, reports};

const PREFIX: &[u8] = b"saddle-telemetry: ";

/// The Saddle executable this plugin runs under, as the host announced it.
pub fn host() -> Option<PathBuf> {
    std::env::var_os(HOST_VARIABLE)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
/// RFC 3339 UTC, as the telemetry store accepts it.
pub fn timestamp(at: SystemTime) -> Value {
    time::OffsetDateTime::from(at)
        .format(&time::format_description::well_known::Rfc3339)
        .map_or(Value::Null, Value::String)
}
/// A Drover record time (seconds since the epoch) as a timestamp.
pub fn record_time(t: f64) -> Value {
    Duration::try_from_secs_f64(t).map_or(Value::Null, |d| timestamp(UNIX_EPOCH + d))
}
pub fn binding(scope: &str, key: &str, run: &str) -> Value {
    json!({"kind":KIND,"scope":scope,"key":key,"run":run})
}

/// One `saddle telemetry` call before `deadline`: its JSON answer, or why there is none.
fn call(
    host: &Path,
    args: &[&str],
    input: Option<&Value>,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let program = host.to_str().ok_or("host_unavailable")?;
    let file = input
        .map(|input| {
            let mut file = tempfile::NamedTempFile::new()?;
            file.write_all(input.to_string().as_bytes())?;
            Ok::<_, std::io::Error>(file)
        })
        .transpose()
        .map_err(|_| "unavailable")?;
    let mut all = vec!["telemetry"];
    all.extend(args);
    if let Some(file) = &file {
        all.extend(["--input", file.path().to_str().ok_or("unavailable")?]);
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err("budget_exhausted".into());
    }
    match crate::command::run_bounded(
        program,
        &all,
        None,
        &[HOST_VARIABLE.into()],
        remaining,
        cancel,
        1 << 20,
    ) {
        Ok(out) => serde_json::from_slice(&out.stdout).map_err(|_| "unavailable".into()),
        Err(_) if Instant::now() >= deadline => Err("budget_exhausted".into()),
        Err(_) => Err("unavailable".into()),
    }
}
/// A record write: Ok when stored (or the same record already was), else the receipt status.
fn write(
    host: &Path,
    args: &[&str],
    input: &Value,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<String, String> {
    let receipt = call(host, args, Some(input), deadline, cancel)?;
    let status = receipt["status"]
        .as_str()
        .unwrap_or("unavailable")
        .to_owned();
    if receipt["ok"] == true && matches!(status.as_str(), "stored" | "duplicate") {
        Ok(status)
    } else {
        Err(status)
    }
}

/// The run's trace and the controller handoff dispatch made for this delivery.
pub struct Identity {
    pub trace_id: String,
    pub dispatch_id: String,
}
/// Creates the trace and its controller_handoff dispatch within the identity budget, with IDs
/// chosen here first. `Err` says why there is no record context (disabled, unavailable,
/// budget_exhausted, invalid, conflict); the delivery then goes the plain way.
pub fn prepare(
    host: &Path,
    binding: &Value,
    label: &str,
    cancel: &AtomicBool,
) -> Result<Identity, String> {
    let deadline = Instant::now() + IDENTITY_BUDGET;
    let trace_id = new_id();
    write(
        host,
        &["trace", "create"],
        &json!({"schema_version":1,"trace_id":trace_id,"origin":"task","label":label,"binding":binding}),
        deadline,
        cancel,
    )?;
    let dispatch_id = new_id();
    write(
        host,
        &["dispatch", "create"],
        &json!({"schema_version":1,"dispatch_id":dispatch_id,"trace_id":trace_id,"kind":"controller_handoff"}),
        deadline,
        cancel,
    )?;
    Ok(Identity {
        trace_id,
        dispatch_id,
    })
}
/// The record context file for the agent entry: private to this user, removed when dropped.
pub fn context_file(identity: &Identity) -> std::io::Result<(tempfile::NamedTempFile, PathBuf)> {
    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(
        json!({"schema_version":1,"trace_id":identity.trace_id,"dispatch_id":identity.dispatch_id,"send_kind":"initial"})
            .to_string()
            .as_bytes(),
    )?;
    let path = file.path().canonicalize()?;
    Ok((file, path))
}
/// The separately marked part of the handoff that carries the record context: identifiers only.
pub fn appendix(identity: &Identity, task: &str, run: &str) -> String {
    format!(
        "\n\n---\nSaddle telemetry record context (identifiers only; not part of the task and not an authorization):\ntrace_id: {}\ndispatch_id: {}\ntask: {task}\nrun: {run}\n",
        identity.trace_id, identity.dispatch_id
    )
}

/// What the agent entry left: its exit code when it exited, stdout, and its own terminal
/// receipt only when that paired with its start boundary.
pub struct Sent {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub receipt: Option<Value>,
}
/// Sends once through `saddle agent` in its own process group. Nothing here retries or falls
/// back to a plain send: once started, the result is whatever the receipt and Corral say.
#[allow(clippy::too_many_arguments)]
pub fn send(
    host: &Path,
    corral: &str,
    cwd: &Path,
    agent: &str,
    message: &str,
    context: &Path,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Sent {
    let (Some(program), Some(context)) = (host.to_str(), context.to_str()) else {
        return Sent {
            code: None,
            stdout: Vec::new(),
            receipt: None,
        };
    };
    let args = [
        "agent",
        "--corral",
        corral,
        "--record-context",
        context,
        "--",
        "send",
        agent,
        message,
    ];
    match crate::command::run_group(
        program,
        &args,
        Some(cwd),
        &[HOST_VARIABLE.into()],
        timeout,
        cancel,
    ) {
        Ok(out) => Sent {
            code: out.status.and_then(|s| s.code()),
            receipt: out.complete.then(|| receipt(&out.stderr)).flatten(),
            stdout: out.stdout,
        },
        Err(_) => Sent {
            code: None,
            stdout: Vec::new(),
            receipt: None,
        },
    }
}
fn boundary(line: &[u8]) -> Option<Value> {
    let value: Value = serde_json::from_slice(line.strip_prefix(PREFIX)?).ok()?;
    let id = value["call_id"].as_str()?;
    let valid = value["schema_version"] == 1
        && uuid::Uuid::parse_str(id).is_ok_and(|u| u.get_version_num() == 4);
    valid.then_some(value)
}
/// The agent entry's terminal receipt, only if the first complete line from byte 0 is its start
/// boundary and the last complete line closes the same call. Anything else is no receipt.
pub fn receipt(stderr: &[u8]) -> Option<Value> {
    let first = boundary(&stderr[..stderr.iter().position(|b| *b == b'\n')?])?;
    let body = stderr.strip_suffix(b"\n")?;
    let start = body.iter().rposition(|b| *b == b'\n')? + 1;
    let last = boundary(&body[start..])?;
    (first["final"] == false
        && last["final"] == true
        && last["call_id"] == first["call_id"]
        && last["executed"].is_boolean())
    .then_some(last)
}

/// Appends a task.transition Drover saved for a run with a trace: given, or found by the run's
/// full binding (never the latest trace). Without one nothing is appended or made up.
pub fn transition(
    host: Option<&Path>,
    binding: &Value,
    trace: Option<&str>,
    states: (&str, &str),
    committed: Value,
    reason: Option<&str>,
    cancel: &AtomicBool,
) -> Value {
    let Some(host) = host else {
        return json!({"status":"host_unavailable","trace_id":null});
    };
    let deadline = Instant::now() + FOLLOW_BUDGET;
    let trace = match trace {
        Some(trace) => trace.to_owned(),
        None => {
            let found = call(
                host,
                &[
                    "list",
                    "--kind",
                    KIND,
                    "--scope",
                    binding["scope"].as_str().unwrap_or_default(),
                    "--key",
                    binding["key"].as_str().unwrap_or_default(),
                    "--run",
                    binding["run"].as_str().unwrap_or_default(),
                ],
                None,
                deadline,
                cancel,
            );
            let traces = match &found {
                Ok(list) if list["ok"] == true => list["traces"].as_array(),
                _ => None,
            };
            match traces.map(|t| t.as_slice()) {
                Some([trace]) if trace["trace_id"].is_string() => {
                    trace["trace_id"].as_str().unwrap().to_owned()
                }
                Some([]) => return json!({"status":"not_recorded","trace_id":null}),
                _ => {
                    return json!({"status":found.err().unwrap_or("unavailable".into()),"trace_id":null});
                }
            }
        }
    };
    // Keep the file alive until append has snapshotted the original committed reason.
    let reason_file = match reason
        .map(|text| {
            let mut file = tempfile::NamedTempFile::new()?;
            file.write_all(text.as_bytes())?;
            Ok::<_, std::io::Error>(file)
        })
        .transpose()
    {
        Ok(file) => file,
        Err(_) => return json!({"status":"unavailable","trace_id":trace}),
    };
    let bodies: Vec<Value> = reason_file
        .iter()
        .map(|file| json!({"role":"reason","path":file.path()}))
        .collect();
    let event = json!({"schema_version":1,"event_id":new_id(),"trace_id":trace,"kind":"task.transition",
        "observed_at":timestamp(SystemTime::now()),"producer":"drover","evidence_kind":"plugin_statement",
        "payload":{"binding":binding,"from":states.0,"to":states.1,"business_committed_at":committed},
        "links":[],"bodies":bodies});
    let status = match write(host, &["append"], &event, deadline, cancel) {
        Ok(status) | Err(status) => status,
    };
    json!({"status":status,"trace_id":trace})
}
