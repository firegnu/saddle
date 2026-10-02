# 阶段04：遥测查询页与 Drover 接入（设计已核对，布局已批准）

2026-10-02。依据[实施计划](任务遥测实施计划.md) 04/05 边界、[接口契约](任务遥测接口契约.md) §1/§7/§8 和[遥测使用](遥测使用.md)。用户已看过主控展示的线框和记录选择方式并明确“批准”，主控已静态核对修订57d9a87并关闭M1–M3；本文定稿不代表功能已实施或运行验证通过。不重开归属、SQLite、默认关闭、来源/晚交、Corral 最后迁移等已定决策。

## 1. 结论

- 查询页属于宿主，直接调用现有 `telemetry::Store` 的 `settings/list_bound/show/events/body`（`src/telemetry/query.rs`、`blobs.rs`），不另写 SQL 访问层，不依赖 Drover/dispatch 启用。
- 首版只做“读取并如实显示”：链路列表 → 链路时间线与事件/派发详情 → 完整正文。不加统计、评分、导出、后台轮询。
- Drover 接入分两件事：**跳转**（缺一个通用插件请求 `telemetry.open.v1`）和**记录**（全部走已有公开 CLI，缺宿主向插件进程提供自身路径）。只有在任何业务发送开始之前、记录身份建立失败时才回到原直接 send 路径；`saddle agent` 一旦启动，结果一律按契约解释，绝不改走直接 send 重发。宿主只收不透明 binding 条件，不读 Drover 文件、不写插件名。
- 不需要改 Corral，也没有依赖反转。

## 2. 入口、焦点与通用约定

- 入口（推荐）：Agents 焦点下按 `t`；Agents 顶部操作行增加 `Telemetry`，与 Plugins/Settings 同色同排，沿用既有窄宽换行规则。底栏帮助加 `t Telemetry`。另一来源是插件跳转（§5）。
- 页面像插件管理页一样是覆盖整个工作区的模态页（`app.rs` 的 `plugin_page` 同类处理：期间暂停底层光标与终端输入），底栏显示 ` Input ▸ Telemetry · Esc Back`。最外层 Esc 关闭并把焦点还给打开前的位置（Agents 或发起跳转的插件窗格）。
- 三层：列表 →(Enter) 链路 →(Enter) 正文；Esc 逐层返回，返回时保留上一层的选中与滚动。鼠标：点行选中、滚轮滚动，按钮与底栏快捷项可点，沿用现有列表行为。
- 查询在后台线程执行，显示 `Loading…`；离开该层即丢弃迟到结果。只在打开、换筛选、翻页和按 `r` 时查询，无定时刷新。
- 外框/页签风格、颜色、选中箭头沿用 Settings/Plugins 页；Store 返回的固定中文说明（`coverage_notice`、`registration`、`unknown`、`submission_notice`）原样显示，不改写其语义。

## 3. 线框（80×24；示意，列宽以实现为准）

**A. 链路列表**（`list_bound`；未筛选即全部链路）

```text
┌ Telemetry ────────────────────────────────────────── Recording: Off (global) ┐
│ Filter: all traces                                     12 traces · r Refresh │
│──────────────────────────────────────────────────────────────────────────────│
│▸ 10-02 08:14  task    drover.task · T57 · run 3f2a9c   登记声明未核验        │
│  10-02 07:50  task    drover.task · T57 · run 9c01e4   未见运行登记声明      │
│  10-01 21:03  ad_hoc  manual review of plugin loader          trace paused   │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│─ selected ───────────────────────────────────────────────────────────────────│
│ label  T57 Fix queue focus                                                   │
│ scope  /Users/me/proj/saddle      key T57      run 3f2a9c1e-…                │
│ coverage_start 2026-10-02T08:14:03Z · trace recording on                     │
│                                                                              │
│ ↑↓ Select  ↵ Open  f Filter  F Clear  r Refresh  Esc Close                   │
└──────────────────────────────────────────────────────────────────────────────┘
 Input ▸ Telemetry · Esc Close
```

- 筛选只暴露 Store 已有字段：`f` 打开一行四格表单 kind / scope / key / run（run 可空）；kind、scope、key 须同时填写，否则表单内提示，不发查询。插件跳转预填并在 Filter 行注明 `(from <插件显示名>)`。`F` 清除。
- 行内不显示 gaps/unknown 计数：`list` 不返回这些，逐行再 `show` 属于 N+1，首版不做；进入链路后显示。
- 无库：正文区一行 `Telemetry is not initialized — nothing recorded yet. No files were created.`；总开关关闭只影响右上角状态，已存记录照常可查。存储不可用/版本不支持：显示错误码与说明，不显示成“无记录”。

**B. 链路详情**（两次独立读取：`show(trace)` 给“本次查询时的状态”，`events` 给固定上限的事件列表；不是同一时刻快照）

```text
┌ Telemetry ▸ T57 Fix queue focus ─────────────────────────────────────────────┐
│ Now (queried 09:05:12) · drover.task · T57 · run 3f2a9c · 登记声明未核验     │
│ recording on (1 off interval) · gaps 2 · ops 3: no end 1, unknown 1  o Ops   │
│ Dispatch: All  ◂ Tab ▸   handoff 1a2b · implementation 77c0 · review 90de    │
│─ Events seq ≤ 214 ───────────────────────────────────────────────────────────│
│ seq time     disp kind                   src notes                           │
│  41 08:14:03 1a2b agent.send.begin       E   message 2.1 KB                  │
│  42 08:14:04 1a2b agent.send.end         E   confirmed                       │
│▸ 57 08:20:11 77c0 brief.snapshot         E   brief 6.4 KB                    │
│  58 08:20:11 77c0 agent.start.begin      E   gap prompt:not_supplied         │
│  63 09:02:40 ·    requirement.recorded   C   late · text 312 B               │
│ ── n: next page (100 per page) ──                                            │
│─ seq 57 ─────────────────────────────────────────────────────────────────────│
│ brief.snapshot · observed by Saddle · op 5be1 agent.start                    │
│ 任务书快照  /Users/me/proj/saddle/docs/任务/T57.md                           │
│ captured 08:20:10Z · phase begin · body brief 6,412 B · ↵ Read               │
│ links  based_on → seq 40 requirement.recorded                                │
│ ↑↓ Select  ↵ Read  Tab Dispatch  o Ops  i Intervals  n More  r Refresh  Esc  │
└──────────────────────────────────────────────────────────────────────────────┘
 Input ▸ Telemetry · Esc Back
```

按 `o` 时底部详情区换成当前操作摘要（同样标注查询时间，不混入事件列表）：

```text
│─ Operations now (queried 09:05:12) ──────────────────────────────────────────│
│ 5be1 agent.start 77c0  begin seq 58 · no end · 执行或交付结果未知            │
│ 9a10 agent.send  1a2b  begin seq 41 · end seq 42                             │
│ c3d2 agent.reply 90de  begin missing · end seq 231 (beyond event list)       │
```

- 两个读取各标各的：顶部摘要、`o` 操作摘要、`i` 开关区间都标 “Now (queried 时刻)”，表示本次 `show` 读到的当前状态；事件列表只标 `seq ≤ upper_seq`，表示事件集合固定在该上限。两者分属不同读事务，摘要可能包含列表上限之后才出现的 operation/事件，这时写出其 seq 并标 `beyond event list`，不倒推成该上限时的历史。
- 事件列表按 seq 升序；`n` 用同一 `upper_seq` 取下一页（`next_after_seq`），新事件不混入。`r` 同时重读两者：新的当前摘要和新上限的第一页。每页的晚交等装饰是该页查询时计算的结果，不承诺全局某一时刻的快照；不为此新增历史快照存储或接口。
- `src` 列：E=Saddle 现场观测，S=系统开关，C=主控声明，P=插件声明；C/P 在详情中写 `declared by <producer> (unverified)`，即 `source_description`。
- 缺 begin/end、结果未知只出现在当前操作摘要（来自 `show(trace).operations`），事件列表里不插入无 seq 的行。摘要直接显示 `unknown` 原文：reply 为“回复查询未完成/结果未知”，不并入交付未知；有 `outcome` 但仍未知（timed_out、pending、缺必需正文等）同样显示原文，不写成成功。选中某个 begin/end 事件时，底部详情另起一行 `op 5be1 now: no end (queried 09:05:12)`，明示是当前状态。
- 晚交：`late_submission` 为真时 notes 写 `late`，详情显示 `submission_notice` 与声明时间/入库时间两者。
- gaps：notes 列出 `gap 角色:原因`；头部 gaps 数来自 `known_gaps`。`i` 把底部详情区切换为开关区间列表（`recording_intervals`）加 `coverage_notice`，再按 `i` 回到事件详情。
- **任务书与任务描述分开标注**：`brief.snapshot` 的 brief 正文固定标“任务书快照”（附原路径、captured_at、publication_phase、begin_missing）；`agent.send.begin` 的 message 标“实际发送内容”（Drover 交付时包含任务描述与记录上下文附段）；需求/授权原文标“需求原文/授权原文（声明来源）”。不互相替代，没有快照时显示 gap，不拿发送内容冒充任务书。
- 派发详情：Tab 在 All 与本链路各 dispatch 间切换，事件按 `dispatch_id` 过滤（`events --dispatch-id`），底部头两行显示该 dispatch 的 kind、parent、created_at（`show(dispatch_id)`）。链接显示目标 seq 与类型；跨 trace 的 `carried_from` 写出目标 trace 短 ID，首版不跳转。

**C. 正文阅读**（`Store::body`，按哈希读取完整原字节）

```text
┌ Telemetry ▸ T57 ▸ seq 57 ▸ 任务书快照 ─────────────────── lines 1–17 / 214 ──┐
│ sha256 9f3c…0e1 · 6,412 bytes · hash verified · UTF-8                        │
│──────────────────────────────────────────────────────────────────────────────│
│ # 任务：修复队列焦点                                                         │
│ …                                                                            │
│ ↑↓ PgUp PgDn Scroll  g/G Top/End  w Wrap  x Hex  Esc Back                    │
└──────────────────────────────────────────────────────────────────────────────┘
```

| 情况 | 显示 |
|---|---|
| 成功、UTF-8 | 全文，默认软换行；`\t`、`\n` 外的控制字符显示为可见转义（如 `\x1b`），不把 ANSI 序列送进终端 |
| 成功、非 UTF-8 | 头部写 `not UTF-8`，默认十六进制视图；`x` 可切到标注 `lossy` 的替换字符视图 |
| 空文件 | `Empty body (0 bytes, hash verified)` |
| body_missing / body_corrupt | 红色错误行及错误码，正文区为空且明说“不是空白正文” |
| 事件只有 gap | `Not captured: <reason>`，不打开阅读层 |
| 读取不可用 | 错误码与说明，可 `r` 重试 |

正文按契约上限一次读入（≤16 MiB），只渲染可见行；不截断，不修改存储。多正文事件按 Enter 先列出各角色再选。

**宽屏**：列表增加 scope 全路径与完整时间列；链路详情在宽度足够时把事件详情放到时间线右侧，时间线占满高度；正文层只是更宽。无另一套布局。

## 4. 证据语义对照（全部来自现有 Store 输出）

单任务边界补充（2026-10-02，规范见[定稿设计](遥测单任务边界设计.md)，分支已实现、未安装）：list/show(trace) 增加 closed_at。列表 `trace ended`、底部 `trace recording ended <closed_at>`、详情 `recording ended` 均按此字段判断，不能从最后区间判断。recording_history/intervals 只纳入该 trace 结束事件 seq 及之前的控制事件，终止区间无条件以 changed_at 封口，无 now 尾段；未结束算法、原始 events 查询不变，时钟回拨不改变 seq 截止。带 closed:true 的 recording.changed 显示 `Trace ended`。缺 end 仍为 no end、原因未知，结束不证明成功，不重做布局或展示语言。

| 要显示的 | 来源 |
|---|---|
| coverage_start、开关区间、全局/链路开关 | `show(trace)` 的 coverage_start / recording_intervals / capture_enabled；`settings()` |
| 已知 gaps | `known_gaps`、事件 `payload.gaps` |
| 来源声明、未核验 | 事件 evidence_kind、producer、`source_description` |
| 缺 begin/end、结果未知 | `operations[]` 的 begin_missing / end_missing / unknown / delivery_result_unknown |
| 晚交 | 事件 late_submission / submission_notice |
| 运行登记声明 | `registration` |
| 正文完整性 | `body()` 的成功 / body_missing / body_corrupt / not_found |

04A 明确实现一项小补充：`show(trace)` 增加只读字段 `dispatches:[{dispatch_id,kind,parent_dispatch_id,created_at}]`，用于 Tab 枚举本链路全部 dispatch，包括事件尚未加载的（只有 D 作用域事件的 dispatch 翻页前看不到）。这是 query.rs 内的附加查询字段，CLI `show` 输出随之多一个字段，不改 schema 或存储；Tab 不再依赖已加载事件推测。

## 5. Drover 最小接入

### 5.1 关联查询跳转（与记录无关）

现状：协议已有 `agent.open.v1`（插件在用户输入回调中请求、宿主核验后执行，`runtime.rs` 约 633 行），没有打开宿主页面的请求。最小补充，形状照抄 agent.open：

- 能力 `telemetry.open.v1`；插件请求 `telemetry.open`，参数 `input_id` 和 `filter:{kind,scope,key,run?}`。宿主只检查来源为当前用户输入、每次输入至多一次，筛选字段的校验与 Store binding 完全一致（kind/scope/key 必填，run 可省；各值非空、不含 NUL，即 `model.rs::nonempty`），然后以该 `BindingFilter` 打开 §3A。不照搬 agent.open 针对 agent 身份的 256 字节上限，不截断、改写或额外拒绝合法 binding（例如很长的规范化项目路径）；整条消息只受既有协议帧上限（`MAX_LINE` 4 MiB）约束，超出按已有协议错误处理。值中的其他控制字符照常接受，显示时按界面安全规则转义（§3C 同一规则），§3A 的 `f` 表单同此。响应 `opened|failed|cancelled`，SDK 超时为 unknown，不重试。
- 调用方向：插件 → 宿主。宿主不解释 kind/scope/key 的业务含义，不按插件 ID 分支；任何声明该能力的插件同样可用。
- Drover：任务详情（有任务号）加 `Telemetry ↗` 按钮，传 Drover 自己创建链路时用的同一组值（建议 kind=`drover.task`、scope=规范化项目根、key=任务号，不带 run 以列出该任务全部轮次）。无任务号的待办不显示按钮。没有记录时页面只是空结果。

### 5.2 记录（契约 §7，全部经公开 CLI）

现状：`saddle telemetry trace create / dispatch create / append / list` 与 `saddle agent --corral PROGRAM --record-context … -- send …` 已实现；Drover 的 dispatch-pending 直接执行 `corral send`（`plugins/drover/src/core.rs` 约 800–857 行）。缺口：插件进程拿不到宿主可执行路径——契约 §2.2 拟议的 `SADDLE_HOST_BIN` 源码中不存在（`runtime.rs` spawn 处未设置）。最小补充：宿主为**所有**进程插件设置 `SADDLE_HOST_BIN=<当前 saddle 绝对路径>`，不放任何记录上下文。

Drover 在 dispatch-pending 中按序（插件侧改动，宿主无业务逻辑）：

1. 取“本次选择”，否则取默认选择（Drover 自己保存，见 §7）。未选择 → 原路径，零变化。
2. 生成 run_id 后，经 `$SADDLE_HOST_BIN telemetry trace create`（origin=task，binding 同 §5.1 加 run）和 `dispatch create`（controller_handoff）建立身份；ID 预先生成并沿用，总等锁预算 300 ms。这一步发生在任何业务发送之前：disabled/unavailable/超预算 → 原直接 `corral send` 路径，结果注明本次无记录上下文。**这是唯一的降级点。**
3. 有上下文时在交付消息末尾加独立标记的附段（仅 trace_id、dispatch_id、声明的 task/run，不含授权），把记录上下文写成 0600 临时文件：`{"schema_version":1,"trace_id":…,"dispatch_id":…,"send_kind":"initial"}`（契约 §7 第3步要求的 initial），执行 `$SADDLE_HOST_BIN agent --corral <现有 --corral 值> --record-context <abs> -- send <MAIN_AGENT> <消息>`。`--corral` 原样传程序路径，不把配置改成含空格的复合命令。无 MAIN_AGENT 时不发送，附段进入 manual_text。
4. 该子进程按契约 §5.2 独立进程组（现 `command.rs` 只 `child.kill`，需为此路径改为组取消），等进程退出并收齐 stderr 后，沿用已有规则判回执：字节 0 的第一条完整行为起始边界、最后一条完整行为终止回执、前缀/版本/final 与同一 call_id 匹配，不设计新协议。
   - 配对成功且 executed=false：确定未执行：返回交付未执行，不登记 start。只有这种情况能说明未执行；退出码 125/127 本身不能证明，配置的程序或 Corral 自己也可能返回同码。
   - 配对成功且 executed=true：按 Corral 原退出码与公开结果走现有映射（confirmed/unconfirmed/rejected/unknown）。
   - 起止边界缺失、截断或不匹配，超时、被取消：结果 unknown，按现有 unknown 规则处理。
   - 采集本身失败（begin/end 保存失败、gap 等）：只体现在 telemetry 字段，业务结果按上面判定。
   - 以上任何情况下，`saddle agent` 已启动后都**不再改走直接 send，也不自动重发**。
5. 按原规则写 Drover start；成功后 `telemetry append` 一条 task.transition（pending→running，带 business_committed_at）。append 失败只影响 telemetry 字段。
6. 操作结果保留原 `delivery`/`record`，另加 `telemetry:{status,trace_id,dispatch_id}`。采集失败不改变业务结果、不阻断交付；但它不是重发或换路径的理由。

之后的提交审查/验收/退回属于契约 §7 既有的整条任务流转，不是需要另批的新需求：若属已记录的 run，Drover 用 `telemetry list --kind --scope --key --run` 找到链路再 append task.transition，不在 Drover 数据中新存 trace_id。任务执行授权仍只来自原流程；附段不构成授权。

单任务边界补充：Accept/Return 业务落盘并尝试 transition（Return 带原始原因）之后，Drover 对已解析的同 run trace 调用 `telemetry trace close`，有独立300 ms预算。Submit 不 close；找不到 trace 或查询失败为 not_attempted，不新建。transition 失败仍尝试 close，结果保留 `telemetry.status/trace_id` 并增加 `telemetry.close:{status}`，分别报告失败/未尝试，不重放业务、不改变任务状态成功。遥测可能永久缺该条流转/原因，Drover 原始原因保留。项目默认/本次覆盖不变，同run返工同trace，Return后新run按既有选择；主控不提前close绑定run。宿主仍只实现通用结束接口，不识别Drover状态机。

## 6. 实施划分

| 步骤 | 内容 | 主要文件 |
|---|---|---|
| 04A 查询页 | §2–§4，含 `show(trace).dispatches` 只读字段；合成库验证 | 新增查询页模块（如 `src/telemetry_view.rs`）、`src/app.rs`、`src/ui.rs`、`src/telemetry/query.rs`、`docs/遥测使用.md` |
| 04B Drover 接入 | `telemetry.open.v1`、`SADDLE_HOST_BIN`、§5 Drover 改动 | `crates/plugin-protocol`、`crates/plugin-sdk`、`src/plugins/runtime.rs`、`src/app_plugins.rs`、`plugins/drover/src/{core,queue,plugin,command}.rs`、`plugins/drover/plugin.toml`、`docs/插件协议.md`、Drover README |

串行：04A 审查后保留分支/worktree，04B 从其上开始；共享集成点是 `app.rs`/`app_plugins.rs` 中“带筛选打开查询页”的入口和 `docs/遥测使用.md`。两半一起集成验收后再合并清理。05 才切旧消费者、退役 Drover Dispatch 页签与 `--dispatch-log`；本轮不删旧入口、不导入旧日志。

## 7. 现状分类

| 已可复用 | 确需补接口/实现 |
|---|---|
| Store 查询、无库只读、晚交/未知/登记判定、正文校验 | 宿主查询页（全部 UI）及 `show(trace).dispatches` |
| `saddle telemetry` 记录命令、`saddle agent` 起止回执与上下文 | `telemetry.open.v1`、`SADDLE_HOST_BIN` |
| `agent.open.v1` 请求模式、Settings 总开关 | Drover 记录选择、附段、wrapper 调用、进程组、回执解析、各流转的 task.transition、跳转按钮 |
| 旧 Drover Dispatch 页签（保留到 05） | — |

**用户已批准的布局和记录选择**：
- 布局：顶部 `Telemetry` ＋ `t` 进入；列表 → 详情 → 正文三层；Drover 任务详情加 `Telemetry ↗` 关联按钮。
- Drover 记录选择：按项目保存默认值、默认关闭，本次交付可单次覆盖（界面在 Dispatch selected 旁，ctl `dispatch` 加可选 `record` 参数）。Saddle总开关仍决定是否允许采集；项目记录选择不代表宿主管理项目分派规则。

已按既有契约确定、无需另批：后续流转写 task.transition；`dispatches` 字段；04A/04B 串行；旧 dlog 入口留到 05。

04 不只是“读 SQLite 显示”：查询页基本如此，但跳转和记录需要上述两项通用接口及 Drover 交付流程改动。
