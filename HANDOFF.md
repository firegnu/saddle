# 会话交接

更新：2026-10-01。当前分支 `main`。**下一项是统一 Saddle 产品：先迁 dispatch-log/corral-dispatch 到 Rust core plugins，Corral 最后迁。用户要求今天保管调研，明天开始；本轮没有实施迁移。主控亲自做，不自行委派。**

## 1. 会话摘要

完成了 Saddle、Corral、corral-dispatch、dispatch-log 的源码/契约/部署调查，并参考 Herdr 官方文档及 v0.9.3 源码。用户明确要求统一产品，纠正了最初 Corral 优先、dlog 外部可选的建议；报告已按外围优先重写并提交推送。

## 2. 完成的工作

- 调研：`docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`，提交 `aeff1d6`，已推送 `origin/main`。包含源码依据、依赖、core plugin 边界、Rust 迁移风险、兼容与推进顺序。此次只做静态调查和文档检查，没有原型、构建、测试或真实服务/agent/队列操作。
- 此前已完成 Clawd 默认启用配置 `mascot_enabled`、Settings 开关、Working 内脚小步与 Waiting 轻微眨眼/示意；动作实现 `cbc4398`，收尾 `79ee94f`，交接 `45ea8e7`。不再自行扩展吉祥物调整。
- 上一次代码交付记录：标准检查 **374 passed / 0 failed / 5 ignored**，Clippy/fmt/diff 及隔离 release 检查通过；不是本轮重新运行的结果。证据见 `docs/任务/Clawd忙碌与等候微动作.md`，Dispatch `04f2f51e9de448619b8ca7d3b991719e`，日志 `/tmp/saddle-clawd-busy-{all,clippy,red,green,release}.log`。
- 已有产品基础：进程插件支持居中/tab/split；Drover 已完整插件化，Diff 已支持自动刷新；Settings/Plugins 界面整理已交付。历史实现与验证在 `docs/DESIGN.md`、`docs/任务/`，旧完整交接可从 `aeff1d6:HANDOFF.md` 查阅。

## 3. 待完成与未提交状态

**保留的三处未提交变更是用户要求的 Diff 演示，不是迁移代码，不要提交、还原或清理：**

- `examples/counter-plugin/src/main.rs`（修改）
- `plugins/diff/README.md`（修改）
- `docs/diff-preview-demo.json`（未跟踪）

三份文件与 `/tmp/saddle-settings-pages-demo-hashes.json` 对照一致。除本次交接提交外，调研已提交推送；迁移尚未开始。

仍需设计：内置插件调用/运行方式、是否支持脱离 TUI 调用、dispatch-log 历史记录兼容、模块间接口、skill 资源安装与更新。用户已经确定产品边界和外围优先顺序，不要再把这些重新列成待批准问题。具体工程接口尚未定稿。

保留 worktree/分支：`../saddle-worktrees/t38-dispatch-study`、`../saddle-worktrees/t55-notification-flow`。不合并、不清理；本轮没有创建 agent 或 worktree。原 Claude 讨论 agent 已在此前按用户要求关闭。

## 4. 关键决定与操作约束

- 最终只有一个 Saddle 产品，统一仓库、Rust 实现的自有运行逻辑、构建、版本和发布；不能把三四个独立包用安装器拼起来作为最终交付。
- **先 dispatch-log → corral-dispatch → 清理 Saddle/Drover/skill 外围接入 → 最后 Corral。** 两个外围模块迁为产品自带 core plugins，用户无需分别安装或配置可执行文件路径。先日志再派发是首阶段内部的建议顺序。
- Skill 的 Markdown 保留为随产品提供的指令资源；可执行工具迁 Rust，不将主控判断机械变成强制流程。不顺带建设中央 daemon、自动任务引擎或重写终端渲染。
- 外围阶段继续通过 Corral 公开 CLI；不读它的内部运行文件。最后迁移核心前保持其生命周期、instance、投递确认和退出行为。统一入口不等于所有功能必须在同一进程。
- 用户要求主控亲自做，覆盖仓库默认委派流程；不要自行开 agent。现有用户 agent 不 stop/send/keys，不批量杀进程。真正开始实现前更新正式设计；当前报告仍是接口方案的讨论依据。
- 当前 Drover 是一个完整 Rust 进程插件，拥有任务数据、状态和通知；旧 CLI/watch 已退役，不恢复旧服务。关闭面板继续后台，停用/退出 Saddle 停止观察通知，用户已接受；Corral agent 则继续存活。
- 当前任务操作只走 `saddle ctl instances` → `saddle ctl plugin --instance ID --plugin drover --method METHOD --params JSON` → `ctl request`。`submit`/`accept`/`dispatch` 分开，无自动下一项，Git 只作参考。结果未知不重发；过期令牌重新读取，不直接操作真实任务文件。
- 源码已核对：Corral `/Users/firegnu/Developer/personal_projs/corral`，remote `https://github.com/firegnu/corral.git`，研究基线 `6923da1`；dispatch-log `/Users/firegnu/Developer/personal_projs/dispatch-log`，基线 `9aa780c`。corral-dispatch 就在 Corral 的 `corral-dispatch-skill/`，并非第三个 repo/service。

## 5. 当前安装与优先阅读文件

本次只读核实：`~/.local/bin/saddle` 链接 `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`；SHA-256 为 `71791e53b4fb94e39c067c2a59b756475f7afdf323a25b7b9032cfc493c9d2f9`。最新宿主备份 `~/Library/Application Support/saddle-release-backups/clawd-busy-20261001-024526/`。未重启用户窗口，也未确认其窗口加载版本。

此前安装记录（本轮没有重新核查服务/插件状态）：Drover 包在 `plugins/drover/dist/drover-plugin`，清单保留 `--dispatch-log /Users/firegnu/Developer/personal_projs/dispatch-log/dlog`，重打包会覆盖自定义 args；此依赖正是下一阶段要消除的内容。旧 `dev.drover.loop` 及 drover/drover-board 链接已撤下；旧仓库与数据保留。完整替换备份在 `~/Library/Application Support/saddle-release-backups/drover-native-20260930-213348/`。

优先阅读：

1. `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md`（最新产品决定、首阶段职责、源码证据）
2. `AGENTS.md`、`docs/DESIGN.md`（结合本会话用户覆盖指令）
3. `docs/插件系统设计.md`、`docs/插件协议.md`、`docs/插件开发入门.md`
4. `plugins/drover/README.md`、`src/plugins/{registry,runtime}.rs`、`crates/plugin-protocol/src/lib.rs`
5. `../dispatch-log/USAGE.md`、`../dispatch-log/dispatch_log/{cli,corral,route,record,store}.py`
6. `../corral/corral-dispatch-skill/`、`../corral/docs/CONTRACT.md`

## 6. 下一步

明天继续时，直接围绕已确定的外围迁移展开：补齐日志存储/公开接口调查，拟定最小 core plugin 入口和兼容方案，再进入隔离实现与验证。实现保持现有记录 ID/历史与投递语义，接好主控和 Drover 消费者；此阶段不动 Corral infra，不把模型策略、任务规则或其他功能一并重做。

正式开发遵守项目标准测试与共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；真实安装/数据切换放在验证之后处理。今天到交接保存为止，不自动开始迁移或推进任务。
