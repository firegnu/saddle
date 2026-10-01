# 会话交接

更新：2026-10-01。当前分支 `main`。本轮完成 Clawd 尺寸缩小与方形眼睛交付；统一 Saddle 产品迁移仍未开始，等用户触发。主控亲自做，不自行委派。

## 1. 会话摘要

用户要求 Clawd 贴近 claude.dev 参考大小与动作，保留全部动作及自主往返，不随 agent 状态变化。本轮将画布从18×7缩为16×5，用户认可大小，随后调整眼睛。用户最后要求“先这样吧。继续往下走”，停止进一步字体/半格定位调查，按当前右眼内收版本合并推送并更新安装；未重启用户窗口。

## 2. 完成的工作

- 本轮实现`b577dda`、`5333405`、`d36ef2c`、`f9b7080`；安装记录`cc82509`；收尾`902975b`。画布16列×5行，站立身体约2–3行；保留38段素材、来回walking、动作节奏、Settings开关和tab/pane/PTY布局，不绑定特定终端、不增加图片协议或字体依赖。
- 眼睛为小方块`▪`，修正额头缺口、重复采样与右侧贴边；两眼整体居中仍有字符格误差，未完成精准居中。用户要求先保留现状，不自行继续调查字体。
- 最新全量串行`cargo test --all-targets -- --test-threads=1`为**373 passed / 0 failed / 5 ignored**；Clippy/fmt/diff通过。默认并发全量两次在`overlay_resize_rejects_the_previously_displayed_frame`遇到`plugin registry busy; refresh and retry`，未改插件代码；此前方形眼睛版本的默认全量通过。日志`/tmp/saddle-clawd-eye-inset-{green,all,all-retry,all-serial,clippy}.log`。
- 发布版巡游/鼠标避让及设置/布局两项隔离流程通过，日志`/tmp/saddle-clawd-size-release-{overlay,settings}.log`。全程假corral/临时HOME，未操作真实agent或队列。
- 证据：`docs/任务/Clawd尺寸收缩.md`；Dispatch `5d674c0d0ede487281c442abd41e0b76`。最新对照`~/Downloads/clawd-reference-20261001-4pdhpd0_/saddle-eye-inset-comparison.png`；原网页`preview.html`同目录。预览为指定字号栅格化，并非用户运行窗口截图。
- 本轮worktree/分支`clawd-size-only`已清理，没有创建agent。此前自由巡游实现`e3ba155`、收尾`e7aea8a`仍保留。
- 此前统一产品调查报告已提交 `aeff1d6`：`docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`。用户确定外围优先；本轮未开展迁移。之前临时 Diff 演示已清理，不恢复。

## 3. 待完成与未提交状态

本轮已按用户要求结束视觉微调并交付；眼睛整体居中误差保留，不声称精准复刻。交接提交后应保持main干净并与origin/main一致。正在运行的窗口未重启，下次启动加载新版；等待用户实际观察动作与大小，不自行扩大范围。

保留既有 worktree/分支 `../saddle-worktrees/t38-dispatch-study`、`../saddle-worktrees/t55-notification-flow`，不合并、不清理。统一产品迁移待用户触发；需确定内置插件调用方式、CLI、历史记录兼容、模块接口及 skill 安装更新，不再重开产品边界和外围优先顺序。

## 4. 操作约束

- 用户要求主控亲自做，覆盖默认委派流程。现有用户 agent 不 stop/send/keys，不批量杀进程；本轮没有需要关闭的自建 agent。
- 统一产品既定顺序：dispatch-log → corral-dispatch → 清理 Saddle/Drover/skill 外围接入 → 最后 Corral。自有可执行逻辑迁 Rust；Skill Markdown 作为随产品资源保留，不机械化主控判断。不顺带建中央 daemon、自动任务引擎或重写终端渲染。
- 外围阶段仅通过 Corral 公开 CLI；不读内部运行文件。当前 Drover 是完整进程插件，拥有数据、状态和通知；不恢复旧 CLI/watch。插件关闭面板继续后台，停用/退出 Saddle 停止观察；Corral agent 继续存活。
- 任务操作只走 `saddle ctl instances` → `saddle ctl plugin --instance ID --plugin drover --method METHOD --params JSON` → `ctl request`。submit/accept/dispatch 分开，不自动下一项；结果未知不重发，令牌过期重新读取，不操作真实任务文件。
- 继续用共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。因日常链接直接指向该 release 二进制，构建 release 前先备份。

## 5. 当前安装与重要文件

`~/.local/bin/saddle` → `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`。当前SHA256 `1fcb2d8d0bcd4fa023032c81ebb3e99d252aa4ee1a94d8851b20a4ad7fb65eb5`。旧版备份 `~/Library/Application Support/saddle-release-backups/clawd-size-20261001-122059/`，旧hash `d2ebb2f549167413324afedff7aac41207e3be97302c7d3c289f0dd0e4fafb16`。

此前安装记录（本轮未重新核查服务/插件状态）：Drover 包在 `plugins/drover/dist/drover-plugin`，清单保留 `--dispatch-log /Users/firegnu/Developer/personal_projs/dispatch-log/dlog`，重打包会覆盖自定义 args。旧 `dev.drover.loop` 及 drover/drover-board 链接已撤下，旧仓库与数据保留；替换备份 `~/Library/Application Support/saddle-release-backups/drover-native-20260930-213348/`。

优先阅读：

1. `docs/任务/Clawd尺寸收缩.md`、`docs/任务/Clawd自由巡游与网页动作.md`、`docs/DESIGN.md`、`src/mascot.rs`、`assets/clawd/README.md`
2. `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`、`AGENTS.md`
3. `docs/插件系统设计.md`、`docs/插件协议.md`、`docs/插件开发入门.md`
4. `plugins/drover/README.md`、`src/plugins/{registry,runtime}.rs`、`crates/plugin-protocol/src/lib.rs`
5. `../dispatch-log/USAGE.md`、`../dispatch-log/dispatch_log/`、`../corral/corral-dispatch-skill/`、`../corral/docs/CONTRACT.md`

## 6. 下一步

先等用户观察重启后的吉祥物效果；除非用户提出调整，不自行扩展。用户触发统一产品迁移后，围绕已接受的外围顺序补齐日志存储/公开接口调查，拟定最小 core plugin 入口与兼容方案，再隔离实现；保留记录 ID/历史与投递语义，接好主控/Drover 消费者，此阶段不动 Corral infra。不自动推进迁移或队列任务。
