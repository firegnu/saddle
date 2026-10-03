# saddle

**A terminal workspace for your coding agents.**

English · [简体中文](README.zh-CN.md)

See your agents and work in their live terminals. Add optional process plugins for task workflows and other tools.

```text
┌ Agents · 2        ──┬─ Viewer ──────────────────────────┐
│ project/ ─────── (2)│                                   │
│ │ ◐ main   working  │  The selected agent's terminal     │
│ │ ○ review idle     │                                   │
│                     │  Type, paste, and interact here.    │
│                     │                                   │
└─────────────────────┴───────────────────────────────────┘
  Plugins opens a searchable palette; each plugin owns its local view.
```

saddle is written in Rust with [Ratatui](https://ratatui.rs/). It hosts [corral](https://github.com/firegnu/corral) agent sessions and optional process plugins, including [Drover](plugins/drover/README.md). It does not require tmux or Zellij.

## Features

- **Agents:** agents grouped by repository (`name/ ──── (n)` headings), each with a gutter and fixed columns for status dot, name, agent type, state, and time, then its title, activity, Git line, directory (in full when it fits; leading levels give way when it does not), and instance · `ATT` (public attach count) · `VIA` (last input source). States: `?` waiting (public `blocked`, yellow), `!` error (red), `◐` working (blue), `○` idle (green), `✕` exited (faint); `▲` stalled, `◌` starting and `·` unknown keep their own looks. Within a group, agents needing a person come first (waiting → error → working → idle → exited); **s** switches to name order. `⦿` before the time marks an agent this saddle is displaying in any pane or tab; `•` marks an unseen finished turn. With more than five agents the list folds to one row per unselected agent until **z** toggles it. Below 50 columns the type column shows only its mark (`✳`, `>_`, `π`). An agent started with a public corral `effort` label shows the following two-cell signal icon after its type and before its state, including when folded:

  | Tier | Icon | Default color |
  |---|---|---|
  | medium | `⣄⡀` | soft green (`agent_idle`) |
  | high | `⣴⡀` | blue (`agent_working`) |
  | xhigh | `⣴⡇` | purple (`agent_starting`) |

  Unlit bars keep only baseline dots, and selection does not change the icon. Agents without a valid label show no icon and keep the column aligned; when no agent has one, the column is not reserved. The icon reflects the delegation label only, not the runtime's actual effort.
- **Git summary per agent:** above each agent's directory, a line such as `⎇ dev-t12 ↑2 main     +18 -4 ?1` describes the worktree at the agent's public corral `cwd`: the current branch (just `⎇` when it matches the agent's name); `↑n base`, the commits ahead of the local `main` (on `main` itself, ahead of its configured upstream, i.e. not yet pushed); uncommitted added/deleted lines against HEAD, staged and unstaged together, after Git's built-in text/eol attributes (so a committed CRLF file whose timestamp changed is not counted), per path with no rename detection (a pure rename counts as all lines deleted and added); and untracked files. The numbers belong to the directory, not the agent: agents sharing a worktree show the same line, and they do not say which agent or task made a commit. Values that cannot be determined show `—` (no local `main`, no upstream, detached HEAD, no commits yet); binary files have no line counts and are listed as `N binary`; when the changes do not fit beside the branch, they move as a whole to the next row, right-aligned; a directory that is not a Git worktree, is gone, or times out shows `git unavailable`. The line refreshes about every 5 seconds from local data only; a slow repository delays the round for every directory. It never fetches or lazily fetches missing objects, and it runs no external diff, textconv, fsmonitor hook or clean/smudge/process filter from any attributes source. When a changed file would need such a filter, the line counts show `+— -—` instead. A parent repository's line does not look inside submodule worktrees: uncommitted changes inside a submodule are not counted, while a submodule whose commit moved counts as a changed gitlink (`+1 -1`). It also skips optional index writes and ignores inherited `GIT_*` variables such as `GIT_DIR`. It does not follow an agent that later `cd`s elsewhere.
- **Drover plugin (optional):** task lists, project selection, task text/details, Links and a Telemetry link, editing and explicit dispatch/submit/accept/return actions. Open it from **Plugins → Drover**. Closing its view keeps background observation running; disabling it stops that source. It never automatically advances tasks. See [setup and controls](plugins/drover/README.md).

  **Dispatch selected:** select a Pending task and click **Dispatch selected** to send that task now; there is no need to move it to the top first. The button is unavailable while the queue is paused, a task is running or awaiting release, the queue is busy or could not be read, or the selected task is not Pending (Current, Awaiting and History tasks have no such button). It has no shortcut and asks for no confirmation.
- **Attention:** waiting/error agents, new replies and items from enabled plugins. Click a row to open its source; nothing is answered or advanced. Drover publishes awaiting tasks and failed history through this same generic interface.
- **Settings:** the `Settings` entry at the right of the Attention line (or **,** in Agents) edits the config file saddle started with, shown at the top. **General** holds the sidebar width, refresh interval; **Colors** starts with the **Theme** (Dune, Tide, Lagoon or Terminal; ←/→, Space/Enter or click), then every color, grouped, with swatches and a small preview. Choosing another theme loads all of its colors into the draft and clears the color overrides; a color you then edit is marked `custom`, and **Default** on it makes it follow the theme again; **Advanced** holds the corral command. Edits stay a draft until **Save / Ctrl-S**; **Cancel / Esc** leaves the file unchanged, and **Default / Ctrl-D** resets the selected value (Save still writes it). Save writes only the edited keys, keeping comments and the rest of the file, and creates the file and its folders if needed. Invalid values are reported and keep the draft. If the file changed on disk after Settings read it, nothing is saved: **Keep my edits** rereads the file under your draft, **Discard my edits** takes the file as it is. Saved colors and sidebar width apply at once (terminal defaults follow the theme; explicitly set program colors are kept); settings marked `Restart required` apply on the next start.
- **Task notifications (Drover plugin):** press **N** in Drover to choose System/In Saddle, then **Ctrl-S** to save. The plugin owns both task state and notification channels; there is no independent watch. First observation and preference changes establish a baseline; existing awaiting tasks are not announced again. In-Saddle prompts keep terminal focus and can open their plugin target.
- **New agents:** choose a project and Codex or Claude, then create an agent with an editable suggested name. Advanced settings hold the full command, first message, opening location, and exact call preview.
- **Viewer tabs and splits:** each tab holds a group of terminals, with left/right/up/down splits. Each pane runs an owned interactive shell or a live `corral attach`, with terminal colors, Unicode, cursor rendering, mouse events, and paste support.
- **Mouse and keyboard:** compact clickable buttons, mouse-wheel and trackpad scrolling, and shortcuts. Scrolling lists keeps the selection and survives normal refreshes.
- **Responsive layout:** Agents keep the left column; plugin overlays and workspace panels adapt to their assigned area.
- **Terminal-native appearance:** transparent panel backgrounds (Agents has its own warm dark palette), semantic state colors, and English interface labels. Task text and agent output keep their original language.

Agents are native Rust widgets; Drover renders its own Ratatui view through the public SDK. Only Viewer panes run child PTYs; saddle does not embed external board interfaces.

## Getting started

### Requirements

- Rust stable **1.96 or later**.
- The bundled Rust `corral` runtime (built with Saddle); external coding agent CLIs such as Claude Code or Codex remain separately installed. The optional Drover plugin owns its task engine. Neither runtime requires the retired Python Corral/Drover programs.
- Git 2.45 or later on `PATH` for the Agents Git summary (it needs `--no-lazy-fetch`); older or missing Git shows `git unavailable`.
- A terminal with Unicode and mouse support; true color is recommended.

Development and interactive validation currently take place on macOS. Other platforms have not been verified.

### Build and run

```sh
git clone https://github.com/firegnu/saddle.git
cd saddle
cargo build --workspace --bins --release --locked
./target/release/saddle
```

Or build an immutable product directory (does not install or switch running programs):

```sh
./scripts/package.sh /absolute/new/saddle-version
/absolute/new/saddle-version/bin/saddle
```

Use `saddle --help` to see the command-line options.

Use **New** in Agents to start an agent, or attach an existing corral session. Register task projects through the Drover plugin command API; the Saddle host does not interpret task data.

### First session

1. Select an agent on the left and press **Enter**, or click its row, to attach.
2. Type directly in Viewer to work with that agent.
3. Press **Ctrl-]** to return to Agents. Viewer stays connected.
4. For task workflows, install and enable the [Drover plugin](plugins/drover/README.md), then open **Plugins → Drover**. Esc returns within a plugin page or closes its view.
5. To exit from anywhere, press **Ctrl-]**, then **q**. Exiting confirms all running shells before ending them and disconnecting the agent displays; corral agents keep running.

**New / n** opens a form ready to create an agent: choose a **Project**, choose **Codex** (default) or **Claude**, then click **Create agent** or press **Ctrl-S**. The project defaults to Saddle’s startup directory; candidates come from public Corral cwd values and the startup directory. Click the project selector (or Ctrl-P) to choose a known directory with the mouse or Up/Down and Enter; **Edit path / Ctrl-E** allows any directory. **Role** defaults to **Controller**, with the name locked to `main`. Choose **Regular** to edit the name; its first value is `main`, and switching roles preserves the Regular draft. **Prefix**, before the name, defaults to `agents` and is editable in both roles (for example `saddle`); it must be non-empty, without spaces, `/` or a leading `-`. The full name is `Prefix/Name`, e.g. `agents/main`, in both the preview and the call. Changing the project, Codex/Claude or the role preserves the prefix, name and role drafts. Both roles pass the exact full name to `corral start`, without `--unique`; the role is also recorded as the public label `role=controller` / `role=regular`, and terminal pane titles show `Controller · name` or `Regular · name`. Agents created elsewhere with the public label `role=implementer` / `role=reviewer` show `Implementer · name` / `Reviewer · name` (`Agent · name` for agents without a valid role label; an empty pane stays `Viewer`). The role does not configure the queue or start task dispatch. Duplicate names show the normal CLI error and keep the draft, without automatic numbering. The actual created name always comes from corral's reply.

Text inputs have labeled borders, placeholders, a highlighted focus, and a visible insertion cursor. Click inside an input to position the cursor; Tab/Shift-Tab changes focus. Left/Right, Home/End, Backspace/Delete, Ctrl-U (clear), and paste edit at the cursor, including Chinese wide characters. Long lines scroll horizontally; multiline messages also scroll vertically and support Up/Down and Enter. In short windows, focus navigation or the wheel brings fields into view while Create and Cancel stay at the bottom.

**Advanced / F4** reveals the full command, optional multiline first message, **Open in** (current pane by default, or new tab / four split directions; when opened from the content picker, the location is fixed and shown as text such as *Opens in a new tab · set by + Tab* or *Opens in a split on the right · set by Split*), and the exact call preview. Clicking Codex or Claude explicitly resets the command to `codex --yolo` or `claude`; custom commands are marked **Custom command** and survive focus changes and collapsing Advanced. The built-in Codex default uses YOLO mode. Custom commands are used as entered; saddle adds no flags to them. Quoted arguments are parsed and passed directly to `corral start`, without shell expansion, pipelines, or redirections. PgUp/PgDn (or the wheel with Preview focused) scrolls the full preview. Failed starts keep the draft. **Cancel / Esc** or **Ctrl-]** returns to Agents; **n** reopens the draft, including an in-flight start.

Clicking an agent row (or Enter) shows it in the active pane, or jumps to its existing pane if it is already open anywhere. Layout starts on the right, place first and content second: **Split ▾** on the active pane's bottom border opens a small menu with **Left ←**, **Right →**, **Above ↑**, and **Below ↓** (arrow keys work too), relative to that pane; **+** in the tab strip asks for a new tab. A list titled with that place (for example *Open content on the right*) offers **Terminal**, **New agent…**, **Plugin…**, and existing agents; the pane's own agent is not offered for its split. Click an agent (or ↑↓ and Enter; the wheel scrolls long lists) and only then is the pane or tab created. An agent already open elsewhere is marked **Move here**: choosing it moves that pane — the same session, output, and any attach still in progress — without attaching again or stopping anything; a split it leaves collapses, and a tab it empties disappears. **Cancel Esc** or Esc at either step returns to the Viewer and Ctrl-] to Agents, leaving the layout exactly as it was; **Terminal** and **New agent…** remain available even with no agents. New agent opens the existing form bound to this location; cancelling leaves no empty pane. Tab buttons show Terminal or the focused pane’s agent name (long names are truncated); the arrow controls reach tabs beyond the visible strip. Click a terminal's content or title to focus it. Closing a pane collapses its split; closing the final tab leaves an empty tab. Tab changes keep background attaches running. A start or attach belongs to the pane reserved at submission: switching tabs does not redirect it, and closing or replacing that target discards its attachment result without stopping a newly created agent. A failed start can leave its reserved pane empty. Tiny windows temporarily show only one branch where a split cannot fit, retaining the full layout for expansion; an empty terminal content area receives no input. Layouts are saved automatically when they change and again on normal exit.

**Plugin…** opens the plugin list for the chosen position. Select a running plugin to put its view there, including plugins that normally open as centered overlays. An existing view shows **Move** and keeps its pane and process; a plugin cannot split beside itself. Cancelling leaves the layout unchanged, and closing a plugin pane keeps it running in the background.

On the next launch, saddle restores tabs, split directions and proportions, and each pane's agent name and original directory. It reconnects only the original agent instance if still running and available; exited, replaced or unavailable agents retain their positions. **Create new agent** opens a confirmation form with the saved name and directory, an editable Regular name, and the current command/model defaults. **Choose existing agent** picks a running agent for that location. Ordinary terminals restore as placeholders: **Open terminal** starts a fresh shell in the original launch directory, without replaying commands. Finished conversations and terminal history are not restored.

Layout state lives in `~/.local/state/saddle/layout.json`, or `$XDG_STATE_HOME/saddle/layout.json` when `XDG_STATE_HOME` is absolute. Missing files open the default layout, and the first save creates missing directories. Corrupt or unsupported state shows a notice and opens the default layout; the original file is preserved and saving stays disabled for that run. Move the old file aside before restarting to save a fresh layout. Write failures show a notice while saddle remains usable; layout changes and normal exit attempt another save. State is separate from `config.toml`.

If another terminal is attached to an agent, detach there before attaching through saddle. Stopping an agent is a separate, confirmed action.

## Local plugins

Open **Settings → Plugins (F5)** to add a trusted local plugin directory, enable it, and open its panel inside Saddle. Adding a plugin leaves it disabled; closing a panel keeps its process running, while Disable stops it. Registration stores the directory path, so keep the files in place.

The fixed Plugins entry opens a searchable launcher showing runtime status. Select a plugin and press Enter to open it or switch to its existing view; disabled or unavailable plugins remain visible with an explanation. Manage plugins opens lifecycle settings. Counter opens a centered overlay: Esc closes it and restores focus, while Ctrl-] returns to Agents. Closing keeps the plugin running. Existing workspace panels are focused rather than duplicated.

To build and package the standalone Counter example, run `./examples/counter-plugin/package.sh`. Add the resulting `examples/counter-plugin/dist/counter-plugin` directory in Settings. End users need only that directory, not Rust or development environment variables. The SDK is a development API; see the [Counter README](examples/counter-plugin/README.md) and [plugin author guide (Chinese)](docs/插件开发入门.md). The complete Drover plugin uses the same SDK. The [Diff plugin](plugins/diff/README.md) continuously shows all uncommitted changes in the source worktree, with staged/unstaged modes and overlay, tab or split placement; build it with `./plugins/diff/package.sh`.

The optional built-in **Dispatch** plugin is included in the source build and defaults to disabled. Enable it in Settings → Plugins to install its `corral-dispatch` skill resources and see the project template instructions. Existing skill links and user modifications are kept and reported as conflicts. The headless route is `echo "<task summary>" | saddle plugin run dispatch route`; it requires `TYPESAFE_API_KEY`. Optional `--record-context /abs/context.json [--brief-file /abs/brief.md]` captures the route through host telemetry; recording starts disabled, and capture failures do not change the business result. See [Dispatch setup](plugins/dispatch/README.md) and [telemetry usage](docs/遥测使用.md). Source integration does not mean the daily binary, existing skill links or consumers have been switched; that migration remains separate.

## Task telemetry

Saddle owns the independent **Telemetry** query page (top action or `t` with Agents focused). A numbered Drover task's **Telemetry ↗** opens all its runs there; task text, run details and Links remain inside Drover. The old Dispatch log tab and runtime dlog dependency have been removed. **Dispatch selected** still performs the explicit task dispatch action.

Recording is optional and starts globally disabled; only explicitly selected chains are captured. The bundled skill includes a same-version [telemetry operation guide](plugins/dispatch/resources/corral-dispatch/遥测操作.md), covering declared sources, per-dispatch identities, task snapshots, replies, reviews, closure and receipts. Business operations are never replayed to recover telemetry. The daily binary, Drover package, both global skill copies and this project’s instructions were switched on 2026-10-02; see the [cutover record](docs/任务/遥测05B-实际切换记录.md). Old logs are kept for offline historical viewing, with no import or deletion.

## Task links (Drover plugin)

Choose **Links** beside **Task text** and **Run details** in Tasks. **Tab / Shift-Tab** cycles these views. Files, commits and agents show their explicit source; click an entry or use Up/Down and Enter. File and commit previews stay inside Tasks; arrows, wheel and PgUp/PgDn scroll, and **Back / Esc** returns to the same Links selection. Esc again closes Tasks.

Only explicit references in the task body and one level of its `Task file` are collected. For example:

```text
Task file: docs/task.md
Review file: docs/review.md
Artifact: output/report.txt
Commit: abcdef123
Agent: project/worker | instance=012345abcdef
```

Chinese aliases are `任务文件`, `审查文件`, `产物`, `提交`, `代理`. Fields accept list prefixes, either colon and backticks around the target. Inline Markdown `[label](path)` file links are also recognized. Field paths are relative to the selected project root; Markdown paths are relative to the task file's directory (or the project root in task text). Fenced examples are ignored; linked documents are not scanned recursively.

Previews accept project-local regular UTF-8 files up to 1 MiB; external URLs, escaping symlinks, upstream internal data, binary or inaccessible files show a reason. Markdown is read as text. **Recorded range** uses public start/end Git endpoints (observed HEAD while running), and does not establish task ownership of its commits. Git previews are bounded and never run external diff, textconv or pagers. Agent links require the original 12-digit hexadecimal instance; missing identity is disabled, and exited, replaced or occupied agents are not opened. An already open matching instance is located without another attach. No link starts an agent or executes document commands.

## Terminal workspace control

**Terminal** starts `$SHELL -i` (fallback `/bin/sh`). The directory is the source pane's known project/start directory, or the Saddle startup directory for an empty pane; it does not follow later `cd`. Shell exit keeps its screen and exit status. Closing a running shell, replacing it, closing a mixed tab, or quitting saddle asks which shells to end; Cancel preserves every session. Agent displays only detach. Shell cleanup targets the owned PTY shell and its foreground process group, not deliberately daemonized processes.

The same binary provides JSON control without starting another TUI:

```sh
saddle ctl instances
saddle ctl inspect [--instance ID]
saddle ctl open --place right --shell
saddle ctl open --place tab --agent project/review
saddle ctl open --place down --name project/helper --cwd /absolute/project --role regular -- codex --yolo
saddle ctl request REQUEST --instance ID
saddle ctl close --pane PANE --instance ID
saddle ctl close --tab TAB --instance ID --confirmation TOKEN --confirm-shells
```

`open` defaults to `--relative-to self`, matching corral name **and instance**, or the injected shell pane identity. Use `--relative-to active` or a returned Pane ID explicitly when intended. `--place` accepts tab/left/right/up/down; `--focus` requests focus at submission, otherwise commands preserve it. Shell and new-agent content accept `--cwd`; new agents accept `--prompt` and use exact names and argv, without adding flags. UI creation focuses the target. Asynchronous completion never changes subsequent user focus or clears their form draft.

Read `instance`, `request_id`, `pane`, `revision`, `cwd`, and `cwd_source` from the result. Poll `request` while `state` is starting/attaching. `accepted`, `agent_created`, and `pty` describe separate steps; display completion does not imply model readiness. A timeout is uncertain: query/retry the original ID and identical arguments, never automatically create a new request. `--request-id` supplies the ID; an omitted ID is generated before sending. Reused IDs with different parameters conflict. Records are kept for the instance lifetime (maximum 256 modifications; new ones are then rejected); restart changes the instance ID and old requests become unavailable.

Closing running shells first returns `confirmation_required`, a stable `targets` list and `confirmation` token without changing the layout. After approving those effects, repeat close with that token and `--confirm-shells`, using a new request ID; retries of this confirmed operation retain that new ID. Changed targets invalidate the token. Active layout dialogs/forms/confirmations return `busy` to remote modifications. No ctl stop, terminal input/output, script runner or task scheduler is provided. Explicit agent stops still use public `corral stop` after checking identity.

Each TUI owns a private 0700 runtime directory entry and 0600 Unix socket: `$XDG_RUNTIME_DIR/saddle`, otherwise `$XDG_CACHE_HOME/saddle/run` or `~/.cache/saddle/run`. `SADDLE_RUNTIME_DIR` overrides the directory with an absolute path; tests use isolated directories. Long Unix socket paths fail with an actionable error. Discovery lists live instances and never chooses the newest one. Transport uses 64 KiB messages, bounded queues/workers and deadlines. Exit removes only this instance's socket. `saddle ctl --help` lists the actual interface; [skills/saddle/SKILL.md](skills/saddle/SKILL.md) documents the agent workflow.

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

```

| Setting | Meaning |
|---|---|
| `corral` | `corral` selects the adjacent bundled runtime; other names/paths explicitly select an external program |
| `left_width` | Preferred width of the left column, in terminal cells |
| `left_split` | No longer used: Agents take the whole left column with optional plugin views. Still accepted (between 0 and 1) so existing configs keep loading |
| `refresh_ms` | Background refresh interval in milliseconds |
| `mascot_enabled` | Show the animated mascot (default `true`); toggle in Settings → General → Mascot and save with Ctrl-S to apply immediately |
| `mascot` | Which pet the mascot is: `clawd` (default), `cat` or `capybara`; choose in Settings → General → Pet and save with Ctrl-S to apply immediately |
| `mascot_display` | How the pet is drawn: `auto` (default; pictures where the terminal shows them, otherwise blocks) or `blocks` (always block glyphs); choose in Settings → General → Display and save with Ctrl-S to apply immediately |



| `theme` | Built-in palette: `dune` (default, the original look), `tide` (cool blue-gray), `lagoon` (deep green-teal) or `terminal` (only the terminal's default and ANSI colors) |
| `colors` | Optional flat table of per-color overrides on top of `theme` |

Command paths support `~/`. The default never falls back to PATH if the bundled runtime is missing. Host plugins receive the resolved absolute path as `SADDLE_AGENT_BIN`; explicit plugin `--corral` overrides it. Keep old product directories while running agents still use their hooks. See [Corral integration](docs/Corral核心Rust集成设计.md). Legacy `[queue]` settings remain accepted but no longer affect the host. Move custom corral/cwd values to its plugin manifest args, as described in [Drover setup](plugins/drover/README.md). Legacy `--dispatch-log <value>` is accepted but deprecated and ignored; new manifests omit it. The plugin owns schema 2 task data and explicit actions; the host does not read Drover projects or task state.

[config.toml](config.toml) is the complete, commented default configuration, ready to copy to the path above; its colors are commented out so they follow the theme. For a theme with a small override, add:

```toml
theme = "tide"
[colors]
focus = "light_cyan"
bg = "default"
agent_selected = "#2b2621"
```

Colors accept `default` (or `reset`), `#RRGGBB`, or lowercase ANSI names: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`. `gray` is normal ANSI white; `dark_gray` is bright black; `white` is bright white. ANSI colors follow the terminal palette.

The table covers backgrounds, selection, borders, focus, text levels, connection/unread indicators, action feedback, agent types, and reply formatting. The Agents column uses its own `agents_*` palette (plus `agent_selected` and the agent-type accents); the shared `agent_*` status accents color the Agents stalled/starting states and effort icon; plugins receive the generic text/background/accent/error theme and own their business colors. Unless the terminal sets `COLORTERM` to `truecolor` or `24bit`, the Agents-only RGB colors are sent as their nearest 256-color entries; ANSI names and other areas are unchanged. Invalid colors or unknown settings report a configuration error. Colors saved from Settings apply at once; edits made outside saddle apply on the next launch, as there is no hot reload. Viewer terminal defaults follow `colors.text` / `colors.bg`; colors explicitly set by the program are kept. Font family and size belong to your terminal settings.

## Controls

| Context | Input | Action |
|---|---|---|
| Agents | ↑↓ / j k | Select an agent |
| Agents | Enter / click a row | Attach in the active pane, or jump to the agent's existing pane |
| Agents | n / New | Open the new-agent form |
| Agents | / / Search | Filter agents by project or name; Enter or a click opens the agent (jumping to its pane if already open), Esc cancels |
| Agents | a / Attention · N | Open Attention; ↑↓ select, Enter or a click opens the agent or task, Esc cancels |
| Agents | , / Settings | Open Settings; Tab/↑↓ select a value, F1–F5 or a click switch page, Ctrl-U clears, Ctrl-D restores the default, Ctrl-S saves, Esc cancels |
| New-agent form | Tab / Shift-Tab, Ctrl-U | Switch field, clear field |
| New-agent form | Ctrl-P / Project, Ctrl-E / Edit path | Choose an observed agent project/startup directory or edit its path |
| New-agent form | Left/Right in Open in | Choose current pane (default), new tab, or a split direction; content-picker placement stays fixed and is shown as text |
| New-agent form | F4 / Advanced, Ctrl-S / Create agent, Esc | Toggle advanced settings, create, or return while keeping the draft |
| Viewer chrome | Split ▾, then a side / + | Choose a split side or a new tab, then Terminal, New agent, Plugin, or an agent to open or move there |
| Viewer chrome | tab / pane title / × | Select a tab/pane, close a tab |
| Viewer chrome | Zoom / Restore | With several panes, temporarily fill the terminal area with the focused pane (Agents and tabs stay); Restore returns the same split with that pane focused. Other panes keep running; focusing another pane, closing the zoomed pane, or a new split ends the zoom |
| Viewer chrome | Close pane / Close tab | Confirm running shells, then close the pane or tab; agent displays only detach |
| Agents | Mouse wheel / trackpad | Scroll the list without changing selection |
| Agents | Tab / Shift-Tab | Focus Viewer |
| Anywhere | Plugins | Search plugins and open or switch to a running view |
| Agents | PgUp / PgDn | Scroll the agent list |
| Agents | s / Sort | Toggle status (default) / name order within repositories |
| Agents | z / Fold | Fold unselected agents to one row, or expand them again |
| Agents | x / Stop, then y | Stop the selected agent; other keys cancel |
| Agents | q | Quit saddle |
| Tasks | ↑↓ / j k / click a task | Select a task; its text or run details show beside the list |
| Tasks | t / Task text, Enter / Run details | Show the task text / its run details |
| Tasks | PgUp / PgDn | Scroll the task text or run details |
| Tasks | Mouse wheel / trackpad | Scroll whichever of the list or the content is under the pointer, without changing selection |
| Tasks | c / project selector | Open the project picker |
| Project picker | Enter / click, e, r | Open project, enter a path, reload registry |
| Path form | Ctrl-U / Enter / Esc | Clear / apply / cancel |
| Tasks (plugin) | r | Refresh; dispatch/submit/accept use explicit task buttons |
| Tasks (plugin) | p / N | Pause or resume dispatch / notification preference |
| Tasks | a / ? | Add a task / open help |
| Tasks, running task selected | Return to pending… (click) | Confirm work has stopped, give a reason, and return the same task to the front of Pending without changing the pause setting; retains run history and does not stop agents |
| Tasks, pending task selected | e / u / d / x | Edit / move up / move down / delete (confirm with y) |
| Tasks | A | Show pending tasks from all registered projects |
| All pending | Mouse wheel / PgUp / PgDn, r, Esc | Scroll / reload / back |
| Add / Edit form | Tab / Ctrl-S / Esc | Switch field / save / cancel |
| Help / result / other pages | Esc / Back | Return to the list |
| Tasks | Esc / Close, q | Close Tasks and return to the previous input target; in text fields, q is text |
| Tasks / Viewer | Ctrl-] | Return to Agents (Tasks closes, keeping its state) |

Viewer forwards input to the agent, except **Ctrl-]**. The bottom bar identifies the current input target. Open dialogs capture their own input; background controls stay inactive. While Tasks is open, keys and the mouse act only on it; the terminals keep running underneath and are not resized. Closing and reopening Tasks keeps the project, the selected task, the text/details view, scroll positions, and any unsubmitted add/edit draft.

**Task text and run details.** The plugin reads task data directly. Run details refresh while visible, showing recorded run/submission/acceptance/return history and repository references. Git state and old check results never gate task submission or acceptance. The page never executes check commands. Missing values stay unknown; a failed refresh marks earlier details stale.

Select a pending task to use **Edit**, **Move up**, or **Move down** at the bottom of Tasks; other task states cannot be edited or reordered. Edit opens a full-size form in the same popup and prefills the title and multiline body; saving or cancelling returns to the same task and view. Refreshes and failed saves preserve the draft; successful changes keep the task selected. The first/last pending task cannot move up/down respectively. The plugin checks the displayed pending content and order under the task write lock; command clients also supply the displayed queue token.

**Delete x** opens a confirmation showing the selected pending task's position, id, title, and body; press **y** or click **Delete** to confirm, or **Esc** / **Cancel** to keep it. The confirmed target is fixed when the dialog opens, so refreshes do not change it. The Drover plugin records the deletion under the same lock and pending recheck: the task leaves the pending queue and drover keeps it in History as **Dropped** (with that reason); it is not erased. Only pending tasks can be deleted here; running tasks use the separate Return to pending confirmation.

**All pending A** opens a read-only page in the Tasks popup listing the pending tasks of every project in `~/.drover/projects`, grouped by project with each task's queue position, id, and full title. Each project is read in the background by the plugin core; a project that is still loading or failed to read is labeled as such, with the full error, while the other projects still show their tasks. Press **r** to reload. The page does not edit or reorder tasks; switch to a project to act on its queue.

**Pause applies to the open project; Dispatch selected, Submit for acceptance, Accept, and Return to pending apply to their explicitly selected task.** A failed refresh disables actions on stale queue data. No active work is shown as `No active tasks`; history remains available, with a range indicator at the bottom.

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
