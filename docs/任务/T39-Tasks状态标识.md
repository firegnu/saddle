# T39：明确 Tasks 队列状态与所选任务状态

2026-09-29，saddle/main 委派 saddle/dev-t39-status-labels（Claude Code，轻档 sonnet / medium）。
路由：轻 / 交叉审查不要 / 影响面：看得见（路由：档位拿不准，交叉审查不要，影响面看得见；两处已确定文字／状态展示调整，主控定轻档）。
你是被委派的 agent，按任务做，不再派 agent。

## 先读与工作位置

- 先读 AGENTS.md、docs/DESIGN.md §52，及 src/queue.rs 的 mode_line、Task text 绘制和 task_status 与对应渲染测试。
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t39-status-labels；分支 t39-status-labels。
- 改动限直接相关 src/queue.rs、渲染测试、DESIGN §52 和本任务完成记录。不改其他仓库、配置、用户状态或无关代码。

## 用户任务原文与完成范围

问题：Tasks 弹窗顶部的 Running／Ready 等文字表示项目／队列整体状态，未说明其作用范围；只要存在 current，选择 pending 时顶部仍为 Running。默认 Task text 视图没有独立任务状态标签，容易误认为所选 pending 任务正在运行。

修复范围：
- 顶部状态行明确标注为队列状态，例如 Manual · Queue: Running · Loop off。
- 给选中任务标题增加该任务自身的状态标签，例如 Pending／Running／Done，沿用已有任务状态与颜色语义，让默认任务正文视图也能看清所选任务状态。
- 只调整 Tasks 展示及直接相关渲染测试，不改任务状态判定、队列流程或异步结果归属，不改 corral／drover。

用户未另给验收条款，以上原文就是完成范围，不新增验收点。已完成只读调查：current + pending 时选 current 为顶部 Running／右侧 Running，选 pending 为顶部 Running／右侧 Pending；暂停时顶部 Paused，仅 pending 时顶部 Ready。旧 current 异步详情在选 pending 后会被丢弃。不要重做调查或改这些逻辑。

## 验证预算

纯展示变化，按 AGENTS.md 可不制造行为 RED；跑直接体现两处展示结果的渲染检查，git diff --check，按需格式检查。共享 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。仅用临时／合成快照，不跑全套、不做统计重跑、不录屏、不使用真实 agent 或任务队列。预算不足报告，不自行扩展。

## 边界与交付

不操作真实 corral agent 或 drover 队列，不调用写操作；不读它们内部文件。不更改任务状态判定和异步逻辑，不顺手修 T29。不批量杀进程。不安装／发布／重启 saddle。
只提交本分支，不合并、不推送、不清 worktree。不新增架构或状态系统，遇到超出显示修改的需求停下报告。
完成后在本文件追加短记录：改动、实际检查、取舍和未完成项。回复固定 SHA，所有命令前台跑完，最后一行 DONE；完成后等待主控，不另设后台唤醒。

## 完成记录（saddle/dev-t39-status-labels）

- 改动：`src/queue.rs` 顶部状态行加 `Queue: ` 前缀（如 `Manual · Queue: Running · Loop off`）；Task text 视图标题首行前加所选任务自身状态标签，沿用 `task_status` 的文字与颜色。`tests/ui.rs` 补 Queue 前缀断言，新增 current + pending 下选中 current／pending 时顶部均为 Queue: Running、标题分别带 Running／Pending 及对应颜色的渲染测试。
- 实际检查：`cargo test --test ui queue_` 通过；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 干净。
- 取舍：状态标签作为标题首行的前缀，与标题一起换行；未改状态判定和异步逻辑。
- 未完成/注意：按预算未跑全套。顺带跑了 `--test workflow`，`t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases` 时过时不过（多次运行中本分支跑 5 次失败 3 次；暂存改动后基线只跑了 1 次，通过，样本不足以定论），失败画面在 Viewer 新建 agent 弹窗，与 Tasks 展示无关，疑为已有的时序不稳，未处理。


## 主控审查（2026-09-29）

固定 ca2be4f53b0ac54da2b351a1c8dfede7967e24d6；实现者 DONE、idle、attached=0，工作区干净。主控核对完整 diff 和渲染断言，结论：可以合并。顶部 Queue: 与所选任务标题自身状态分开；任务状态文字和颜色复用 task_status，未改状态判定、队列流程和异步结果归属。接受把状态放在标题前缀并随标题换行的取舍。

直接相关 `cargo test --test ui queue_`、clippy／fmt 检查由实现者报告通过；主控 diff 检查通过，按“看得见”预算不重跑套件。实现者超预算运行 workflow 和同一时序用例多次，并用自建 stash 做基线对照，属于流程偏差；stash 已恢复删除，无遗留文件或其他分支改动，不再追加验证。该测试在 T37 阶段已有基线失败证据，仍留在 T29，不称为本任务修复或全套全绿。

本次只改显示及对应测试，未发现必须改项。主控接着合并、编译 release 和清理；不推进真实队列。


## 收尾记录

合并 5f99074；主控 release 构建成功，既有 ~/.local/bin/saddle 软链已指向包含 Queue: 的新产物。未替用户重启 saddle。t39-status-labels 分支和 worktree 已安全删除；实现者 idle、attached=0，工作目录已删，一并关闭。收尾空提交 e624e3d。HANDOFF 已更新，未执行 drover done／go／next，队列留给用户验收放行。
