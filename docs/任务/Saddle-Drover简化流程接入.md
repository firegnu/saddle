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

## 完成记录（2026-09-30）

- 实现提交：`5fbe04f`（接入 Drover schema 2 的显式任务流转），分支 `drover-schema2`；完成记录另以文档提交保存。工作树保持在本任务路径，未合并、推送、安装或发布。
- 读取：list/show 仅接受任务接口 schema 2 和成功响应；消费 task.actions、run_id、submission、previous_runs、return_history、completion_record 和 evidence。通知偏好仍为 schema 1。历史按公开列表倒序显示，有编号的 Pending 支持 show。
- 操作：复用 Tasks 确认子页，Running 提交 done、Awaiting 接受 go；二者均可退回，退回要求原因及工作已停止确认。保留 Dispatch selected 和 Pause/Resume；删除 Next、Loop、旧 Check & release 和人工覆盖完成入口。所有转换只发送一条绑定项目、任务、运行与画面目标令牌的命令，不连做，不自动重试。
- 结果与归属：记录成功须 task_id/run_id/state/record.status 匹配。派发送达和记账分开报告，响应矛盾、失败或未知不宣称成功。失败后 Refresh 重新确认，退回重新勾选工作停止；详情按项目/编号/打开代次/运行归属，分组或 run_id 变化丢弃旧读取。
- 展示与通知：Git/检查只作仓库参考，陈旧检查同时保留原失败结果；旧完成、人工覆盖、真实接受及此前运行的交付/退回字段如实展示，缺失 t2 不补验收。通知直接使用公开 notification_key，保留启动基线、偏好切换、去重和 Attention 行为。

### RED → GREEN 与检查

全部通过前台命令等待完成，仅使用假 CLI、合成数据和现有测试设施；未调用真实任务写操作。

- `cargo test --test schema2 -- --nocapture` 首次 RED：合法 v2 列表缺少旧 mode，被旧解码拒绝；改用 v2 校验后 GREEN。
- `cargo test --test schema2 v2_show -- --nocapture` RED：旧 show 拒绝 schema_version 2；替换任务与证据模型后 GREEN。
- `cargo test --test schema2 retired_queue -- --nocapture` RED：旧 g 仍发出无目标 go；退役旧快捷键后 GREEN。
- `cargo test --test schema2 notification_identity -- --nocapture` RED：没有 Git 端点的 Awaiting 即便有公开 key 也不通知；改为公开 key 后 GREEN。
- `cargo test --test schema2 inconsistent_dispatch -- --nocapture` RED：status=confirmed 但 confirmed=false 时报告仍写 Delivered；收紧报告后 GREEN。
- 定向回归：Drover 13、Queue 20、Notify 5、schema2 8、UI 46 项通过；另已运行 Links 7、Dispatch 4 项通过。schema2 检查覆盖四种合法转换、原因/停止确认、令牌原样传递、过期重读、任务/运行结果核对、旧异步读取丢弃、无重试与无串联。
- 标准检查各执行一次：
  - `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test --all-targets`：退出 0，**293 passed / 0 failed / 3 ignored**；包含 workflow 82 项通过。完整日志 `/tmp/saddle-schema2-all-targets.log`。
  - `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo clippy --all-targets -- -D warnings`：退出 0。日志 `/tmp/saddle-schema2-clippy.log`。
  - `git diff --check`：通过。
- 原始失败未隐去：早期定向 UI/workflow 运行中，4 项旧界面预期随契约迁移修正；另 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases`、`terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal` 出现失败，未修改这些测试或终端逻辑，随后要求的全量运行均通过。原始日志 `/tmp/saddle-schema2-rendered.log`；未顺带修 T29。

### 取舍与未执行项

- 详细取舍已记入 `docs/DESIGN.md` §61。公开 v2 契约未提供历史 Git 结束端点，Links 不补造历史区间；Running 仅在有效仓库参考与端点齐全时提供 start..HEAD。旧记录保留原始事实，不把历史覆盖解释成检查通过。
- 没有必须扩展 Drover/Corral 的契约缺口；未改它们的代码、技能、真实配置或状态，未操作 T57/T55/T38，未触碰真实服务/用户 agent，未清理任何 worktree。
- 3 个原有 opt-in CLI 联调测试保持忽略；本轮未真实联调、录屏或发布。实现停在待主控审查，后续隔离联合主路径验证与统一切换由主控另行安排。
