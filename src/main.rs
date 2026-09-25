use anyhow::{Result, bail};
use saddle::config::{Config, expand_home};
fn main() {
    if let Err(error) = run() {
        eprintln!("saddle: {error:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = match args.next().as_deref() {
        None => expand_home("~/.config/saddle/config.toml"),
        Some("--config") => args
            .next()
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("--config needs a path"))?,
        Some("--help" | "-h") => {
            println!(
                "saddle [--config PATH]\n\nAgents: ↑↓/j/k select, Enter attach, r reply, PgUp/PgDn scroll, s sort, x then y stop, q quit.\nFocus: Ctrl-] returns to Agents; Tab opens Queue, Shift-Tab opens Viewer. Mouse clicks switch panes.\nQueue (native, clickable buttons): c projects, Enter details, Esc list, ? help, a add (Ctrl-S save), r refresh, g release, n next, p pause/resume, l loop, q back.\nProjects: ~/.drover/projects (read-only).\nDefault config: ~/.config/saddle/config.toml"
            );
            return Ok(());
        }
        Some(arg) => bail!("unknown argument {arg}; use --help"),
    };
    if args.next().is_some() {
        bail!("unexpected argument; use --help");
    }
    saddle::app::run(Config::load(&path)?)
}
