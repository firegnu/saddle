# 会话交接

更新：2026-09-30。**Drover 已完整替换为一个 Saddle 插件**，包括任务核心、数据读写、界面、状态流转、内部/系统通知。旧 CLI 和 launchd watch 已退役。主控亲自完成，没有委派、没有操作其他 agent。

## 当前状态

- 最新修复：Tasks 插件 `Failed · output queue full`。宿主每轮转入8条却只发送1条造成内部积压，已对齐有界发送预算并在内部队列满时暂停转入。实现 `62a8a92`，主控亲自完成并更新日常宿主；任务数据、插件包、队列规则均未改变。记录见 `docs/任务/插件输出队列溢出修复.md`；Dispatch `e37de990ef504bb89baa5e492b10d367`。
- 用户已重启过完整插件版，但当前窗口仍需**再次退出并启动 Saddle**加载此次宿主修复，再从 Plugins → Tasks（Drover）打开。没有强制退出当前窗口；只重启插件不能加载宿主修复。
- 实现 `a024ad2`，合并 `826a5e0`；安装验证发现的 macOS ctl 读取修复 `4187aa3`，合并 `e0caa64`；空提交收尾 `5bc3231`。均已落地 main；本轮 worktree/分支 `drover-native-core` 已清理。
- Dispatch `7785984ca0bc4ff186b50014fc89263b`；需求、审查、验证和发布证据见 `docs/任务/Drover插件完整替换.md`。
- 插件已登记启用，无需重新添加目录。完整插件切换后的用户启动已确认，随后报告上述通信错误。
- 真实任务未推进。三个已登记项目新旧投影一致：Drover 5待办/24历史，JB 0待办/1历史，Saddle 8待办/50历史；均无Running/Awaiting。安装后队列、事件、配置、暂停文件逐字节核对未变。

## 安装与退役

- `~/.local/bin/saddle` 仍链接共享 `.target/release/saddle`。队列修复版SHA-256：`45b2d857fbb17abeecd6f122407ad88312a5b4754388c01316cc75bd23009691`。
- 本次宿主备份：`~/Library/Application Support/saddle-release-backups/plugin-output-queue-20260930-220920/`，包含此前 `d722aff4…` 宿主与安装hash记录；未替换插件包或修改真实配置。
- 正式插件目录：`/Users/firegnu/Developer/personal_projs/saddle/plugins/drover/dist/drover-plugin`。插件二进制SHA：`72b47ccf071e9cd4f6f73b8ab31ddfaaaa3495d3a0f6d4671fe60f833b4ac19b`。
- 本机PATH无dlog，包清单args保留 `/Users/firegnu/Developer/personal_projs/dispatch-log/dlog`。重新打包会覆盖清单，应保留该参数。默认Corral来自PATH；旧 `--drover` 参数已移除。
- `dev.drover.loop` 已 `launchctl bootout`，LaunchAgents链接及原plist已撤下；`~/.local/bin/drover`、`drover-board` 两个已核实链接撤下。没有按名字杀进程，没有删除原Drover仓库或用户数据。
- 备份：`/Users/firegnu/Library/Application Support/saddle-release-backups/drover-native-20260930-213348`，包含原宿主/插件、命令链接、Saddle配置、Drover数据/服务配置、旧公开快照、文件hash和installed.json。回退先停用新插件，再恢复旧程序与服务；不要直接用备份数据覆盖后续新任务。
- 用**实际日常入口+正式插件包**另起隔离宿主、假Corral，只读三个真实项目，已通过并正常退出；没有打开/操作真实agent。日志 `/tmp/saddle-native-installed-final.log`。这不等于用户窗口已重启，也没有宣称肉眼看到了系统通知。

## 后续操作边界

- 一个Drover进程插件拥有全部业务。关闭面板继续后台，停用/退出Saddle停止观察和通知；用户已明确接受。每用户数据同时只允许一个插件所有者。
- 主控不再调用旧 `drover ...`。使用 `saddle ctl instances` 选实例，再用 `saddle ctl plugin --instance ID --plugin drover --method METHOD --params JSON`，随后 `ctl request` 取结果。方法、令牌、分页和例子见 `plugins/drover/README.md`。
- `submit` 提交验收，`accept` 接受，`dispatch` 明确派发，`return` 需要原因和工作已停止确认。没有自动下一项；Git检查只作参考。用户授权范围与Pending/Awaiting放行边界仍须遵守。
- 结果未知不能自动重发；同一请求ID只重读原回执。展示令牌过期必须重新读取并确认。通用ctl沿用每实例256条回执上限；参数/结果48 KiB，列表分页摘要、详情单读。不要通过读写真实内部文件绕开公开操作接口。
- 旧数据格式/路径沿用，只有插件核心负责业务读写；宿主保持通用。Corral/corral-dispatch和dispatch-log保持独立、没有改它们的代码。用户要求主控亲自做，后续不要自行委派。

## 验证

- 输出队列修复：自动化复现明确 RED `Failed · output queue full` → GREEN，完整有序接收160条消息；暂不可写/大消息/子进程回收通过。标准检查 **335 passed / 0 failed / 4 ignored**，Clippy、fmt、diff通过；release插件23 passed/1 ignored、真实插件输入流程1 passed。均为临时配置/合成数据，日志 `/tmp/saddle-output-queue-*.log`。用户窗口重启后的实际效果仍待用户观察。
- 最终 `cargo test --all-targets -- --test-threads=4`：**332 passed / 0 failed / 4 ignored**；Clippy、fmt和diff检查通过。日志 `/tmp/saddle-native-complete-{all,clippy}.log`。
- release主控入口1 passed、通知1 passed、插件进程9 passed；旧Counter/Attention二进制兼容3 passed。均合成任务、临时配置/数据和假Corral/osascript；三项旧demo可选测试已单独运行。
- 原CLI缺失、大正文列表超限均先复现失败再修复；原生状态/旧事件/过期及ABA令牌/并发锁/送达与记录/单所有者均覆盖。
- ctl缺陷已确认：macOS对端关闭后设置SO_RCVTIMEO会EINVAL，即使缓冲区已有完整响应。改为poll等待并保留两秒截止；新增完整/截断响应2个回归，RED→GREEN。原交接的同类偶发错误不再仅标作“重跑通过”。日志 `/tmp/saddle-control-closed-peer-{red,green}.log`。

## 保留事项

T38 `t38-dispatch-study` 和 T55 `t55-notification-flow` worktree/分支保持原样，不合并或清理。原Claude讨论agent已在此前按用户要求关闭，本轮没有新agent。原Drover仓库 `/Users/firegnu/Developer/personal_projs/drover` 保留，仅作历史来源，不再是运行依赖。

设计理由见 `docs/DESIGN.md` 最后的完整替换小节；命令协议见 `docs/插件协议.md` §12。先读本交接、AGENTS.md、dispatch-log/USAGE.md、插件README再继续。
