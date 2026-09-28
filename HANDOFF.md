# 交接

更新：2026-09-28。设计与理由见 `docs/DESIGN.md`；本次证据见 `docs/任务/T23-快速切换与临时放大.md` 和 `docs/任务/T23-主控审查.md`。

## 当前状态

用户已验证 T23，确认“基本满足我的要求”，T23 不再待放行。用户已手动派发 TASK T22，公开队列 current=T22（doing）、awaiting=null。Attention 按 DESIGN 第 42 节与 `docs/任务/T22-统一待处理入口.md` 实施；主控正创建 `t22-attention` 独立 worktree 并派出实现者。

T23 已实现、主控复核通过、本地合并并构建发布。实现 `8540b73`，鼠标手势隔离返工 `b5a1634`，合并 `fa44936`，收尾空提交 `3365c54`。本次交接随收尾文档提交推送 origin/main；后续以实时 Git 状态为准。

- Agents 按 `/` 或点 `/ Search` 搜索项目名／agent 名称，点击或 Enter 进入；已打开的跳到原窗格。
- split 活动窗格底部提供 Zoom／Restore；临时占满右侧，保留 Agents、tab 条和后台会话，恢复原布局及焦点。单窗格无 Zoom。
- 主控发现的搜索点击鼠标松开泄漏已修复，回归确认 PTY 仅收到随后正常输入。

## 验证与发布

- 主控第一轮标准检查：非 workflow 全通过，workflow 59 passed、1 failed、2 ignored；失败为旧 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases` 在 picker 后等待 Create agent 超时。实现者首轮全套通过，两份记录各自保留。
- 返工定向复核：T23 workflow 3 passed、terminals 11 passed、该旧 T20 用例单独 1 passed；Clippy 全 targets、fmt、diff 通过。按预算未重复全套，不宣称 T29 已修复。
- `cargo build --release` 通过；共享 target/release/saddle、仓库 target/release/saddle 和默认 ~/.local/bin/saddle 三入口 SHA-256：`9dabe776ea53e79ec290de879638bf14c87bf4c64449933babbd3c31701c6e30`。默认入口仍链接共享 release。未重启用户现场；下次启动使用新版本。

## 队列与开发环境

- 最新公开队列：current=T22（doing）、awaiting=null、loop=false、gate=true、paused=false。本轮未调用 go/next。
- Pending 顺序：T27 绑定位置说明 → T29 偶发测试失败 → T28 ctl 上限 → T24 终端历史搜索复制 → T25 布局恢复 → T26 任务产物跳转。逐项讨论后再实施，不自动派发下一件。
- 自有 `t23-search-zoom` worktree／分支已清理；`saddle/dev-t23-search-zoom-1` 在工作目录删除后已一并关闭。仓库外临时审查探针已清理；用户 agent 未动。迟到提醒查到 not_found 即忽略。

## 仍需注意

- T29 的 picker／close confirmation 偶发 workflow 问题未定位。一次定向或全套通过都不能当成修复；T23 主控所见失败保留在审查记录。
- 共用 target 在跨 checkout／临时副本复用时曾读到旧二进制；缺少新 UI 的失败不算目标 RED。运行检查时核实构建对应当前源码，不靠重复测试碰运气。
- ctl 单实例 256 次修改上限仍在，已单列 T28。本轮不改 corral／drover／corral-dispatch，也不改全局 skill。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。

## 下一步

派出 T22 实现者后等待完成提醒，按任务单做主控审查、合并和收尾。T22 以外待办继续等逐项讨论，不自动推进队列。
