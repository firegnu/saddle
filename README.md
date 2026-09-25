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

Agents 的附加信息跟随每个条目显示，包括实例、ATT、SOURCE、目录、标题和活动；长内容折行。`r` 打开的 Last reply 是独立区域，不替换这些信息。Queue 顶部是当前项目及项目操作，底部是详情、新增和帮助；放行/下一件/暂停/循环始终作用于当前项目，与选中的历史任务无关。

左侧两个面板使用终端默认背景，选中项用竖线与文字强调标识。原生按钮采用英文标签、圆角细边框，无填充底色；悬停提亮、按下强调，按下后在同一按钮内松开才执行，移出即取消；不可用操作置灰。项目选择、任务详情、新增、帮助和操作反馈在弹层中显示。停止 agent 有独立确认弹层，只有 `y` 或确认停止按钮会执行。弹层打开时，背后的窗格不会接收点击或输入。

## 按键

| 位置 | 按键 | 操作 |
|---|---|---|
| Agents | ↑↓ / j k | 选择 agent |
| Agents | Enter / 点击行 | 接入 Viewer 并切换焦点 |
| Agents | Tab / Shift-Tab | 焦点到 Queue / Viewer |
| Agents | r | 显示/隐藏上一轮回复 |
| Agents | PgUp / PgDn | Last reply 打开时翻回复，否则翻列表 |
| Agents | s | 项目内按名字/状态排序 |
| Agents | x 然后 y | 停止选中的 agent；其他键取消 |
| Agents | q | 退出 saddle |
| Queue | ↑↓ / j k、鼠标点击/滚轮 | 选择任务 |
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
