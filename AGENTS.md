# saddle

一个 Rust 写的终端界面程序，把 corral 的 agent 看板、任务队列和选中 agent 的实时终端放进同一个界面。设计和分步计划见 `docs/DESIGN.md`，所有决定以它为准；实现中要改设计，先改那份文档并在提交里说明。

## 规矩

- Rust stable，只用成熟、活跃维护的库；不依赖 tmux、zellij 等外部程序。
- 只通过公开命令和 corral、drover 打交道（`corral ls/status/reply/attach/stop`、`drover ...`），不读它们的内部文件，不改它们的仓库。
- 用户授权的例外：可只读 `~/.drover/projects` 枚举项目目录；任务数据和操作仍全部走公开 CLI，不直接读各项目的 `.drover.conf`、队列或状态文件。
- **不要干扰用户正在用的 agent**：`corral ls` 里现有的 agent 都是用户的。可以用 `corral ls/status/reply` 读；不要对它们 `corral stop`、`corral send`、`corral keys`，也不要 attach 上去打字。需要真实 agent 做测试时，自己开一个 `saddle/test-<名字>`（例如 `corral start saddle/test-a --cwd /tmp -- codex --yolo -m gpt-5.6-luna`），用完 `corral stop` 掉。
- 不要按项目名或路径批量杀进程（`pkill -f corral` 这类），会误杀用户的 agent。停自己起的进程用记下的 PID。
- 直接在 main 上开发、按功能小步提交；仅在用户明确授权发布时推送。
- 测试不依赖真实 agent：需要时用一个假的 `corral` 脚本输出固定 JSON。
- 进度和没做完的事写在 `docs/PROGRESS.md`，方便下次接着干。
