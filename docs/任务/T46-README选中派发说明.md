# T46：README 补充派发选中任务的使用说明

2026-09-29，saddle/main 交给 Claude Code，轻档 sonnet / medium。
路由：轻 / 交叉审查不要 / 影响面：看得见（JEV 三项均明确，照用）。
类型：样式／文案调整
依据：补充已有按钮的用户文档，不改变功能。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的实现者，照本文件做，不再委派。

## 先读与范围

读 AGENTS.md、README.md 的 Tasks 说明、docs/DESIGN.md §59；必要时核对 src/queue.rs 的 dispatch_target。
在 `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t46-readme-selected`，分支 `t46-readme-selected` 工作。只修改 README.md，并在本任务文件追加简短完成记录。

## 要做的与验收

用户任务原文：「在 README 的 Tasks 说明中补一小段 Dispatch selected 的使用方法：选中 Pending 任务后点击按钮即可派发该项，无需先调整顺序；简要说明按钮不可用的情况。只改文档，不改代码。」

用户已通过 Tasks 派发，主控只读确认 current=T46。沿用 README 的语言和写法，不扩写其他功能。

验证只做 git diff --check 和对照现有按钮行为阅读新增段落；不跑测试、clippy 或编译，不操作真实队列或 agent，不改上游。不合并、不推送，只在本分支提交。

## 做完

追加修改、检查和必要取舍的简短完成记录。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
