//! Deliberately narrow metadata extraction. Never persist the full argv or environment.
use super::*;
use std::os::unix::ffi::OsStrExt;

fn unknown(payload: &mut Value, key: &str, reason: &str) {
    payload[key] = Value::Null;
    gap(payload, key, reason);
}

fn parameters(args: &[OsString]) -> Option<Value> {
    let program = std::path::Path::new(args.first()?).file_name()?.to_str()?;
    if !matches!(program, "codex" | "claude" | "pi" | "omp") {
        return None;
    }
    let mut values = json!({});
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].to_str()?;
        let (flag, inline) = arg
            .split_once('=')
            .map_or((arg, None), |(k, v)| (k, Some(v)));
        let key = match flag {
            "-m" | "--model" => Some("model"),
            "--effort" if program == "claude" => Some("effort"),
            "-c" | "--config" if program == "codex" => Some("config"),
            // Known value-taking options are skipped, never inspected as other flags.
            "--api-key" | "--settings" | "--permission-mode" | "--provider" | "--extension" => {
                Some("ignored")
            }
            "--yolo"
            | "--dangerously-bypass-approvals-and-sandbox"
            | "--dangerously-skip-permissions"
                if inline.is_none() =>
            {
                None
            }
            _ => return None,
        };
        if let Some(key) = key {
            let value = match inline {
                Some(v) => v,
                None => {
                    i += 1;
                    args.get(i)?.to_str()?
                }
            };
            if key == "config" {
                let config = toml::from_str::<toml::Table>(value).ok()?;
                for (key, field) in [("model", "model"), ("model_reasoning_effort", "effort")] {
                    if let Some(value) = config.get(key) {
                        let value = value.as_str().filter(|v| nonempty(v))?;
                        values[field] = json!(value);
                    }
                }
            } else if key != "ignored" {
                if !nonempty(value) {
                    return None;
                }
                values[key] = json!(value);
            }
        }
        i += 1;
    }
    Some(values)
}

pub(super) fn begin(
    invocation: &Invocation<'_>,
    files: &mut Vec<tempfile::NamedTempFile>,
) -> telemetry::Result<Observation> {
    let args = invocation.business;
    let mut bodies = vec![];
    let mut payload = json!({"gaps":[]});
    match invocation.kind() {
        "start" => {
            let mut cwd = None;
            let mut prompt = None;
            let mut labels = json!({});
            let mut agent = None;
            let mut recognized = true;
            let mut i = 2; // command and name
            while i < args.len() {
                let flag = args[i].to_str();
                if flag == Some("--") {
                    agent = Some(&args[i + 1..]);
                    break;
                }
                if flag == Some("--unique") {
                    i += 1;
                    continue;
                }
                if !matches!(flag, Some("--cwd" | "--prompt" | "--env" | "--label")) {
                    recognized = false;
                    break;
                }
                i += 1;
                let Some(value) = args.get(i) else {
                    recognized = false;
                    break;
                };
                match flag {
                    Some("--cwd") => cwd = Some(value),
                    Some("--prompt") => prompt = Some(value),
                    Some("--label") => {
                        if let Some((key, value)) = value.to_str().and_then(|s| s.split_once('=')) {
                            if matches!(key, "role" | "model" | "effort") {
                                if nonempty(value) {
                                    labels[key] = json!(value);
                                } else {
                                    labels = Value::Null;
                                }
                            }
                        } else {
                            labels = Value::Null;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if recognized {
                if let Some(cwd) = cwd.and_then(|v| v.to_str()).filter(|v| nonempty(v)) {
                    payload["cwd"] = json!(cwd);
                } else {
                    unknown(
                        &mut payload,
                        "cwd",
                        if cwd.is_some() {
                            "unrecognized"
                        } else {
                            "not_supplied"
                        },
                    );
                }
                if let Some(program) = agent
                    .and_then(|a| a.first())
                    .and_then(|v| v.to_str())
                    .filter(|v| nonempty(v))
                {
                    payload["agent_program"] = json!(program);
                } else {
                    unknown(&mut payload, "agent_program", "unrecognized");
                }
                if let Some(params) = agent.and_then(parameters) {
                    payload["explicit_parameters"] = params;
                } else {
                    unknown(&mut payload, "explicit_parameters", "unrecognized");
                }
                if labels.is_null() {
                    unknown(&mut payload, "labels", "unrecognized");
                } else {
                    payload["labels"] = labels;
                }
                if let Some(prompt) = prompt {
                    bodies = body(prompt.as_bytes(), "prompt", &mut payload, files);
                } else {
                    gap(&mut payload, "prompt", "not_supplied");
                }
            } else {
                for key in ["cwd", "agent_program", "explicit_parameters", "labels"] {
                    unknown(&mut payload, key, "unrecognized");
                }
                gap(&mut payload, "prompt", "unrecognized");
            }
        }
        kind => {
            // Unsupported business syntax is a telemetry gap, never a reason to replay/refuse it.
            let target = args
                .get(1)
                .and_then(|v| v.to_str())
                .filter(|v| nonempty(v) && !v.starts_with('-'))
                .ok_or_else(|| telemetry::Error::invalid("unrecognized target_name"))?;
            payload["target_name"] = json!(target);
            if kind == "send" {
                payload["send_kind"] = json!(invocation.context.as_ref().unwrap().send_kind);
                if let Some(message) = args.get(2) {
                    bodies = body(message.as_bytes(), "message", &mut payload, files);
                } else {
                    gap(&mut payload, "message", "unrecognized");
                }
            }
        }
    }
    Ok(Observation {
        observed_at: now(),
        payload,
        bodies,
    })
}
