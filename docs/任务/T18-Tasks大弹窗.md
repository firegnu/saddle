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

## 实施中补充：入口显示简短状态

主控提出 Tasks 关闭后任务进度会隐藏，入口可保留简短状态，例如“运行中”或“待放行”。用户回复：「我觉得你上面踢的那个留意的点是必要的。」本条已获用户同意，纳入 T18，具体见主仓库 DESIGN 第 32 节新增条目。

在常驻 Tasks 入口展示当前项目的简短状态或待办数量，弹窗关闭时也能知道是否需要处理。沿用公开快照与现有配色，不新增通知系统。此补充与现有入口一起实现；如主体已完成，追加最小修改和一条直接检查，不为此重复无关全套。最终完成报告包含本项。
