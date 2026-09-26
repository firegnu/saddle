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

- **Agents:** a repository tree with live status, agent type, activity, attachment count, working directory, and title. Color distinguishes working, idle, blocked, stalled, and error states. When an agent was started with a public corral `effort` label, a small dotted signal icon shows it: `⣄⡀`, `⣴⡀`, or `⣴⡇` for medium, high, or xhigh (three bars packed into two character cells, using the theme’s idle/working/starting colors—soft green/blue/purple by default; unlit bars keep only baseline dots, and selection does not change the icon). Agents without the label, or with any other value, show no icon. It reflects the delegation label only, not the runtime's actual effort.
- **Git summary per agent:** below each agent's directory, a line such as `dev-t12 · C2(main) · +18 -4 · ?1` describes the worktree at the agent's public corral `cwd`: the current branch; commits ahead of the local `main` (on `main` itself, ahead of its configured upstream, i.e. not yet pushed); uncommitted added/deleted lines against HEAD, staged and unstaged together, after Git's built-in text/eol attributes (so a committed CRLF file whose timestamp changed is not counted), per path with no rename detection (a pure rename counts as all lines deleted and added); and untracked files. The numbers belong to the directory, not the agent: agents sharing a worktree show the same line, and they do not say which agent or task made a commit. Values that cannot be determined show `—` (no local `main`, no upstream, detached HEAD, no commits yet); binary files have no line counts and are listed as `N binary`; a directory that is not a Git worktree, is gone, or times out shows `git unavailable`. The line refreshes about every 5 seconds from local data only; a slow repository delays the round for every directory. It never fetches or lazily fetches missing objects, and it runs no external diff, textconv, fsmonitor hook or clean/smudge/process filter from any attributes source. When a changed file would need such a filter, the line counts show `+— -—` instead. A parent repository's line does not look inside submodule worktrees: uncommitted changes inside a submodule are not counted, while a submodule whose commit moved counts as a changed gitlink (`+1 -1`). It also skips optional index writes and ignores inherited `GIT_*` variables such as `GIT_DIR`. It does not follow an agent that later `cd`s elsewhere.
- **Queue:** current, awaiting, pending, and historical tasks. Click a task for its status and progress details, add tasks, edit, reorder, and delete pending tasks, view pending tasks across all registered projects, switch projects, release work, and control pause and loop settings through native controls.
- **New agents:** choose a project and Codex or Claude, then create an agent with an editable suggested name. Advanced settings hold the full command, first message, opening location, and exact call preview.
- **Viewer tabs and splits:** each tab holds a group of terminals, with left/right/up/down splits. Each pane runs a live `corral attach` with terminal colors, Unicode, cursor rendering, mouse events, and paste support.
- **Mouse and keyboard:** compact clickable buttons, mouse-wheel and trackpad scrolling, and shortcuts. Scrolling lists keeps the selection and survives normal refreshes.
- **Responsive layout:** three panes in a wide terminal; Agents and Queue become tabs in a narrow window.
- **Terminal-native appearance:** transparent panel backgrounds, semantic state colors, and English interface labels. Task text and agent output keep their original language.

Agents and Queue are native Rust widgets. Only Viewer panes run child PTYs; saddle does not embed external board interfaces.

## Getting started

### Requirements

- Rust stable **1.96 or later**.
- `corral` and `drover` available on `PATH`, or configured by path.
- Git 2.45 or later on `PATH` for the Agents Git summary (it needs `--no-lazy-fetch`); older or missing Git shows `git unavailable`.
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

Use **New** in Agents to start an agent, or attach an existing corral session. Register queue projects with drover separately; saddle does not initialize queue projects.

### First session

1. Select an agent on the left and press **Enter**, or click its row, to attach.
2. Type directly in Viewer to work with that agent.
3. Press **Ctrl-]** to return to Agents. Viewer stays connected.
4. Press **Tab** to focus Queue, or click the pane. Use **Project** to choose a registered project.
5. To exit from anywhere, press **Ctrl-]**, then **q**. Exiting disconnects all of saddle's viewers; the agents keep running.

**New / n** opens a form ready to create an agent: choose a **Project**, choose **Codex** (default) or **Claude**, then click **Create agent** or press **Ctrl-S**. The project defaults to the current Tasks directory; selecting a different project only affects this new agent. Click the project selector (or Ctrl-P) to choose a registered directory with the mouse or Up/Down and Enter; **Edit path / Ctrl-E** allows any directory. The suggested `<project>/<agent>` name follows these choices until you edit it. Suggested names use `corral start --unique`; edited names are passed exactly, including normal CLI duplicate-name errors. The actual created name always comes from corral's reply.

Text inputs have labeled borders, placeholders, a highlighted focus, and a visible insertion cursor. Click inside an input to position the cursor; Tab/Shift-Tab changes focus. Left/Right, Home/End, Backspace/Delete, Ctrl-U (clear), and paste edit at the cursor, including Chinese wide characters. Long lines scroll horizontally; multiline messages also scroll vertically and support Up/Down and Enter. In short windows, focus navigation or the wheel brings fields into view while Create and Cancel stay at the bottom.

**Advanced / F4** reveals the full command, optional multiline first message, **Open in** (new tab by default, or current pane / four split directions), and the exact call preview. Clicking Codex or Claude explicitly resets the command to `codex` or `claude`; custom commands are marked **Custom command** and survive focus changes and collapsing Advanced. Add desired model or permission flags yourself; saddle adds none. Quoted arguments are parsed and passed directly to `corral start`, without shell expansion, pipelines, or redirections. PgUp/PgDn (or the wheel with Preview focused) scrolls the full preview. Failed starts keep the draft. **Cancel / Esc** or **Ctrl-]** returns to Agents; **n** reopens the draft, including an in-flight start.

Clicking an agent row (or Enter) shows it in the active pane, or jumps to its existing pane if it is already open anywhere. Layout starts on the right, place first and agent second: **Split ▾** on the active pane's bottom border opens a small menu with **Left ←**, **Right →**, **Above ↑**, and **Below ↓** (arrow keys work too), relative to that pane; **+ Tab** in the tab strip asks for a new tab. A list titled with that place (for example *Open agent on the right*) then offers the agents; the pane's own agent is not offered for its split. Click an agent (or ↑↓ and Enter; the wheel scrolls long lists) and only then is the pane or tab created. An agent already open elsewhere is marked **Move here**: choosing it moves that pane — the same session, output, and any attach still in progress — without attaching again or stopping anything; a split it leaves collapses, and a tab it empties disappears. **Cancel Esc** or Esc at either step returns to the Viewer and Ctrl-] to Agents, leaving the layout exactly as it was; with no agents to offer the list says so and keeps Cancel. The arrow controls reach tabs beyond the visible strip. Click a terminal's content or title to focus it. Closing a pane collapses its split; closing the final tab leaves an empty tab. Tab changes keep background attaches running. A start or attach belongs to the pane reserved at submission: switching tabs does not redirect it, and closing or replacing that target discards its attachment result without stopping a newly created agent. A failed start can leave its reserved pane empty. Tiny windows temporarily show only one branch where a split cannot fit, retaining the full layout for expansion; an empty terminal content area receives no input. Layouts last only for the current saddle run.

If another terminal is attached to an agent, detach there before attaching through saddle. Stopping an agent is a separate, confirmed action.

## Configuration

At startup, saddle reads `$XDG_CONFIG_HOME/saddle/config.toml` when `XDG_CONFIG_HOME` is an absolute path; if unset, empty, or relative, it reads `~/.config/saddle/config.toml`. Missing files and omitted settings use defaults. `--config` takes priority:

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
| `colors` | Optional flat table for interface and agent-type colors |

Command paths and `queue.cwd` support `~/`. Queue reads the project registry at `~/.drover/projects`: it prefers `queue.cwd`, then the startup directory if registered, then the first registered project. With no registry entries, it tries the startup directory. The project picker also accepts a manual path; switching projects only affects the current session.

saddle gets task data through **`drover list --json`**. Full history requires a drover version that returns the complete history array; older versions return only the latest ten records. saddle cannot display records that the interface omits. Task details come from the read-only **`drover show Tn --json --with-agent-status`** (schema version 1); a drover without it shows the error on the details page. Apart from the project registry, it does not read corral or drover's internal data files.

[config.toml](config.toml) is the complete, commented default configuration, ready to copy to the path above. Its defaults preserve the current appearance. For a small override, add:

```toml
[colors]
focus = "light_cyan"
bg = "default"
agent_selected = "#302a23"
```

Colors accept `default` (or `reset`), `#RRGGBB`, or lowercase ANSI names: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`. `gray` is normal ANSI white; `dark_gray` is bright black; `white` is bright white. ANSI colors follow the terminal palette.

The table covers backgrounds, selection, borders, focus, text levels, connection/unread indicators, action feedback, agent types, and reply formatting. `agent_*` status accents are shared by Agents and Queue, including Queue action feedback. Invalid colors or unknown settings report a configuration error. Changes apply on the next launch; there is no hot reload. Viewer terminal output keeps its own colors. Font family and size belong to your terminal settings.

## Controls

| Context | Input | Action |
|---|---|---|
| Agents | ↑↓ / j k | Select an agent |
| Agents | Enter / click a row | Attach in the active pane, or jump to the agent's existing pane |
| Agents | n / New | Open the new-agent form |
| New-agent form | Tab / Shift-Tab, Ctrl-U | Switch field, clear field |
| New-agent form | Ctrl-P / Project, Ctrl-E / Edit path | Choose a registered project or edit its path |
| New-agent form | Left/Right in Open in | Choose current pane, new tab (default), or a split direction |
| New-agent form | F4 / Advanced, Ctrl-S / Create agent, Esc | Toggle advanced settings, create, or return while keeping the draft |
| Viewer chrome | Split ▾, then a side / + Tab | Choose a split side or a new tab, then the agent to open or move there |
| Viewer chrome | tab / pane title / × | Select a tab/pane, close a tab |
| Viewer chrome | Close pane / Close tab | Disconnect the pane or every pane in the tab |
| Agents | Mouse wheel / trackpad | Scroll the list without changing selection |
| Agents | Tab / Shift-Tab | Focus Queue / Viewer |
| Agents | PgUp / PgDn | Scroll the agent list |
| Agents | s | Toggle name / state sorting within repositories |
| Agents | x, then y | Stop the selected agent; other keys cancel |
| Agents | q | Quit saddle |
| Queue | ↑↓ / j k | Select a task |
| Queue | Click a task / Enter | Show its details in the Tasks area |
| Queue list | Mouse wheel / trackpad | Scroll tasks and history without changing selection |
| Queue | c | Open the project picker |
| Project picker | Enter / click, e, r | Open project, enter a path, reload registry |
| Path form | Ctrl-U / Enter / Esc | Clear / apply / cancel |
| Task details | Esc / Back | Return to the list |
| Task details | t / Task | Open the full task text in a dialog |
| Task details / task text | e / Edit | Edit the task while it is pending |
| Task details / result | ↑↓ / j k / Mouse wheel / PgUp / PgDn | Scroll content |
| Queue | r / g / n | Refresh / check and release / send next task |
| Queue | p / l | Pause or resume / toggle loop |
| Queue | a / ? | Add a task / open help |
| Queue pending task | e / u / d / x | Edit / move up / move down / delete (confirm with y) |
| Queue | A | Show pending tasks from all registered projects |
| All pending | Mouse wheel / PgUp / PgDn, r, Esc | Scroll / reload / close |
| Add / Edit form | Tab / Ctrl-S / Esc | Switch field / save / cancel |
| Queue | q | Return to Agents; in text fields, q is text |
| Queue / Viewer | Ctrl-] | Return to Agents |

Viewer forwards input to the agent, except **Ctrl-]**. The bottom bar identifies the current input target. Open dialogs capture their own input; background controls stay inactive. Unsubmitted Queue drafts survive a temporary return to Agents.

**Task details.** Click a task (or select it and press **Enter**) to switch the Tasks area to that task; **Esc** or **Back** returns to the list with the same selection and scroll position. Agents and Viewer stay as they are, and keys and scrolling in the details stay in Queue. For current, awaiting, and history tasks, saddle runs the read-only `drover show <id> --json --with-agent-status` in the project when the page opens and about every 5 seconds while it stays open, one query at a time; returning to the list, switching projects, or quitting stops it. The page follows the task id, so a task that finishes or is released keeps showing the same task. It shows:

- the status, elapsed time, and any attention hint or inconsistent-snapshot warning;
- the completion checks recomputed now, with each reason; for history tasks the completion-time checks were not saved, and today's are not applied;
- the last check-command record, which is only a previous result and is never run by the page;
- Git progress: the start..HEAD (or start..end) range count, which is not a per-task count, main's progress, and recorded and current commits;
- routing from the current task file, hold, and the attention hint (an inference, not proof that work stopped);
- start, finish, and release times, and the body.

Unknown or unrecorded values are labelled as such, never shown as zero or passing. A failed refresh shows the error and marks older details stale; the next refresh retries. Pending and unnumbered tasks are not covered by `drover show`, so their page shows the list's title, body, and status; a pending task switches to full details once it starts. **Task t** opens the full title and body in a dialog; closing it restores the detail reading position. Pending tasks have **Edit e** in both the detail pane and the text dialog. Saving or cancelling returns to the view that opened the editor. Other queue actions work from the list.

Select a pending task to use **Edit**, **Move up**, or **Move down**; other task states cannot be edited or reordered. Edit prefills the title and multiline body. Refreshes and failed saves preserve the draft; successful changes keep the task selected. The first/last pending task cannot move up/down respectively. Before writing, saddle rechecks the public pending snapshot and rejects stale content or order. The current CLI does not expose a version for atomic protection, so another writer can still race between this check and the write.

**Delete x** opens a confirmation showing the selected pending task's position, id, title, and body; press **y** or click **Delete** to confirm, or **Esc** / **Cancel** to keep it. The confirmed target is fixed when the dialog opens, so refreshes do not change it. saddle runs `drover drop --pos <position> "Deleted in saddle"` after the same pending recheck: the task leaves the pending queue and drover keeps it in History as **Dropped** (with that reason); it is not erased. Only pending tasks can be deleted here; current tasks cannot be returned to pending.

**All pending A** opens a read-only dialog listing the pending tasks of every project in `~/.drover/projects`, grouped by project with each task's queue position, id, and full title. Each project is read in the background with its own `drover list --json`; a project that is still loading or failed to read is labeled as such, with the full error, while the other projects still show their tasks. Press **r** to reload. The dialog does not edit or reorder tasks; switch to a project to act on its queue.

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
