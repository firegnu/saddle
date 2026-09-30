# Saddle 接入 Drover 简化任务流转

2026-09-30，saddle/main 交给 saddle/dev-drover-schema2（Codex，gpt-6-astra / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由三项均拿不准；按已定公开契约跨模块接入，主控审查，状态规则与存储由 Drover 承担）。
类型：功能变更
依据：已授权 Drover 简化流程，现接入 Saddle；不发布、不推进真实 T57。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/任务/Drover-任务流转简化重构.md 的已确认行为、明确不做；同名主控审查文档的返工复审。
- docs/DESIGN.md §61（本次已定行为；替代涉及旧流程的条款）。
- /Users/firegnu/Developer/personal_projs/drover-worktrees/task-flow-simplification/docs/任务流转JSON接口.md：公开契约，提交 cb67fe8。本次仅授权读取此交付契约；不要读真实 Drover 内部数据、不要改 Drover 仓库。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/drover-schema2，分支 drover-schema2。
- 范围为 src/drover.rs、queue/detail、notify/attention 及直接调用点、对应测试和必要设计文档。不要改终端输入、pane、新建 agent 或其他任务。
- Drover 分支保留供后续联调；不清理它，也不碰 T55/T38。

## 要做的
- 接入 schema_version=2 的 list/show/task.actions、run_id、submission/previous_runs/return_history 与 evidence；不再消费旧门槛字段。未知版本或失败响应明确报错，不能默认为空队列或成功。通知偏好接口仍是 schema 1。
- 复用现有 Tasks 布局。对选定 Running 提供 Submit for review（done），对选定 Awaiting 提供 Accept（go）；都用画面上确认的 task id 和不透明 target_token。移除旧 Check & release 连做、Loop、gate、人工覆盖完成入口。保留明确的 Dispatch selected；旧 Next 入口退役，不暗中转发下一项。
- Running 和 Awaiting 都可 Return to pending，保留原因及工作已停止的确认；不再声称撤回会暂停队列。Pause/Resume 仅控制显式派发。
- 结果按新契约区分记录成功、派发送达和不确定结果；保留项目/任务/运行与异步结果归属保护，目标过期刷新后重新确认，不自动重试，不串联 done/go/派发。
- 详情中的 Git/检查仅作仓库参考，失败、未知、陈旧如实显示；不把它们当任务完成门槛。旧完成、真实接受、旧人工覆盖与退回记录保留事实，不补造接受或测试通过。
- 通知消费公开 notification_key，沿用既有启动基线、偏好与去重；Awaiting 进入 Attention，通知不改变状态。不自行从 Git 字段推导新通知身份。

## 怎么算做完
用户验收原话：
> 各个task不能相互影响各自的任务扭转状态。而且我认为现在的很多问题都是类似调研之类的任务，不需要分支合并的。
> 保持简单可靠，没有必要套各种死板流程

验证预算：目标行为先 RED 后 GREEN，按 AGENTS.md；使用假 CLI / 合成数据。标准 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test --all-targets` 与同前缀 `cargo clippy --all-targets -- -D warnings` 各一次。无关偶发失败可单独复跑一次，保留原始结果，不顺带修 T29。不要录屏、真实 agent 测试、扩建框架或额外覆盖矩阵。后续主控再安排隔离联合主路径验证，本轮不要真实联调或安装发布。

## 不要做
- 不做 Drover 插件化、并发任务、自动推进/完成检测/提醒、新页面或重设计布局。
- 不改 Corral、corral-dispatch 或 Drover 代码；若契约存在必需缺口，报告具体调用点，不能自行扩展。
- 不改真实任务/队列/配置，不执行真实 done/go，不启动/停止/重启 Saddle、服务或用户 agent，不改全局命令入口。
- 不合并 main、不推送、不发布、不清理 worktree。不要按项目名或路径批量杀进程。

## 做完
在本文件追加完成记录：提交、改动、检查结果、取舍与未完成项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
