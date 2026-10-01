# 阶段04：遥测查询页与 Drover 接入（精简设计，待用户确认布局）

2026-10-02。依据[实施计划](任务遥测实施计划.md) 04/05 边界、[接口契约](任务遥测接口契约.md) §1/§7/§8 和[遥测使用](遥测使用.md)。本文是草案：线框待用户看过后再实施；不重开归属、SQLite、默认关闭、来源/晚交、Corral 最后迁移等已定决策。

## 1. 结论

- 查询页属于宿主，直接调用现有 `telemetry::Store` 的 `settings/list_bound/show/events/body`（`src/telemetry/query.rs`、`blobs.rs`），不另写 SQL 访问层，不依赖 Drover/dispatch 启用。
- 首版只做“读取并如实显示”：链路列表 → 链路时间线与事件/派发详情 → 完整正文。不加统计、评分、导出、后台轮询。
- Drover 接入分两件事：**跳转**（缺一个通用插件请求 `telemetry.open.v1`）和**记录**（全部走已有公开 CLI，缺宿主向插件进程提供自身路径）。宿主只收不透明 binding 条件，不读 Drover 文件、不写插件名。
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

**B. 链路详情**（先 `events` 固定 `upper_seq` 取第一页，再 `show(trace)`）

```text
┌ Telemetry ▸ T57 Fix queue focus ───────────────────── fixed at seq ≤ 214 ────┐
│ drover.task · T57 · run 3f2a9c   登记声明未核验                              │
│ coverage 08:14:03Z · recording on (1 off interval) · gaps 2 · unknown 1      │
│ Dispatch: All  ◂ Tab ▸   handoff 1a2b · implementation 77c0 · review 90de    │
│──────────────────────────────────────────────────────────────────────────────│
│ seq time     disp kind                   src notes                           │
│  41 08:14:03 1a2b agent.send.begin       E   message 2.1 KB                  │
│  42 08:14:04 1a2b agent.send.end         E   confirmed                       │
│▸ 57 08:20:11 77c0 brief.snapshot         E   brief 6.4 KB                    │
│  58 08:20:11 77c0 agent.start.begin      E   gap prompt:not_supplied         │
│   ·          77c0 agent.start (no end)   ·   执行或交付结果未知              │
│  63 09:02:40 ·    requirement.recorded   C   late · text 312 B               │
│ ── n: next page (100 per page) ──                                            │
│─ seq 57 ─────────────────────────────────────────────────────────────────────│
│ brief.snapshot · observed by Saddle · op 5be1 agent.start                    │
│ 任务书快照  /Users/me/proj/saddle/docs/任务/T57.md                           │
│ captured 08:20:10Z · phase begin · body brief 6,412 B · ↵ Read               │
│ links  based_on → seq 40 requirement.recorded                                │
│ ↑↓ Select  ↵ Read  Tab Dispatch  i Intervals  n More  r Refresh  Esc Back    │
└──────────────────────────────────────────────────────────────────────────────┘
 Input ▸ Telemetry · Esc Back
```

- 时间线按 seq 升序；`n` 用同一 `upper_seq` 取下一页（`next_after_seq`），新事件不混入；`r` 重新固定上限并从头读。上限显示在标题右侧。
- `src` 列：E=Saddle 现场观测，S=系统开关，C=主控声明，P=插件声明；C/P 在详情中写 `declared by <producer> (unverified)`，即 `source_description`。
- 缺 begin/end 来自 `show(trace).operations`：缺 end 插入一条无 seq 的 `(no end)` 行并附 `unknown` 原文；reply 显示“回复查询未完成/结果未知”，不并入交付未知。缺 begin 时在 end 行注明 `begin missing`。有 `outcome` 但仍未知（timed_out、pending、缺必需正文等）同样显示 `unknown` 原文，不写成成功。
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

| 要显示的 | 来源 |
|---|---|
| coverage_start、开关区间、全局/链路开关 | `show(trace)` 的 coverage_start / recording_intervals / capture_enabled；`settings()` |
| 已知 gaps | `known_gaps`、事件 `payload.gaps` |
| 来源声明、未核验 | 事件 evidence_kind、producer、`source_description` |
| 缺 begin/end、结果未知 | `operations[]` 的 begin_missing / end_missing / unknown / delivery_result_unknown |
| 晚交 | 事件 late_submission / submission_notice |
| 运行登记声明 | `registration` |
| 正文完整性 | `body()` 的成功 / body_missing / body_corrupt / not_found |

唯一建议的小补充：`show(trace)` 增加只读字段 `dispatches:[{dispatch_id,kind,parent_dispatch_id,created_at}]`，否则 Tab 只能用已加载事件和 operations 推测本链路有哪些 dispatch（只有 D 作用域事件的 dispatch 可能翻页前看不到）。这是 query.rs 内的附加字段，CLI `show` 输出随之多一个字段，不改存储。

## 5. Drover 最小接入

### 5.1 关联查询跳转（与记录无关）

现状：协议已有 `agent.open.v1`（插件在用户输入回调中请求、宿主核验后执行，`runtime.rs` 约 633 行），没有打开宿主页面的请求。最小补充，形状照抄 agent.open：

- 能力 `telemetry.open.v1`；插件请求 `telemetry.open`，参数 `input_id` 和 `filter:{kind,scope,key,run?}`。宿主只检查来源为当前用户输入、字符串非空、无控制字符、各 ≤256 字节、每次输入至多一次，然后以该 `BindingFilter` 打开 §3A。响应 `opened|failed|cancelled`，SDK 超时为 unknown，不重试。
- 调用方向：插件 → 宿主。宿主不解释 kind/scope/key 的业务含义，不按插件 ID 分支；任何声明该能力的插件同样可用。
- Drover：任务详情（有任务号）加 `Telemetry ↗` 按钮，传 Drover 自己创建链路时用的同一组值（建议 kind=`drover.task`、scope=规范化项目根、key=任务号，不带 run 以列出该任务全部轮次）。无任务号的待办不显示按钮。没有记录时页面只是空结果。

### 5.2 记录（契约 §7，全部经公开 CLI）

现状：`saddle telemetry trace create / dispatch create / append / list` 与 `saddle agent --corral PROGRAM --record-context … -- send …` 已实现；Drover 的 dispatch-pending 直接执行 `corral send`（`plugins/drover/src/core.rs` 约 800–857 行）。缺口：插件进程拿不到宿主可执行路径——契约 §2.2 拟议的 `SADDLE_HOST_BIN` 源码中不存在（`runtime.rs` spawn 处未设置）。最小补充：宿主为**所有**进程插件设置 `SADDLE_HOST_BIN=<当前 saddle 绝对路径>`，不放任何记录上下文。

Drover 在 dispatch-pending 中按序（插件侧改动，宿主无业务逻辑）：

1. 取“本次选择”，否则取默认选择（Drover 自己保存，见 §7）。未选择 → 原路径，零变化。
2. 生成 run_id 后，经 `$SADDLE_HOST_BIN telemetry trace create`（origin=task，binding 同 §5.1 加 run）和 `dispatch create`（controller_handoff）建立身份；ID 预先生成并沿用，总等锁预算 300 ms。disabled/unavailable/超预算 → 原路径，结果注明本次无记录上下文。
3. 有上下文时在交付消息末尾加独立标记的附段（仅 trace_id、dispatch_id、声明的 task/run，不含授权），把上下文 JSON 写成 0600 临时文件，执行 `$SADDLE_HOST_BIN agent --corral <现有 --corral 值> --record-context <abs> -- send <MAIN_AGENT> <消息>`。`--corral` 原样传程序路径，不把配置改成含空格的复合命令。无 MAIN_AGENT 时附段进入 manual_text。
4. 该子进程按契约 §5.2 独立进程组（现 `command.rs` 只 `child.kill`，需为此路径改为组取消）；收齐 stderr 后按首尾配对判回执。只有配对成功且 executed=false 才算未执行；125/127 不触发 start 登记；配对失败一律 unknown，**不自动重发**。Corral 原退出码按现有映射处理。
5. 按原规则写 Drover start；成功后 `telemetry append` 一条 task.transition（pending→running，带 business_committed_at）。append 失败只影响 telemetry 字段。
6. 操作结果保留原 `delivery`/`record`，另加 `telemetry:{status,trace_id,dispatch_id}`。采集任何一步失败都不改变业务结果、不阻断交付。

之后的提交审查/验收/退回若属已记录的 run，Drover 用 `telemetry list --kind --scope --key --run` 找到链路再 append task.transition，不在 Drover 数据中新存 trace_id。任务执行授权仍只来自原流程；附段不构成授权。

## 6. 实施划分

| 步骤 | 内容 | 主要文件 |
|---|---|---|
| 04A 查询页 | §2–§4；可选的 `dispatches` 字段；合成库验证 | 新增查询页模块（如 `src/telemetry_view.rs`）、`src/app.rs`、`src/ui.rs`、`src/telemetry/query.rs`、`docs/遥测使用.md` |
| 04B Drover 接入 | `telemetry.open.v1`、`SADDLE_HOST_BIN`、§5 Drover 改动 | `crates/plugin-protocol`、`crates/plugin-sdk`、`src/plugins/runtime.rs`、`src/app_plugins.rs`、`plugins/drover/src/{core,queue,plugin,command}.rs`、`plugins/drover/plugin.toml`、`docs/插件协议.md`、Drover README |

串行：04A 审查后保留分支/worktree，04B 从其上开始；共享集成点是 `app.rs`/`app_plugins.rs` 中“带筛选打开查询页”的入口和 `docs/遥测使用.md`。两半一起集成验收后再合并清理。05 才切旧消费者、退役 Drover Dispatch 页签与 `--dispatch-log`；本轮不删旧入口、不导入旧日志。

## 7. 现状分类

| 已可复用 | 确需补接口/实现 | 需要用户决定 |
|---|---|---|
| Store 查询、无库只读、晚交/未知/登记判定、正文校验 | 宿主查询页（全部 UI） | 入口：`t` 加顶部 `Telemetry` 字样（推荐），还是只要按键 |
| `saddle telemetry` 记录命令、`saddle agent` 起止回执 | `show(trace).dispatches`（小、可选） | Drover 默认记录选择存放：按项目（推荐，存 Drover 项目配置，默认关）还是全局 |
| `agent.open.v1` 请求模式、Settings 总开关 | `telemetry.open.v1`、`SADDLE_HOST_BIN` | 本次覆盖的形式：Dispatch selected 旁一个单次开关，ctl `dispatch` 加可选 `record` 参数（推荐两者都有） |
| 旧 Drover Dispatch 页签（保留到 05） | Drover 选择、附段、wrapper 调用、进程组、回执解析、task.transition、跳转按钮 | 首版是否对后续流转（提交/验收/退回）也写 task.transition（推荐写） |

04 不只是“读 SQLite 显示”：查询页基本如此，但跳转和记录需要上述两项通用接口及 Drover 交付流程改动。
