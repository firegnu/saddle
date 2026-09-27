# 任务：右侧终端窗格显示 agent 角色

2026-09-27，saddle/main 交给 saddle/dev-role-title（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由档位拿不准，主控选常规；交叉审查不要；影响面改行为）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 34、36 节。
- src/launch.rs、src/corral.rs、src/terminals.rs、src/ui.rs 及直接相关测试。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/agent-role-title，分支 agent-role-title（从 main 建好）。
- 只修改 New 角色标签、对应窗格标题所需代码与直接相关测试、必要的现有用户说明及本任务完成记录。

## 要做的
按设计第 36 节，New 将 Controller / Regular 记录为 role=controller / role=regular 的 corral 标签；右侧标题显示 Controller / Regular 加 agent 名称，没有有效标签则显示 Agent 加名称。空窗格仍为 Viewer。使用已有公开快照，在两条标题渲染路径中保持一致，不凭名称推断身份。

## 怎么算做完
用户原话：
> 我的意思是，能否得到这个ageent的角色，把viewer换成对应的角色名。这个调查一下是不是可以做
>
> 修改范围是哪个repo？
>
> 好的，开始干吧

验证预算：角色行为的定向自动检查先 RED 再 GREEN，随后项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，以及 `git diff --check`。Cargo 命令加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不改派发流程、corral/drover 仓库、精确名称、命令默认值、打开位置、终端生命周期或输入规则；不增加角色控件、配置项、开发/审查职责或其他视觉调整。
- 不操作现有真实 agent、队列或用户配置；验证使用假 CLI/合成数据。
- 不按项目名或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。
- 已知 `full_workflow_routes_input_switches_safely_and_survives_disappearance` 旧终端鼠标坐标超时为基线问题，不扩修；范围外问题如实报告。

## 做完
本文件末尾追加完成记录并提交：改动、验证、取舍、未做事项。回复带提交 SHA 和待主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（saddle/dev-role-title，2026-09-27）

### 改动
- `src/corral.rs`：`Agent::role()` 只认公开标签 `role=controller` / `role=regular`（小写精确值），其他值、大小写不符或缺失都返回无角色；不看名字。
- `src/launch.rs`：`Form::args()` 在 `--cwd` 之后按所选角色加 `--label role=controller` 或 `--label role=regular`；预览直接用同一组参数，所以一致（预览沿用现有 shell 引号，显示为 `--label 'role=controller'`）。
- `src/ui.rs`：新增 `pane_title(showing, agents)`，在已有 `panel.agents` 快照里按名称找 agent，显示 `Controller · 名称` / `Regular · 名称` / `Agent · 名称`，空窗格为 `Viewer`。无 terminals 的渲染路径使用它。
- `src/terminals.rs`：`terminals::draw` 多接一个 `agents: &[Agent]`，每个窗格用同一个 `pane_title` 显示各自 agent 的角色。
- README / README.zh-CN：New 段落补一句角色标签和窗格标题。

### 验证
- RED（改代码前均为断言失败）：`launch::tests::role_is_passed_as_a_public_label_in_args_and_preview`（参数缺 `--label`）；`tests/ui.rs::viewer_title_shows_the_public_role_label_or_agent_without_guessing`（标题仍为 `Viewer · demo/main`）；workflow 的 `split_and_new_tab_…`、`new_agent_choices_…`、`new_agent_previews_…`（标题/参数不符）。
- GREEN：上述定向检查全部通过。
- `cargo test --all-targets`：除已知基线 `full_workflow_routes_input_switches_safely_and_survives_disappearance` 外全部通过；该用例仍在 tests/workflow.rs:293 等待鼠标事件 `input p/a 1b5b3c303b333b324d` 超时，与任务文件记载的旧终端鼠标坐标基线一致，未扩修。
- `cargo clippy --all-targets -- -D warnings`：通过。`git diff --check`：通过。

### 测试侧改动
- 假 `corral`（tests/fixtures/corral.py）：`status` 返回可选的 `labels.json` 中该 agent 的标签；`start` 把 `--label` 记进 `labels.json`。只影响测试临时目录。
- workflow：分屏用例给 p/a、p/b 设 controller/regular，断言两个窗格分别显示 `Controller · p/a`、`Regular · p/b` 且无 Viewer；New 用例断言 `Controller · main-actual`、`Regular · a文中-actual` 以及实际 start 参数带角色标签；其余无标签 agent 的断言由 `Viewer · p/…` 改为 `Agent · p/…`。
- `tests/terminals.rs` 的 `terminals::draw` 调用补传 `&[]`。

### 取舍
- 角色来自 `corral status` 已返回的 labels（与 effort 同一条快照路径），没加轮询或配置。标签值只接受精确小写，避免猜测。
- 底部状态栏的 `Viewer · disconnected` 提示不是窗格标题，没改。

### 未做
- 未合并、未推送、未构建 release；未操作任何真实 agent。
- 已有 agent 不会补标签，它们显示 `Agent · 名称`，符合设计第 36 节。

## 主控审查（2026-09-27）

- 审查通过：核对 d755024 完整 diff、完成记录与回复；改动限于 Saddle 的 New 标签、两条标题渲染路径及相关测试/说明，未改派发、终端生命周期或现有 agent。
- 同意实现取舍：只认精确小写的公开角色标签；旧 agent/未知角色回退 Agent，空窗格保留 Viewer；底部 Viewer · disconnected 为连接提示，维持原样。不另开基线修复任务。
- 主控在最终开发提交上运行一次项目标准检查：145 passed、1 failed、2 ignored；Clippy 与 diff 检查通过。唯一失败为已知 full_workflow 旧鼠标坐标超时（断言入口 tests/workflow.rs:245），没有新增失败，不宣称全套全绿；角色参数、预览、新建后的角色标题和多窗格检查均通过。
- 没有阻挡项，合并发布；已有 agent 不补标签，用户重启 Saddle 后通过 New 新建的 agent 才会记录角色。
