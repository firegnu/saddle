# 进度

## 第 1 步：实现完成，待真实终端目视验收

2026-09-25，在 main 开发；未推送，未修改 corral/drover 仓库。用户明确要求不启动真实 agent，因此原型和所有自动测试只使用合成流、临时目录中的假 corral/queue。

已完成：

- Rust stable 项目、默认/自定义 TOML 配置、参数校验、三窗格布局与缩放。
- 原生 Agents：每秒后台 ls/status、按项目前缀分组、项目内按状态排序、完整字段和窄屏身份折行、状态/未读标记、选中名字保持、滚动条与上下隐藏数量。
- reply 默认隐藏，开启后按需请求、分页和滚轮；x/y 确认 stop，其他键取消；公开 CLI 的错误和超时可见。
- Viewer/Queue：portable-pty 独立读写线程，alacritty_terminal 解析网格；真彩/256 色、宽字符、光标、鼠标、粘贴、应用光标键和修饰 Enter。
- Ctrl-] 回 Agents；Agents 中 Tab 到 Queue、Shift-Tab 到 Viewer；鼠标选 agent / 切焦点。
- attach 前现查 ATT，已有其他接入时拒绝；切换只 SIGINT 本程序的 attach PID，等待退出后再启动新 attach，3 秒后可强制结束该 PID；agent 消失后不自动接入其他人。
- 快速切换时以最新选择为准；Queue 命令退出保留最后输出；正常退出恢复外层终端并清理本程序的子进程。
- README 含运行方法、配置、按键与验证边界。

## 设计决定

- 按用户确认将第 10 节真实 Claude Code 原型改为合成流比较。`examples/compare_parsers.rs` 同时运行 alacritty_terminal 0.26 和 vt100 0.16；颜色、中文、光标、清屏、滚动区结果一致，选择前者是因为它能通过事件回传终端查询。vt100 只作开发依赖。
- 第 5 节落实焦点按键；第 4 节明确窄屏左列最多占一半。
- 实现前完整读取 corral/tools/board；只通过公开 ls/status/reply 核对真实 JSON，未 attach、输入、启动或停止任何真实 agent。

## 自动验证

最终检查均通过：

- `cargo test --all-targets`：20 项测试通过。
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `cargo run --example compare_parsers`
- `cargo build --release`：可执行文件 `target/release/saddle`。
- `git diff --check`

RED→GREEN 的行为检查包含：布局分配、非法配置、公开 JSON 合并、刷新后选中项保持、焦点路由、UTF-8/控制键编码、PTY 输入、鼠标本地坐标、网格绘制。整程序回归进一步发现并修复：

- 传统 Ctrl-] 的 0x1d 被 crossterm 解码为 Ctrl-5，最初错误传入终端；同时修复其他传统控制字符的数字别名。
- 修饰 Enter 最初被当作普通提交键。
- Queue 退出时最初清空最后报错。
- 从 A 切去 B 尚未结束时重新选 A，最初仍会接入 B。

外层 PTY 完整流程覆盖：键盘和鼠标接入、中文粘贴、Viewer/Queue 输入、ATT 显示、旧 attach 退出后才启动新 attach、agent 新增/状态变化/消失、回复分页、停止确认与取消、拒绝已有其他 attach、双 PTY resize、持续输出时响应退出、退出恢复和模拟 agent 继续存在。测试不将模拟结果当作真实 corral 生命周期的证明。

## 尚需用户验收

在真实终端运行 `./target/release/saddle`，按 DESIGN 第 11 节确认真实 Claude Code / drover 显示与操作、真实 ATT 和断开后 agent 继续运行。本轮按用户要求不执行这部分，也不录屏。

第 2、3 步尚未开始；左下仍运行配置命令（默认 drover board）。没有待用户裁定的设计问题。
