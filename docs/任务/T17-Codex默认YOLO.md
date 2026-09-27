# T17：New Agent 中 Codex 默认 YOLO

2026-09-27，saddle/main 交给 saddle/dev-t17-yolo（Codex，常规档：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规，交叉审查与影响面拿不准；主控判断为用户明确指定的局部启动默认参数调整，不改权限机制或全局配置）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 33 节。
- src/launch.rs 及直接相关的现有测试。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t17-codex-yolo，分支 t17-codex-yolo（从 main 建好）。
- 只动 src/launch.rs、直接受影响的测试、确需同步的现有默认命令说明，以及本任务完成记录。

## 要做的
New Agent 的内置 Codex 默认以 `codex --yolo` 启动。初始化和明确选择 Codex 均使用这个默认值，界面正确识别内置选项，预览与实际命令一致。高级自定义命令保持原样，Claude 及其他默认值不改。主控已用本机 `codex --yolo --version` 确认参数可用。

## 怎么算做完
> 增加新agent的时候，claude code已经默认auto mode，但是新codex会出现permission对话框，我不想要，我想直接可以以yolo mode来新建codex agent

验证预算：针对本次默认参数的定向自动检查，先确认旧默认值导致目标检查失败，再最小实现；项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，以及 `git diff --check`。命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不修改 Claude 行为、全局 Codex 配置、已存在的 agent、UI 样式或其他功能。
- 不操作真实 agent、drover 队列或其他仓库；用假 CLI/合成数据测试。
- 不按项目名或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。
- 范围外失败记录报告，不扩修。已知基线 full_workflow 旧终端鼠标坐标检查失败尚未修复。

## 做完
在本文件末尾追加完成记录并提交：做了什么、验证结果、取舍及未做事项。回复带提交 SHA 和需要主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
