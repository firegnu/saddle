//! Headless core plugin commands; no running TUI or external-plugin process is required.
use super::{
    core::{self, Catalog, Receipts, State},
    registry::Manifest,
};
use saddle_core_plugin::{Call, Operation};
use serde_json::json;
use std::{ffi::OsString, io::Write, path::PathBuf};

struct Invocation {
    config: PathBuf,
    action: Action,
}
enum Action {
    Status(Option<String>),
    Run {
        id: String,
        command: String,
        context: Option<PathBuf>,
        brief: Option<PathBuf>,
    },
}
fn path(value: &str) -> Result<PathBuf, &'static str> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || value.contains('\0') {
        return Err("absolute_path_required");
    }
    Ok(path)
}
fn parse(args: &[OsString]) -> Result<Invocation, &'static str> {
    let args = args
        .iter()
        .map(|a| a.to_str().ok_or("invalid_utf8"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut args = args.as_slice();
    let mut config = None;
    while args.first() == Some(&"--config") {
        if config.is_some() {
            return Err("duplicate_option");
        }
        config = Some(path(args.get(1).ok_or("missing_option_value")?)?);
        args = &args[2..];
    }
    let config = config.unwrap_or_else(crate::config::default_path);
    let action = match args {
        ["status"] => Action::Status(None),
        ["status", id] if !id.starts_with('-') => Action::Status(Some((*id).into())),
        ["run", id, command, rest @ ..] if !id.starts_with('-') && !command.starts_with('-') => {
            let mut context = None;
            let mut brief = None;
            let mut rest = rest.iter();
            while let Some(option) = rest.next() {
                let target = match *option {
                    "--record-context" => &mut context,
                    "--brief-file" => &mut brief,
                    _ => return Err("invalid_option"),
                };
                if target.is_some() {
                    return Err("duplicate_option");
                }
                *target = Some(path(rest.next().ok_or("missing_option_value")?)?);
            }
            Action::Run {
                id: (*id).into(),
                command: (*command).into(),
                context,
                brief,
            }
        }
        _ => return Err("invalid_arguments"),
    };
    Ok(Invocation { config, action })
}

pub fn run(args: &[OsString], catalog: Catalog) -> i32 {
    if args == [OsString::from("--help")] || args == [OsString::from("-h")] {
        println!(
            "saddle plugin [--config /abs/config.toml] status [ID]\nsaddle plugin [--config /abs/config.toml] run ID COMMAND [--record-context /abs/context.json] [--brief-file /abs/brief.md]"
        );
        return 0;
    }
    // Select the transport before validating arguments. Even malformed run calls start at
    // stderr byte zero; status is a read-only JSON query with ordinary query exit codes.
    let mut head = args;
    while head.first().is_some_and(|a| a == "--config") && head.len() >= 2 {
        head = &head[2..];
    }
    let status = head.first().is_some_and(|a| a == "status");
    let receipts = (!status).then(Receipts::start);
    let mut receipt = json!({"plugin":null,"command":null,"executed":false,"outcome":null,"error":null,"operation_id":null,"begin":"not_requested","end":"not_requested","gaps":[]});
    let result = (|| {
        let invocation = parse(args).map_err(|e| (2, e))?;
        let registry = core::registry(
            invocation
                .config
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("plugins.toml"),
            catalog,
        );
        if let Action::Run { id, command, .. } = &invocation.action {
            receipt["plugin"] = json!(id);
            receipt["command"] = json!(command);
        }
        if registry.error.is_some() {
            return Err((1, "registry_unavailable"));
        }
        match invocation.action {
            Action::Status(id) => {
                if let Some(id) = &id
                    && !catalog.iter().any(|p| p.manifest().id == id)
                    && !registry.entries.iter().any(|e| &e.id == id)
                {
                    return Err((2, "unknown_plugin"));
                }
                // Read-only: classification uses lstat/O_NOFOLLOW reads; nothing is created.
                let resources = super::resources::Resources::from_environment();
                let core = catalog.iter().map(|p| p.manifest()).filter(|m| id.as_ref().is_none_or(|id| id == m.id)).map(|m| {
                    let status = resources.status(m).map_err(|_| (1, "resource_record_unavailable"))?;
                    Ok(json!({"id":m.id,"name":m.name,"version":m.version,
                        "enabled":registry.core.get(m.id).is_some_and(|e| e.enabled),
                        "state":core::state(&registry, m.id),
                        "commands":m.commands.iter().map(|c| json!({"name":c.name,"capture":match c.capture {Some(Operation::Route)=>Some("route"), _=>None}})).collect::<Vec<_>>(),
                        "resources":status.resources,
                        "setup_note":m.setup_note,
                        "setup_files":status.setup_files}))
                }).collect::<Result<Vec<_>, _>>()?;
                let plugins = registry.entries.iter().filter(|e| id.as_ref().is_none_or(|id| id == &e.id)).map(|e| json!({"id":e.id,"directory":e.directory,"enabled":e.enabled,"manifest_readable":Manifest::read(&e.directory).is_ok()})).collect::<Vec<_>>();
                let result = json!({"core":core,"plugins":plugins});
                writeln!(std::io::stdout().lock(), "{result}")
                    .map_err(|_| (1, "output_unavailable"))?;
                Ok(0)
            }
            Action::Run {
                id,
                command,
                context,
                brief,
            } => {
                let Some(plugin) = catalog.iter().find(|p| p.manifest().id == id) else {
                    return Err((
                        2,
                        if registry.entries.iter().any(|e| e.id == id) {
                            "external_plugin"
                        } else {
                            "unknown_plugin"
                        },
                    ));
                };
                match core::state(&registry, &id) {
                    State::Conflict => return Err((2, "plugin_conflict")),
                    State::Disabled => return Err((2, "plugin_disabled")),
                    State::Enabled => (),
                }
                let declaration = plugin
                    .manifest()
                    .commands
                    .iter()
                    .find(|c| c.name == command)
                    .ok_or((2, "unknown_command"))?;
                if context.is_some() && declaration.capture.is_none() {
                    return Err((2, "capture_not_supported"));
                }
                let mut recorder =
                    super::capture::HostRecorder::check(&id, context.as_deref(), brief)
                        .map_err(|e| (2, e))?;
                receipt["executed"] = json!(true);
                // This entry point owns one synchronous headless invocation. The default
                // hook can block on a full stderr before catch_unwind sees the panic, and
                // can disclose arbitrary plugin panic text. Only the host receipt reports it.
                let hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(|_| {}));
                let completion = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    plugin.run(Call {
                        command: &command,
                        stdin: &mut std::io::stdin().lock(),
                        recorder: &mut recorder,
                    })
                }));
                std::panic::set_hook(hook);
                match completion {
                    Ok(completion) if completion.exit_code < 125 => {
                        let mut stdout = std::io::stdout().lock();
                        if stdout
                            .write_all(&completion.stdout)
                            .and_then(|()| stdout.flush())
                            .is_err()
                        {
                            receipt["gaps"]
                                .as_array_mut()
                                .unwrap()
                                .push(json!({"role":"output","reason":"write_failed"}));
                        }
                        receipt["outcome"] =
                            json!({"kind":"exited","exit_code":completion.exit_code});
                        let report = recorder.finish(receipt["outcome"].clone(), completion.end);
                        merge_capture(&mut receipt, report);
                        Ok(i32::from(completion.exit_code))
                    }
                    _ => {
                        receipt["outcome"] = json!({"kind":"unknown"});
                        receipt["error"] = json!({"code":"plugin_internal_failure","message":"plugin result is unknown"});
                        let report = recorder.finish(receipt["outcome"].clone(), None);
                        merge_capture(&mut receipt, report);
                        Ok(126)
                    }
                }
            }
        }
    })();
    let code = match result {
        Ok(code) => code,
        Err((code, error)) => {
            let message = if error == "external_plugin" {
                "use saddle ctl plugin through a running Saddle instance"
            } else {
                error
            };
            if status {
                eprintln!("saddle-plugin: {message}");
            }
            receipt["error"] = json!({"code":error,"message":message});
            if status { code } else { 125 }
        }
    };
    if let Some(receipts) = receipts {
        receipts.finish(receipt);
    }
    code
}

fn merge_capture(receipt: &mut serde_json::Value, report: serde_json::Value) {
    for key in ["operation_id", "begin", "end"] {
        receipt[key] = report[key].clone();
    }
    receipt["gaps"]
        .as_array_mut()
        .unwrap()
        .extend(report["gaps"].as_array().unwrap().clone());
}
