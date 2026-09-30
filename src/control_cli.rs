use super::*;
use crate::terminals::Place;
use std::collections::BTreeMap;

const HELP: &str = "saddle ctl (JSON output)\n\
  instances\n\
  plugin --plugin ID --method METHOD --params JSON [--instance ID] [--request-id ID]\n\
  inspect [--instance ID]\n\
  open [--instance ID] [--relative-to self|active|PANE] --place tab|left|right|up|down\n\
       (--shell [--cwd PATH] | --agent NAME | --name NAME [--cwd PATH] [--role ROLE] [--prompt TEXT] -- PROGRAM ARG...)\n\
       [--focus] [--request-id ID]\n\
  request REQUEST --instance ID\n\
  close (--pane PANE|--tab TAB) --instance ID [--request-id ID]\n\
        [--confirmation TOKEN --confirm-shells]\n\n\
Self uses CORRAL_NAME + CORRAL_INSTANCE, or shell SADDLE_INSTANCE/PANE/REVISION.\n\
Poll request with the returned instance and request_id. Retry uncertain operations with the SAME request ID and arguments.\n\
Runtime: $XDG_RUNTIME_DIR/saddle, otherwise $XDG_CACHE_HOME/saddle/run or ~/.cache/saddle/run.\n\
SADDLE_RUNTIME_DIR overrides this with an absolute private directory (also for isolated tests).";

pub fn run(args: Vec<String>) -> i32 {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        println!("{}", json!({"ok":true,"help":HELP}));
        return 0;
    }
    let result = execute(args).unwrap_or_else(|e| error("invalid_request", format!("{e:#}")));
    println!("{result}");
    if result["ok"] == true { 0 } else { 1 }
}
fn execute(args: Vec<String>) -> Result<Value> {
    let verb = &args[0];
    let mut values = BTreeMap::new();
    let mut command = Vec::new();
    let mut request = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--" {
            command.extend(iter.cloned());
            break;
        }
        if verb == "request" && !arg.starts_with('-') && request.is_none() {
            request = Some(arg.clone());
            continue;
        }
        let allowed = match verb.as_str() {
            "inspect" => &["--instance"][..],
            "plugin" => &[
                "--instance",
                "--plugin",
                "--method",
                "--params",
                "--request-id",
            ][..],
            "open" => &[
                "--instance",
                "--relative-to",
                "--place",
                "--shell",
                "--cwd",
                "--agent",
                "--name",
                "--role",
                "--prompt",
                "--focus",
                "--request-id",
            ][..],
            "close" => &[
                "--instance",
                "--pane",
                "--tab",
                "--request-id",
                "--confirmation",
                "--confirm-shells",
            ][..],
            "request" => &["--instance"][..],
            "instances" => &[][..],
            _ => bail!("unknown ctl command; use saddle ctl --help"),
        };
        if !allowed.contains(&arg.as_str()) {
            bail!("unknown option {arg}");
        }
        let value = if matches!(arg.as_str(), "--shell" | "--focus" | "--confirm-shells") {
            "true".into()
        } else {
            iter.next().context("option needs a value")?.clone()
        };
        if values.insert(arg.as_str(), value).is_some() {
            bail!("duplicate option {arg}");
        }
    }
    let caller = Caller::environment();
    if verb == "instances" {
        if !command.is_empty() {
            bail!("unexpected command arguments");
        }
        return Ok(json!({"ok":true,"instances":instances(&caller)?}));
    }
    let mut instance = values.remove("--instance");
    let request_id = values.remove("--request-id");
    let operation = match verb.as_str() {
        "inspect" => Operation::Inspect,
        "plugin" => Operation::Plugin {
            plugin: values.remove("--plugin").context("--plugin required")?,
            method: values.remove("--method").context("--method required")?,
            params: serde_json::from_str(
                &values.remove("--params").unwrap_or_else(|| "{}".into()),
            )?,
        },
        "request" => {
            if instance.is_none() {
                bail!("request requires --instance");
            }
            Operation::Request {
                request: request.context("request needs an ID")?,
            }
        }
        "open" => {
            let place = match values.remove("--place").as_deref() {
                Some("tab") => Place::Tab,
                Some("left") => Place::Left,
                Some("right") => Place::Right,
                Some("up") => Place::Up,
                Some("down") => Place::Down,
                _ => bail!("--place must be tab, left, right, up or down"),
            };
            let relative_to = values
                .remove("--relative-to")
                .unwrap_or_else(|| "self".into());
            let focus = values.remove("--focus").is_some();
            let shell = values.remove("--shell").is_some();
            let agent = values.remove("--agent");
            let name = values.remove("--name");
            if usize::from(shell) + usize::from(agent.is_some()) + usize::from(name.is_some()) != 1
            {
                bail!("choose exactly one of --shell, --agent or --name");
            }
            let cwd = values
                .remove("--cwd")
                .map(|p| -> Result<String> {
                    let path = crate::config::expand_home(&p)
                        .canonicalize()
                        .context("invalid cwd")?;
                    if !path.is_dir() {
                        bail!("cwd must be a directory");
                    }
                    Ok(path.to_string_lossy().into_owned())
                })
                .transpose()?;
            let content = if shell {
                Content::Shell { cwd }
            } else if let Some(name) = agent {
                if cwd.is_some() {
                    bail!("--agent does not accept --cwd");
                }
                Content::Agent { name }
            } else {
                let argv = std::mem::take(&mut command);
                if argv.first().is_none_or(|a| a.is_empty()) {
                    bail!("new agent needs -- PROGRAM ARG...");
                }
                Content::NewAgent {
                    name: name.unwrap(),
                    cwd,
                    role: values.remove("--role").unwrap_or_else(|| "regular".into()),
                    prompt: values.remove("--prompt"),
                    argv,
                }
            };
            Operation::Open {
                relative_to,
                place,
                content,
                focus,
            }
        }
        "close" => {
            if instance.is_none() {
                bail!("close requires --instance");
            }
            let pane = values.remove("--pane");
            let tab = values.remove("--tab");
            let target = match (pane, tab) {
                (Some(id), None) => CloseTarget::Pane(id.parse().context("invalid pane ID")?),
                (None, Some(id)) => CloseTarget::Tab(id.parse().context("invalid tab ID")?),
                _ => bail!("close requires exactly one of --pane or --tab"),
            };
            Operation::Close {
                target,
                confirmation: values.remove("--confirmation"),
                confirm_shells: values.remove("--confirm-shells").is_some(),
            }
        }
        _ => bail!("unknown ctl command; use saddle ctl --help"),
    };
    if !values.is_empty() || !command.is_empty() {
        bail!("options do not apply to the chosen content/command");
    }
    if instance.is_none() {
        // Corral identity always takes precedence over an inherited shell hint.
        if caller.name.is_none()
            && caller.corral_instance.is_none()
            && caller.saddle_instance.is_some()
        {
            instance = caller.saddle_instance.clone();
        } else {
            let found = instances(&caller)?;
            let own: Vec<_> = found
                .iter()
                .filter(|v| v["caller"]["pane"].is_u64())
                .collect();
            let selected = if own.len() == 1 {
                own[0]
            } else if own.is_empty() && found.len() == 1 {
                &found[0]
            } else {
                return Ok(error(
                    "ambiguous_instance",
                    "select --instance from saddle ctl instances",
                ));
            };
            instance = selected["instance"].as_str().map(str::to_owned);
        }
    }
    let instance = instance.context("no saddle instance available")?;
    let mut message = Message {
        instance: instance.clone(),
        caller,
        operation,
        request_id,
    };
    if matches!(
        message.operation,
        Operation::Open { .. } | Operation::Close { .. } | Operation::Plugin { .. }
    ) && message.request_id.is_none()
    {
        message.request_id = Some(random_id()?);
    }
    match exchange(&message) {
        Ok(mut value) => {
            value["instance"] = json!(message.instance);
            if message.request_id.is_some() {
                value["request_id"] = json!(message.request_id);
            }
            Ok(value)
        }
        Err(e) => {
            let mut value = error("instance_unavailable", e);
            value["instance"] = json!(instance);
            value["request_id"] = json!(message.request_id);
            value["state"] = json!("uncertain");
            Ok(value)
        }
    }
}
