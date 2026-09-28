# 交接

## 当前工作：任务通知实施（2026-09-28，以本节为准）

当前固定交付为 `409e34c9cebf5a0453ed3ca926ab4df69bba0f1e`，实现者 DONE、idle、attached=0，开发 worktree 干净；临时 baseline worktree 已移除。用户确认收紧审查：撤回持久去重阻塞，只按正常使用验收处理必须改项，停止统计式重跑。此前补齐要求 not_idle 未送达，不再发送。

主控审查接受交付，与 Drover `2b20205` 的隔离主流程一次通过；Clippy 通过。标准检查 255 passed／1 failed／3 ignored，旧 T20 窗格用例超时，单项核对一次通过；不宣称整套全绿或无负载回归，未重跑基线统计。进入联合落地。接受 saddle 会话去重与启动基线，边界与最终结论记录在 `docs/任务/任务通知-主控审查.md`。联调通过前不合并发布、清开发 worktree 或推进真实队列；不关闭 drover/main。

用户已授权「可以，按照你的计划开干吧」。按两项目串行计划推进：先委托 drover/main 派 Codex 做公开通知开关与稳定去重，审查通过后 saddle/main 再派 Claude Code 接 Settings 与内部提示，最后联调。设计见 DESIGN 第 48 节，共同接口与两份实施任务书见 `docs/任务/任务通知-*`。

- 当前阶段：Drover 已审查通过待集成，saddle 已派发 `saddle/dev-task-notifications-1`（instance `b012b3c82a77`，Claude Code `opus[1m] / high`，role=implementer）。分支 `task-notifications`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/task-notifications`，基线为派发文档提交 `3cb5cb3`；路由常规／改行为／不独立审查。任务 `docs/任务/任务通知-saddle实施.md`，收到完成提醒后主控审查并组织隔离联调。
- Drover 固定交付 `2b20205eb3ae423aa5f3eb9cc4a289ca84fd8340`，实现 `9b6d9c2`，末提交仅主控审查记录；m35-notifications worktree 干净。公开接口为该 worktree 的 `docs/通知JSON接口.md`，命令为该 worktree 的 `bin/drover`。其主控审查通过无必须改项，无独立审查 agent；开发 8 项 RED→GREEN、一次 CLI 回归，主控一次 CLI 回归和错误 JSON 定点检查通过。
- saddle/main 已核对固定 HEAD、末提交文档范围与差异检查，并在临时 HOME 用交付命令执行 status→off→off→on→status，确认六字段、revision 幂等和 status 无落盘。未重复上游测试套件、未调用安装主分支命令或真实 loop／通知。t0 六元组身份按公开 binary64 大端十六进制规范接入，不猜 JSON 十进制格式。
- Drover 主控已回报派发进展（尚未完成实现）：契约与 `40be7f0` 一致、暂无异议；实现者 `drover/dev-notifications-1`，Codex `gpt-6-astra / high`；路由三项拿不准后回退常规／改行为／不独立交叉审查，完成后由 Drover 主控审查。分支 `m35-notifications`，worktree `/Users/firegnu/Developer/personal_projs/drover-worktrees/m35-notifications`，任务 `docs/任务/m35-notifications 通知开关与稳定去重.md`，任务提交 `8bdb4a6`。对方已挂完成提醒自行接续审查；此处记录其回报，不把任务提交当作实现交付 SHA，不提前启动 saddle。
- 共同设计及三份实施／接口文档已本地提交 `40be7f0`，未推送。正式实施委托已通过 corral 发给 `drover/main`（instance `a781ee31bc67`），返回 confirmed；要求它先报实际派发信息，并在后续开发与审查完成后向 saddle/main 交付固定 SHA。当前 HANDOFF 与 T33 研究／咨询记录仍未提交，均为本会话文档，不覆盖。
- 两边在集成通过前保留分支／worktree／自开开发 agent，不先合并、发布或清理。Drover 主控保持运行，由其管理实现者及审查；不得关闭用户主控。
- 用户再次明确：drover/main 不是本主控委派开的，任务完成后也不得主动关闭。其实现者 `drover/dev-notifications-1` 由 Drover 主控管理；我只管理自己开的 saddle 实现者。
- T33 仍保留原调研边界与报告分支；原研究 worktree 保留是用户决定，不作为未收尾功能分支清理。调研 agent 已关闭。不用这次授权倒填 T33 为已实现，不用虚构收尾提交推进研究队列。
- 实施采用独立任务书协调，当前真实队列仍 current=T33，未调用 done／go／next；其他待办顺序不动。收到 Drover 回复先分清“已派发”与“开发及审查完成”，只有后者才能开始依赖的 saddle 实施。

## T33 当前进展（2026-09-28，以本节为准）

当前下一步：用户同意先写 Drover 最小改动说明并交其主控核实。已写 `docs/任务/T33-Drover通知最小改动说明.md`，通过 corral 将只读咨询送给现有 `drover/main`（instance `a781ee31bc67`，发送 confirmed）。只评估通知开关、去重及与任务执行隔离，不写文件、不派发实现、不改队列／配置／服务。收到回复后主控汇总待定事项给用户；本说明不是实施授权，不关闭 Drover 主控。

最新操作：按用户「先关掉刚才派出去的那个调研的agent」指示，已关闭 `saddle/dev-t33-notification-research-1`（instance `a8a85574980e`）；关闭前 idle、attached=0，关闭后公开 status 返回 not_found。报告、研究分支及 worktree 保留。下文的 agent 保留／idle 记录为关闭前状态。

用户确认「正式委派出去调研」后，T33 调研已完成并通过主控审查。调研者 `saddle/dev-t33-notification-research-1`（instance `a8a85574980e`，Codex gpt-6-astra / xhigh）当前 idle、attached=0，回复 DONE；最终提交 `bf895087fba40a5f91fffd43dd1bf8dfcca6796f`，研究 worktree 干净。

- 任务书及主控审查：`docs/任务/T33-系统通知收敛调研.md`。报告已在调研 worktree 的 `docs/任务/T33-系统通知调研报告.md` 产出，调研者完成记录在同目录任务文件中。
- 分支 `t33-notification-research`；worktree `../saddle-worktrees/t33-notification-research`；基线 `ce12adb`。主仓库任务书及本交接为派发记录，尚未提交。
- 提交只含指定两份调研文档，差异检查通过；主控抽查关键发送链与触发条件，未发现阻塞项，未跑测试套件。现有通知分散于 Drover、Hammerspoon 和 agent／终端渠道，Attention 不能完整替代；实际 OS 投递、客户端回调内部作用等仍未验证。详细方案和依据见报告。
- 等用户决定覆盖范围、saddle 不可用时策略及允许改动范围。没有实施、合并、推送或发布；保留研究分支、worktree 和调研 agent，不自动进行功能任务收尾。
- 实时公开队列 current=T33(doing)、awaiting=null、loop=false、gate=true；未推进队列。正文末尾的“不立即启动调研或委派”已由上述最新确认取代，仅调研的边界不变。
- 原有其他项目主控均保留。下文为 T30 已完成的历史交接，不代表当前队列状态。

### T33 后续本机清理（2026-09-28）

用户决定 Hammerspoon 提醒原样保留，并授权主控自行检查／清理 CCNotify。发现 Stop／UserPromptSubmit 仍在记录数据后，用户明确选择停用记录、移除 CCNotify、保留数据备份。现已精确移除这两个全局 hook，归档运行安装及旧源码副本到 `~/.local/state/ccnotify-removal/20260928-171636/`，数据库、日志及原配置均保留；其他 hook 未变，Hammerspoon 哈希核对不变。详细记录见主仓库 T33 任务文件末尾。研究报告是清理前快照，尚未修改。

下一步与用户讨论 Drover 的系统通知向 saddle 迁移；这部分未实施。T33 研究 agent／worktree 继续保留，未推进队列、合并或发布。

更新：2026-09-28。T30 设计与理由见 `docs/DESIGN.md` 第 47 节；实施、审查和发布证据见 `docs/任务/T30-Settings配置入口.md`、`T30-主控审查.md`、`T30-独立审查.md`。

## 当前状态

T30 Settings 配置入口已通过主控及独立复核，合并并本机构建发布。实现 `85ead03`，合并 `5b88932`，收尾空提交 `27e18d2`。本交接随最终文档提交推送 origin/main；后续以实时 Git 状态为准。

- Settings 固定在 Agents 顶部第二行右侧，与 Attention 同行，窄栏另起一行；点击或在 Agents 按逗号打开，关闭回原焦点。
- General／Colors／Advanced 编辑现有 config.toml；提供草稿、单项恢复默认、颜色局部预览、Save／Cancel。颜色及侧栏宽度保存后立即生效，其余项标 Restart required，下次启动生效。
- 保留注释和未编辑内容，外部修改在 Save 时检测，冲突提供 Keep／Discard／Back；失败保留草稿。Keep 重载失败后重试仍保留草稿，无法解析目标的符号链接拒绝保存而不覆盖链接。
- 局部快捷键 F1–F3 切页、Tab／↑↓ 选字段、Ctrl-U 清空、Ctrl-D 默认、Ctrl-S 保存、Esc 取消。设置打开时新 ctl 修改请求返回 busy，Inspect／Request 等查询正常。
- 只改 saddle；未改 corral／drover／corral-dispatch 或全局技能。未重启用户当前 saddle，下次启动使用新版本。

## 验证与发布

- 主控首轮标准测试 242 passed／0 failed／2 ignored，Clippy、fmt、diff 通过。返工主控 Settings 12 项和 app 1 项通过，受影响 Clippy、fmt、diff 通过；独立复核 Settings 12 项通过。未重复无关全套，不宣称 T29 偶发问题已修复。
- 首轮独立审查 R1 草稿丢失、R2 悬空链接被覆盖均修复并关闭；最终必须改 0，原非阻塞建议 S1 1，新增相关问题 0。首轮取舍全部认可。
- 合并后 src、tests、Cargo.toml、Cargo.lock 与审查提交完全一致。
- main 共享 target `cargo build --release` 通过。共享 `../saddle-worktrees/.target/release/saddle`、仓库 `target/release/saddle`、默认 `~/.local/bin/saddle` 三入口 SHA-256 一致：`61a169b9fb6db2faab8e35d999777c9053d6c2e2f8822bb8151d6edb7851bd7c`。默认入口仍链接共享 release，`--help` 已核对 Settings。
- 测试只用临时配置、合成文件／链接、假 CLI 及隔离状态和 runtime；未访问用户真实配置、布局、agent 或 saddle socket。未验证断电、ACL／owner／xattr、网络文件系统及最终替换时同步竞写，不作额外保证。

## 队列与开发环境

- `drover done T30` 核对通过，退出 8 等用户放行。公开状态 current=null、awaiting=T30(done)，loop=false、gate=true；没有调用 go／next。
- Pending 顺序：T31 评估并行派发多个不同任务 → T29 偶发测试失败 → T28 ctl 上限 → T32 统一接入本地与远程 corral agents。后续需求待逐项讨论，不自动设计或派发。
- T30 队列正文保留派发时旧占位稿；手动下放后用户已明确确认按最新任务书正式实施，设计和完成记录以任务书与 DESIGN 为准，不改上游历史。
- T30 实现与独立审查 worktree、实现分支均已清理。删除前确认两 agent idle、attached=0、工作区干净且提交已合入 main，随工作目录删除一并关闭：`saddle/dev-t30-settings-1`（acef3e8b5939）、`saddle/dev-t30-review-1`（3bbf22741dd6）。迟到提醒查到 not_found 即忽略。
- saddle 仅保留主控 `saddle/main`，cwd 主仓库；原有 corral/main、drover/main、globalmesh/main、owlet/main 保留在各自目录。

## 仍需注意与下一步

等用户体验 Settings 并放行，再讨论下一项。

- T30 S1：长配置路径可能把单行保存错误的实际原因挤掉，保存失败本身仍可见且保留草稿。本轮记录为非阻塞建议，已告知用户，尚未另立任务，不自动扩大修复。
- T30 实现者曾回复 DONE、输出回到输入提示符而 corral 仍显示 working。主控按用户指示对固定提交继续审查；之后公开状态恢复 idle，清理前再次确认。原因未诊断，不据此宣称上游已修复。
- T29 picker／close confirmation 偶发 workflow 问题未定位；一次通过不代表修复。
- 共用 target 跨 checkout 曾复用旧二进制，检查须核实构建对应当前源码。
- T28 ctl 单实例 256 次修改上限仍在；T24 系统剪贴板真实 Copy 写入仍无明确现场验证反馈。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。
