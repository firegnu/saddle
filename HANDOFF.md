# 会话交接

更新：2026-10-01。**Drover 已完整替换为一个 Saddle 插件**，包括任务核心、数据读写、界面、状态流转、内部/系统通知。旧 CLI 和 launchd watch 已退役。主控亲自完成，没有委派、没有操作其他 agent。

## 当前状态

- 统一产品调研：用户明确最终只有一个 Saddle 产品，不能由多个独立包/语言拼装。顺序已改为先将 corral-dispatch 与 dispatch-log 迁入 Rust core plugins，清理外围消费者和安装依赖，最后迁移 infra Corral；替代此前 Corral 优先、dlog 外部可选的建议。报告 `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md` 已更新。今天仅调研/文档，明天再开始；没有代码、合仓、发布、agent/队列/服务操作或委派。下一轮设计内置插件入口与记录兼容，不重问已确定的产品边界。
- 最新交付：按用户反馈Working增加半秒节拍原地内脚小步，外侧脚稳住，原小手动作保持；Waiting每4秒一次短眨眼、两次轻示意，多数时间静候。四细腿始终保留，外形/颜色/尺寸/位置、Idle节奏、配置开关和业务不变。实现 `cbc4398`，收尾 `79ee94f`，已合并推送、备份更新日常宿主并清理 `clawd-busy` 分支/worktree；用户重启后观察。主控亲自完成，没有委派或操作真实agent。
- 本轮标准检查 **374 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；隔离release开关/输入/布局1项通过。有效RED→GREEN确认Working脚会动，跨96帧检查身体稳定、四腿细且分离、眼睛和字符颜色；Working上身与Waiting仍至少四分之三时间静止。任务/审查 `docs/任务/Clawd忙碌与等候微动作.md`，Dispatch `04f2f51e9de448619b8ca7d3b991719e`；日志 `/tmp/saddle-clawd-busy-{all,clippy,red,green,release}.log`。三处Diff演示hash不变；真实配置/插件/agent/队列/服务未操作，未重启用户窗口。

- 此前交付：顶层 `mascot_enabled = true` 默认启用，缺省仍开启；Settings → General → Clawd mascot支持点击、空格/Enter/左右切换，Ctrl-S保存布尔值并立即生效，Esc取消、Ctrl-D恢复默认。关闭只隐藏绘制并暂停计时，未改外形/动作/布局/输入或agent状态。实现 `63a6042`，收尾 `1c74266`，已合并推送、备份更新日常宿主并清理 `mascot-setting` 分支/worktree；用户重启一次加载新版，以后Settings切换无需重启。用户真实配置没有修改。
- 本轮标准检查 **373 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；隔离release开关主路径1项通过。新增配置/Settings/真实CLI检查覆盖默认、启动false、布尔值保存、取消、恢复默认、即时显示/隐藏、无输入光标、无agent按键和布局不变，有效RED→GREEN。任务/审查 `docs/任务/吉祥物配置开关.md`，Dispatch `cb6c5068d16c4c0bb2b3d82becd6dc26`；日志 `/tmp/saddle-mascot-setting-{all,clippy,red,green,flow,release}.log`。三处Diff演示hash不变；真实配置/插件/agent/队列/服务未操作，未重启用户窗口。主控亲自完成，没有委派。

- 此前交付：用户已认可参考轮廓，本轮仅Clawd动作节奏收尾。外形/尺寸/颜色/位置固定；Idle约走3秒停4秒，起止缓动、边缘先停再折返，停顿偶尔眨眼；Working约2秒轻动手，Waiting约8秒短暂示意，其余静候。没有改业务或状态来源。实现 `61dea22`，收尾 `ba62f9c`，已合并推送、备份更新日常宿主并清理 `clawd-rhythm` 分支/worktree；用户重启观察。主控亲自完成，没有委派或操作真实agent；不再自行扩展吉祥物调整。
- 本轮标准检查 **370 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；隔离release动画/点击不输入1项通过。新增停顿恢复与动作静候比例检查有效RED→GREEN，轮廓常量及draw函数与认可版逐字一致。任务/审查 `docs/任务/Clawd动画节奏收尾.md`，Dispatch `93771e3c372f464aa4a9c1dbe6d953c7`；日志 `/tmp/saddle-clawd-rhythm-{all,clippy,red,green,release}.log`。三处Diff演示hash不变；真实配置/插件/agent/队列/服务未操作，未重启用户窗口。

- 此前交付：用户提供Claude Code启动截图指出轮廓失真。Clawd改为四分字符块，8列×3行（顶部半行留白），恢复细竖黑眼、矩形身体、低位短臂及四条细腿；走路四腿始终可见，只轻微移动内脚。仍复用tab空白，不改变布局/状态/输入业务。实现 `e56dedb`，收尾 `6b5912e`，已合并推送、备份更新日常宿主并清理 `clawd-reference` 分支/worktree；用户重启后观察。主控亲自完成，没有委派或操作真实agent。
- 本轮标准检查 **368 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；隔离release动画与点击不输入1项通过。任务/审查 `docs/任务/Clawd参考截图轮廓校正.md`，Dispatch `09d4a2217bfe4098bcac316c4868256d`；日志 `/tmp/saddle-clawd-reference-{all,clippy,target,release}.log`。实际渲染导出预览 `/tmp/saddle-clawd-reference-sprite.png` 已对照参考图（非用户窗口截图）。三处Diff演示hash不变；真实配置/插件/agent/队列/服务未操作，未重启用户窗口。

- 此前交付：按用户进一步反馈，Clawd缩为7列×2行，沿tab空白区底部对齐。走路上身不变，四短腿只在原位置交替抬起，停顿四脚落地，不向外甩腿。保留橘色/黑眼和既有状态、速度、布局与输入。实现 `71c1a71`，收尾 `da5c902`，已合并推送、备份更新日常宿主并清理 `clawd-tiny` 分支/worktree；用户重启后观察。主控亲自完成，没有委派或操作真实agent。
- 本轮标准检查 **368 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；隔离release动画与点击不输入1项通过。任务/审查 `docs/任务/Clawd再缩小与短腿步态.md`，Dispatch `c2775ff6e5624f068ec6dfc106a37d65`；日志 `/tmp/saddle-clawd-tiny-{all,clippy,target,release}.log`。三处Diff演示hash不变；真实配置/插件/agent/队列/服务未操作，未重启用户窗口。

- 此前交付：Clawd缩小至9列×3行，复用tab栏右侧实际空白；不再新增4行，不挤tab，终端pane恢复原高度。tab拥挤时隐藏。保留陶土橘/黑眼/四腿及原状态绑定，动作收敛至6帧/秒。实现 `c7ab8cd`，收尾 `5275420`，已合并推送、备份后原子更新日常宿主并清理 `clawd-compact` 分支/worktree。主控亲自完成，无委派；用户重启后观察，未重启其窗口。
- 本轮验证：标准 **368 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；隔离release动画/状态/鼠标不输入1项通过。有效RED证明旧版多占4行。首轮移动workflow在回分屏阶段过早读取部分重画，补齐输入目标与原输出等待后目标及全套通过，未改移动实现。任务/审查 `docs/任务/Clawd小号与无占位布局.md`，Dispatch `502c463cff7340afb34d55c5bc322bd8`；日志 `/tmp/saddle-clawd-compact-{all-final,clippy-final,release}.log`。渲染预览 `/tmp/saddle-clawd-compact-layout.png`，不是用户窗口截图。三处Diff演示hash不变，配置/插件包/agent/队列/服务未操作。

- 此前交付：Clawd状态吉祥物，按参考图11×8轮廓/陶土橘 `#D97757`/黑眼/短臂/四腿绘制；右侧tab下方4行动画带，只跟随当前焦点pane的agent。Idle慢走转向停顿，Working原地动手，Waiting挥手问号，错误/疑似停滞静止叹号。小窗口/普通终端/插件隐藏，原状态判据及输入/会话/任务/插件业务不变。实现 `5e6acb2`，收尾 `cdeb2d4`，已合并推送、备份后原子更新日常宿主并清理 `clawd-mascot` 分支/worktree。主控亲自完成，无委派或真实agent操作；用户重启后观察。
- 本轮验证：标准 **367 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；发布版动画/不传鼠标输入、modified Enter/粘贴2项通过。首轮两项workflow失败查明为旧坐标/尺寸与部分重画过早断言，改测试定位/等待后完整复跑通过，未改输入/会话移动代码。任务/审查 `docs/任务/Clawd状态吉祥物.md`，Dispatch `de272393c79b4e83abae5f4d46276225`；日志 `/tmp/saddle-clawd-{all-final,clippy-final,release-check}.log`。实际渲染导出预览 `/tmp/saddle-clawd-preview.png`，不是用户窗口截图。三处Diff演示hash不变，配置/插件包/agent/队列/服务未操作，未重启用户窗口。

- 此前交付：仅UI/UX整理。插件启动面板使用箭头/加粗选中、按条目数收缩、显示可见范围/总数，窄窗口名称与状态分隔且帮助自适应；失败保留原因并补管理入口指引。General收紧为12行/8列数字框；Agents实例/ATT 0/VIA元信息提亮。实现 `3210e1c`，收尾 `80f47ee`，已合并推送、备份后原子更新日常宿主并清理 `interface-polish` 分支/worktree。主控亲自做，无委派；用户重启后观察，未重启其窗口。
- 本轮边界：默认选择、状态/动作门控、键鼠动作、设置编辑/校验/保存逐段核对未改；未改业务、插件生命周期、任务流转、命令、协议或数据。标准 **362 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；release隔离Settings流程1项通过。记录 `docs/任务/界面可读性与紧凑布局.md`，Dispatch `cf97a33503754763883ef63ed4712eac`；日志 `/tmp/saddle-interface-polish-{red,green,all,clippy,release-check}.log`。三处Diff演示hash不变，真实配置、插件包、agent、队列与服务未操作。

- 此前交付：Colors预览改为Status/Text两行，与真实操作隔开；Plugins管理页复用Settings配置路径与五个页签，统一76列宽度/内边距，点击或F1–F4直接切页，F5留在当前选择，设置草稿保留。插件操作仍即时生效。实现 `c76e761`，收尾 `37b8a6c`，已合并推送、原子更新日常宿主并清理 `settings-pages` 分支/worktree。主控亲自做；用户重启后观察，本轮未重启其窗口。
- 验证：标准 **360 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；最终release切页/草稿保存及预览2项通过。日志 `/tmp/saddle-settings-pages-{red,green-final,all,clippy,release-check}.log`。任务/审查 `docs/任务/Settings预览与插件页签统一.md`，Dispatch `9125dd85794742b383fbb2978a33658d`。三处Diff演示hash不变，真实配置、插件包、agent、队列和服务未操作。

- 此前交付：插件管理页按实际插件数量收缩高度，列表与详情间一行留白；保留Settings草稿但暂停其背后绘制，消除透出的输入光标。管理页仅Add local目录输入显示光标，返回Settings恢复原输入；状态栏标明当前输入目标。实现 `e956557`，收尾 `c601b38`，已合并推送、原子更新日常宿主并清理 `plugin-manager-layout` 分支/worktree。主控亲自完成；用户重启后观察，本轮没有重启用户窗口。
- 管理页验证：最终标准 **358 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；最终release隔离PTY复现回归2项通过。首轮未改动的Viewer切换测试在3秒截止处超时，单项复查及完整复跑通过；保留失败记录，不声称修复此偶发问题。任务/审查 `docs/任务/插件管理页空白与光标.md`，Dispatch `660365b63e6949c58239fb3725e03b56`；日志 `/tmp/saddle-plugin-manager-{red,green,all,all-final,viewer-recheck,clippy,release-check}.log`。三处Diff演示hash不变，真实配置、agent、队列、服务和插件包未操作。

- 此前交付：Agents标题右侧将Plugins/Settings合并为同排操作，统一清晰正文色并悬停提亮；Attention独占第二行，0时弱化，有条目时文字与计数用状态色。窄窗口入口自动换到状态下方。实现 `42da29d`，收尾 `e693894`；已合并推送、更新日常宿主并清理 `agents-header-actions` 分支/worktree。任务/审查 `docs/任务/Agents顶部入口与状态分层.md`，Dispatch `3ffa43dbf0a6486c9203bfa3ebc0b72c`。主控亲自做，未重启用户窗口；下一步由用户重启观察。
- 顶部整理验证：标准 **356 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；独立release候选2项布局/配色和1项PTY设置保存/侧栏调整通过。日志 `/tmp/saddle-header-actions-{all,clippy,release-check}.log`。下述三处Diff演示文件hash前后一致，插件包、配置、队列、服务未改动。

- 此前交付：Settings 顶部改用紧凑单行标签，正常宽度五项同排，窄窗口紧凑换行。实现 `52d6a81`，收尾 `259b861`；已合并推送、更新日常宿主、清理 `settings-compact-tabs` 分支/worktree。主控亲自做，没有委派；用户重启 Saddle 后观察，本轮未重启其窗口。任务/审查 `docs/任务/Settings紧凑标签栏.md`，Dispatch `1d77432fe0ce40978e99c4c5fc5a20fa`。
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

- 当前Clawd忙碌/等候微动作版宿主SHA-256 `71791e53b4fb94e39c067c2a59b756475f7afdf323a25b7b9032cfc493c9d2f9`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-busy-20261001-024526/` 含旧版及安装记录。日常入口symlink不变；用户重启加载，无需重装插件。
- 此前吉祥物开关版宿主SHA-256 `edc860153666829bf2f1446cce2a2e77a3feea0eff3912b5f2ce03cc7a96a768`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/mascot-setting-20261001-023823/` 含旧版及安装记录。日常入口symlink不变；用户重启加载新版，无需重装插件。
- 此前Clawd节奏定稿宿主SHA-256 `2e399432f3388f105c8d955eb690ea31dd70655b07ce537faac60a216665c7bc`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-rhythm-20261001-022715/` 含旧版及安装记录。日常入口symlink不变；用户重启后加载，无需重装插件。
- 此前参考轮廓校正版宿主SHA-256 `e4ef277dd085c8d89953e0dadc116b650db72e7cb5f2de6dd91232fd3ac34494`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-reference-20261001-021626/` 含旧版及安装记录。日常入口symlink不变；用户重启后加载，无需重装插件。
- 此前7列×2行Clawd宿主SHA-256 `7bdb229ad9799814c6ec37a6c4d95173e0b7d176051aa9cbad4ff1aa68896642`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-tiny-20261001-020657/` 含旧版及安装记录。日常入口symlink不变，用户重启后加载，不需要重装插件。
- 此前小号Clawd宿主SHA-256 `8ad60e7a9eb6b27a0bd75a35712ccebacef6a882f10ec5c723686bb76edb7b14`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-compact-20261001-015841/` 含更新前程序与安装记录，入口symlink不变。只更新宿主，不需要重新安装插件；等待用户重启观察。
- 此前Clawd宿主SHA-256 `7983c23351ed092f64f3bd60af2b9b119167ba75f5f1b789d489df476ab8fd64`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-mascot-20261001-014512/` 含更新前程序与安装记录，入口symlink不变。只更新宿主，不需要重新安装插件。
- 本轮日常宿主SHA-256 `296b5028b1bb3798e220a011f69139d1abf66b83d2b8d8aaa64f5643d850c42a`；备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/interface-polish-20261001-012020/` 含更新前程序及安装hash记录。仅更新宿主二进制，入口symlink不变，未重启用户窗口。
- 最新宿主备份：`~/Library/Application Support/saddle-release-backups/settings-pages-20261001-010154/`，保存此前插件管理页修复版和安装hash记录。日常入口已更新并核对SHA；等待用户重启。

- 此前宿主备份：`~/Library/Application Support/saddle-release-backups/plugin-manager-layout-20261001-004752/`，保存此前Agents顶部版和安装hash记录。已核对日常入口SHA，等待用户重启加载。

- 此前宿主备份：`~/Library/Application Support/saddle-release-backups/agents-header-actions-20261001-003543/`，包含此前Settings紧凑标签版及安装hash记录。日常入口已原子替换并核对SHA，当前窗口未重启。

- 此前宿主备份：`~/Library/Application Support/saddle-release-backups/settings-compact-tabs-20261001-002259/`，包含此前 `eebb3117…` 插件分屏版及安装hash记录。日常程序已原子替换并核对SHA；旧窗口继续运行，等待用户重启。

- `~/.local/bin/saddle` 仍链接共享 `.target/release/saddle`。Settings页签统一版SHA-256：`ed44209ed84a2fa5b81d06e7418944fa230659ec85e3edcaa59ed66d6cca10a5`。
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
