# Saddle 任务遥测设计草案与 SQLite 存储调查

日期：2026-10-01。状态：调查与设计建议，尚未实施。本文依据本轮用户确认的需求，研究内置遥测的数据模型、采集位置和存储方案。调查使用源码、公开说明、SQLite 官方文档及临时合成数据，没有读取真实记录目录、队列正文或私人会话，没有调用真实 agent 或 JEV。

结论：采用 Saddle 内置记录模块、SQLite 保存关联与事件、内容哈希文件保存原文。区分链路、派发、操作、事件四种身份；将需求依据、任务书版本和实际执行关联起来，供以后分析。首期不引入专门的采集 daemon、数据库服务器或自动路由评分。下文的表字段、命令名称和原话录入方式是待评审草案，不代表已经实现或全部得到用户确认。

> 后续确认：用户已选择主控逐字提交原话、首次默认关闭、允许有标记的手工晚交。下文调研过程中的“建议/待确认”保留为历史语境，实施以 [接口契约](../任务遥测接口契约.md) 和 DESIGN.md 最新决定为准。

## 本轮已经确定的方向

- 遥测采集、存储、查询接口归 Saddle。Drover 提供任务记录开关和任务相关展示，调用 Saddle 接口，不拥有遥测接口或数据目录。
- Drover 不为记录代理 Corral。Saddle 执行入口只在显式携带记录标识时采集；corral-dispatch 保持独立可用。
- 普通口头临时操作默认不采集；用户明确要求记录时，可以建立没有 Drover 任务关联的独立链路。
- 记录覆盖现有 dispatch-log 的任务书、路由、主控决定、派发、回复、审查及返工材料，并保留原始需求、补充授权、完整任务书及修订关系，供事后比较是否扩大范围。
- Saddle 有独立的遥测总开关；关闭时停止新增采集，历史仍可查询。Drover 的默认选择和单次口头选择都不能越过总开关。
- 存储采用 SQLite 加内容哈希正文文件。不兼容、不导入旧 dispatch-log 数据，不保留旧 ID 或事件格式的约束。
- 当前先在 Saddle 提供对 Corral 公开 CLI 的薄适配；将来 Corral 迁入后换成内部调用。插件不承担遥测包装；当前并非已经消除了所有外部适配。
- Corral 核心始终不依赖遥测，由 Saddle 上层分别调用业务与记录模块；记录上下文和 recorder 回调都不进入 Corral。corral-dispatch 可选接入遥测，不以遥测启用为运行条件。现有 Drover 插件和第三方插件都通过公开接口接入，Saddle 不反向依赖具体插件。

这替代此前“dispatch-log 作为独立 core plugin”的归属选择。Corral 最后迁移、继续通过其公开接口访问的约束不变。已确认边界同步到 [DESIGN.md](../DESIGN.md)。本轮设计自审后收敛的字段、命令、交付顺序与故障回执见 [接口契约 v1](../任务遥测接口契约.md)；本文保留设计理由和调查证据，两份文档都不代表已经实现。

## 当前实现中影响设计的事实

源码快照：Saddle `5ddd544`；dispatch-log `9aa780c`。

| 事实 | 设计影响 | 依据 |
|---|---|---|
| dispatch-log 用 project、task、dispatch_id、parent 关联；没有 Drover run_id | 新设计须显式携带执行轮次，而非照搬旧关联 | `../dispatch-log/dispatch_log/store.py` |
| 一个 Drover 任务退回后再次派发会有新 run_id，并保留 previous_runs | 一个任务号可以对应多轮链路，不能只用 T42 作主键 | `plugins/drover/src/core.rs` 的 fold |
| JEV 输出分档、交叉审查、影响面；模型家族由主控选 | 路由模型、主控选择的执行模型、实际显式参数必须分开 | 已安装 `corral-dispatch/route.py` 与 SKILL.md |
| 旧路由记录保存整理前的解析 JSON，但不是 HTTP 原始字节 | 新设计不能把它称作原始网络响应，也不能用整理建议替代 | `../dispatch-log/dispatch_log/route.py` |
| start/send 包装在执行前保存任务书；reply 只保存当次公开返回 | 事后查询不能替代现场采集；文件快照不证明 agent 已读 | `../dispatch-log/dispatch_log/corral.py` |
| 主控选择和审查标为 controller_statement；reply 关联标为 not_proven | 存储不能把判断升级成工具事实，也不能凭 instance 相同证明回复因果 | 同上及 `record.py` |
| 审查与收尾主要是自由文本，没有统一 verdict | 原文供回看；新设计需区分审查结论、agent 自报和任务状态 | `cli.py` 的 note 与 Drover 的事件展示 |
| Drover Dispatch 视图逐次调用外部 ls/show/cat，按 operation_id 合并 intent/result 展示 | 可复用展示含义，需要替换查询入口；不能只替换二进制路径 | `plugins/drover/src/dispatch.rs` |
| 现有插件 command.v1 需要运行中的 TUI，参数与结果各 48 KiB，ctl 回执上限 256 | 不宜直接承载完整正文和长期遥测事件流 | `docs/插件协议.md` 第 12 节、`src/control.rs` |

## 用一条完整链路定义身份

合成例子：项目 A 的 T42 第一次执行为 run-a。主控路由后派发，实现者回复，独立审查要求修改，原实现者返工，审查通过，主控收尾。

```text
项目 A / T42 / run-a  ← Drover 提供的可选关联
└─ trace-a            ← 本轮记录链路
   ├─ dispatch-a      ← 首次派发
   │  ├─ route-1      ← 一次路由调用：begin / end
   │  ├─ decision-1   ← 主控实际选择
   │  ├─ start-1      ← 一次 agent start：begin / end
   │  ├─ reply-1      ← 一次 agent reply：begin / end
   │  ├─ send-1       ← 给原实现者返工：begin / end
   │  └─ reply-2      ← 返工后的回复
   ├─ dispatch-b      ← 独立审查，parent = dispatch-a
   │  └─ 路由、决定、启动、回复等自己的事件
   └─ 主控审查与收尾原文
```

同一个实现者返工，沿用 dispatch-a，但每次 send/reply 都有新的 operation_id。换人或升档重派建立新的 dispatch，parent 指向前次派发；独立审查建立 kind=review 的 dispatch。同一任务存在多份子任务书时，也保留不同 dispatch，不强迫“一任务等于一个 agent”。

T42 退回 Pending 后再次派发，run-b 建立新的 trace，通过同一个外部任务关联查到两轮。口头操作显式开启记录时，也可以建立 trace，但没有 Drover 绑定。不根据 project、cwd、agent 名字或时间接近自动合并链路。

| 身份 | 表示什么 | 不能替代什么 |
|---|---|---|
| trace_id | 一次显式开启的记录链路 | 不等于任务号或 agent 会话 |
| dispatch_id | 一次委派，包括其路由、决定和后续跟进 | 不等于 Corral start；start 之前已有路由 |
| operation_id | 一次 route/start/send/reply 调用 | 不等于平台 turn ID，也不是重执行许可 |
| event_id | 一条不可变事实或陈述 | 不等于命令回执 ID |
| agent name + instance | 公开 Corral 实例身份 | 不证明回复属于哪次 send |

## 原始需求、授权与实际任务书

“任务书”指主控实际交给实现者或审查者的完整文件，不是 Drover 的任务描述，也不是送给 JEV 的摘要。两种材料各有用途，不能互相替代。

首期建议由主控逐字提交相关用户原话，保存确切提交字节，并标为 `controller_transcription`。目前所调查的 Corral 公开接口没有提供主控聊天窗口中的用户原文；不读取私人会话文件来绕过这个边界，也不要求用户每次另写原文文件。此录入方式仍是建议。它支持对比，但不能独立证明主控没有漏抄、改写或漏报授权。

每份需求证据保存：声明的说话者、实际提交者、取得方式、原文正文引用、提交时间、可选的原消息标识和声明时间。未知消息标识留空。主控的理解另存为 `controller.summary`，不能填进“用户原话”。用户主动提供的文件可标为 `user_supplied_file`，哈希只证明所保存的字节，不证明作者身份。

简短的“可以”“继续”必须附带它回应的完整提议及明确引用；单独保存这两个字无法解释授权范围。上下文不足就标为不完整。这里的“授权依据”只是主控提交的证据关联，不是系统鉴定的许可，也不改变原来的任务放行规则。

用合成实例核对完整链路：

| 次序 | 保存材料与关联 | 以后能核对什么 |
|---|---|---|
| 1 | U1：用户原话“只调整登录按钮的文字，不改登录逻辑”；保留转录来源 | 本次记录中的原始范围 |
| 2 | P1：主控提议的全文；U2：“可以”，显式回应 P1 | 简短同意究竟指向哪项提议 |
| 3 | B1：实际实现任务书的完整快照，`basis=[U1,P1,U2]` | 主控如何把需求写成任务书 |
| 4 | R1：实际 JEV 请求、完整解析响应和整理结果；引用 B1 及摘要 S1 | 路由判断用了哪些内容、摘要是否失真 |
| 5 | D1：主控选型、强度、验证预算、理由，引用 R1 | 建议与实际决策有什么差异 |
| 6 | A1：start 前再次读取真实任务书为 B2；保存实际 prompt、显式参数 | B2 是否已经偏离 B1，实际交出去的文字是什么 |
| 7 | Q1：公开 reply 的原文与实例；V1：审查原文及可选结构化结论 | 返回内容和审查者判断分别是什么 |
| 8 | B3：返工任务书新快照，引用 B2、V1；S2：实际 send 片段及返回 | 返工新增了什么、依据是否在此前已记录 |
| 9 | V2：复审；C1：主控收尾；T1：Drover 已提交的任务状态 | agent 自报、审查、主控收尾和任务接受是否一致 |

每次 start/send 在调用前保存当时文件字节；不能只记路径，也不能只引用最初 B1。路由用 B1、派发用 B2 时，即使二者不同也如实保留，不自动重路由或修改任务书。文件在快照后仍可能变化；快照证明调用前观察到的内容，不能证明 agent 实际读取了该版本。

任务书正文不做截断；文件不可读或超过明确的资源限制时返回记录缺口，不能把截断文本称为完整快照。实际 prompt、返工文字和 `merged_with_draft` 情况另存，不能用任务书替代实际发送内容。公开接口未返回合并后全文时，该全文保持未知。

原文、任务书修订和授权补充都追加新事件，用显式引用关联，不覆盖旧事件。执行 begin 固定当时已知的依据和版本；后来补录的授权不能倒填到原 begin 中。晚交材料同时保留入库时间和声明发生时间，分析时区分“当时已记录”和“事后补交”。未记录的授权不等于未经授权。

## 建议的数据表

以下是逻辑字段草案，不是已经批准的完整 DDL。数据库迁移版本与事件 payload 版本分开管理。

| 表 | 主要字段 | 作用 |
|---|---|---|
| traces | trace_id、origin、label、created_at、capture_enabled、generation、binding_kind、binding_scope、binding_key、binding_run | 链路及可选外部关联；Drover 可传项目根、任务号、run_id，Saddle 将其视为来源声明 |
| dispatches | dispatch_id、trace_id、parent_dispatch_id、kind、created_at、metadata_json | 保留首次派发、重派、独立审查关系 |
| operations | operation_id、trace_id、dispatch_id、kind、producer | 定义一次外部调用的身份；不另存一份容易失真的业务状态 |
| events | seq、event_id、trace_id、dispatch_id、operation_id、kind、phase、producer、evidence_kind、observed_at、recorded_at、payload_version、payload_json、fingerprint | 追加事件；seq 用于本数据库稳定翻页，fingerprint 用于检测同 ID 不同内容 |
| blobs | sha256、bytes | 已可靠保存的正文；路径由哈希推导，不以用户传入的任意路径作为存储位置 |
| event_blobs | event_id、role、sha256 | 引用完整保存的需求、任务书、prompt、请求、响应或回复正文；捕获失败在事件中明确描述，不伪造空正文 |
| event_links | event_id、relation、target_event_id | 关联依据、回应对象、前一版任务书和修订原因；目标必须已存在，通常同 trace；仅显式 evidence.reused 的 carried_from 可跨 trace 引用旧证据 |
| recording_policy | 单行：enabled、generation、updated_at | 全局采集开关和版本，用于拒绝关闭前遗留的在途写入 |

`events.dispatch_id`、`operation_id` 可以为空，允许链路级需求、笔记和开关事件。原生操作的 begin/end 必须绑定已知 operation、dispatch、trace。仅全局开关的最小控制事件允许没有 trace；不能借此保存无关联的任务内容。正文引用必须完整。

外部任务绑定由 Drover 提供；Saddle 不为验证绑定去读 queue.md、tasks.state 或 Drover 的内部配置。建议同一个已知 `(来源, 项目, 任务号, run_id)` 对应一条 trace；未见运行登记声明/未核验的尝试单列，不只按任务号合并，失败后试分配的任务号可能复用。插件未启用时允许无任务绑定的 trace。项目移动时保留原始 scope，不自动重写历史路径。

主控选择的 model/effort、JEV verdict、送达信息等先作为经过校验的结构化 payload 保存。用 SQL view 暴露分析列，避免一开始就把所有可选指标复制成顶层字段。需要高频过滤的字段再依据真实查询建立表达式索引；原始正文不因提取字段而丢弃。

首期基础索引建议为：外部绑定、dispatch 的 trace/parent、事件 `(trace_id, seq)`、`(dispatch_id, seq)`、`(kind, observed_at)`，以及 operation 阶段唯一索引。不增加全文检索、向量检索或自动内容分类。

## 事件清单与采集位置

拟议命名如下；旧事件仅供功能核对，不实现导入映射。

| 新事件 | 当前等价来源 | 必须保留的数据 | 采集者 |
|---|---|---|---|
| requirement.recorded / proposal.recorded / authorization.recorded | 新增需求与授权依据 | 逐字提交的正文、取得方式、声明说话者、实际提交者、回应及依据引用 | 主控提交；来源明确标为转录或提供文件 |
| evidence.reused | 新增显式证据复用 | 原正文和来源不变、carried_from、当前复用声明 | 主控/插件显式请求，宿主校验复制 |
| brief.snapshot / controller.summary | 扩展任务书快照及路由摘要 | 完整任务书或主控摘要、版本关系、原路径、依据和采集时间 | 快照由执行入口取得；摘要由主控提交 |
| route.begin | jev.intent | 摘要、实际请求、路由实现版本或摘要、路由模型 | 路由执行入口 |
| route.end | jev.route | 完整解析响应、整理建议、退出结果、缺失原因、与 begin 相同的 operation_id | 路由执行入口 |
| controller.decision | controller.decide | 计划使用的模型/强度、验证预算、选择理由 | 主控显式提交 |
| agent.start.begin/end | corral.intent + corral.start | cwd、显式模型参数、标签、prompt、执行前任务书快照；返回 name/instance、退出码 | Saddle agent 执行入口 |
| agent.send.begin/end | corral.intent + corral.send | 发送片段、initial/followup/rework、可选修订任务书；pending、confirmed、merged_with_draft、退出码 | 同上 |
| agent.reply.begin/end | corral.intent + corral.reply | 请求目标、公开返回的 name/instance/at/text、instance 检查及归属声明 | 同上 |
| controller.note | controller.note | review/rework/decision 类型和完整原文；收尾内容仍可放在原文中 | 主控显式提交 |
| recording.changed | 新增开关配套事件 | 开启/关闭、时间和显式操作来源 | Saddle 记录模块 |
| review.recorded / task.transition | 新增可分析结果 | 审查原文及可选 verdict；Drover 已持久化的前后状态、run_id | 主控陈述／插件提交，分别标识 |

派发创建保存在 dispatches，不需要再制造一条含义相同的事件。JEV 原本的网络重试仍属于同一次路由 operation，记录器不额外重放；首期不新增每次 HTTP 尝试的追踪。agent 执行 token、费用、完整终端流不属于现有采集范围，不能从路由 usage 或回复内容冒充取得。

`producer` 是提交者的未认证声明；`evidence_kind` 区分执行入口直接观察、声明的主控陈述和声明的插件提交。公开 append 不能认证主控与插件的身份，task.transition 须由消费者通过插件公开接口核验业务状态。通用写入入口不能把调用方声明升格为执行入口直接观察；后者由实际采集路径生成。仍不承诺抵御本机数据库所有者修改数据，不将正文哈希称为防篡改审计。

路由记录还保存 Saddle、路由实现、规则/skill 的可得版本或内容哈希；未知就明示未知。现有固定脚本 hash 白名单不沿用为新模型限制。请求正文按实际送给 JEV 的内容保存，不含认证 header；完整解析响应与整理建议分别保存。不把解析 JSON 称为 HTTP 原始字节。

JEV 的 model 是路由模型，不能当成执行者 model。planned_model、命令显式参数和 agent labels 分开；未显式提供的参数为未知，不从全局设置或标签推测实际服务端采用值。分类建议中 verdict=null 表示拿不准，不能拿最高概率的 level 替代。

## 最终结果与分析标注的缺口

现有记录可以保存“审查通过”的原文，但没有固定字段能可靠表示它。一次 corral reply 成功不等于任务验收通过。新设计仍允许分析字段为空，不能从自由文本自动补成确定结论。

建议主控提交审查时可附 `verdict=passed/changes_requested/inconclusive`；Drover 在自己的状态变更已经持久化后，提交带未认证来源声明的 `task.transition` 事件。这能区分 agent 自报完成、主控审查通过和用户接受。具体字段属于实施草案，不另增用户验收要求。

即使增加 task.transition，Drover 状态写入与遥测数据库也没有共同事务。首期应明确缺口，不能承诺必达；如果以后要求必达，需要独立设计可靠投递与去重，不能顺带加入后台补采服务。

当前 Drover 派发先发送给主控，再持久化任务状态。接入时可在发送前用预生成的 run_id 申请 trace，并将绑定标为插件声明；实际任务状态仍以随后提交的 task.transition 为准。trace 存在不表示任务派发成功。初始交付若需保存执行证据，由 Drover 调用公开 Saddle agent 入口并显式携带关联；这是任务操作消费宿主能力，Drover 不解析或包装 Corral 以采集遥测。记录失败不触发重发，任务状态写入失败也不能由遥测自动修复。

“路由正确率”不能直接由现有字段产生。路由建议被推翻、发生升档、出现返工都是可分析现象，不自动证明路由错误。以后明确评价标准时，将分析结论作为带规则版本的派生结果，保持原始事件不变。

## 时间顺序与重复提交

`observed_at` 是采集方记录的时间；`recorded_at` 是入库时间。Corral 返回的 at 原样保留，不能替代前两者。时间使用统一 UTC 表示；声明的用户消息时间另存，不伪称是宿主观测时间。

数据库 seq 表示本地接收顺序，适合游标翻页，不声称是实际因果顺序。因果关联依靠 operation_id、dispatch parent 和显式引用。两个来源的时钟和迟到提交，不能用排序消除不确定性。外部调用耗时可由同一调用方的单调时钟记录，但 start 命令耗时不是 agent 完成任务耗时；跨 reply 的时间只能标作观察区间。

建议约束：

- dispatch parent 必须存在于同一 trace；不可指向自己。原生 parent 创建后不修改，避免构造循环。
- operation 和 event 的 trace/dispatch 必须一致，用复合外键加 NOT NULL/CHECK 约束，不能只分别检查 ID 是否存在。
- 同一 operation 至多一个 begin、一个 end；允许只有 begin，或因前置记录失败只有 end。缺的一侧保持缺失。
- 相同 event_id 和相同请求内容重复提交，返回原记录；相同 ID 携带不同内容报冲突，绝不覆盖。
- trace/dispatch 创建也采用稳定 ID 幂等。事件及其正文引用、links 一次提交后不可修改；更正另加事件，不能事后给旧操作补上当时不存在的依据。
- 指纹包含来源提交的身份、payload、正文引用、事件关联及时间，不包含服务端新生成的 recorded_at/seq。规范化规则必须版本化；路由完整响应的正文哈希仍基于实际保存字节。
- operation 结果可重复传递去重，但任何数据库重试都不得重新执行 Corral 或 JEV。没有收到落盘确认时，只查询或重交同一记录。

SQLite 复合外键在部分子列为 NULL 时会跳过对应检查，因此仅有 FOREIGN KEY 不够。每条连接也必须显式启用 foreign_keys。该行为已查官方文档并在合成检查中覆盖。[SQLite 外键规则](https://www.sqlite.org/foreignkeys.html)

## 写入流程与故障语义

建议每次写入按以下顺序执行：

1. 验证事件版本、显式记录上下文和字段白名单，检查总开关与链路开关并记下版本。无记录标识或已关闭的操作不读取遥测正文、不保存临时副本。begin/end 沿用操作开始时的版本，不在 end 时换取新版本来绕过期间的关闭。
2. 在调用现场取得任务书或响应的确切字节；后续写入使用这份快照，不让存储端稍后重读已变化的任务书。
3. 将正文写入同文件系统临时文件、计算哈希并同步文件。外部调用、读取大文件和计算哈希不占用写事务。
4. 开数据库写事务，复核采集开关及版本。仍有效才以不覆盖方式发布正文并同步目录，写入 blob 元数据、事件和引用；同次操作的 operation、任务书快照、begin 和关联一并提交，提交后才确认成功。并发相同哈希时复用已校验对象，不在锁内重新读取大正文。关闭/重开导致版本变化则丢弃本次记录，不补写。首期不自动垃圾回收。
5. start/send 等实际动作只执行一次，begin/end 分别写入；不在一个数据库事务里等待 agent 或网络返回。

正文发布及目录同步会短暂占用写锁，实际耗时须在实现中测量；这是为了让“关闭成功”与发布动作有确定顺序，不承诺固定毫秒延迟。拿不到有效开关版本时跳过采集并报告缺口，实际操作仍按既定语义执行。

正文与 SQLite 不共享事务。正文发布后数据库回滚，允许留下完整但未引用的文件；正确写入流程应避免已提交事件指向尚未可靠发布的正文。数据库外手工删文件或损坏仍需读取时校验并报告，数据库外键不能保证磁盘文件存在。

| 中断位置 | 保存结果 | 查询语义 |
|---|---|---|
| 正文尚未发布 | 可能有临时文件 | 没有可引用正文，不返回已记录 |
| 正文已发布，数据库未提交 | 可能有孤立正文 | 不虚构已提交事件 |
| begin 已提交，动作执行前或执行中中断 | 只有 begin | 结果未知，不能认定未执行 |
| 动作完成，end 写入失败 | 操作可能成功但记录不完整 | 报记录缺口，不重复执行动作 |
| 事件已提交，确认响应丢失 | 数据已在库中 | 按原 event_id 查询或幂等提交 |

创建链路、提交纯笔记等纯记录命令，保存失败应非零退出。带采集的执行命令区分操作结果与记录结果，记录失败不能覆盖实际 stdout/退出码。目录完全不可写时，错误提示可能是唯一留下的证据，不能保证把“丢了一条记录”也写进失败的存储。

## 开关与生命周期

实际采集条件是：`全局 enabled && 链路 capture_enabled && 本次操作显式携带有效关联`。建议首次安装默认关闭；这个初始默认值是提案，不是已经实现的行为。

| 控制 | 归属 | 含义 |
|---|---|---|
| 遥测总开关 | Saddle Settings / headless CLI | 关闭后所有链路停止新增内容采集；历史查询可用 |
| 新任务默认记录 | Drover Settings | 仅决定新任务执行是否申请 trace，不控制宿主总开关 |
| 本次记录或不记录 | 主控按用户要求显式选择 | 可覆盖 Drover 默认选择；不能越过关闭的总开关 |
| 停止某条链路 | Saddle 公开记录接口 | 仅停止该 trace 的后续采集，保留其历史 |

普通口头临时操作不传记录关联，默认不采集。用户要求“这次也记下来”时建立独立 trace，无需 Drover。已有记录上下文也不得通过环境变量自动传给其他临时操作。Drover 被停用不关闭 Saddle 遥测；已经建立的 trace 仍可显式使用。

总开关关闭成功是一个写入屏障：开关修改与最终正文发布、事件提交使用同一数据库写锁排序，返回成功后不再发布或提交被撤销版本的新内容。关闭前已经开始读取/暂存的字节不承诺安全擦除；进程崩溃可能留下临时或孤立文件。开关失败必须明确报失败，不能让界面显示已关闭。

总开关的实际状态变化才递增 generation，重复设置同值是幂等成功；链路开关也有自己的 generation。执行入口在调用开始取得版本，在保存结果时复核；经过关闭再开启也不能把旧的在途结果写回。开关关闭时，新动作直接跳过采集，不读取任务书来制作遥测副本。已经开始的实际动作继续，关闭前有 begin、没有 end 的记录显示结果未知和采集暂停背景，不判定失败。

允许保存最小的开关控制历史（状态、时间、版本），用于解释采集区间；它不含任务正文、prompt、路由或回复。查询某条 trace 时合并全局开关区间与链路开关事件。重新开启只允许以后开始的显式操作采集，不回放、不自动扫描文件或会话补录。

记录和查询由一次性 `saddle telemetry` 入口及同一 Rust 模块提供，不要求 TUI 或 Drover 正在运行。多个短命进程连接同一本地 SQLite；首期不建中心 daemon。

## 并发配置与公开查询

建议仅支持本机磁盘数据库，使用 WAL、foreign_keys=ON、synchronous=FULL、短写事务及有限 busy timeout。BEGIN IMMEDIATE 用于需要读取再判断后写入的操作，锁等待超时明确返回记录失败。设置值尚需在实现中按交互预算验证；不以数据库锁等待为由重做外部操作。

WAL 允许读写并行，但同一时刻仍只有一个写入者，不支持把同一个活跃数据库放到跨机器网络文件系统。长读事务会拖延 checkpoint，分页查询及时结束事务。[SQLite WAL 说明](https://www.sqlite.org/wal.html) FULL 在 WAL 中提供更强的提交持久性；NORMAL 可能在断电后丢失已经提交的最近事务。[同步配置说明](https://www.sqlite.org/pragma.html#pragma_synchronous)

实施时应固定实际链接的 SQLite 版本并核对已知修复，不能只看操作系统 sqlite3 命令版本。官方说明 WAL-reset 问题修复于 3.51.3 及后续版本，也有旧分支补丁。本轮 Python 试验使用 SQLite 3.53.4；Saddle 当前尚未接入 SQLite，试验版本不代表将来的发布二进制。[官方 WAL-reset 说明](https://www.sqlite.org/wal.html#walresetbug)

建议公开查询覆盖：按来源绑定列出链路、读取链路和派发、按 seq 游标分页事件、按哈希读取正文、导出原始事件。字段与接口名仍属提案。正文通过流式 stdin/stdout 或显式文件入口传递，保持原始字节；不塞入目前 48 KiB 的插件 command.v1。Drover 只调用公开查询，不直接开数据库或遍历正文目录。

需要为插件提供可靠的 Saddle 可执行文件定位方式，不能继续要求配置 dlog 仓库路径。它属于通用宿主接入，小型能力即可；不为此引入一套插件事件总线。ctl 的临时回执和持久遥测 event_id 分开，不把 256 条内存回执当遥测历史。

## 公开接口草案与旧程序退役

以下是职责级接口草案，命令名尚未实施。只提供足够串起本轮数据的入口，不增加通用插件事件总线。

| 入口 | 输入 | 输出与边界 |
|---|---|---|
| `saddle telemetry settings` | 查询或修改全局开关 | 持久化状态、generation；关闭状态仍可查询 |
| `saddle telemetry trace` | 新建/停用链路、可选通用外部绑定 | trace_id、记录状态；不读取 Drover 数据 |
| `saddle telemetry dispatch` | trace、kind、可选 parent | dispatch_id；与 task、Corral instance 分离 |
| `saddle telemetry append` | 版本化 JSON、event_id、关联、正文文件引用 | 已保存/同内容重复/冲突；主控陈述或插件提交，不冒充执行观测 |
| `saddle telemetry list/show/events/body` | 外部绑定或 ID、游标/页大小 | 结构化元数据或正文原字节；不依赖 TUI |
| `saddle agent start/send/reply` | 实际操作参数、可选记录上下文及任务书路径 | 保留 Corral 操作输出和退出语义；采集失败另报，不重复操作 |
| 内置 dispatch 路由入口 | 实际路由输入、可选 trace/dispatch | 路由结果；同步记录实际请求、完整解析响应与整理建议 |

记录上下文至少包含 trace_id、dispatch_id；一次调用生成一个 operation_id，将 begin/end 配对。初始 Drover 向主控交付建立 `controller_handoff` 派发，由 Drover 在实际交付消息中附上显式上下文，后续主控逐次传参；不由通用遥测包装器修改 prompt。参数语法错误、已确认 ID 不存在或跨链路关联是调用错误，执行前拒绝；数据库不可用等采集故障不阻止有效的业务操作，也不猜测或改绑关联。开始记录失败但动作已执行，后续只可提交记录结果，不允许再调用业务动作。

路由入口明确为同一 executable 的 headless `saddle dispatch route`，由内置 dispatch 模块执行并在进程内调用遥测，以固定 generation 原子采集；不通过公开 append 伪造执行观测。采集存储由 Saddle 提供，Corral 操作由 Saddle agent 入口提供。三个职责不要求三个常驻进程。Drover 通过公开接口提交任务事件和查询历史，禁用 Drover 不影响另外两者。

正文以文件字节或单独流读取；JSON 只携带引用和元数据。任意正文内容不拼接进 shell 命令，CLI 参数以数组传递。查询分页、正文原字节读取和事件 payload 各有版本，不能把界面预览截断结果作为分析原始数据。Drover 视图按 task/run 查询；没有绑定的临时链路仍能从 Saddle 查询入口读到。

不做旧 dispatch-log 数据导入、ID 沿用、旧事件映射或双写。不更改旧目录与程序；等新功能验证、消费者切换完成，再单独执行退役。新数据库有独立 schema version；遇到更高版本拒绝写入，不自动降级。旧历史继续由旧工具访问，不作为新查询接口的兼容承诺。

## 备份和以后分析

备份先取得 SQLite 一致性快照，再复制快照所引用的不可变正文，并校验缺失；备份期间禁止清理这些正文。恢复检查数据库完整性、外键和全部正文哈希。不能直接复制活跃 WAL 模式的主数据库文件当完整备份。[SQLite Backup API](https://www.sqlite.org/backup.html)

第一批值得支持的分析是：查看路由建议与主控选择、列出明确的升档重派、查找被标为 rework 的 send、检查记录缺口、按任务汇总多轮派发。这些不要求先建立自动评分系统。

统计应按 dispatch 或显式 operation 去重，不能把 begin/end 算成两次尝试，也不能把多次 reply 查询算成多次交付。独立审查和实现派发分组统计。没有记录标识的操作根本不在样本中，不能把采集样本比例称作全部工作的比例；未来按开关选择采集也会有选择偏差。

审查通过率、模型成本和路由正确率分别需要结构化结论、可信费用来源和明确评价规则。数据没有这些条件时，输出样本覆盖率与未知数量，不填零，不从没有返工记录推导“一次成功”。

## 本轮验证与尚未验证的部分

临时目录：`/tmp/saddle-telemetry-study-R8Ry1H/`。`probe.py` 为独立调查脚本，`result.json` 为结果。使用 Python 自带 SQLite 3.53.4、合成身份和正文，运行六表结构的约束子集。

实际命令：`python3 /tmp/saddle-telemetry-study-R8Ry1H/probe.py`，退出码 0，11 项通过：

1. 拒绝跨 trace 的 parent。
2. 拒绝跨 trace 的 operation。
3. 拒绝跨 trace 的 event。
4. 同 event_id 同内容返回重复成功，不同内容拒绝。
5. 拒绝同 operation 的重复 begin。
6. 只有 begin 时查询为未完成结果，而非自动判失败。
7. 四个独立进程各写 32 条事件，128 条全部保留。
8. WAL 读事务在并发写入时保持原快照，结束事务后看到新增事件。
9. 正文发布后、数据库提交前让子进程直接退出，留下未引用正文，未留下已提交的正文引用。
10. operation 的必要关联列为 NULL 时仍被 CHECK 拒绝。
11. 链路游标查询使用 `(trace_id, seq)` 索引。

最终有 130 条事件，integrity_check=ok，foreign_key_check 无结果。没有性能计时结论，没有 Rust/插件集成、真实掉电、磁盘满、macOS 发布二进制 SQLite 链接、备份恢复或开关并发验证。试验只证明原六表约束子集和这些合成情形可行；没有覆盖后来增加的 event_links、recording_policy、全局开关屏障或原话来源设计，不能把它作为实现已经完成的证据。

## 实施前的收敛与分步验证

本轮已确定归属、记录范围、SQLite 方案和无历史兼容要求。原话首期由主控逐字提交是待评审建议；它不能满足独立核验聊天原文的更强要求。若以后需要独立核验，应另行设计用户侧原文入口，不能把这项能力悄悄宣称已经具备。

建议按以下顺序实施；这是技术验证计划，不是新增用户验收项，也没有启动任务派发：

1. 存储与 headless 公开查询：先用合成数据检查跨链路拒绝、证据引用、版本追加、正文完整性、同 ID 幂等与冲突，再实现最小模块。
2. 总开关与执行采集：先用假 Corral/JEV 复现正常记录、记录失败动作仍只执行一次、关闭/重开期间的在途结果拒绝入库。以进程中断验证正文发布与事务失败边界，不用真实 agent。
3. 一条完整样例：原话 U1 → 提议/授权 → B1 路由 → B2 派发 → 回复/审查 → B3 返工 → 收尾；导出能还原每个版本与当时依据。后补授权不能改变历史操作引用。
4. Drover 接入：默认开关、task/run 绑定、状态提交与查询展示；验证插件停用后手工链路仍可记录/查询，以及未选记录的临时操作不产生内容。
5. 消费者切换与退役：按明确实施授权更新主控指令、技能、旧 dlog 配置与安装依赖；最后再处理旧工具退役。Corral 核心迁移仍在后续阶段。

字段与命令、失败回执、外部绑定交付时机和开关并发规则已收敛到 [接口契约 v1](../任务遥测接口契约.md)，已按 Claude 三轮审查补齐 M1–M4，主控与审查者认同技术方案可进入实施拆分，详见 [最终审查记录](Saddle遥测设计-Claude审查-2026-10-01.md)。原话取得方式和首次默认关闭仍标为产品建议；具体资源预算在实现中验证。当前只改设计文档；没有派发开发任务、提交 Git、安装或切换程序。后续行为实现遵循项目 RED→GREEN 与标准检查要求。
