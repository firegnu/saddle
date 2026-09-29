# 交接

更新：2026-09-29。当前 main，T43 已审查、合并、清理并重新编译 release，待用户重启 Saddle 查看；不自动推进队列。

## 本轮 T43

- 核心实现 `45564a7`，滚动检查修正 `13a4ce2`，合并 `131b55c`，审查 `44ae599`，收尾 `bc9f79f`。首次成功列表确定默认折叠状态，之后新增／退出 agent 不再改变它；z 仍可手动切换。范围／设计见 `docs/任务/T43-Agents展开状态.md`、DESIGN §58。
- 主控首轮标准套件 283 passed／3 failed／3 ignored；clippy 通过。其中鼠标表单和待启动窗格 cwd 两项各单独一次复跑通过，不调查 T29。
- 直接相关的滚动失败已交原实现者修正：旧鼠标 y 命中头部分隔线，原自动折叠掩盖了测试无效。改为实际 agent 行后，主控定向滚动检查 1 passed（10.67 秒），确认到底、回顶、刷新保位、不 attach。产品滚动和布局未改。未重复全套，不将首轮改写为全绿。详细记录及日志见 `docs/任务/T43-主控审查.md`。
- main release 编译成功（5.43 秒，`/tmp/saddle-t43-release.log`），原 `~/.local/bin/saddle` 软链可用；未重启用户界面。
- 实现者 idle、attached=0，工作树干净且已合入后，删除 t43-agents-fold 分支／worktree，并关闭其中自开的 `saddle/dev-t43-agents-fold-1`。历史 t38-dispatch-study 保留，其他用户 agent 未动。
- 路由、派发、两轮回复、返工和审查 note 保存在同一 dispatch `29308f5afca7415dbb5a10cbd9edf606`。本交接随后提交推送，实际状态以 Git 为准。

## 队列与下一步

只读核对 current=T43、awaiting=null，pending 为 T29、T28、T32、T34；loop=false、gate=true、paused=false。本轮没有真实队列写操作。用户重启 Saddle 查看 T43，之后由用户放行；不自动推进 T29。T29 仍仅记录排查方向。

## 前次 T42

T42 已完成并推送 `2312d3d`，当前 release 包含它。任务书 `docs/任务/T42-焦点仓库Tasks.md`，设计 §57，Dispatch `25d80505684e4cbf9a6f757097e21e6a`。自动切换仅匹配已登记 Drover 项目；打开后有用户输入就放弃切换。该轮主控标准套件 282 passed／2 failed／3 ignored，两项单独复跑通过，限制保留，不重复验证。

## 前次 T41

T41 已完成并推送 `376b183`；用户确认返回按钮可用，也找到了 Dispatch 中 Start 行的派发任务书快照。任务文件 `docs/任务/T41-Dispatch返回按钮样式.md`，记录 ID `b460a96365bc4b14a347f2d307ef968a`。无需重复审查。

## 此前撤回待办功能

- Saddle 实现 9a65ba9、合并 302a7cc、收尾 6725382、交接 99a27b7；Drover 实现 d545ce9。两主控按用户授权直接协作，未给 T29 开始排查。设计见 DESIGN §55，验证见 `docs/撤回待办验证.md`。
- 实际 Panel／Client 与真实 Drover CLI 在临时 Git/HOME/XDG 中隔离联调通过。Saddle 当轮全套 282 passed／1 failed／3 ignored，失败是 picker 用例 tests/workflow.rs:2294，单独一次通过；clippy 通过。未把全套写成全绿或修复 T29 根因。
- Drover 主控后来报告：用户授权重启引擎，旧 PID 37103 换为 46666，部署记录 ce055de，三个项目均 loop off，部署未推进队列，未推送。此前“重启待授权”的状态已解除，不重复重启。用户随后反馈“看到了，很好用”；本轮只读确认 T29 已在 Pending。

## T38 与记录器

- T38 已合并、编译发布；用户已看到真实 Dispatch 记录。任务／审查见对应 T38 文档。首轮全套 control socket 单例失败、单独复跑通过的限制保留在审查，不重写为全绿。
- T38 记录 ID `973aed4655fa419fa4d78e0176d7085a`；本机记录器 `/Users/firegnu/Developer/personal_projs/dispatch-log/dlog`。Saddle 已配置可读取，当前不需重复配置。
- AGENTS.md 已要求主控先读 dispatch-log/USAGE.md 并使用记录器；CLAUDE.md 链接到同一文件。被委派者无需采集。绕过入口不会自动补录，不读私人会话日志。
- dispatch-log 由 dispatchlog/main 维护；本主控此前按用户要求写入 agents模版.md 并补其 HANDOFF，未纳入 Saddle 提交。其当前提交状态请在对应仓库核实。

## 约束

- 保留旧 t38-dispatch-study 分支／worktree（c15bc4d 调研文档未合入）；它仍可能影响 branches_merged，不为放行自动删除或合并。
- T37／T39／T40 已完成，不重复处理；T29 根因排查尚未实施。
- 不扩大范围、不读上游内部文件、不自动推进队列或启用循环；不干扰用户 agent，不改全局采集指令。
