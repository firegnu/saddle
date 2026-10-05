# saddle

**给 coding agent 用的终端工作区。**

[English](README.md) · 简体中文

Saddle 用 Rust/Ratatui 实现 Agents 面板和终端，通过 PATH 上由 [ranch](https://github.com/firegnu/ranch) 安装的 `corral` 使用运行时，不依赖 tmux 或 Zellij。Saddle 定位为保底前端：不再加新功能，只保证运行时兼容。

## 功能

- **Agents：** 按仓库分组，显示状态、类型、公开角色和 effort 标签、标题、活动、目录、实例及接入信息。需要输入或出错的 agent 优先，`s` 切换名称排序，`z` 折叠列表；本地接入标记覆盖所有 tab 和窗格。
- **Git 摘要：** 根据 agent 的公开 cwd 显示本地分支、领先提交数、修改行数和未跟踪文件。不 fetch，不运行外部 diff、textconv、fsmonitor 或 clean/smudge filter；取不到的值保持未知。
- **Attention：** 汇总等待输入、出错和有新回复的 agent。点击只选中并接入，不代答或停止。
- **终端：** 普通 shell 与 agent 实时终端、tab 和四向分屏，支持终端颜色、Unicode、鼠标、粘贴、历史搜索和复制。
- **设置：** 配置、主题、吉祥物、Diagnostics 和 Updates。保存的颜色和侧栏宽度立即生效；标为需要重启的项在重开后生效。
- **布局：** 保存 tab、分屏、活动窗格和 agent 身份。只有原来的存活实例自动重连；退出的 agent 和 shell 保留占位。
- **本地控制：** `saddle ctl` 通过本地 Unix socket 查看工作区、打开终端/agent 和关闭显示。

2026-10-05 已删除遥测、Drover 和整个插件系统，不再提供插件窗格、SDK/协议、任务队列、录制开关，以及 `saddle agent`、`saddle telemetry`、`saddle plugin`、`saddle ctl plugin`。

派发路由归 ranch：使用 `ranch dispatch route`；corral-dispatch 技能由 `ranch dispatch install-skills` 安装。Saddle 不安装技能，派任务不记遥测。

## 构建和运行

要求 Rust stable 1.96+、PATH 上的 ranch Corral、支持 Unicode 的终端。Codex、Claude Code 等程序另行安装。Git 2.45+ 用于 Agents 的 Git 摘要。目前开发和交互验证在 macOS 上进行，其他平台未验证。

```sh
git clone https://github.com/firegnu/saddle.git
cd saddle
cargo build --bin saddle --release --locked
./target/release/saddle
```

构建不可覆盖的版本目录：

```sh
./scripts/package.sh /absolute/new/saddle-version
/absolute/new/saddle-version/bin/saddle
```

包里只有 `BUILD.txt` 和 `bin/saddle`。打包不安装、不切换运行中的程序。部署不改 `~/.local/bin/corral`，不安装 corral 或 corral-dispatch 技能。保留 `~/.local/share/saddle/versions/` 下旧目录，会话可能仍使用里面的 Corral。部署后由用户正常退出并重开 Saddle，现有 agent 继续运行。

## 工作区操作

选中 agent 按 Enter 接入。已经显示的 agent 跳到原窗格，不重复接入。`Ctrl-]` 把输入交回 Agents；Viewer 中的键盘输入交给终端。关闭 agent 显示只断开接入；停止使用独立的 `x`、再 `y` 确认。

**New / n** 打开创建表单：选择项目、Codex/Claude 和角色。Controller 名称固定为 `main`，Regular 可编辑名称；完整名称为 `Prefix/Name`。项目候选来自公开 agent cwd 和启动目录，也可输入路径。Advanced 提供命令、首条消息、打开位置和完整调用预览。创建传入公开 role 标签，不自动编号。

**Split ▾** 先选择 Left/Right/Above/Below，再选 Terminal、New agent 或现有 agent；**+ Tab** 使用同样的内容选择器。移动已经打开的 agent 保留会话与尚未完成的接入，取消不改布局。

**Close pane / Close tab / Quit** 遇到运行中的 shell 先确认。**Zoom / Restore** 临时放大活动窗格，保留分屏。窄窗口可能暂时只显示一侧，放大后恢复。

窗格底部的 **History** 将滚动、搜索和选区输入留在 Saddle。滚轮、方向键、PgUp/PgDn 滚动，`/` 搜索，`n/N` 切换匹配，Copy 复制。Esc 先退出搜索，再返回实时输入。

## 设置、Diagnostics 和 Updates

通过 **Settings** 打开，或在 Agents 按逗号。

| 页面 | 按键 | 内容 |
|---|---|---|
| General | F1 | 侧栏宽度、刷新间隔、吉祥物及显示方式 |
| Colors | F2 | 主题、颜色覆盖和预览 |
| Advanced | F3 | Corral 命令 |
| Diagnostics | F4 | 只读运行时、配置与布局检查 |
| Updates | F5 | Saddle 源码/安装/运行版本，以及 Corral 会话兼容性 |

**Save / Ctrl-S** 只写改动的配置键，保留注释和未知的旧配置；**Cancel / Esc** 不改文件，**Default / Ctrl-D** 恢复当前字段的默认草稿。无效值保留草稿。外部文件变化时选择 Keep my edits、Discard my edits 或 Back，不覆盖未确认的变化。

Updates 独立从 PATH（或显式 Corral 配置）查找 Corral，不从 Saddle 包内查找。**Upgrade all** 沿用确认和公开 Corral 升级命令；它操作 agent 运行时，与安装 Saddle 是两件事。

## 本地 ctl

```sh
saddle ctl instances
saddle ctl inspect --instance INSTANCE
saddle ctl open --instance INSTANCE --relative-to active --place right --shell --cwd /absolute/path
saddle ctl open --instance INSTANCE --relative-to active --place tab --agent project/name --focus
saddle ctl request REQUEST --instance INSTANCE
saddle ctl close --instance INSTANCE --pane PANE
saddle ctl --help
```

输出为 JSON。`open` 也支持 `--name`、`--role`、`--prompt` 和 `-- PROGRAM ARG...` 创建 agent。`--relative-to self` 使用调用方公开 Corral 身份或 Saddle 创建的 shell 环境。结果不确定时先查询 request；重试使用同一 request ID 和参数。关闭运行中的 shell 要使用返回的确认 token 和 `--confirm-shells`。

## 配置和数据

配置路径为绝对 `$XDG_CONFIG_HOME/saddle/config.toml`，否则 `~/.config/saddle/config.toml`；`--config PATH` 优先。完整注释见 [config.toml](config.toml)。

```toml
corral = "corral"
left_width = 52
refresh_ms = 1000
theme = "dune"
mascot_enabled = true
mascot = "clawd"
mascot_display = "auto"
```

命令路径支持 `~/`；命令名从 PATH 查找，没有同目录或包内 Corral 回退。主题包括 Dune、Tide、Lagoon、Terminal。

布局保存在绝对 `$XDG_STATE_HOME/saddle/layout.json`，否则 `~/.local/state/saddle/layout.json`。旧插件位置恢复为空窗格，其余内容和分屏位置保持。损坏或不支持的布局保留原文件并禁用保存，由用户处理；不重放 shell 历史或已结束对话。

旧 `~/.local/state/saddle/telemetry/`、`~/.drover`、插件登记和 `plugin-resources.json` 留在磁盘上，运行程序不读也不删。本次迁移仅解除 corral-dispatch 的 Saddle 归属条目，保留技能文件交给 ranch 接管。

## 开发与验证

当前决定见 [DESIGN](docs/DESIGN.md)，旧遥测、插件和任务文档只作历史记录。按 [UI 回归指引](docs/UI回归.md) 选择检查；跨模块修改和发布前运行：

```sh
export CARGO_TARGET_DIR="$HOME/Developer/personal_projs/saddle-worktrees/.target"
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

测试使用临时数据和假 Corral。不操作用户现有 agent，不为通过检查而放宽断言或修改超时。
