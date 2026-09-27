# New Agent：默认 main 按精确名称创建

2026-09-27，saddle/main 交给 saddle/dev-exact-name（Codex，常规档：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由档位拿不准，主控选常规；交叉审查不要，影响面改行为）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 34 节。
- src/launch.rs 及直接相关现有测试。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/new-agent-exact-name，分支 new-agent-exact-name（从 main 建好）。
- 只动 New 的名称参数与名称说明、直接受影响的测试及中英文 README、本任务完成记录。

## 要做的
New 保留默认 main 时，按精确名称 main 创建，而不是自动编号 main-1。按设计第 34 节取消 New 对未手改名称追加 --unique 的旧规则，默认与手工名称使用一致的精确名称语义。重名沿用公开 CLI 失败提示和草稿保留，不新增重试或改名策略。只影响 New 入口。

## 怎么算做完
> 现在有一个问题，new agent的时候，如果不改默认的main，但是出来的agent的名字是main-1.你来调查一下。
>
> 关键是现在有些工作流依赖主控的名字是main

验证预算：本次名称行为的定向自动检查先 RED 再 GREEN，项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，`git diff --check`。命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不改变派发流程、Codex YOLO 默认、Claude 默认、打开位置、UI 样式或其他功能。
- 不操作现有 main-1 或任何真实 agent、队列和其他仓库；测试使用假 CLI/合成数据。
- 不按项目名或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。
- 已知 full_workflow 旧终端鼠标坐标失败不扩修；其他范围外问题记录报告。

## 做完
本文件末尾追加完成记录并提交：改动、验证、取舍、未做事项。回复带提交 SHA 和待主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（2026-09-27）

- 根因与改动：`Form::args()` 对未手改名称追加 `--unique`，使默认 `main` 交给 CLI 自动编号。现已删除这处分支，默认和手工名称都精确传递；名称提示统一为 `Exact name · edit freely`。同步中英文 README，修正旧的项目/agent 建议名说明。
- 测试改动：现有默认名称测试验证精确 argv、预览与名称提示；项目/Claude 切换测试和假 CLI 工作流的默认提交预期移除 `--unique`。保留手改名称保护和公开返回名处理的原有检查。
- 验证：以下 Cargo 命令均加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，逐条在前台等待结束。
  - 初次误用 `cargo test --bin saddle launch::tests::default_project_and_codex_create_exact_main_without_typing_a_command -- --exact`，运行 0 个测试，不计作验证。
  - RED：`cargo test --lib launch::tests::default_project_and_codex_create_exact_main_without_typing_a_command -- --exact`，1 个失败；实际 argv 多出 `--unique`，符合目标缺陷，退出码 101。
  - GREEN：`cargo test --lib launch::tests::`，6 个通过，包括默认精确名称、预览及手工名称保护。
  - `cargo test --all-targets`，执行一次，退出码 101；此前各测试目标通过，workflow 为 36 通过、1 失败、2 忽略。默认 New 的假 CLI 提交、手工名称、预览及失败草稿保留测试通过。唯一失败为任务已注明的 `full_workflow_routes_input_switches_safely_and_survives_disappearance`：旧鼠标坐标发送后，等待 `input p/a 1b5b3c303b333b324d` 超时；未扩修或重跑全套。
  - `cargo clippy --all-targets -- -D warnings`，执行一次，通过。
  - `git diff --check`，通过。
- 取舍与未做：遵循设计第 34 节，无需变更设计；沿用现有 CLI 失败反馈与草稿保留，不新增重试、编号或接入已有 agent 的策略。不改派发、命令默认值、打开位置或 UI 样式；未操作真实 agent/队列、未构建 release、未合并或推送。
- 待主控：审查本分支，决定后续合并；已知 full_workflow 旧鼠标坐标问题另行安排。
