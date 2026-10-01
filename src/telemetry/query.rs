use super::{validate, *};
use rusqlite::{OptionalExtension, params};

pub struct BindingFilter {
    pub kind: String,
    pub scope: String,
    pub key: String,
    pub run: Option<String>,
}
impl BindingFilter {
    /// The binding rule: kind, scope and key nonempty without NUL; run likewise when given.
    pub fn validate(&self) -> Result<()> {
        for value in [&self.kind, &self.scope, &self.key] {
            model::nonempty(value)?;
        }
        if let Some(run) = &self.run {
            model::nonempty(run)?;
        }
        Ok(())
    }
}

pub struct EventQuery {
    pub trace_id: Option<String>,
    pub dispatch_id: Option<String>,
    pub after_seq: i64,
    pub upper_seq: Option<i64>,
    pub limit: i64,
}

impl Default for EventQuery {
    fn default() -> Self {
        Self {
            trace_id: None,
            dispatch_id: None,
            after_seq: 0,
            upper_seq: None,
            limit: 100,
        }
    }
}

fn registration(conn: &Connection, trace: &str) -> Result<&'static str> {
    let declared: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM events WHERE trace_id=? AND kind='task.transition')",
        [trace],
        |r| r.get(0),
    )?;
    Ok(if declared {
        "登记声明未核验"
    } else {
        "未见运行登记声明"
    })
}

fn result_unknown(kind: &str, end: &Value) -> bool {
    let payload = &end["payload"];
    match payload["outcome"]["kind"].as_str() {
        Some("spawn_failed") => false,
        Some("exited") => {
            let has_body = |role: &str| {
                end["bodies"]
                    .as_array()
                    .is_some_and(|bodies| bodies.iter().any(|body| body["role"] == role))
            };
            match kind {
                // A saved exit code alone does not identify a started instance or
                // establish delivery. Pending is never a delivered result.
                "agent.start" => {
                    !payload["name"].is_string()
                        || !payload["instance"].is_string()
                        || payload["pending"] == true
                }
                "agent.send" => payload["confirmed"] != true || payload["pending"] == true,
                "agent.reply" if payload["outcome"]["exit_code"] == 0 => !has_body("reply"),
                "route" if payload["outcome"]["exit_code"] == 0 => {
                    !has_body("response") || !has_body("suggestion")
                }
                _ => false,
            }
        }
        // Includes timed_out, unknown and signaled: storing the end is not
        // evidence that the business outcome became known.
        _ => true,
    }
}

fn operations(conn: &Connection, trace: &str) -> Result<Vec<Value>> {
    let mut stmt=conn.prepare("SELECT operation_id,dispatch_id,kind,producer FROM operations WHERE trace_id=? ORDER BY operation_id")?;
    let mut result = Vec::new();
    for row in stmt.query_map([trace], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })? {
        let (id, dispatch, kind, producer) = row?;
        let mut phases = conn.prepare(
            "SELECT phase,event_id,record_json FROM events WHERE operation_id=? AND phase IS NOT NULL",
        )?;
        let mut begin = None;
        let mut end = None;
        let mut end_result = None;
        for row in phases.query_map([&id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (phase, event_id, record) = row?;
            if phase == "begin" {
                begin = Some(event_id);
            } else {
                end = Some(event_id);
                end_result = Some(serde_json::from_str::<Value>(&record)?);
            }
        }
        let result_unknown = end_result
            .as_ref()
            .is_none_or(|end| result_unknown(&kind, end));
        let unknown = if result_unknown {
            Some(if kind == "agent.reply" {
                "回复查询未完成/结果未知"
            } else {
                "执行或交付结果未知"
            })
        } else {
            None
        };
        result.push(json!({"operation_id":id,"trace_id":trace,"dispatch_id":dispatch,"kind":kind,"producer":producer,
            "begin_event_id":begin,"end_event_id":end,"begin_missing":begin.is_none(),"end_missing":end.is_none(),
            "unknown":unknown,"delivery_result_unknown":result_unknown && kind!="agent.reply"}));
    }
    Ok(result)
}

fn trace_details(conn: &Connection, record: &mut Value) -> Result<()> {
    let id = record["trace_id"].as_str().expect("trace ID").to_owned();
    let mut stmt=conn.prepare("SELECT event_id FROM events WHERE trace_id=? OR (trace_id IS NULL AND kind='recording.changed') ORDER BY seq")?;
    let mut history = Vec::new();
    let mut gaps = Vec::new();
    let mut evidence = Vec::new();
    for row in stmt.query_map([&id], |r| r.get::<_, String>(0))? {
        let event = validate::event(conn, &row?)?.expect("stored event");
        if event["kind"] == "recording.changed" {
            history.push(event.clone());
        }
        if let Some(known) = event["payload"]["gaps"].as_array() {
            for gap in known {
                gaps.push(json!({"event_id":event["event_id"],"gap":gap}));
            }
        }
        if event["kind"].as_str().is_some_and(validate::evidence) {
            evidence.push(decorate_event(conn, event)?);
        }
    }
    let created = validate::timestamp(&record["created_at"])?.expect("created time");
    let mut global = false;
    let mut local = true;
    let mut start = record["created_at"].clone();
    let mut intervals = Vec::new();
    for change in &history {
        let at = &change["payload"]["changed_at"];
        let time = validate::timestamp(at)?.expect("control time");
        if time > created {
            intervals.push(json!({"start":start,"end":at,"enabled":global && local}));
            start = at.clone();
        }
        if change["trace_id"].is_null() {
            global = change["payload"]["enabled"]
                .as_bool()
                .expect("control boolean");
        } else {
            local = change["payload"]["enabled"]
                .as_bool()
                .expect("control boolean");
        }
    }
    intervals.push(json!({"start":start,"end":null,"enabled":global && local}));
    record["recording_history"] = json!(history);
    record["recording_intervals"] = json!(intervals);
    record["known_gaps"] = json!(gaps);
    record["evidence"] = json!(evidence);
    record["operations"] = json!(operations(conn, &id)?);
    // Every dispatch of the trace, including ones whose events are not loaded yet.
    let mut stmt = conn.prepare("SELECT dispatch_id,kind,parent_dispatch_id,created_at FROM dispatches WHERE trace_id=? ORDER BY created_at,dispatch_id")?;
    let dispatches = stmt
        .query_map([&id], |r| {
            Ok(json!({"dispatch_id":r.get::<_, String>(0)?,"kind":r.get::<_, String>(1)?,
                "parent_dispatch_id":r.get::<_, Option<String>>(2)?,"created_at":r.get::<_, String>(3)?}))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    record["dispatches"] = json!(dispatches);
    record["registration"] = json!(registration(conn, &id)?);
    record["coverage_notice"] =
        json!("仅描述已记录材料；未记录操作不可见，缺口原因未知，不代表任务完整历史。");
    Ok(())
}

pub(super) fn decorate_event(conn: &Connection, mut event: Value) -> Result<Value> {
    if event["source_auth"] == "unverified" {
        event["source_description"] = json!("声明的说话者/来源（未经独立核验）");
        let declared = event["payload"]
            .get("declared_at")
            .filter(|v| !v.is_null())
            .unwrap_or(&event["observed_at"]);
        let at = validate::timestamp(declared)?;
        let recorded = validate::timestamp(&event["recorded_at"])?.expect("recorded time");
        let trace = event["trace_id"].as_str().expect("statement trace");
        let created: String = conn.query_row(
            "SELECT created_at FROM traces WHERE trace_id=?",
            [trace],
            |r| r.get(0),
        )?;
        let mut start = validate::timestamp(&json!(created))?.expect("created time");
        let mut stmt=conn.prepare("SELECT record_json FROM events WHERE kind='recording.changed' AND (trace_id IS NULL OR trace_id=?) ORDER BY seq")?;
        for row in stmt.query_map([trace], |r| r.get::<_, String>(0))? {
            let change: Value = serde_json::from_str(&row?)?;
            let time =
                validate::timestamp(&change["payload"]["changed_at"])?.expect("control time");
            if change["payload"]["enabled"] == true && time <= recorded {
                start = start.max(time);
            }
        }
        let late = at.is_some_and(|at| at < start);
        event["late_submission"] = json!(late);
        event["declared_time_outside_current_coverage"] = json!(late);
        event["submission_notice"] = if late {
            json!("事后提交/声明时间在未采集区间或早于当前记录时段")
        } else {
            Value::Null
        };
    }
    Ok(event)
}

impl Store {
    pub fn list(&self) -> Result<Value> {
        self.list_bound(None)
    }

    pub fn list_bound(&self, filter: Option<&BindingFilter>) -> Result<Value> {
        if let Some(filter) = filter {
            filter.validate()?;
        }
        let Some(mut conn) = self.reader()? else {
            return Ok(json!({"schema_version":1,"ok":true,"initialized":false,"traces":[]}));
        };
        let tx = conn.transaction()?;
        let traces = {
            let mut stmt=tx.prepare("SELECT record_json,created_at,capture_enabled,generation FROM traces WHERE (?1 IS NULL OR (binding_kind=?1 AND binding_scope=?2 AND binding_key=?3 AND (?4 IS NULL OR binding_run=?4))) ORDER BY created_at,trace_id")?;
            let mut traces = Vec::new();
            for row in stmt.query_map(
                params![
                    filter.map(|f| &f.kind),
                    filter.map(|f| &f.scope),
                    filter.map(|f| &f.key),
                    filter.and_then(|f| f.run.as_ref())
                ],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, bool>(2)?,
                        r.get::<_, i64>(3)?,
                    ))
                },
            )? {
                let (raw, at, enabled, generation) = row?;
                let mut record: Value = serde_json::from_str(&raw)?;
                record["coverage_start"] = json!(at);
                record["capture_enabled"] = json!(enabled);
                record["generation"] = json!(generation);
                record["registration"] = json!(registration(
                    &tx,
                    record["trace_id"].as_str().expect("trace ID")
                )?);
                traces.push(record);
            }
            traces
        };
        tx.commit()?;
        Ok(json!({"schema_version":1,"ok":true,"initialized":true,"traces":traces}))
    }

    pub fn events(&self, query: &EventQuery) -> Result<Value> {
        if query.after_seq < 0
            || query.upper_seq.is_some_and(|v| v < query.after_seq)
            || !(1..=1000).contains(&query.limit)
        {
            return Err(Error::invalid("invalid cursor or limit (1..1000)"));
        }
        let Some(mut conn) = self.reader()? else {
            return Ok(
                json!({"schema_version":1,"ok":true,"initialized":false,"events":[],"upper_seq":0,"next_after_seq":query.after_seq,"has_more":false}),
            );
        };
        let tx = conn.transaction()?;
        let upper = match query.upper_seq {
            Some(v) => v,
            None => tx.query_row("SELECT coalesce(max(seq),0) FROM events", [], |r| r.get(0))?,
        };
        if query.after_seq > upper {
            return Err(Error::invalid("cursor is beyond the event upper bound"));
        }
        let mut events = Vec::new();
        {
            let mut stmt=tx.prepare("SELECT event_id FROM events WHERE seq>?1 AND seq<=?2 AND (?3 IS NULL OR trace_id=?3) AND (?4 IS NULL OR dispatch_id=?4) ORDER BY seq LIMIT ?5")?;
            for row in stmt.query_map(
                params![
                    query.after_seq,
                    upper,
                    query.trace_id,
                    query.dispatch_id,
                    query.limit + 1
                ],
                |r| r.get::<_, String>(0),
            )? {
                events.push(decorate_event(
                    &tx,
                    validate::event(&tx, &row?)?.expect("stored event"),
                )?);
            }
        }
        let has_more = events.len() > query.limit as usize;
        if has_more {
            events.pop();
        }
        let next = events
            .last()
            .and_then(|e| e["seq"].as_i64())
            .unwrap_or(query.after_seq);
        tx.commit()?;
        Ok(
            json!({"schema_version":1,"ok":true,"initialized":true,"events":events,"upper_seq":upper,"next_after_seq":next,"has_more":has_more}),
        )
    }

    pub fn show(&self, id: &str) -> Result<Value> {
        let Some(mut conn) = self.reader()? else {
            return Ok(json!({"schema_version":1,"ok":true,"initialized":false,"record":null}));
        };
        let tx = conn.transaction()?;
        let mut records = Vec::new();
        for (table, column) in [("traces", "trace_id"), ("dispatches", "dispatch_id")] {
            if let Some(raw) = tx
                .query_row(
                    &format!("SELECT record_json FROM {table} WHERE {column}=?"),
                    [id],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                let mut record: Value = serde_json::from_str(&raw)?;
                let created: String = tx.query_row(
                    &format!("SELECT created_at FROM {table} WHERE {column}=?"),
                    [id],
                    |r| r.get(0),
                )?;
                record["created_at"] = json!(created);
                if table == "traces" {
                    let (enabled, generation) = storage::policy(&tx, Some(id))?;
                    record["capture_enabled"] = json!(enabled);
                    record["generation"] = json!(generation);
                    record["coverage_start"] = json!(created);
                    trace_details(&tx, &mut record)?;
                }
                records.push(record);
            }
        }
        if let Some(event) = validate::event(&tx, id)? {
            records.push(decorate_event(&tx, event)?);
        }
        if let Some(trace) = tx
            .query_row(
                "SELECT trace_id FROM operations WHERE operation_id=?",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            records.extend(
                operations(&tx, &trace)?
                    .into_iter()
                    .filter(|o| o["operation_id"] == id),
            );
        }
        if records.len() > 1 {
            return Err(Error::invalid("ID is ambiguous across object types"));
        }
        let record = records
            .pop()
            .ok_or_else(|| Error::new("invalid", "not_found", "unknown record ID"))?;
        tx.commit()?;
        Ok(json!({"schema_version":1,"ok":true,"initialized":true,"record":record}))
    }
}
