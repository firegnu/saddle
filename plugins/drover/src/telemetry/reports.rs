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
    pub observations: Vec<Node>,
}

/// The latest recorded event in each category, not a completion gate or agent live state.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Node {
    pub stage: String,
    pub count: usize,
    pub seq: i64,
    pub kind: String,
    pub recorded_at: String,
    pub status: String,
}

fn observation(event: &Value) -> Option<Node> {
    let kind = event["kind"].as_str()?;
    let stage = kind
        .strip_suffix(".begin")
        .or_else(|| kind.strip_suffix(".end"))?;
    if !matches!(
        stage,
        "agent.send" | "route" | "agent.start" | "agent.reply"
    ) {
        return None;
    }
    let p = &event["payload"];
    let has_body = |role| {
        event["bodies"]
            .as_array()
            .is_some_and(|b| b.iter().any(|b| b["role"] == role))
    };
    let status = if kind.ends_with(".begin") {
        "已记录开始；结果未知".into()
    } else if p["outcome"]["kind"] == "exited" && p["outcome"]["exit_code"] == 0 {
        match stage {
            "agent.send" if p["pending"] == true => "已排队；交付未确认",
            "agent.send" if p["confirmed"] == true && p["pending"] == false => {
                "已确认交付（非任务完成）"
            }
            "agent.start" if p["pending"] == true => "启动已排队；结果未知",
            "agent.start" if p["name"].is_string() && p["instance"].is_string() => {
                "已返回 Agent 身份（非实现完成）"
            }
            "agent.reply" if has_body("reply") => "已读取回复（任务关联未证实）",
            "route" if has_body("response") && has_body("suggestion") => {
                "已保存响应与建议（非主控决定）"
            }
            _ => "命令退出 0；结果证据不完整",
        }
        .into()
    } else {
        match p["outcome"]["kind"].as_str() {
            Some("exited") => format!("命令退出 {}", p["outcome"]["exit_code"]),
            Some("timed_out") => "超时 timed_out；结果未知".into(),
            Some("signaled") => "命令被信号终止；结果未知".into(),
            Some("spawn_failed") => "命令启动失败".into(),
            _ => "结果未知".into(),
        }
    };
    Some(Node {
        stage: stage.into(),
        count: 1,
        seq: event["seq"].as_i64()?,
        kind: kind.into(),
        recorded_at: event["recorded_at"].as_str()?.into(),
        status,
    })
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
            if let Some(mut node) = observation(event) {
                if let Some(old) = result
                    .observations
                    .iter_mut()
                    .find(|old| old.stage == node.stage)
                {
                    if node.seq > old.seq {
                        node.count += old.count;
                        *old = node;
                    } else {
                        old.count += 1;
                    }
                } else {
                    result.observations.push(node);
                }
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
        assert args[args.index('--upper-seq')+1] == '4'
        assert args[args.index('--after-seq')+1] == '2'
    n = 1 if first else 3
    event = {'trace_id':'trace','event_id':str(n),'seq':n,'kind':'controller.note','dispatch_id':None,
        'recorded_at':'2026-10-02T00:00:00Z','payload':{'note_kind':'closure'},'links':[],
        'bodies':[{'role':'text','sha256':'missing'}]}
    route = dict(event, event_id='route'+str(n), seq=n+1, kind='route.end', payload={'outcome':{'kind':'timed_out'}}, bodies=[])
    print(json.dumps({'schema_version':1,'ok':True,'events':[event,route],'upper_seq':4,'next_after_seq':n+1,'has_more':first}))
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
        assert_eq!(result.observations.len(), 1);
        assert_eq!(result.observations[0].count, 2);
        assert_eq!(result.observations[0].seq, 4);
        assert!(result.observations[0].status.contains("timed_out"));
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
    #[test]
    fn observation_distinguishes_missing_pending_timeout_and_recorded_results() {
        let node = |kind: &str, payload: Value, roles: &[&str]| {
            observation(&json!({
                "kind":kind,"payload":payload,"seq":1,"recorded_at":"NOW",
                "bodies":roles.iter().map(|r| json!({"role":r})).collect::<Vec<_>>()
            }))
            .unwrap()
            .status
        };
        assert!(node("route.begin", json!({}), &[]).contains("结果未知"));
        for outcome in ["timed_out", "signaled", "unknown"] {
            assert!(
                node("route.end", json!({"outcome":{"kind":outcome}}), &[]).contains("结果未知")
            );
        }
        let exited = json!({"outcome":{"kind":"exited","exit_code":0}});
        assert!(node("agent.reply.end", exited.clone(), &[]).contains("证据不完整"));
        assert!(node("agent.reply.end", exited.clone(), &["reply"]).contains("任务关联未证实"));
        assert!(node("route.end", exited.clone(), &["response"]).contains("证据不完整"));
        assert!(
            node("route.end", exited.clone(), &["response", "suggestion"]).contains("已保存响应")
        );
        let mut pending = exited;
        pending["confirmed"] = json!(true);
        pending["pending"] = json!(true);
        assert!(node("agent.send.end", pending.clone(), &[]).contains("交付未确认"));
        pending["pending"] = Value::Null;
        assert!(node("agent.send.end", pending.clone(), &[]).contains("证据不完整"));
        pending["pending"] = json!(false);
        assert!(node("agent.send.end", pending, &[]).contains("非任务完成"));
        assert!(
            node(
                "agent.reply.end",
                json!({"outcome":{"kind":"exited","exit_code":1}}),
                &[]
            )
            .contains("退出 1")
        );
    }
}
