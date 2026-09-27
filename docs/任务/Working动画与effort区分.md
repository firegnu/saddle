# Working 动画与 effort 区分

2026-09-27，saddle/main 交给 saddle/dev-working-mark（Claude Code，轻档 sonnet / medium，role=implementer）。
路由：轻 / 交叉审查不要 / 影响面：看得见（route.py：tier拿不准、cross_review不要、impact看得见；主控定轻：删除一处既有动画，视觉规则已经明确）。
你是被委派的 agent，不再委派。

## 先读

AGENTS.md、docs/DESIGN.md 第40节末尾「Working 动画与 effort 的视觉区分」，src/ui.rs 状态行绘制、tests/ui.rs 相关渲染断言。

## 范围与目标

worktree /Users/firegnu/Developer/personal_projs/saddle-worktrees/working-mark；分支 working-mark。
仅改 src/ui.rs、直接受影响的 tests/ui.rs 与 README 中英文及本任务完成记录。去掉 working 文字前的紫色盲文动画，保留首列蓝色圆点旋转和 working 文字。effort 原位置、字形、颜色、未知留空和折叠显示保持，状态列与时间列仍对齐；无需删配置项或顺带重构。

## 用户确认

「我确认了。就是那个紫色的动画引起的。不working的时候，就正确显示effort了」。此前主控建议去掉 working 前的紫色动画，保留最左侧转动圆点，effort 位置和样式不动；按 DESIGN 第40节执行。

## 验证预算

纯视觉：git diff --check，加直接观察上述显示结果的渲染检查；运行直接受影响的 UI 检查即可，不伪造 RED，不跑全套/交叉审查/录屏/缺陷植入。共享 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target，命令前台完成。若旧动画专属断言需要调整，保留圆点旋转及 effort 的原断言，不删相邻无关检查。

## 边界

不改 agent 生命周期、输入、状态判定、模型参数或 effort 解析；不新增模型名显示。不改原稿、DESIGN、其他代码/测试或配置；不操作用户 agent/现场TUI/socket/真实队列，不读 corral/drover 内部文件，不改其他仓库。不批量杀进程。不 merge/push/release，不写 main，只在本分支提交。

## 做完

本文件末尾追加完成记录，写改变、验证与取舍，提交并回复 SHA。全部命令前台结束，回复最后一行 DONE。

## 完成记录（2026-09-27）

**改变**：`src/ui.rs` 状态行绘制处（`entries` 函数）删掉 working 状态文字前的紫色盲文旋转帧（`⣾⣽⣻⢿⡿⣟⣯⣷` 循环及其 `agents_purple` 着色和前置空格），只保留其后的 `working` 文字本身。首列蓝色圆点旋转（`◐◓◑◒`，360ms 一帧）、effort 两格信号柱（位置、字形、颜色、未知留空、折叠显示）均未改动。

`tests/ui.rs`：
- `agent_states_have_the_designed_dots_colors_labels_and_activity` 删掉紫色盲文专属断言（原来的圆点旋转与颜色断言保留）。
- `working_spinners_advance_at_their_own_rates`（原用于验证盲文 120ms 与圆点 360ms 两种独立节奏）整体重写为 `working_dot_advances_every_360ms`，只保留圆点自身节奏的断言，删掉盲文取样逻辑。
- `design_sample_fits_fifty_columns_without_wrapping` 的 50 列期望行去掉 `⣽ ` 前缀。
- `forty_two_columns_show_agent_icons_and_keep_columns_aligned`、`effort_keeps_its_first_row_slot_when_folded_and_narrow` 里判断状态列首字符的断言从"`i` 或盲文字符集"改为"`i` 或 `w`"（working 文字直接从 `w` 开始，不再有盲文前缀）。

`README.md` / `README.zh-CN.md`：working 状态说明里去掉"with a braille spinner" / "带盲文转圈"。

**验证**：`cargo build`；`cargo test --test ui`（39 passed）；`cargo test --all-targets`（全部通过，含之前失败的三个用例修好后）；`cargo clippy --all-targets -- -D warnings`（无警告）；`git diff --check`（无空白问题）。命令均加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，前台跑完。

**取舍**：没有按原计划只"调整"`working_spinners_advance_at_their_own_rates`，而是整体重写为单一职责的圆点节奏测试——原测试的核心断言（盲文与圆点节奏不同）依赖已删除的动画，无法保留原样；重写后仍验证圆点在 360ms 边界内不变、跨边界变化，覆盖面不缩水。未改动效果柱（effort bars）用到的相似盲文字符集（`⣴⡇`/`⣴⡀`/`⣄⡀`），那是独立信号，不在本任务范围内。
