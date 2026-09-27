# New Agent 可填写名称前缀

2026-09-27，主控 main 交给 saddle/dev-name-prefix（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规，交叉审查和影响面拿不准；主控按普通表单行为变更判定）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 34、36、38 节。
- src/launch.rs 及直接相关测试。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/new-agent-prefix，分支 new-agent-prefix（从 main 建好）。
- 范围：src/launch.rs、必要的 launch 输入支持、直接受影响的测试、中英文 README、本任务完成记录。设计有疑问先报告。

## 要做的
按 DESIGN 第 38 节，把 agents 前缀放在普通 New 表单中供用户填写，默认 agents；预览与提交使用填写的前缀和名称。保持既有 Controller 固定 main、Regular 可编辑名称及角色草稿行为，沿用现有表单风格。

## 怎么算做完
> 还有一个问题是，新建agent的时候，默认是agents/main 我觉得这个agents也要放出来让用户填写

验证预算：目标行为自动检查先 RED 再 GREEN；运行受影响定向检查及项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，另做 `git diff --check`。Cargo 命令加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。已知 full_workflow 旧鼠标坐标基线失败如仍出现，记录即可，不扩修。

## 不要做
- 不改命令默认、角色标签、打开位置、派发流程、其他 UI 或全局配置。
- 不操作任何真实 agent 或队列，不读写 corral/drover 内部文件或仓库；测试使用假 CLI 和合成数据。
- 不按项目名或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。

## 做完
在本文件末尾追加完成记录并提交：改动、验证、取舍、未做事项。回复附提交 SHA 和待主控决定事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
