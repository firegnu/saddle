# 交接

更新：2026-09-29。当前 main，T47 文档任务已审查、合并、清理；本交接随最终提交一并推送。不自动推进队列，release 仍为已编译的 T45 版本。

## 本轮 T47

- 用户点击 Next 顺序派发后，公开 current=T47，才启动实现。README 的 Build and run 新增一句 `Use saddle --help to see the command-line options.`（实际命令使用行内代码），无代码变更。任务及审查见 `docs/任务/T47-README命令行帮助.md`。
- 类型样式／文案调整，轻档 sonnet/medium、看得见、不交叉审查。实现 29e5ad2，合并 24a9c54，审查 0ad6f4d，收尾 95c12e4。文档 diff 和新增句核对通过，未跑测试、clippy 或编译。
- t47-readme-help 分支和 worktree 已清理，工作目录删除后一并关闭自开的 `saddle/dev-t47-readme-help-1`；历史 t38-dispatch-study 保留。
- Dispatch `21125206edfc46428debf199d91915ae`，最终增量 note 在实际推送后保存。收尾记号后只读核对仍 current=T47/doing、awaiting=null、loop=false、gate=true。用户正在对比顺序派发的状态和通知，主控不调用 done/go/next，不改开关，等待用户观察。

## 本轮 T46

- 用户在 Tasks 选中派发后，公开 current=T46，才启动实现。README 的 Tasks 条目新增一段 Dispatch selected 用法及主要禁用条件，仅文档；任务及审查见 `docs/任务/T46-README选中派发说明.md`。
- 类型样式／文案调整，JEV 轻／看得见／不交叉审查，Claude sonnet/medium。实现 aa7b912，合并 cf76dfe，审查 10b6a46，收尾 6ab5ec8。文档核对及 diff 检查通过，没有运行测试、clippy 或编译。
- 分支/worktree t46-readme-selected 已清理，其中自开的 `saddle/dev-t46-readme-selected-1` 一并关闭。历史 t38-dispatch-study 保留，未干扰其他 agent。
- Dispatch `7c4b208f9a2a4e93a5945f98abb9daae` 保留路由、决定、任务书快照和回复；本轮最终 note 在实际推送成功后写，避免重复累计文档。用户自行检查／放行 T46，不推进下一项、不处理 T29。

## 本轮 T45

- Drover 指定派发接口交付后，用户重新明确放行且派发前只读确认 current=T45，才委派实现。任务书 `docs/任务/T45-派发选中任务.md`，设计 §59，审查 `docs/任务/T45-主控审查.md`，Dispatch `5574d31b97324b619d77afb3070b4026`。
- Tasks 的 Pending 操作增加 `Dispatch selected`，使用该项公开 pos/token 和项目，不先重排。旧接口缺字段或队列不允许派发时禁用；发送、记录与手动模式分别表达，失败不自动重发。实现 `e93b14d`，三处反馈修正 `4ee0e79`，合并 `ac89efc`，最终任务审查 `049fe49`，收尾 `802681a`。
- 主控首轮标准测试 290 passed / 1 failed / 3 ignored；失败为 T25 通用 PTY 退出控制序列断言，单独复跑一次通过；clippy 通过。返工后仅 7 项定向检查通过，未重复全套。保留全套非全绿的限制，不宣称已解决偶发测试或 T29。
- release 编译成功（5.72 秒，`/tmp/saddle-t45-release.log`），`~/.local/bin/saddle` 仍指向共享 release 可执行文件；未重启用户界面。
- 确认实现者 idle、attached=0、提交不变且工作树干净后合并；已清理 t45-dispatch-selected 分支/worktree，工作目录删除后一并关闭 `saddle/dev-t45-dispatch-selected-1`。保留历史 t38-dispatch-study，未动其他用户 agent。
- 审查 note 存同一 Dispatch。未改上游、未操作真实队列、未处理 T29。用户重启后查看按钮；只有选中 Pending 且未暂停、无 Current/Awaiting、目标可用时才可点击。真实派发仍由用户操作。

## 本轮 T44

- 用户正式送入 TASK T44 后，只读确认 current=T44，才恢复工作。此前误派发已停止，旧 Dispatch 按用户要求清除；本轮新记录 `9ddd04109c714f96ace9e4431f295fdf` 如实说明沿用保留实现 `a717a6d`。
- 类型为样式／文案调整，任务书含新版技能的类型、依据和简短提示。轻档 sonnet/medium、影响面看得见、不交叉审查。接手记录 `55a853f`，合并 `c560c85`，主控审查 `e809755`，收尾 `526776b`。任务书见 `docs/任务/T44-Dispatch返回文字.md`，设计 §56。
- Dispatch 全文页 `Back Esc` 恢复共享单行紧凑文字。实现者定向 workflow 显示检查通过，主控 diff 检查通过；按预算未跑全套或 clippy。release 编译成功（5.81 秒，`/tmp/saddle-t44-release.log`），原可执行软链可用；未重启用户界面。
- 工作树干净、已合入、实现者 idle 且 attached=0 后，清理 t44-dispatch-back-text 分支／worktree，关闭其中自开的 `saddle/dev-t44-dispatch-text-approved-1`。保留历史 t38-dispatch-study，其他用户 agent 未动。

## 前次 T43

- 核心实现 `45564a7`，滚动检查修正 `13a4ce2`，合并 `131b55c`，审查 `44ae599`，收尾 `bc9f79f`。首次成功列表确定默认折叠状态，之后新增／退出 agent 不再改变它；z 仍可手动切换。范围／设计见 `docs/任务/T43-Agents展开状态.md`、DESIGN §58。
- 主控首轮标准套件 283 passed／3 failed／3 ignored；clippy 通过。其中鼠标表单和待启动窗格 cwd 两项各单独一次复跑通过，不调查 T29。
- 直接相关的滚动失败已交原实现者修正：旧鼠标 y 命中头部分隔线，原自动折叠掩盖了测试无效。改为实际 agent 行后，主控定向滚动检查 1 passed（10.67 秒），确认到底、回顶、刷新保位、不 attach。产品滚动和布局未改。未重复全套，不将首轮改写为全绿。详细记录及日志见 `docs/任务/T43-主控审查.md`。
- main release 编译成功（5.43 秒，`/tmp/saddle-t43-release.log`），原 `~/.local/bin/saddle` 软链可用；未重启用户界面。
- 实现者 idle、attached=0，工作树干净且已合入后，删除 t43-agents-fold 分支／worktree，并关闭其中自开的 `saddle/dev-t43-agents-fold-1`。历史 t38-dispatch-study 保留，其他用户 agent 未动。
- 路由、派发、两轮回复、返工和审查 note 保存在同一 dispatch `29308f5afca7415dbb5a10cbd9edf606`。本交接随后提交推送，实际状态以 Git 为准。

## 队列与下一步

用户已自行放行 T46，再点击 Next 派发 T47；主控仅在公开 current=T47 后实施。T47 文档已收尾，当前队列现场留用户观察，未登记完成或放行。此前通知调查已暂停，尚未解释用户历史上看到的 Awaiting 与提示变化；不能把当前 loop off 快照当作历史原因。T29 仍仅记录排查方向，不开展调查或修复。

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
