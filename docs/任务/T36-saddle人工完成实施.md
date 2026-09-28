# T36：saddle 人工确认完成入口

2026-09-28，saddle/main 交给 saddle/dev-t36-manual-complete-1（Claude Code，opus[1m] / high）。
路由：常规 / 交叉审查要 / 影响面：碰要害（路由三项 verdict 均拿不准；界面复用现有 Tasks，按常规实现；确认目标／异步结果错绑会完成错误任务，因此安排独立审查）。你是被委派实现者，只做本任务，不再派发。

## 先读与工作位置

- AGENTS.md、DESIGN 第 49 节，现有 Tasks 弹层／公开 CLI 适配直接相关代码。
- 分支 `t36-manual-complete`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t36-manual-complete`（主控建立）。只在这里改 saddle 的相关界面、适配、检查与文档，不改上游或技能。
- 已审查 Drover 固定交付 `0e4d031d3a570ef00e66b4df2f1e97b191f6607f`。必须读取其公开契约 `/Users/firegnu/Developer/personal_projs/drover-worktrees/m36-manual-complete/docs/人工完成JSON接口.md`；不读取其内部状态文件、不调用私有函数、不重做它的审查。

## 要做的

保留既有自动检查、Check & release、普通 go／next 行为，新增已确认的人工完成入口。

- Tasks 当前进行中任务详情的操作区加次要按钮 `Mark complete manually…`，无新单键快捷键。复用现有弹层风格：项目和 T 编号／标题、公开检查结果、一行 Reason、Cancel／Mark complete；说明人工确认后仍等待用户放行，不处理 Git 分支或停止 agent。
- 入口绑定当前具体运行。用该项目 `show Tn --json` 的 `manual_completion.target_token` 原样回传，不能组装或解析。项目／选中任务变化、关闭后旧查询返回、目标过期不能变成确认另一任务；原因留为草稿，取消不写。
- 确认调用同一项目的 `complete-manually Tn --target-token TOKEN --reason REASON --json`。非空原因，成功 JSON 为 schema_version=1、ok=true、task_id 对应目标、state=awaiting_release；固定进入待放行，不发 go／next、不更改 loop／pause。原检查不满足不是本界面替用户决定“通过”的依据。
- 展示失败检查、未运行状态和 last_check 的留存样本性质，不自动执行 CHECK_CMD。成功后展示人工完成的原因／时间／快照，来自 `task.completion_record`；不能与现有 show.completion 当前作用域混淆，旧历史无字段不倒填自动通过。
- 失败按公开 code 如实反馈（含 target_changed、state_busy、旧版本缺字段或命令不支持），不得失败后回退调用 go。过期目标须刷新并由用户重新确认；不自动替换令牌后重试写入。保留已有终端输入隔离、Tasks 确认交互和异步结果归属。

以上是 DESIGN 第 49 节与固定公开契约的实施摘要，不新增任务类型、消息中心、分支归属或通用事务框架。具体局部实现自行遵循现有代码，契约缺口报 saddle/main，不自己改 Drover。

## 用户依据

用户确认人工完成方案：「可以的」。用户指出任务已正式下发：「这个任务已经处于下方状态了」。用户强调：「这个功能我理解就是增加而并不是修改现有的行为吧？」原流程保留、新增人工完成是已确认范围。

## 验证预算与隔离

直接相关自动检查，关注确认目标和取消／失败不写的行为；项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，`git diff --check`。共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。使用假 CLI、临时 HOME／XDG／runtime、合成任务，不用真实 agent 或 socket。若检查失败，记录并针对该失败处理，不自行重跑整套、做基线统计或修 T29；预算不足在完成回复说明，不自行扩大。

两边真实公开 CLI 的一次隔离主流程联调由 saddle/main 组织，本轮不提前运行；交付提供现有相关检查入口即可，不搭建额外验证平台。联调固定使用上述 Drover worktree 命令，独立 HOME／XDG／TMPDIR、合成项目、假 corral／禁用 OS 发送器，不能用已安装主分支命令或真实项目写状态。

## 做完

在本文件追加简短完成记录：改动、实际检查结果、取舍与未解决项，提交本分支并回固定 SHA。只提交本任务；不合并／推送／发布、不清 worktree，不操作真实队列／配置／服务，不向其他用户 agent 送话或停止它们。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（saddle/dev-t36-manual-complete-1，2026-09-28）

改动：
- `src/drover.rs`：show 解析可选 `manual_completion`、`task.completion_record`（保存的检查／last_check／workspace 解析不了时单项略去，不猜）；新增 `Operation::CompleteManually` 和 `ManualError`。执行前核对操作项目与 worker 项目（按真实路径比较），只接受退出 0 + schema_version=1 + ok=true + 同 task_id + state=awaiting_release；失败按公开 code 报告，非 JSON 报“不支持”，不回退 go/next。
- `src/queue.rs`：Current 且有编号时底部出现 `Mark complete manually…`（只能点击，无快捷键）；确认子页 `Page::Manual` 显示项目、T 编号／标题、说明、show 报告的检查与 Last check（标明未运行／留存样本）、一行 Reason，`Mark complete ↵`／`Refresh ^r`／`Cancel Esc`。每次打开或 Refresh 用新序号读一次 show，旧序号结果丢弃；令牌原样回传。target_changed 后标为过期，须 Refresh 并再次确认；取消不写，原因按任务号留草稿；进行中不能取消或切换项目。成功后显示结果页并把视图切到 Run details。
- `src/app.rs`：确认页一次性 show 读取（Tasks 关闭即丢弃，重开重读）；确认页算输入框（q 不关闭），外部跳转视作未完成页。`src/ui.rs` 状态行提示。`src/detail.rs`：Run details 新增 `Manual completion` 段读 `task.completion_record`，与 recomputed checks 分开；人工完成的待放行提示注明非检查通过。
- DESIGN §49 追加“T36 saddle 实现取舍”。

检查（共享 CARGO_TARGET_DIR、假 drover 脚本和合成 JSON，未用真实 agent／socket／项目）：
- 新增：`tests/drover.rs` 2 项（show 字段可选与记录解析；令牌逐字、参数、公开 code、旧版本／不符答复、跨项目不发送）、`tests/queue.rs` 3 项（绑定打开的运行、旧读取丢弃、取消／拒绝不写、过期须刷新再确认、不可绑定目标）、`tests/ui.rs` 1 项（按钮只对运行中任务、点击打开、页面内容、小窗口不崩）。
- `cargo test --all-targets` 一次：`tests/viewer.rs` 的 `choosing_current_agent_again_during_detach_cancels_the_obsolete_switch` 失败（3 秒时限的 PTY 计时用例，本改动未碰 viewer），cargo 因此没跑 `workflow`。针对处理：单独跑该用例一次通过；补跑 `--test workflow` 一次为 75 passed／1 failed／3 ignored，失败是已知 T29 的 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal`（终端关闭确认，与 Tasks 无关），未修。其余各 target 全部通过。不能称整套全绿。
- `cargo clippy --all-targets -- -D warnings` 一次通过；`cargo fmt` 已格式化；`git diff --check` 通过。
- Clippy 之后修了一处：启动时面板项目是真实路径而 worker cwd 是配置路径，符号链接下会一律拒发。先加用例复现（RED：“The project changed”），改为按真实路径比较后，`--test drover`、`--test queue`、`--test ui manual` 通过；按预算没有重跑整套和 clippy。

取舍与未解决：
- 入口只能用鼠标（任务要求不加单键快捷键），没有另设键盘路径。
- 除 target_changed 外（如 state_busy）留在原页，由用户自己决定是否再按；从不自动重试或换令牌。
- 确认页不按列表 t0 另核对 show 的 started_at：令牌本身绑定运行，页面显示的检查与令牌来自同一次 show。
- 列表 `list --json` 的 completion_record 未接入（只在 Run details 读 show），Task text 视图不显示人工记录。
- 与真实 Drover CLI 的隔离联调按任务书留给 saddle/main。
