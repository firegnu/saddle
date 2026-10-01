//! Commands executed inside the one plugin process; no separate task CLI or daemon.
use crate::{
    core,
    drover::{Operation, Transition},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};
fn string<'a>(p: &'a Value, k: &str) -> Result<&'a str> {
    p[k].as_str()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("{k} is required"))
}
fn position(p: &Value, k: &str) -> Result<usize> {
    let n = p[k]
        .as_u64()
        .context(format!("{k} must be a positive position"))?;
    ensure!(n > 0 && n <= usize::MAX as u64, "invalid position");
    Ok(n as usize - 1)
}
pub fn call(corral: &str, method: &str, p: &Value, cancel: &AtomicBool) -> Result<Value> {
    if method == "projects" {
        return Ok(
            json!({"ok":true,"projects":crate::drover::registered_projects(&crate::config::expand_home("~/.drover/projects"))?}),
        );
    }
    if method == "notifications" {
        let set = match p.get("system_enabled") {
            None => None,
            Some(v) => Some(v.as_bool().context("system_enabled must be boolean")?),
        };
        let pref = core::preference(set)?;
        return Ok(
            json!({"ok":true,"system_enabled":pref.system_enabled,"revision":pref.revision}),
        );
    }
    let project = PathBuf::from(string(p, "project")?);
    ensure!(project.is_absolute(), "project must be absolute");
    if method == "register" {
        return core::register(
            &project,
            string(p, "name")?,
            p["main_agent"].as_str().unwrap_or_default(),
        );
    }
    if method == "list" {
        let mut v = core::list(&project)?;
        for name in ["current", "awaiting"] {
            summarize(&mut v[name]);
        }
        for name in ["pending", "history"] {
            let rows = v[name].as_array_mut().context("Invalid task list")?;
            let total = rows.len();
            let offset = p[format!("{name}_offset")].as_u64().unwrap_or(0) as usize;
            // Each group is paged independently; positions retain their original queue meaning.
            *rows = rows.iter().skip(offset).take(20).cloned().collect();
            for row in rows {
                summarize(row);
            }
            v[format!("{name}_total")] = json!(total);
            v[format!("{name}_offset")] = json!(offset);
        }
        return Ok(v);
    }
    if method == "show" {
        return core::show(&project, string(p, "id")?, corral, cancel);
    }
    let operation = match method {
        "submit" | "accept" | "return" => {
            let id = string(p, "id")?;
            let v = core::list(&project)?;
            let t = [&v["current"], &v["awaiting"]]
                .into_iter()
                .find(|t| t["id"] == id)
                .context("Task is not running or awaiting")?;
            if method == "return" {
                ensure!(
                    p["work_stopped"] == true,
                    "return requires work_stopped=true"
                );
            }
            Operation::Transition {
                project: project.display().to_string(),
                id: id.into(),
                run_id: t["run_id"].as_str().context("missing run identity")?.into(),
                token: string(p, "target_token")?.into(),
                action: match method {
                    "submit" => Transition::Submit,
                    "accept" => Transition::Accept,
                    _ => Transition::Return,
                },
                reason: p["reason"].as_str().unwrap_or_default().into(),
            }
        }
        "dispatch" => Operation::DispatchPending {
            project: project.display().to_string(),
            pos: (position(p, "pos")? + 1) as u64,
            token: string(p, "target_token")?.into(),
            record: match &p["record"] {
                Value::Null => None,
                v => Some(v.as_bool().context("record must be true or false")?),
            },
        },
        "add" => Operation::Add {
            title: string(p, "title")?.into(),
            body: p["body"].as_str().unwrap_or_default().into(),
        },
        "pause" => Operation::Pause(true),
        "resume" => Operation::Pause(false),
        "edit" | "move" | "drop" => {
            string(p, "queue_token")?;
            let pending = serde_json::from_value(core::list(&project)?["pending"].clone())?;
            let index = position(p, "pos")?;
            match method {
                "edit" => Operation::Edit {
                    pending,
                    index,
                    title: string(p, "title")?.into(),
                    body: p["body"].as_str().unwrap_or_default().into(),
                },
                "move" => Operation::Move {
                    pending,
                    index,
                    to: position(p, "to")?,
                },
                _ => Operation::Delete { pending, index },
            }
        }
        _ => anyhow::bail!("Unknown Drover plugin method: {method}"),
    };
    core::execute_expected(
        &project,
        &operation,
        corral,
        cancel,
        if matches!(method, "edit" | "move" | "drop") {
            Some(string(p, "queue_token")?)
        } else {
            None
        },
    )
}
fn summarize(task: &mut Value) {
    if let Some(map) = task.as_object_mut() {
        map.retain(|key, _| {
            matches!(
                key.as_str(),
                "id" | "title"
                    | "status"
                    | "run_id"
                    | "actions"
                    | "notification_key"
                    | "t0"
                    | "t1"
                    | "t2"
            )
        });
    }
}
fn result(value: Result<Value>) -> Value {
    value.unwrap_or_else(|e| {
        let code = e
            .downcast_ref::<crate::drover::TransitionError>()
            .map_or("invalid_request", |e| e.code.as_str());
        json!({"ok":false,"error":{"code":code,"message":format!("{e:#}")}})
    })
}
struct Request {
    id: String,
    method: String,
    params: Value,
}
pub struct Worker {
    send: mpsc::SyncSender<Request>,
    pub results: mpsc::Receiver<(String, Value)>,
    cancel: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Worker {
    pub fn start(corral: String) -> Self {
        let (send, receive) = mpsc::sync_channel::<Request>(32);
        let (tx, results) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let thread = thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let request = match receive.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(r) => r,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                };
                let v = result(call(&corral, &request.method, &request.params, &stop));
                if tx.send((request.id, v)).is_err() {
                    break;
                }
            }
        });
        Self {
            send,
            results,
            cancel,
            thread: Some(thread),
        }
    }
    pub fn request(&self, id: String, method: String, params: Value) -> Result<()> {
        self.send
            .try_send(Request { id, method, params })
            .context("Plugin command queue is full or closed")
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
