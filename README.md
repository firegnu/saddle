# saddle

Rust 终端工作台：左上原生 corral Agents 面板，左下运行 `drover board`，右边运行选中 agent 的 `corral attach`。

## 运行

需要 Rust stable 1.96+，以及 PATH 中的 `corral` 和 `drover`。在仓库目录运行：

```sh
cargo run --release
```

编译后也可直接运行 `./target/release/saddle`。程序只调用公开 CLI；退出只断开本程序的 attach、结束本程序的 Queue 命令，agent 由 corral 继续托管。

## 配置

默认读 `~/.config/saddle/config.toml`；没有文件就用以下默认值。也可用 `saddle --config /path/to/config.toml`。

```toml
corral = "corral"
left_width = 52
left_split = 0.5
refresh_ms = 1000

[queue]
command = ["drover", "board"]
# cwd = "~/Developer/personal_projs/drover"
```

`command` 是可执行文件和参数的数组，不经过 shell；需要 shell 语法时显式写 `["sh", "-c", "..."]`。没有设置 `cwd` 时继承启动 saddle 的目录。命令路径和 cwd 支持 `~/`。

左列在窄屏时最多占一半。无效配置会在进入终端界面之前报错；Queue 启动失败在窗格中显示，命令退出保留最后输出。

## 按键

| 位置 | 按键 | 操作 |
|---|---|---|
| Agents | ↑↓ / j k | 选择 agent |
| Agents | Enter / 点击 agent 行 | 接入 Viewer 并切换焦点 |
| Agents | Tab / Shift-Tab | 焦点到 Queue / Viewer |
| Queue / Viewer | Ctrl-] | 回 Agents，保持 attach 连接 |
| Agents | r | 显示/隐藏上一轮回复 |
| Agents | PgUp / PgDn | 回复翻页 |
| Agents | s | 项目内按名字/状态排序 |
| Agents | x 然后 y | 停止选中的 agent；其他键取消 |
| Agents | q | 退出 saddle |
| 所有窗格 | 鼠标点击 | 切换焦点 |

Agents 列表滚轮只滚动列表，不改变选择；回复区滚轮滚动回复。Queue/Viewer 的普通按键、鼠标和粘贴交给内层程序；只有 Ctrl-] 被截取。终端可区分的 Shift-Enter 会保留为修饰键序列。已有其他窗口接入的 agent 会被拒绝，先在原窗口断开再接入。

显示 `▶` 的是正在 Viewer 中接入的 agent。`!` 表示 blocked，`?` 与 board 一样表示启动或输出长时间没有进展，`●` 表示未查看的已完成轮次。

## 验证与范围

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run --example compare_parsers
```

自动测试只使用临时目录中的假 corral/queue 和合成终端流，不启动或操作真实 agent。完整程序测试在 PTY 中验证接入/切换、输入/粘贴/鼠标、刷新、回复、停止确认、缩放、输出压力和退出恢复。

当前实现设计第 1 步。真实 Claude Code / drover 的显示、手感及真实 corral 生命周期仍需用户按 [设计第 11 节](docs/DESIGN.md#11-第-1-步怎么算做完)目视验收。第 2、3 步原生 Queue 面板尚未实现。详细记录见 [PROGRESS](docs/PROGRESS.md)。
