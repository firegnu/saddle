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

## 完成记录（saddle/dev-name-prefix，2026-09-27）

### 改动
- `src/launch.rs`：New 表单增加 `Prefix` 输入框，默认 `agents`，与 Name 同一行放在其左侧（约占三分之一宽），不增加表单高度。Tab 顺序为 Role → Prefix → Name（Controller 时跳过只读 Name）。Controller 与 Regular 下 Prefix 都可键盘、粘贴、Ctrl-U、点击编辑；项目、工具、角色切换不覆盖前缀草稿。
- 提交与预览统一使用 `Prefix/Name`（默认 `agents/main`），仍按精确名称调用 `corral start`，不加 `--unique`；角色标签、命令、打开位置不变。
- 校验：Prefix 为空白、含 `/`、含空白或以 `-` 开头时报 `Prefix needs text, no spaces, '/' or leading '-'`（NUL 沿用通用提示）；提交失败时 `reveal_invalid` 把焦点移到 Prefix。Name 校验不变。
- 把输入框绘制提成 `draw_input`，供 Prefix 与原有四个输入框共用；Name 占位文字由 `project/my-agent` 改为 `my-agent`（前缀已单独填写）。
- 测试：新增 `prefix_defaults_to_agents_and_is_editable_for_both_roles`；现有 launch 单测与 `tests/workflow.rs` New 相关用例改为期望 `agents/...`，原先在 Name 中手写 `p/new`、`p/late`、`p/hidden` 的用例改为 Prefix 填 `p`、Name 填后半段，断言不变。
- 中英文 README 补充 Prefix 说明。

### 验证
- RED：新测试在实现前失败于 `left: "main" / right: "agents/main"`；实现后 GREEN。
- `cargo test --lib launch`：11 通过。`cargo test --test workflow new_`：5 通过。
- `cargo test --all-targets`：除已知 `full_workflow_routes_input_switches_safely_and_survives_disappearance`（旧鼠标坐标基线，未用 New 表单）外全部通过（workflow 36 过 / 1 败 / 2 忽略）。
- `cargo clippy --all-targets -- -D warnings`：通过。`git diff --check`：通过。
- 本次改动的文件 `rustfmt --check` 通过；`cargo fmt --check` 另报 `tests/ui.rs` 一处既有格式差异，非本任务改动，未动。

### 取舍
- Prefix 与 Name 并排而非单独一段，避免表单变高、影响短窗口和既有对话框尺寸。
- Name 仍允许含 `/`（沿用现有名称校验），因此 Regular 名称写 `p/new` 时完整名为 `agents/p/new`。

### 未做 / 待主控决定
- 是否要让 Name 也拒绝 `/`（避免 `agents/p/new` 这类多段名）——设计要求沿用现有校验，未改。
- 前缀草稿不跨表单持久化（关闭 New 后重新打开回到 `agents`），与设计“不增加配置”一致。
