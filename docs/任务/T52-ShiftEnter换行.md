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

## 完成记录

2026-09-30，实施于 `t52-shift-enter`，基线 `07f7dd1`。

### 根因与直接依据

- 已查证的缺陷在外层输入采集：`TerminalGuard` 原来只开启备用屏幕、鼠标和 bracketed paste，没有请求键盘修饰键消歧。按 [Kitty 键盘协议的 legacy C0 表与增强模式](https://sw.kovidgoyal.net/kitty/keyboard-protocol/)，未开启增强时 Shift+Enter 与 Enter 均可编码成 CR；Shift 在进入 Crossterm 前已经丢失，后续 `encode_key` 无法恢复。
- 先新增真实二进制 PTY 检查 `shift_enter_survives_outer_terminal_negotiation_and_viewer_pty`：复用临时 HOME/状态目录及假 CLI，独立配置的 Alacritty 终端模型解析 Saddle 的实际输出，再按外层模式发送合成 Shift+Enter，经 Crossterm、Viewer、内层 PTY，检查假 `corral attach` 收到的字节。修复前两次有效运行均失败：实际 `0d`，预期 `1b5b31333b3275`（`CSI 13;2u`），直接复现“换行键变成提交字节”。最初编写测试时有一次泛型类型推断编译错误，修正后才取得上述 RED；编译错误不计作 RED。
- RED 后比较三项可证伪原因：外层未请求增强、Crossterm 丢修饰键、Viewer/PTY 重编码丢修饰键。对照测试直接向外层 PTY 写入 `CSI 13;2u`，修复前即成功送达假客户端；同一对照还确认 CR、LF、Ctrl/Alt+Enter、Tab、粘贴及 Ctrl-] 路由。结合启动输出未开启增强，定位到第一项。
- 推测：用户真实外层终端可能在客户端直接运行时接受了增强请求，而在 Saddle 中缺少该请求，因此表现不同。本任务没有采集用户终端协议流，不能把这一推测写成真实环境的已证实根因。

### 改动与取舍

- `src/app.rs`：进入备用屏幕后 push `DISAMBIGUATE_ESCAPE_CODES`，退出备用屏幕前配对 pop。主屏幕和备用屏幕有独立的模式栈，次序保证恢复正确的栈。
- `tests/workflow.rs`：新增上述协商链路测试，并验证退出时主屏幕、备用屏幕各自的预存模式均恢复；新增字节透传对照，覆盖 Shift+Enter、普通 Enter、Ctrl/Alt+Enter、LF、增强 Ctrl-C/Shift-Tab/Ctrl-]、普通 Tab/Shift-Tab 和多行粘贴。聚合假客户端的输入日志，避免依赖 PTY 每次 read 的分块方式。
- `docs/DESIGN.md` 第 5 节记录输入采集修复和终端支持边界，落实已批准的透传约定，没有改变按键设计。未修改 `src/input.rs`、内层 `Screen` 或 PTY 传输逻辑，也未新增依赖。
- 仅请求修饰键消歧，不请求按键释放、关联文本等额外模式，也不启用内层完整 Kitty 协议，以保持修复范围最小。不支持增强且只上报 CR 的外层终端无法区分两种按键；已有显式 CSI-u 或 LF 绑定继续走原路径。

### 验证结果

所有命令均以前台进程运行并等待结束；以下 Cargo 命令均使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

- RED：`cargo test --test workflow shift_enter_survives_outer_terminal_negotiation_and_viewer_pty -- --exact --nocapture`，两次有效运行都是 `0 passed; 1 failed`，因 `0d != 1b5b31333b3275` 失败。
- 修复前对照：`cargo test --test workflow viewer_preserves_modified_enter_bytes_and_legacy_input -- --exact --nocapture`，`1 passed`。
- GREEN：同一目标命令，`1 passed`；包含退出恢复断言。
- 直接相关回归：`cargo test --test input --test workflow enter -- --nocapture`，输入编码测试 `1 passed`，两项 PTY workflow 测试 `2 passed`。
- `cargo test --all-targets` 按预算运行一次，退出码 101：已运行结果合计 `293 passed; 1 failed; 3 ignored`。其中 workflow 为 `80 passed; 1 failed; 3 ignored`，新增两项均通过。原有 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal` 在点击新 tab 的 Terminal 后等待 `SHELL READY` 超时，画面仍停在选择器；疑似无关偶发问题，未证实其根因。
- 按允许预算单独复跑该失败项一次：`cargo test --test workflow terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal -- --exact --nocapture`，`1 passed`（2.29 秒）。保留原失败，不将复跑通过写成完整套件全绿，也不据此宣称修复了选择器问题；未扩展修改。
- `cargo clippy --all-targets -- -D warnings` 按预算运行一次，退出码 0。
- `cargo fmt --all -- --check`、`git diff --check` 通过。
- 本机原始标准检查日志保存在 `/tmp/saddle-t52-validation.9afpaB/`：`cargo-test-all-targets.log`、`picker-rerun.log`、`cargo-clippy.log`，供主控本次审查使用；临时路径不作为长期工件。

### 未验证边界与主控事项

- 验证涵盖真实 Saddle 二进制、Crossterm、Viewer、内层 PTY 和假公开 CLI；外层终端为协议模型，假客户端仅记录字节。因此没有宣称真实 Claude Code/Codex 多行提示框验收通过。
- 未操作任何真实 agent、私人会话、真实队列/布局或运行中的 Saddle，也未替换用户二进制；未修改 Corral/Drover，不合并、不推送。
- 主控审查时需保留上述全套原失败记录，并在后续获准的真实终端验收中确认用户所用外层终端及两个客户端的多行输入。当前范围未发现需要上游修改或改变已批准设计的接口缺口。

## 主控审查（2026-09-30）

结论：实现 3458e1e 可以合并。代码仅为 TerminalGuard 配对增加外层修饰键消歧的 push/pop，位置位于备用屏幕进入后、退出前；维持既有 Viewer 编码与路由。协议模型驱动真实二进制/PTY 的失败复现及修复前直送编码对照，支持外层协商缺失这一代码缺陷；不能据此宣称已捕获用户终端的真实根因。

同意只启用 DISAMBIGUATE_ESCAPE_CODES、不扩大内层协议实现的取舍；同意不支持增强且只提供 CR 的外层终端仍无法区分 Shift。同意保留无关 picker 首轮失败并不扩展 T29。

主控复核：cargo test --all-targets 一次通过（294 passed、0 failed、3 ignored），cargo clippy --all-targets -- -D warnings 一次通过，git diff --check main...t52-shift-enter 通过。使用共享编译目录；原始日志 /tmp/saddle-t52-controller-tests.log 和 /tmp/saddle-t52-controller-clippy.log。实现者全套为 293 passed、1 failed、3 ignored，picker 单独复跑通过，保留该历史结果，主控本次通过不证明 T29 根因已修复。

真实 Claude Code/Codex 输入框尚未验收；编译交付后须用户重开 Saddle 验证当前终端下的 Shift+Enter。未操作真实用户 agent 或上游仓库。
