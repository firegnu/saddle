# T16：pending 删除与列表编辑修复

2026-09-27，saddle/main 交给 saddle/dev-t16-pending（Codex，常规档：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由三项均拿不准；主控按已有交互故障修复定常规，沿用现有公开 CLI 和写入校验，不涉及模型或并发协议重构）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 20 节（pending 编辑）、第 24 节（删除 pending）、第 26 节及后续 pending 详情/任务正文入口补充。
- src/queue.rs、src/app.rs、src/ui.rs 中相关输入和命中路径；tests/queue.rs、tests/workflow.rs 中相关现有检查。
- 若存在 CONTEXT.md 或相关 ADR，也先阅读。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t16-pending-actions，分支 t16-pending-actions（从 main 建好）。
- 范围：pending 删除及从列表直接 Edit 的必要 queue/app/UI 交互修复、对应测试、本任务完成记录。若需要改变已批准的设计，先报告，不自行选交互方案；仅修复既定行为不需另改设计。

## 要做的 / 怎么算做完

用户原话：
> 1: 如果我发现这个task不再需要了，我没法从pending状态remove掉这个task
> 2: pending状态下如果不进入这个任务的详情状态，直接在任务列表edit这个task，task无法编辑

先定位并复现两个现象，再做最小修复。主控尚未确认根因，不能凭推测修改。主控已运行 `cargo test --test workflow pending_ -- --test-threads=1`，4 passed；既有 pending 编辑测试通过 Tab 切到正文，删除测试通过键盘选中，尚未覆盖用户报告的完整点击路径。主控已向用户询问删除入口是否缺失及编辑具体卡点；收到补充会转交。

验证预算：两个现象各自的定向自动回归，修复前确认因目标缺陷 RED、修复后 GREEN；项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，及 `git diff --check`。全部 Cargo 命令设置 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。只使用假 CLI、合成任务。不能复现则报告已尝试路径与缺少的信息，不写猜测性修复。

## 不要做
- 不改右侧 agent 按钮样式/高度，不做 T17 或其他任务。
- 不重构通用表单、终端生命周期、队列存储；不改删除后 Dropped 历史规则，不削弱现有确认与过期快照检查。
- 不读 drover/corral 内部文件，不改其仓库，不操作真实任务或用户 agent，不批量杀进程。
- 不合并 main、不推送、不构建 release；只在本分支提交。
- 范围外失败只记录，不扩修：T15 主控已在基线确认 `full_workflow_routes_input_switches_safely_and_survives_disappearance` 失败，本任务不要为此扩大范围或弱化断言。

## 做完
在本文件末尾追加「## 完成记录」并提交：根因、改动、RED/GREEN 与标准检查结果、取舍和未做的事。回复附提交 SHA、待主控决定事项；命令都在前台跑完，全部做完后，回复最后一行写 DONE。
