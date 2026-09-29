# T41：Dispatch 详情返回按钮样式

2026-09-29，saddle/main 交给 Claude Code，轻档 sonnet / medium。
路由：轻 / 交叉审查不要 / 影响面：看得见（JEV 档位拿不准、交叉审查不要、影响面拿不准；主控判断为复用现有按钮组件的局部纯显示修正）。
你是被委派的实现者：照本文件做，不再委派。

## 用户原话与验收

「tasks面板中，选中某一个task，右边的dispach面板中再选中一个进入这个详情的时候的返回按钮样式不对」

## 先读

- AGENTS.md
- docs/DESIGN.md §34（弹窗轮廓按钮）、§54（Dispatch）、§56（本次修正）
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
