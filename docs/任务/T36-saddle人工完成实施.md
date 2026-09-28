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
