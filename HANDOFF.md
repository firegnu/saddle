# 会话交接

更新：2026-09-30。Drover + Saddle 简化流程已联合发布。用户已用真实 T57 验证待验收通知并接受，T57 为 Done。新文档测试 T58 已完成委派、审查、合并、推送和清理，done 返回 awaiting_release；随后公开状态出现接受记录，现为 Done。主控未执行 go，不自动派发。用户随后确认进程式插件长期架构及在 Saddle 内绘制界面；主控已亲自完成首个插件宿主、Rust SDK 与独立 demo；用户在临时开发版确认效果后授权日常发布。已更新日常 release，补齐一条命令打包与开发入门；用户随后明确“跑起来了”，确认日常版 Counter 可用。用户看过Counter后进一步确认：Saddle拥有外壳，插件不增加常驻UI；主控已亲自把侧栏逐插件按钮替换为统一插件命令面板。工作区/覆盖绘制与SDK保留，Attention 来源与 Drover 迁移仍未实施。

## 当前状态

- 本轮统一面板：实现63ae93b，合并baab2f2，空提交收尾0999dd5；plugin-palette分支/worktree已清理。主控亲自实现，原讨论agent及T38/T55工作树未操作。
- 固定Plugins入口在Settings附近；面板约72列18行，搜索/选择/状态/Open或Switch，Manage plugins为次要入口。所有注册插件可见；没有新增快捷键。旧Counter侧栏按钮与F6逻辑已移除，SDK/能力清单/Counter二进制保持不变。
- 最终完整标准检查（test-threads=4）：316 passed / 1 failed / 7 ignored；失败为既有T25布局保存用例收尾picker未找到目标，单独复跑通过，根因未确认。插件模块最终13 passed / 1 ignored，Clippy通过。实际日常release的全部8条插件PTY检查（含真实Counter）通过，日志/tmp/saddle-palette-release-pty.log；不宣称全套全绿。
- 日常入口~/.local/bin/saddle仍指向共享target/release/saddle；实际release检查完成后SHA-256为`631d2fed76b93b0d3f225b482d709daccaeb7c847b07d717ca61126eb8610324`，对应合并baab2f2源代码。旧程序备份`/Users/firegnu/Library/Application Support/saddle-release-backups/plugin-palette-20260930-184030`（旧哈希e9bd3444）。插件登记/配置/程序未改变，也没有强制重启用户窗口；本轮面板尚待用户重启后目视确认。

- 上轮入口实现（现已替换其入口）：9a8e656；独立 SDK 固定与审查 dea3710；合并 bf072b8 已推送；空提交收尾 e0696cc，plugin-entry 分支/worktree 已清理。Counter 清单默认 overlay，启用后左侧显示 Counter；鼠标直接打开，Agents 中 F6/Enter 也可打开。Esc 关闭恢复焦点，Ctrl-] 返回 Agents，关闭保留进程。已有旧 Counter tab 优先复用，关闭旧 tab 后再点入口才使用 overlay。
- 设计依据仍为89a4709及 docs/插件入口与界面接入设计.md，第一步已实施；第二步 Attention 来源、第三步 Drover 迁移尚未开始。没有真实队列/服务/上游修改，没有委派或操作其他 agent。
- 上轮入口标准检查312 passed / 1 failed / 7 ignored；失败是原有 t20_r1_pending_new_pane_keeps_known_source_cwd_for_shell 的关闭确认 unwrap，单独复跑通过，根因未确认。早先一次 picker/task_links 失败在最终检查通过。Clippy通过；插件模块12 passed，普通插件PTY3 passed，真实独立SDK/Counter的2条ignored检查显式通过，实际日常release的同2条检查也通过。详见 docs/任务/插件直接入口与覆盖界面实现.md，不宣称全绿。
- Counter 独立项目现固定公开 SDK revision `9a8e6568ee65dcc247aa6516294a451f3ecd502b`；仓库外打包与真实PTY验证通过。旧清单仍兼容，新清单要求 ui.entry.v1 / panel.overlay.v1；overlay 不写布局，不启动额外插件进程。

- 上轮实现交付：6d39a06（示例 package.sh、开发入门及 README），合并 27f3279，收尾 fc2c060；plugin-delivery 分支/worktree 已清理，main 已推送。主控亲自完成，没有委派、队列或上游变更。
- 用户一条打包命令：`./examples/counter-plugin/package.sh`；正式产物已生成在主仓库 `examples/counter-plugin/dist/counter-plugin`。在日常新版 Settings → Plugins 中添加该完整目录、Enable、Open panel；本轮没有替用户登记。
- 上轮标准检查：306 passed / 1 failed / 6 ignored，Clippy通过；失败为已记录的 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal`（workflow.rs:2508）。独立仓库外打包通过；debug 和实际 release 的插件 PTY 主路径各 1 passed。详见 docs/任务/插件Demo交付与发布.md，不写全绿。

- 前轮插件实现：5191a78，demo 固定公开 SDK revision 与审查记录 13f2e47，合并 d05f7a4，空提交收尾 5977f49。分支 plugin-demo 与独立基线 worktree 均已清理，没有创建/关闭/发送任何 agent；主控遵照用户要求自己做。
- 已有功能：Settings → Plugins F5；登记本地目录（默认停用）、启停/重启/移除、新 tab 面板、结构化输入、内部通知；插件进程独立 session/无 PTY，SDK 处理 stdio；layout v2 保留旧备份，ctl inspect 支持 plugin。
- SDK 在 crates/plugin-protocol、crates/plugin-sdk；独立 demo 在 examples/counter-plugin，当时依赖 Git revision 5191a782bb7671390c124485258d83e434104560 的公开 SDK（当前版本见本轮记录），有自己的 Cargo.lock。复制到 /tmp/saddle-plugin-external.h9LoWT/counter 独立构建并用其产物验证通过；没有将该目录登记到真实 Saddle。
- 前轮实现验证：最终标准套件 305 passed / 1 failed / 6 ignored；失败为既有 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases，在未修改基线2bfe1c4同一步再次复现。Clippy通过。两项本轮需外部构建的 ignored 检查已显式运行通过（SDK stdio probe、实际 Saddle PTY demo）。另新增极小管理页窗口检查单独通过。详情见 docs/任务/插件首个Demo实现.md，不写全绿。
- 隔离 PTY 覆盖添加/启用/打开、Enter/鼠标、通知、Shift+Enter、关闭重开保留、重启归零、停用；假 Corral/Drover、临时 HOME/config/state/runtime。进程故障回收、宽字符和旧布局等检查通过。
- 上轮日常入口 `~/.local/bin/saddle` 指向共享 target/release/saddle，当时构建为 bf072b8 对应入口/覆盖版，SHA-256 `e9bd34449a432265138334f8c43b2e41aab78c295ca05678bd6dbaa45b79f5d6`；同一正式 Counter 目录的程序与清单已更新，登记无需重做。旧二进制与旧插件包备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/plugin-entry-20260930-175548`（旧二进制99695dbe）。实际 release 两条插件PTY检查通过，日志 /tmp/saddle-entry-release-pty.log。仅更新磁盘，未强制重启用户窗口；该版用户已看到，随后要求改为统一命令面板；最新发布见本轮记录。

- 设计记录：docs/DESIGN.md §62、docs/插件系统设计.md、docs/插件协议.md、docs/插件Demo设计.md。本轮已增加侧栏入口与覆盖界面；其他挂载位置按实际需求补充，Drover 迁移尚未实施。

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

插件工作的下一步：用户自行重启日常Saddle，点击左侧固定Plugins，搜索Counter，Enter或行尾Open/Switch使用；无需重新添加插件。Counter不再有独立常驻按钮，旧F6逐插件入口已撤下，新全局快捷键尚未选定。面板显示本实例运行状态，停用/故障项保留但不可执行；启用仍立即启动，关闭视图保留进程。当前工作区/覆盖视图优先复用。SDK与Counter程序不需升级。

这轮只改统一入口，不继续迁移Drover或实现Attention来源。此前“Drover原位置常驻Tasks”已经被用户新的外壳原则替代：未来通过统一插件面板进入，真实内建Drover尚未迁移。待用户观察新面板后再讨论后续。现有Tasks/通知/任务队列未改；仍不自动推进。测试偶发失败如实保留，不顺带修T29。

本轮后续由主控亲自做，不再委派。已有讨论/审阅 agent `saddle/dev-plugin-design-1`（main 仓库 cwd，最近 idle、attached=1）保留，不再主动送新任务；用户未要求关闭。Corral/corral-dispatch 与 Drover 仓库、真实状态均未改。

## 先读与记录

AGENTS.md、dispatch-log/USAGE.md；docs/DESIGN.md §62 与两份插件设计文档；任务流转参考 §61；docs/任务/Saddle-Drover联合发布.md及对应主控审查/隔离联调记录；Drover主仓库docs/任务流转JSON接口.md。真实操作仅走公开CLI。

Dispatch：Drover 0ec134ce246c420aabb6654254ddcf97；Saddle 05458465051541c4a9c2cc908dc9fe3e。Drover第一阶段回调若晚到，先核对已处理，不重复安装或启动服务。

T58 Dispatch：a11dd4a152ae454c9eb1e6b69ddfda94；任务与审查记录 docs/任务/T58-任务流转文档测试.md。

插件讨论与设计记录：Dispatch 7c3ecbc86ee14ec8bf887e78a55ef9d1。两轮讨论与首次文档审阅已结束；审阅回复 at=1790753355.926681 已处理。旧回调不重复处理或自动委派，需求与决定以 §62 及最新用户指示为准。

插件实现记录：Dispatch d2cd3a7e030b4a619cc707241f33aedd，仅记录主控实施和审查，无代理派发、无队列变更。测试日志 /tmp/saddle-plugin-demo-final-tests.log，基线 /tmp/saddle-plugin-baseline-t20-repeat.log，审查说明 docs/任务/插件首个Demo实现.md。

插件交付/发布记录：Dispatch a54eebf9f78f4544a969aa1487288e9c，docs/任务/插件Demo交付与发布.md；只记录主控工作，不是代理派发。

插件日常入口设计记录：Dispatch 6f1022252d304e12899e8d72264bffc8，docs/任务/插件日常入口接入设计.md；主控文档工作，无代理派发。

插件直接入口实施记录：Dispatch a981890142784b43b5ece49b90f827d1，docs/任务/插件直接入口与覆盖界面实现.md；仅主控实施/审查，无代理派发。

统一插件命令面板记录：Dispatch 7a3a7633047440acb3ef27635a2eb18e；docs/任务/插件命令面板.md，主控亲自实施/审查，无代理派发。
