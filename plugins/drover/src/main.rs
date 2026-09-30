fn main() -> anyhow::Result<()> {
    let mut corral = "corral".to_owned();
    let mut dispatch_log = "dlog".to_owned();
    let mut cwd = None;
    let mut refresh = 2000;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing value for {flag}"))?;
        match flag.as_str() {
            "--corral" => {
                corral = saddle_drover_plugin::config::expand_home(&value)
                    .display()
                    .to_string()
            }
            "--dispatch-log" => {
                dispatch_log = saddle_drover_plugin::config::expand_home(&value)
                    .display()
                    .to_string()
            }
            "--cwd" => {
                cwd = Some(
                    saddle_drover_plugin::config::expand_home(&value)
                        .display()
                        .to_string(),
                )
            }
            "--refresh-ms" => {
                refresh = value.parse::<u64>()?;
                anyhow::ensure!(refresh > 0, "refresh interval must be positive");
            }
            _ => anyhow::bail!("unknown argument {flag}"),
        }
    }
    saddle_plugin_sdk::run(|| {
        let cwd = cwd.unwrap_or_else(|| {
            let current = std::env::current_dir()
                .unwrap_or_default()
                .display()
                .to_string();
            let projects = saddle_drover_plugin::drover::registered_projects(
                &saddle_drover_plugin::config::expand_home("~/.drover/projects"),
            )
            .unwrap_or_default();
            if projects.contains(&current) {
                current
            } else {
                projects.first().cloned().unwrap_or(current)
            }
        });
        Box::new(saddle_drover_plugin::plugin::Drover::with_refresh(
            corral,
            dispatch_log,
            cwd,
            std::time::Duration::from_millis(refresh),
        ))
    })
}
