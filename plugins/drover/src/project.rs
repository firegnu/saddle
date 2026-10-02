//! Project onboarding belongs to Drover. Reads never initialize or repair anything.
use super::*;

pub fn inspect(project: &Path) -> Result<Value> {
    let path = project
        .canonicalize()
        .unwrap_or_else(|_| project.to_owned());
    let registered =
        crate::drover::registered_projects(&crate::config::expand_home("~/.drover/projects"))?
            .contains(&path.display().to_string());
    let mut v = json!({"ok":true,"project":path,"registered":registered,"state":"unavailable","main_agent":""});
    let result = (|| -> Result<()> {
        ensure!(
            path.is_dir(),
            "Project directory is unavailable: {}",
            path.display()
        );
        ensure!(
            !path.to_string_lossy().chars().any(char::is_control),
            "Project path contains control characters"
        );
        let cp = path.join(".drover.conf");
        // A broken symlink is an invalid configuration, not an uninitialized project.
        let config = match fs::symlink_metadata(&cp) {
            Ok(_) => Some(read(&cp, false)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(config) = &config {
            let s = Snapshot::load(&path)?;
            v["main_agent"] = json!(s.conf.get("MAIN_AGENT").cloned().unwrap_or_default());
            v["data_dir"] = json!(s.directory);
            v["state"] = json!(if registered { "registered" } else { "existing" });
            v["token"] = json!(hash(format!("{}:{registered}:{config}", path.display())));
        } else {
            ensure!(
                !registered,
                "Registered project is missing .drover.conf; restore its existing configuration"
            );
            v["state"] = json!("new");
            v["token"] = json!(hash(format!("{}:new", path.display())));
        }
        Ok(())
    })();
    if let Err(e) = result {
        v["error"] = json!(format!("{e:#}"));
        v["token"] = Value::Null;
    }
    Ok(v)
}

fn ignore_config(project: &Path) -> Result<()> {
    let path = project.join(".gitignore");
    if let Ok(m) = fs::symlink_metadata(&path) {
        ensure!(
            !m.file_type().is_symlink(),
            ".gitignore is a symlink; add .drover.conf manually"
        );
    }
    let mut text = read(&path, true)?;
    if !text
        .lines()
        .any(|l| matches!(l.trim(), ".drover.conf" | "/.drover.conf"))
    {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(".drover.conf\n");
        atomic_write(&path, &text)?;
    }
    Ok(())
}

pub fn save(project: &Path, name: &str, agent: &str, token: &str) -> Result<Value> {
    ensure!(
        !agent.chars().any(char::is_control)
            && !agent.contains(['\'', '"'])
            && agent.trim() == agent,
        "Invalid main agent name"
    );
    let home = crate::config::expand_home("~/.drover");
    let _registry_lock = Lock::new(&home.join(".projects.lock"))?;
    let info = inspect(project)?;
    ensure!(
        info["state"] != "unavailable",
        "{}",
        info["error"].as_str().unwrap_or("Project unavailable")
    );
    require(
        info["token"].as_str() == Some(token),
        "target_changed",
        "Project configuration changed; reload before saving",
    )?;
    let project = project.canonicalize()?;
    let cp = project.join(".drover.conf");
    match info["state"].as_str() {
        Some("new") => {
            ensure!(
                !name.is_empty()
                    && name
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
                "Project name allows lowercase letters, digits and hyphens"
            );
            let directory = home.join(name);
            fs::create_dir(&directory).context(
                "Data directory already exists or cannot be created; choose another name",
            )?;
            let result = (|| -> Result<()> {
                let mut f = tempfile::NamedTempFile::new_in(&project)?;
                write!(
                    f,
                    "# Drover plugin data; lifecycle follows Saddle.\nHANDOFF_DIR={}\nMAIN_AGENT={agent}\nTASK_FILE_DIR=\n",
                    directory.display()
                )?;
                f.as_file().sync_all()?;
                f.persist_noclobber(&cp).map_err(|e| e.error)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = fs::remove_dir(&directory);
            }
            result?;
        }
        Some("registered") => {
            let first = Snapshot::load(&project)?;
            let _lock = Lock::new(&first.directory.join(".tasks.lock"))?;
            first.fresh()?;
            require(
                inspect(&project)?["token"].as_str() == Some(token),
                "target_changed",
                "Project configuration changed; reload before saving",
            )?;
            let line = format!("MAIN_AGENT={agent}\n");
            let mut found = false;
            let mut config: String = first
                .config
                .split_inclusive('\n')
                .map(|l| {
                    if l.trim().split_once('=').map(|(k, _)| k.trim()) == Some("MAIN_AGENT") {
                        found = true;
                        line.clone()
                    } else {
                        l.to_owned()
                    }
                })
                .collect();
            if !found {
                if !config.is_empty() && !config.ends_with('\n') {
                    config.push('\n');
                }
                config.push_str(&line);
            }
            first.fresh()?;
            atomic_write(&cp, &config)?;
        }
        Some("existing") => {} // Reuse exactly; binding changes are a separate Settings action.
        _ => anyhow::bail!("Project unavailable"),
    }
    let registry = home.join("projects");
    let raw = read(&registry, true)?;
    if !crate::drover::registered_projects(&registry)?.contains(&project.display().to_string()) {
        let sep = if raw.is_empty() || raw.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        atomic_write(&registry, &format!("{raw}{sep}{}\n", project.display())).context(
            "Configuration saved, but project registration failed; reload and retry to reuse it",
        )?;
    }
    let warning = if info["state"] != "registered" {
        ignore_config(&project)
            .err()
            .map(|e| format!("Project added, but .gitignore was not updated: {e:#}"))
    } else {
        None
    };
    Ok(
        json!({"ok":true,"project":project,"warning":warning,"message":"Project saved; no task dispatched"}),
    )
}
