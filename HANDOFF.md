# 交接

更新：2026-09-28。设计与理由见 `docs/DESIGN.md` 第 42 节；本次证据见 `docs/任务/T22-统一待处理入口.md`。

## 当前状态

T22 Attention 已实现、主控审查通过、本地合并并构建发布。实现 `212d295`，合并 `67519b1`，审查与设计取舍记录 `e59ee2c`，收尾空提交 `6790397`。本次交接随收尾文档提交推送 origin/main；后续以实时 Git 状态为准。

- Agents 标题下增加 `Attention · N`，点击或在 Agents 按 `a` 打开；弹层分 Needs attention 和 New replies。
- 跨项目汇总等待输入、错误、待放行、历史失败和本次运行内未读回复；打开行进入对应 agent 或项目任务，已打开的 agent 复用原窗格。
- 查看不处理等待／待放行；历史失败可显式 Mark seen，本次运行内隐藏，不改 drover 历史。读取失败显示失败来源。
- T23 已获用户现场验证“基本满足我的要求”；T22 用户已看到入口，公开队列已无待放行。T27 方案已确认，设计见第 43 节，任务书 `docs/任务/T27-绑定打开位置说明.md` 已准备；尚未创建 worktree／agent 或开始实现。

## 验证与发布

- T22 主控标准检查：`cargo test --all-targets` 195 passed、0 failed、2 ignored（workflow 62 passed、2 ignored）；Clippy 全 targets、fmt、diff 检查通过。
- 实现者首轮标准检查曾遇旧 T20 picker 偶发失败，主控本轮未复现；不据此认定 T29 已修复，详见任务完成记录与主控审查。
- main 的 `cargo build --release` 通过；共享 target/release/saddle、仓库 target/release/saddle 和默认 ~/.local/bin/saddle 三入口 SHA-256：`21b5f85ecab1554d40e417f8f1320c9d4b0c9692e04c08cca3cc8c5b5e916be9`。默认入口仍链接共享 release，`--help` 已包含 Attention。未重启用户现场；下次启动使用新版本。

## 队列与开发环境

- 最新公开队列 current=null、awaiting=null，loop=false、gate=true、paused=false，T22 已不再待放行。本轮仅同步 T27 待办说明，未调用 go/next。
- Pending 顺序：T27 绑定位置说明 → T29 偶发测试失败 → T28 ctl 上限 → T24 终端历史搜索复制 → T25 布局恢复 → T26 任务产物跳转。逐项讨论后再实施，不自动派发下一件。
- 自有 `t22-attention` worktree／分支已清理；确认 idle、attached=0、工作区干净且分支已合并后，删除 worktree 并一并关闭 `saddle/dev-t22-attention-1`（instance `21cf1c48ece8`）。当前仅 main worktree；用户 agent 未动。迟到提醒查到 not_found 即忽略。

## 仍需注意

- T29 的 picker／close confirmation 偶发 workflow 问题未定位。一次定向或全套通过都不能当成修复。
- 共用 target 在跨 checkout／临时副本复用时曾读到旧二进制；缺少新 UI 的失败不算目标 RED。运行检查时核实构建对应当前源码，不靠重复测试碰运气。
- ctl 单实例 256 次修改上限仍在，已单列 T28。本轮未改 corral／drover／corral-dispatch，也未改全局 skill。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。

## 下一步

等待用户手动派发 TASK T27，再按已备任务单创建独立 worktree 并派实现者；其余待办继续等逐项讨论，不自动推进队列。
