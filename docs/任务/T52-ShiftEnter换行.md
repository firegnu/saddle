# T52：右侧 agent 中 Shift+Enter 无法换行而直接发送

2026-09-30，saddle/main 交给 saddle/dev-t52-shift-enter（Codex，重档 gpt-6-astra / xhigh；实际实例名由 corral 返回）。
路由：重 / 交叉审查不要 / 影响面：改行为（JEV：重、不要、改行为，采纳）。
类型：Bug 修复
依据：用户已明确授权“排查并委派修复，这是block级别的。”，覆盖原队列正文“仅登记待办”的旧限制。
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 5 节、第 9 节及与本次输入透传直接相关的段落。
- src/input.rs、src/app.rs 的 TerminalGuard 和 Viewer 输入路径、src/terminal.rs、src/pty.rs。
- tests/input.rs、tests/workflow.rs 的现有 PTY Harness、tests/fixtures/corral.py。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t52-shift-enter
- 分支：t52-shift-enter，从 main 07f7dd1 建立。
- 只修改本问题直接需要的 src、tests、docs/DESIGN.md 和本任务文件；不顺带处理 T51、T53、T29。

## 要做的
用户原话：“现在右侧打开的agent。shift+enter都不能多行输入了，而是一旦按下shift+enter就会直接发出去，claude code和codex都一样。他们在saddle外运行是没问题的。”

排查并修复上述阻塞问题，保持设计已约定的 Viewer 按键透传行为。先建立能复现实际输入链路缺陷的自动检查，确认因目标缺陷失败后，再最小修复。记录已查证事实、推测和未验证边界，不能把浅层编码测试通过写成真实客户端通过。若需要改变现有批准的设计或修改上游，停下报告最小接口缺口和影响，不自行扩大范围。

主控已查证：src/input.rs 已把带 SHIFT 的 Enter 编为 CSI 13;2u；现有 modified_enter_is_distinct_from_submit_for_multiline_prompts 单测通过（1 passed），它直接构造 KeyEvent，尚未复现用户问题。可从外层按键采集、Viewer 路由与内层终端协议之间建立复现；以上只是定位入口，不是根因结论。诊断先取得失败信号，再比较可证伪的原因，不预先锁定某一种修复。

## 怎么算做完
用户原话：“排查并委派修复，这是block级别的。”
问题原话：“现在右侧打开的agent。shift+enter都不能多行输入了，而是一旦按下shift+enter就会直接发出去，claude code和codex都一样。他们在saddle外运行是没问题的。”

验证预算：目标检查 RED→GREEN 与直接相关回归；cargo test --all-targets、cargo clippy --all-targets -- -D warnings 各一次；git diff --check。所有 Cargo 命令使用 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。使用合成输入、临时 HOME/状态目录和假 CLI，不操作真实用户 agent。标准套件疑似无关失败可单独复跑一次，保留原失败，不据此宣称根因修复；觉得预算不足先报告原因。

## 不要做
- 不读 Corral/Drover 内部文件，不修改它们的仓库；仅用公开 CLI。
- 不向用户 agent 发送任何输入，不停止或重启用户进程，不创建真实测试 agent；如必须真实客户端才能确认则报告所缺证据。
- 不读私人会话日志，不操作真实队列、Loop、Gate 或真实 Saddle 布局，不替换或重启用户正在运行的 Saddle。
- 不按项目名或路径批量杀进程；自己启动的测试进程仅按记下的 PID 清理。
- 不重构无关模块，不调模型配置，不处理偶发测试独立问题。
- 不合并到 main，不推送，只在本分支提交。

## 做完
在本文件末尾追加“## 完成记录”并提交：根因与直接依据、改动、RED/GREEN 与回归结果、取舍、未验证边界。命令都在前台跑完。回复只写这些和需要主控决定的事项，全部完成后最后一行写 DONE。
