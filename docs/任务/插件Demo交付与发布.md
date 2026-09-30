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
