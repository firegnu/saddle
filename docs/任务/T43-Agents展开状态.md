# T43：修复派发新 agent 后 Agents 意外全部收缩

2026-09-29，saddle/main 交给 Codex，常规 gpt-6-astra / high。
路由：常规 / 交叉审查不要 / 影响面：改行为（采纳 JEV 三项 verdict）。
你是被委派的实现者，照本文件做，不再委派。用户已明确「确认放行」，此前仅创建待办的限制结束。

## 用户原话与验收

「再加一个任务，当左侧agents的某一个主控agent委派出一个agent的时候，左侧的agents区域会全部收缩起来，需要我按下z才能再次展开」。

目标：修复派发新 agent 时左侧 Agents 意外全部收缩的问题，保留用户原有的展开状态。

## 先读与位置

- AGENTS.md、docs/DESIGN.md §40（Agents 折叠）、§58（本次修正）。
- src/agents.rs 的 absorb／folded／toggle_fold，以及 tests/agents.rs、tests/ui.rs 中直接相关检查；按需核对 src/app.rs 的 z 入口。
- worktree：`/Users/firegnu/Developer/personal_projs/saddle-worktrees/t43-agents-fold`，分支 `t43-agents-fold`。只改 Agents 折叠状态及直接相关检查、文档。

## 要做的

先核实并复现根因，再最小修复。主控只读线索：`Panel::folded()` 在 fold=None 时每次按 agents.len()>5 返回；从五个增加到六个会改变展示。不要将此线索冒充已经完成的复现。

保留启动时按初始成功列表数量决定默认折叠的规则；随后列表刷新、新增或退出 agent 不再改变已有展开／折叠状态。保留 z 手动切换和已有选中项展开规则，不增加持久化、配置或新控件。设计已在 §58 更新。

## 验证预算

先写或调整一项针对缺陷的自动检查，确认因目标行为失败，再最小实现到 GREEN；跑直接相关回归。标准 `cargo test --all-targets` 与 `cargo clippy --all-targets -- -D warnings` 各一次，`git diff --check`。使用共享 CARGO_TARGET_DIR，仅合成数据／假 CLI；不加覆盖矩阵、录屏或变异检查。不顺手调查或修复无关 T29 测试失败，如实记录即可。

## 不要做与完成回复

不改变布局、配色、排序、选中、agent 生命周期或队列流程；不改 corral、drover、dispatch-log 或全局技能，不操作真实队列／用户 agent，不处理 T29，不清理历史 t38-dispatch-study。不按项目名批量杀进程。不合并、不推送，只在本分支提交。

本文件末尾追加完成记录：根因、修改、检查、取舍与待决定事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

主控 dispatch_id：`29308f5afca7415dbb5a10cbd9edf606`；实现者无需采集。

## 完成记录（2026-09-29）

- 根因已复现：旧 `folded()` 在 `fold=None` 时持续按当前列表数量计算。将既有检查改为先载入 5 个、再载入 6 个合成 agent，要求继续展开；修改实现前，检查在第二次载入后的 `assert!(!panel.folded())` 失败（退出码 101），不是编译或夹具错误。
- 最小修改：首次 `absorb` 用初始列表数量填入尚未设置的 `fold`，之后保留该值；`folded()` 不再读取当前数量。应用仅对成功列表调用 `absorb`，失败不会初始化默认状态，成功的空列表会选择展开。已有手动选择不被覆盖，z 入口和选中项展开规则未改。
- 检查调整：保留 5→6→8 的展开状态、手动折叠／展开后刷新；补充初始 6 个随后退出、初始空列表随后新增的回归。UI 的默认折叠夹具改为首次载入大列表，effort 绘制检查显式切换折叠。
- 所有 Cargo 命令均使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，前台运行并等待退出，仅使用合成数据／假 CLI。
  - RED→GREEN：`cargo test --test agents fold_keeps_initial_choice_across_agent_count_changes_until_toggled`，实现前失败，实现后 1 项通过。
  - 直接回归：`cargo test --test agents --test ui`，7 + 45 项全部通过。
  - 标准测试（一次）：`cargo test --all-targets`，退出码 101；前面的测试目标全部通过，workflow 为 78 通过、1 失败、3 忽略。唯一失败为 `wheel_over_agents_scrollbar_reaches_last_agent_without_attaching`，在 `tests/workflow.rs:951` 等待 `worker-07` 出现时失败。当时未核实该失败就将其归入无关 T29，此归类不准确；它直接涉及本次展开状态变化，已按下方复修记录处理。未重跑全量测试。
  - 标准静态检查（一次）：`cargo clippy --all-targets -- -D warnings` 通过。
  - `git diff --check` 通过。
- 取舍：复用原有 `Option<bool>` 表示尚未收到成功列表，无新增状态字段、配置或持久化；遵循 DESIGN §58，无设计变更。
- 首轮遗留：上述滚动检查需核实，现已按下方复修记录修正。本分支提交，不合并、不推送。

## 主控审查复修（2026-09-29）

- 只读主仓库 `docs/任务/T43-主控审查.md` 后，在本 worktree 修正直接相关的滚动检查；核心 `src/agents.rs` 最小修复保持不变，未修改主仓库审查文件。
- 根因核实：在测试的 140×40 终端里，Agents 列表从零基 y=4（终端第 5 行）开始，滚动条在零基 x=50。原鼠标序列的终端第 4 行转换为 y=3，命中头部分隔线，未落入 `hits.list` 扩展一列后的滚动区。旧自动折叠让末项无需滚动就可见，掩盖了检查的错误坐标；不能归为既有偶发失败或无关 T29。
- 修改：从实际显示的 `worker-00` 行确定鼠标 y，保留滚动条列与正文列两种入口；先断言展开且末项不可见，再等待末项五行完整到达底部及滚动条滑块到达末行。验证刷新后末项坐标不变、首项仍不可见，回顶后首项恢复原行且末项不可见，再从正文滚动到底，最后检查没有 attach。等待完整末项避免把刚进入视口的中间帧误当最终位置。
- 验证：所有 Cargo 命令仍用共享 `CARGO_TARGET_DIR`，前台运行并等到结束，仅合成数据／假 CLI。
  - `cargo test --test workflow wheel_over_agents_scrollbar_reaches_last_agent_without_attaching -- --exact`：修正后 1 项通过；沿用首轮该项失败作为修正前证据。
  - `cargo test --test agents`：7 项通过。
  - `cargo test --test ui fold`：2 项通过。
  - `cargo test --test ui agents_scrollbar_reaches_the_bottom_when_the_last_row_is_visible -- --exact`：1 项通过。
  - `cargo test --test workflow fold_toggles_by_key_and_bar_and_background_tab_agents_keep_the_local_mark -- --exact`：1 项通过。
  - `rustfmt --edition 2024 --check tests/workflow.rs`、`git diff --check`：通过。
- 取舍与待决定事项：正常列表命中下滚动工作正常，仅修正测试坐标和等待／断言，无产品代码或布局修改；本轮未重跑全套或 clippy，未加变异检查，未扩展其他失败。复修范围内无待决定事项，提交后交主控审查。
