# 会话交接

更新：2026-10-01。当前分支 `main`。本轮补齐 Clawd 十一段紧凑动作至31段并安装；统一 Saddle 产品迁移仍未开始，等用户触发。主控亲自做，不自行委派。

## 1. 会话摘要

用户认可静止效果，但反馈动作粗糙，要求精选十来个容易表达的动作精修，尽量不越过agent上方边框。用户确认12种动作；已改成固定静止轮廓的小幅关键帧，全部放在3行内，合并推送并安装。用户之后确认其他动作都没问题，仅反馈墨镜像额头缺口；已修正镜片，其他11段逐字节不变。随后用户要求新增兴奋和经典眼罩海盗，动作自行设计；已加入自主巡游，之后又接受推荐的六段紧凑动作，本轮再补齐9段简化动作和2段跳舞，现总31段，保留此前20段帧数据。没有重启用户窗口，等待其实际反馈。

## 2. 完成的工作

- 最新十一段实现 `f54e357`，安装记录 `9e31182`，收尾 `ee35e03`。新增desktop、arcade、bubble、spark、lightbulb、watering、berrypicking、blooming、fishing、dancing、dancinghappy，总31段1320帧，保持16列×3行与原20段素材/调色板逐字节不变。用户强调一次交付前完成打磨。
- 观察三张全关键姿态图；公开Mascot.draw模拟20分钟14400帧，匹配确认新11段共38种非静止独特姿态全部参与巡游。生成器重复生成一致；临时导出example已移除。调度仍为walking/边缘转身与随机动作，最近5段排除，不关联agent状态。
- 标准全量最终 **377 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过，发布版Mascot **9 passed**。首次全量在既有插件面板流程等待Clicks界面处失败，单独复核及第二次全量通过，未改插件代码/测试。日志 `/tmp/saddle-clawd-eleven-{all-final,clippy,workflow-retry,release-build,release-test,coverage}.log`。
- 最新预览：`saddle-eleven-actions.mp4`/`.gif`、`saddle-eleven-keyframes-{1,2,3}.png`；全31段总览 `saddle-curated-31.mp4`/`-poses.png`；实际巡游节选 `saddle-eleven-patrol-excerpts.mp4` 附原模拟秒数，有跳切。均在原Downloads目录，已打开；旧版预览保留，注意查看最新文件。
- 记录 `docs/任务/Clawd剩余十一个紧凑动作.md`，Dispatch `8453af9bbe4c46a99cd7396f8472b71d`。worktree/分支 `clawd-eleven-actions` 已清理，没有创建agent。8段未加入：jumping、jumpinghappy、juggling、confetti、confettihappy、rainbow、kite、breakdancing。31段包括30段网页动作和1段原创海盗。

- 此前六动作实现 `f825ad9`，安装记录 `3e7e07a`，收尾 `87f9c46`。新增phone、meditating、laptop、rose、heart、hulahoop，总20段813帧，仍为16列×3行；此前14段帧数据/调色板逐字节不变。仅增加离线关键帧、调色板及普通字符映射，不增加运行时机制。
- 全关键姿态逐张观察；实际公开Mascot.draw五分钟3600帧导出，匹配确认六段全部20种非静止独特姿态参与巡游。临时example已移除。生成器重复生成一致。
- 标准全量最终 **377 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过，发布版Mascot **9 passed**。第一次全量在既有插件背压测试的3秒启动等待处超时，单独重跑及第二次全量通过，未改插件代码/测试。日志 `/tmp/saddle-clawd-six-{all-final,clippy,plugin-retry,release-build,release-test}.log`。
- 此前六动作预览在原Downloads目录：`saddle-six-actions.mp4`/`.gif`、`saddle-six-actions-keyframes.png`；完整总览 `saddle-curated-20.mp4`/`.gif`/`-poses.png`；实际公开绘制五分钟巡游 `saddle-twenty-patrol.mp4`。旧14段及旧巡游预览保留，注意查看新文件。目录已用open打开。
- 记录 `docs/任务/Clawd六个紧凑动作.md`，Dispatch `690266cb13e64e2ab5e772b8f44501fb`。worktree/分支 `clawd-six-actions` 已清理，没有创建agent。用户强调一次成型；已完成交付前观察和验证，尚无用户对新增六段的实际窗口反馈。

- 此前兴奋/海盗实现 `e716c13`，安装记录 `d309c06`，收尾 `2ed353b`。新增兴奋39帧（双手欢呼、笑眼、脚步节拍、闪光）和海盗56帧（红头巾、单眼眼罩、相连横向细带、扶眼罩、抬手致意、眨眼），总14段517帧，全部仍在16列×3行内。原12段帧数据和原调色板逐字节不变。
- 两个新动作检查先因缺少动作而RED，后GREEN；标准全量 **377 passed / 0 failed / 5 ignored**，Clippy/fmt/diff通过；发布版Mascot **9 passed**。日志 `/tmp/saddle-clawd-excited-pirate-{all,clippy,green,release-build,release-test}.log`。
- 新预览 `saddle-excited-pirate.mp4`、`saddle-excited-keyframes.png`、`saddle-pirate-keyframes.png`；完整总览 `saddle-curated-14.mp4`/`.gif`/`-poses.png`；实际公开绘制巡游 `saddle-curated-patrol.mp4` 已更新为120秒，覆盖两种新动作。均在原Downloads预览目录，已用open打开。旧12动作预览保留。已观察全部关键姿态；海盗初版斜线像划痕，最终改为与眼罩相连的横向细带。
- 记录 `docs/任务/Clawd兴奋与海盗.md`；Dispatch `37d50d80c3ec43df8229d86b252b66b5`。worktree/分支 `clawd-excited-pirate` 已清理，没有创建agent。

- 最新墨镜修正`f82361f`，安装记录`f8f6aaa`，收尾`9618e2d`。镜片改为肤色背景上的`■`，保留完整额头和细鼻梁，尺寸/时间不变。公开绘制检查先RED后GREEN；标准全量**375 passed / 0 failed / 5 ignored**、Clippy/fmt/diff通过，发布版7项Mascot检查通过。日志`/tmp/saddle-clawd-sunglasses-{red,green,all,clippy,release-test}.log`。
- 墨镜对照`saddle-sunglasses-fix.png`/`.mp4`在原预览目录；12动作总览与90秒巡游均已同步更新。记录`docs/任务/Clawd墨镜额头修正.md`；Dispatch `cdebdd65b29646f38d2a0703aecb9003`；worktree/分支`clawd-sunglasses`已清理。

- 实现`e3172f4`、安装记录`3c10099`、收尾`297fd89`。仅播放walking、转身、张望、招手、思考、咖啡、笔记、耳机、看表、打盹、墨镜、轻轻摇摆；用户已确认这个名单，替代此前全部38段的要求。
- 画布16列×3行，全部422帧在agent上边框上方。静止帧去掉两行空白后与此前已认可版本逐字节一致；保持tab/pane/PTY布局。动作改为离线网格关键帧和停顿，固定地面，缩小道具；不跟随agent状态、不绑定终端、不调查或安装字体。
- 此前精选动作版本的标准`cargo test --all-targets`为**374 passed / 0 failed / 5 ignored**；Clippy/fmt/diff通过。新增公开绘制检查先RED（旧版tick107跨边框）、后GREEN。日志`/tmp/saddle-clawd-curated-{red,green,all-final,clippy}.log`。
- 发布版巡游/鼠标避让与设置/布局两项隔离流程通过，日志`/tmp/saddle-clawd-curated-release-{patrol,settings}.log`。使用假corral/临时HOME，没有操作用户agent或真实队列。
- 预览均在`~/Downloads/clawd-reference-20261001-4pdhpd0_/`：`saddle-curated-12.mp4`/`.gif`为全部12动作总览，`saddle-curated-12-poses.png`为代表姿态，`saddle-curated-patrol.mp4`现为实际Mascot.draw导出的120秒巡游。指定字号栅格化预览，不是用户窗口截图。桌面控制连接失败；已按既有授权用系统open打开目录。
- 记录见`docs/任务/Clawd精选动作.md`；Dispatch `36a692086a834fadbc3bf8dd26ce6d1f`。本轮worktree/分支`clawd-curated-motion`已清理，没有创建agent。
- 此前统一产品调查报告已提交 `aeff1d6`：`docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`。用户确定外围优先；本轮未开展迁移。之前临时 Diff 演示已清理，不恢复。

## 3. 待完成与未提交状态

本轮实现与安装已完成。等待用户观察新启动窗口里的动作效果；预览不能代替实际窗口反馈。眼睛沿用此前接受的字符近似，仍不声称像素级复刻。交接提交后main应干净并与origin/main一致，不自行扩展动作或字体方案。

保留既有 worktree/分支 `../saddle-worktrees/t38-dispatch-study`、`../saddle-worktrees/t55-notification-flow`，不合并、不清理。统一产品迁移待用户触发；需确定内置插件调用方式、CLI、历史记录兼容、模块接口及 skill 安装更新，不再重开产品边界和外围优先顺序。

## 4. 操作约束

- 用户要求主控亲自做，覆盖默认委派流程。现有用户 agent 不 stop/send/keys，不批量杀进程；本轮没有需要关闭的自建 agent。
- 统一产品既定顺序：dispatch-log → corral-dispatch → 清理 Saddle/Drover/skill 外围接入 → 最后 Corral。自有可执行逻辑迁 Rust；Skill Markdown 作为随产品资源保留，不机械化主控判断。不顺带建中央 daemon、自动任务引擎或重写终端渲染。
- 外围阶段仅通过 Corral 公开 CLI；不读内部运行文件。当前 Drover 是完整进程插件，拥有数据、状态和通知；不恢复旧 CLI/watch。插件关闭面板继续后台，停用/退出 Saddle 停止观察；Corral agent 继续存活。
- 任务操作只走 `saddle ctl instances` → `saddle ctl plugin --instance ID --plugin drover --method METHOD --params JSON` → `ctl request`。submit/accept/dispatch 分开，不自动下一项；结果未知不重发，令牌过期重新读取，不操作真实任务文件。
- 继续用共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。因日常链接直接指向该 release 二进制，构建 release 前先备份。

## 5. 当前安装与重要文件

`~/.local/bin/saddle` → `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`。当前SHA256 `7ae6d76083bd2aa41775d2f61162113d8a91a8dc0ae9e63291566d6c323d3e9e`，已验证内含31段完整素材。构建前旧版备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-eleven-actions-20261001-140905`，旧hash `cd17f0e33449e7963270dfa33cac46ba5ca7826b589b065a4bb235d44535df57`。

此前安装记录（本轮未重新核查服务/插件状态）：Drover 包在 `plugins/drover/dist/drover-plugin`，清单保留 `--dispatch-log /Users/firegnu/Developer/personal_projs/dispatch-log/dlog`，重打包会覆盖自定义 args。旧 `dev.drover.loop` 及 drover/drover-board 链接已撤下，旧仓库与数据保留；替换备份 `~/Library/Application Support/saddle-release-backups/drover-native-20260930-213348/`。

优先阅读：

1. `docs/任务/Clawd剩余十一个紧凑动作.md`、`docs/任务/Clawd六个紧凑动作.md`、`docs/任务/Clawd兴奋与海盗.md`、`docs/任务/Clawd墨镜额头修正.md`、`docs/任务/Clawd精选动作.md`、`docs/任务/Clawd尺寸收缩.md`、`docs/DESIGN.md`、`src/mascot.rs`、`assets/clawd/README.md`
2. `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`、`AGENTS.md`
3. `docs/插件系统设计.md`、`docs/插件协议.md`、`docs/插件开发入门.md`
4. `plugins/drover/README.md`、`src/plugins/{registry,runtime}.rs`、`crates/plugin-protocol/src/lib.rs`
5. `../dispatch-log/USAGE.md`、`../dispatch-log/dispatch_log/`、`../corral/corral-dispatch-skill/`、`../corral/docs/CONTRACT.md`

## 6. 下一步

先等用户观察重启后的吉祥物效果；除非用户提出调整，不自行扩展。用户触发统一产品迁移后，围绕已接受的外围顺序补齐日志存储/公开接口调查，拟定最小 core plugin 入口与兼容方案，再隔离实现；保留记录 ID/历史与投递语义，接好主控/Drover 消费者，此阶段不动 Corral infra。不自动推进迁移或队列任务。
