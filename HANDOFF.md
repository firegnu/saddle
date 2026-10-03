# Saddle 交接

更新：2026-10-03。Settings 已安装更新提示已实现、发布编译并安装。用户在获知 10 项测试失败仍未解决后明确要求「那就收尾，handoff 之后提交+推送」；本轮据此合并收尾，保留失败记录，不宣称标准回归全绿。

## 完成状态

- 当前分支 `main`。功能提交 `a542931` 已快进合并；本任务 `update-indicator` 分支及 worktree 已删除，收尾空提交 `74c93fc`。本轮没有委派；清理前公开列表仅有工作目录在主仓库的 `saddle/main`，未停止任何 agent。
- 已实现 Settings 小点、Updates F6 页面、每 30 秒只读检测已安装程序、用户触发的 Upgrade all、逐项 pen/after 回执与后续公开核验。unknown 不自动重发，页面重开及重复点击不重复升级。正式设计见 `docs/DESIGN.md`。
- Release 构建成功，`~/.local/bin/saddle` 指向 `~/.local/share/saddle/versions/a542931/bin/saddle`；已安装入口、SHA256 与 `--help` 核对通过。
- 收尾时 `lsof` 确认 Saddle PID `15011` 已加载 `a542931/bin/saddle`。这是本轮快照；无需因这次安装再次重开。只读运行版本核对不代表功能端到端验收。
- 只重编译了 Saddle；Corral、Drover、Diff 二进制复用 `711ab18`，Corral 命令入口、插件注册及配置保持原样。没有升级、恢复或重启任何用户 agent。
- 本文、部署记录及任务收尾记录随本轮文档提交推送。最终提交号、远端一致性和工作区是否干净，以 `git log -3`、`git status -sb`、`git ls-remote origin refs/heads/main` 回读为准。

## 验证与未解决事项

- 最终直接回归 71 passed（Updates 10、Settings 24、telemetry_settings 6、UI 31）。新增页签使 General 说明被挤掉的回归已修复。宿主 all-targets clippy，以及最后布局修正后的 lib clippy 通过。
- 首次标准 test 在 agent_capture 中断：109 passed、5 failed、4 ignored；限定串行复跑为 23 passed、1 failed。其余宿主补跑 401 passed、12 failed、5 ignored，其中 1 项页面回归已修；drover_telemetry 限定串行复跑为 5 passed、9 failed。其他 workspace 包补跑 128 passed；workflow 101 passed、4 ignored。首次 clippy 的本轮布尔表达式警告已修。不能将这些结果合称一次标准全套绿。
- **仍有 10 项失败**：agent_capture 的 `timeout_reaps_only_the_direct_client_and_reports_execution_as_unknown` 在 500ms 内未获得预期部分 stdout；drover_telemetry 9 项多为 `budget_exhausted/no_context` 或缺 trace 的断言。根因及是否既有问题未确认，没有修改预算或放宽断言。原始日志 `/tmp/saddle-update-indicator-*-20261003.log`，详细证据见实施记录；既定复跑预算已用完，本次收尾未新增复跑。
- 用户要求不做端到端测试，功能实际使用验证留待用户使用时进行。本轮未操作真实 Tasks/遥测、未执行真实 Upgrade all。

## 重要文件

- `docs/任务/已安装更新提示-实施.md`：实现、RED/GREEN、标准失败及后续收尾记录。
- `docs/任务/已安装更新提示-部署记录.md`：构建、四程序哈希、入口及私有备份位置。
- `docs/DESIGN.md`、`docs/Corral通用升级设计.md`：已批准设计及公开升级协议边界。
- `docs/任务/Corral通用升级-部署记录.md`：旧成套版本 `711ab18` 的部署历史；保留旧包，现存 pen/hook 可能仍引用。

## 保留工作区与边界

- 历史 worktree `corral-live-upgrade-research`、`review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow` 保留。本轮未改动；其中 `review-telemetry-design` 有 6 个既有未提交文件，不属于本任务，不清理或提交。
- 构建继续共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。固定安装包使用不可变版本目录，不能把共享构建输出或符号链接当作存活进程已切版的证据。
- 本轮用户明确要求直接实施、不委派；没有新建或等待独立审查 agent。真实 agent 继续遵守公开接口与不干扰约束。

## 下一步

本轮提交推送并回读后停止，等待用户下一项指令。不自动派发队列任务、不扩大测试或修复范围；10 项失败保留为后续待处理问题，用户实际使用时再验证新功能。
