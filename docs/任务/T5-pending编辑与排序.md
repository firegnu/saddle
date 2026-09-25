# T5：pending 任务编辑与调整次序

2026-09-26，saddle/main 交给 saddle/dev-t5-pending（Codex，常规：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由三项均拿不准；现有表单和公开 CLI 的接入，不设计存储或改造并发机制，按改行为预算；项目由主控审查）。
你是被委派的 agent：照本文件做，不再开别的 agent。

## 先读

- AGENTS.md；docs/DESIGN.md 第 5、8、13、20、21 节中相关界面/公开 CLI 约定。
- src/queue.rs、src/drover.rs、src/app.rs 的 Queue 请求和刷新处理；src/input.rs、src/buttons.rs 的既有输入/按钮模式。
- tests/queue.rs、tests/drover.rs、tests/workflow.rs 及假 drover。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t5-pending-edit，分支 t5-pending-edit，已从 main 建好。
- 范围：pending 编辑/排序的 UI、公开命令接入、相关输入/选择/刷新逻辑、针对性测试和使用/设计文档，以及本任务记录。
- 所有 cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 要做的

用户原话：

> 任务无法修改，应该是pending状态下是可以修改的
>
> 补充：pending 状态的任务目前无法调整次序，应支持调整待办任务的顺序。

- 沿用既有原生轻量按钮及表单。选中的 pending 任务增加 Edit、Move up、Move down 操作；编辑表单预填标题和正文，沿用 Save/Cancel、Tab、Ctrl-S 和多行正文模式。排序用相邻上移/下移即可，不加拖拽或复杂排序界面；首末项对应方向不可用。键位选不与现有动作冲突的简单键，并在按钮/帮助注明。
- Current、Awaiting、History 不可编辑或调序；选中项决定操作对象，位置按 pending 列表计算，不按混合列表索引。后台刷新保留编辑草稿；失败呈现实际错误并保留草稿；成功刷新并保持选中被操作任务，延续既有忙碌/错误状态约束。
- 数据和修改只走公开 CLI：`drover edit <待办位置> <标题> [说明行…] [--expect V]`、`drover move <原位置> <新位置> [--expect V]`。命令参数直接传递，不启动 shell。先更新 DESIGN 相关约定再实现。
- 主控已核对当前公开 `drover list --json` 仅返回 mode/paused/current/awaiting/pending/history，pending 有 id/title/body，未见队列版本字段；帮助里的 --expect 接收 queue.md 指纹，但不能直接读内部文件来计算。实现前核实公开接口能力。提交时至少通过公开快照核对原任务仍是 pending、内容和顺序没有发生冲突，检测到过期目标应提示并保留草稿，不能盲用弹层打开时的旧位置。能通过公开接口获得原子保护则使用；不能时在完成记录明确预检查与执行之间的接口限制，不声称消除了竞态，不擅自改 drover。若必须扩展跨仓库接口才能实现所需操作，停下来报告具体缺口。
- 更新 README 中英文操作说明及 --help 的相关新增入口；不顺带修无关遗留文案。

## 怎么算做完

> 任务无法修改，应该是pending状态下是可以修改的
>
> 补充：pending 状态的任务目前无法调整次序，应支持调整待办任务的顺序。

验证预算：按照轻量 TDD，先为两个实际缺失行为取得可运行的 RED，再最小实现到 GREEN；针对性检查覆盖选中 pending 的编辑和调序及其直接回归风险。项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各跑一次，另做 git diff --check。不加覆盖矩阵、录屏或缺陷注入；假 CLI/临时数据验证，不动真实队列或 agent。预算不足先说明。

## 不要做

- 不开始 T4 过程详情、T2 tab/split；不新增删除、放弃、持久化服务或其他任务操作。
- 不读取 corral/drover 内部文件，不改它们的仓库，不改用户真实配置、队列和服务，不操作真实 agent。
- 不按项目名或路径批量杀进程，只停止自己记录 PID 的进程。
- 不新增超出当前交互的架构或复杂配置；遇到必须改变公开接口的阻塞，报告后等主控决定。
- 不合并 main、不推送、不更新 HANDOFF；只在 t5-pending-edit 提交。

## 做完

在本文件末尾追加「## 完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事、公开接口限制，各几句话。回复同样列这些和需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-09-26，saddle/dev-t5-pending，在 t5-pending-edit 完成。

- 做了什么：先更新 DESIGN 第 5、20 节，再接入选中 pending 的 Edit e、Move up u、Move down d；编辑复用标题/多行正文表单、字段点击、Tab、Ctrl-S、Save/Cancel。刷新与失败保留草稿及原始基线；成功刷新后按任务身份和目标位置保持选择，包括无编号且内容相同的待办。非 pending 不显示新入口、快捷键不生效，首末方向禁用；保持忙碌/读取错误约束。更新中英文 README、Queue 帮助和 --help。
- RED→GREEN：`cargo test --test queue selected_pending_edit_prefills_and_saves_the_native_form` 在有效输入下因 Edit 未打开表单失败，随后通过；`cargo test --test queue selected_pending_moves_use_pending_positions_and_stop_at_the_ends` 因未产生 Move 请求失败，随后通过。另定位并修复相同无编号任务移动后选择回到首项的直接回归，针对性检查先失败后通过。首次编辑测试的缺失 mode 测试输入错误已修正，不计作 RED。
- 验证：所有 cargo 命令均使用约定的共享 CARGO_TARGET_DIR，全部前台跑完。Queue/Drover 针对性测试通过，覆盖混合列表中的待办位置、首末边界、刷新/失败/忙碌草稿、过期内容/顺序/目标状态拒绝写入、字面参数及多行正文。假 CLI 的真实 PTY 工作流验证按钮编辑、实际错误、重试、Ctrl-] 暂回与恢复、q 作为正文输入、双向移动及选择保持。
- 标准检查：按预算执行一次 `cargo test --all-targets`，在 UI 的历史底栏测试发现新增禁用按钮挤占一行；改为仅选中 pending 时显示新入口，未改旧断言。随后执行 `cargo test --test ui --test workflow --test viewer --example compare_parsers`，复验 UI 并补跑此前未执行的目标，全部通过；结合此前已通过目标，共 73 项通过、2 项原有 installed-drover 测试忽略。`cargo clippy --all-targets -- -D warnings` 一次通过；`cargo run -- --help` 核对入口，`git diff --check` 通过。
- 拿主意的地方：e 在任务列表编辑、在 Projects 页仍输入路径，u/d 只在列表调序，均不与既有动作冲突。编辑打开/移动触发时保存 pending 基线，worker 写入前再取公开快照，严格核对整份 pending 的 id/title/body/顺序；即使是其他待办改变也保守拒绝，提示重新打开编辑或刷新后重试，保留当前草稿。
- 公开接口限制：通过 `drover --help` 以及隔离 HOME、临时 git 项目中的公开 init/add/list/edit/move 核实了 1 起始待办位置、多行正文和编号保持；list JSON 只有 mode/paused/current/awaiting/pending/history，没有版本字段。子命令没有独立 --help，本 worktree 未接入 drover 时返回配置缺失错误。公开 --expect 需要 queue.md 指纹，未发现公开获取入口，因此未传伪造版本或读内部文件。预检查与执行之间仍有竞态窗口，不能保证原子保护；不修改 drover。
- 没做的事：未读取内部队列/配置文件，未操作用户真实队列、配置、服务或 agent；未分派、未做覆盖矩阵/录屏/缺陷注入，未扩展其他任务操作。未合并 main、推送或更新 HANDOFF，只在本任务分支提交。
- 需主控决定：无新增阻塞；按任务已允许的预检查方案交付。若以后要求消除写入竞态，需要另行授权 drover 提供公开版本/原子条件写入能力。
