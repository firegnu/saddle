//! Host-owned, headless telemetry. No task files or agent services are consulted.
mod blobs;
mod capture;
mod cli;
mod events;
mod model;
mod query;
mod storage;
mod validate;
pub use blobs::MAX_BODY_BYTES;
pub use capture::{Capture, Observation, OperationInput};
pub use cli::run;
pub use model::*;
pub use query::{BindingFilter, EventQuery};

use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    pub status: &'static str,
    pub code: &'static str,
    pub message: String,
}

impl Error {
    fn new(status: &'static str, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid", "invalid_input", message)
    }
    fn unavailable(message: impl Into<String>) -> Self {
        Self::new("unavailable", "storage_unavailable", message)
    }
    fn disabled() -> Self {
        Self::new(
            "disabled",
            "recording_disabled",
            "recording is disabled or the capture generation expired",
        )
    }
    fn closed() -> Self {
        Self::new("disabled", "trace_closed", "trace is permanently closed")
    }
    fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", "id_conflict", message)
    }
    pub fn exit_code(&self) -> i32 {
        match self.status {
            "invalid" => 2,
            "conflict" => 3,
            "disabled" => 4,
            _ => 1,
        }
    }
    pub fn receipt(&self) -> Value {
        json!({"schema_version":1,"ok":false,"status":self.status,
            "error":{"code":self.code,"message":self.message}})
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::unavailable("SQLite operation failed")
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::unavailable("file operation failed")
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::invalid(format!(
            "invalid JSON or fields at line {}, column {}",
            error.line(),
            error.column()
        ))
    }
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Explicit root makes callers and tests independent of process-global environment.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn from_environment() -> Result<Self> {
        let state = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
            .ok_or_else(|| Error::unavailable("HOME is unavailable"))?;
        Ok(Self::new(state.join("saddle/telemetry")))
    }
    fn reader(&self) -> Result<Option<Connection>> {
        let path = self.root.join("telemetry.sqlite3");
        if !path.try_exists()? {
            return Ok(None);
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.busy_timeout(Duration::from_millis(100))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if !matches!(version, 1 | 2) {
            return Err(Error::unavailable("unsupported database version"));
        }
        Ok(Some(conn))
    }
    pub fn settings(&self) -> Result<Value> {
        let conn = self.reader()?;
        let (enabled, generation) = if let Some(conn) = &conn {
            conn.query_row(
                "SELECT enabled,generation FROM recording_policy WHERE singleton=1",
                [],
                |r| Ok((r.get::<_, bool>(0)?, r.get::<_, i64>(1)?)),
            )?
        } else {
            (false, 0)
        };
        Ok(
            json!({"schema_version":1,"ok":true,"initialized":conn.is_some(),
            "enabled":enabled,"generation":generation,"sqlite_version":rusqlite::version()}),
        )
    }
}
