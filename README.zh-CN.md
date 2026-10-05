# saddle

**把你的编程 agent 放进同一个终端工作台。**

[English](README.md) · 简体中文

在一个窗口里查看 agent、使用实时终端，并通过可选插件管理任务。

```text
┌ Agents · 2        ──┬─ Viewer ──────────────────────────┐
│ project/ ─────── (2)│                                   │
│ │ ◐ main   working  │  选中 agent 的实时终端             │
│ │ ○ review idle     │                                   │
│                     │  在这里输入、粘贴和交互。          │
│                     │                                   │
└─────────────────────┴───────────────────────────────────┘
  Plugins 打开统一插件面板；Drover 任务界面由可选插件提供。
```

saddle 使用 Rust 和 [Ratatui](https://ratatui.rs/) 编写，承载 [corral](https://github.com/firegnu/ranch) 的 agent 会话与可选进程插件（包括 [Drover](plugins/drover/README.md)），不依赖 tmux 或 Zellij。

## 功能

- **Agents：** 按仓库分组（`name/ ──── (n)` 分组标题），每个 agent 左侧一条竖线，首行固定列依次为状态点、名称、agent 类型、状态和时间，下面是标题、活动、Git 行、目录（放得下时完整显示，放不下才从左侧省去前面的层级）和 实例 · `ATT`（公开接入数）· `VIA`（最近输入来源）。状态：`?` waiting（公开状态 `blocked`，黄）、`!` error（红）、`◐` working（蓝）、`○` idle（绿）、`✕` exited（极暗）；`▲` stalled、`◌` starting、`·` unknown 保留各自样式。组内默认需要人处理的在前（waiting → error → working → idle → exited），**s** 切换为按名称。时间前的 `⦿` 表示本 saddle 在任一窗格或标签页中正显示该 agent；`•` 表示有未看的已完成回合。agent 超过 5 个时默认折叠，未选中项只留首行，**z** 切换。不足 50 列时类型列只显示标识（`✳`、`>_`、`π`）。带有 corral 公开 `effort` 标签的 agent，会在首行类型之后、状态之前显示下表中的两字符点阵信号图标（折叠时同样显示）：

  | 档位 | 图标 | 默认颜色 |
  |---|---|---|
  | medium | `⣄⡀` | 柔绿（`agent_idle`） |
  | high | `⣴⡀` | 蓝（`agent_working`） |
  | xhigh | `⣴⡇` | 紫（`agent_starting`） |

  未亮柱只留底点，选中与否图标一致。没有有效标签时不显示图标但保留对齐；所有 agent 都没有有效标签时不占该列。图标只反映委派时的公开标签，不代表运行时实际 effort。
- **每个 agent 的 Git 摘要：** 目录上面一行，例如 `⎇ dev-t12 ↑2 main     +18 -4 ?1`，描述该 agent 公开 corral `cwd` 所在 worktree：当前分支（与 agent 名称相同时只显示 `⎇`）；`↑n 基准`，即比本地 `main` 多的提交数（在 `main` 上则相对其配置的上游，即尚未推送的提交）；未提交的增删行数（暂存与未暂存一起相对 HEAD，按 Git 内建 text/eol 属性规范化后比较，已提交的 CRLF 文件只改时间戳不算改动；按路径统计、不做重命名检测，纯改名算全删加全增）；未跟踪文件数。数字属于目录而不是 agent：共用同一 worktree 的 agent 显示同一行，也不能说明提交是哪个 agent 或哪个任务做的。无法确定的值显示 `—`（没有本地 `main`、没有上游、detached HEAD、还没有提交）；二进制文件没有行数，单独显示为 `N binary`；增删放不进分支那一行时整组移到下一行右对齐；不是 Git worktree、目录已删除或超时显示 `git unavailable`。约每 5 秒刷新，只读本地数据，慢仓库会拖慢所有目录的这一轮。不 fetch，缺对象时也不补取。无论哪个 attributes 来源，都不运行外部 diff、textconv、fsmonitor 钩子或 clean/smudge/process filter；改动的文件需要这类 filter 才能比较时，增删行显示 `+— -—`。父仓库的摘要不进入子模块工作区：子模块里未提交的改动不计入，子模块提交变了按 gitlink 变化计（`+1 -1`）。不顺带写索引，也不继承 `GIT_DIR` 等 `GIT_*` 环境变量。agent 之后 cd 到别处不会跟随。
- **Drover 插件（可选）：** 任务列表、项目切换、详情、Links/Telemetry 关联跳转、编辑与显式派发/提交/接受/退回。通过 **Plugins → Drover** 打开；关闭视图继续后台观察，停用才停止。不会自动推进任务。见[安装与操作](plugins/drover/README.md)。
- **Attention：** 汇总需要输入/出错的 agent、新回复和已启用插件的当前条目。点击只打开来源，不回答或推进业务。Drover 插件通过通用接口提供待验收任务与失败历史；历史标记已看在插件中操作。
- **Settings：** Attention 同一行右侧的 `Settings` 入口（或在 Agents 按 **,**）编辑 saddle 启动时使用的配置文件，路径显示在顶部。**General** 含侧栏宽度和刷新间隔；**Colors** 按用途分组列出全部 `[colors]` 值，带色块和小范围预览；**Advanced** 含 corral 命令。修改先留在草稿中，**Save / Ctrl-S** 才保存；**Cancel / Esc** 不改文件；**Default / Ctrl-D** 把当前项恢复默认值，仍需 Save。保存只写改动的键，保留注释和其余内容，文件或目录不存在时自动创建。无效值会报错并保留草稿。若 Settings 读取后文件在磁盘上被改动，则不保存：**Keep my edits** 重读文件并保留草稿，**Discard my edits** 采用文件现状。保存后的颜色和侧栏宽度立即生效（agent 输出保留自己的颜色）；标注 `Restart required` 的设置下次启动生效。
- **任务通知（Drover 插件）：** 在插件中按 **N** 选择 System/In Saddle，**Ctrl-S** 经公开 Drover CLI 保存。现有系统通知 watch 不变；首次观察和偏好变更建立基线，不补弹旧任务。内部提示不抢终端输入，点击打开插件目标。

  三步使用示例：

  1. 打开 **Plugins → Drover**，按 **N**，选择 **In Saddle**，按 **Ctrl-S** 保存。
  2. 等待一件新任务完成并进入 **Awaiting release**，右下角会出现短提示。
  3. 点击提示查看任务；查看或关闭提示都不会放行，仍由你手动放行。

- **内置启动：** 原生表单填写目录、名称、命令与首条消息，预览确认后调用公开 corral start。
- **Viewer 标签页和分屏：** 每个 tab 保存一组可四向分割的窗格，各自运行实时 `corral attach` 会话，支持终端颜色、Unicode、光标、鼠标事件与粘贴。
- **鼠标与键盘：** 紧凑的可点击按钮、鼠标滚轮、触控板和快捷键。滚动列表不改变选择，正常刷新保留滚动位置。
- **响应布局：** Agents 独占左栏；插件在分配的局部覆盖界面或工作区内适应宽度。
- **终端原生外观：** 面板背景透明（Agents 使用自己的暖暗配色），状态有语义颜色，界面标签使用英文；任务内容和 agent 输出保留原文。

Agents 和 Tasks 均为 Rust 原生控件。只有 Viewer 窗格使用子 PTY，不嵌入外部看板界面。

## 快速开始

### 环境要求

- Rust stable **1.96 或更高版本**。
- 由 [ranch](https://github.com/firegnu/ranch) 安装在 PATH 上的 `corral`；Claude Code、Codex 等 coding agent CLI 仍由用户安装。可选 Drover 插件自带任务引擎，无需旧 Python Corral/Drover 程序。
- Agents 的 Git 摘要需要 `PATH` 中有 Git 2.45 或更高版本（依赖 `--no-lazy-fetch`）；更旧或没有 Git 时显示 `git unavailable`。
- 支持 Unicode 和鼠标的终端，建议支持真彩色。

目前在 macOS 上开发并进行交互验证，其他平台尚未验证。

### 构建和运行

```sh
git clone https://github.com/firegnu/saddle.git
cd saddle
cargo build --workspace --bins --release --locked
./target/release/saddle
```

也可以构建不可变产品目录（不安装、不切换正在运行的程序）：

```sh
./scripts/package.sh /absolute/new/saddle-version
/absolute/new/saddle-version/bin/saddle
```

可在 Agents 的 New 中启动 agent，也可接入已有 corral 会话。任务项目通过 Drover 插件登记；宿主不读取任务业务文件。

### 第一次使用

1. 在左侧选择 agent，按 **Enter** 或点击该行接入。
2. 在 Viewer 中直接输入，与 agent 交互。
3. 按 **Ctrl-]** 回到 Agents，Viewer 保持连接。
4. 任务功能需先安装启用 [Drover 插件](plugins/drover/README.md)，再从 **Plugins → Drover** 打开。Esc 返回插件子页面或关闭视图。
5. 从任意位置退出：先按 **Ctrl-]**，再按 **q**。退出会统一确认并结束运行中的 shell，断开 agent 的显示；corral agent 继续运行。

如果其他终端已经接入某个 agent，请先在那里断开，再通过 saddle 接入。停止 agent 是独立操作，需要确认。

## 启动 agent、标签页和分屏

Agents 的 **New / n** 打开可直接创建的表单：选 **Project**、选 **Codex**（默认）或 **Claude**，点击 **Create agent / Ctrl-S**。默认目录为 Saddle 启动目录；项目候选来自公开 Corral cwd 和启动目录，与 Drover 无关。点击项目选择框或 Ctrl-P 后，可鼠标选择已登记目录，也可上下键选择、Enter 确认；**Edit path / Ctrl-E** 可手填目录。**Role** 默认选 **Controller**（主控），名称锁定为只读 `main`；选择 **Regular**（普通 agent）后可编辑名称，首次值为 `main`，角色往返切换时保留普通名称草稿。名称前的 **Prefix** 默认 `agents`，两种角色下都可改（例如 `saddle`），须非空、不含空白和 `/`、不以 `-` 开头；完整名称为 `Prefix/Name`（如 `agents/main`），预览与实际调用一致。切换项目、Codex/Claude 或角色不改变前缀、名称和角色草稿。角色另记为公开标签 `role=controller` / `role=regular`，右侧终端窗格标题显示 `Controller · 名称` 或 `Regular · 名称`；其他入口带公开标签 `role=implementer` / `role=reviewer` 创建的 agent 显示英文 `Implementer · 名称` / `Reviewer · 名称`；没有有效角色标签的 agent 显示 `Agent · 名称`，空窗格仍为 `Viewer`。角色不配置队列或开启任务派发。两种角色都按精确名称调用公开 `corral start`，不加 `--unique`；重名时显示 CLI 报错并保留草稿，不自动编号。实际名称取 corral 返回值。

输入框有标签、框线、占位提示、焦点高亮及真实插入光标。点击内容定位光标，Tab／Shift-Tab 切焦点，左右键、Home／End、Backspace／Delete、Ctrl-U 清空及粘贴都按当前位置编辑，支持中文宽字符。长行横向滚动，多行消息还可上下移动、纵向滚动及 Enter 换行；小窗口可切焦点或用滚轮访问字段，底部保留创建和取消入口。

**Advanced / F4** 默认收起，包含完整命令、可选多行首条消息、**Open in** 打开位置（默认当前窗格，可选新标签页或四向分屏；内容选择入口打开的表单绑定该位置）和完整调用预览。明确点击 Codex／Claude 会把命令重置为 `codex --yolo`／`claude`；内置 Codex 默认使用 YOLO 模式。手填命令显示 **Custom command**，切焦点或收起高级设置不会丢弃，并按输入原样使用，不额外追加参数。命令支持引号分组，直接拆为 argv 调用公开 `corral start`，不展开 shell 变量、管道或重定向。PgUp／PgDn 或 Preview 聚焦后的滚轮查看完整预览；失败保留草稿。**Cancel / Esc**／Ctrl-] 返回 Agents，n 可重新打开草稿或查看正在进行的启动。

**Enter／点击 agent 行** 接入活动窗格；同一 agent 已经打开时跳到现有位置，不重复接入。布局从右侧开始，先选位置再选内容：活动窗格底边的 **Split ▾** 打开紧凑菜单 **Left ←**／**Right →**／**Above ↑**／**Below ↓**（方向键同样可用），方向相对于该窗格；标签条的 **+ Tab** 表示新标签页。随后弹出以位置为标题的列表（例如 *Open content on the right*），列出 **Terminal**、**New agent…**、**Plugin…** 和已有 agent，分屏时不列出该窗格自己的 agent。点击 agent（或 ↑↓ 加 Enter，列表长时可用滚轮）之后才创建窗格或 tab。已在别处打开的 agent 标出 **Move here**：选中后移动那个窗格，会话、输出和尚未完成的接入都随之移动，不重新接入，也不停止任何东西；它离开的分屏会合并，被搬空的 tab 会消失。任一步点 **Cancel Esc** 或按 Esc 回到 Viewer，Ctrl-] 回 Agents，布局保持原样；没有已有 agent 时仍可选 Terminal 和 New agent。New agent 打开绑定该位置的原有创建表单，取消不留空位。左右箭头访问放不下的标签；点击 tab 切换布局，点击终端内容或标题切换输入目标。终端内容区保留原有输入透传，Ctrl-] 回 Agents。

**Plugin…** 打开目标位置的插件列表。选择运行中的插件后，它会在该窗格显示，包括默认居中弹出的插件；已有视图标为 **Move**，移动原窗格并保留进程和状态。不能把插件分到自己旁边。取消不改变布局，关闭插件窗格后仍在后台运行。

每个 tab 保存自己的分屏和活动窗格，切换 tab 保留接入。**Close pane** 关闭活动窗格并合并分屏；**× / Close tab** 断开该 tab 的所有接入。最后一个 tab 关闭后留一个空 tab。关闭 agent 显示只断开自有 attach，agent 继续运行；含运行 shell 的关闭或退出先统一确认，取消不影响任何会话；停止仍走原来的独立确认操作。异步启动／接入始终归属于提交时预留的窗格，目标关闭或被替换后不会接到别处，也不会停止新建 agent。启动失败可能留下预留的空窗格。

尺寸不足以容纳某处分屏时暂时只画其中一侧，放大后恢复完整关系；内容区为空的终端不接收输入。布局变化时自动保存，正常退出时再保存一次。

下次启动自动恢复 tab、分屏方向与比例、窗格对应的 agent 名称和原启动目录。只有仍在运行且可接入的原 agent 实例会自动重新接入；已退出、同名换实例或暂时不可接入的 agent 保留位置与提示。占位中的 **Create new agent** 预填原名称、目录，以可编辑的 Regular 名称打开表单；命令和模型使用当前默认值，确认后才创建。**Choose existing agent** 选择已有 agent 放到该位置。普通终端只恢复占位，点击 **Open terminal** 才在原启动目录打开新的 shell，不重放命令，也不恢复终端历史或已结束对话。

布局文件默认是 `~/.local/state/saddle/layout.json`；绝对路径的 `XDG_STATE_HOME` 改为 `$XDG_STATE_HOME/saddle/layout.json`，与 `config.toml` 分开。首次启动或文件不存在时打开默认布局，首次保存自动建目录。文件损坏或版本不支持时提示并回退默认布局，保留原文件，本次运行禁止覆盖；如需重新保存，可先移走原文件再重启。保存失败只提示，不阻止使用；后续布局变化和正常退出时会再次尝试保存。

## 本地插件

打开 **Settings → Plugins（F5）**，添加可信的本地插件目录，再启用并打开 Saddle 内的面板。添加后默认停用；关闭面板保留进程，Disable 停止进程。登记保存的是目录路径，请保留该目录。

左侧固定 Plugins 入口打开可搜索的插件面板；列表显示运行状态，Enter 打开或切换已有界面。停用、失败项仍可选择查看原因；管理插件通过次要入口进入，不自动启用或重启。Counter 默认居中弹出，Esc 关闭并恢复来源焦点，Ctrl-] 返回 Agents；关闭仍保留插件进程。已有工作区面板优先聚焦，不重复创建。

开发者运行 `./examples/counter-plugin/package.sh`，即可构建并打包独立 Counter 示例；在设置中添加产物 `examples/counter-plugin/dist/counter-plugin`。使用者只需拿到这个目录，不需要 Rust 或开发环境变量。SDK 目前是开发接口，详见 [Counter 说明](examples/counter-plugin/README.md) 和 [插件开发入门](docs/插件开发入门.md)。Drover 插件迁移另行进行。

## 任务遥测

Saddle 核心提供独立 **Telemetry** 查询页（顶部入口，或 Agents 焦点下 `t`）；有任务号的 Drover 详情通过 **Telemetry ↗** 查看该任务所有轮次。源码已移除旧 Dispatch 日志页和运行时 dlog 依赖，**Dispatch selected** 显式派发动作仍保留。

记录可选、全局默认关闭，只采集显式选择的链路。随 dispatch 插件交付的技能包含同版本[遥测操作指引](plugins/dispatch/resources/corral-dispatch/遥测操作.md)，说明来源声明、每次派发身份、任务书快照、回复、审查、实际收尾和配对回执；业务不会为补遥测而重放。日常二进制、插件包、全局技能和项目指令**尚未实际切换**，安装与外部 dispatch-log 退役留给主控后续执行。旧历史保留供离线查看，不导入、不删除。

## 任务关联跳转（Drover 插件）

在 Tasks 右侧的 **Task text**、**Run details** 旁选择 **Links**；**Tab / Shift-Tab** 在三个页签间切换。Files、Commits、Agents 按明确引用分组并显示来源，点击条目或用上下方向键选择后 Enter 打开。文件和提交在 Tasks 内只读查看，方向键、滚轮、PgUp/PgDn 滚动；**Back / Esc** 回到原 Links 位置，再 Esc 关闭 Tasks。

仅收集任务正文及其明确 `Task file` 的一层内容，例如：

```text
任务文件: docs/task.md
审查文件: docs/review.md
产物: output/report.txt
提交: abcdef123
代理: project/worker | instance=012345abcdef
```

英文同义字段为 `Task file`、`Review file`、`Artifact`、`Commit`、`Agent`。允许列表前缀、中英文冒号、目标外的反引号，以及 Markdown 行内文件链接 `[说明](路径)`。字段路径以项目根为基准；Markdown 相对路径在正文中以项目根为基准，在任务书中以任务书目录为基准。忽略围栏代码块，不递归扫描链接文件。

文件预览限项目内不超过 1 MiB 的普通 UTF-8 文件；外部 URL、越界符号链接、上游内部数据、二进制和读取失败均提示原因，Markdown 原样阅读。**Recorded range** 使用公开起止 SHA（进行中取 observed HEAD），不表示区间中的提交都属于该任务；Git 输出有大小和超时限制，不运行外部 diff、textconv 或 pager。agent 必须记录原 12 位十六进制实例 ID；缺失身份禁用接入，退出、换实例或其他终端占用均拒绝。已打开的相同实例只定位，不重复接入。跳转不会启动 agent 或执行文档命令。

## 普通终端与 ctl 命令

Terminal 启动 `$SHELL -i`，缺失时回退 `/bin/sh`。目录取来源窗格的已知项目／启动目录，空窗格回退 Saddle 启动目录；不跟踪 shell 后续 cd。自行退出后保留屏幕与退出状态，不重启。关闭、替换运行中的 shell 或退出 saddle 时先确认，混合 tab 整体确认后统一关闭。只回收自有 PTY 的 shell 和前台进程组，不承诺回收主动 daemonize 的进程。

同一个二进制提供 JSON 命令，不启动第二个 TUI：

```sh
saddle ctl instances
saddle ctl inspect [--instance ID]
saddle ctl open --place right --shell
saddle ctl open --place tab --agent project/review
saddle ctl open --place down --name project/helper --cwd /absolute/project --role regular -- codex --yolo
saddle ctl request REQUEST --instance ID
saddle ctl close --pane PANE --instance ID
saddle ctl close --tab TAB --instance ID --confirmation TOKEN --confirm-shells
```

open 默认 `--relative-to self`，按 corral 名称与实例身份定位，或使用 shell 的 saddle 窗格环境。只有明确写 `--relative-to active` 才使用接收请求时的活动窗格；也可传查询得到的 Pane ID。place 为 tab/left/right/up/down。命令默认保留焦点，`--focus` 在提交时切换；异步完成不抢后来的焦点，也不清空用户表单。shell／新 agent 可传 cwd；新 agent 可传 prompt，名称精确使用，argv 不经 shell、不追加参数。

读取实际返回的 instance、request_id、pane、revision、cwd 和 cwd_source。starting／attaching 时用 request 查询；accepted、agent_created、pty 分别表示接收、创建和显示步骤，不代表模型就绪。超时保留未确定状态，用原请求 ID、相同参数查询或重试，不自动换编号重建。修改可传 `--request-id`，否则发送前生成。相同编号不同参数报冲突。每实例最多记录 256 次修改，满后拒绝新修改、保留旧记录；重启变更实例 ID，旧请求不可用。

含运行 shell 的 close 首次返回 confirmation_required、targets 和 confirmation，不改变布局。确认所列影响后，携带原凭据与 `--confirm-shells`，用一个新请求 ID 完成；重试这次确认操作时仍用这个新 ID。目标变化使旧凭据失效。正在操作布局弹层、表单或关闭确认时，远程修改返回 busy。没有 ctl stop、发键、读屏、运行脚本或调度接口；明确停止 agent 仍核对身份后用公开 corral stop。

每个 TUI 有独立随机实例 ID 和 Unix socket：目录 0700、socket 0600，优先 `$XDG_RUNTIME_DIR/saddle`，否则 `$XDG_CACHE_HOME/saddle/run` 或 `~/.cache/saddle/run`。绝对路径 `SADDLE_RUNTIME_DIR` 可覆盖目录，测试完全隔离；路径超过 Unix socket 限制时明确报错。发现只列活实例，不选“最新”；退出只移除自己的 socket。传输限制 64 KiB、有界队列／线程和超时。实际语法见 `saddle ctl --help`，agent 操作说明见 [skills/saddle/SKILL.md](skills/saddle/SKILL.md)。

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

```

| 配置项 | 含义 |
|---|---|
| `corral` | corral 命令名或路径 |
| `left_width` | 左栏期望宽度，单位为终端字符格 |
| `left_split` | 已不再使用：Agents 独占左栏。为兼容既有配置仍接受（大于 0 且小于 1） |
| `refresh_ms` | 后台刷新间隔，单位毫秒 |


| `colors` | 可选的平铺颜色表，控制界面和 agent 类型配色 |

命令路径支持 `~/`。旧 `[queue]` 表仍可加载，但不再驱动宿主。自定义 Corral 命令和目录转到插件清单 args，见[插件说明](plugins/drover/README.md)。旧 `--dispatch-log <值>` 兼容解析但已弃用且不使用，新清单不提供。插件通过公开 schema 2 list/show/action 接口工作，宿主不再读取 Drover 项目或任务数据。

[config.toml](config.toml) 提供完整默认配置与简短注释，可直接复制到上述位置，默认呈现与原界面一致。只想改几项时，可添加：

```toml
[colors]
focus = "light_cyan"
bg = "default"
agent_selected = "#2b2621"
```

颜色支持 `default`（或 `reset`，终端默认色）、`#RRGGBB` 和小写 ANSI 色名：`black`、`red`、`green`、`yellow`、`blue`、`magenta`、`cyan`、`gray`、`dark_gray`、`light_red`、`light_green`、`light_yellow`、`light_blue`、`light_magenta`、`light_cyan`、`white`。其中 `gray` 是普通 ANSI 白，`dark_gray` 是亮黑，`white` 是亮白；ANSI 色随终端调色板变化。

颜色表覆盖背景、选中底色、边框、焦点、文字层次、连接/未读标记、操作反馈、agent 类型及回复格式。Agents 栏使用自己的 `agents_*` 配色（另加 `agent_selected` 和 agent 类型色）；共用的 `agent_*` 状态强调色用于 Agents 的 stalled/starting 状态和 effort 图标。插件接收通用文字/背景/强调/错误色，自行管理业务状态配色。终端未把 `COLORTERM` 设为 `truecolor` 或 `24bit` 时，Agents 专用的 RGB 颜色按最近的 256 色发送；ANSI 颜色名和其他区域不变。未知字段和无效颜色沿用配置错误报告。在 Settings 中保存的颜色立即生效；在 saddle 外修改的配置下次启动生效，不支持热加载；Viewer 的终端输出保留自己的颜色。字体和字号仍由外部终端设置控制。

## 操作

| 位置 | 输入 | 操作 |
|---|---|---|
| Agents | ↑↓ / j k | 选择 agent |
| Agents | Enter / 点击行 | 接入活动窗格，已打开时跳到现有位置 |
| Agents | n / New | 创建新 agent |
| Agents | / / Search | 按项目名或 agent 名称过滤；Enter 或点击进入对应终端（已打开时跳到现有位置），Esc 取消 |
| Agents | a / Attention · N | 打开 Attention；↑↓ 选择，Enter 或点击打开对应 agent 或任务，Esc 取消 |
| Agents | , / Settings | 打开 Settings；Tab/↑↓ 选择设置项，F1–F5 或点击切换页签，Ctrl-U 清空，Ctrl-D 恢复默认值，Ctrl-S 保存，Esc 取消 |
| Viewer 边框 | Split ▾ 后选方向 / + Tab | 先选分屏方向或新标签页，再选 Terminal、New agent、Plugin 或要打开／移动的已有 agent |
| Viewer 边框 | Zoom / Restore | 多窗格时让当前窗格临时占满右侧终端区域（保留 Agents 和 tab 条）；Restore 回到原分屏和比例，焦点仍在该窗格。其他窗格继续运行；切到其他窗格、关闭该窗格或新建分屏都会结束放大 |
| Agents | 鼠标滚轮 / 触控板 | 滚动列表，不改变选择 |
| Agents | Tab / Shift-Tab | 聚焦 Viewer |
| 任意位置 | Plugins | 搜索并打开/切换插件视图 |
| Agents | PgUp / PgDn | 滚动 agent 列表 |
| Agents | s / Sort | 仓库内按状态（默认）/ 名称排序 |
| Agents | z / Fold | 折叠未选中的 agent 为一行，或重新展开 |
| Agents | x / Stop，然后 y | 停止选中 agent，其他键取消 |
| Agents | q | 退出 saddle |
| Tasks | ↑↓ / j k / 点击任务 | 选择任务，右侧显示它的原文或运行详情 |
| Tasks | t / Task text、Enter / Run details | 显示任务原文 / 运行详情 |
| Tasks | PgUp / PgDn | 滚动原文或运行详情 |
| Tasks | 鼠标滚轮 / 触控板 | 滚动指针下的列表或内容，不改变选择 |
| Tasks | c / 项目选择 | 打开项目选择页 |
| 项目选择页 | Enter / 点击、e、r | 打开项目、手动目录、重读登记清单 |
| 路径表单 | Ctrl-U / Enter / Esc | 清空 / 应用 / 取消 |
| Tasks（插件） | r | 刷新；派发/提交/接受使用显式任务按钮 |
| Tasks（插件） | p / N | 暂停或恢复派发 / 通知偏好 |
| Tasks | a / ? | 新增任务 / 帮助 |
| Tasks，选中待办 | e / u / d / x | 编辑 / 上移 / 下移 / 删除（按 y 确认） |
| Tasks | A | 查看所有登记项目的待办 |
| 所有待办页 | 鼠标滚轮 / PgUp / PgDn、r、Esc | 滚动 / 重新读取 / 返回 |
| 新增 / 编辑表单 | Tab / Ctrl-S / Esc | 切换字段 / 保存 / 取消 |
| 帮助 / 反馈等页面 | Esc / Back | 返回列表 |
| Tasks | Esc / Close、q | 关闭 Tasks，回到打开前的输入目标；文本字段中 q 作为普通文字 |
| Tasks / Viewer | Ctrl-] | 返回 Agents（Tasks 关闭，保留状态） |

选中待办后可用 Tasks 底部的 **Edit**、**Move up**、**Move down**；其他状态不能编辑或调序。编辑在同一个弹窗中打开完整表单，预填标题和多行正文，保存或取消后回到原任务和原视图；刷新及保存失败保留草稿，成功后保持选中被操作任务。首项不能上移、末项不能下移。写入前会重新核对公开待办快照，内容或顺序过期时拒绝操作；当前 CLI 没有公开的版本获取入口，预检查和写入之间仍有并发改动的窗口。

**Delete x** 打开确认弹层，显示选中待办的位置、id、标题和正文；按 **y** 或点 **Delete** 确认，按 **Esc** 或点 **Cancel** 保留。确认目标在弹层打开时固定，刷新不会改变它。Drover 插件做同样的待办核对后运行 `drover drop --pos <位置> "Deleted in saddle"`：任务移出待办，drover 在 History 中保留为 **Dropped**（带这个原因），不是抹掉记录。这里只能删除待办；运行中的任务使用独立的 Return to pending 确认。

**All pending A** 在 Tasks 弹窗中打开只读页，列出 `~/.drover/projects` 中每个项目的待办，按项目分组，显示任务在队列中的位置、id 和完整标题。每个项目在后台各自运行 `drover list --json`；仍在读取或读取失败的项目会如实标出并显示完整错误，其余项目照常显示。按 **r** 重新读取。该页不编辑、不调序；要操作某个项目的队列，请切换到该项目。

Viewer 将输入传给 agent，**Ctrl-]** 除外。底栏显示当前输入目标。弹层打开时只处理自身输入，背景控件不响应。Tasks 打开期间按键和鼠标只作用于它，下面的终端继续运行、不改变大小。关闭再打开 Tasks 会保留项目、选中任务、原文/详情视图、滚动位置和未提交的新增/编辑草稿。

**任务原文与运行详情：** 选中的任务显示在列表右侧，默认是完整标题和正文（**Task text t**）；**Run details ↵** 切到它的状态与过程，所选视图在切换任务时保持。对当前、待放行和历史任务，运行详情来自只读的 `drover show <id> --json --with-agent-status`，在项目目录下运行：显示时立即查询，保持显示时约每 5 秒再查一次，同一时间只有一个查询；切回原文、换任务、切换项目、关闭 Tasks 或退出即停止。详情按任务 id 跟踪，任务完成或放行后仍显示同一个任务。内容包括：

- 状态、耗时，以及关注提示或快照不一致的警告；
- 现在重算的完成依据与判据及各自原因；历史任务的完成时判据没有保存，不套用现在的结果；
- 上次验收命令的记录，只是之前的结果，详情页不会执行验收；
- Git 进展：start..HEAD（或 start..end）区间提交数（不是本任务独占的提交数）、main 的前进，以及记录的和现在的提交；
- 来自当前任务文件的路由、Hold，以及关注提示（这是推断，不代表已经停工）；
- 开始、完成、放行时间和正文。

未知或未记录的值会明确标出，不显示成 0 或通过。刷新失败时显示错误，并把之前的数据标为旧数据，下一次刷新自动重试。`drover show` 不覆盖待办和未编号任务，它们的运行详情显示列表已有的标题、正文和状态；待办开始后自动转为完整详情。

**Pause 作用于打开的项目；Dispatch selected、Submit for acceptance、Accept 和 Return to pending 作用于明确选中的任务。**读取失败会禁用旧数据上的操作。没有待处理工作时显示 `No active tasks`，历史记录仍可查看，底部显示可见范围。

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

### 外部 Corral 与升级

默认 `corral = "corral"` 从 PATH 查找 ranch 安装的命令；显式配置命令名、路径和 `saddle agent --corral` 覆盖仍保留。宿主开停、接入、插件默认调用和 Updates 比较及升级均使用外部 Corral，不再依赖 Saddle 同目录的程序。普通终端继续使用 `corral start/attach/status/stop`，无需先打开 Saddle。

Saddle 定位为保底版，不再加新功能，只保持与运行时兼容。打包不含 `bin/corral` 和 `share/corral/`；部署不切换 `~/.local/bin/corral`、不安装 corral 技能，二者由 ranch 管理。`~/.local/share/saddle/versions/` 下旧版本目录先保留，现有会话仍可能使用其中的 corral。遥测、dispatch 和插件协议的迁出等待后续说明。Corral 实现与设计文档现由 ranch（`../ranch`，paddock 主控兼管）维护。
