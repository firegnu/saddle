# T42：打开 Tasks 时优先显示焦点 agent 所属仓库

2026-09-29，saddle/main 交给 Claude Code，常规 opus[1m] / high。
路由：常规 / 交叉审查不要 / 影响面：改行为（JEV 三项 verdict 均拿不准；主控判断为现有界面项目选择行为调整，复用只读异步机制，不改队列核心）。
你是被委派的实现者：照本文件做，不再委派。用户已回复「开始吧」，此前“仅创建待办”的限制结束。

## 用户原话与验收

「加一个简单的任务，我当前的焦点在那个agent的时候，打开tasks面板的时候应该直接切换到该repo所在的tasks，如果这个repo没有任务，才是fallback到现在的逻辑。」

## 先读

- AGENTS.md
- docs/DESIGN.md §32（Tasks 弹窗）、§57（本次行为）
- src/app.rs 的 Tasks 打开与项目切换、src/terminals.rs::Pane 的公开目标信息；相关 src/corral.rs、src/git.rs、src/drover.rs 与 tests/workflow.rs

## 在哪里干活

分支 `t42-focused-tasks`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t42-focused-tasks`。
只改打开 Tasks 的项目选择及直接相关辅助代码／检查。可在 DESIGN §57 记录必要取舍，并在本文件追加完成记录。

## 要做的

普通 Tasks 入口（点击、Agents 的 Tab）打开时，优先显示当前焦点 agent 所属 repo 的任务。Agents 焦点取当前选中 agent；Viewer 焦点取当前活动窗格中的 agent，不拿侧栏旧选中项替代。使用公开 cwd 和必要的只读 Git 信息确认所属 repo，兼顾 repo 子目录及已有 worktree；不按 agent 名称猜项目。

“有任务”包括 current、awaiting、pending 或 history 中任意任务。无焦点 agent、无法确认仓库、该仓库无任务或读取失败时，保留现有项目选择；不增加配置。外部读取不得卡界面，沿用现有超时／异步读取方式。

保持已有显式导航、项目手选和表单保护：Attention 指定的目标不被覆盖，不丢尚未保存的编辑或改变正在进行的任务操作；已关闭或失效的打开请求不能事后切换项目。只在打开时优先，不持续跟随焦点。

## 验证预算

目标行为先有能因本次缺失而失败的自动检查，再实现到 GREEN，并跑直接相关回归。项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次；`git diff --check`。使用共享 CARGO_TARGET_DIR。只用临时／合成项目和假 CLI，不操作真实队列或 agent。不加覆盖矩阵、录屏或变异检查。遇到无关既有失败如实记录，勿顺手修 T29。

## 不要做

不改任务状态判定、队列流转、任务操作权限，不改 corral、drover、dispatch-log 或全局技能，不读取它们的内部状态文件。不要推进真实 T42 或 T29，不清理历史 t38-dispatch-study。不要按名称批量杀进程。不合并、不推送；只在自己的分支提交。若公开数据不能支持需求，先报告缺口，不自行扩展上游。

## 做完

末尾追加完成记录：修改、验证、取舍和待决定事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

主控记录器 dispatch_id：`25d80505684e4cbf9a6f757097e21e6a`；实现者无需采集。

## 完成记录（实现者，2026-09-29）

修改：
- `src/git.rs`：新增只读 `repository()`，给出 cwd 的 worktree 顶层与共享仓库目录（规范化路径）。
- `src/drover.rs`：新增 `RepoTasks` 后台查找——按共享仓库目录匹配登记项目（同顶层优先），再用公开 `drover list --json` 判断 current／awaiting／pending／history 是否有任意任务；可取消，Drop 时停止。
- `src/app.rs`：Agents 的 Tab 与点击 Tasks 入口打开时，取焦点 agent（Agents 选中项；Viewer 当前活动窗格的 agent，shell 窗格不算）的公开 cwd 启动查找；`tick` 中仅当 Tasks 仍打开、之后无输入、项目未变、仍为列表页且无写操作时切换项目，否则丢弃。Attention／提示框的显式打开不受影响。
- `tests/workflow.rs`：新增 `tasks_open_on_the_focused_agents_repository_unless_it_has_no_tasks`（合成 git 仓库 + worktree 子目录 + 空队列仓库 + 假 drover／corral）。
- `docs/DESIGN.md` §57 追加实现取舍。

验证：
- 新测试先 RED（停在 project-one，未切到 project-two），实现后 GREEN；另临时给空仓库加一条 history 任务确认 fallback 步骤会失败（已还原）。
- `cargo test --all-targets` 全部通过（workflow 79 passed、3 ignored 为既有）；`cargo clippy --all-targets -- -D warnings` 无警告；`git diff --check` 通过。均用共享 CARGO_TARGET_DIR，未碰真实队列或 agent。

取舍：见 DESIGN §57「T42 实现取舍」。要点：只认登记表中的项目；同仓库多个登记项优先同 worktree 顶层；打开后任何输入即放弃自动切换。

待决定：
- 打开后“任何输入即放弃切换”偏保守：若 drover 很慢而用户马上开始翻列表，会停在原项目。如需“仅手选项目才放弃”，可放宽。
- 未登记到 `~/.drover/projects` 的仓库不会被切到（即使该目录有 drover 队列）。
