use anyhow::{Context, Result, ensure};
pub fn check_agent(
    status: &serde_json::Value,
    request: &super::runtime::Navigation,
    locally_attached: bool,
) -> Result<()> {
    ensure!(
        status["instance"].as_str() == Some(&request.instance),
        "agent identity changed or unavailable"
    );
    ensure!(
        status["state"].as_str().is_some_and(|s| s != "exited")
            && status["starting"] != true
            && status["incompatible"] != true
            && status.get("error").is_none_or(serde_json::Value::is_null),
        "agent has exited or is unavailable"
    );
    let attached = status["attached"]
        .as_u64()
        .context("agent attachment state unavailable")?;
    ensure!(
        attached == u64::from(locally_attached),
        "agent is attached elsewhere or local attachment changed"
    );
    Ok(())
}
