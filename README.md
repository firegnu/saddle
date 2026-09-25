# saddle

**A terminal workspace for your coding agents.**

English · [简体中文](README.zh-CN.md)

See your agents, manage the task queue, and work in an agent's live terminal—all in one window.

```text
┌─ Agents ─────────────┬─ Viewer ──────────────────────────┐
│ project/            │                                   │
│ ├─ main   working   │  The selected agent's terminal     │
│ └─ review idle      │                                   │
├─ Queue ─────────────┤  Type, paste, and interact here.    │
│ Current / Pending   │                                   │
│ History             │                                   │
└─────────────────────┴───────────────────────────────────┘
```

saddle is written in Rust with [Ratatui](https://ratatui.rs/). It brings together [corral](https://github.com/firegnu/corral) agent sessions and the [drover](https://github.com/firegnu/drover) task queue. It does not require tmux or Zellij.

## Features

- **Agents:** a repository tree with live status, agent type, activity, attachment count, working directory, and title. Color distinguishes working, idle, blocked, stalled, and error states.
- **Queue:** current, awaiting, pending, and historical tasks. Read details, add tasks, switch projects, release work, and control pause and loop settings through native controls.
- **Viewer:** the selected agent's live `corral attach` session, with terminal colors, Unicode, cursor rendering, mouse events, and paste support.
- **Mouse and keyboard:** compact clickable buttons, mouse-wheel and trackpad scrolling, and shortcuts. Scrolling lists keeps the selection and survives normal refreshes.
- **Responsive layout:** three panes in a wide terminal; Agents and Queue become tabs in a narrow window.
- **Terminal-native appearance:** transparent panel backgrounds, semantic state colors, and English interface labels. Task text and agent output keep their original language.

Agents and Queue are native Rust widgets. Only Viewer runs a child PTY; saddle does not embed external board interfaces.

## Getting started

### Requirements

- Rust stable **1.96 or later**.
- `corral` and `drover` available on `PATH`, or configured by path.
- A terminal with Unicode and mouse support; true color is recommended.

Development and interactive validation currently take place on macOS. Other platforms have not been verified.

### Build and run

```sh
git clone https://github.com/firegnu/saddle.git
cd saddle
cargo run --release --locked
```

Or install the executable from the checkout:

```sh
cargo install --path . --locked
saddle
```

Start your agents with corral and register your queue projects with drover separately. saddle displays existing sessions and projects; it does not create agents or initialize queue projects.

### First session

1. Select an agent on the left and press **Enter**, or click its row, to attach.
2. Type directly in Viewer to work with that agent.
3. Press **Ctrl-]** to return to Agents. Viewer stays connected.
4. Press **Tab** to focus Queue, or click the pane. Use **Project** to choose a registered project.
5. To exit from anywhere, press **Ctrl-]**, then **q**. Exiting disconnects saddle's viewer; the agents keep running.

If another terminal is attached to an agent, detach there before attaching through saddle. Stopping an agent is a separate, confirmed action.

## Configuration

saddle reads `~/.config/saddle/config.toml`. A missing file uses defaults. To select another file:

```sh
saddle --config /path/to/config.toml
```

```toml
corral = "corral"
left_width = 52
left_split = 0.5
refresh_ms = 1000

[queue]
drover = "drover"
# cwd = "~/projects/my-project"
```

| Setting | Meaning |
|---|---|
| `corral` | corral executable name or path |
| `left_width` | Preferred width of the left column, in terminal cells |
| `left_split` | Fraction of the left column height allocated to Agents; between 0 and 1 |
| `refresh_ms` | Background refresh interval in milliseconds |
| `queue.drover` | drover executable name or path |
| `queue.cwd` | Optional initial queue project directory |

Command paths and `queue.cwd` support `~/`. Queue reads the project registry at `~/.drover/projects`: it prefers `queue.cwd`, then the startup directory if registered, then the first registered project. With no registry entries, it tries the startup directory. The project picker also accepts a manual path; switching projects only affects the current session.

saddle gets task data through **`drover list --json`**. Full history requires a drover version that returns the complete history array; older versions return only the latest ten records. saddle cannot display records that the interface omits. Apart from the project registry, it does not read corral or drover's internal data files.

Font family and size belong to your terminal settings. Theme configuration is not yet available; panel backgrounds follow the terminal, with dedicated status and agent-type accents.

## Controls

| Context | Input | Action |
|---|---|---|
| Agents | ↑↓ / j k | Select an agent |
| Agents | Enter / click a row | Attach and focus Viewer |
| Agents | Mouse wheel / trackpad | Scroll the list without changing selection |
| Agents | Tab / Shift-Tab | Focus Queue / Viewer |
| Agents | PgUp / PgDn | Scroll the agent list |
| Agents | s | Toggle name / state sorting within repositories |
| Agents | x, then y | Stop the selected agent; other keys cancel |
| Agents | q | Quit saddle |
| Queue | ↑↓ / j k / click | Select a task |
| Queue list | Mouse wheel / trackpad | Scroll tasks and history without changing selection |
| Queue | c | Open the project picker |
| Project picker | Enter / click, e, r | Open project, enter a path, reload registry |
| Path form | Ctrl-U / Enter / Esc | Clear / apply / cancel |
| Queue | Enter / Esc | Open details / return to the list |
| Queue details / result | Mouse wheel / PgUp / PgDn | Scroll content |
| Queue | r / g / n | Refresh / check and release / send next task |
| Queue | p / l | Pause or resume / toggle loop |
| Queue | a / ? | Add a task / open help |
| Add form | Tab / Ctrl-S / Esc | Switch field / save / cancel |
| Queue | q | Return to Agents; in text fields, q is text |
| Queue / Viewer | Ctrl-] | Return to Agents |

Viewer forwards input to the agent, except **Ctrl-]**. The bottom bar identifies the current input target. Open dialogs capture their own input; background controls stay inactive. Unsubmitted Queue drafts survive a temporary return to Agents.

**Go, Next, Pause, and Loop apply to the selected project**, regardless of which history task is highlighted. A failed refresh disables actions on stale queue data. No active work is shown as `No active tasks`; history remains available, with a range indicator at the bottom.

## Development

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Default tests use fake public CLIs, temporary directories, and synthetic terminal streams. They do not start real agents.

Optional integration tests use an installed drover in isolated temporary projects:

```sh
SADDLE_DROVER_BIN="$(command -v drover)" \
  cargo test --test workflow installed_drover_ -- --ignored
```

The complete-history test requires the drover history-limit fix described above. These tests do not modify real queues or use real agents.

Generate synthetic UI previews or compare the terminal parsers:

```sh
cargo run --example ui_preview -- /tmp/saddle-ui-preview
cargo run --example compare_parsers
```

Previews are SVG and text exports of Ratatui buffers, not recordings of live agents. Synthetic checks do not establish compatibility with every agent terminal application.

See [the design](docs/DESIGN.md) and [handoff notes](HANDOFF.md) for architectural decisions and validation records. These documents are currently in Chinese.
