# 进度

## 第 1 步（原生 UI 修订版）：实现与自动验收完成

2026-09-25，直接在 main 分步提交，未推送。按用户后续要求，原来的「Queue 先嵌入 drover board」方案已经作废。

### 当前实现

- Agents：ratatui 原生面板。公开 ls/status 每秒后台刷新；项目分组、状态排序、完整字段和窄屏折行、选中项保持、未读标记、滚动条/隐藏数量、回复分页、x/y 停止确认。
- Queue：ratatui 原生列表、详情、帮助、操作反馈、标题/正文新增表单。公开 `drover list --json` 提供 mode、paused、current、awaiting、pending、history；g/n/p/l/a 通过公开 CLI 执行放行、下一件、暂停/恢复、循环和新增。失败保留新增草稿，执行中不重复提交。
- Viewer：唯一使用 PTY 的窗格，仅运行公开 `corral attach`。Rust/alacritty_terminal 解析、ratatui 绘制颜色、中文、光标、鼠标和粘贴；切换等待旧 attach 退出，再接入最新选择。已有其他 attach 时拒绝接入，agent 消失后不自动换人。
- 所有 saddle UI 都是 Rust。生产代码没有 `board` 或 Python UI 调用；不启动外部编辑器。测试中的 Python 文件只是假的 CLI/字节流边界，不作为 UI 实现。
- Queue 的 `queue.command` 已删除并在解析时拒绝；新配置为 `queue.drover` 和可选 `queue.cwd`。cwd 默认启动目录，不读取 drover 内部项目注册表。
- TOML 配置、三窗格缩放、焦点路由、后台命令取消/超时、终端退出恢复、README 和 `--help` 已同步。

### 验证

全部通过：

- `cargo test --all-targets`：27 项通过；依赖本机 drover 的 1 项默认忽略，已单独执行通过。
- `SADDLE_DROVER_BIN=/Users/firegnu/.local/bin/drover cargo test --test workflow installed_drover_cli -- --ignored`：真实公开 CLI 在隔离 HOME/临时项目中驱动原生 Queue，验证正文、暂停/恢复、循环开关、多行中文新增和缩放。不调用外部看板，不启动真实 agent。
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `cargo build --release`：可直接运行 `target/release/saddle`。
- `git diff --check`
- 生产调用点检查：仅 `src/viewer.rs` 调用 `Session::spawn`；无 `board`、`python`、`queue.command` 调用。

已有合成原型 `cargo run --example compare_parsers` 对比 alacritty_terminal 与 vt100 的中文、真彩/256 色、光标、清屏和滚动区；选择前者的理由是终端查询的事件回传能力。

RED→GREEN 包括布局、配置、公开 JSON、刷新选中项保持、输入路由/编码、PTY 输入、鼠标坐标和网格绘制；整程序回归覆盖接入/切换顺序、ATT、状态变化/新增/消失、回复、停止确认与取消、输出压力和退出恢复。原生 Queue 新增 RED→GREEN 覆盖公开数据/动作、选择/新增状态机、拒绝外部 UI 配置、禁止 Queue 按键进入 PTY。

### 边界与记录

- 按用户约束，未启动、attach、输入或停止任何真实 agent；真实 Claude Code 显示与真实 corral 生命周期未验证，不用合成测试代替该结论。
- 本机 Terminal GUI 控制被工具拒绝，没有绕过限制；交互验收在独立 PTY 中完成，没有录屏。
- 核对 CLI 时误将 `drover init --help` 当成帮助命令。确认创建时间及精确内容后，已删除它意外创建的 saddle 配置、空的 `--help` 交接目录和末尾新增登记项，并恢复 `.gitignore`；后续所有 drover 写入测试都使用隔离 HOME/临时项目。
- 无待用户拍板的实现问题。原生 Queue 只显示公开 JSON 已有字段；更细的判据数据若将来需要，应由 drover 增加公开 API，saddle 不读取内部文件补齐。
