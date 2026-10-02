use super::{model::*, *};
use rusqlite::{OptionalExtension, params};
use std::collections::HashSet;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

pub(super) fn timestamp(value: &Value) -> Result<Option<OffsetDateTime>> {
    if value.is_null() {
        return Ok(None);
    }
    let parsed = OffsetDateTime::parse(
        value
            .as_str()
            .ok_or_else(|| Error::invalid("timestamp must be UTC string or null"))?,
        &Rfc3339,
    )
    .map_err(|_| Error::invalid("invalid UTC timestamp"))?;
    if parsed.offset() != UtcOffset::UTC {
        return Err(Error::invalid("timestamp must use UTC"));
    }
    Ok(Some(parsed))
}

pub(super) fn fields(value: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| Error::invalid("payload must be an object"))?;
    if required.iter().any(|k| !object.contains_key(*k))
        || object
            .keys()
            .any(|k| !required.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err(Error::invalid("missing or unknown payload field"));
    }
    Ok(())
}
pub(super) fn string(value: &Value) -> Result<()> {
    nonempty(
        value
            .as_str()
            .ok_or_else(|| Error::invalid("expected string"))?,
    )
}
pub(super) fn choice(value: &Value, choices: &[&str]) -> Result<()> {
    if !value.as_str().is_some_and(|v| choices.contains(&v)) {
        return Err(Error::invalid("invalid enum value"));
    }
    Ok(())
}

pub(super) fn statement(input: &EventInput) -> Result<()> {
    version(input.schema_version)?;
    nonempty(&input.event_id)?;
    nonempty(&input.trace_id)?;
    nonempty(&input.producer)?;
    timestamp(&input.observed_at)?;
    if !matches!(
        input.evidence_kind.as_str(),
        "controller_statement" | "plugin_statement"
    ) {
        return Err(Error::invalid(
            "append accepts only unverified controller/plugin statements",
        ));
    }
    if input.operation_id.is_some() {
        return Err(Error::invalid("statements cannot belong to an operation"));
    }
    let p = &input.payload;
    let dispatch = match input.kind.as_str() {
        "requirement.recorded" => {
            fields(p, &["declared_speaker", "acquisition", "declared_at"], &[])?;
            timestamp(&p["declared_at"])?;
            false
        }
        "proposal.recorded" => {
            fields(p, &["declared_speaker", "acquisition"], &[])?;
            false
        }
        "authorization.recorded" => {
            fields(
                p,
                &["declared_speaker", "acquisition", "context_complete"],
                &[],
            )?;
            let complete = p["context_complete"]
                .as_bool()
                .ok_or_else(|| Error::invalid("context_complete must be boolean"))?;
            if complete && !input.links.iter().any(|l| l.relation == "responds_to") {
                return Err(Error::invalid(
                    "complete authorization requires responds_to",
                ));
            }
            false
        }
        "evidence.reused" => {
            fields(p, &["reused_by"], &[])?;
            if p["reused_by"] != input.producer {
                return Err(Error::invalid("reused_by must match the declared producer"));
            }
            if input.links.len() != 1
                || input.links[0].relation != "carried_from"
                || !input.bodies.is_empty()
            {
                return Err(Error::invalid(
                    "reuse requires exactly one carried_from and no replacement body",
                ));
            }
            false
        }
        "controller.summary" => {
            fields(p, &["purpose"], &[])?;
            choice(&p["purpose"], &["route", "dispatch", "review"])?;
            true
        }
        "controller.decision" => {
            fields(
                p,
                &["planned_model", "planned_effort", "validation_budget"],
                &[],
            )?;
            for key in ["planned_model", "planned_effort", "validation_budget"] {
                if !p[key].is_null() {
                    string(&p[key])?;
                }
            }
            true
        }
        "review.recorded" => {
            fields(p, &["reviewer"], &["verdict"])?;
            string(&p["reviewer"])?;
            if let Some(v) = p.get("verdict") {
                choice(v, &["passed", "changes_requested", "inconclusive"])?;
            }
            true
        }
        "controller.note" => {
            fields(p, &["note_kind"], &[])?;
            choice(
                &p["note_kind"],
                &["review", "rework", "decision", "closure", "other"],
            )?;
            input.dispatch_id.is_some()
        }
        "task.transition" => {
            fields(p, &["binding", "from", "to", "business_committed_at"], &[])?;
            serde_json::from_value::<Binding>(p["binding"].clone())?.validate()?;
            string(&p["from"])?;
            string(&p["to"])?;
            timestamp(&p["business_committed_at"])?;
            false
        }
        _ => return Err(Error::invalid("event kind is not a statement kind")),
    };
    if dispatch != input.dispatch_id.is_some() {
        return Err(Error::invalid("invalid event scope"));
    }
    if let Some(id) = &input.dispatch_id {
        nonempty(id)?;
    }
    if matches!(
        input.kind.as_str(),
        "requirement.recorded" | "proposal.recorded" | "authorization.recorded"
    ) {
        string(&p["declared_speaker"])?;
        choice(
            &p["acquisition"],
            &[
                "controller_transcription",
                "user_supplied_file",
                "plugin_provided",
            ],
        )?;
    }
    let role = match input.kind.as_str() {
        "task.transition" if !input.bodies.is_empty() => Some("reason"),
        "evidence.reused" | "task.transition" => None,
        "controller.decision" => Some("reason"),
        _ => Some("text"),
    };
    if let Some(role) = role {
        if input.bodies.len() != 1 || input.bodies[0].role != role {
            return Err(Error::invalid("missing or invalid body role"));
        }
    } else if !input.bodies.is_empty() {
        return Err(Error::invalid("event does not accept body input"));
    }
    Ok(())
}

pub(super) fn event(conn: &Connection, id: &str) -> Result<Option<Value>> {
    let row = conn
        .query_row(
            "SELECT seq,recorded_at,fingerprint,record_json FROM events WHERE event_id=?",
            [id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    row.map(|(seq, at, fingerprint, record)| {
        let mut value: Value = serde_json::from_str(&record)?;
        value["seq"] = json!(seq);
        value["recorded_at"] = json!(at);
        value["fingerprint"] = json!(fingerprint);
        Ok(value)
    })
    .transpose()
}
pub(super) fn identity(conn: &Connection, input: &EventInput) -> Result<()> {
    if let Some(dispatch) = &input.dispatch_id {
        let found: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM dispatches WHERE dispatch_id=? AND trace_id=?)",
            params![dispatch, input.trace_id],
            |r| r.get(0),
        )?;
        if !found {
            return Err(Error::invalid("dispatch must belong to this trace"));
        }
    }
    if input.kind == "task.transition" {
        let raw: String = conn.query_row(
            "SELECT record_json FROM traces WHERE trace_id=?",
            [&input.trace_id],
            |r| r.get(0),
        )?;
        if serde_json::from_str::<Value>(&raw)?["binding"] != input.payload["binding"] {
            return Err(Error::invalid("binding must match trace declaration"));
        }
    }
    Ok(())
}
pub(super) fn evidence(kind: &str) -> bool {
    matches!(
        kind,
        "requirement.recorded" | "proposal.recorded" | "authorization.recorded" | "evidence.reused"
    )
}

pub(super) fn links(conn: &Connection, input: &EventInput) -> Result<()> {
    let mut seen = HashSet::new();
    for link in &input.links {
        if !seen.insert((&link.relation, &link.target_event_id)) {
            return Err(Error::invalid("duplicate link"));
        }
        let target = event(conn, &link.target_event_id)?
            .ok_or_else(|| Error::invalid("link target must already exist"))?;
        let kind = target["kind"].as_str().expect("stored kind");
        let same = target["trace_id"] == input.trace_id;
        let allowed = match (input.kind.as_str(), link.relation.as_str()) {
            ("evidence.reused", "carried_from") => evidence(kind),
            (
                "requirement.recorded"
                | "proposal.recorded"
                | "authorization.recorded"
                | "controller.summary"
                | "controller.decision"
                | "review.recorded"
                | "controller.note"
                | "brief.snapshot",
                "supersedes",
            ) => kind == input.kind,
            ("proposal.recorded", "based_on") | ("authorization.recorded", "responds_to") => {
                evidence(kind)
            }
            ("controller.summary", "based_on") => evidence(kind) || kind == "brief.snapshot",
            ("controller.decision", "based_on") => {
                evidence(kind) || matches!(kind, "route.begin" | "route.end")
            }
            ("review.recorded", "based_on") => {
                evidence(kind)
                    || matches!(
                        kind,
                        "brief.snapshot" | "agent.reply.end" | "review.recorded"
                    )
            }
            ("controller.note", "based_on") => {
                evidence(kind)
                    || matches!(
                        kind,
                        "brief.snapshot"
                            | "route.begin"
                            | "route.end"
                            | "agent.reply.end"
                            | "review.recorded"
                            | "controller.decision"
                    )
            }
            ("brief.snapshot", "based_on") => evidence(kind) || kind == "review.recorded",
            ("route.begin", "based_on") => evidence(kind) || kind == "controller.summary",
            ("agent.start.begin", "based_on") => evidence(kind),
            ("agent.send.begin", "based_on") => evidence(kind) || kind == "review.recorded",
            ("agent.start.begin" | "agent.send.begin", "uses_decision") => {
                kind == "controller.decision"
            }
            ("route.begin" | "agent.start.begin" | "agent.send.begin", "uses_brief") => {
                kind == "brief.snapshot"
                    && target["operation_id"].as_str() == input.operation_id.as_deref()
            }
            ("route.end" | "agent.start.end" | "agent.send.end", _)
                if input.payload["begin_missing"] == true =>
            {
                let mut recovered = input.clone();
                recovered.kind = input.kind.replace(".end", ".begin");
                recovered.links = vec![link.clone()];
                links(conn, &recovered)?;
                true
            }
            _ => false,
        };
        if !allowed || !(same || input.kind == "evidence.reused" && link.relation == "carried_from")
        {
            return Err(Error::invalid(
                "link relation, target kind or trace is invalid",
            ));
        }
    }
    Ok(())
}

fn bool_field(value: &Value) -> Result<()> {
    if !value.is_boolean() {
        return Err(Error::invalid("expected boolean"));
    }
    Ok(())
}
fn gap_for(payload: &Value, role: &str) -> bool {
    payload["gaps"]
        .as_array()
        .is_some_and(|g| g.iter().any(|v| v["role"] == role))
}
fn nullable_string(payload: &Value, key: &str) -> Result<()> {
    if payload[key].is_null() {
        if !gap_for(payload, key) {
            return Err(Error::invalid("unknown metadata requires a gap"));
        }
    } else {
        string(&payload[key])?;
    }
    Ok(())
}
fn parameter_map(payload: &Value, key: &str, keys: &[&str]) -> Result<()> {
    if payload[key].is_null() {
        return nullable_string(payload, key);
    }
    fields(&payload[key], &[], keys)?;
    for value in payload[key].as_object().expect("validated object").values() {
        string(value)?;
    }
    Ok(())
}
fn outcome(value: &Value) -> Result<bool> {
    let kind = value["kind"]
        .as_str()
        .ok_or_else(|| Error::invalid("outcome.kind is required"))?;
    match kind {
        "exited" => {
            fields(value, &["kind", "exit_code"], &[])?;
            if !value["exit_code"]
                .as_i64()
                .is_some_and(|v| (0..=255).contains(&v))
            {
                return Err(Error::invalid("invalid exit_code"));
            }
            Ok(value["exit_code"] == 0)
        }
        "signaled" => {
            fields(value, &["kind", "signal"], &[])?;
            if !value["signal"].as_i64().is_some_and(|v| v > 0 && v < 128) {
                return Err(Error::invalid("invalid signal"));
            }
            Ok(false)
        }
        "spawn_failed" | "timed_out" | "unknown" => {
            fields(value, &["kind"], &[])?;
            Ok(false)
        }
        _ => Err(Error::invalid("invalid outcome kind")),
    }
}

/// Validate the closed execution event schema after staging has turned read failures into gaps.
pub(super) fn observed(input: &EventInput, bodies: &[Value]) -> Result<()> {
    version(input.schema_version)?;
    timestamp(&input.observed_at)?;
    if input.evidence_kind != "execution_observed"
        || input.operation_id.is_none()
        || input.dispatch_id.is_none()
    {
        return Err(Error::invalid("invalid execution scope or source"));
    }
    let p = &input.payload;
    let (allowed, required, metadata): (&[&str], &[&str], &[&str]) = match input.kind.as_str() {
        "brief.snapshot" => {
            fields(
                p,
                &[
                    "absolute_path",
                    "captured_at",
                    "publication_phase",
                    "begin_missing",
                ],
                &[],
            )?;
            if !p["absolute_path"]
                .as_str()
                .is_some_and(|v| std::path::Path::new(v).is_absolute())
            {
                return Err(Error::invalid("snapshot requires absolute_path"));
            }
            if timestamp(&p["captured_at"])?.is_none() {
                return Err(Error::invalid("snapshot capture time is required"));
            }
            choice(&p["publication_phase"], &["begin", "end"])?;
            bool_field(&p["begin_missing"])?;
            if (p["publication_phase"] == "end") != (p["begin_missing"] == true) {
                return Err(Error::invalid("snapshot phase mismatch"));
            }
            (&["brief"], &["brief"], &[])
        }
        "route.begin" => {
            fields(
                p,
                &["router_model", "router_version", "rules_version", "gaps"],
                &[],
            )?;
            string(&p["router_model"])?;
            nullable_string(p, "router_version")?;
            nullable_string(p, "rules_version")?;
            (
                &["summary", "request"],
                &["summary", "request"],
                &["router_version", "rules_version", "brief"],
            )
        }
        "agent.start.begin" => {
            fields(
                p,
                &[
                    "cwd",
                    "agent_program",
                    "explicit_parameters",
                    "labels",
                    "gaps",
                ],
                &[],
            )?;
            nullable_string(p, "cwd")?;
            nullable_string(p, "agent_program")?;
            parameter_map(p, "explicit_parameters", &["model", "effort"])?;
            parameter_map(p, "labels", &["role", "model", "effort"])?;
            (
                &["prompt"],
                &[],
                &[
                    "cwd",
                    "agent_program",
                    "explicit_parameters",
                    "labels",
                    "brief",
                ],
            )
        }
        "agent.send.begin" => {
            fields(p, &["target_name", "send_kind", "gaps"], &[])?;
            string(&p["target_name"])?;
            choice(&p["send_kind"], &["initial", "followup", "rework"])?;
            (&["message"], &["message"], &["brief"])
        }
        "agent.reply.begin" => {
            fields(p, &["target_name", "gaps"], &[])?;
            string(&p["target_name"])?;
            (&[], &[], &[])
        }
        "route.end" => {
            fields(p, &["outcome", "begin_missing", "gaps"], &[])?;
            bool_field(&p["begin_missing"])?;
            let success = outcome(&p["outcome"])?;
            (
                &["response", "suggestion"],
                if success {
                    &["response", "suggestion"]
                } else {
                    &[]
                },
                if p["begin_missing"] == true {
                    &["brief"]
                } else {
                    &[]
                },
            )
        }
        "agent.start.end" | "agent.send.end" => {
            fields(
                p,
                &[
                    "outcome",
                    "begin_missing",
                    "confirmed",
                    "pending",
                    "merged_with_draft",
                    "name",
                    "instance",
                    "at",
                    "gaps",
                ],
                &[],
            )?;
            bool_field(&p["begin_missing"])?;
            outcome(&p["outcome"])?;
            for key in ["confirmed", "pending", "merged_with_draft"] {
                if p[key].is_null() {
                    if !gap_for(p, key) {
                        return Err(Error::invalid("unknown result requires gap"));
                    }
                } else {
                    bool_field(&p[key])?;
                }
            }
            for key in ["name", "instance"] {
                nullable_string(p, key)?;
            }
            if p["at"].is_null() {
                nullable_string(p, "at")?;
            } else if !p["at"].is_number() {
                return Err(Error::invalid("at must be a Unix timestamp number or null"));
            }
            (
                &[],
                &[],
                if p["begin_missing"] == true {
                    &[
                        "confirmed",
                        "pending",
                        "merged_with_draft",
                        "name",
                        "instance",
                        "at",
                        "brief",
                    ]
                } else {
                    &[
                        "confirmed",
                        "pending",
                        "merged_with_draft",
                        "name",
                        "instance",
                        "at",
                    ]
                },
            )
        }
        "agent.reply.end" => {
            fields(
                p,
                &[
                    "outcome",
                    "begin_missing",
                    "name",
                    "instance",
                    "at",
                    "association",
                    "gaps",
                ],
                &[],
            )?;
            bool_field(&p["begin_missing"])?;
            choice(&p["association"], &["not_proven"])?;
            for key in ["name", "instance"] {
                nullable_string(p, key)?;
            }
            if p["at"].is_null() {
                nullable_string(p, "at")?;
            } else if !p["at"].is_number() {
                return Err(Error::invalid("at must be a Unix timestamp number or null"));
            }
            (
                &["reply"],
                if outcome(&p["outcome"])? {
                    &["reply"]
                } else {
                    &[]
                },
                &["name", "instance", "at"],
            )
        }
        _ => return Err(Error::invalid("unknown execution event kind")),
    };
    let mut roles = HashSet::new();
    for body in bodies {
        let role = body["role"]
            .as_str()
            .ok_or_else(|| Error::invalid("body role must be a string"))?;
        if !allowed.contains(&role) || !roles.insert(role) {
            return Err(Error::invalid("invalid or repeated body role"));
        }
    }
    if input.kind != "brief.snapshot" {
        let gaps = p["gaps"]
            .as_array()
            .ok_or_else(|| Error::invalid("gaps must be an array"))?;
        for gap in gaps {
            fields(gap, &["role", "reason"], &[])?;
            let role = gap["role"]
                .as_str()
                .ok_or_else(|| Error::invalid("gap role must be a string"))?;
            if (!allowed.contains(&role) && !metadata.contains(&role)) || !roles.insert(role) {
                return Err(Error::invalid("invalid, repeated or conflicting gap role"));
            }
            choice(
                &gap["reason"],
                &[
                    "not_supplied",
                    "unreadable",
                    "too_large",
                    "unrecognized",
                    "not_available",
                    "write_failed",
                ],
            )?;
        }
        if roles.contains("brief") && input.links.iter().any(|l| l.relation == "uses_brief") {
            return Err(Error::invalid("brief gap conflicts with snapshot"));
        }
    }
    if required.iter().any(|r| !roles.contains(r)) {
        return Err(Error::invalid("required execution body or gap is missing"));
    }
    Ok(())
}
