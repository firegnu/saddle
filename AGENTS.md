# saddle

一个 Rust 写的终端界面程序，把 corral 的 agent 看板、任务队列和选中 agent 的实时终端放进同一个界面。设计和分步计划见 `docs/DESIGN.md`，所有决定以它为准；实现中要改设计，先改那份文档并在提交里说明。

## 规矩

- Rust stable，只用成熟、活跃维护的库；不依赖 tmux、zellij 等外部程序。
- Corral 仍只通过公开命令访问，不改它的仓库。旧 dispatch-log 已退出本项目操作链，旧程序与数据保留供离线查看。用户授权完整替换 Drover：任务核心、数据和通知由 `plugins/drover/` 单一插件拥有，可读写沿用的 Drover 数据格式；旧独立 CLI/watch 退役。Saddle 宿主只提供通用插件接口，不读取业务文件。主控任务操作走 `saddle ctl plugin`，见 `plugins/drover/README.md`。
- **不要干扰用户正在用的 agent**：`corral ls` 里现有的 agent 都是用户的。可以用 `corral ls/status/reply` 读；不要对它们 `corral stop`、`corral send`、`corral keys`，也不要 attach 上去打字（主控按分派流程开出来的 `saddle/dev-*`、`saddle/test-*` 是它自己的，照流程送话、关闭）。需要真实 agent 做测试时，自己开一个 `saddle/test-<名字>`（例如 `corral start saddle/test-a --cwd /tmp -- codex --yolo -m gpt-5.6-luna`），用完 `corral stop` 掉。
- 不要按项目名或路径批量杀进程（`pkill -f corral` 这类），会误杀用户的 agent。停自己起的进程用记下的 PID。
- 测试不依赖真实 agent：需要时用一个假的 `corral` 脚本输出固定 JSON。
- 项目标准测试：`cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。
- 所有 worktree 共用一个编译目录，避免每个 worktree 从头编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 开发方式（主控分派）

- 主控按 corral-dispatch 技能执行项目已授权的分派流程；本链路显式选择记录时，先读该技能同目录的遥测操作.md，使用 Saddle 公开遥测、agent 与插件命令。全局关闭或本链路未选不自行开启；记录准备失败不妨碍原已授权业务，已尝试业务后不得因遥测失败、125/127 或缺回执直调重发。来源按实际取得方式声明，收尾只记录实际结果。被委派的实现者和审查者无需采集；此要求不改变任务授权及队列放行流程，不回退旧 dlog。
- 这个项目的开发任务由主控（`saddle/main`）拆开，派给别的 agent 做。主控负责拆任务、写任务文件、审查、合并，不自己写功能代码。分派时按 corral-dispatch 技能做。
- 被委派的 agent（任务文件里写明了身份）照任务文件做，不再往下派。
- 需求单只写用户要的结果；验收照抄用户原话，不补验收点。主控觉得该加的，列出来问用户。
- agent 名字以 `saddle/dev-` 开头；任务文件放 `docs/任务/`；每个任务一个分支，worktree 放 `../saddle-worktrees/<分支>`，交叉审查用 detached worktree `../saddle-worktrees/review-<分支>`。
- 创建派发 agent 时，在公开 `corral start` 参数中注明职责：实现者加 `--label role=implementer`，独立审查者加 `--label role=reviewer`；保留既有 model/effort 标签。主控创建时用 `role=controller`。标签仅供显示，不改变职责分工、权限或派发动作；不改已有 agent，不根据名字猜职责。
- 审查：主控审查每个任务。
- 合并：审查通过后本地合并进 main，然后推送到 origin。
- 收尾记号：一件活合并完、worktree 和分支清干净之后，在 main 上补一条空提交（`git commit --allow-empty`），首行写「收尾: 」加一句话说明这件活是什么。只记真正落地的活；说好不合并、停在审查的不记。
- 收尾之后更新 `HANDOFF.md`：现在在哪、下一步干什么、有什么悬着。设计和理由进 `docs/DESIGN.md`，别写进交接文件。
- 开出来的 agent：清掉某个 worktree 时，把住在里面的那个一并关掉（它的工作目录没了，接不了新活）；其余的用户说关才关。
