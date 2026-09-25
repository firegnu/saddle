# saddle

Rust 终端工作台：Agents 和 Queue 都由 **ratatui 原生绘制**，Viewer 用 Rust 终端解析器绘制 `corral attach` 的输出。不会启动 `corral/tools/board`、`drover board` 或其他外部看板 UI。

## 运行

需要 Rust stable 1.96+，以及 PATH 中的 `corral` 和 `drover`：

```sh
cargo run --release
```

编译后可直接运行 `./target/release/saddle`。退出只断开本程序的 attach，agent 继续由 corral 托管。

## 配置

默认读取 `~/.config/saddle/config.toml`；文件不存在时使用默认值。也可用 `saddle --config /path/to/config.toml`。

```toml
corral = "corral"
left_width = 52
left_split = 0.5
refresh_ms = 1000

[queue]
drover = "drover"
# cwd = "~/Developer/personal_projs/your-project"
```

Queue 自动读取 `~/.drover/projects` 中的登记项目，点击 **项目** 按钮可切换。启动时优先使用 `queue.cwd`；没有配置时，若启动目录已登记就选它，否则选登记的首个项目；无登记时尝试启动目录。项目页也可点击 **目录** 手动输入，切换仅本次运行生效。目录应已接入 drover，saddle 不会自动初始化它。命令路径和 cwd 支持 `~/`。旧的 `queue.command` 配置已移除。

底栏始终显示当前输入目标。宽度 ≥140 列时左侧默认 52 列，100–139 列时最多 44 列；小于 100 列时左侧最多 34 列，Agents/Queue 收为可点击标签，Viewer 始终保留。极窄窗口左侧不超过一半。数据读取与操作在后台执行，读取失败会显示完整、可滚动的错误，成功后恢复。除用户授权的项目登记清单外，saddle 不读取 corral/drover 的内部文件；任务数据和操作全部使用公开 CLI。

Agents 顶部显示总数，每个 repo 行右侧显示分组数量；按 repo 树形分组（├─/└─），宽窗优先显示完整名称，同组类型列对齐，条目间留一行空白；类型在主行可见时第二行不再重复，路径弱化显示。附加信息跟随每个条目显示，包括 6 位实例 ID、ATT、VIA、目录、标题和活动；路径只显示末两级，目录和标题不加标签，长内容折行。状态文字加粗并按 working 蓝、idle 绿、blocked 黄、stalled 橙、error 红、starting 紫着色。宽窗主行类型名使用字符标识（✳ claude、>_ codex、π pi、π omp），Claude 保留陶土橙，Codex 使用薄荷青加粗；无需图标字体；窄窗仍在附加信息显示类型。Agents 界面标签和状态使用英文，源数据保持原文。鼠标在列表正文或滚动条上均可滚动，内容与滑块同步到底；手动滚动保持位置，不改变选择或接入对象。Last reply 的读取/渲染功能保留，但 Reply 按钮隐藏，Agents 的 `r` 暂不响应。Queue 顶部是当前项目及项目操作，底部是详情、新增和帮助；放行/下一件/暂停/循环始终作用于当前项目，与选中的历史任务无关。

界面使用终端默认前景/背景和 ANSI 调色板，跟随 Terminal 主题；Agents 的选中背景、状态和品牌标识使用独立强调色，Viewer 内容保留原色，暂不提供配色配置。Agents 选中条目整行使用薄暖灰底色与竖线，Queue 选中项用竖线与文字强调。原生按钮均为英文、无填充底色；Agents、Queue 及 Queue 弹层使用单行轻量边界（如 ‹Attach ↵›），停止确认保留圆角细边框；悬停提亮、按下强调，按下后在同一按钮内松开才执行，移出即取消；不可用操作置灰。项目选择、任务详情、新增、帮助和操作反馈在弹层中显示。停止 agent 有独立确认弹层，只有 `y` 或确认停止按钮会执行。弹层打开时，背后的窗格不会接收点击或输入。

Queue 任务列表支持鼠标滚轮和触控板直接滚动，正文与滚动条区域均响应；不改变选中任务，正常刷新后保留位置，点击或用方向键选择任务时恢复跟随。历史列表可滚到两端，滑块与内容同步。

## 按键

| 位置 | 按键 | 操作 |
|---|---|---|
| Agents | ↑↓ / j k | 选择 agent |
| Agents | Enter / 点击行 | 接入 Viewer 并切换焦点 |
| Agents | Tab / Shift-Tab | 焦点到 Queue / Viewer |
| Agents | PgUp / PgDn | 翻动 agent 列表 |
| Agents | s | 项目内按名字/状态排序 |
| Agents | x 然后 y | 停止选中的 agent；其他键取消 |
| Agents | q | 退出 saddle |
| Queue | ↑↓ / j k、鼠标点击 | 选择任务 |
| Queue 任务列表 | 鼠标滚轮 / 触控板 | 直接滚动列表，保持任务选择 |
| Queue | c | 打开已登记项目列表；点击或 Enter 切换 |
| Queue 项目页 | e / r | 手动目录 / 重读登记清单 |
| Queue 目录表单 | Ctrl-U / Enter / Esc | 清空路径 / 应用 / 取消 |
| Queue | Enter / Esc | 详情 / 返回列表 |
| Queue | PgUp / PgDn | 滚动详情或操作反馈 |
| Queue | ? / h | 原生帮助页 |
| Queue | r / g / n | 刷新 / 核对放行 / 下一件 |
| Queue | p / l | 暂停或恢复 / 循环开关 |
| Queue | a | 原生新增表单 |
| Queue 新增表单 | Tab / Ctrl-S / Esc | 切字段 / 提交 / 取消 |
| Queue | q | 返回 Agents（表单内作为文字输入） |
| Queue / Viewer | Ctrl-] | 回 Agents；Viewer 保持连接 |
| 所有窗格 | 鼠标点击 | 切换焦点（弹层内仅操作弹层） |

Viewer 除 Ctrl-] 外的按键、鼠标和粘贴交给 agent；终端可区分的 Shift-Enter 保留为修饰键序列。Queue 所有操作由 Rust 控件处理，表单不会启动外部编辑器。新增失败保留草稿；Ctrl-] 暂回 Agents 后，Tab 回 Queue 可继续编辑；Esc 取消。执行期间不会重复提交。

已有其他窗口接入的 agent 会被拒绝，先在原窗口断开再接入。Agents 的青色 `◉` 表示 Viewer 正在连接；`◆` 表示待处理，`▲` 表示长时间没有进展，紫色“新”表示未查看的已完成轮次。选中行竖线、连接标记和琥珀焦点边框分别表示三个独立状态。

## 验证

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run --example compare_parsers
cargo run --example ui_preview -- /tmp/saddle-ui-preview
```

`ui_preview` 直接把 Ratatui 的合成网格导出为 SVG 和文本，包含三种尺寸、项目选择、新增、停止确认和回复状态，不运行任何 agent。

默认测试使用临时目录中的假公开 CLI 和合成终端流，不启动真实 agent。测试覆盖两个原生看板、Viewer 接入/切换、输入/粘贴/鼠标、刷新、回复、停止确认、原生 Queue 表单及操作、缩放、输出压力和退出恢复。

安装了 drover 时，可运行真实公开 CLI 集成测试；它只使用隔离 HOME 和临时项目，不调用外部 UI：

```sh
SADDLE_DROVER_BIN="$(command -v drover)" cargo test --test workflow installed_drover_cli -- --ignored
```

实现与验收记录见 [PROGRESS](docs/PROGRESS.md)。按用户约束，没有启动真实 Claude Code 会话，因此不将合成测试作为真实 agent 显示和生命周期的证明。
