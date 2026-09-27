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

## 完成记录（saddle/dev-task-colors，2026-09-27）

- 改动：
  - `src/queue.rs` draw_content：两个页签前加 `●`（当前）/`○`（非当前）；当前页签在描边和紧凑两种样式下都用 Primary（focus 色文字与边框）并整块加粗，非当前页签的标签文字改 muted，边框沿用原色，仍无填充。悬停时 Pointer 仍把整块刷成 bright，当前项靠 `●` 和加粗区分。视图切换键、命中区域逻辑不变（标签多两列，命中框随宽度变化）。
  - `src/detail.rs`：分节标题（含 Body）改用 `reply_heading` 加粗；Hold 开启时 `On` 琥珀色，关闭 text、未知 muted，`(at task end)` 等说明中性；Attention 仅状态词着色，suggested / awaiting_release 琥珀色，其余中性，reason 与 `(inferred)` 保持 text；Unmet 条目琥珀色；Main agent 只给状态词着色（working/blocked/idle/starting 沿用 Agents 配色，其余 muted），名称、via、idle 时长中性。顶部 “Suggested attention …” 一行由 agent_stalled（橙）改为 agent_blocked（琥珀），与设计第 35 节 “Attention suggested 为琥珀色” 保持一致。时间、路径、SHA、数量未动，仍为中性。未改主题默认值、颜色配置项或用户配置。
  - `tests/ui.rs`：`management_layouts_…` 的页签标签断言改为带 `●`/`○`；新增 `task_tabs_mark_the_chosen_view_and_details_color_structured_states`，在 160×48（描边）与 80×24（紧凑）下检查页签标识随视图切换、当前项 focus+加粗、非当前项 muted 不加粗；悬停两个页签时当前项仍加粗；详情中分节标题为 reply_heading、Running 蓝、Suggested/Unmet/Hold On 琥珀、Main agent 仅状态词着色（idle 绿、working 蓝）、Hold 未知 muted、其余说明 text。
- 验证：`git diff --check`、`cargo fmt --check` 通过；`cargo test --test ui` 29 passed；`cargo test --lib` 9 passed；`cargo test --test queue` 16 passed；`cargo test --test workflow` 36 passed、1 failed、2 ignored，唯一失败为已知 `full_workflow_routes_input_switches_safely_and_survives_disappearance`（tests/workflow.rs:245 旧鼠标坐标，未扩修）；`cargo clippy --all-targets -- -D warnings` 通过。未跑全套矩阵、未录屏，纯显示调整未制造 RED。
- 取舍：Hold 未知用 muted 而非 text（第 35 节 “未知保持 muted/dim”，仍属中性）；非当前页签只把标签内部刷 muted，不动边框，紧凑样式的 `‹›` 保持 border 色。
- 未做：未改 buttons.rs、主题系统、其他 UI；未处理 full_workflow 旧问题。

## 主控审查（2026-09-27）

- fef8627 可以合并。页签在描边与紧凑两种样式下用 ●/○ 区分当前项，当前项 focus 色加粗，悬停后仍保留标识；正文切换、CLI 请求与状态语义未改。
- 详情标题沿用 reply_heading，Hold/Attention/Unmet/Main agent 的结构化状态词使用现有主题色，其他文本保持中性；接受 Suggested attention 顶部提示统一琥珀色、Hold 未知用 muted 两项取舍，均符合 DESIGN 第 35 节。Main agent 不额外推断 stalled/error。
- 主控核对完整 diff 和开发渲染断言，直接运行 `task_tabs_mark_the_chosen_view_and_details_color_structured_states` 1 条通过，diff 检查通过；按纯视觉预算不重跑套件。开发报告 UI 29、lib 9、queue 16 通过及 Clippy/fmt 通过，workflow 唯一失败仍是记录在案的旧鼠标坐标问题，不称全套全绿。
- 无阻挡意见。配置项、主题默认值和用户配置均未改，其他界面/队列/agent 不在本次范围。
