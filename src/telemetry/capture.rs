use super::{blobs::StagedBody, events::insert_event, storage::*, *};
use rusqlite::{TransactionBehavior, params};
use std::path::PathBuf;

/// In-process execution context. Generation and operation/event IDs are never supplied by callers.
pub struct OperationInput {
    pub schema_version: u32,
    pub trace_id: String,
    pub dispatch_id: String,
    pub kind: String,
    pub producer: String,
    pub basis_event_ids: Vec<String>,
    pub decision_event_id: Option<String>,
    pub previous_brief_event_id: Option<String>,
}

pub struct Observation {
    pub observed_at: Value,
    pub payload: Value,
    pub bodies: Vec<BodyInput>,
}

struct Prepared {
    input: EventInput,
    bodies: Vec<StagedBody>,
}

impl Prepared {
    fn record(&self) -> Result<Value> {
        let mut record = serde_json::to_value(&self.input)?;
        record["bodies"] = json!(self.bodies.iter().map(|b| &b.reference).collect::<Vec<_>>());
        record["source_auth"] = json!("execution_observed");
        Ok(record)
    }
}

/// A non-serializable capture credential and its original staged bytes. Keep it until end.
/// It cannot be refreshed after either generation changes, even if begin was never saved.
pub struct Capture {
    root: PathBuf,
    operation_id: String,
    end_id: String,
    kind: String,
    generations: (i64, Option<i64>),
    begin: Prepared,
    snapshot: Option<Prepared>,
}

impl Capture {
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn begin_event_id(&self) -> &str {
        &self.begin.input.event_id
    }
    pub fn end_event_id(&self) -> &str {
        &self.end_id
    }
    pub fn gaps(&self) -> &[Value] {
        self.begin.input.payload["gaps"]
            .as_array()
            .expect("validated gaps")
    }
}

fn add_gap(input: &mut EventInput, role: &str, reason: &str) -> Result<()> {
    let gaps = input.payload["gaps"]
        .as_array_mut()
        .ok_or_else(|| Error::invalid("gaps must be an array"))?;
    if !gaps.iter().any(|gap| gap["role"] == role) {
        gaps.push(json!({"role":role,"reason":reason}));
    }
    Ok(())
}

fn body_roles(kind: &str, payload: &Value) -> &'static [&'static str] {
    match kind {
        "route.begin" => &["summary", "request"],
        "agent.send.begin" => &["message"],
        "route.end"
            if payload["outcome"]["kind"] == "exited" && payload["outcome"]["exit_code"] == 0 =>
        {
            &["response", "suggestion"]
        }
        "agent.reply.end"
            if payload["outcome"]["kind"] == "exited" && payload["outcome"]["exit_code"] == 0 =>
        {
            &["reply"]
        }
        _ => &[],
    }
}

impl Store {
    /// Check explicit associations even while recording is disabled, without reading bodies
    /// or initializing storage. Unavailable storage cannot establish an invalid association.
    pub fn check_operation(&self, input: &OperationInput) -> Result<()> {
        let conn = self.reader()?.ok_or_else(Error::disabled)?;
        let mut links: Vec<_> = input
            .basis_event_ids
            .iter()
            .map(|id| Link {
                relation: "based_on".into(),
                target_event_id: id.clone(),
            })
            .collect();
        if let Some(id) = &input.decision_event_id {
            links.push(Link {
                relation: "uses_decision".into(),
                target_event_id: id.clone(),
            });
        }
        let event = EventInput {
            schema_version: input.schema_version,
            event_id: String::new(),
            trace_id: input.trace_id.clone(),
            dispatch_id: Some(input.dispatch_id.clone()),
            operation_id: None,
            kind: format!("{}.begin", input.kind),
            observed_at: Value::Null,
            producer: input.producer.clone(),
            evidence_kind: "execution_observed".into(),
            payload: json!({}),
            links,
            bodies: vec![],
        };
        validate::identity(&conn, &event)?;
        validate::links(&conn, &event)?;
        if let Some(id) = &input.previous_brief_event_id {
            let target = validate::event(&conn, id)?
                .ok_or_else(|| Error::invalid("unknown previous brief"))?;
            if target["kind"] != "brief.snapshot" || target["trace_id"] != input.trace_id {
                return Err(Error::invalid(
                    "previous brief must be a snapshot in this trace",
                ));
            }
        }
        enabled(&conn, Some(&input.trace_id))?;
        Ok(())
    }

    pub fn prepare_operation(
        &self,
        input: OperationInput,
        begin: Observation,
        brief_file: Option<PathBuf>,
    ) -> Result<Capture> {
        model::version(input.schema_version)?;
        if !begin.payload.is_object() {
            return Err(Error::invalid("payload must be an object"));
        }
        for id in [&input.trace_id, &input.dispatch_id, &input.producer] {
            model::nonempty(id)?;
        }
        if !matches!(
            input.kind.as_str(),
            "route" | "agent.start" | "agent.send" | "agent.reply"
        ) {
            return Err(Error::invalid("invalid operation kind"));
        }
        if input.kind == "agent.reply"
            && (brief_file.is_some() || input.previous_brief_event_id.is_some())
        {
            return Err(Error::invalid("reply does not capture a brief"));
        }
        let operation_id = uuid::Uuid::new_v4().to_string();
        let mut links = input
            .basis_event_ids
            .iter()
            .map(|id| Link {
                relation: "based_on".into(),
                target_event_id: id.clone(),
            })
            .collect::<Vec<_>>();
        if let Some(id) = input.decision_event_id {
            links.push(Link {
                relation: "uses_decision".into(),
                target_event_id: id,
            });
        }
        let mut event = EventInput {
            schema_version: 1,
            event_id: uuid::Uuid::new_v4().to_string(),
            trace_id: input.trace_id,
            dispatch_id: Some(input.dispatch_id),
            operation_id: Some(operation_id.clone()),
            kind: format!("{}.begin", input.kind),
            observed_at: begin.observed_at,
            producer: input.producer,
            evidence_kind: "execution_observed".into(),
            payload: begin.payload,
            links,
            bodies: begin.bodies,
        };
        let mut conn = self.writer(false)?;
        let read = conn.transaction()?;
        let generations = enabled(&read, Some(&event.trace_id))?;
        validate::identity(&read, &event)?;
        validate::links(&read, &event)?;
        let mut snapshot_links = Vec::new();
        for link in &event.links {
            let target = validate::event(&read, &link.target_event_id)?.expect("validated target");
            if link.relation == "based_on"
                && (validate::evidence(target["kind"].as_str().expect("kind"))
                    || target["kind"] == "review.recorded")
            {
                snapshot_links.push(link.clone());
            }
        }
        if let Some(id) = input.previous_brief_event_id {
            let target = validate::event(&read, &id)?
                .ok_or_else(|| Error::invalid("unknown previous brief"))?;
            if target["kind"] != "brief.snapshot" || target["trace_id"] != event.trace_id {
                return Err(Error::invalid(
                    "previous brief must be a snapshot in this trace",
                ));
            }
            snapshot_links.push(Link {
                relation: "supersedes".into(),
                target_event_id: id,
            });
        }
        read.commit()?;
        let snapshot = if let Some(path) = brief_file {
            if !path.is_absolute() {
                return Err(Error::invalid("brief path must be absolute"));
            }
            match self.stage(&BodyInput {
                role: "brief".into(),
                path: path.clone(),
            }) {
                Ok(body) => {
                    let captured = now();
                    let snapshot = EventInput {
                        schema_version: 1,
                        event_id: uuid::Uuid::new_v4().to_string(),
                        trace_id: event.trace_id.clone(),
                        dispatch_id: event.dispatch_id.clone(),
                        operation_id: event.operation_id.clone(),
                        kind: "brief.snapshot".into(),
                        observed_at: json!(captured),
                        producer: event.producer.clone(),
                        evidence_kind: "execution_observed".into(),
                        payload: json!({"absolute_path":path,"captured_at":captured,"publication_phase":"begin","begin_missing":false}),
                        links: snapshot_links,
                        bodies: vec![],
                    };
                    event.links.push(Link {
                        relation: "uses_brief".into(),
                        target_event_id: snapshot.event_id.clone(),
                    });
                    Some(Prepared {
                        input: snapshot,
                        bodies: vec![body],
                    })
                }
                Err(error) if matches!(error.code, "unreadable" | "too_large") => {
                    add_gap(&mut event, "brief", error.code)?;
                    None
                }
                Err(error) => return Err(error),
            }
        } else {
            if input.kind != "agent.reply" {
                add_gap(&mut event, "brief", "not_supplied")?;
            }
            None
        };
        let begin = self.prepare_observation(event)?;
        Ok(Capture {
            root: self.root.clone(),
            operation_id,
            end_id: uuid::Uuid::new_v4().to_string(),
            kind: input.kind,
            generations,
            begin,
            snapshot,
        })
    }

    fn prepare_observation(&self, mut input: EventInput) -> Result<Prepared> {
        for role in body_roles(&input.kind, &input.payload) {
            if !input.bodies.iter().any(|b| b.role == *role) {
                add_gap(&mut input, role, "not_supplied")?;
            }
        }
        // Validate metadata and role conflicts before doing any body IO.
        let placeholders = input
            .bodies
            .iter()
            .map(|b| json!({"role":b.role}))
            .collect::<Vec<_>>();
        validate::observed(&input, &placeholders)?;
        let mut bodies = Vec::new();
        for body in input.bodies.clone() {
            match self.stage(&body) {
                Ok(staged) => bodies.push(staged),
                Err(error) if matches!(error.code, "unreadable" | "too_large") => {
                    add_gap(&mut input, &body.role, error.code)?
                }
                Err(error) => return Err(error),
            }
        }
        validate::observed(
            &input,
            &bodies
                .iter()
                .map(|b| b.reference.clone())
                .collect::<Vec<_>>(),
        )?;
        Ok(Prepared { input, bodies })
    }

    fn check_capture(&self, conn: &Connection, capture: &Capture) -> Result<()> {
        if self.root != capture.root {
            return Err(Error::invalid("capture belongs to another store"));
        }
        if enabled(conn, Some(&capture.begin.input.trace_id))? != capture.generations {
            return Err(Error::disabled());
        }
        Ok(())
    }

    fn insert_operation(&self, conn: &Connection, capture: &Capture) -> Result<()> {
        let input = &capture.begin.input;
        conn.execute(
            "INSERT INTO operations VALUES(?,?,?,?,?,?,?) ON CONFLICT(operation_id) DO NOTHING",
            params![
                capture.operation_id,
                input.trace_id,
                input.dispatch_id,
                capture.kind,
                input.producer,
                capture.generations.0,
                capture.generations.1
            ],
        )?;
        Ok(())
    }

    fn save_prepared(
        &self,
        conn: &Connection,
        prepared: &Prepared,
        missing_begin: bool,
    ) -> Result<()> {
        let mut input = prepared.input.clone();
        if input.kind == "brief.snapshot" && missing_begin {
            input.payload["begin_missing"] = json!(true);
            input.payload["publication_phase"] = json!("end");
        }
        validate::observed(
            &input,
            &prepared
                .bodies
                .iter()
                .map(|b| b.reference.clone())
                .collect::<Vec<_>>(),
        )?;
        validate::links(conn, &input)?;
        let mut record = prepared.record()?;
        record["payload"] = input.payload;
        for body in &prepared.bodies {
            self.publish(conn, body)?;
        }
        insert_event(conn, &record)
    }

    pub fn record_begin(&self, capture: &Capture) -> Result<Value> {
        let mut conn = self.writer(false)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        self.check_capture(&tx, capture)?;
        let record = capture.begin.record()?;
        if let Some(result) =
            existing(&tx, "events", "event_id", capture.begin_event_id(), &record)?
        {
            return Ok(result);
        }
        if validate::event(&tx, capture.end_event_id())?.is_some() {
            return Err(Error::invalid("cannot fill in begin after end"));
        }
        self.insert_operation(&tx, capture)?;
        if let Some(snapshot) = &capture.snapshot {
            self.save_prepared(&tx, snapshot, false)?;
        }
        self.save_prepared(&tx, &capture.begin, false)?;
        tx.commit()?;
        Ok(receipt("stored", capture.begin_event_id()))
    }

    pub fn record_end(&self, capture: &Capture, end: Observation) -> Result<Value> {
        self.record_end_before(capture, end, None)
    }

    pub(crate) fn record_end_before(
        &self,
        capture: &Capture,
        end: Observation,
        deadline: Option<std::time::Instant>,
    ) -> Result<Value> {
        if !end.payload.is_object() {
            return Err(Error::invalid("payload must be an object"));
        }
        let mut conn = self.writer(false)?;
        let read = conn.transaction()?;
        self.check_capture(&read, capture)?;
        read.commit()?;
        if end.payload.get("begin_missing").is_some() {
            return Err(Error::invalid("begin_missing is determined by storage"));
        }
        let begin = &capture.begin.input;
        let mut input = EventInput {
            schema_version: 1,
            event_id: capture.end_id.clone(),
            trace_id: begin.trace_id.clone(),
            dispatch_id: begin.dispatch_id.clone(),
            operation_id: begin.operation_id.clone(),
            kind: format!("{}.end", capture.kind),
            observed_at: end.observed_at,
            producer: begin.producer.clone(),
            evidence_kind: "execution_observed".into(),
            payload: end.payload,
            links: vec![],
            bodies: end.bodies,
        };
        input.payload["begin_missing"] = json!(false);
        let mut prepared = self.prepare_observation(input)?;
        if let Some(deadline) = deadline {
            let remaining = deadline
                .checked_duration_since(std::time::Instant::now())
                .ok_or_else(|| Error::unavailable("capture deadline elapsed"))?;
            conn.busy_timeout(remaining.min(std::time::Duration::from_millis(100)))?;
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        self.check_capture(&tx, capture)?;
        let missing = validate::event(&tx, capture.begin_event_id())?.is_none();
        let snapshot_available = if let Some(snapshot) = &capture.snapshot {
            snapshot.bodies[0].available()?
                || validate::event(&tx, &snapshot.input.event_id)?.is_some()
        } else {
            false
        };
        prepared.input.payload["begin_missing"] = json!(missing);
        if missing && capture.kind != "agent.reply" {
            prepared.input.links = begin.links.clone();
            if !snapshot_available {
                prepared
                    .input
                    .links
                    .retain(|link| link.relation != "uses_brief");
                let reason = begin.payload["gaps"]
                    .as_array()
                    .and_then(|gaps| gaps.iter().find(|g| g["role"] == "brief"))
                    .and_then(|gap| gap["reason"].as_str())
                    .unwrap_or("not_available");
                add_gap(&mut prepared.input, "brief", reason)?;
            }
        }
        let record = prepared.record()?;
        if let Some(result) = existing(&tx, "events", "event_id", capture.end_event_id(), &record)?
        {
            return Ok(result);
        }
        self.insert_operation(&tx, capture)?;
        if missing
            && snapshot_available
            && let Some(snapshot) = &capture.snapshot
        {
            self.save_prepared(&tx, snapshot, true)?;
        }
        self.save_prepared(&tx, &prepared, false)?;
        tx.commit()?;
        Ok(receipt("stored", capture.end_event_id()))
    }
}
