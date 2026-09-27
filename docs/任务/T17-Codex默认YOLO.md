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

## 完成记录（2026-09-27）

- 实现：`src/launch.rs` 用同一个 `codex --yolo` 默认命令供初始化、明确选择 Codex、内置选项识别使用；预览继续从实际 argv 生成。同步 README 默认命令说明及受影响的假 CLI 断言。
- 定向 RED：先增强现有 `default_project_and_codex_can_create_without_typing_a_command` 检查，再运行 `cargo test --lib launch::tests::default_project_and_codex_can_create_without_typing_a_command -- --exact`，退出 101；实际 argv 以 `codex` 结束，期望为 `codex --yolo`，确认为目标缺陷。
- 定向 GREEN：最小实现后原命令退出 0，覆盖初始化、从 Claude 切回 Codex、内置选中标识、无 Custom command 误标及完整预览。
- 标准测试（各一次）：`cargo test --all-targets` 退出 101；所有其他测试通过，workflow 为 36 通过、1 失败、2 忽略。唯一失败是任务已注明的 `full_workflow_routes_input_switches_safely_and_survives_disappearance`：旧坐标鼠标输入后未出现期望的 `input p/a 1b5b3c303b333b324d`。本次默认创建、编辑名称后创建、Claude 默认、自定义命令预览和失败草稿检查均通过。`cargo clippy --all-targets -- -D warnings` 退出 0。
- 格式及差异：`cargo fmt --check` 首次提示本次新增代码的换行，已运行 `cargo fmt` 修正；最终 `git diff --check` 通过。所有 Cargo 命令均使用规定的共享 `CARGO_TARGET_DIR`，所有命令在前台等待结束。
- 取舍及未做事项：只修改内置默认值，不在提交阶段追加参数；高级自定义命令与 Claude 行为保持原样。设计第 33 节已明确授权，无需再改设计。未操作真实 agent、队列、全局配置或其他仓库；未合并、推送或构建 release。已知基线失败未扩修，交由主控另行安排；无新增设计决策待确认。
