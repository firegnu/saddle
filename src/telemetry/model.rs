use super::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub kind: String,
    pub scope: String,
    pub key: String,
    pub run: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TraceInput {
    pub schema_version: u32,
    pub trace_id: String,
    pub origin: String,
    pub label: String,
    pub binding: Option<Binding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchInput {
    pub schema_version: u32,
    pub dispatch_id: String,
    pub trace_id: String,
    pub kind: String,
    pub parent_dispatch_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingInput {
    pub schema_version: u32,
    pub enabled: bool,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceSettingInput {
    pub schema_version: u32,
    pub trace_id: String,
    pub enabled: bool,
    pub actor: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BodyInput {
    pub role: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub relation: String,
    pub target_event_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventInput {
    pub schema_version: u32,
    pub event_id: String,
    pub trace_id: String,
    pub dispatch_id: Option<String>,
    pub operation_id: Option<String>,
    pub kind: String,
    pub observed_at: Value,
    pub producer: String,
    pub evidence_kind: String,
    pub payload: Value,
    pub links: Vec<Link>,
    pub bodies: Vec<BodyInput>,
}

pub(super) fn version(version: u32) -> Result<()> {
    if version != 1 {
        return Err(Error::invalid("unsupported schema_version"));
    }
    Ok(())
}
pub(super) fn nonempty(value: &str) -> Result<()> {
    if value.is_empty() || value.contains('\0') {
        return Err(Error::invalid("expected nonempty string without NUL"));
    }
    Ok(())
}

impl Binding {
    pub(super) fn validate(&self) -> Result<()> {
        for value in [&self.kind, &self.scope, &self.key, &self.run] {
            nonempty(value)?;
        }
        Ok(())
    }
}

impl TraceInput {
    pub(super) fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        nonempty(&self.trace_id)?;
        if !matches!(self.origin.as_str(), "task" | "ad_hoc") {
            return Err(Error::invalid("origin must be task or ad_hoc"));
        }
        if let Some(binding) = &self.binding {
            binding.validate()?;
        }
        if self.origin == "task" && self.binding.is_none() {
            return Err(Error::invalid("task requires a complete binding"));
        }
        Ok(())
    }
}

impl DispatchInput {
    pub(super) fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        nonempty(&self.dispatch_id)?;
        nonempty(&self.trace_id)?;
        if !matches!(
            self.kind.as_str(),
            "implementation" | "review" | "redispatch" | "controller_handoff"
        ) {
            return Err(Error::invalid("invalid dispatch kind"));
        }
        if let Some(parent) = &self.parent_dispatch_id {
            nonempty(parent)?;
        }
        Ok(())
    }
}
