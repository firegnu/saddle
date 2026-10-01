# 会话交接

更新：2026-10-01。当前 main；Clawd 的 Opus 原版对照精修已审查、合并、备份发布安装并清理，用户确认“那就先这样”。用户已确认下一主题为统一 Saddle 产品迁移，稍后回来先讨论设计；本轮只更新交接、提交并推送，不启动新任务。

## 1. 会话摘要

Clawd 本轮完成并收住：31段、1319帧，16列×3行；Opus精修11段，静止与其余20段保持不变。用户确认下一步先讨论 dispatch-log、corral-dispatch 内置模块的接口、统一入口和兼容方案，再进入实施，Corral 暂时不动。等用户回来继续讨论，不将本次交接授权理解为立即实施或派发。

## 2. 完成的工作

- 实现 `8194bcc`，审查 `c4d6d46`，安装记录 `b4bd865`，收尾 `2353856`，前次交接 `870676c`，均已推送。本次编辑前main干净并与origin/main一致；本次仅更新HANDOFF，提交后推送。
- 精修 looking、waving、thinking、headphones、watch、snooze、hulahoop、desktop、dancing、dancinghappy、swaying。静止帧和其余20段（含walking、turning）逐字节不变；无布局、PTY、状态绑定或随机调度改动。
- 主控看过全部11段对照图和代码diff，复核素材差异恰为11段、眼睛有肤色边距、四脚数量稳定、摇摆底行逐字节固定。取舍与理由见任务和DESIGN，不再扩大范围。
- 实现者完整测试日志核得45套件、377 passed / 0 failed / 5 ignored；Clippy通过依据其完成记录。实现者实际四次全量超出一次预算，已明确记录，主控未重复标准套件。发布构建成功；发布版Mascot 9 passed。日志 `/tmp/clawd-opus-review-impl/cargo-test.log`、`/tmp/saddle-clawd-opus-release-{build,test}.log`。
- Dispatch `4190240786d94e738c765ac187b630f5`；此前只读审查 `fe2b3f58445546a19f04d23472999a6a`。agent `saddle/dev-clawd-opus-review-1`（instance `17efec1ad4f8`）已在确认idle/attached0、清理工作目录后关闭；worktree `review-clawd-opus` 与分支 `clawd-opus-polish` 已安全删除。
- 本轮提醒曾在Opus中途停下时提前触发，继续后未重新挂，主控已向用户承认疏漏；最新回复为真正DONE，已读并收尾。

## 3. 当前安装与预览

- `~/.local/bin/saddle` → `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`。发布检查后SHA256 `8878d329da65371bc99b62ebd8dbf3a1b5a74179a36a3eecbdbd09ad69b87d3b`，核实包含当前190302字节完整素材。未重启用户窗口，新启动进程使用新版。
- 构建前备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-opus-polish-20261001-151441`；旧SHA256 `7ae6d76083bd2aa41775d2f61162113d8a91a8dc0ae9e63291566d6c323d3e9e`。
- 最新对照在 `~/Downloads/clawd-reference-20261001-4pdhpd0_/`：`saddle-opus-polish-11-before-after.png`（上旧下新）、`.mp4`（左旧右新，12fps）。旧 `saddle-curated-31` 和其他预览未更新，不要误当新版。原始 `preview.html` 为大体积内嵌素材，勿整份输出。
- CUA连接曾失败；用户已授权用系统open打开预览目录。不调查/安装字体，不用终端图片协议。

## 4. 待完成与约束

本轮授权工作已完成，用户决定先保持现状；不把等待实际窗口反馈作为继续推进的任务。预览和自动检查不代表实机观感验证。保留的 notebook、phone、laptop、arcade 原有右眼偏边缘问题未改，范围外；不自动开修复。原版下蹲、上下看、腮红在3行限制下未强行实现。

共31段为30段参考动作加原创海盗；8段未加入：jumping、jumpinghappy、juggling、confetti、confettihappy、rainbow、kite、breakdancing。调度仍自主walking/边缘转身与随机动作、最近5段排除，不关联agent状态。

保留既有 `../saddle-worktrees/t38-dispatch-study` 与 `t55-notification-flow`；不合并、不清理。用户agent不stop/send/keys，不批量杀进程。真实队列无操作，不自动下一项。共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；因日常链接直接指向release，构建前先备份。

统一产品迁移待用户触发，既定顺序为 dispatch-log → corral-dispatch → 清理Saddle/Drover/skill外围接入 → 最后Corral，不重新讨论已接受产品边界。不顺带建中央daemon、任务引擎或重写终端。外围仍只通过Corral公开CLI。

此前安装记录（本轮未重验插件/服务）：Drover包 `plugins/drover/dist/drover-plugin`，清单自定义args保留 `--dispatch-log /Users/firegnu/Developer/personal_projs/dispatch-log/dlog`，重打包会覆盖。旧独立CLI/watch已退役，旧仓库数据保留，备份 `~/Library/Application Support/saddle-release-backups/drover-native-20260930-213348/`。任务操作只走Saddle公开ctl plugin/request，submit/accept/dispatch分开，未知结果不重发。

## 5. 优先阅读

1. `docs/任务/Clawd动作精修实施-Opus.md`、`docs/调研/Clawd动作精修-Opus审查-2026-10-01.md`、`docs/DESIGN.md` 的Clawd末尾章节。
2. `scripts/prepare-clawd.py`、`assets/clawd/README.md`、`src/mascot.rs`、`tests/mascot.rs`；此前各批动作记录在 `docs/任务/Clawd*.md`。
3. `AGENTS.md`、`../dispatch-log/USAGE.md`；后续分派用corral-dispatch技能，等待新轮完成时重新挂提醒。
4. 迁移触发后读 `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`、插件设计/协议文档、`plugins/drover/README.md`。

## 6. 下一步

用户回来后，从 `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md` 第7、9、11、12节接续讨论，先形成可实施的设计方案：

1. dispatch-log、corral-dispatch 内置模块如何由 Saddle 提供，命令入口是否脱离 TUI 可用。
2. 记录数据和历史 ID 如何兼容，模块之间使用什么接口。
3. skill 资源如何随版本提供、安装和更新。

产品目标和迁移顺序已确定，不重新打开讨论：一个产品统一交付；先dispatch-log，再corral-dispatch，再清理外围调用，最后Corral。具体接口和切换方案尚未确定；讨论完成前不移植、不派发、不动真实历史或队列。Clawd没有待启动的新任务。
