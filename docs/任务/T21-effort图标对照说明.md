# T21：整理 README 的 effort 图标对照说明

2026-09-27，saddle/main 交给 saddle/dev-t21-effort-docs（Codex，轻档 gpt-5.6-luna / medium，role=implementer）。
路由：轻 / 交叉审查不要 / 影响面：看得见（用户在 T21 明确指定家、模型、强度及纯文档预算，不另行改变）。
你是被委派的 agent，照本文件做，不再开其他 agent。

## 先读

AGENTS.md；README.md 与 README.zh-CN.md 的 Agents/effort 段落；docs/DESIGN.md 第23节以及第40节 effort 与 Working 动画补充；src/corral.rs 的 effort 解析及 src/ui.rs 的 effort_bars（只读核对）。

## 在哪里干活

worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t21-effort-docs；分支 t21-effort-docs。仅修改 README.md、README.zh-CN.md 和本文件末尾完成记录。

## 目标与验收（用户任务原文）

用户要求：「任务表里面开一个新任务，我来触发，顺便测试一下effort的表示」。

小任务：把 README.md 与 README.zh-CN.md 中已有的 effort 三档图标说明整理成简短对照表（档位、图标、默认颜色），保留“只反映创建时公开标签，不代表运行时实际 effort；无有效标签不显示”的说明。只整理已有内容，不新增产品行为、不改代码或设计。

用户现已通过任务表触发 T21，主控负责实际派发，公开 model/effort 标签与实际参数一致。不新增模型名显示，不改 effort 功能。

## 验证预算

核对中英文与现有实现一致，git diff --check；不跑全套、不交叉审查、不构建/发布二进制，不造 RED，不装依赖或做截图。命令前台完成。

## 不要做

不改 src、测试、DESIGN、原始设计稿或其他文件；不修改主仓库或其他worktree，不操作任何真实agent/用户TUI/socket/队列，不读 corral/drover 内部文件，不批量杀进程。不合并、不推送、不发布，只在此分支提交。

## 做完

在本文件末尾追加完成记录：改动、实际检查与取舍/未完成事项。提交后回复新 SHA，最后一行 DONE。主控负责后续审查、合并推送、清理与空提交收尾，再公开 drover done T21，等待用户放行，不 go/next。
