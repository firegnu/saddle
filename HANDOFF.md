# 会话交接

更新：2026-10-01。**Drover 已完整替换为一个 Saddle 插件**，包括任务核心、数据读写、界面、状态流转、内部/系统通知。旧 CLI 和 launchd watch 已退役。主控亲自完成，没有委派、没有操作其他 agent。

## 当前状态

- 最新交付：Settings 顶部改用紧凑单行标签，正常宽度五项同排，窄窗口紧凑换行。实现 `52d6a81`，收尾 `259b861`；已合并推送、更新日常宿主、清理 `settings-compact-tabs` 分支/worktree。主控亲自做，没有委派；用户重启 Saddle 后观察，本轮未重启其窗口。任务/审查 `docs/任务/Settings紧凑标签栏.md`，Dispatch `1d77432fe0ce40978e99c4c5fc5a20fa`。
- Settings 验证：标准 **354 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；独立release候选的设置保存/侧栏调整及标签布局2项通过。日志 `/tmp/saddle-settings-tabs-{all,clippy,release-check}.log`。插件包、配置、队列和服务未改动。
- 用户已查看Diff演示并反馈“cool”。三处未提交临时演示继续保留：`examples/counter-plugin/src/main.rs`、`plugins/diff/README.md`、`docs/diff-preview-demo.json`；本轮前后逐文件hash一致，不将其纳入正式提交，也不自动撤回。

- 此前交付：实时只读 **Diff 插件**，实现 `7eb8c7d`，收尾 `c7c0b74`。主控亲自实现；已合并推送并清理 `diff-plugin` 分支/worktree，没有创建或操作其他 agent。打开即连续展示来源 worktree 全部改动，自动刷新；文件列表只用于跳转，支持现有居中/tab/split。任务与审查 `docs/任务/实时Diff插件.md`，Dispatch `ec99fea90a2248bfa0ff7ac31bae5ad3`。
- Diff 包已放在 `/Users/firegnu/Developer/personal_projs/saddle/plugins/diff/dist/diff-plugin`，SHA-256 `21db815d69db57be88b12fe2c16187aeb69e6b307681586d88abbd1737326f9f`。此前交付时由用户自行添加启用，用户现已试用反馈。插件更新与宿主重启独立；本轮没有改插件登记、真实配置、队列或服务。
- Diff 验证：标准 **353 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；真实打包插件与隔离 Saddle 的连续多文件、自动刷新、点击定位、居中/split/tab 移动和模式切换额外1项通过。日志 `/tmp/saddle-diff-all-complete.log`、`/tmp/saddle-diff-clippy-complete.log`、`/tmp/saddle-diff-host-final.log`。用户已试用反馈“cool”。

- 此前交付：实现 `510d907`，`Split → 方向 → Plugin…` 可在分屏打开插件，标签条 `+ → Plugin…` 在新 tab 打开；已有视图显示 Move，移动原窗格且保持单进程和状态。已合并推送、更新日常宿主、清理 `plugin-split` 分支/worktree。用户重启 Saddle 后观察，不需要重新安装插件。记录见 `docs/任务/插件在分屏中打开.md`，Dispatch `ab988a2e99c24f2d8421cddb8bb5233e`。主控亲自做，未改变真实队列/配置/插件包，也未重启用户窗口。
- 此前入口整理：实现 `460935d`，Plugins移到Agents标题右侧，与下一行Settings右对齐；左侧只保留Agents/Attention。窄窗口放不下时右侧换行。记录见 `docs/任务/Plugins入口右对齐.md`；Dispatch `8b9c4cebd9da493eae61b3018c0509f1`。
- 插件居中：实现 `1b8af30`，插件弹层按全窗口宽高80%居中，保留调暗背景，用主题灰色细线外框；用户已反馈可以。记录见 `docs/任务/插件全窗口居中与灰色外框.md`，Dispatch `3781a843710a41c687a641acfde2dce7`。
- 此前单层外框：实现 `0618693`，Drover不再画整页框，临时运行提示移到底部；用户已确认更安静。此次全窗口居中替代了上一轮隐藏整个右侧工作区的做法。记录见 `docs/任务/插件单层外框整理.md`，Dispatch `ab034c84cd5740c7a364161fde44119b`。
- 最新修复：Tasks 插件 `Failed · output queue full`。宿主每轮转入8条却只发送1条造成内部积压，已对齐有界发送预算并在内部队列满时暂停转入。实现 `62a8a92`，主控亲自完成并更新日常宿主；任务数据、插件包、队列规则均未改变。记录见 `docs/任务/插件输出队列溢出修复.md`；Dispatch `e37de990ef504bb89baa5e492b10d367`。
- 用户已重启验证过输出队列修复，Tasks成功打开，随后报告重复边框。下一步**退出并启动 Saddle**加载最新界面整理，再从 Plugins → Tasks（Drover）打开；只重启插件不能加载宿主更改。
- 实现 `a024ad2`，合并 `826a5e0`；安装验证发现的 macOS ctl 读取修复 `4187aa3`，合并 `e0caa64`；空提交收尾 `5bc3231`。均已落地 main；本轮 worktree/分支 `drover-native-core` 已清理。
- Dispatch `7785984ca0bc4ff186b50014fc89263b`；需求、审查、验证和发布证据见 `docs/任务/Drover插件完整替换.md`。
- 插件已登记启用，无需重新添加目录。完整插件切换后的用户启动已确认，随后报告上述通信错误。
- 真实任务未推进。三个已登记项目新旧投影一致：Drover 5待办/24历史，JB 0待办/1历史，Saddle 8待办/50历史；均无Running/Awaiting。安装后队列、事件、配置、暂停文件逐字节核对未变。

## 安装与退役

- 最新宿主备份：`~/Library/Application Support/saddle-release-backups/settings-compact-tabs-20261001-002259/`，包含此前 `eebb3117…` 插件分屏版及安装hash记录。日常程序已原子替换并核对SHA；旧窗口继续运行，等待用户重启。

- `~/.local/bin/saddle` 仍链接共享 `.target/release/saddle`。Settings紧凑标签版SHA-256：`1459a6b6d3631305056ae4ee8f3723d7effa1ed9fa9e2ab3b71a49e2e9996c16`。
- 最新宿主备份：`~/Library/Application Support/saddle-release-backups/plugin-split-20260930-231227/`，保存入口右对齐版 `5080dbb2…` 和安装hash记录。插件包、配置和真实任务数据均未改动。
- 此前宿主备份：`~/Library/Application Support/saddle-release-backups/plugins-header-right-20260930-224044/`，包含此前 `a259aba2…` 程序和hash记录。
- 当前宿主备份：`~/Library/Application Support/saddle-release-backups/plugin-centered-overlay-20260930-223126/`，包含此前 `3db84341…` 宿主和hash记录。插件包与真实配置/任务数据不变。
- 最新备份：`~/Library/Application Support/saddle-release-backups/plugin-single-frame-20260930-221953/`，包括更新前宿主和Drover二进制。插件清单、Counter包、真实配置/任务数据未改动。
- 本次宿主备份：`~/Library/Application Support/saddle-release-backups/plugin-output-queue-20260930-220920/`，包含此前 `d722aff4…` 宿主与安装hash记录；未替换插件包或修改真实配置。
- 正式插件目录：`/Users/firegnu/Developer/personal_projs/saddle/plugins/drover/dist/drover-plugin`。插件二进制SHA：`85476253b67f6888c3e43a0943e3ac81b24716171485891f5d581791a223e8b2`。
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

- 插件分屏：有效 RED 为 Right 菜单缺少 Plugin…；标准 **340 passed / 0 failed / 4 ignored**，Clippy/fmt/diff通过。四方向、布局恢复、移动原 Pane、取消、禁用、单进程与状态保留均覆盖；发布版插件选择2项、真实Drover分屏1项、已打包Counter居中回归1项通过，全部隔离数据/假Corral。日志 `/tmp/saddle-plugin-split-{red,flow,layout,all,clippy,release,counter}.log`。用户窗口实际效果待重启观察。
- Plugins入口右对齐：标准335 passed / 0 failed / 4 ignored，Clippy/fmt/diff通过；发布版首行Plugins位置及Settings保存/侧栏缩放检查1项通过。旧Settings测试从第3行改为新布局第2行。日志 `/tmp/saddle-plugins-right-{all-final,clippy-final,release-app}.log`。
- 全窗口居中：最终标准335 passed / 0 failed / 4 ignored，Clippy/fmt/diff通过；实际Drover和Counter流程各1项，release定位/灰色外框/关闭恢复1项通过。旧滚轮测试硬编码右侧坐标已改为从画面定位列表，首尾滚动通过后重跑全套成功。日志 `/tmp/saddle-centered-{all-final,clippy-final,scroll,release-layout}.log`，隔离PTY画面 `/tmp/saddle-centered-{drover,counter}.txt`。
- 单层外框：标准335 passed / 0 failed / 4 ignored，Clippy/fmt/diff通过；实际Counter与Drover流程debug/release各1项通过。已检查隔离PTY渲染文本，去除临时捕获代码，日志 `/tmp/saddle-single-frame-*.log`；用户实际窗口的最新效果待其重启观察。
- 输出队列修复：自动化复现明确 RED `Failed · output queue full` → GREEN，完整有序接收160条消息；暂不可写/大消息/子进程回收通过。标准检查 **335 passed / 0 failed / 4 ignored**，Clippy、fmt、diff通过；release插件23 passed/1 ignored、真实插件输入流程1 passed。均为临时配置/合成数据，日志 `/tmp/saddle-output-queue-*.log`。用户窗口重启后的实际效果仍待用户观察。
- 最终 `cargo test --all-targets -- --test-threads=4`：**332 passed / 0 failed / 4 ignored**；Clippy、fmt和diff检查通过。日志 `/tmp/saddle-native-complete-{all,clippy}.log`。
- release主控入口1 passed、通知1 passed、插件进程9 passed；旧Counter/Attention二进制兼容3 passed。均合成任务、临时配置/数据和假Corral/osascript；三项旧demo可选测试已单独运行。
- 原CLI缺失、大正文列表超限均先复现失败再修复；原生状态/旧事件/过期及ABA令牌/并发锁/送达与记录/单所有者均覆盖。
- ctl缺陷已确认：macOS对端关闭后设置SO_RCVTIMEO会EINVAL，即使缓冲区已有完整响应。改为poll等待并保留两秒截止；新增完整/截断响应2个回归，RED→GREEN。原交接的同类偶发错误不再仅标作“重跑通过”。日志 `/tmp/saddle-control-closed-peer-{red,green}.log`。

## 保留事项

T38 `t38-dispatch-study` 和 T55 `t55-notification-flow` worktree/分支保持原样，不合并或清理。原Claude讨论agent已在此前按用户要求关闭，本轮没有新agent。原Drover仓库 `/Users/firegnu/Developer/personal_projs/drover` 保留，仅作历史来源，不再是运行依赖。

设计理由见 `docs/DESIGN.md` 最后的完整替换小节；命令协议见 `docs/插件协议.md` §12。先读本交接、AGENTS.md、dispatch-log/USAGE.md、插件README再继续。
