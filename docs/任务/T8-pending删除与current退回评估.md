# T8：pending 删除与 current 退回评估

2026-09-26，saddle/main 交给 saddle/dev-t8-delete（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规，交叉审查与影响面拿不准；本轮仅接入既有公开 pending 放弃命令，不改 drover 核心规则，主控审查）。
你是被委派的 agent：照本文件做，不再开别的 agent。

## 先读

- AGENTS.md；docs/DESIGN.md 第 3、5、20、21、22 节。
- src/drover.rs 的 Operation、execute；src/queue.rs 的 pending 操作、原生弹层和输入处理；src/ui.rs、src/buttons.rs 中相关入口和确认交互。
- tests/drover.rs、tests/queue.rs、tests/ui.rs、tests/workflow.rs 的 pending 编辑/排序检查。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t8-pending-delete，分支 t8-pending-delete，已从 main 建好。
- 范围：当前项目 Queue 中 pending 删除入口、公开 CLI 调用与相关反馈、必要测试和文档、本任务记录。
- 所有 cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 用户原话

> 1: 提供可以在看板中删除pending task的功能
> 2: current task如果没有在运行可以退回到pending中（不知道现在是不是已经有这个功能了，这个任务要评估一下爆炸半径大不大）

## 主控评估与本轮范围

- 用户最终决定（2026-09-26）：「那这个先不做了。只做删除」。T8 收窄为 pending 删除；current 退回 pending 的只读评估已结束，本次不实施，也不作为 T8 完成的阻挡项。不自动另开后续开发任务。
- 2026-09-26 本机 `drover --help` 已提供 `drover drop --pos <位置> <原因> [--expect V]`，用于放弃还没开始的任务，包括无编号任务；按公开语义从待办移除并保留放弃历史，不是抹掉审计记录。saddle 目前无此入口。第一项可在既有 pending 操作范围内接入，影响局限于 UI 与公开调用，现已授权实施。
- 本机公开帮助没有 current 退回 pending 的命令；`list --json` 的 current 只表明任务已发出，不能证明主控/开发 agent 此刻是否执行它，也没有任务到所有执行 agent 的完整公开关联。不能靠 UI 的 Running 文案、主控 idle 或 paused 直接断定安全退回。
- 第二项需 drover 定义新的状态转换及公开命令，核对历史/start 基线、重复分派、循环/放行与正在结束的任务之间的关系。推断它涉及队列核心语义，影响明显大于 saddle 加按钮；具体范围应由 drover 主控评估。本轮只记录此结论，不实现退回、不把 drop+add 当成等价替代，不扩大跨仓库权限。主控已询问用户是否转交 drover/main 进一步评估。

## 本轮要做的

- 在现有 Queue 的 pending 操作中增加删除入口，沿用英文轻量按钮、主题及原生确认交互；能明确辨认将移除的任务，并能取消。按钮位置和快捷键由你按现有布局简单决定，先写 DESIGN 再实现。
- 只操作选中的 pending，通过公开 drover 命令按其已有语义执行；沿用后台请求、忙碌/错误保护、结果反馈及刷新。不让刷新或选中项变化偷偷更换已确认的目标。
- 沿用 DESIGN 第 20 节的 pending 基线检查；公开 list 无可用于 --expect 的指纹，不能读内部文件补齐，也不声称预检查与写入原子化。若实现发现比该已记录限制更大的接口缺口，报告具体情况。
- 同步必要的中英文 README 和帮助说明，让用户知道删除是移出待办并保留 Dropped 历史。T6 的 All pending 汇总继续只读。

## 怎么算做完

本轮实现验收原话：

> 1: 提供可以在看板中删除pending task的功能

第二项只读评估已结束，用户决定本次不做；T8 按第一项删除验收及收尾，不宣称第二项已实现。

验证预算：按轻量 TDD 先取得目标行为的真实 RED，再最小实现到 GREEN；针对性检查及项目标准 cargo test --all-targets、cargo clippy --all-targets -- -D warnings 各一次，另做 git diff --check。使用假 CLI、临时目录和合成任务，不操作真实队列，不做录屏、覆盖矩阵或缺陷注入；失败可运行修复所需的相关检查。

## 不要做

- 不删除真实任务、不修改真实配置/登记或用户 agent；不读取 drover 内部文件，不修改 corral/drover 仓库。
- 不实现 current 退回、不改任务状态机、自动发任务或做跨项目删除；不开始 T4、T2。
- 不新增复杂架构，不修无关问题；不按项目名或路径批量杀进程，自己起的进程只按记录 PID 停止。
- 不合并 main、不推送、不更新 HANDOFF，只在 t8-pending-delete 分支提交。

## 做完

在本文件末尾追加「## 第一项完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复同样列这些和需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
