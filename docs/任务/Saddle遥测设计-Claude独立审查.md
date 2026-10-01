# Saddle 遥测设计：Claude Code 独立审查

日期：2026-10-01。你是本任务的独立设计审查者，不是实现者或主控。不再委派其他 agent。

## 用户原话与本次授权

用户本次要求：

> 你把前因后果都和他交代清楚以及你定下来的的方案。开一个claude code去审查吧。你要把相关的信息都交代清楚。你们达成一致了你来把最终结论汇报给我

范围：只读设计和必要源码，提出具体问题，与 Saddle 主控讨论并复审。不得实现、改文件、提交、安装、派发开发任务、操作真实队列或用户 agent。任务验收原话即上段，不另加用户验收条件。下文是帮助审查的上下文和主控检查建议。

## 为什么走到现在

1. 原来 Corral、corral-dispatch、Drover、dispatch-log 是独立的东西。dispatch-log 的目的不是一般运行日志，而是保存每次任务派发和流转的数据，将来评价路由与 corral-dispatch 的准确性。
2. 用户先讨论将 dispatch-log、corral-dispatch 作为 Saddle 官方 core plugins 统一交付；随后明确不要求机械移植，而是满足实际的数据需求。
3. 曾讨论把 dispatch-log 融入 Drover。用户不希望一个任务插件包装最底层的 Corral，并且 corral-dispatch 不是只为 Drover 服务，因此最终把遥测放在 Saddle 核心。
4. 用户最初不想记录口头临时开 agent 的活动，后来确认可以显式选择记录这些活动。普通临时操作默认不采集，任务可用 Drover 默认选择，单次选择可覆盖默认。
5. 用户明确还需要独立的遥测总开关。总开关关闭优先，业务继续，历史保留，重开不自动补录关闭期间。
6. 用户选 SQLite。当前方案为 SQLite 保存结构化关联/事件，内容哈希文件保存完整正文。不是服务器/采集 daemon；headless 记录/查询无需 TUI 或 Drover。
7. 用户强调任务书是主控真正交给实现者的文件，不是任务描述或路由摘要；希望事后检查主控是否扩大需求。因此要保存原始需求依据、补充授权、任务书各版本、实际 prompt 和 send、JEV 请求/完整解析结果/整理建议、主控选择/预算/理由、返回、审查、返工及收尾。
8. 用户明确不必兼容以前 dispatch-log 的记录。无需导入旧 ID、格式或历史，不双写。旧程序/数据暂时保持；新能力验证及消费者切换完成后再退役。
9. 用户确认先由 Saddle 在上层薄适配公开 Corral CLI，后面 Corral 迁入 Saddle 后替换为内部调用。现在不改 Corral 仓库。
10. 最后用户明确说：**“corral不能依赖哦”**。Corral 核心绝不能依赖遥测；Saddle 上层分别调用业务和记录。将来 agent-core 也不接收 trace、记录器、sink 或遥测回调。

## 不能误认的当前状态

- Drover 已在 2026-09-30 完整替换成 Saddle 中的 `plugins/drover/`，拥有任务、数据和通知；旧独立 CLI/watch 退役。
- 主控本次刚现场核实：GitHub `firegnu/drover` 的 isArchived=true；launchctl 无 Drover 服务，旧 CLI 和已知服务 plist 不在；目前有 saddle-drover 插件进程。不要再讨论恢复旧 Drover 服务或重新移植 Drover。
- Drover 与第三方插件消费 Saddle 通用遥测接口，宿主不导入 Drover 的业务代码，不读其业务文件，不依据具体插件名称分支。
- corral-dispatch 可选接入遥测，遥测不能成为路由/派发的运行前提。Corral 和 corral-dispatch 是不同职责，不能混淆。
- 当前新遥测没有实现。旧 dispatch-log 还在使用；项目当前记录要求仍有效，由主控负责，审查者不用采集。

## 当前方案：请独立检查，不必赞同

主控已经写下以下设计，都是待实现的设计而非运行事实：

- 身份分 trace / dispatch / operation / event。任务绑定由插件显式传 kind/scope/key/run，不按 agent 名、目录或时间猜关联。
- 八张逻辑表：traces、dispatches、operations、events、blobs、event_blobs、event_links、recording_policy。字段细节见文档。
- 记录为追加，不可后改正文引用和依据链接；同 ID 同内容幂等，不同内容冲突。显式复用旧需求使用 evidence.reused/carried_from，保留原来源。
- 路由/派发/返工各次现场保存文件版本。任务书快照、operation、begin、引用在同一数据库事务提交；end 另存。缺 begin/end 不推断业务成败。
- 正文先暂存/fsync，取得写锁复核开关版本后发布并入库。正文和 SQLite 无共同事务，允许孤立正文，不允许确认未可靠发布的引用。
- 开关 generation 撤销旧在途捕获；重复设置同值不改变版本。关闭成功作为发布/提交屏障，关闭后重新开启也不能接受旧操作 end。关闭前开始的暂存不承诺安全擦除。
- 业务调用只执行一次。遥测失败不触发 Corral/JEV 重放；已确认参数/关联错误在业务调用前拒绝，存储不可用时跳过采集继续业务。
- 纯记录命令返回记录状态；执行入口保留业务 stdout/退出语义，通过独立 stderr 回执报告遥测情况。
- Drover 默认选择记录后生成 trace/controller_handoff，在它原本的主控任务交付消息中附带显式上下文。Corral 只传普通消息字节，不解析 ID；采集器不偷偷修改 prompt。之后主控每次调用显式传上下文。
- Drover 先交付主控、后持久化 task/run 状态，随后提交 task.transition；没有跨库原子保证，trace 存在不表示任务已登记或已获放行。
- 查询保留未知、缺口和来源。保存 reply 不等于任务完成；instance 一致不证明回复因果；文件快照不证明 agent 读过；没有返工记录不代表一次成功。首期没有自动路由评分、费用或完整终端流采集。
- 插件定位宿主二进制拟由 SADDLE_HOST_BIN 提供；记录上下文不进环境。接口文档里 CLI 名称和退出码尚是实施提案。

## 两项仍是产品建议，不能替用户拍板

1. 原始用户消息目前不能从调查过的 Corral 公共接口直接取得。主控建议逐字提交并标 controller_transcription，不读私人会话日志、不要求用户另写文件。用户曾说没理解这项问题，主控已解释，但没有得到明确二选一回答。因此不是已获批准的独立真实性保证。检查这种来源限制是否已被准确表达，是否还存在误导。
2. 首次安装遥测默认关闭是主控建议；总开关存在和优先级是用户已经确认的要求。不要把默认值称为用户决定。

## 阅读材料与源码定位

审查工作目录是 detached worktree，源代码基于主仓库当前 HEAD；四份设计文档和本任务书已由主控复制当前未提交内容，故应审查工作区文件，不是 git show HEAD 中旧内容。不要修改这些快照。主控后续修改通过 corral 告知并更新快照，不能自行追最新文件导致审查对象漂移。

按顺序阅读：

1. `AGENTS.md`；本任务只读授权优先，不派发、不修改任何文件。
2. `docs/任务遥测接口契约.md`（主要评审对象，含明确 Corral 零遥测依赖）。
3. `docs/调研/Saddle任务遥测数据模型与SQLite存储-2026-10-01.md`（来源/场景/故障/存储及试验证据）。
4. `docs/DESIGN.md` 最后“核心遥测”段与 2026-09-30 Drover 完整替换段。其他历史方案已被后续决定替代，勿以早期描述推翻当前授权。
5. `docs/调研/Saddle统一仓库与运行入口-2026-10-01.md` 的统一产品与 Corral 最后迁移方向。
6. 必要源码：`plugins/drover/src/core.rs`（DispatchPending，先 send 后 append）；`plugins/drover/src/dispatch.rs`、`plugins/drover/README.md`；`src/main.rs`、`src/corral.rs`、`src/command.rs`；`src/plugins/runtime.rs`、`docs/插件协议.md`。

可只读对照兄弟仓库的源码（不要运行其业务命令或读取真实日志）：`/Users/firegnu/Developer/personal_projs/dispatch-log/dispatch_log/{store,record,corral,route}.py`、`/Users/firegnu/Developer/personal_projs/corral/docs/CONTRACT.md`。不要检查用户私人会话、凭据、真实任务正文或 Corral 状态目录。

## 已有验证与限制

此前 Python/SQLite 临时合成试验有 11 项通过，仅覆盖原六表约束子集、并发写、幂等、WAL 读快照和进程退出后的孤立正文。没有验证新加的证据关联、开关屏障或任何 Rust/插件集成。不能引用它证明整个新设计已经通过测试。本轮不要 cargo build/test，不启动真实或假 agent，不运行原型，不联网请求 JEV；用源码和具体时序反例进行设计审查。

## 希望你检查的风险（不是额外用户验收项）

- Corral 是否有隐藏的反向依赖；第三方插件接入是否仍被 Drover 特例绑定。
- 开关屏障、在途结果、记录重交、正文发布和失败路径是否存在自相矛盾。
- 是否足以按真实版本/时序分析任务书扩大、JEV 输入偏差、主控选择和返工；哪些无法观测必须保持未知。
- 显式关联怎样传递和恢复，是否有丢链、错误归属、权限或进程语义问题。
- 字段/接口是否已足够进入实现，哪些设计可以删掉或简化。
- 发现缺口请给出具体触发场景、文档行号、必要最小改法；不把未实现功能当 bug，不引入未授权强制任务引擎/后台补采/通用事件总线。

## 回答与讨论方式

先独立给出结论（可进入实施 / 修改后可进入 / 存在阻断），再按严重性列问题，区分必须修改、建议、已接受限制。每项给证据及反例，最后列你认同的边界、仍需用户决定的内容。不要迎合主控或以“达成一致”为由忽略真实分歧。

不写文件，直接在会话中回复完整审查结果。主控会通过 corral 继续讨论，你复核修订后明确哪些问题关闭、是否还有阻断。技术一致不等于替用户决定上述产品偏好。

命令都在前台跑完，全部做完后，回复最后一行写 DONE。
