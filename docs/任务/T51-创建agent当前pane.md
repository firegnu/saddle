# T51：创建 agent 时允许选择当前 pane

2026-09-30，saddle/main 交给 saddle/dev-t51-current-pane（Claude Code，常规 opus[1m] / high；实际实例名以 corral 返回为准）。
路由：常规 / 交叉审查不要 / 影响面：改行为（JEV 三项一致，采纳）。
类型：功能变更
依据：旧设计故意锁定 Viewer 入口位置，本轮按用户明确需求调整已有创建表单交互。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 5、27、39、43 节及创建/替换窗格直接相关说明。
- src/launch.rs 的 focus_order、key、draw 和 bound_location 测试。
- src/app.rs 的 placement::Choice::NewAgent、Control::CreateAgent、New 表单提交路径；src/app_control.rs 的既有替换确认。
- tests/workflow.rs 的 New agent、位置选择、替换和过期目标用例。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t51-current-pane
- 分支：t51-current-pane，基线 1415472。
- 范围：本问题直接相关 src、tests、docs/DESIGN.md 与本任务书。不要改上游仓库。

## 要做的
用户原话：“我在saddle中创建agent的时候，还是无法选择在当前的pane中打开。如下图”。
用户已补截图，显示 Open in 下为“Opens in a new tab · set by + Tab”，然后明确“开始这个任务吧”。这覆盖旧队列正文“仅登记待办”的限制。

主控已核对：Viewer + Tab/Split 的 New agent 设置 form.anchor 与默认 form.place；launch.rs 因 anchor 存在隐藏选择器并跳过该焦点。Agents New 已有六位置选择。问题不是无法启动 agent，而是入口把位置锁死。

本轮结果：创建表单可以选择在当前 pane 打开。沿用已有 Open in 选择交互，入口提供默认位置；保留发起 pane 的身份绑定、失效检查和既有替换确认，当前 pane 指发起创建的 pane，不因异步焦点改变而换目标。默认仍来自 + Tab 或 Split 的原始位置。不要为了开放位置选择直接清空 anchor，它还负责来源校验、草稿恢复与取消行为。保持创建参数和 agent 生命周期行为不变。

先更新 DESIGN.md 对本次规则的说明，标明 T51 替代旧 T27 的位置锁定呈现。用最小修改复用现有控件；不重新设计表单，不添加新的设置或全局快捷键。占位窗格入口沿用既有默认当前位置；如果发现调整会涉及不可兼容设计或需改上游，停下报告最小影响。

## 怎么算做完
用户原话：“我在saddle中创建agent的时候，还是无法选择在当前的pane中打开。如下图”。
用户启动指令：“开始这个任务吧”。

验证预算：先加目标行为自动检查，运行确认当前实现因位置不可选而失败，再做实现并取得 GREEN；直接相关回归；cargo test --all-targets、cargo clippy --all-targets -- -D warnings 各一次；git diff --check。所有 Cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。测试使用临时 HOME/状态和假公开 CLI。标准套件疑似无关失败可单独复跑一次，保留原始失败，不据此宣称修复 T29；预算不足先报告。

## 不要做
- 不处理 T29、T53、T54、T55，不扩大到其他创建功能。
- 不操作真实用户 agent，不操作真实队列、布局、Loop/Gate；不读私人会话或 Corral/Drover 内部文件。
- 不创建真实测试 agent、不重启或替换用户运行中的 Saddle。
- 不按名称/路径批量杀进程，测试进程只按记下的 PID 清理。
- 不合并到 main，不推送；只在本分支提交，不改 HANDOFF.md。

## 做完
在本任务书末尾追加完成记录并提交：改动、RED/GREEN/回归结果、取舍、未验证边界。所有命令在前台跑完。回复列出这些与需要主控决定的事，全部完成后最后一行写 DONE。

## 完成记录（2026-09-30，saddle/dev-t51-current-pane）

改动：
- docs/DESIGN.md：新增 §60 T51，§43 标题注明位置锁定呈现由 §60 替代。
- src/launch.rs：去掉 `anchor.is_some()` 时隐藏 Open in 选择器、跳过 Tab 焦点、屏蔽 F5／←→ 的四处条件，删除只供静态说明用的 `bound_place`。`+ Tab`／Split／占位窗格入口仍把各自位置写进 `form.place` 作为默认值，`anchor` 原样保留。提交路径（`src/app.rs`）、既有替换确认与失效检查均未改：绑定表单提交时 anchor 仍为发起窗格，Current 时运行中的 shell 仍走 `request_replace` 确认。
- 测试：launch 单测 `bound_location_is_static_text_and_new_keeps_the_selector` 改为 `bound_location_defaults_the_selector_and_can_choose_the_current_pane`（入口默认值显示在选择器上、Tab 可达、←选到 Current pane、anchor 不变）；tests/workflow.rs 新增 `t51_location_new_can_choose_the_originating_pane_with_existing_confirmation`（shell 窗格点 + Tab → New agent… → Open in 改为 Current pane → 提交 → 既有 End shells 确认 → agent 在同一窗格 id 启动，tab 数与活动窗格不变，无 stop）。

RED／GREEN／回归：
- RED：两条新检查在实现前均失败，画面显示 `Opens in a new tab · set by + Tab`，没有选择器（单测断言失败；集成测试 `see("Open in: New tab")` 超时）。
- GREEN：两条均通过。直接相关回归：launch 单测 13 passed；workflow 中 new_agent／placement／t20_rework／t25_placeholders／replacing_a_running_shell／split_and_new_tab／starting_in_a_hidden 共 20 passed。
- `cargo test --all-targets`：293 passed、2 failed、3 ignored。失败为 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal`（2501 行等 SHELL READY 超时）与 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases`（3232 行等 Create agent 超时）；单独复跑一次 2 passed。与 HANDOFF 所记 T29 偶发等待超时同类，保留原始失败，不据此宣称修复 T29。
- `cargo clippy --all-targets -- -D warnings` 通过；`git diff --check` 通过。

取舍：
- 选择器文案沿用 Agents New 的 `Open in: <位置> (←/→)`，不再显示「set by + Tab」来源说明，未加新文案或控件。
- Current pane 在绑定入口下指发起窗格（`+ Tab` 时的活动窗格、Split 或占位窗格本身），与原有提交路径的 anchor 解析一致，未新增逻辑。

未验证边界：
- 未在真实终端和真实 corral 下手测；只用假公开 CLI 与临时 HOME 的自动化测试验证。
- 发起窗格是已接入 agent（非 shell）时选 Current，会沿用 Agents New 选 Current 的既有替换行为（断开显示、不停止 agent），本轮未另加集成测试。

## 主控审查（2026-09-30）

结论：实现 3081ce2 可以合并。差异仅移除位置选择被 anchor 禁用的条件及失去用途的静态文案，沿用原选择器；anchor、提交目标校验、草稿恢复与 shell 替换确认未改。目标流程检查通过，验证 + Tab 创建改选 Current 后仍在原 pane，tab 数不变且不调用 agent stop。

取舍：同意不追加入口来源文案，选择器已有当前位置说明；保留既有 agent 显示替换语义；不扩大 T29，不增加额外真实 agent 测试。真实终端手动验收仍待用户重新打开新版 Saddle。

主控标准检查一次：cargo test --all-targets 为 294 passed、1 failed、3 ignored，T51 检查通过。失败 t20_r1_pending_new_pane_keeps_known_source_cwd_for_shell 于 tests/workflow.rs:3153 读取已有 close confirmation 字段时 unwrap(None)；该处不在本次 diff 内，也未在本例修改 Open in。按预算单独复跑一次通过（1.23 秒），未确认该偶发失败根因，不把全套写成全绿。实现者原始两条 workflow 失败及单独复跑通过保留。cargo clippy --all-targets -- -D warnings 与 git diff --check main...t51-current-pane 通过。全部使用隔离输入与共享编译目录。

本次日志：/tmp/saddle-t51-controller-tests.log、/tmp/saddle-t51-controller-rerun.log、/tmp/saddle-t51-controller-clippy.log。没有证据表明原有失败由这次选择器改动引入；本轮接受目标检查及相关回归结果，不宣称 T29 修复。
