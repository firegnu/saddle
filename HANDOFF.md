# 会话交接

更新：2026-10-01。当前分支 `main`。本轮完成 Clawd 自由巡游与跨边框动画；统一 Saddle 产品迁移仍未开始，等用户触发。主控亲自做，不自行委派。

## 1. 会话摘要

用户认可 claude.dev 提取的吉祥物大小与动画，要求保留完整动作、加来回 walking，不对应 agent 状态。最终确认保留 tab 右侧位置，允许动画跨越下方 agent 上边框、覆盖少量顶部内容，并减弱边框颜色；撤回新增高度方案。已实现、验证、合并推送及更新安装版本，未重启用户窗口。

## 2. 完成的工作

- 实现提交 `e3ba155`，收尾 `e7aea8a`。全部38个素材片段内置；自主往返、停下做动作、边缘转身。保留 tab/pane/PTY 几何，保护控件与浮层点击，沿用 Settings 开关。设计与取舍见 `docs/DESIGN.md` 最后一个 Clawd 章节。
- 最终标准测试 **372 passed / 0 failed / 5 ignored**；Clippy、fmt、diff 通过，3项变异检查检出目标退化，2项隔离发布版流程通过。证据：`docs/任务/Clawd自由巡游与网页动作.md`；Dispatch `2fc3de7c1ae74a84878685a1db8de8b4`；日志 `/tmp/saddle-clawd-{all,clippy}-final.log` 和 `/tmp/saddle-clawd-release-{overlay,settings}.log`。
- 原网页离线提取与预览：`~/Downloads/clawd-reference-20261001-4pdhpd0_/preview.html`；实际 Rust Buffer 的栅格化效果预览 `saddle-overlay-preview.gif`/`.png`（不是用户窗口截图）。终端网格会损失微小细节，物理大小随字体变化。
- 本轮 worktree/分支 `clawd-free-roam` 已清理；没有创建 agent，没有操作真实队列或现有 agent。
- 此前统一产品调查报告已提交 `aeff1d6`：`docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`。用户确定外围优先；本轮未开展迁移。之前临时 Diff 演示已清理，不恢复。

## 3. 待完成与未提交状态

本轮功能暂无已知待完成工作，交接提交后应保持 main 干净并与 origin/main 一致。发布版已通过假 agent 隔离检查；用户正在运行的窗口未重启，尚未取得其实际显示反馈，下次启动加载新版。

保留既有 worktree/分支 `../saddle-worktrees/t38-dispatch-study`、`../saddle-worktrees/t55-notification-flow`，不合并、不清理。统一产品迁移待用户触发；需确定内置插件调用方式、CLI、历史记录兼容、模块接口及 skill 安装更新，不再重开产品边界和外围优先顺序。

## 4. 操作约束

- 用户要求主控亲自做，覆盖默认委派流程。现有用户 agent 不 stop/send/keys，不批量杀进程；本轮没有需要关闭的自建 agent。
- 统一产品既定顺序：dispatch-log → corral-dispatch → 清理 Saddle/Drover/skill 外围接入 → 最后 Corral。自有可执行逻辑迁 Rust；Skill Markdown 作为随产品资源保留，不机械化主控判断。不顺带建中央 daemon、自动任务引擎或重写终端渲染。
- 外围阶段仅通过 Corral 公开 CLI；不读内部运行文件。当前 Drover 是完整进程插件，拥有数据、状态和通知；不恢复旧 CLI/watch。插件关闭面板继续后台，停用/退出 Saddle 停止观察；Corral agent 继续存活。
- 任务操作只走 `saddle ctl instances` → `saddle ctl plugin --instance ID --plugin drover --method METHOD --params JSON` → `ctl request`。submit/accept/dispatch 分开，不自动下一项；结果未知不重发，令牌过期重新读取，不操作真实任务文件。
- 继续用共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。因日常链接直接指向该 release 二进制，构建 release 前先备份。

## 5. 当前安装与重要文件

`~/.local/bin/saddle` → `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`。本轮发布版 SHA256 `d2ebb2f549167413324afedff7aac41207e3be97302c7d3c289f0dd0e4fafb16`。旧版备份 `~/Library/Application Support/saddle-release-backups/clawd-roam-20261001-112416/`，旧 hash `71791e53b4fb94e39c067c2a59b756475f7afdf323a25b7b9032cfc493c9d2f9`。

此前安装记录（本轮未重新核查服务/插件状态）：Drover 包在 `plugins/drover/dist/drover-plugin`，清单保留 `--dispatch-log /Users/firegnu/Developer/personal_projs/dispatch-log/dlog`，重打包会覆盖自定义 args。旧 `dev.drover.loop` 及 drover/drover-board 链接已撤下，旧仓库与数据保留；替换备份 `~/Library/Application Support/saddle-release-backups/drover-native-20260930-213348/`。

优先阅读：

1. `docs/任务/Clawd自由巡游与网页动作.md`、`docs/DESIGN.md`、`src/mascot.rs`、`assets/clawd/README.md`
2. `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`、`AGENTS.md`
3. `docs/插件系统设计.md`、`docs/插件协议.md`、`docs/插件开发入门.md`
4. `plugins/drover/README.md`、`src/plugins/{registry,runtime}.rs`、`crates/plugin-protocol/src/lib.rs`
5. `../dispatch-log/USAGE.md`、`../dispatch-log/dispatch_log/`、`../corral/corral-dispatch-skill/`、`../corral/docs/CONTRACT.md`

## 6. 下一步

先等用户观察重启后的吉祥物效果；除非用户提出调整，不自行扩展。用户触发统一产品迁移后，围绕已接受的外围顺序补齐日志存储/公开接口调查，拟定最小 core plugin 入口与兼容方案，再隔离实现；保留记录 ID/历史与投递语义，接好主控/Drover 消费者，此阶段不动 Corral infra。不自动推进迁移或队列任务。
