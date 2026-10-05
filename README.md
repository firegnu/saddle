# saddle

**A terminal workspace for your coding agents.**

English · [简体中文](README.zh-CN.md)

Saddle is a Rust/Ratatui frontend for Agents and terminals. It uses the `corral` command installed on PATH by [ranch](https://github.com/firegnu/ranch), with no tmux or Zellij dependency. Saddle is maintained as a fallback frontend: runtime compatibility fixes, without new features.

## Features

- **Agents:** repository groups, status, agent type, public role and effort labels, title, activity, working directory, instance and attach information. Waiting/error agents sort first; `s` switches to name order, and `z` folds the list. A local connection mark covers all tabs and panes.
- **Git summary:** local branch, ahead count, changed lines and untracked files for each agent's public working directory. It does not fetch or run external diff, textconv, fsmonitor or clean/smudge filters. Missing or unavailable information stays unknown.
- **Attention:** agents waiting for input, errors and new replies. Opening an item only selects and attaches to that agent.
- **Terminals:** ordinary shells and live agent sessions, tabs and four-way splits, terminal colors, Unicode, mouse input, paste, history, search and copy.
- **Settings:** configuration editing, colors, mascot, Diagnostics and Updates. Saved colors and sidebar width apply immediately; settings marked as requiring a restart apply on reopening.
- **Layout:** tab, split, active-pane and agent identity persistence. Only the original live agent instance reconnects; exited agents and shells restore as placeholders.
- **Local control:** `saddle ctl` can inspect the workspace, open terminals/agents and close displays through a bounded local Unix socket.

Telemetry, Drover and the entire plugin system were removed on 2026-10-05. There are no plugin panes, SDK/protocol, task queues, recording switch, or `saddle agent`, `saddle telemetry`, `saddle plugin` and `saddle ctl plugin` commands.

Dispatch routing belongs to ranch: `ranch dispatch route`. The `corral-dispatch` skill is installed by `ranch dispatch install-skills`; Saddle does not install it or record dispatch telemetry.

## Build and run

Requirements: Rust stable 1.96+, ranch's `corral` on PATH, and a Unicode terminal. Coding agent programs such as Codex and Claude Code are installed separately. Git 2.45+ provides the Agents Git summary. Development and interactive validation take place on macOS; other platforms have not been verified.

```sh
git clone https://github.com/firegnu/saddle.git
cd saddle
cargo build --bin saddle --release --locked
./target/release/saddle
```

To build an immutable product directory:

```sh
./scripts/package.sh /absolute/new/saddle-version
/absolute/new/saddle-version/bin/saddle
```

The package contains `BUILD.txt` and `bin/saddle` only. Packaging does not install or switch programs. Deployment must not switch `~/.local/bin/corral` or install Corral/dispatch skills. Keep old `~/.local/share/saddle/versions/` directories while sessions may still use their Corral executables. Reopen Saddle normally after deploying a new version; existing agents keep running.

## Using the workspace

Select an agent and press Enter to attach. An already displayed agent focuses its existing pane. `Ctrl-]` returns input to Agents; typing in Viewer goes to the terminal. Closing an agent display detaches it without stopping the agent. Stopping uses the separate `x`, then `y` confirmation.

**New / n** opens the agent form. Choose a project, Codex or Claude, and a role. Controller locks the name to `main`; Regular lets you edit it. The full name is `Prefix/Name`. Project choices come from public agent working directories and Saddle's startup directory; a path can also be entered explicitly. Advanced fields include the command, first message, location and exact Corral call preview. Creation passes public role labels without adding automatic numbering.

Use **Split ▾** to choose Left/Right/Above/Below, then select Terminal, New agent or an existing agent. **+ Tab** opens the same picker for a new tab. Moving an existing agent keeps its session and pending attachment. Cancel leaves the layout unchanged.

**Close pane / Close tab / Quit** confirms before closing running shells. **Zoom / Restore** temporarily enlarges one pane while retaining the split layout. Narrow windows may show only one side of a split; resizing restores the visible arrangement.

**History** in the pane footer keeps scroll/search/selection input inside Saddle. Use the wheel, arrows or PageUp/PageDown to scroll, `/` to search, `n/N` for matches and Copy for selected text. Esc leaves search, then returns to live input.

## Settings, Diagnostics and Updates

Open **Settings**, or press comma with Agents focused.

| Page | Key | Contents |
|---|---|---|
| General | F1 | Sidebar width, refresh interval, mascot and display |
| Colors | F2 | Theme, color overrides and preview |
| Advanced | F3 | Corral command |
| Diagnostics | F4 | Read-only runtime, config and layout checks |
| Updates | F5 | Saddle source/installed/running versions and Corral agent compatibility |

**Save / Ctrl-S** writes only edited config keys, preserving comments and unknown legacy settings. **Cancel / Esc** keeps the file unchanged; **Default / Ctrl-D** resets the selected draft value. Invalid values keep the draft. External file changes require choosing Keep my edits, Discard my edits or Back.

Updates resolves Corral independently from Saddle through PATH (or the explicit Corral configuration). **Upgrade all** uses the existing confirmation and public Corral upgrade command; it is an agent runtime operation, separate from installing Saddle.

## Local control

```sh
saddle ctl instances
saddle ctl inspect --instance INSTANCE
saddle ctl open --instance INSTANCE --relative-to active --place right --shell --cwd /absolute/path
saddle ctl open --instance INSTANCE --relative-to active --place tab --agent project/name --focus
saddle ctl request REQUEST --instance INSTANCE
saddle ctl close --instance INSTANCE --pane PANE
saddle ctl --help
```

Responses are JSON. `open` also supports creating a named agent with `--name`, `--role`, `--prompt` and `-- PROGRAM ARG...`. `--relative-to self` resolves the caller's public Corral identity or the environment of a Saddle-created shell. Reuse the same request ID and arguments after an uncertain response; query its result before retrying. Closing running shells requires the returned confirmation token and `--confirm-shells`.

## Configuration and stored data

The config is `$XDG_CONFIG_HOME/saddle/config.toml` when XDG_CONFIG_HOME is absolute; otherwise `~/.config/saddle/config.toml`. `--config PATH` overrides it. See the commented [config.toml](config.toml).

```toml
corral = "corral"
left_width = 52
refresh_ms = 1000
theme = "dune"
mascot_enabled = true
mascot = "clawd"
mascot_display = "auto"
```

Command paths support `~/`. A command name resolves on PATH; no adjacent/bundled Corral fallback exists. Themes: Dune, Tide, Lagoon, Terminal.

Layout lives in `$XDG_STATE_HOME/saddle/layout.json` for an absolute XDG_STATE_HOME, otherwise `~/.local/state/saddle/layout.json`. Old plugin slots restore empty while other panes and split positions remain. Corrupt or unsupported layouts are preserved and saving is disabled until the user resolves them. Shell history and ended conversations are not replayed.

Old `~/.local/state/saddle/telemetry/`, `~/.drover`, plugin registrations and `plugin-resources.json` remain on disk. The application does not read or delete them. The migration only releases Saddle's corral-dispatch ownership entries, retaining skill files for ranch to take over.

## Development

Current decisions are in [DESIGN](docs/DESIGN.md); older telemetry/plugin/task documents are historical. Choose checks by impact using [UI regression guidance](docs/UI回归.md). For cross-module changes and releases:

```sh
export CARGO_TARGET_DIR="$HOME/Developer/personal_projs/saddle-worktrees/.target"
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Tests use temporary data and fake Corral programs. Do not operate existing user agents for testing, weaken assertions or change timeouts merely to obtain a pass.
