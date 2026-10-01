//! Generic, same-build plugin catalog and transport. No business plugin is imported here.
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub type Catalog = &'static [&'static dyn saddle_core_plugin::CorePlugin];

/// A failed or partial start must never be followed by a terminal receipt.
pub(super) struct Receipts {
    call_id: String,
    started: bool,
}
impl Receipts {
    pub fn start() -> Self {
        let call_id = uuid::Uuid::new_v4().to_string();
        let started = write_receipt(&json!({"schema_version":1,"call_id":call_id,"final":false}));
        Self { call_id, started }
    }
    pub fn finish(&self, mut receipt: Value) {
        if self.started {
            receipt["schema_version"] = json!(1);
            receipt["call_id"] = json!(self.call_id);
            receipt["final"] = json!(true);
            write_receipt(&receipt);
        }
    }
}

/// Fixed per-line budget, including writable-but-failing descriptors. Never change flags
/// on an inherited open file description (dup would share those flags too).
fn write_receipt(value: &Value) -> bool {
    let line = format!("saddle-plugin: {value}\n");
    let mut bytes = line.as_bytes();
    let deadline = Instant::now() + Duration::from_secs(1);
    while !bytes.is_empty() {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return false;
        };
        let mut ready = libc::pollfd {
            fd: libc::STDERR_FILENO,
            events: libc::POLLOUT,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut ready, 1, remaining.as_millis().min(1000) as i32) };
        if result < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if result <= 0 || ready.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return false;
        }
        if ready.revents & libc::POLLOUT == 0 {
            continue;
        }
        let count = unsafe { libc::write(ready.fd, bytes.as_ptr().cast(), bytes.len().min(512)) };
        if count <= 0 {
            return false;
        }
        bytes = &bytes[count as usize..];
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Enabled,
    Disabled,
    Conflict,
}
pub fn state(registry: &super::registry::Registry, id: &str) -> State {
    if registry.entries.iter().any(|e| e.id == id) {
        State::Conflict
    } else if registry.core.get(id).is_some_and(|e| e.enabled) {
        State::Enabled
    } else {
        State::Disabled
    }
}
pub fn registry(path: std::path::PathBuf, catalog: Catalog) -> super::registry::Registry {
    super::registry::Registry::with_reserved(path, catalog.iter().map(|p| p.manifest().id))
}
