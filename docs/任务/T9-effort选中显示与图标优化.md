# T9：effort 选中显示修复与图标优化

2026-09-26，saddle/main 交给 saddle/dev-t9-effort（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规、不要，影响面拿不准；修复选中前后档位显示的回归并调整同一图标，按改行为预算，主控审查）。
你是被委派的 agent：照本文件做，不再开别的 agent。

## 先读

- AGENTS.md；docs/DESIGN.md 第 3、19、23 节；docs/任务/T7-Agents委派effort图标.md 的实现和审查记录。
- src/ui.rs 的 agent_rows、effort_bars；src/theme.rs 的颜色；src/corral.rs 的 effort()；tests/ui.rs 的 delegated_effort_shows_strength_bars_and_unknown_stays_blank。
- /Users/firegnu/.agents/skills/diagnosing-bugs/SKILL.md：本次范围小，使用针对选中前后渲染的快速复现与回归，不扩展为广泛调查。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t9-effort-polish，分支 t9-effort-polish，已从 main 建好。
- 范围：effort 图标的选中/焦点显示、图标外观、直接相关测试、必要设计/中英文说明和本任务记录。
- 所有 cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 要做的

用户原话：

> 用户发现：派出的 agent 是 Claude，委派 effort 为 high（用户原文 heigh）。焦点不在该 agent 时，effort 图标正确显示到第二格；点击选中该 agent 后，图标立即看起来变成满格。请核查是否为 bug。
>
> 另外现在effort的图标显得很不精致，看看能不能用一个更加精致的图标来展示这个effort
>
> 这是两个任务，一并完成

最后一句是用户对本轮开始核查、修复与外观改进的明确授权，替代队列正文早先的「先记录，暂不开始」。两项作用于同一图标，在此工作项一起完成，不另开 agent。

- 先复现并判断是档位数据变化还是选中渲染造成视觉混淆，再修复实际问题；完成记录说明原因。图标在选中、未选中、焦点变化时应保持相同档位的辨认结果。
- 将现有厚重的三格信号图标改得更精致、紧凑；具体字形与排版由你按现有终端界面决定。让三档在形状上也能分辨，不只靠接近的灰度猜测。避免依赖新增字体安装或大型图标库，复用现有主题与布局，不改整个 Agents 面板。先把所选方案写入 DESIGN 第 23 节，再实现，并在完成记录展示三档字符示意。
- 保留已确定的数据边界：只读公开 labels.effort，medium/high/xhigh 三档，缺失或其他值不显示；不推断默认值，不显示 model。无需修改用户配置。

## 怎么算做完

> 焦点不在该 agent 时，effort 图标正确显示到第二格；点击选中该 agent 后，图标立即看起来变成满格。请核查是否为 bug。
>
> 另外现在effort的图标显得很不精致，看看能不能用一个更加精致的图标来展示这个effort
>
> 这是两个任务，一并完成

验证预算：对报告的缺陷先用真实渲染路径取得针对性 RED，再最小修复到 GREEN；纯外观调整用合成渲染观察，不编造失败测试。针对性检查及项目标准 cargo test --all-targets、cargo clippy --all-targets -- -D warnings 各一次，另做 git diff --check。使用假数据与既有测试工具，不录屏、不做覆盖矩阵或缺陷注入；失败可运行修复所需的相关检查。

## 不要做

- 不操作用户 agent，不修改真实队列、配置或登记，不读 corral/drover 内部文件或改它们的仓库；测试不启动真实 agent。
- 不修改状态判断、排序、接入、选中行为和委派元数据语义；不做新主题体系，不夹带 T11 diff 展示、T4 或 T2。
- 不按项目名或路径批量杀进程，自己起的进程只按记录 PID 停止。
- 不合并 main、不推送、不更新 HANDOFF，只在 t9-effort-polish 分支提交。

## 做完

在本文件末尾追加「## 完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复同样列这些和需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
