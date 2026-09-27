# T18：Tasks 大弹窗

2026-09-27，saddle/main 交给 saddle/dev-t18-tasks（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档位拿不准，交叉审查不要，影响面改行为；主控按已确认的界面方案选常规）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 32 节（本次已批准方案），第 20、24、26 节的既有任务编辑、删除及详情行为。
- src/layout.rs、src/app.rs、src/ui.rs、src/queue.rs 相关布局和输入路径；对应 tests 下现有检查。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t18-tasks-popup，分支 t18-tasks-popup（从 main 建好）。
- 只动 Tasks 弹窗和 Agents 布局所必需的 src 文件、直接受影响的 tests、相关配置/说明文档及本任务完成记录。不要顺便改其他功能。

## 要做的
实施 DESIGN 第 32 节。用户已看过并同意左侧固定入口、大弹窗内左右分栏、列表与正文/运行详情并排、编辑在同一弹窗中切换的方案。延续已有视觉语言和公开 drover 操作；用户会体验后再提出调整。

## 怎么算做完
用户原话：
> 考虑把左下侧的tasks整体改为以弹出框的形式展示，现在太挤了。
> 所以说，如果tasks窗口从左侧以弹出框的形式弹出，涉及到两个问题1: agents区域就是独占左边。2: 弹出tasks需要有一个入口 3: tasks变大了，所以ui和ux可以按照稍大的空间进行重新设计
> 可以。先做出来。我再测试看看有什么问题再修改。

验证预算：与本次交互行为直接相关的定向自动检查，行为变化先确认旧实现因目标行为 RED，再最小实现 GREEN；项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次及 `git diff --check`。不做覆盖矩阵、缺陷植入、录屏或真实 agent 测试。Cargo 命令设置 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不实现已取消的 T16，不实现 T17，不改右侧 agent 顶部按钮的高度、宽度或样式。
- 不改任务状态/删除语义、公开 CLI 协议或 PTY 生命周期，不引入通用 UI 框架或新增配置项。
- 不读 drover/corral 内部数据，不改其他仓库，不操作真实任务或用户 agent；测试仅假 CLI/合成数据。
- 不按项目名或路径批量杀进程。
- 不合并到 main、不推送、不构建 release。只在本分支提交。
- 若需改变已批准方案，先报告主控，不能自行改成另一种交互。范围外失败如实记录，不擅自扩修。已知基线 `full_workflow_routes_input_switches_safely_and_survives_disappearance` 在 T15 修改前后均失败；仅当布局变更直接影响相关检查时按新几何调整，不掩盖无关失败。

## 做完
在本文件末尾追加「## 完成记录」并提交：改动、验证、实现取舍、未做的事。回复附提交 SHA、待主控决定事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-09-27，saddle/dev-t18-tasks。

### 改动
- 布局（`src/layout.rs`）：Agents 在任何宽度下独占左列（宽度规则不变），去掉窄窗 Agents/Queue 标签行和左下 Queue 格；新增 `Panes.tasks`：从左边缘、Agents 顶边框下一行展开，约 85% 宽高（不足时至少 64 列 / 20 行，不超过屏幕），不盖住底部状态行。
- 入口（`src/ui.rs`）：Agents 顶边框右侧固定紧凑按钮 `‹Tasks Tab›`，不随列表滚动；打开时以主色高亮且不可点。Agents 原有按钮行未动。
- 弹窗（`src/queue.rs`）：沿用现有弹框样式（`t.block` + overlay 底色）、紧凑按钮和英文文案。顶部两行：`Project ‹<项目> ▾ c›`、目录、右侧队列状态；`‹Next n› ‹Check & release g› ‹Pause p› ‹Loop l›`，`‹Refresh r›` 靠右。中部左约 1/3 为 Current/Awaiting/Pending/History 列表，右约 2/3 为选中任务，顶部 `‹Task text t› ‹Run details ↵›` 切换；两边独立滚动，单击任务只更新右侧。底部 `‹Add task a›`、选中 pending 时的 `Edit e / Move up u / Move down d / Delete x`、`All pending A`、`Help ?`，`‹Close Esc›` 靠右。
- 子视图：Help、Action result、Projects、Project path、All pending、Add/Edit、Delete 确认都在同一弹窗内替换列表+内容区（顶部项目行保留但禁用，分隔线标出子视图名）；Add/Edit 表单占满该区，正文获得主要空间；保存/取消回到原任务与原视图。去掉旧的 Detail/Task 页和居中 76×28 覆盖层。
- 输入（`src/app.rs`）：Tasks 打开即 `Focus::Queue`，键盘和鼠标只作用于弹窗，弹窗外点击忽略；Agents 的 Tab 或点击入口打开，记住打开前的输入目标；列表页 Esc / Close / `q`（文本字段外）关闭并回到该目标；Ctrl-] 按既有全局规则回 Agents。关闭不改页面，再打开保留项目、选中、视图、滚动和草稿。底层终端照常运行，不重建、不缩放。
- 运行详情：`drover show` 只在弹窗打开且显示 Run details 时按原规则每 5 秒查询；切回原文、换任务、关弹窗、换项目即停止。
- 配置与文档：`left_split` 仍解析校验但不再使用，`config.toml` 注释和中英文 README 已更新（布局图、功能、按键表、详情说明）。`examples/ui_preview.rs` 改用新布局 API。

### 验证
- RED：新增 `tests/ui.rs::agents_own_the_left_column_and_tasks_open_as_a_large_popup` 与 `tests/workflow.rs::tasks_entry_opens_the_popup_and_closing_returns_to_the_previous_target`，旧实现上均因没有 Tasks 入口、Queue 仍常驻左下而失败；实现后 GREEN。
- 直接受影响的既有检查按新几何/交互改写：`tests/layout_config.rs`（两项）、`tests/ui.rs`（详情并排、视图切换保留阅读位置、历史滚动条、页脚位置等）、`tests/queue.rs`（Detail/Task 页相关四项改为选中驱动的内容与视图）、`tests/app.rs`、`src/launch.rs` 单测，以及 `tests/workflow.rs` 中使用左下 Queue 的用例（先打开 Tasks、新按钮文案、resize 后等重绘再点击、弹窗遮住 Viewer 处改等事件）。两个 `#[ignore]` 的真实 drover 用例同步改写，只编译未运行。
- `cargo test --all-targets`：除已知基线 `full_workflow_routes_input_switches_safely_and_survives_disappearance` 外全部通过（workflow 36 过 / 1 败 / 2 忽略），workflow 套件连续多次运行稳定。该基线用例失败在第 293 行 `h.event("input p/a 1b5b3c303b333b324d")`（经 245 行 `event` 辅助函数），发生在任何 Tasks 操作之前：点击 (56,4) 因终端标签栏（T2）下移了窗格而不再落入终端内容区，未转发；与本次布局无关，未修。其后与 Queue 几何相关的一处断言已按新布局调整。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。

### 实现取舍
1. 入口放在 Agents 顶边框右侧，而不是加进按钮行，避免改变四按钮换行和列表起始位置；文案带快捷键 `Tab`，与其他按钮一致。
2. 弹窗描边沿用现有弹框（New agent、Stop、Projects 等）的 `t.block` 粗描边 + overlay 底色，未另做圆角外框；内部按钮是现有紧凑样式。
3. 弹窗从 (0,1) 开始，保留 Agents 顶边框（入口）和状态行可见；不画标题栏 ×，关闭走 `Close Esc`、列表页 Esc、`q`。子视图里 Esc 为返回，`q`/Ctrl-] 可直接关闭。
4. Ctrl-] 在弹窗里仍回 Agents（既有全局规则，不另加快捷键）；Esc/Close/q 才回到打开前的输入目标（例如从 Viewer 点入口打开，关闭后回 Viewer）。
5. 右侧默认显示 Task text（草图中排第一），Enter/Run details 切换；所选视图跨任务、关闭再打开、切换项目都保留。
6. 列表与内容并存后，g/n/p/l/a/A/c/u/d/x/?/r 在显示运行详情时也可用（旧详情页禁用它们，是因为当时详情替换了列表）。
7. 选中改为驱动内容：换到另一任务即新的 show 目标（新序号），同一任务在两个视图间来回切换沿用原目标和已取到的数据。
8. 小窗口：弹窗内宽 < 60 时列表叠在内容上方（列表约 2/5 高），顶部按钮可换行；表单太矮时沿用原「Enlarge the window」提示。
9. 按草图改了部分按钮文案：`Go g`→`Check & release g`、`Add a`→`Add task a`、`Project c`→`‹<项目名> ▾ c›`、`Details ↵`→`Run details ↵`、`Task t`→`Task text t`；帮助文本同步。原文视图正文为空时显示 `No body`。
10. 操作反馈：弹窗打开时状态行显示最近操作消息，失败或 Next/放行结果照旧进 Action result 子视图；弹窗关闭期间完成的操作要重新打开才看得到。

### 未做的事
- 未实现已取消的 T16 和 T17；未改右侧终端 tab、分屏及按钮样式；未改任务状态/删除语义、公开 CLI 或 PTY 生命周期；未加配置项。
- 未改 `docs/DESIGN.md`（第 32 节按已批准方案实施，上面的实现取舍待主控决定是否写入）。
- 未做真实 agent、录屏或覆盖矩阵测试；未合并、未推送、未构建 release。
- 基线用例 `full_workflow_…` 的鼠标转发断言未修（范围外，见上）。

## 实施中补充：入口显示简短状态（完成记录）

需求原文见主仓库本文件末尾「实施中补充：入口显示简短状态」及 DESIGN 第 32 节新增条目（用户：「我觉得你上面踢的那个留意的点是必要的。」）。本分支在主体完成后追加。

### 改动
- `queue::Panel::entry_status`：只从现有公开快照 `drover list --json` 得出入口状态，按需要关注的优先级取一项：`Awaiting release`（待放行色）> `Running`（运行色）> `Paused`（待放行同色）> `N pending`（Pending 组色）> `Idle`（空闲色）。尚未读到显示 `Loading…`；读取失败显示 `Read failed`（错误色），不沿用失败前的旧快照。没有新增查询、通知或状态推断。
- 入口改为 `‹Tasks · <状态> Tab›`，状态文字用上述语义色；弹窗打开时入口仍高亮、不可点。Agents 左列窄时依次去掉 `Tab` 提示、改短写（`Awaiting`、`Failed`、待办只留数字），再不够则只显示 `Tasks`。
- 中英文 README 补充入口状态说明。

### 验证
- RED→GREEN：新增 `tests/ui.rs::closed_tasks_entry_keeps_the_projects_short_status_in_semantic_colors`（Loading、各状态文字与颜色、读取失败、窄列短写），旧实现只显示 `Tasks Tab` 而失败，实现后通过。
- 定位入口的既有检查从精确文字 `Tasks Tab` 改为稳定前缀 `‹Tasks`（`tests/workflow.rs`、`tests/app.rs`、`tests/ui.rs`）。
- 重跑受影响目标：`ui`、`app`、`queue` 全过；`workflow` 36 过 / 1 败（仍只有已知基线 `full_workflow_…`，失败点同前）/ 2 忽略。`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。按要求未重复无关全套。

### 取舍
- 暂停但仍有运行中/待放行任务时显示后者（更需处理）；暂停且无运行时显示 `Paused` 而不是待办数。
- 入口文字随状态变长，窄列按上述顺序降级；因此测试不再依赖入口完整文字。
