# 交接

更新：2026-09-29。当前 main，T42 已审查、合并、清理并重新编译 release，待用户重启 Saddle 查看；不自动推进队列。

## 本轮 T42

- 实现 `3ef9e4e`，合并 `0c46cce`，主控审查 `6c6a101`，收尾 `96321e2`。普通 Tasks 入口优先匹配当前焦点 agent 所属已登记仓库，有任务则切换；无任务或无法读取时保留当前项目。取舍与范围见 DESIGN §57、`docs/任务/T42-焦点仓库Tasks.md`。
- 主控标准套件一次：282 passed／2 failed／3 ignored，T42 定向用例通过，clippy 与 diff 检查通过。两项失败分别为 `ctl_shell_creation_is_idempotent_preserves_focus_and_confirms_close` 的 fixture JSON EOF，以及 `t20_r1_pending_new_pane_keeps_known_source_cwd_for_shell` 缺关闭确认字段；各单独一次复跑通过。全套未全绿，不据此宣布 T29 根因解决。详细日志路径见任务书。
- 接受“打开后已有输入则不再自动换项目”和“只匹配已登记 Drover 项目”的保守取舍。实现者额外临时扰动 fixture 超出验证预算，已记录且不再追加检查。
- main release 编译通过（`/tmp/saddle-t42-release.log`，5.45 秒），`~/.local/bin/saddle` 原软链仍指向共享 release；未重启用户界面。
- 实现者 idle、attached=0，工作树干净并确认已合入后，删除其分支／worktree，并关闭 `saddle/dev-t42-focused-tasks-1`（工作目录已删，一并关闭）。旧 t38-dispatch-study 保留；其他用户 agent 未动。
- 回复与审查 note 已存同一 dispatch：`25d80505684e4cbf9a6f757097e21e6a`，project=saddle 主仓库，task=T42。本交接随后提交推送，以实际 Git 状态为准。

## 队列与下一步

只读核对 current=T42、awaiting=null；pending 顺序 T43、T29、T28、T32、T34，loop=false、gate=true、paused=false。用户重启查看 T42，之后自行放行；本轮未执行真实队列写操作。

T43 是用户刚要求新增的待办：主控委派新 agent 后左侧 Agents 意外全部收缩，需要按 z 恢复。已入队，未调查、未实现、未派发；不要自动开始。T29 仍仅记录排查方向。

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
