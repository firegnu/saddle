//! Run details uses only the host's read-only public queries, never its SQLite files.
use super::*;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Reports {
    pub state: String,
    pub trace_id: Option<String>,
    pub detail: Option<String>,
    pub entries: Vec<Report>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Report {
    pub event_id: String,
    pub seq: i64,
    pub kind: String,
    pub dispatch_id: Option<String>,
    pub recorded_at: String,
    pub verdict: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub body_error: Option<String>,
    #[serde(default)]
    pub links: Vec<Value>,
}

pub fn reports(host: Option<&Path>, binding: &Value, cancel: &AtomicBool) -> Reports {
    let Some(host) = host else {
        return Reports {
            state: "unavailable".into(),
            detail: Some("Host query unavailable".into()),
            ..Reports::default()
        };
    };
    match read(host, binding, cancel) {
        Ok(reports) => reports,
        Err(detail) => Reports {
            state: "unavailable".into(),
            detail: Some(detail),
            ..Reports::default()
        },
    }
}

fn read(host: &Path, binding: &Value, cancel: &AtomicBool) -> Result<Reports, String> {
    // A bounded part of the existing detail worker, so closing it remains cancellable.
    let deadline = Instant::now() + Duration::from_secs(2);
    let query = |args: &[&str]| -> Result<Value, String> {
        let value = call(host, args, None, deadline, cancel)?;
        if value["ok"] != true || value["schema_version"] != 1 {
            return Err("Host query failed".into());
        }
        Ok(value)
    };
    let field = |key: &str| {
        binding[key]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Missing run identity".to_owned())
    };
    let list = query(&[
        "list",
        "--kind",
        field("kind")?,
        "--scope",
        field("scope")?,
        "--key",
        field("key")?,
        "--run",
        field("run")?,
    ])?;
    let traces = list["traces"].as_array().ok_or("Invalid trace list")?;
    if traces.is_empty() {
        return Ok(Reports {
            state: "not_recorded".into(),
            ..Reports::default()
        });
    }
    if traces.len() != 1 || traces[0]["binding"] != *binding {
        return Err("No unique trace for this exact run".into());
    }
    let trace = traces[0]["trace_id"]
        .as_str()
        .ok_or("Missing trace identity")?;
    let mut result = Reports {
        state: "available".into(),
        trace_id: Some(trace.into()),
        ..Reports::default()
    };
    let mut after = 0;
    let mut upper = None;
    loop {
        let after_arg = after.to_string();
        let upper_arg = upper.map(|n: i64| n.to_string());
        let mut args = vec![
            "events",
            "--trace-id",
            trace,
            "--after-seq",
            &after_arg,
            "--limit",
            "1000",
        ];
        if let Some(u) = &upper_arg {
            args.extend(["--upper-seq", u]);
        }
        let page = query(&args)?;
        let bound = page["upper_seq"]
            .as_i64()
            .ok_or("Missing event upper bound")?;
        if upper.is_some_and(|u| u != bound) {
            return Err("Event upper bound changed".into());
        }
        upper = Some(bound);
        let events = page["events"].as_array().ok_or("Invalid event list")?;
        for event in events {
            if event["trace_id"] != trace {
                return Err("Event trace mismatch".into());
            }
            if event["kind"] != "review.recorded"
                && !(event["kind"] == "controller.note"
                    && event["payload"]["note_kind"] == "closure")
            {
                continue;
            }
            let mut report: Report = serde_json::from_value(json!({
                "event_id":event["event_id"], "seq":event["seq"], "kind":event["kind"],
                "dispatch_id":event["dispatch_id"], "recorded_at":event["recorded_at"],
                "verdict":event["payload"]["verdict"], "text":null, "links":event["links"]
            }))
            .map_err(|_| "Invalid report metadata")?;
            let body = event["bodies"]
                .as_array()
                .and_then(|b| b.iter().find(|b| b["role"] == "text"));
            let text = match body.and_then(|b| b["sha256"].as_str()) {
                Some(hash) => read_body(host, hash, deadline, cancel),
                None => Err("Report body not recorded".into()),
            };
            match text {
                Ok(text) => report.text = Some(text),
                Err(error) => report.body_error = Some(error),
            }
            result.entries.push(report);
        }
        match page["has_more"].as_bool() {
            Some(false) => break,
            Some(true) => {
                let next = page["next_after_seq"]
                    .as_i64()
                    .ok_or("Missing event cursor")?;
                if next <= after || next > bound {
                    return Err("Invalid event cursor".into());
                }
                after = next;
            }
            None => return Err("Missing pagination state".into()),
        }
    }
    Ok(result)
}

fn read_body(
    host: &Path,
    hash: &str,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<String, String> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err("Read budget exhausted; open Telemetry for the body".into());
    }
    let out = crate::command::run_bounded(
        host.to_str().ok_or("Host unavailable")?,
        &["telemetry", "body", "--sha256", hash],
        None,
        &[HOST_VARIABLE.into()],
        remaining,
        cancel,
        16 * 1024 * 1024,
    )
    .map_err(|_| "Body unavailable; open Telemetry for details")?;
    if !out.status.success() {
        return Err("Body unavailable; open Telemetry for details".into());
    }
    String::from_utf8(out.stdout)
        .map_err(|_| "Non-UTF-8 body; open Telemetry to inspect bytes".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn pages_keep_the_upper_bound_and_missing_bodies_are_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("host");
        std::fs::write(&host, r#"#!/usr/bin/env python3
import json, sys
args = sys.argv[1:]
assert args[0] == 'telemetry'
if args[1] == 'list':
    assert args[2:] == ['--kind','drover.task','--scope','/project','--key','T1','--run','r1']
    print(json.dumps({'schema_version':1,'ok':True,'traces':[{'trace_id':'trace','binding':{'kind':'drover.task','scope':'/project','key':'T1','run':'r1'}}]}))
elif args[1] == 'events':
    first = args[args.index('--after-seq')+1] == '0'
    if not first:
        assert args[args.index('--upper-seq')+1] == '2'
        assert args[args.index('--after-seq')+1] == '1'
    n = 1 if first else 2
    event = {'trace_id':'trace','event_id':str(n),'seq':n,'kind':'controller.note','dispatch_id':None,
        'recorded_at':'2026-10-02T00:00:00Z','payload':{'note_kind':'closure'},'links':[],
        'bodies':[{'role':'text','sha256':'missing'}]}
    print(json.dumps({'schema_version':1,'ok':True,'events':[event],'upper_seq':2,'next_after_seq':n,'has_more':first}))
elif args[1] == 'body':
    print('read failed', file=sys.stderr)
    sys.exit(1)
else:
    raise AssertionError('unexpected write')
"#).unwrap();
        std::fs::set_permissions(&host, std::fs::Permissions::from_mode(0o700)).unwrap();
        let result = reports(
            Some(&host),
            &binding("/project", "T1", "r1"),
            &AtomicBool::new(false),
        );
        assert_eq!(result.state, "available", "{result:?}");
        assert_eq!(result.entries.len(), 2);
        assert!(
            result
                .entries
                .iter()
                .all(|r| r.text.is_none() && r.body_error.is_some())
        );
        let cancelled = reports(
            Some(&host),
            &binding("/project", "T1", "r1"),
            &AtomicBool::new(true),
        );
        assert_eq!(cancelled.state, "unavailable");
    }
}
