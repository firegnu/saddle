# 插件 Demo 交付与发布

用户原话：“可以的”。对应主控提出的三项：发布当前 Saddle；提供一条插件打包命令，生成包含 plugin.toml 和程序的目录；补齐 Counter 开发说明。延续用户要求，由主控亲自完成，不委派。

分支 plugin-delivery，worktree ../saddle-worktrees/plugin-delivery；不加入或推进任务队列，不迁移 Drover，不操作其他 agent。发布更新日常二进制，保留当前运行窗口，重开后使用新版。

## 工程检查

本轮不改变宿主/SDK 协议或界面行为，新增示例交付脚本与文档。脚本采用实际构建、输出目录和已有插件 PTY 流程检查，不为构建配置制造失败测试；发布前运行标准检查，保留既有失败事实。执行结果在收尾时补充。

## 主控审查与验证

- 变更仅为示例 package.sh、示例忽略目录与说明、开发入门、README 入口和设计/记录；宿主、SDK、测试及锁文件均未改变。脚本调用 Cargo 的公开本地安装能力将指定二进制交付到输出目录，使用 --no-track，不写全局安装登记。原清单只在构建成功后复制。
- 脚本 sh -n、文档本地链接、git diff --check 通过。属于构建交付配置，使用真实打包与已有行为检查验证，没有伪造 RED。
- 示例复制到 `/tmp/saddle-delivery-4oa051k7/counter source` 后，从 `/tmp` 调用 package.sh，使用固定 Git SDK 依赖及共享 target 成功打包；默认和含空格的自定义输出目录均通过。产物恰为 plugin.toml 和 bin/saddle-counter，清单入口存在、有执行权限，是复制的程序而非符号链接，无 Cargo 安装追踪文件。
- 用上述独立打包产物显式运行 `plugin_counter_installs_opens_notifies_and_closes_without_stopping`：1 passed。覆盖实际 Saddle PTY 中的添加、启用、面板、键鼠计数、通知、Shift+Enter、关闭重开、重启及停用。数据在测试临时环境，Corral/Drover 为夹具，不使用用户工作区。
- 标准 `cargo test --offline --all-targets --no-fail-fast`：306 passed / 1 failed / 6 ignored。失败 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal`，workflow.rs:2508，picker 仍显示；这是上一轮已经在旧基线记录的 picker 失败，本轮没有更改其实现或测试。不声称全绿，不扩大到 T29。
- `cargo clippy --offline --all-targets -- -D warnings` 通过。日志 `/tmp/saddle-plugin-delivery-{tests,clippy,package,package-custom,pty}.log`。全部构建/测试使用共享 CARGO_TARGET_DIR。

审查结论：本轮交付范围通过，已知 picker 不稳定保留；可以合并并按用户授权更新日常 release。没有新增宿主协议/行为，也没有迁移 Drover。发布与备份结果后续记在本文件及 HANDOFF。

## 发布结果

实现 6d39a06，合并 27f3279；主控完成审查并推送 main，清理 plugin-delivery 分支/worktree，空提交收尾 fc2c060。没有创建或操作 agent。

用户已在临时开发版确认 Counter 效果，随后授权日常发布。本轮从 main 构建 release，核对日常入口 `/Users/firegnu/.local/bin/saddle` 指向 `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`，SHA-256 `99695dbeb72724160443418be64809e61b33995badfa7d7515d2a17ed29961da`。旧二进制与前后哈希保存在 `/Users/firegnu/Library/Application Support/saddle-release-backups/plugin-20260930-170510`；旧哈希 e63d8e5ab76012e1b67e34b9232636a5ee3067972ea2382771f147a1d6d9dd76。入口链接保持不变。

在 main 使用 package.sh 生成正式示例目录 `/Users/firegnu/Developer/personal_projs/saddle/examples/counter-plugin/dist/counter-plugin`；未登记进日常配置。随后用 `cargo test --offline --locked --release --test workflow plugin_counter_installs_opens_notifies_and_closes_without_stopping -- --ignored --exact` 验证实际 release 宿主与正式打包程序，1 passed；仍为假 Corral/Drover、临时配置与 PTY，无真实状态修改。日志 `/tmp/saddle-plugin-delivery-release.log`、`/tmp/saddle-plugin-delivery-package-main.log`、`/tmp/saddle-plugin-delivery-release-pty.log`。

发布更新的是磁盘上的日常启动版本，没有退出/重启用户现有 Saddle，没有改日常插件登记、队列或通知服务；用户下一次正常启动 Saddle 即使用新版。用户重启后的现场验收尚未发生，不宣称已经切换现有窗口。
