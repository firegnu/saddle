use super::{model::*, *};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
    time::Instant,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub(super) fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("UTC timestamp")
}
pub(super) fn fingerprint(value: &Value) -> String {
    let mut canonical = value.clone();
    canonical.sort_all_objects();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).expect("JSON value"))
    )
}
pub(super) fn receipt(status: &str, id: &str) -> Value {
    json!({"schema_version":1,"ok":true,"status":status,"id":id})
}
pub(super) fn private_dir(path: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    Ok(())
}

fn remaining_busy(conn: &Connection, started: Instant) -> Result<()> {
    conn.busy_timeout(Duration::from_millis(100).saturating_sub(started.elapsed()))?;
    Ok(())
}
pub(super) fn policy(conn: &Connection, trace: Option<&str>) -> Result<(bool, i64)> {
    if let Some(id) = trace {
        conn.query_row(
            "SELECT capture_enabled,generation FROM traces WHERE trace_id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| Error::invalid("unknown trace_id"))
    } else {
        Ok(conn.query_row(
            "SELECT enabled,generation FROM recording_policy WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }
}
pub(super) fn enabled(conn: &Connection, trace: Option<&str>) -> Result<(i64, Option<i64>)> {
    let (on, generation) = policy(conn, None)?;
    if !on {
        return Err(Error::disabled());
    }
    let trace_generation = if let Some(trace) = trace {
        let (on, generation) = policy(conn, Some(trace))?;
        if !on {
            return Err(Error::disabled());
        }
        Some(generation)
    } else {
        None
    };
    Ok((generation, trace_generation))
}
pub(super) fn existing(
    conn: &Connection,
    table: &str,
    column: &str,
    id: &str,
    value: &Value,
) -> Result<Option<Value>> {
    let old: Option<String> = conn
        .query_row(
            &format!("SELECT record_json FROM {table} WHERE {column}=?"),
            [id],
            |r| r.get(0),
        )
        .optional()?;
    match old {
        None => Ok(None),
        Some(old) if serde_json::from_str::<Value>(&old)? == *value => {
            Ok(Some(receipt("duplicate", id)))
        }
        Some(_) => Err(Error::conflict("ID already has different content")),
    }
}

impl Store {
    pub(super) fn writer(&self, initialize: bool) -> Result<Connection> {
        if rusqlite::version_number() < 3_051_003 {
            return Err(Error::unavailable(
                "SQLite lacks the required WAL reset fix",
            ));
        }
        let path = self.root.join("telemetry.sqlite3");
        if !initialize && !path.try_exists()? {
            return Err(Error::disabled());
        }
        if initialize {
            private_dir(&self.root)?;
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => {
                    file.sync_all()?;
                    fs::File::open(&self.root)?.sync_all()?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
        }
        let started = Instant::now();
        let mut conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        // Opening/schema probes do not each spend another full busy timeout.
        // Initialization and the first write share the remaining budget.
        remaining_busy(&conn, started)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        remaining_busy(&conn, started)?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        remaining_busy(&conn, started)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version == 0 && initialize {
            remaining_busy(&conn, started)?;
            let tables: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE type='table'",
                [],
                |r| r.get(0),
            )?;
            if tables != 0 {
                return Err(Error::unavailable("unrecognized database"));
            }
            remaining_busy(&conn, started)?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            remaining_busy(&conn, started)?;
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
            if version == 0 {
                let tables: i64 = tx.query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type='table'",
                    [],
                    |r| r.get(0),
                )?;
                if tables != 0 {
                    return Err(Error::unavailable("unrecognized database"));
                }
                tx.execute_batch(include_str!("schema.sql"))?;
                tx.execute("INSERT INTO recording_policy VALUES(1,0,0,?)", [now()])?;
            } else if version != 1 {
                return Err(Error::unavailable("unsupported database version"));
            }
            tx.commit()?;
        } else if version != 1 {
            return Err(Error::unavailable("unsupported database version"));
        }
        remaining_busy(&conn, started)?;
        Ok(conn)
    }

    pub fn set_recording(&self, trace: Option<&str>, input: SettingInput) -> Result<Value> {
        version(input.schema_version)?;
        nonempty(&input.actor)?;
        if trace.is_some() && self.reader()?.is_none() {
            return Err(Error::invalid("unknown trace_id"));
        }
        let mut conn = self.writer(trace.is_none())?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (current, generation) = policy(&tx, trace)?;
        let id = trace.unwrap_or("global");
        if current == input.enabled {
            return Ok(receipt("duplicate", id));
        }
        let at = now();
        let generation = generation + 1;
        if let Some(trace) = trace {
            tx.execute(
                "UPDATE traces SET capture_enabled=?,generation=? WHERE trace_id=?",
                params![input.enabled, generation, trace],
            )?;
        } else {
            tx.execute(
                "UPDATE recording_policy SET enabled=?,generation=?,updated_at=? WHERE singleton=1",
                params![input.enabled, generation, at],
            )?;
        }
        let event_id = uuid::Uuid::new_v4().to_string();
        let record = json!({"schema_version":1,"event_id":event_id,"trace_id":trace,"dispatch_id":null,"operation_id":null,
            "kind":"recording.changed","observed_at":at,"producer":"saddle","evidence_kind":"system_control",
            "source_auth":"system_control","payload":{"enabled":input.enabled,"generation":generation,"actor":input.actor,"changed_at":at},"links":[],"bodies":[]});
        tx.execute("INSERT INTO events(event_id,trace_id,kind,recorded_at,observed_at,fingerprint,record_json) VALUES(?,?,'recording.changed',?,?,?,?)",
            params![event_id,trace,at,at,fingerprint(&record),record.to_string()])?;
        tx.commit()?;
        Ok(receipt("stored", id))
    }

    pub fn create_trace(&self, input: TraceInput) -> Result<Value> {
        input.validate()?;
        let mut conn = self.writer(false)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        enabled(&tx, None)?;
        let present: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM traces WHERE trace_id=?)",
            [&input.trace_id],
            |r| r.get(0),
        )?;
        if present {
            enabled(&tx, Some(&input.trace_id))?;
        }
        let value = serde_json::to_value(&input)?;
        if let Some(result) = existing(&tx, "traces", "trace_id", &input.trace_id, &value)? {
            return Ok(result);
        }
        let binding = input.binding.as_ref();
        if let Some(binding) = binding {
            let taken: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM traces WHERE binding_kind=? AND binding_scope=? AND binding_key=? AND binding_run=?)",
                params![binding.kind,binding.scope,binding.key,binding.run],|r|r.get(0))?;
            if taken {
                return Err(Error::conflict(
                    "complete binding already belongs to another trace",
                ));
            }
        }
        tx.execute(
            "INSERT INTO traces VALUES(?,?,?,1,0,?,?,?,?,?)",
            params![
                input.trace_id,
                input.origin,
                now(),
                binding.map(|b| &b.kind),
                binding.map(|b| &b.scope),
                binding.map(|b| &b.key),
                binding.map(|b| &b.run),
                value.to_string()
            ],
        )?;
        tx.commit()?;
        Ok(receipt("stored", &input.trace_id))
    }

    pub fn create_dispatch(&self, input: DispatchInput) -> Result<Value> {
        input.validate()?;
        let mut conn = self.writer(false)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        enabled(&tx, Some(&input.trace_id))?;
        let value = serde_json::to_value(&input)?;
        if let Some(result) =
            existing(&tx, "dispatches", "dispatch_id", &input.dispatch_id, &value)?
        {
            return Ok(result);
        }
        if let Some(parent) = &input.parent_dispatch_id {
            let valid: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM dispatches WHERE dispatch_id=? AND trace_id=?)",
                params![parent, input.trace_id],
                |r| r.get(0),
            )?;
            if !valid {
                return Err(Error::invalid(
                    "parent must already exist in the same trace",
                ));
            }
        }
        tx.execute(
            "INSERT INTO dispatches VALUES(?,?,?,?,?,?)",
            params![
                input.dispatch_id,
                input.trace_id,
                input.parent_dispatch_id,
                input.kind,
                now(),
                value.to_string()
            ],
        )?;
        tx.commit()?;
        Ok(receipt("stored", &input.dispatch_id))
    }
}
