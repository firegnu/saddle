# 会话交接

更新：2026-10-01。当前分支 `main`。本轮完成 Clawd 墨镜额头缺口修正并安装；统一 Saddle 产品迁移仍未开始，等用户触发。主控亲自做，不自行委派。

## 1. 会话摘要

用户认可静止效果，但反馈动作粗糙，要求精选十来个容易表达的动作精修，尽量不越过agent上方边框。用户确认12种动作；已改成固定静止轮廓的小幅关键帧，全部放在3行内，合并推送并安装。用户之后确认其他动作都没问题，仅反馈墨镜像额头缺口；已修正镜片，其他11段逐字节不变。没有重启用户窗口，等待其实际反馈。

## 2. 完成的工作

- 最新墨镜修正`f82361f`，安装记录`f8f6aaa`，收尾`9618e2d`。镜片改为肤色背景上的`■`，保留完整额头和细鼻梁，尺寸/时间不变。公开绘制检查先RED后GREEN；标准全量**375 passed / 0 failed / 5 ignored**、Clippy/fmt/diff通过，发布版7项Mascot检查通过。日志`/tmp/saddle-clawd-sunglasses-{red,green,all,clippy,release-test}.log`。
- 墨镜对照`saddle-sunglasses-fix.png`/`.mp4`在原预览目录；12动作总览与90秒巡游均已同步更新。记录`docs/任务/Clawd墨镜额头修正.md`；Dispatch `cdebdd65b29646f38d2a0703aecb9003`；worktree/分支`clawd-sunglasses`已清理。

- 实现`e3172f4`、安装记录`3c10099`、收尾`297fd89`。仅播放walking、转身、张望、招手、思考、咖啡、笔记、耳机、看表、打盹、墨镜、轻轻摇摆；用户已确认这个名单，替代此前全部38段的要求。
- 画布16列×3行，全部422帧在agent上边框上方。静止帧去掉两行空白后与此前已认可版本逐字节一致；保持tab/pane/PTY布局。动作改为离线网格关键帧和停顿，固定地面，缩小道具；不跟随agent状态、不绑定终端、不调查或安装字体。
- 此前精选动作版本的标准`cargo test --all-targets`为**374 passed / 0 failed / 5 ignored**；Clippy/fmt/diff通过。新增公开绘制检查先RED（旧版tick107跨边框）、后GREEN。日志`/tmp/saddle-clawd-curated-{red,green,all-final,clippy}.log`。
- 发布版巡游/鼠标避让与设置/布局两项隔离流程通过，日志`/tmp/saddle-clawd-curated-release-{patrol,settings}.log`。使用假corral/临时HOME，没有操作用户agent或真实队列。
- 预览均在`~/Downloads/clawd-reference-20261001-4pdhpd0_/`：`saddle-curated-12.mp4`/`.gif`为全部12动作总览，`saddle-curated-12-poses.png`为代表姿态，`saddle-curated-patrol.mp4`为实际Mascot.draw导出的90秒巡游。指定字号栅格化预览，不是用户窗口截图。桌面控制连接失败；已按既有授权用系统open打开目录。
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

`~/.local/bin/saddle` → `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`。当前SHA256 `1fdab61fcdb50cd5c34ccb29fe4fa7cfff6c108d410adac2a988f2b4ce4e7530`，已验证内含墨镜修正素材。旧版备份`/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-sunglasses-20261001-130622`，旧hash`cdfafd4527ad23cc0746083eff86d11285f51e237e47a5558f866ab788ed7bdf`。

此前安装记录（本轮未重新核查服务/插件状态）：Drover 包在 `plugins/drover/dist/drover-plugin`，清单保留 `--dispatch-log /Users/firegnu/Developer/personal_projs/dispatch-log/dlog`，重打包会覆盖自定义 args。旧 `dev.drover.loop` 及 drover/drover-board 链接已撤下，旧仓库与数据保留；替换备份 `~/Library/Application Support/saddle-release-backups/drover-native-20260930-213348/`。

优先阅读：

1. `docs/任务/Clawd墨镜额头修正.md`、`docs/任务/Clawd精选动作.md`、`docs/任务/Clawd尺寸收缩.md`、`docs/DESIGN.md`、`src/mascot.rs`、`assets/clawd/README.md`
2. `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`、`AGENTS.md`
3. `docs/插件系统设计.md`、`docs/插件协议.md`、`docs/插件开发入门.md`
4. `plugins/drover/README.md`、`src/plugins/{registry,runtime}.rs`、`crates/plugin-protocol/src/lib.rs`
5. `../dispatch-log/USAGE.md`、`../dispatch-log/dispatch_log/`、`../corral/corral-dispatch-skill/`、`../corral/docs/CONTRACT.md`

## 6. 下一步

先等用户观察重启后的吉祥物效果；除非用户提出调整，不自行扩展。用户触发统一产品迁移后，围绕已接受的外围顺序补齐日志存储/公开接口调查，拟定最小 core plugin 入口与兼容方案，再隔离实现；保留记录 ID/历史与投递语义，接好主控/Drover 消费者，此阶段不动 Corral infra。不自动推进迁移或队列任务。
