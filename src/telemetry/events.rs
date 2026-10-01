use super::{storage::*, *};
use rusqlite::{TransactionBehavior, params};

impl Store {
    pub fn append(&self, input: EventInput) -> Result<Value> {
        validate::statement(&input)?;
        let mut conn = self.writer(false)?;
        // Take both versions from one read snapshot, then release it before reading bodies.
        let read = conn.transaction()?;
        let generations = enabled(&read, Some(&input.trace_id))?;
        validate::identity(&read, &input)?;
        validate::links(&read, &input)?;
        let mut record = serde_json::to_value(&input)?;
        record["source_auth"] = json!("unverified");
        if input.kind == "evidence.reused" {
            let target =
                validate::event(&read, &input.links[0].target_event_id)?.expect("validated target");
            for field in ["declared_speaker", "acquisition", "declared_at"] {
                if let Some(value) = target["payload"].get(field) {
                    record["payload"][field] = value.clone();
                }
            }
            record["bodies"] = target["bodies"].clone();
        }
        read.commit()?;
        let mut staged = Vec::new();
        for body in &input.bodies {
            staged.push(self.stage(body)?);
        }
        if input.kind != "evidence.reused" {
            record["bodies"] = json!(staged.iter().map(|b| &b.reference).collect::<Vec<_>>());
        } else {
            // Reuse verifies the existing immutable bytes; it never accepts replacements.
            for body in record["bodies"].as_array().expect("stored bodies") {
                self.body(body["sha256"].as_str().expect("stored hash"))?;
            }
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if enabled(&tx, Some(&input.trace_id))? != generations {
            return Err(Error::disabled());
        }
        if let Some(result) = existing(&tx, "events", "event_id", &input.event_id, &record)? {
            return Ok(result);
        }
        for body in &staged {
            self.publish(&tx, body)?;
        }
        insert_event(&tx, &record)?;
        tx.commit()?;
        Ok(receipt("stored", &input.event_id))
    }
}

pub(super) fn insert_event(conn: &Connection, record: &Value) -> Result<()> {
    let id = record["event_id"].as_str().expect("event ID");
    let kind = record["kind"].as_str().expect("kind");
    let phase = if record["operation_id"].is_string() {
        kind.rsplit('.')
            .next()
            .filter(|p| matches!(*p, "begin" | "end"))
    } else {
        None
    };
    conn.execute("INSERT INTO events(event_id,trace_id,dispatch_id,operation_id,kind,phase,recorded_at,observed_at,fingerprint,record_json) VALUES(?,?,?,?,?,?,?,?,?,?)",
        params![id,record["trace_id"].as_str(),record["dispatch_id"].as_str(),record["operation_id"].as_str(),kind,phase,now(),record["observed_at"].as_str(),fingerprint(record),record.to_string()])?;
    for body in record["bodies"].as_array().expect("body list") {
        conn.execute(
            "INSERT INTO event_blobs VALUES(?,?,?)",
            params![id, body["role"].as_str(), body["sha256"].as_str()],
        )?;
    }
    for link in record["links"].as_array().expect("link list") {
        conn.execute(
            "INSERT INTO event_links VALUES(?,?,?)",
            params![
                id,
                link["relation"].as_str(),
                link["target_event_id"].as_str()
            ],
        )?;
    }
    Ok(())
}
