# saddle

**A terminal workspace for your coding agents.**

English · [简体中文](README.zh-CN.md)

See your agents, manage the task queue, and work in an agent's live terminal—all in one window.

```text
┌ Agents · 2  Tasks ──┬─ Viewer ──────────────────────────┐
│ project/ ─────── (2)│                                   │
│ │ ◐ main   working  │  The selected agent's terminal     │
│ │ ○ review idle     │                                   │
│                     │  Type, paste, and interact here.    │
│                     │                                   │
└─────────────────────┴───────────────────────────────────┘
  Tasks opens as a large popup centered on the screen, over the Viewer.
```

saddle is written in Rust with [Ratatui](https://ratatui.rs/). It brings together [corral](https://github.com/firegnu/corral) agent sessions and the [drover](https://github.com/firegnu/drover) task queue. It does not require tmux or Zellij.

## Features

- **Agents:** agents grouped by repository (`name/ ──── (n)` headings), each with a gutter and fixed columns for status dot, name, agent type, state, and time, then its title, activity, Git line, directory (in full when it fits; leading levels give way when it does not), and instance · `ATT` (public attach count) · `VIA` (last input source). States: `?` waiting (public `blocked`, yellow), `!` error (red), `◐` working (blue), `○` idle (green), `✕` exited (faint); `▲` stalled, `◌` starting and `·` unknown keep their own looks. Within a group, agents needing a person come first (waiting → error → working → idle → exited); **s** switches to name order. `⦿` before the time marks an agent this saddle is displaying in any pane or tab; `•` marks an unseen finished turn. With more than five agents the list folds to one row per unselected agent until **z** toggles it. Below 50 columns the type column shows only its mark (`✳`, `>_`, `π`). An agent started with a public corral `effort` label shows the following two-cell signal icon after its type and before its state, including when folded:

  | Tier | Icon | Default color |
  |---|---|---|
  | medium | `⣄⡀` | soft green (`agent_idle`) |
  | high | `⣴⡀` | blue (`agent_working`) |
  | xhigh | `⣴⡇` | purple (`agent_starting`) |

  Unlit bars keep only baseline dots, and selection does not change the icon. Agents without a valid label show no icon and keep the column aligned; when no agent has one, the column is not reserved. The icon reflects the delegation label only, not the runtime's actual effort.
- **Git summary per agent:** above each agent's directory, a line such as `⎇ dev-t12 ↑2 main     +18 -4 ?1` describes the worktree at the agent's public corral `cwd`: the current branch (just `⎇` when it matches the agent's name); `↑n base`, the commits ahead of the local `main` (on `main` itself, ahead of its configured upstream, i.e. not yet pushed); uncommitted added/deleted lines against HEAD, staged and unstaged together, after Git's built-in text/eol attributes (so a committed CRLF file whose timestamp changed is not counted), per path with no rename detection (a pure rename counts as all lines deleted and added); and untracked files. The numbers belong to the directory, not the agent: agents sharing a worktree show the same line, and they do not say which agent or task made a commit. Values that cannot be determined show `—` (no local `main`, no upstream, detached HEAD, no commits yet); binary files have no line counts and are listed as `N binary`; when the changes do not fit beside the branch, they move as a whole to the next row, right-aligned; a directory that is not a Git worktree, is gone, or times out shows `git unavailable`. The line refreshes about every 5 seconds from local data only; a slow repository delays the round for every directory. It never fetches or lazily fetches missing objects, and it runs no external diff, textconv, fsmonitor hook or clean/smudge/process filter from any attributes source. When a changed file would need such a filter, the line counts show `+— -—` instead. A parent repository's line does not look inside submodule worktrees: uncommitted changes inside a submodule are not counted, while a submodule whose commit moved counts as a changed gitlink (`+1 -1`). It also skips optional index writes and ignores inherited `GIT_*` variables such as `GIT_DIR`. It does not follow an agent that later `cd`s elsewhere.
- **Tasks:** a large popup opened from the **Tasks** entry at the top of Agents (or **Tab**). While closed, the entry keeps the project's short state in its status color — `Awaiting release`, `Running`, `Paused`, the pending count, or `Idle` (`Loading…` / `Read failed` until the queue is read) — so you can tell whether anything needs attention. Project and queue actions sit on top; current, awaiting, pending, and historical tasks are listed on the left with the selected task's text or run details beside them. Add tasks, edit, reorder, and delete pending tasks, view pending tasks across all registered projects, switch projects, release work, and control pause and loop settings through native controls.

  **Dispatch selected:** select a Pending task and click **Dispatch selected** to send that task now; there is no need to move it to the top first. The button is unavailable while the queue is paused, a task is running or awaiting release, the queue is busy or could not be read, or the selected task is not Pending (Current, Awaiting and History tasks have no such button). It has no shortcut and asks for no confirmation.
- **Attention:** the `Attention · N` line under the Agents header counts what needs you across agents and every project in `~/.drover/projects`; click it or press **a** in Agents. **Needs attention** lists agents waiting for input or in error, tasks awaiting release, and failed tasks from the queue history; **New replies** lists agents with an unseen finished turn from this run. Each agent takes one row, its need first. Rows leave only when the public state changes (answered, released) or, for new replies, when you view the agent. **Mark seen / m** hides a failed history task for this run only; the queue history is unchanged, and a restart shows it again. A source that cannot be read (corral, the project list, or a project) is shown as a failed row, never as nothing to do; `…` / `loading…` marks sources not read yet. Projects are reread about every five refresh periods and whenever Attention opens. Opening a row shows the agent's terminal, or opens Tasks on that project with the task selected by its id; nothing is answered, released or advanced.
- **Settings:** the `Settings` entry at the right of the Attention line (or **,** in Agents) edits the config file saddle started with, shown at the top. **General** holds the sidebar width, refresh interval and initial Tasks project (empty means Automatic); **Colors** holds every `[colors]` value, grouped, with swatches and a small preview; **Advanced** holds the corral and drover commands. Edits stay a draft until **Save / Ctrl-S**; **Cancel / Esc** leaves the file unchanged, and **Default / Ctrl-D** resets the selected value (Save still writes it). Save writes only the edited keys, keeping comments and the rest of the file, and creates the file and its folders if needed. Invalid values are reported and keep the draft. If the file changed on disk after Settings read it, nothing is saved: **Keep my edits** rereads the file under your draft, **Discard my edits** takes the file as it is. Saved colors and sidebar width apply at once (agent output keeps its own colors); settings marked `Restart required` apply on the next start.
- **Task notifications:** Settings **General** also chooses where Drover's "task done, awaiting release" notices appear, for your user across all registered projects: **System** (Drover's system notifications) or **In saddle** (Drover's system notifications off; saddle shows a short prompt at the bottom right, such as `saddle · T34 ready for review`, for about five seconds). The choice is Drover's own preference, read and saved with `drover notifications`, never in saddle's config; Space/←→ or a click chooses, and Save hands it to Drover, which applies it at its next notification check. A prompt never takes focus or keyboard input; clicking it opens that task in Tasks (several at once open Attention), × closes it. Closing or opening a prompt does not release anything; the task stays in Attention. Tasks already awaiting at start or when the choice changes are not prompted, and a run is prompted at most once per saddle run. If Drover cannot report the choice, Settings says so and saddle shows no prompts.
- **New agents:** choose a project and Codex or Claude, then create an agent with an editable suggested name. Advanced settings hold the full command, first message, opening location, and exact call preview.
- **Viewer tabs and splits:** each tab holds a group of terminals, with left/right/up/down splits. Each pane runs an owned interactive shell or a live `corral attach`, with terminal colors, Unicode, cursor rendering, mouse events, and paste support.
- **Mouse and keyboard:** compact clickable buttons, mouse-wheel and trackpad scrolling, and shortcuts. Scrolling lists keeps the selection and survives normal refreshes.
- **Responsive layout:** Agents keep the whole left column at every width; the Tasks popup takes about 85% of the window and stacks its list above the content when narrow.
- **Terminal-native appearance:** transparent panel backgrounds (Agents has its own warm dark palette), semantic state colors, and English interface labels. Task text and agent output keep their original language.

Agents and Tasks are native Rust widgets. Only Viewer panes run child PTYs; saddle does not embed external board interfaces.

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

Use `saddle --help` to see the command-line options.

Use **New** in Agents to start an agent, or attach an existing corral session. Register queue projects with drover separately; saddle does not initialize queue projects.

### First session

1. Select an agent on the left and press **Enter**, or click its row, to attach.
2. Type directly in Viewer to work with that agent.
3. Press **Ctrl-]** to return to Agents. Viewer stays connected.
4. Press **Tab**, or click **Tasks** at the top of Agents, to open the task popup. Use the project selector at its top to choose a registered project; **Esc** closes it and returns input where it was.
5. To exit from anywhere, press **Ctrl-]**, then **q**. Exiting confirms all running shells before ending them and disconnecting the agent displays; corral agents keep running.

**New / n** opens a form ready to create an agent: choose a **Project**, choose **Codex** (default) or **Claude**, then click **Create agent** or press **Ctrl-S**. The project defaults to the current Tasks directory; selecting a different project only affects this new agent. Click the project selector (or Ctrl-P) to choose a registered directory with the mouse or Up/Down and Enter; **Edit path / Ctrl-E** allows any directory. **Role** defaults to **Controller**, with the name locked to `main`. Choose **Regular** to edit the name; its first value is `main`, and switching roles preserves the Regular draft. **Prefix**, before the name, defaults to `agents` and is editable in both roles (for example `saddle`); it must be non-empty, without spaces, `/` or a leading `-`. The full name is `Prefix/Name`, e.g. `agents/main`, in both the preview and the call. Changing the project, Codex/Claude or the role preserves the prefix, name and role drafts. Both roles pass the exact full name to `corral start`, without `--unique`; the role is also recorded as the public label `role=controller` / `role=regular`, and terminal pane titles show `Controller · name` or `Regular · name`. Agents created elsewhere with the public label `role=implementer` / `role=reviewer` show `Implementer · name` / `Reviewer · name` (`Agent · name` for agents without a valid role label; an empty pane stays `Viewer`). The role does not configure the queue or start task dispatch. Duplicate names show the normal CLI error and keep the draft, without automatic numbering. The actual created name always comes from corral's reply.

Text inputs have labeled borders, placeholders, a highlighted focus, and a visible insertion cursor. Click inside an input to position the cursor; Tab/Shift-Tab changes focus. Left/Right, Home/End, Backspace/Delete, Ctrl-U (clear), and paste edit at the cursor, including Chinese wide characters. Long lines scroll horizontally; multiline messages also scroll vertically and support Up/Down and Enter. In short windows, focus navigation or the wheel brings fields into view while Create and Cancel stay at the bottom.

**Advanced / F4** reveals the full command, optional multiline first message, **Open in** (current pane by default, or new tab / four split directions; when opened from the content picker, the location is fixed and shown as text such as *Opens in a new tab · set by + Tab* or *Opens in a split on the right · set by Split*), and the exact call preview. Clicking Codex or Claude explicitly resets the command to `codex --yolo` or `claude`; custom commands are marked **Custom command** and survive focus changes and collapsing Advanced. The built-in Codex default uses YOLO mode. Custom commands are used as entered; saddle adds no flags to them. Quoted arguments are parsed and passed directly to `corral start`, without shell expansion, pipelines, or redirections. PgUp/PgDn (or the wheel with Preview focused) scrolls the full preview. Failed starts keep the draft. **Cancel / Esc** or **Ctrl-]** returns to Agents; **n** reopens the draft, including an in-flight start.

Clicking an agent row (or Enter) shows it in the active pane, or jumps to its existing pane if it is already open anywhere. Layout starts on the right, place first and content second: **Split ▾** on the active pane's bottom border opens a small menu with **Left ←**, **Right →**, **Above ↑**, and **Below ↓** (arrow keys work too), relative to that pane; **+** in the tab strip asks for a new tab. A list titled with that place (for example *Open content on the right*) offers **Terminal**, **New agent…**, and existing agents; the pane's own agent is not offered for its split. Click an agent (or ↑↓ and Enter; the wheel scrolls long lists) and only then is the pane or tab created. An agent already open elsewhere is marked **Move here**: choosing it moves that pane — the same session, output, and any attach still in progress — without attaching again or stopping anything; a split it leaves collapses, and a tab it empties disappears. **Cancel Esc** or Esc at either step returns to the Viewer and Ctrl-] to Agents, leaving the layout exactly as it was; **Terminal** and **New agent…** remain available even with no agents. New agent opens the existing form bound to this location; cancelling leaves no empty pane. Tab buttons show Terminal or the focused pane’s agent name (long names are truncated); the arrow controls reach tabs beyond the visible strip. Click a terminal's content or title to focus it. Closing a pane collapses its split; closing the final tab leaves an empty tab. Tab changes keep background attaches running. A start or attach belongs to the pane reserved at submission: switching tabs does not redirect it, and closing or replacing that target discards its attachment result without stopping a newly created agent. A failed start can leave its reserved pane empty. Tiny windows temporarily show only one branch where a split cannot fit, retaining the full layout for expansion; an empty terminal content area receives no input. Layouts are saved automatically when they change and again on normal exit.

On the next launch, saddle restores tabs, split directions and proportions, and each pane's agent name and original directory. It reconnects only the original agent instance if still running and available; exited, replaced or unavailable agents retain their positions. **Create new agent** opens a confirmation form with the saved name and directory, an editable Regular name, and the current command/model defaults. **Choose existing agent** picks a running agent for that location. Ordinary terminals restore as placeholders: **Open terminal** starts a fresh shell in the original launch directory, without replaying commands. Finished conversations and terminal history are not restored.

Layout state lives in `~/.local/state/saddle/layout.json`, or `$XDG_STATE_HOME/saddle/layout.json` when `XDG_STATE_HOME` is absolute. Missing files open the default layout, and the first save creates missing directories. Corrupt or unsupported state shows a notice and opens the default layout; the original file is preserved and saving stays disabled for that run. Move the old file aside before restarting to save a fresh layout. Write failures show a notice while saddle remains usable; layout changes and normal exit attempt another save. State is separate from `config.toml`.

If another terminal is attached to an agent, detach there before attaching through saddle. Stopping an agent is a separate, confirmed action.

## Local plugins

Open **Settings → Plugins (F5)** to add a trusted local plugin directory, enable it, and open its panel inside Saddle. Adding a plugin leaves it disabled; closing a panel keeps its process running, while Disable stops it. Registration stores the directory path, so keep the files in place.

The fixed Plugins entry opens a searchable launcher showing runtime status. Select a plugin and press Enter to open it or switch to its existing view; disabled or unavailable plugins remain visible with an explanation. Manage plugins opens lifecycle settings. Counter opens a centered overlay: Esc closes it and restores focus, while Ctrl-] returns to Agents. Closing keeps the plugin running. Existing workspace panels are focused rather than duplicated.

To build and package the standalone Counter example, run `./examples/counter-plugin/package.sh`. Add the resulting `examples/counter-plugin/dist/counter-plugin` directory in Settings. End users need only that directory, not Rust or development environment variables. The SDK is a development API; see the [Counter README](examples/counter-plugin/README.md) and [plugin author guide (Chinese)](docs/插件开发入门.md). Drover migration is a separate future step.

## Task dispatch records

**Dispatch** (after **Run details**) shows how the selected task was handed out, when the controller recorded it with the separate [dispatch-log](https://github.com/firegnu/dispatch-log) recorder: each dispatch, rework and review in recorded order, one row per step with its time, target, model/effort and short result — the JEV route (its input, request, full parsed response and suggestion), the controller's decision and budget, the start with its task file snapshot, sends, the implementer's reply and review notes. Up/Down and Enter (or a click) open a step's full text inside Tasks; **Back / Esc** returns to the same row. Records are read only through the public `dlog ls --project <project> --task <id>`, `dlog show <id>` and `dlog cat <sha256>`, once per task opening (**Refresh r** reads them again); saddle never records, routes or sends through dispatch-log. Records are matched by the project root and explicit task number only; unnumbered tasks have none. Snapshots are copies saved at dispatch time, not the current files (those are in Links). The recorder is optional: when `queue.dispatch_log` is not found, there are no records, a record is missing a part, or a read fails or is in an unknown format, Dispatch says which, and the other views and task actions work as before.

## Task links

Choose **Links** beside **Task text**, **Run details** and **Dispatch** in Tasks. **Tab / Shift-Tab** cycles these views. Files, commits and agents show their explicit source; click an entry or use Up/Down and Enter. File and commit previews stay inside Tasks; arrows, wheel and PgUp/PgDn scroll, and **Back / Esc** returns to the same Links selection. Esc again closes Tasks.

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

**Terminal** starts `$SHELL -i` (fallback `/bin/sh`). The directory is the source pane's known project/start directory, or the Tasks project for an empty pane; it does not follow later `cd`. Shell exit keeps its screen and exit status. Closing a running shell, replacing it, closing a mixed tab, or quitting saddle asks which shells to end; Cancel preserves every session. Agent displays only detach. Shell cleanup targets the owned PTY shell and its foreground process group, not deliberately daemonized processes.

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

[queue]
drover = "drover"
# cwd = "~/projects/my-project"
dispatch_log = "dlog"
```

| Setting | Meaning |
|---|---|
| `corral` | corral executable name or path |
| `left_width` | Preferred width of the left column, in terminal cells |
| `left_split` | No longer used: Agents take the whole left column and Tasks is a popup. Still accepted (between 0 and 1) so existing configs keep loading |
| `refresh_ms` | Background refresh interval in milliseconds |
| `queue.drover` | drover executable name or path |
| `queue.cwd` | Optional initial queue project directory |
| `queue.dispatch_log` | Optional dispatch-log (`dlog`) executable name or path for the Dispatch view |
| `colors` | Optional flat table for interface and agent-type colors |

Command paths and `queue.cwd` support `~/`. Queue reads the project registry at `~/.drover/projects`: it prefers `queue.cwd`, then the startup directory if registered, then the first registered project. With no registry entries, it tries the startup directory. The project picker also accepts a manual path; switching projects only affects the current session.

saddle gets task data through **`drover list --json`**. Full history requires a drover version that returns the complete history array; older versions return only the latest ten records. saddle cannot display records that the interface omits. Task details come from the read-only **`drover show Tn --json --with-agent-status`** (schema version 1); a drover without it shows the error on the details page. Apart from the project registry, it does not read corral or drover's internal data files.

[config.toml](config.toml) is the complete, commented default configuration, ready to copy to the path above. Its defaults preserve the current appearance. For a small override, add:

```toml
[colors]
focus = "light_cyan"
bg = "default"
agent_selected = "#2b2621"
```

Colors accept `default` (or `reset`), `#RRGGBB`, or lowercase ANSI names: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`. `gray` is normal ANSI white; `dark_gray` is bright black; `white` is bright white. ANSI colors follow the terminal palette.

The table covers backgrounds, selection, borders, focus, text levels, connection/unread indicators, action feedback, agent types, and reply formatting. The Agents column uses its own `agents_*` palette (plus `agent_selected` and the agent-type accents); the shared `agent_*` status accents color Tasks, including Queue action feedback, and the Agents stalled/starting states and effort icon. Unless the terminal sets `COLORTERM` to `truecolor` or `24bit`, the Agents-only RGB colors are sent as their nearest 256-color entries; ANSI names and other areas are unchanged. Invalid colors or unknown settings report a configuration error. Colors saved from Settings apply at once; edits made outside saddle apply on the next launch, as there is no hot reload. Viewer terminal output keeps its own colors. Font family and size belong to your terminal settings.

## Controls

| Context | Input | Action |
|---|---|---|
| Agents | ↑↓ / j k | Select an agent |
| Agents | Enter / click a row | Attach in the active pane, or jump to the agent's existing pane |
| Agents | n / New | Open the new-agent form |
| Agents | / / Search | Filter agents by project or name; Enter or a click opens the agent (jumping to its pane if already open), Esc cancels |
| Agents | a / Attention · N | Open Attention; ↑↓ select, Enter or a click opens the agent or task, m marks a failed task seen, Esc cancels |
| Agents | , / Settings | Open Settings; Tab/↑↓ select a value, F1–F5 or a click switch page, Ctrl-U clears, Ctrl-D restores the default, Ctrl-S saves, Esc cancels |
| New-agent form | Tab / Shift-Tab, Ctrl-U | Switch field, clear field |
| New-agent form | Ctrl-P / Project, Ctrl-E / Edit path | Choose a registered project or edit its path |
| New-agent form | Left/Right in Open in | Choose current pane (default), new tab, or a split direction; content-picker placement stays fixed and is shown as text |
| New-agent form | F4 / Advanced, Ctrl-S / Create agent, Esc | Toggle advanced settings, create, or return while keeping the draft |
| Viewer chrome | Split ▾, then a side / + | Choose a split side or a new tab, then Terminal, New agent, or an agent to open or move there |
| Viewer chrome | tab / pane title / × | Select a tab/pane, close a tab |
| Viewer chrome | Zoom / Restore | With several panes, temporarily fill the terminal area with the focused pane (Agents and tabs stay); Restore returns the same split with that pane focused. Other panes keep running; focusing another pane, closing the zoomed pane, or a new split ends the zoom |
| Viewer chrome | Close pane / Close tab | Confirm running shells, then close the pane or tab; agent displays only detach |
| Agents | Mouse wheel / trackpad | Scroll the list without changing selection |
| Agents | Tab / Shift-Tab | Open Tasks / focus Viewer |
| Anywhere | Click Tasks (top of Agents) | Open Tasks; closing returns to the previous input target |
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
| Tasks | r / g / n | Refresh / check and release / send next task |
| Tasks | p / l | Pause or resume / toggle loop |
| Tasks | a / ? | Add a task / open help |
| Tasks, running task selected | Return to pending… (click) | Confirm work has stopped, give a reason, and return the same task to the front of Pending with the queue paused; retains run history and does not stop agents |
| Tasks, pending task selected | e / u / d / x | Edit / move up / move down / delete (confirm with y) |
| Tasks | A | Show pending tasks from all registered projects |
| All pending | Mouse wheel / PgUp / PgDn, r, Esc | Scroll / reload / back |
| Add / Edit form | Tab / Ctrl-S / Esc | Switch field / save / cancel |
| Help / result / other pages | Esc / Back | Return to the list |
| Tasks | Esc / Close, q | Close Tasks and return to the previous input target; in text fields, q is text |
| Tasks / Viewer | Ctrl-] | Return to Agents (Tasks closes, keeping its state) |

Viewer forwards input to the agent, except **Ctrl-]**. The bottom bar identifies the current input target. Open dialogs capture their own input; background controls stay inactive. While Tasks is open, keys and the mouse act only on it; the terminals keep running underneath and are not resized. Closing and reopening Tasks keeps the project, the selected task, the text/details view, scroll positions, and any unsubmitted add/edit draft.

**Task text and run details.** The selected task shows beside the list, first as its full title and body (**Task text t**). **Run details ↵** switches to its status and progress; the chosen view stays as you move between tasks. For current, awaiting, and history tasks, run details come from the read-only `drover show <id> --json --with-agent-status`, run in the project when the view shows and about every 5 seconds while it stays open, one query at a time; choosing Task text, another task, switching projects, closing Tasks, or quitting stops it. The details follow the task id, so a task that finishes or is released keeps showing the same task. They show:

- the status, elapsed time, and any attention hint or inconsistent-snapshot warning;
- the completion checks recomputed now, with each reason; for history tasks the completion-time checks were not saved, and today's are not applied;
- the last check-command record, which is only a previous result and is never run by the page;
- Git progress: the start..HEAD (or start..end) range count, which is not a per-task count, main's progress, and recorded and current commits;
- routing from the current task file, hold, and the attention hint (an inference, not proof that work stopped);
- start, finish, and release times, and the body.

Unknown or unrecorded values are labelled as such, never shown as zero or passing. A failed refresh shows the error and marks older details stale; the next refresh retries. Pending and unnumbered tasks are not covered by `drover show`, so their details show the list's title, body, and status; a pending task switches to full details once it starts.

Select a pending task to use **Edit**, **Move up**, or **Move down** at the bottom of Tasks; other task states cannot be edited or reordered. Edit opens a full-size form in the same popup and prefills the title and multiline body; saving or cancelling returns to the same task and view. Refreshes and failed saves preserve the draft; successful changes keep the task selected. The first/last pending task cannot move up/down respectively. Before writing, saddle rechecks the public pending snapshot and rejects stale content or order. The current CLI does not expose a version for atomic protection, so another writer can still race between this check and the write.

**Delete x** opens a confirmation showing the selected pending task's position, id, title, and body; press **y** or click **Delete** to confirm, or **Esc** / **Cancel** to keep it. The confirmed target is fixed when the dialog opens, so refreshes do not change it. saddle runs `drover drop --pos <position> "Deleted in saddle"` after the same pending recheck: the task leaves the pending queue and drover keeps it in History as **Dropped** (with that reason); it is not erased. Only pending tasks can be deleted here; current tasks cannot be returned to pending.

**All pending A** opens a read-only page in the Tasks popup listing the pending tasks of every project in `~/.drover/projects`, grouped by project with each task's queue position, id, and full title. Each project is read in the background with its own `drover list --json`; a project that is still loading or failed to read is labeled as such, with the full error, while the other projects still show their tasks. Press **r** to reload. The page does not edit or reorder tasks; switch to a project to act on its queue.

**Next, Check & release, Pause, and Loop apply to the selected project**, regardless of which history task is highlighted. A failed refresh disables actions on stale queue data. No active work is shown as `No active tasks`; history remains available, with a range indicator at the bottom.

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
