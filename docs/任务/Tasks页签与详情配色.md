# Tasks 页签选中态与运行详情配色

2026-09-27，saddle/main 交给 saddle/dev-task-colors（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由结论一致）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 35 节。
- src/queue.rs 的 draw_content、src/detail.rs、src/theme.rs 及直接相关渲染检查。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/tasks-detail-colors，分支 tasks-detail-colors（从 main 建好）。
- 只动 src/queue.rs、src/detail.rs、直接相关渲染检查及本任务完成记录；确有必要才更新现有颜色配置说明，不改主题默认值或用户配置文件。

## 要做的
按设计第 35 节加强两个页签的选中态，并补齐运行详情的结构化状态颜色。当前页签使用 focus 色、加粗和 ●，非当前项灰色和 ○；保持无填充描边与紧凑降级。详情分节标题用 reply_heading，状态使用已有语义色，普通内容中性；从结构化字段取状态，不猜测任意文本含义。

## 怎么算做完
> 但任务的task text和run details这两个按钮没有状态比那话，我不知道现在是哪个。以及run detals的内容字体全是白的。没有状态的颜色。状态的颜色你来考虑怎么配置。

验证预算：纯显示调整，不要求制造 RED。`git diff --check`，加一条直接渲染检查观察页签选中区分和详情状态配色，运行直接受影响的现有检查；不跑全套、矩阵、录屏。必要 Cargo 命令加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不改变视图切换操作、详情内容含义、数据查询、任务生命周期或用户主题配置；不扩展主题系统、按钮框架或其他 UI。
- 不读或修改其他仓库，不操作真实 agent、drover 队列；使用假 CLI/合成数据。
- 不按名称或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。
- 范围外问题记录报告，不扩修已知 full_workflow 旧鼠标坐标问题。

## 做完
在本文件末尾追加完成记录并提交：改动、验证、取舍、未做事项。回复带提交 SHA 和需要主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
