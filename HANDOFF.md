# 会话交接

更新：2026-09-30。Drover + Saddle 简化流程已联合发布。用户已用真实 T57 验证待验收通知并接受，T57 为 Done。新文档测试 T58 已完成委派、审查、合并、推送和清理，done 返回 awaiting_release；随后公开状态出现接受记录，现为 Done。主控未执行 go，不自动派发。用户随后确认进程式插件长期架构及在 Saddle 内绘制界面；主控已亲自完成首个插件宿主、Rust SDK 与独立 demo；用户在临时开发版确认效果后授权日常发布。现已更新日常 release，补齐一条命令打包与开发入门；没有退出用户窗口，重开后使用新版。

## 当前状态

- 最新交付：6d39a06（示例 package.sh、开发入门及 README），合并 27f3279，收尾 fc2c060；plugin-delivery 分支/worktree 已清理，main 已推送。主控亲自完成，没有委派、队列或上游变更。
- 用户一条打包命令：`./examples/counter-plugin/package.sh`；正式产物已生成在主仓库 `examples/counter-plugin/dist/counter-plugin`。在日常新版 Settings → Plugins 中添加该完整目录、Enable、Open panel；本轮没有替用户登记。
- 最新标准检查：306 passed / 1 failed / 6 ignored，Clippy通过；失败为已记录的 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal`（workflow.rs:2508）。独立仓库外打包通过；debug 和实际 release 的插件 PTY 主路径各 1 passed。详见 docs/任务/插件Demo交付与发布.md，不写全绿。

- 前轮插件实现：5191a78，demo 固定公开 SDK revision 与审查记录 13f2e47，合并 d05f7a4，空提交收尾 5977f49。分支 plugin-demo 与独立基线 worktree 均已清理，没有创建/关闭/发送任何 agent；主控遵照用户要求自己做。
- 已有功能：Settings → Plugins F5；登记本地目录（默认停用）、启停/重启/移除、新 tab 面板、结构化输入、内部通知；插件进程独立 session/无 PTY，SDK 处理 stdio；layout v2 保留旧备份，ctl inspect 支持 plugin。
- SDK 在 crates/plugin-protocol、crates/plugin-sdk；独立 demo 在 examples/counter-plugin，只依赖 Git revision 5191a782bb7671390c124485258d83e434104560 的公开 SDK，有自己的 Cargo.lock。复制到 /tmp/saddle-plugin-external.h9LoWT/counter 独立构建并用其产物验证通过；没有将该目录登记到真实 Saddle。
- 前轮实现验证：最终标准套件 305 passed / 1 failed / 6 ignored；失败为既有 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases，在未修改基线2bfe1c4同一步再次复现。Clippy通过。两项本轮需外部构建的 ignored 检查已显式运行通过（SDK stdio probe、实际 Saddle PTY demo）。另新增极小管理页窗口检查单独通过。详情见 docs/任务/插件首个Demo实现.md，不写全绿。
- 隔离 PTY 覆盖添加/启用/打开、Enter/鼠标、通知、Shift+Enter、关闭重开保留、重启归零、停用；假 Corral/Drover、临时 HOME/config/state/runtime。进程故障回收、宽字符和旧布局等检查通过。
- 日常入口 `~/.local/bin/saddle` 仍指向共享 target/release/saddle，现已构建为插件版，SHA-256 `99695dbeb72724160443418be64809e61b33995badfa7d7515d2a17ed29961da`。备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/plugin-20260930-170510`。**已更新磁盘二进制，没有强制退出/重启现有窗口，也未改日常配置/插件登记/队列/服务**；下一次正常启动生效。
- 设计记录：docs/DESIGN.md §62、docs/插件系统设计.md、docs/插件协议.md、docs/插件Demo设计.md。侧栏/状态栏等挂载位置以后按实际需求补充，当前支持右侧工作区面板；Drover 第二阶段尚未实施。

- 前轮联合发布 Saddle main：2a6862c；主控审查 c97270a；空提交收尾 c939fd1。本交接与发布记录随最终文档提交推送，确切 SHA 见 git log 和 Dispatch 05458465051541c4a9c2cc908dc9fe3e 收尾 note。无遗留本轮实现改动。
- Drover main 已发布 e0d8118（合并 b02f142，收尾16ef806），origin/main 已核对一致。Drover 无关未跟踪 T27 文档保留。
- 前次联合发布的 Saddle 二进制哈希 e63d8e5a 已被本轮插件版替换并备份；当时用户重启实例 5d8f7242dcc4636b、PID63344 是历史核验，不代表当前运行实例或本轮已重启。
- 服务 dev.drover.loop 已改为 `drover notifications watch`，launchctl running、PID83246；旧推进 PID46666 已退出。保留的 loop 名字不是自动推进功能。
- 发布时公开状态与备份一致；后续用户现场测试 T57，已进入 Done。T58 已提交并被接受，现为 Done（run_id cd6de94f20b244538dde947598122f88；t1=1790749359.1966999，t2=1790749376.7627158）。T48、T54、T55 已从 Pending 删除，具体状态以公开 list/show 为准。
- T58 实现 1863d22，审查 85baab4，空提交收尾 cc9e52a；交付仅新增指定文档与任务记录，不改代码或配置。文档 diff 检查通过，未测试、编译或重启。

## 验证与限制

- Drover 旧 gate=false 后 go 的兼容返工已复审；新 accepted 严格要求 Awaiting，不伪造旧接受。
- Saddle 主控标准检查292 passed / 1 failed / 3 ignored，Clippy通过。失败为既有 picker 用例 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases，独立 target 的 main基线亦在同一步失败；未修 T29，不能写全绿。
- 隔离联合主路径通过，主控复跑1 passed：A退回保留未合并分支，B提交后出现测试PTY内部提示/Attention，接受后Done，无自动派发。该证据为隔离测试；发布后用户另行现场确认 T57 已待验收并弹出通知，随后接受。
- 备份目录：/Users/firegnu/Library/Application Support/saddle-release-backups/20260930-135645。含旧数据/配置/服务、旧Saddle二进制、公开切换前后状态和核验。新事件不兼容旧二进制写操作，回退须协调版本与日志。

## 保留的工作

- 本轮 Saddle drover-schema2 和 Drover task-flow-simplification 分支/worktree 均已清理；自开 saddle/dev-drover-schema2-1 随工作目录删除已关闭。drover/main 保留，其他用户 agent 未停止。
- T55 的 t55-notification-flow（3cc417d）与 T38 的 t38-dispatch-study（c15bc4d）保留，不能为解除完成阻挡而合并/删除。
- T57 测试文档已交付并由用户接受。T58 worktree/分支已安全清理，自开 saddle/dev-t58-flow-doc-1 因工作目录已删而一并关闭。
- T51 当前pane、T52 Shift+Enter 已交付并由用户放行。其他 Pending 不自动启动。

## 下一步

T58 已 Done；最近核对时没有 Running 或 Awaiting 任务。本轮设计不操作队列，不自动派发。新版流程为 dispatch-pending → done提交验收 → 用户go接受，Running/Awaiting可退回；没有Loop自动提交/派发。

插件工作的下一步：用户重开日常 Saddle，按需从正式产物目录添加 demo。磁盘 release 已更新，当前窗口未强制重启，重启后的现场验收尚未发生。开发说明见 docs/插件开发入门.md；不要再要求用户执行那串临时测试环境命令。当前是已验证的开发协议，不是稳定1.0或多插件长稳性能承诺。Demo 通路已验证，Drover 为第二个插件；先讨论迁移清单和必要挂载位置，未授权前不迁移或重写 Drover。T29既有picker失败保留，不自动开启新任务。

本轮后续由主控亲自做，不再委派。已有讨论/审阅 agent `saddle/dev-plugin-design-1`（main 仓库 cwd，最近 idle、attached=1）保留，不再主动送新任务；用户未要求关闭。Corral/corral-dispatch 与 Drover 仓库、真实状态均未改。

## 先读与记录

AGENTS.md、dispatch-log/USAGE.md；docs/DESIGN.md §62 与两份插件设计文档；任务流转参考 §61；docs/任务/Saddle-Drover联合发布.md及对应主控审查/隔离联调记录；Drover主仓库docs/任务流转JSON接口.md。真实操作仅走公开CLI。

Dispatch：Drover 0ec134ce246c420aabb6654254ddcf97；Saddle 05458465051541c4a9c2cc908dc9fe3e。Drover第一阶段回调若晚到，先核对已处理，不重复安装或启动服务。

T58 Dispatch：a11dd4a152ae454c9eb1e6b69ddfda94；任务与审查记录 docs/任务/T58-任务流转文档测试.md。

插件讨论与设计记录：Dispatch 7c3ecbc86ee14ec8bf887e78a55ef9d1。两轮讨论与首次文档审阅已结束；审阅回复 at=1790753355.926681 已处理。旧回调不重复处理或自动委派，需求与决定以 §62 及最新用户指示为准。

插件实现记录：Dispatch d2cd3a7e030b4a619cc707241f33aedd，仅记录主控实施和审查，无代理派发、无队列变更。测试日志 /tmp/saddle-plugin-demo-final-tests.log，基线 /tmp/saddle-plugin-baseline-t20-repeat.log，审查说明 docs/任务/插件首个Demo实现.md。

插件交付/发布记录：Dispatch a54eebf9f78f4544a969aa1487288e9c，docs/任务/插件Demo交付与发布.md；只记录主控工作，不是代理派发。
