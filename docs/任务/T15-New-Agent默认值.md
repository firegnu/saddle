# T15：New Agent 对话框默认值

2026-09-27，saddle/main 交给 saddle/dev-t15-defaults（Codex，轻档：gpt-5.6-luna / medium）。
路由：轻 / 交叉审查不要 / 影响面：改行为（路由：档位拿不准，交叉审查不要，影响面改行为；主控按两个已确定默认值的机械调整选轻档）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 31 节。
- src/launch.rs 的表单默认值与现有测试，以及 tests/workflow.rs 中受默认值影响的检查。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t15-new-agent-defaults，分支 t15-new-agent-defaults（从 main 建好）。
- 只动 src/launch.rs、受这两个默认值直接影响的现有测试及本任务完成记录。

## 要做的
New Agent 默认名字改为 `main`，默认使用当前窗格打开。

## 怎么算做完
> New Agent有两个默认值要改一下，一个是默认名字。改为main，然后使用current panel打开而不是new tab

验证预算：针对两个默认值的定向自动检查，先确认旧实现因目标默认值失败，再最小实现；项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，及 `git diff --check`。命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不扩展名称冲突规则、其他布局操作、PTY 生命周期或 UI 样式。
- 不操作真实 agent、drover 队列或其他仓库；测试用假 CLI/合成数据。
- 不按项目名或路径批量杀进程。
- 不合并到 main，不推送，不构建 release；只在本分支提交。
- 遇到范围外失败记录并报告，不擅自扩大修复。

## 做完
在本文件末尾追加「## 完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事。回复加提交 SHA 和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
