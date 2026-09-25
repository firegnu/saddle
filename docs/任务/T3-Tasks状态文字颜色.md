# T3：Tasks 状态文字颜色区分

2026-09-25，saddle/main 交给 saddle/dev-t3-colors（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由三项结论一致）。项目要求主控审查。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读

- AGENTS.md。
- docs/DESIGN.md 第 8、21 节；src/queue.rs 的 tasks 和列表绘制；src/theme.rs、config.toml；tests/ui.rs 的 Queue 配色检查。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t3-task-colors，分支 t3-task-colors，已从 main 建好。
- 只改 Tasks 文字配色、对应渲染检查、相关配置注释/文档和本任务记录。所有 cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 要做的

- 用户原话：“Tasks 中的 current 和 pending 文字要能通过颜色区分；看一下这个地方有哪些不同的状态。”
- 队列旧正文有占位限制，但用户已在主控本次会话明确回答“是的”，确认完整范围为“梳理 Tasks 的全部状态，并让状态文字通过颜色区分”，现可实施。
- 已核对列表分组 Current、Awaiting、Pending、History，目前标题都用 muted；任务行已有 Running、Awaiting、Pending、Done、Failed、Dropped，以及未知原值或缺失占位。
- Current 标题及 Running 文字用现有 agent_working 蓝；Awaiting 标题及文字用 agent_blocked 琥珀；Pending 标题及文字用 agent_starting 紫。History 是多结果集合，标题保留中性，历史行仍按 Done 绿 agent_idle、Failed 红 agent_error、Dropped 橙 agent_stalled；未知值保留原文和中性色，不猜测。
- 只做这处状态文字辨识，复用 T1 配置值，不另建主题体系。配置注释及 DESIGN 对应说明同步；完成记录列明分组与状态的区别及实际清单。

## 怎么算做完

> Tasks 中的 current 和 pending 文字要能通过颜色区分；看一下这个地方有哪些不同的状态。

用户确认的完整范围：

> 梳理 Tasks 的全部状态，并让状态文字通过颜色区分。

验证预算：纯视觉变更无需伪造 RED；git diff --check，加一条直接观察渲染结果的检查（使用并按需补充 tests/ui.rs，运行 cargo test --test ui）。无需全套测试、录屏、真实 agent 或缺陷注入；若发现需要行为改动，停下报告。

## 不要做

- 不改任务状态、分组、排序、滚动、CLI 数据及操作、任务正文。不要扩展到 T4 的过程详情或 T2 的 tab/split。
- 不改 Agents 和 Queue 顶部项目状态配色，不调整界面布局或引入新交互。
- 不读 corral/drover 内部文件，不改它们的仓库；不改用户真实配置、队列或服务，不操作真实 agent。
- 不按项目名或路径批量杀进程，只停自己记录 PID 的进程。
- 不合并 main、不推送、不更新 HANDOFF.md；只在 t3-task-colors 提交。

## 做完

在本文件末尾追加「## 完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复同样列这几样和需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
