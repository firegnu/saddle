# T41：Dispatch 详情返回按钮样式

2026-09-29，saddle/main 交给 Claude Code，轻档 sonnet / medium。
路由：轻 / 交叉审查不要 / 影响面：看得见（JEV 档位拿不准、交叉审查不要、影响面拿不准；主控判断为复用现有按钮组件的局部纯显示修正）。
你是被委派的实现者：照本文件做，不再委派。

## 用户原话与验收

「tasks面板中，选中某一个task，右边的dispach面板中再选中一个进入这个详情的时候的返回按钮样式不对」

## 先读

- AGENTS.md
- docs/DESIGN.md §30（弹窗轮廓按钮）、§54（Dispatch）、§56（本次修正）
- src/queue_dispatch.rs、src/buttons.rs，与该处直接相关的渲染检查

## 在哪里干活

分支 `t41-dispatch-back`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t41-dispatch-back`。
只改 Dispatch 全文详情页的返回按钮渲染和直接相关显示检查；可在本文件追加完成记录。

## 要做的

Dispatch 记录全文页目前使用单行紧凑 `Back Esc`。使它复用弹窗既有的三行圆角轮廓按钮，正文沿按钮下方布局；保持现有点击／Esc 返回所选记录的行为。遵守共享组件的主题、命中区与窄窗处理，不另造按钮样式。

## 验证预算

纯显示修改，不要求人为制造 RED。`git diff --check` 加一项能直接确认按钮外观与正文位置的合成渲染检查；沿用现有相关检查即可。不跑全套、录屏、真实数据验证或植入缺陷。编译使用项目共享 CARGO_TARGET_DIR。

## 不要做

不改 Links、共享按钮组件、导航行为、采集读取或任何队列流程；不改 corral、drover、dispatch-log 或全局技能，不操作真实任务／agent。不合并、不推送，只在自己的分支提交。

## 做完

末尾追加完成记录：修改、检查、取舍与提交 SHA。命令都在前台跑完，回复最后一行写 DONE。

主控记录器 dispatch_id：`b460a96365bc4b14a347f2d307ef968a`；此字段供主控关联，实现者不负责采集。

## 完成记录

- 修改：`src/queue_dispatch.rs` 全文页返回按钮由 `draw_compact_top` 改为共享 `draw_outlined_top`（三行圆角轮廓），正文用其返回的剩余区域，点击／Esc 返回不变。
- 检查：`tests/workflow.rs` 的 `dispatch_tab_lists_recorded_steps_opens_full_text_and_returns` 增加断言：按钮上下为 `╭`／`╰`，正文在按钮下方；该测试通过，`git diff --check` 干净，clippy 通过。
- 取舍：未动 Links、共享按钮组件；未跑全套。

## 主控审查

结论：通过。实现提交 `b818247`，仅将 Dispatch 全文页的 `draw_compact_top` 换成 `draw_outlined_top`，沿用共享按钮与返回区域，不改导航或读取；接受局部修正，不扩展到 Links。现有合成 workflow 检查补了轮廓上下边框和正文位置，仍检查 Esc 回到原记录；实现者报告该项与 clippy 通过，diff 检查干净。纯显示预算，主控看 diff，不重复套件。

任务书先读节号原误写 §34，主控已纠正为 §30；完成提交 SHA 由主控补记，未要求额外返工。路由、决定、派发快照、完成回复与本审查通过同一 dispatch_id 保留；早先纠正文档消息因 agent working 被拒绝，记录为未送达，不冒充返工已执行。

## 合并与收尾

合并 `b4fed14`，主控审查 `59cedd5`，收尾 `a86a36b`。在 idle、attached=0、工作树干净且已合入的条件下，清理实现分支／worktree，并关闭其中自开的实现者。旧 t38-dispatch-study 保留。main release 编译成功（`/tmp/saddle-t41-release.log`），原可执行软链生效；未代用户重启 Saddle。公开只读核对 current=T41、T29 在 pending、loop=false、paused=false；未推进真实队列。
