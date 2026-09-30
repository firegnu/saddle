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
