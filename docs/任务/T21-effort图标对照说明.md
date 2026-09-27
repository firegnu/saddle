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

## 完成记录

- 改动：在 `README.md` 与 `README.zh-CN.md` 的 Agents 说明中，将已有的 `medium`、`high`、`xhigh` effort 图标、默认颜色和对应主题色整理为简短对照表；保留公开委派标签语义、运行时 effort 非同义、无有效标签不显示及列对齐说明。
- 实际检查：已只读核对 `docs/DESIGN.md` 第 23 节与第 40 节 effort/Working 补充、`src/corral.rs` 的三档解析和 `src/ui.rs` 的图标/颜色映射；执行 `git diff --check`。
- 取舍/未完成事项：仅做 README 与本任务记录的文档整理，未改代码、设计、测试或产品行为；未运行全套测试、构建、交叉审查或发布。


## 主控审查（2026-09-27）：可以合并

核对实现者 idle/attached=0、新回复 DONE、提交 cddbce4 与完整差异。仅 README 中英文和任务完成记录改动；三档、字形、默认色与 src/corral.rs::effort、src/ui.rs::effort_bars 及主题常量一致，均保留公开委派标签不代表运行时 effort、未知不显示、列对齐和折叠显示语义。表格中英一致，同意仅整理已有说明的取舍，无剩余必须改或建议改。

主控 git diff --check 通过；按用户纯文档预算不跑套件、不交叉审查、不构建或发布二进制。用户现场已确认「我看到了，已经好了。等任务结束。」作为本次 effort 显示体验反馈记录，不扩大为运行时模型档位检测结论。按流程合并推送、清理收尾后公开 drover done T21，等用户放行。


## 合并与清理

合并90fda11、审查e14f6d0已推送origin/main。核对实现者idle/attached=0、worktree干净、分支已合入main后，正常删除t21-effort-docs worktree/分支；工作目录已删，一并关闭自有saddle/dev-t21-effort-docs-1。未发布二进制。随后补收尾空提交、交接并公开drover done T21，等待用户放行。
