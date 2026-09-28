# 交接

更新：2026-09-28。设计与理由见 `docs/DESIGN.md` 第 43 节；本次证据见 `docs/任务/T27-绑定打开位置说明.md`。

## 当前状态

T27 绑定打开位置说明已实现、主控审查通过、本地合并并构建发布。实现 `3633bd2`，合并 `a90d8cb`，审查与设计取舍记录 `8daf3fb`，收尾空提交 `4a44769`。本次交接随收尾文档提交推送 origin/main；后续以实时 Git 状态为准。

- 从 + Tab／Split 打开的 New agent，在 Advanced 中以静态灰字说明打开位置与来源（`Opens in … · set by …`），无点击命中，Tab 焦点跳过该行。
- 从 Agents → New 进入仍保留位置选择器；创建命令、绑定规则和终端行为不变。
- T23 已获用户现场验证“基本满足我的要求”；T22 已放行，Attention 入口可用。T27 用户已确认“看到了，感觉还不错”，公开队列已无待放行。
- T24 用户已明确「确认，派发」，交互与范围已写入 DESIGN 第 44 节和 `docs/任务/T24-终端历史查看搜索与复制.md`。此前队列派发正文保留了审查稿措辞，现由用户后续确认和最新任务书取代，主控按最终方案派发。

## 验证与发布

- T27 实现者定向渲染测试 `bound_location_is_static_text_and_new_keeps_the_selector`、fmt、diff 检查通过。主控核对 diff、测试内容和完成记录，diff 检查通过；按纯呈现调整预算未重跑套件或执行全套测试、Clippy。
- 最近一次全套为 T22 主控检查：195 passed、0 failed、2 ignored；Clippy 通过。旧 T20 picker 偶发失败仍归 T29，不宣称已修复。
- main 的 `cargo build --release` 通过；共享 target/release/saddle、仓库 target/release/saddle 和默认 ~/.local/bin/saddle 三入口 SHA-256：`768918c8c948b2b1600372efa74536cbb1863ea8bddffdb1beef81a6ce8e36a3`。默认入口仍链接共享 release。未重启用户现场；下次启动使用新版本。

## 队列与开发环境

- T27 已完成并放行；最新公开队列 current=T24（doing）、awaiting=null，loop=false、gate=true。未调用 go/next。
- Pending 顺序：T25 布局恢复 → T26 任务产物跳转 → T29 偶发测试失败 → T28 ctl 上限。逐项讨论后再实施，不自动派发下一件。
- 自有 `t27-bound-location` worktree／分支已清理，确认 idle、attached=0、工作区干净且分支合并后，删除 worktree 并一并关闭 `saddle/dev-t27-bound-location-1`（instance `d5e214c9e1e2`）。当前仅 main worktree；用户 agent 未动。迟到提醒查到 not_found 即忽略。

## 仍需注意

- T29 的 picker／close confirmation 偶发 workflow 问题未定位。一次定向或全套通过都不能当成修复。
- 共用 target 在跨 checkout／临时副本复用时曾读到旧二进制；缺少新 UI 的失败不算目标 RED。运行检查时核实构建对应当前源码，不靠重复测试碰运气。
- ctl 单实例 256 次修改上限仍在，已单列 T28。本轮未改 corral／drover／corral-dispatch，也未改全局 skill。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。

## 下一步

派出 T24 实现者后等待完成提醒，读取状态和回复，按任务单做主控审查、合并和收尾。其余待办继续等逐项讨论，不自动推进队列。
