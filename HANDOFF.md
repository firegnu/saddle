# Saddle 交接

更新：2026-10-05，会话退出前。主控 `saddle/main`，分支 `main`。本次会话已将 T49 转交 cairn、T76 转交 paddock，完成 Saddle 侧记录与原实现者关闭；用户要求交接、提交并推送后退出，不启动新任务。本次交接前 main 为 `114206e`，工作区干净，没有待提交的产品代码。

## 当前接续点

- T49 自动记忆系统已由用户决定转入独立仓库 `/Users/firegnu/Developer/personal_projs/cairn`。转交时的初始化提交 `e463ac4` 只有文档；现在已有独立主控 `cairn/main`，并有自己的开发 agent，不再是等待建立主控的状态。当前设计以该仓库 `docs/DESIGN.md` 和 `docs/背景与决策记录.md` 为准，Saddle 不继续实施，也不干扰其开发。
- Saddle 中的 T49 方案仅作讨论历史：“同仓并随 Saddle 发布”和“文件主存储”已被 cairn 的独立 repo、不随 Saddle 打包、SQLite 主存储决定取代。取代说明已追加到 `docs/任务/T49-自动记忆边界共识.md`。
- T76 原型提交 `2c59e17` 已落盘；已转入独立仓库 `/Users/firegnu/Developer/personal_projs/paddock`，转交时初始化提交为 `261737c`、没有远程。现在已有独立主控 `paddock/main`。本次未重新检查该项目后续提交、远程或体验结果；以其 `AGENTS.md`、`HANDOFF.md`、`docs/DESIGN.md`、`docs/背景与决策记录.md` 为准。
- paddock 单向按 Git 提交 `df1c727` 引用 Saddle 公开库，自管依赖、锁文件、编译目录和工具链；Saddle main 不引入 GPUI。完整边界已记入 `docs/DESIGN.md` 的 T76 转出条目。

## 本轮完成

- `t49-memory-design` 通过快进合入 main，转交说明提交 `dab4da3` 已推送到 origin/main。
- 保留了 cairn 引用的原始提交：`5f602e8`、`41307eb`、`fc44fe0`、`3afc47c`；没有 squash 或 rebase。
- 已清理 `t49-memory-design` worktree 和本地分支；main 空收尾提交为 `715e225`，交接提交 `a81f710` 已推送。
- 用户随后授权主控关闭 `saddle/dev-t49-memory-1`（instance `c732ebf96a33`），已关闭并确认不再运行。
- T76 转出边界与交接提交 `114206e` 已推送；GPUI 代码和依赖未进入 main，仅新增文档记录。T76 队列说明已按用户选择更新，详情见下文。
- 验证限定为文档差异、引用目标、提交祖先关系、工作区和远端核对；没有运行 Cargo 或真实 hooks 实验。

## 未完成与 agent 边界

- 退出前公开 `corral ls` 中，Saddle 只剩当前主控 `saddle/main`（instance `61ebba2b93b0`）。`cairn/main`、`cairn/dev-fixpath`、`paddock/main` 属于独立项目，不关闭、不送话、不重新派发其工作；具体运行状态以后用公开命令再核对。
- T76 实现者 `saddle/dev-claude-1`（instance `1e0e83ee0a3e`）已在交接后按用户授权关闭，公开 status 确认不再运行。不要再向旧实例派发。
- 原型范围仍为一个 GPUI 窗口、一个真实终端窗格。实现者报告 31 项自动测试、clippy、release 构建及显示/生命周期实测通过；本主控本轮只核对记录与改动范围，未重跑构建或交互验收。
- 转交时键盘直接输入、真实中文输入法、鼠标、滚动、拖动缩放及与 Zed 并排体验未实际操作，后续由用户与 paddock 主控处理，本次未核验后续进展。迁仓不是体验验收，Saddle 新接口须另行授权。
- 此前多次完成提醒提前到达，不再处理旧回复或重复派发。`saddle/test-t76-claude` 和作者启动的原型进程据其交接已停止；没有复制截图入库。
- T49 旧研究 `../saddle-worktrees/t49-handoff-study` 仍保留（`8a08f6e`），未包含在这次授权的分支清理内；其原 Codex `saddle/dev-t49-1` 已按用户要求关闭。
- T76 研究 `../saddle-worktrees/t76-gpui-research`（`ea067ea`）及原型 `../saddle-worktrees/t76-gpui-prototype`（`2c59e17`）的分支和 worktree 均保留，明确不合并进 Saddle main；待用户确认 paddock 可用后再决定清理或保留。
- 其他历史 worktree 保留：`corral-live-upgrade-research`、`review-telemetry-design`（detached）、`t38-dispatch-study`、`t55-notification-flow`，不擅自删除。

## Tasks 与既有验证限制

- 用户选择保留 T76 Pending，并注明已转 Paddock、不要再派发。转出时已通过公开 `saddle ctl plugin` edit 更新标题为 `[已转 Paddock，勿派发] 评估用 GPUI 将 Saddle 演进为独立桌面应用`，正文补充转出说明并保留原登记内容；当时回读核对标题、完整正文及 Pending 均一致，位置为 5。此前 dispatch 明确 rejected，没有队列 run；没有 Submit/Accept/Drop 或重发。退出交接未再修改队列，后续操作必须重新读取状态和令牌。
- T49 的队列状态未在转交中调整。即使队列里仍显示它，也不要据此在 Saddle 重新实施或重复派发：该产品已转由 cairn 主控接手。其他 Pending 不因本次交接自动触发。
- T77 此前已合并、部署和收尾，旧 trace 已结束，不复用。旧交接中的安装/进程现场是 2026-10-04 的核验，本轮没有重新核验运行版本，也没有安装或重启。
- 既有测试限制继续保留：旧交接记载宿主 `tests/app.rs` 的三项旧头部位置断言失败，以及 `tests/drover_telemetry.rs::without_a_record_context_the_delivery_goes_the_plain_way_once` 的 disabled/budget_exhausted 差异。未在本轮复跑或修复，不宣称全仓全绿。详情见 `docs/任务/T77-遥测展示整理主控审查.md`。
- 按 `docs/UI回归.md` 选择验证；Cargo 共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。文档收尾不触发产品部署。

## 优先阅读与下一步

1. `docs/任务/T49-自动记忆边界共识.md` 末尾：cairn 转交说明；历史方案在 `docs/调研/T49-自动记忆产品边界与方案.md`，不要按其中已被取代的建议实施。
2. cairn 的 `docs/DESIGN.md`、`docs/背景与决策记录.md`：自动记忆当前依据，由该项目自己的主控接手；本主控不修改该仓库。
3. paddock 的 `HANDOFF.md`、`prototypes/gpui-terminal/README.md`：用户体验入口及未测边界。当前主控不在 Saddle 继续实现桌面前端；接口候选见该仓库 DESIGN 和原型 README。
4. `AGENTS.md`、`docs/UI回归.md`、`plugins/drover/README.md`：分派、验证和公开任务接口约束。

交接提交推送后停止，等待用户下一次指令。没有正在等待本主控推进的实现任务；不自动实施 cairn/paddock，不清理待用户确认的 GPUI worktree，不修复历史测试或新增 Saddle 接口。
