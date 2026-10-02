use super::{BindingFilter, Error, EventQuery, Result, SettingInput, Store, TraceSettingInput};
use serde_json::Value;

pub fn run(args: &[String]) -> i32 {
    if args == ["--help"] || args == ["-h"] {
        println!(
            "saddle telemetry (headless; global recording defaults to off)\n\
settings get\n\
settings set --input FILE|-\n\
trace create|set-recording --input FILE|-\n\
trace close --input FILE|-\n\
dispatch create --input FILE|-\n\
append --input FILE|-\n\
list [--kind KIND --scope SCOPE --key KEY [--run RUN]]\n\
show --id ID\n\
events [--trace-id ID] [--dispatch-id ID] [--after-seq N] [--upper-seq N] [--limit 1..1000]\n\
body --sha256 HASH (raw bytes; errors go to stderr)\n\n\
JSON schema and examples: docs/遥测使用.md\n\
Write exit codes: 0 stored/duplicate, 1 unavailable, 2 invalid, 3 conflict, 4 disabled."
        );
        return 0;
    }
    if args.first().is_some_and(|s| s == "body") {
        use std::io::Write;
        let result = match args {
            [_, flag, hash] if flag == "--sha256" => {
                Store::from_environment().and_then(|store| store.body(hash))
            }
            _ => Err(Error::invalid("body requires --sha256 HASH")),
        }
        .and_then(|bytes| {
            std::io::stdout()
                .lock()
                .write_all(&bytes)
                .map_err(Error::from)
        });
        return match result {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("{}", error.receipt());
                error.exit_code()
            }
        };
    }
    match execute(args) {
        Ok(value) => {
            println!("{value}");
            0
        }
        Err(error) => {
            println!("{}", error.receipt());
            error.exit_code()
        }
    }
}

fn execute(args: &[String]) -> Result<Value> {
    let store = Store::from_environment()?;
    if args.first().is_some_and(|s| s == "events") {
        let mut query = EventQuery::default();
        for (flag, value) in options(&args[1..])? {
            match flag {
                "--trace-id" => query.trace_id = Some(value.to_owned()),
                "--dispatch-id" => query.dispatch_id = Some(value.to_owned()),
                "--after-seq" => query.after_seq = number(value)?,
                "--upper-seq" => query.upper_seq = Some(number(value)?),
                "--limit" => query.limit = number(value)?,
                _ => return Err(Error::invalid("unknown events option")),
            }
        }
        return store.events(&query);
    }
    if args.first().is_some_and(|s| s == "list") && args.len() > 1 {
        let mut filter = BindingFilter {
            kind: String::new(),
            scope: String::new(),
            key: String::new(),
            run: None,
        };
        for (flag, value) in options(&args[1..])? {
            match flag {
                "--kind" => filter.kind = value.into(),
                "--scope" => filter.scope = value.into(),
                "--key" => filter.key = value.into(),
                "--run" => filter.run = Some(value.into()),
                _ => return Err(Error::invalid("unknown list option")),
            }
        }
        for value in [&filter.kind, &filter.scope, &filter.key] {
            super::model::nonempty(value)?;
        }
        return store.list_bound(Some(&filter));
    }
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["settings", "get"] => store.settings(),
        ["list"] => store.list(),
        ["settings", "set", "--input", path] => store.set_recording(None, read_input(path)?),
        ["trace", "set-recording", "--input", path] => {
            let input: TraceSettingInput = read_input(path)?;
            store.set_recording(
                Some(&input.trace_id),
                SettingInput {
                    schema_version: input.schema_version,
                    enabled: input.enabled,
                    actor: input.actor,
                },
            )
        }
        ["trace", "create", "--input", path] => store.create_trace(read_input(path)?),
        ["trace", "close", "--input", path] => store.close_trace(read_input(path)?),
        ["dispatch", "create", "--input", path] => store.create_dispatch(read_input(path)?),
        ["append", "--input", path] => store.append(read_input(path)?),
        ["show", "--id", id] => store.show(id),
        _ => Err(Error::invalid("use saddle telemetry --help")),
    }
}

fn options(args: &[String]) -> Result<Vec<(&str, &str)>> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut chunks = args.chunks_exact(2);
    for chunk in &mut chunks {
        if !seen.insert(&chunk[0]) {
            return Err(Error::invalid("duplicate option"));
        }
        result.push((chunk[0].as_str(), chunk[1].as_str()));
    }
    if !chunks.remainder().is_empty() {
        return Err(Error::invalid("option needs a value"));
    }
    Ok(result)
}
fn number(value: &str) -> Result<i64> {
    value
        .parse()
        .map_err(|_| Error::invalid("expected integer"))
}

fn read_input<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    use std::io::Read;
    let mut bytes = Vec::new();
    if path == "-" {
        std::io::stdin().read_to_end(&mut bytes)?;
    } else {
        bytes = std::fs::read(path)?;
    }
    Ok(serde_json::from_slice(&bytes)?)
}
