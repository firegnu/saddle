# saddle

一个 Rust 写的终端界面程序，把 corral 的 agent 看板、任务队列和选中 agent 的实时终端放进同一个界面。设计和分步计划见 `docs/DESIGN.md`，所有决定以它为准；实现中要改设计，先改那份文档并在提交里说明。

## 规矩

- Rust stable，只用成熟、活跃维护的库；不依赖 tmux、zellij 等外部程序。
- 只通过公开命令和 corral、drover 打交道（`corral ls/status/reply/attach/stop`、`drover ...`），不读它们的内部文件，不改它们的仓库。
- 用户授权的例外：可只读 `~/.drover/projects` 枚举项目目录；任务数据和操作仍全部走公开 CLI，不直接读各项目的 `.drover.conf`、队列或状态文件。
- **不要干扰用户正在用的 agent**：`corral ls` 里现有的 agent 都是用户的。可以用 `corral ls/status/reply` 读；不要对它们 `corral stop`、`corral send`、`corral keys`，也不要 attach 上去打字。需要真实 agent 做测试时，自己开一个 `saddle/test-<名字>`（例如 `corral start saddle/test-a --cwd /tmp -- codex --yolo -m gpt-5.6-luna`），用完 `corral stop` 掉。
- 不要按项目名或路径批量杀进程（`pkill -f corral` 这类），会误杀用户的 agent。停自己起的进程用记下的 PID。
- 测试不依赖真实 agent：需要时用一个假的 `corral` 脚本输出固定 JSON。
- 项目标准测试：`cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。
- 所有 worktree 共用一个编译目录，避免每个 worktree 从头编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 开发方式（主控分派）

- 这个项目的开发任务由主控（`saddle/main`）拆开，派给别的 agent 做。主控负责拆任务、写任务文件、审查、合并，不自己写功能代码。分派时按 corral-dispatch 技能做。
- 被委派的 agent（任务文件里写明了身份）照任务文件做，不再往下派。
- 需求单只写用户要的结果；验收照抄用户原话，不补验收点。主控觉得该加的，列出来问用户。
- agent 名字以 `saddle/dev-` 开头；任务文件放 `docs/任务/`；每个任务一个分支，worktree 放 `../saddle-worktrees/<分支>`，交叉审查用 detached worktree `../saddle-worktrees/review-<分支>`。
- 审查：主控审查每个任务。
- 合并：审查通过后本地合并进 main，然后推送到 origin。
- 收尾记号：一件活合并完、worktree 和分支清干净之后，在 main 上补一条空提交（`git commit --allow-empty`），首行写「收尾: 」加一句话说明这件活是什么。只记真正落地的活；说好不合并、停在审查的不记。
- 收尾之后更新 `HANDOFF.md`：现在在哪、下一步干什么、有什么悬着。设计和理由进 `docs/DESIGN.md`，别写进交接文件。
- 开出来的 agent：清掉某个 worktree 时，把住在里面的那个一并关掉（它的工作目录没了，接不了新活）；其余的用户说关才关。
