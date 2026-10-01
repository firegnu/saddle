//! Host-owned Recorder adapter. Plugins never receive storage, identities or generations.
use crate::telemetry::{self, BodyInput, Capture, Observation, OperationInput, Store};
use saddle_core_plugin::{Begin, Captured, End, Missing, Recorder};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    trace_id: String,
    dispatch_id: String,
    #[serde(default)]
    basis_event_ids: Vec<String>,
    previous_brief_event_id: Option<String>,
}
fn context(path: &Path, brief: bool) -> Result<Context, &'static str> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "unreadable_context")?;
    if !file.metadata().map_err(|_| "unreadable_context")?.is_file() {
        return Err("invalid_context_file");
    }
    let mut bytes = vec![];
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "unreadable_context")?;
    if bytes.len() > 65536 {
        return Err("context_too_large");
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_context")?;
    if value.get("decision_event_id").is_some() || value.get("send_kind").is_some() {
        return Err("invalid_context_combination");
    }
    let context: Context = serde_json::from_slice(&bytes).map_err(|_| "invalid_context")?;
    if context.schema_version != 1
        || [&context.trace_id, &context.dispatch_id]
            .into_iter()
            .chain(context.basis_event_ids.iter())
            .chain(context.previous_brief_event_id.iter())
            .any(|s| s.is_empty() || s.contains('\0'))
        || context
            .basis_event_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != context.basis_event_ids.len()
    {
        return Err("invalid_context");
    }
    if context.previous_brief_event_id.is_some() && !brief {
        return Err("invalid_context_combination");
    }
    Ok(context)
}
fn now() -> Value {
    json!(
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("UTC timestamp")
    )
}
fn gap(value: &mut Value, role: &str, reason: &str) {
    let entry = json!({"role":role,"reason":reason});
    let gaps = value["gaps"].as_array_mut().expect("gaps array");
    if !gaps.contains(&entry) {
        gaps.push(entry);
    }
}

pub(super) struct HostRecorder {
    store: Option<Store>,
    input: Option<OperationInput>,
    brief: Option<PathBuf>,
    capture: Option<Capture>,
    called: bool,
    report: Value,
}
impl HostRecorder {
    pub fn check(
        id: &str,
        context_file: Option<&Path>,
        brief: Option<PathBuf>,
    ) -> Result<Self, &'static str> {
        let mut this = Self {
            store: None,
            input: None,
            brief,
            capture: None,
            called: false,
            report: json!({"operation_id":null,"begin":"not_requested","end":"not_requested","gaps":[]}),
        };
        let Some(path) = context_file else {
            return Ok(this);
        };
        let context = context(path, this.brief.is_some())?;
        let input = OperationInput {
            schema_version: context.schema_version,
            trace_id: context.trace_id,
            dispatch_id: context.dispatch_id,
            kind: "route".into(),
            producer: format!("saddle.plugin.{id}"),
            basis_event_ids: context.basis_event_ids,
            decision_event_id: None,
            previous_brief_event_id: context.previous_brief_event_id,
        };
        match Store::from_environment().and_then(|store| {
            store.check_operation(&input)?;
            Ok(store)
        }) {
            Ok(store) => {
                this.store = Some(store);
                this.input = Some(input);
                this.status("not_reached");
            }
            Err(e) if e.status == "invalid" => return Err("invalid_context_or_association"),
            Err(e) => this.status(if e.status == "disabled" {
                "disabled"
            } else {
                "unavailable"
            }),
        }
        Ok(this)
    }
    fn status(&mut self, status: &str) {
        self.report["begin"] = json!(status);
        self.report["end"] = json!(status);
    }
    pub fn finish(mut self, outcome: Value, end: Option<End>) -> Value {
        if let (Some(store), Some(capture)) = (&self.store, &self.capture) {
            let mut payload = json!({"outcome":outcome,"gaps":[]});
            let (response, suggestion) = match end {
                Some(End::Route(end)) => (end.response, end.suggestion),
                _ => (
                    Captured::Missing(Missing::NotAvailable),
                    Captured::Missing(Missing::NotAvailable),
                ),
            };
            let mut files = vec![];
            let mut bodies = body(response, "response", &mut payload, &mut files);
            bodies.extend(body(suggestion, "suggestion", &mut payload, &mut files));
            self.report["gaps"]
                .as_array_mut()
                .unwrap()
                .extend(payload["gaps"].as_array().unwrap().clone());
            self.report["end"] = match store.record_end(
                capture,
                Observation {
                    observed_at: now(),
                    payload,
                    bodies,
                },
            ) {
                Ok(v) => v["status"].clone(),
                Err(e) => json!(e.status),
            };
        }
        self.report
    }
}
impl Recorder for HostRecorder {
    fn begin(&mut self, begin: Begin) {
        if self.called {
            gap(&mut self.report, "capture", "unrecognized");
            return;
        }
        self.called = true;
        // Inactive preflight states are final for this invocation, regardless of begin calls.
        let Some(input) = self.input.take() else {
            return;
        };
        let Begin::Route(begin) = begin else {
            self.status("unavailable");
            gap(&mut self.report, "capture", "unrecognized");
            return;
        };
        let mut payload = json!({"router_model":begin.router_model,"router_version":begin.router_version,"rules_version":begin.rules_version,"gaps":[]});
        for version in ["router_version", "rules_version"] {
            if payload[version].is_null() {
                gap(&mut payload, version, "not_available");
            }
        }
        let mut files = vec![];
        let mut bodies = body(
            Captured::Bytes(begin.summary),
            "summary",
            &mut payload,
            &mut files,
        );
        bodies.extend(body(
            Captured::Bytes(begin.request),
            "request",
            &mut payload,
            &mut files,
        ));
        let store = self.store.as_ref().expect("checked store");
        match store.prepare_operation(
            input,
            Observation {
                observed_at: now(),
                payload,
                bodies,
            },
            self.brief.take(),
        ) {
            Ok(capture) => {
                self.report["operation_id"] = json!(capture.operation_id());
                self.report["gaps"]
                    .as_array_mut()
                    .unwrap()
                    .extend_from_slice(capture.gaps());
                self.report["begin"] = match store.record_begin(&capture) {
                    Ok(v) => v["status"].clone(),
                    Err(e) => json!(e.status),
                };
                // Keep this exact credential even if record_begin failed. Store owns end
                // recovery and rejects stale generations; never prepare again at end.
                self.capture = Some(capture);
            }
            Err(e) => {
                self.status(if e.status == "disabled" {
                    "disabled"
                } else {
                    "unavailable"
                });
                gap(
                    &mut self.report,
                    "capture",
                    if e.status == "invalid" {
                        "unrecognized"
                    } else {
                        "write_failed"
                    },
                );
            }
        }
    }
}

// Like agent's private body helper: private 0600 files live through Store staging. Keeping
// this small adapter local avoids changing agent execution/cancellation semantics.
fn body(
    captured: Captured,
    role: &str,
    payload: &mut Value,
    files: &mut Vec<tempfile::NamedTempFile>,
) -> Vec<BodyInput> {
    let bytes = match captured {
        Captured::Bytes(bytes) => bytes,
        Captured::Missing(reason) => {
            gap(
                payload,
                role,
                match reason {
                    Missing::TooLarge => "too_large",
                    Missing::Unrecognized => "unrecognized",
                    _ => "not_available",
                },
            );
            return vec![];
        }
    };
    if bytes.len() as u64 > telemetry::MAX_BODY_BYTES {
        gap(payload, role, "too_large");
        return vec![];
    }
    match tempfile::NamedTempFile::new().and_then(|mut f| {
        f.write_all(&bytes)?;
        Ok(f)
    }) {
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
