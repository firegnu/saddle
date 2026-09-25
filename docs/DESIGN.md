# saddle：设计

一个 Rust 写的终端界面程序。一条命令打开，就是现在手动在终端里分三格拼出来的那套工作台：左上看所有 agent，左下看任务队列，右边是选中 agent 的实时终端。一个焦点、一套按键，左边选中谁，右边立刻切过去。

## 1. 为什么做

现在每次开工要：开一个终端 → 左右分割、拉宽度 → 左边再上下分割 → 三格里依次运行 `board`、`drover board`、`boardv`。三个程序是分开的进程，左边选人、右边切换靠临时目录里的文件对接。

要的就是把这套手动步骤变成一个程序，并把「靠文件对接」换成进程内直接切换。

## 2. 范围

**做**

- 一条命令打开，布局已经排好。
- 左上：原生的 agent 面板，信息和按键与现在的 `corral/tools/board` 一致。
- 右边：内嵌终端，运行 `corral attach <选中的 agent>`，能直接往 agent 里打字。
- 左下：任务队列。第 1 步先内嵌终端跑现有的 `drover board`；第 2、3 步换成原生面板（见第 9 节）。
- 一个配置文件。

**不做**

- 常驻服务、会话恢复：agent 由 corral 在后台托管，关掉本程序什么都不丢。
- 新开 agent（`corral start`）：用户在外面的终端手动开。
- 多标签页、自由分屏、插件、主题系统，以及 herdr 那类额外功能。
- 修改 corral、drover 的任何代码或数据。

## 3. 和 corral、drover 的关系

- **只走公开命令，不读它们的内部文件。** corral：`corral ls`、`corral status <名字>`、`corral reply <名字>`、`corral attach <名字>`、`corral stop <名字>`，都输出 JSON（`attach` 除外）。drover：第 1 步不直接调用；第 2、3 步用 `drover list --json` 和 `drover go / next / pause / resume / loop / add` 等命令。
- **判断全在它们那边。** agent 的状态（working、idle、blocked……）由 corral 根据钩子事件判定；队列、判据、放行由 drover 决定。本程序只显示和转发按键，行为因此和现在手动开的那套一致。
- `corral` 从 `PATH` 找，配置里可以改路径。

## 4. 布局

```
┌─ Agents ─────────────┬──────────────────────────────────┐
│ corral/main   ● idle │                                  │
│ drover/main   ◐ work │   选中 agent 的实时终端           │
│ …                    │   （corral attach）              │
├─ Queue ──────────────┤                                  │
│ （第 1 步：drover     │                                  │
│   board 内嵌终端）    │                                  │
└──────────────────────┴──────────────────────────────────┘
```

- 左列默认 52 列，左边上下默认对半；都可在配置里改。窄屏时左列最多占窗口一半，给 Viewer 留出空间。
- 窗口缩放时按比例重排；嵌进来的终端收到新尺寸（PTY 调整大小），里面的程序自己重画。
- 焦点所在的格子边框高亮，其余暗一些。

## 5. 焦点和按键

三个格子：Agents、Queue、Viewer（右边）。任何时候只有一个有焦点。

| 操作 | 按键 |
|---|---|
| 在 Agents 里上下选 | ↑↓ / j k |
| 让右边显示选中的 agent 并把焦点给它 | 回车，或鼠标点这一行 |
| 焦点从 Viewer 回到左边 | `Ctrl-]`（被本程序截下，不传给 agent；和 `corral attach` 自己的断开键一致，用户已经习惯） |
| 在格子之间切焦点 | 鼠标点格子；Queue/Viewer 中 `Ctrl-]` 回 Agents，Agents 中 `Tab` 到 Queue、`Shift-Tab` 到 Viewer |
| 退出 | 焦点在 Agents 时按 `q` |

- 焦点在 Viewer 时，除了 `Ctrl-]`，所有按键、粘贴、鼠标事件都原样送进 `corral attach`。
- 焦点在 Queue 且第 1 步是内嵌终端时，同理：除了切焦点的键，其余原样送进去。
- Agents 面板现有的按键全部保留：`r` 显示或隐藏回复区、`PgUp/PgDn` 滚动回复区、`s` 按状态分组、`x` 再按 `y` 停掉 agent、滚轮在列表上滚动。

## 6. Agents 面板（原生）

**信息和按键与现在的 `corral/tools/board` 一致，样子可以更好。** 实现前先读一遍 `tools/board`（Python，约 1000 行），它是行为的依据。

- 每秒刷新：`corral ls` 拿名单，每个 agent 再 `corral status` 拿状态字段。
- 列：NAME、KIND、INST、STATE、DOING（在做什么、用了多久）、QUIET（多久没输出）、ATT（几个人接在上面）、SOURCE（最近输入来自谁）、DIR、TITLE。
- 保留的行为：按项目前缀分组；working 有转圈；需要人处理的（blocked 等）醒目；窄屏时身份信息按项折行；列表放不下时有滚动条和「上下还藏着几个」的提示；回复区默认隐藏，按 `r` 显示选中 agent 的上一轮回复（`corral reply`）。
- 右边正在显示的那个 agent，在列表里有标记。

## 7. Viewer（右边的内嵌终端）

- 用 PTY 运行 `corral attach <名字>`，终端解析库把输出变成一格格字符，画进右边的格子。
- **切换 agent**：给旧的 `corral attach` 发 SIGINT（它会自己还原终端并退出，被断开的 agent 继续跑），等它退出后，起新的。和现在 `boardv` 的做法一样。
- 没选中任何 agent 时，显示一句提示：在左边选一个 agent。
- 被选中的 agent 消失了（被关掉）：`attach` 退出，右边回到提示画面，不自动选别的。
- 需要支持：256 色和真彩色、中文等宽字符、光标、清屏和滚动区、鼠标上报（Claude Code 等会用到）、粘贴。

## 8. 配置

`~/.config/saddle/config.toml`，没有文件就用默认值。

```toml
corral = "corral"              # corral 命令的路径
left_width = 52                # 左列宽度（列）
left_split = 0.5               # 左边上格占的比例
refresh_ms = 1000              # Agents 面板刷新间隔

[queue]                        # 左下格（第 1 步：内嵌终端跑一个命令）
command = ["drover", "board"]
# cwd = "~/Developer/personal_projs/drover"
```

## 9. 分三步

每一步做完都能直接用。

| 步 | 做什么 | 做完的样子 |
|---|---|---|
| 1 | 布局、焦点、Agents 原生面板、Viewer 内嵌终端；左下内嵌终端跑配置里的命令（默认 `drover board`） | 取代手动分三格和 `board`、`boardv`；`drover board` 照常用 |
| 2 | 左下换成原生的队列面板，只看：信息和现在的 drover 看板一致 | `drover board` 退为后备 |
| 3 | 把 drover 看板的操作键搬过来（放行、下一件、暂停、循环、加任务、帮助页、待办详情页） | 完整替代 |

- 第 2、3 步需要 drover 提供更多 JSON（任务正文、判据结果、待办详情），那是 drover 仓库的活，到时另开。
- 新程序成为主力后，Python 版的 `tools/board` 和 `drover-board` 保留做后备，不再加新功能。

## 10. 技术选型

- Rust stable（本机 1.96）。
- 界面：`ratatui` + `crossterm`。
- 进程和 PTY：`portable-pty`。
- 终端解析：选用 `alacritty_terminal` 0.26。按用户确认，不启动真实 agent；`cargo run --example compare_parsers` 将同一合成流逐字节交给它和 `vt100` 0.16。两者网格、256 色/真彩色、中文、光标、清屏和滚动区一致；选择前者是因为 `Event::PtyWrite` 能直接回传终端查询（原型验证了光标位置查询），避免另写查询解析器。`vt100` 仅作为开发依赖保留用于比较。真实 Claude Code/drover 的观感与手感仍需用户目视确认。
- 读 PTY 输出用单独的线程或异步任务，界面按帧刷新，不能因为某个 agent 输出多就卡住按键。
- 只依赖成熟、活跃维护的库；不引入 tmux、zellij 等外部程序。

## 11. 第 1 步怎么算做完

- 一条命令打开，三格按配置排好。
- 左边选中一个 agent 按回车或点击，右边切到它，能打字、能看到它的实时输出；`Ctrl-]` 回到左边。
- 换 agent 时，被断开的 agent 继续跑（`corral status` 看得到）；右边正在看的那个，ATT 显示 1。
- agent 状态变化、派出去的新 agent 出现，和现在的 `board` 一致（同样来自 `corral ls/status`）。
- 窗口缩放后三格重排，右边和左下的程序正常重画。
- 左下内嵌的 `drover board` 能正常使用，焦点切过去后它的按键都有效。
- 退出程序后，所有 agent 继续在跑。

## 12. 验证

- 纯逻辑写单元测试：布局计算、焦点状态机、解析 `corral ls/status` 的 JSON、按键路由（哪个键给谁）。
- `cargo test`、`cargo clippy` 通过。
- 真实终端里的观感和手感由用户目视确认，不做录屏。
- 测试不启动真实 agent；需要时用一个假的 `corral` 脚本输出固定 JSON。
