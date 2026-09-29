# T47：README 补充命令行帮助入口

2026-09-29，saddle/main 交给 Claude Code，轻档 sonnet / medium。
路由：轻 / 交叉审查不要 / 影响面：看得见（JEV 三项明确，照用）。
类型：样式／文案调整
依据：补充已有命令的帮助入口说明，只改文档。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的实现者，不再委派。

## 先读与范围

读 AGENTS.md 和 README.md 的 Getting started / Build and run。
worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t47-readme-help`，分支 `t47-readme-help`。只修改 README.md，并在本文件追加简短完成记录。

## 要做的与验收

用户原文：「在 README 的运行说明附近补一句：使用 saddle --help 查看命令行选项。沿用 README 的语言，只改这一处文档，不改代码。」

用户已通过 Next 派发，主控确认 current=T47。只做上述一句说明，不扩写。

验证只做 git diff --check 和阅读新增一句是否符合要求；不跑测试、clippy 或编译。不操作真实队列、通知设置或服务；不合并、不推送，只提交本分支。

## 做完

追加修改与检查的简短完成记录。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 在 README.md「Build and run」的安装说明之后补了一句：`Use saddle --help to see the command-line options.`，只改这一处，未改代码。
- 检查：`git diff --check` 无问题；已读新增句，符合要求。未跑测试、clippy 或编译。
