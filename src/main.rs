use anyhow::{Result, bail};
use saddle::config::{Config, default_path};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "telemetry") {
        std::process::exit(saddle::telemetry::run(&args[1..]));
    }
    if args.first().is_some_and(|s| s == "ctl") {
        std::process::exit(saddle::control::run(args[1..].to_vec()));
    }
    if let Err(error) = run() {
        eprintln!("saddle: {error:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = match args.next().as_deref() {
        None => default_path(),
        Some("--config") => args
            .next()
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("--config needs a path"))?,
        Some("--help" | "-h") => {
            println!("Telemetry: saddle telemetry --help (headless storage and queries).\n");
            println!(
                "Control: saddle ctl instances|inspect|open|request|close (JSON); saddle ctl --help lists all options.\n"
            );
            println!(
                "saddle [--config PATH]\n\nAgents: ↑↓/j/k select, Enter attach, a attention, r reply, PgUp/PgDn scroll, s sort, , (comma) settings, x then y stop, q quit.\nFocus: Ctrl-] returns to Agents; Tab / Shift-Tab opens Viewer. Mouse clicks switch panes.\nHistory (pane footer): wheel/↑↓/PgUp/PgDn scroll, / search then Enter, n older / N newer match, drag to select, Copy to the clipboard; Esc leaves the search, then returns to the live terminal.\nPlugins: use the Plugins palette to open enabled plugins; closing a view keeps its process running. Drover task UI and reminders are provided by the optional Drover plugin.\nLayout: saved on changes and exit; restored at startup from absolute $XDG_STATE_HOME/saddle/layout.json, otherwise ~/.local/state/saddle/layout.json. Exited agents and shells keep placeholders.\nDefault config: $XDG_CONFIG_HOME/saddle/config.toml (absolute XDG_CONFIG_HOME only); otherwise ~/.config/saddle/config.toml. --config PATH takes priority."
            );
            return Ok(());
        }
        Some(arg) => bail!("unknown argument {arg}; use --help"),
    };
    if args.next().is_some() {
        bail!("unexpected argument; use --help");
    }
    saddle::app::run(Config::load(&path)?, path)
}
