# saddle

**把你的编程 agent 放进同一个终端工作台。**

[English](README.md) · 简体中文

在一个窗口里查看 agent、管理任务队列，并直接使用选中 agent 的实时终端。

```text
┌─ Agents ─────────────┬─ Viewer ──────────────────────────┐
│ project/            │                                   │
│ ├─ main   working   │  选中 agent 的实时终端             │
│ └─ review idle      │                                   │
├─ Queue ─────────────┤  在这里输入、粘贴和交互。          │
│ Current / Pending   │                                   │
│ History             │                                   │
└─────────────────────┴───────────────────────────────────┘
```

saddle 使用 Rust 和 [Ratatui](https://ratatui.rs/) 编写，把 [corral](https://github.com/firegnu/corral) 的 agent 会话与 [drover](https://github.com/firegnu/drover) 的任务队列整合起来，不依赖 tmux 或 Zellij。

## 功能

- **Agents：** 按仓库分组的树形列表，展示实时状态、agent 类型、活动、接入数量、工作目录和标题。用颜色区分工作、空闲、阻塞、停滞和错误。
- **Queue：** 当前任务、待放行、待办和历史记录。原生控件支持查看详情、新增任务、切换项目、放行、暂停和循环设置。
- **Viewer：** 选中 agent 的实时 `corral attach` 会话，支持终端颜色、Unicode、光标、鼠标事件与粘贴。
- **鼠标与键盘：** 紧凑的可点击按钮、鼠标滚轮、触控板和快捷键。滚动列表不改变选择，正常刷新保留滚动位置。
- **响应布局：** 宽窗口显示三窗格；窄窗口将 Agents 和 Queue 收为标签。
- **终端原生外观：** 面板背景透明，状态有语义颜色，界面标签使用英文；任务内容和 agent 输出保留原文。

Agents 和 Queue 均为 Rust 原生控件。只有 Viewer 使用子 PTY，不嵌入外部看板界面。

## 快速开始

### 环境要求

- Rust stable **1.96 或更高版本**。
- `corral` 和 `drover` 在 `PATH` 中，或在配置中指定路径。
- 支持 Unicode 和鼠标的终端，建议支持真彩色。

目前在 macOS 上开发并进行交互验证，其他平台尚未验证。

### 构建和运行

```sh
git clone https://github.com/firegnu/saddle.git
cd saddle
cargo run --release --locked
```

也可以从仓库安装可执行文件：

```sh
cargo install --path . --locked
saddle
```

请先用 corral 启动 agent，并用 drover 登记队列项目。saddle 展示已有会话与项目，不负责创建 agent 或初始化队列项目。

### 第一次使用

1. 在左侧选择 agent，按 **Enter** 或点击该行接入。
2. 在 Viewer 中直接输入，与 agent 交互。
3. 按 **Ctrl-]** 回到 Agents，Viewer 保持连接。
4. 按 **Tab** 或点击 Queue 切换焦点，通过 **Project** 选择已登记项目。
5. 从任意位置退出：先按 **Ctrl-]**，再按 **q**。退出只断开 saddle 的 Viewer，agent 继续运行。

如果其他终端已经接入某个 agent，请先在那里断开，再通过 saddle 接入。停止 agent 是独立操作，需要确认。

## 配置

启动时，若 `XDG_CONFIG_HOME` 为绝对路径，读取 `$XDG_CONFIG_HOME/saddle/config.toml`；未设置、为空或为相对路径时，读取 `~/.config/saddle/config.toml`。文件不存在和省略的配置项均使用默认值。`--config` 指定的路径优先：

```sh
saddle --config /path/to/config.toml
```

```toml
corral = "corral"
left_width = 52
left_split = 0.5
refresh_ms = 1000

[queue]
drover = "drover"
# cwd = "~/projects/my-project"
```

| 配置项 | 含义 |
|---|---|
| `corral` | corral 命令名或路径 |
| `left_width` | 左栏期望宽度，单位为终端字符格 |
| `left_split` | Agents 占左栏高度的比例，大于 0 且小于 1 |
| `refresh_ms` | 后台刷新间隔，单位毫秒 |
| `queue.drover` | drover 命令名或路径 |
| `queue.cwd` | 可选的初始队列项目目录 |
| `colors` | 可选的平铺颜色表，控制界面和 agent 类型配色 |

命令路径和 `queue.cwd` 支持 `~/`。Queue 读取 `~/.drover/projects` 项目清单：优先使用 `queue.cwd`，其次是已登记的启动目录，再其次是登记的首个项目；没有登记项目时尝试启动目录。项目选择页也支持手动输入路径，切换仅对本次运行生效。

saddle 通过 **`drover list --json`** 获取任务数据。完整历史需要 drover 返回全部历史数组；旧版本仅返回最近十条，接口未返回的记录无法显示。除了项目登记清单，saddle 不读取 corral 或 drover 的内部数据文件。

[config.toml](config.toml) 提供完整默认配置与简短注释，可直接复制到上述位置，默认呈现与原界面一致。只想改几项时，可添加：

```toml
[colors]
focus = "light_cyan"
bg = "default"
agent_selected = "#302a23"
```

颜色支持 `default`（或 `reset`，终端默认色）、`#RRGGBB` 和小写 ANSI 色名：`black`、`red`、`green`、`yellow`、`blue`、`magenta`、`cyan`、`gray`、`dark_gray`、`light_red`、`light_green`、`light_yellow`、`light_blue`、`light_magenta`、`light_cyan`、`white`。其中 `gray` 是普通 ANSI 白，`dark_gray` 是亮黑，`white` 是亮白；ANSI 色随终端调色板变化。

颜色表覆盖背景、选中底色、边框、焦点、文字层次、连接/未读标记、操作反馈、agent 类型及回复格式。`agent_*` 状态强调色由 Agents 和 Queue 共用，也用于 Queue 操作反馈。未知字段和无效颜色沿用配置错误报告。修改后下次启动生效，不支持热加载；Viewer 的终端输出保留自己的颜色。字体和字号仍由外部终端设置控制。

## 操作

| 位置 | 输入 | 操作 |
|---|---|---|
| Agents | ↑↓ / j k | 选择 agent |
| Agents | Enter / 点击行 | 接入并将焦点切到 Viewer |
| Agents | 鼠标滚轮 / 触控板 | 滚动列表，不改变选择 |
| Agents | Tab / Shift-Tab | 焦点到 Queue / Viewer |
| Agents | PgUp / PgDn | 滚动 agent 列表 |
| Agents | s | 仓库内按名称 / 状态排序 |
| Agents | x，然后 y | 停止选中 agent，其他键取消 |
| Agents | q | 退出 saddle |
| Queue | ↑↓ / j k / 点击 | 选择任务 |
| Queue 列表 | 鼠标滚轮 / 触控板 | 滚动任务和历史，不改变选择 |
| Queue | c | 打开项目选择页 |
| 项目选择页 | Enter / 点击、e、r | 打开项目、手动目录、重读登记清单 |
| 路径表单 | Ctrl-U / Enter / Esc | 清空 / 应用 / 取消 |
| Queue | Enter / Esc | 打开详情 / 返回列表 |
| Queue 详情 / 反馈 | 鼠标滚轮 / PgUp / PgDn | 滚动内容 |
| Queue | r / g / n | 刷新 / 核对放行 / 发送下一件 |
| Queue | p / l | 暂停或恢复 / 切换循环 |
| Queue | a / ? | 新增任务 / 帮助 |
| 新增表单 | Tab / Ctrl-S / Esc | 切换字段 / 保存 / 取消 |
| Queue | q | 返回 Agents；文本字段中作为普通文字 |
| Queue / Viewer | Ctrl-] | 返回 Agents |

Viewer 将输入传给 agent，**Ctrl-]** 除外。底栏显示当前输入目标。弹层打开时只处理自身输入，背景控件不响应。暂时回到 Agents 时，Queue 未提交的草稿会保留。

**Go、Next、Pause、Loop 始终作用于当前项目**，与选中的历史任务无关。读取失败会禁用旧数据上的操作。没有待处理工作时显示 `No active tasks`，历史记录仍可查看，底部显示可见范围。

## 开发与验证

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

默认测试使用假的公开 CLI、临时目录和合成终端流，不启动真实 agent。

安装了 drover 后，可在隔离临时项目中运行可选集成测试：

```sh
SADDLE_DROVER_BIN="$(command -v drover)" \
  cargo test --test workflow installed_drover_ -- --ignored
```

完整历史测试要求 drover 已包含上文所述的历史条数修复。这些测试不修改真实队列，也不使用真实 agent。

生成合成 UI 预览，或比较终端解析器：

```sh
cargo run --example ui_preview -- /tmp/saddle-ui-preview
cargo run --example compare_parsers
```

预览是 Ratatui 网格导出的 SVG 和文本，不是真实 agent 录屏。合成检查不代表已验证所有 agent 终端应用的兼容性。

架构决定与验证记录见[设计文档](docs/DESIGN.md)和[交接记录](HANDOFF.md)，目前均使用中文。
