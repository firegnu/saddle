# Saddle 交接

更新：2026-10-05。主控 `saddle/main`，分支 `main`。本轮仅收尾 T49 讨论文档并转交独立项目 cairn，没有产品代码修改、构建、部署或 Tasks 状态变更。

## 当前接续点

- T49 自动记忆系统已由用户决定转入独立仓库 `/Users/firegnu/Developer/personal_projs/cairn`。初始化提交 `e463ac4` 只有文档；用户会在那里另开主控。当前设计以该仓库 `docs/DESIGN.md` 和 `docs/背景与决策记录.md` 为准，Saddle 不继续实施这项系统。
- Saddle 中的 T49 方案仅作讨论历史：“同仓并随 Saddle 发布”和“文件主存储”已被 cairn 的独立 repo、不随 Saddle 打包、SQLite 主存储决定取代。取代说明已追加到 `docs/任务/T49-自动记忆边界共识.md`。
- T76 GPUI 终端原型仍是独立工作，尚未由主控验收。本轮只读核对时，`t76-gpui-prototype` 仍有未提交的任务书和 `prototypes/`，没有正式原型交付提交。不得把之前的研究回复当原型完成。

## 本轮完成

- `t49-memory-design` 通过快进合入 main，转交说明提交 `dab4da3` 已推送到 origin/main。
- 保留了 cairn 引用的原始提交：`5f602e8`、`41307eb`、`fc44fe0`、`3afc47c`；没有 squash 或 rebase。
- 已清理 `t49-memory-design` worktree 和本地分支；main 空收尾提交为 `715e225`。本交接文档随后提交、推送，最终状态以 Git 为准。
- 用户明确安排 `saddle/dev-t49-memory-1`（instance `c732ebf96a33`）自行关闭，主控没有停止它。它原来的工作目录已删除，不能再向该会话派发 Saddle 任务。
- 验证限定为文档差异、引用目标、提交祖先关系、工作区和远端核对；没有运行 Cargo 或真实 hooks 实验。

## 未完成与 agent 边界

- T76 实现者：`saddle/dev-claude-1`，instance `1e0e83ee0a3e`。启动 cwd 是 Saddle 主仓库，但任务要求所有开发操作显式指向 `../saddle-worktrees/t76-gpui-prototype`。
- 原型范围：一个 GPUI 窗口、一个真实终端窗格，验证现有 PTY/终端内核接入和终端体验。用户明确要求独立 worktree、不污染主分支；不合并推送、不安装覆盖现用 Saddle、不将原型或研究结束当成 T76 全部交付。
- 此前多次 after 提醒提前到达；现场曾为 working 或 blocked，reply 仍是旧研究 `ea067ea`。最后一次遇到 blocked 后已告知用户处理提示，未再续挂同类提醒。接手时先用公开 status/reply 核对实例、回复时间和新提交，正式回复/提交/完成记录齐全且停工后再审查；不要重复派发，不代答权限框，不干扰用户 attach。
- T49 旧研究 `../saddle-worktrees/t49-handoff-study` 仍保留（`8a08f6e`），未包含在这次授权的分支清理内；其原 Codex `saddle/dev-t49-1` 已按用户要求关闭。
- T76 研究 `../saddle-worktrees/t76-gpui-research` 保留，最新研究提交 `ea067ea`，原型以它为基线。
- 其他历史 worktree 保留：`corral-live-upgrade-research`、`review-telemetry-design`（detached）、`t38-dispatch-study`、`t55-notification-flow`，不擅自删除。

## Tasks 与既有验证限制

- 本轮没有查询或修改 Tasks 状态。此前 T49、T76 为 Pending；T76 向忙碌主控的 dispatch 明确 rejected，没有队列 run。不要把实际 agent 委派或文档归档算成 Tasks 已完成。
- T77 此前已合并、部署和收尾，旧 trace 已结束，不复用。旧交接中的安装/进程现场是 2026-10-04 的核验，本轮没有重新核验运行版本，也没有安装或重启。
- 既有测试限制继续保留：旧交接记载宿主 `tests/app.rs` 的三项旧头部位置断言失败，以及 `tests/drover_telemetry.rs::without_a_record_context_the_delivery_goes_the_plain_way_once` 的 disabled/budget_exhausted 差异。未在本轮复跑或修复，不宣称全仓全绿。详情见 `docs/任务/T77-遥测展示整理主控审查.md`。
- 按 `docs/UI回归.md` 选择验证；Cargo 共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。文档收尾不触发产品部署。

## 优先阅读与下一步

1. `docs/任务/T49-自动记忆边界共识.md` 末尾：cairn 转交说明；历史方案在 `docs/调研/T49-自动记忆产品边界与方案.md`，不要按其中已被取代的建议实施。
2. cairn 的 `docs/DESIGN.md`、`docs/背景与决策记录.md`：自动记忆当前依据，由该项目自己的主控接手；本主控不修改该仓库。
3. 原型 worktree 的 `docs/任务/T76-GPUI终端原型.md`、`prototypes/gpui-terminal/`：正式交付后审查改动范围、运行方式、中文输入法实测与未测边界，再向用户报告体验方式。
4. `AGENTS.md`、`docs/UI回归.md`、`plugins/drover/README.md`：分派、验证和公开任务接口约束。

T49 文档归档后等待用户进一步指令；不自动实施 cairn、不修改队列、不扩展 T76 范围。
