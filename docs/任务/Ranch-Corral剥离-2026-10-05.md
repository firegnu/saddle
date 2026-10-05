# Corral 转出 ranch

2026-10-05，用户转来 paddock/main 的已定安排，随后明确「你自己改，不要委派了」。本任务由 saddle/main 直接实施，没有创建开发或审查 agent，不操作 Tasks 队列，不启用本链路遥测。

## 用户要求

paddock 那边定了新安排（用户 10-05）：corral、遥测、dispatch 和插件协议独立成新仓库 ranch（../ranch，github.com/firegnu/ranch），由 paddock 主控兼管；Saddle 和 paddock 都只是前端，只调用 ranch 装好的命令。Saddle 定位为保底版：不再加新功能，只保证和运行时对得上。

请在 Saddle 的流程里把 corral 剥离出去（遥测和 dispatch 到时另有说明）：
1. 删掉 crates/corral-core 和 docs/Corral核心Rust集成设计.md、docs/Corral通用升级设计.md（都已迁到 ranch），从 workspace 成员里去掉。
2. 打包（scripts/package.sh）不再带 bin/corral、share/corral/；部署不再切换 ~/.local/bin/corral，也不再安装 corral 技能。这两样归 ranch。
3. Saddle 开停接入 agent、插件都改用 PATH 上的 corral（ranch 装好的那份），不再用和 saddle 打包在一起的那份（src/agent_program.rs 的 bundled() 和默认配置）。
4. Updates 页按 PATH 上的 corral 来比较和升级。
5. AGENTS.md 里“corral 由 paddock 维护”那句改为：corral 由 ranch（../ranch，paddock 主控兼管）维护，Saddle 只通过 corral 命令使用它。
6. ~/.local/share/saddle/versions/ 下的旧版本目录先别删，还有会话在用里面的 corral。

做完后告诉用户，paddock 主控会用测试 agent 核对 Saddle 和 ranch 的 corral 能不能配合。

## 实施与边界

- 独立分支/worktree：`ranch-corral-extraction`。先在 DESIGN 记录已定边界，再移除 Corral crate、两份设计文档、workspace/锁文件条目和包内程序/资源。
- 默认配置字面值仍为 `corral`，从 PATH 解析为绝对路径；显式路径和命令名配置继续有效。不存在的 PATH 命令返回错误，不意外执行当前目录或包内副本。宿主开停/接入与 headless agent 沿用已有统一解析入口；插件默认构造入口同步。插件协议和显式覆盖优先级不变。
- Updates 的默认 Corral 查找独立于 Saddle 安装目录，仍调用同一公开升级命令，保留确认、回执、逐 agent 状态与不重发语义。
- 仓库只有打包脚本，没有独立部署脚本；部署限制写入 AGENTS、DESIGN 和中英文 README。历史部署记录保留原样；DESIGN 中已迁走文档的链接改向相邻 ranch 仓库。
- 不修改 ranch、遥测、dispatch 或插件协议；不修改真实配置/技能/插件登记，不切换任何安装链接，不操作现有 agent。旧 Saddle 版本目录保留。

## 验证记录

跨模块行为与工作区构建变化，按 `docs/UI回归.md` 运行直接回归及全量标准检查。所有 Cargo 命令使用共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。测试使用临时目录/假 Corral。

- RED：`cargo test -p saddle --test agent_program` 的新默认规则在旧实现上实际得到 `bundle`，预期 `path`，1 项失败。改动后该项通过。
- RED：`cargo test -p saddle --test updates default_corral_uses_path_independently_of_the_saddle_package -- --exact` 在旧实现上实际选择 `new/bin/corral`，预期 `ranch/bin/corral`。测试将 PATH 放在独立子进程中，不改变并行测试环境。
- 插件环境传递测试同样使用子进程 PATH 与假 Corral，验证 `SADDLE_AGENT_BIN` 等于 PATH 中的命令；保留显式路径与身份变量剥离断言。
- GREEN：`cargo test -p saddle --test updates --test agent_program --test plugins --test diagnostics --test settings --test ui_second_settings`：85 通过，1 项原有 ignored（SDK stdio probe 需要额外环境），0 失败。Updates 的 PATH 与缺失 Saddle 安装场景、假 Corral 升级调用均通过。
- `scripts/package.sh /tmp/saddle-ranch-package.tW618F/product` release 构建成功。包文件为 `BUILD.txt`、`bin/saddle`、两个插件程序与各自清单；无 `bin/corral`/`share/corral`，三个程序 SHA-256 与 BUILD.txt 一致。`bin/saddle --help` 成功。此包来自未提交工作区，仅作验证，没有安装；构建记录如实标注 modified。
- `cargo metadata --format-version 1 --no-deps` 确认工作区无 corral-core；`sh -n scripts/package.sh`、`cargo fmt --all --check`、`git diff --check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。

## 主控差异审查

- 删除范围为已迁走的 Corral crate 与两份设计文档；保留现有遥测、dispatch、插件协议实现及历史记录。
- 宿主默认解析、插件构造入口和 Updates 均不再拼出同目录 Corral；默认配置仍是命令名 `corral`，显式配置继续优先。Updates 的升级参数、确认和回执处理未改。
- 打包清单和 checksum 清单同步移除 Corral；仓库不存在另一个安装脚本。部署归属约束已写入当前项目指引，中英文 README 已同步。
- 使用独立临时目录及假程序完成验证，未安装/切换程序、未运行真实升级、未开停或送话给任何现有 agent。真实 Saddle/ranch 配合核对留给 paddock 主控。

## 全量结果与定向复核

- `cargo test --all-targets --no-fail-fast` 完整运行 67 个 target，合计 **661 passed / 6 failed / 5 ignored**，退出 101。`--no-fail-fast` 用于收齐后续 target 结果，没有改变测试并发、断言、预算或 ignored 规则。
- `tests/app.rs` 三项失败：`agent_workspace_starts_and_restores_terminal_after_quit`、`settings_open_with_comma_save_to_the_file_and_resize_the_sidebar_at_once`、`telemetry_page_takes_all_input_and_closes_back_to_where_it_opened`。与 `T77-遥测展示整理主控审查.md` 已记录的旧头部入口断言一致，未复跑/修复。
- `tests/agent_capture.rs` 的 `m1_missing_host_final_does_not_accept_a_business_boundary_pair` 首次退出码实际 124、期待 127；`m1_start_backpressure_never_replaces_the_host_anchor_with_business_text` 首次没有在原 5 秒内看到 ready barrier。这两项都显式使用假 Corral，测试及相应 agent/telemetry 执行代码没有本轮 diff。
- `tests/workflow.rs::native_mouse_buttons_cover_forms_and_stop_confirmation` 首次在 858 行失败；同位置曾在 T77 集成记录出现。
- 对上述两项 agent_capture 与一项 workflow，逐项运行全量检查产出的原测试程序，参数 `--exact <完整测试名> --nocapture`，每项只复核一次：**3 passed / 0 failed**。首轮失败仍保留；定向通过不代表时序根因已修复。没有改产品预算、超时、sleep 或断言。
- 全量中的 `drover_telemetry` 本次 15 项全过，包括历史交接里曾失败的 disabled/budget_exhausted 项；不据此宣称那个历史问题已修复。
- 日志：`/tmp/saddle-ranch-all-tests.log`、`/tmp/saddle-ranch-targeted.log`、`/tmp/saddle-ranch-clippy.log`、`/tmp/saddle-ranch-package.log`；逐项复核命令及退出码在 `/tmp/saddle-ranch-recheck.json`，对应输出在同前缀的 recheck 日志。

结论：本次 Corral 剥离的直接回归、静态检查和 release 打包通过，可以合并。全仓仍有上述已知 UI 断言失败，首次全量另有三项复核通过的波动，不宣称全仓全绿。没有真实部署或真实会话配合验收；由 paddock 主控后续用测试 agent 核对。
