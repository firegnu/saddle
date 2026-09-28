# 交接

## 当前工作：任务通知联合落地（2026-09-28，以本节为准）

saddle 已落地：固定实现 `409e34c`，合并 `fadce6f`，收尾空提交 `94f50be`。主控接受正常使用验收，与 Drover 固定 `2b20205` 的隔离主流程一次通过；Clippy 通过。标准测试 255 passed／1 failed／3 ignored，旧 T20 窗格用例超时，单项核对一次通过，不宣称整套全绿或无负载回归，T29 仍未解决。用户确认撤回持久去重阻塞，接受 saddle 会话去重／启动基线，边界见 DESIGN §48 和 `docs/任务/任务通知-主控审查.md`。

- saddle release 已构建，默认链接、共享 target 和仓库 target 三入口 SHA-256 相同：`f19b4d78848a7c1a119e2d4de044a99bbab130babc337d725b4ef9971bc1287a`；未重启当前 saddle、未修改用户渠道偏好。
- 实现者 `saddle/dev-task-notifications-1`（b012b3c82a77）关闭前 idle、attached=0、worktree 干净、分支已合并；worktree 和分支成功移除后已关闭。临时 baseline worktree 也已移除。
- 下一步：通知 Drover 主控联调通过，按既有联合实施授权协调其代码落地及旧引擎加载新版；等待其固定落地 SHA、部署状态和清理回报。saddle 发布不代表 Drover 常驻旧引擎已经更新。不得推进真实队列，不关闭用户主控 drover/main；其实现 agent 和 worktree 由它管理。
- Drover 已审查通过的固定交付 `2b20205eb3ae423aa5f3eb9cc4a289ca84fd8340`（实现 9b6d9c2），branch m35-notifications，worktree `../drover-worktrees/m35-notifications`。公开契约 `docs/通知JSON接口.md`；没有修改六字段、命令或身份编码。saddle 收紧去重只影响消费方内部，不要求 Drover 返工。
- T33 是已完成调研、待用户决定保留的研究分支／worktree，agent 已按用户指示关闭；不把本次功能落地记作 T33 队列完成。不动真实 done／go／next。新增 T34 沙箱与定时任务仅入队，不自动设计实施。
- 主仓库两份未提交的 T33 研究／咨询记录继续保留；下文为历史背景，不代表当前实施状态。

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
