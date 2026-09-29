# T44：Dispatch 详情返回按钮恢复文字样式

2026-09-29，saddle/main 交给 Claude Code，轻档 sonnet / medium。
路由：轻 / 交叉审查不要 / 影响面：看得见（JEV 档位拿不准、交叉审查不要、影响面看得见；主控按局部外观调整判轻档）。
类型：样式／文案调整
依据：本轮按用户偏好恢复返回入口的文字外观，不改变返回行为。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的实现者：照本文件做，不再委派。

## 用户原话与验收

「这样还是把tasks列表中dispatch选中某一个进入具体内容的那个按钮样式改回文字吧。现在的有点突兀。」

## 先读

- AGENTS.md、docs/DESIGN.md §56（含 T44 更新）。
- src/queue_dispatch.rs 的全文页渲染、src/buttons.rs 的紧凑文字按钮。
- tests/workflow.rs 的 dispatch_tab_lists_recorded_steps_opens_full_text_and_returns。

## 在哪里干活

分支 `t44-dispatch-back-text`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t44-dispatch-back-text`。
只改该处渲染、直接相关显示检查，并在本文件追加完成记录。

## 要做的

将 Dispatch 记录内容详情页的 `Back Esc` 从三行圆角轮廓恢复为已有的单行紧凑文字样式。保持点击／Esc 返回及所选记录位置。

## 验证预算

`git diff --check` 加上述一项直接合成显示检查；纯显示调整不造 RED、不跑全套或 clippy、不录屏。编译使用项目共享 CARGO_TARGET_DIR。

## 不要做

不改共享按钮、Links、读取或队列流程，不操作真实任务或 agent。不合并、不推送，只在自己的分支提交。

## 做完

在本文件追加修改、检查和取舍记录。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 本轮接手

用户已正式送入 TASK T44，公开队列 current=T44。分支保留先前未审查、未合并的实现提交 `a717a6d`；以下旧记录不是本轮验证结果。请先核对现有改动，符合要求则直接沿用，不重做；按上述预算运行一项直接显示检查，并追加本轮完成记录，有直接问题才修正。

## 前次保留改动记录

- 修改：`src/queue_dispatch.rs` 全文页的 `Back Esc` 由 `draw_outlined_top`（三行圆角轮廓）改为 `draw_compact_top`（单行紧凑文字，与 Links 全文页一致）；按钮仍是 `KeyCode::Esc`，点击／Esc 返回及所选记录位置不变。
- 检查：`tests/workflow.rs` 的 `dispatch_tab_lists_recorded_steps_opens_full_text_and_returns` 由断言三行轮廓改为断言 Back 上下无 `╭`/`╰`、正文在其下方；该测试通过。`git diff --check` 无问题。
- 取舍：按验证预算，未造 RED、未跑全套和 clippy、未录屏；未动共享按钮、Links、读取与队列流程。
