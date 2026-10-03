# Corral 通用升级：实施前一致记录

日期：2026-10-03。记录者：saddle/main。

来源：主控通过公开 `corral reply saddle/dev-live-upgrade-review-1` 取得最终修订回复，instance=`43b27dc12c46`，reply at=1791020880.96019。下文为该次公开回复原文，不代表运行验证。

主控结论：双方就首个具备升级协议版本之后的通用升级达成可实施一致；现存旧 pen、固定适配器和旧 after 的首次过渡仍未解决。用户在确认主要修改仓库内 Rust Corral 核心后明确回复“批准”，授权设计落盘、隔离实现、验证和项目审查流程。部署与首次过渡另行确认。已核对的正式约束以 `docs/Corral通用升级设计.md` 为准；回复内部 recover 成功的简写以 B3 的 complete 判定为准。

此记录保留其限制：不是无缝能力已验证，不承诺任意崩溃无损。主控前期发现的不安全重试已删除；适配层升级已纳入必要范围。

## 最终公开回复

本轮无需新命令。先逐点修订，再给自洽的最终版，最后给一致性判断。补充一条源码依据：客户端断开后 pen 不会清空已排队的输入块，`drop_client` 只移除客户端，输入块仍会写入 master，只是找不到回执目标【源码 pen.rs:243-255、555-559】。这直接支持主控对 B2 重试规则的否定。

## 一、逐点修订

**B1 两处矛盾。承认，小修。**

- Hold 的下限不是保证。进入 Hold 的前提是旧 exe 也无法 exec，K 在 EOF 时执行旧 exe 很可能同样失败。修订：K 在 EOF 且标记为 `json` 或 `committed` 时，依次尝试快照记录的最新 `recover` 目标与旧 exe；全部 execve 失败则 `_exit`，master 关闭，agent 收 SIGHUP。结果如实记为「Hold 中 N 崩溃且无可执行恢复映像时无保证」。Hold 的真正下限只有一条：N 自身不崩溃时 agent 存活。
- `recover` 的结果对应。快照新增 `epoch`、`attempt`、`target`；每次 `recover` 在 execve 前把 `attempt` 加一并写入快照与目标，回执为 `{"ok":true,"accepted":true,"epoch":E,"attempt":A,"target":T}`。`accepted` 只表示已接受。status 的 `upgrade` 字段为 `{state, epoch, attempt, target, last_error:{attempt, reason}}`。调用方按 `(epoch, attempt)` 对应：`exe` 等于 T 且 `attempt` 等于 A，为本次成功；`state` 为 hold 且 `last_error.attempt` 等于 A，为本次失败及原因；`last_error.attempt` 小于 A 且 `state` 为 hold 不会出现，因为 `recover` 是同步的，execve 失败即刻写入 `last_error`。新的 `recover` 不改 epoch，只增 attempt，旧状态不会被误读。

**B2 删除重试规则。承认。** 「recent 未被挤出即未接受」不成立，在途请求可能仍在 backlog 或 socket 缓冲，status 连接可先于它被处理，客户端断开后已排队输入仍会执行【源码同上】。修订如下：

- sending 阶段无法证实时记 unknown，不自动重发，不换 `request_id`。不引入接收侧幂等，因为那需要 pen 持久保存 `request_id` 并定义保留周期，超出必要范围。
- 证据分三级，不互相替代。一，pen 的 `recent` 条目含该 `request_id` 与 `written`，证明 pen 已接收或已完整写入本次请求，只保留最近 10 条，被挤出即不可得。二，events 的摘要与时间匹配，是既有 CLI `confirmed` 的语义【源码 cli.rs:216-232】，不含 `request_id`，同文本两次发送无法归属到某一条提醒；记录中写 `confirmed_by_event`，不升格为精确送达证明。三，二者皆无为 unknown。
- 接续规则按阶段：waiting 继续等待；sending 先查 `recent` 的 `request_id`，有且 `written` 为真则转 sent，为假则等待其变真，无则查既有事件匹配，再无则 unknown；sent 只做既有确认，超时为 not_delivered；终态不动作。
- handover 检查位置。worker 不再把 `turn_end` 当黑盒调用，而是自己持有三个循环：等待循环每 200 毫秒一次 status 请求，确认循环每 100 毫秒一次事件读取，重试循环每 2 秒一次【源码 cli.rs:303、245、596】。`handover` 检查放在每个循环迭代的开头，即每次外部请求之前。持有者只在「已宣告 sending 且 pen 回执未返回」这一段不释放，该段上限为一次请求的 30 秒。因此释放延迟上限约为 30 秒加一个迭代间隔，而不是整个等待函数。
- 锁文件规则。锁文件为空，永不写入，永不自动删除；取锁后按登记锁的方式复核路径 inode 与 fd inode 一致【源码 state.rs:107】；`handover` 只是提示，不授予执行权；新持有者取锁后重读记录。
- 边界如实表述：正常升级路径上 worker 在迭代边界交接，提醒连续；非预期崩溃落在「已宣告 sending 到记录回执」窗口内，且 `recent` 证据不可得时，为 unknown。不写成已证明恰好一次。

**B3 统一判定。承认。**

- 结果状态五种，CLI 与 status 共用：`accepted` 为请求已接受且未到 `active`；`running_new_pending` 为 `exe` 等于目标但 `state` 为 pending，K 未退出或标记未删；`complete` 为 `exe` 等于目标且 `state` 为 none；`failed` 带 `attempt` 与原因，含回到 S0 与进入 Hold 两种落点；`unknown` 为无回执且 status 不可达。只有 `complete` 称成功，`running_new_pending` 期间拒绝新的 `upgrade`。
- `active` 之后到 K 退出之间的 N 崩溃，如实列为「升级期间该阶段无恢复保证」，不再归入既有运行故障。各次 rename 只保证自身原子，不使多个动作原子化；`active` 之后的回执、写 `X`、删标记、custody 切换各自幂等可重试，但不构成一个事务。

**范围一致性。承认并纳入。** 外部 helper 的翻译逻辑与进程内扩展里的回合判定、输入匹配是 Corral 业务逻辑，属于「最新核心」的一部分；只有 agent 自身的 hook 调用形状与扩展加载方式归 agent。因此稳定入口与采集器分离纳入必要实施项，且必须在首个支持升级的版本中一起发布，否则在此之后、补齐之前启动的 agent 又会带上冻结的旧适配器。

## 二、最终版要点

**正常升级与失败状态**

| 阶段 | 动作 | 失败结果 | 保证 |
|---|---|---|---|
| S0 准入 | `upgrade` 仅在 Owner、无标记、无 K 时接受，epoch 加一 | 回 `upgrade_pending` 或 `hold` | 现状不变 |
| S1 预检 | `__pen-probe` 解析快照副本 | 回 failed | 现状不变 |
| S2 冻结与快照 | custody 置 Handoff，写 `upgrade.json` | 写快照到 fork 之间 N panic，无保护 | 升级引入、无恢复保证的窗口，长度为一次文件写 |
| S3 fork K | 双管道，K 只读管道一 | fork 失败回 S0 | 现状不变 |
| S4 exec | 清 CLOEXEC 后 execve | 返回错误则恢复标志、写 `A`、回 S0 | 现状不变 |
| S5 验证 | 零 I/O，fstat 与 inode 核对 | 失败则 exec 回旧 exe；再失败进入 Hold | S4 到 S6 期间 N 崩溃由 K 精确恢复，pen PID 变 |
| S6 提交 | rename `committed`，构造 Pen，重写 meta | meta 失败继续并重试 | 同上 |
| S7 交接 | rename `active`，进入 poll，回执，写 `X` | N 崩溃则 K 不动作，agent 收 SIGHUP | 升级期间该阶段无恢复保证 |
| S8 完成 | 管道二 EOF 后删标记，custody Owner | K 不退出则保持 pending | status 可见 |
| Hold | 只接受 status 与 recover，同 epoch 重试 | N 崩溃时 K 依次尝试 recover 目标与旧 exe，全失败则无保证 | N 存活则 agent 存活 |

**worker 接续规则**：锁持有者唯一执行；先宣告阶段再动作；迭代边界检查 handover；sending 不可证实即 unknown；不重发；`confirmed_by_event` 沿用既有语义不升格；锁文件不写不删并复核 inode。

**必要实施项，依序**

1. pen：`Custody` 与 Drop 规则；`upgrade`、`recover`、`__pen-probe`、`__pen-resume [--observer]`；快照；三级标记与双管道 K；Hold；status 的 `exe`、`custody`、`upgrade` 五元组。
2. CLI：`corral upgrade`，输出五种结果状态，绝不停止任何进程。
3. 适配层：外部 hook 的稳定入口链接；pi、omp 采集器与读取端判定分离，读取端同时接受 v1 判定记录与 v2 原始记录。
4. 提醒：持久记录、锁持有者协议、`request_id` 与 `recent` 的 `request_id`、`written`。

明确不在本次范围：有损接管；cursor 分名；revision 元数据；中央托管层。

**实施验证项**：fd 与标志跨 exec；流分区在半帧、延时 send、挂起请求下不缺不重；K 的三种反应与两处崩溃注入；Observer 退出观察；Hold 往返与 `(epoch, attempt)` 对应；custody 下 panic 不杀 agent；worker 在三阶段的崩溃接管与同文本两次发送；适配层在合成外部 hook 与真实 pi、omp 上升级后继续工作；真实 Claude、Codex 容忍窗口；时延与 5 秒、30 秒超时的关系。

**首次边界**：现存 pen 无 `upgrade` 入口；现存 agent 的 hook 路径与扩展固定；现存 after worker 的文本只在内存并随各自 `--timeout` 结束。三者不可由设计改变，不写成已满足。

## 三、一致性判断

我同意主控本轮全部修订。

A。面向具备升级协议版本的后续通用升级设计，我认为已达到可实施程度，仍须用户授权开始。保留的技术风险都是实施验证项，不是设计矛盾。本机制是「在现有每 agent 一个 pen 的架构下原地升级」的方案；若用户选择其他架构，那是另一份设计，不断言所有路线都需要它。

B。对今天现存的 agent，完整目标仍被首次过渡卡住。准确限制是：它们的 pen 没有升级入口，适配器已固定，待触发的提醒只在旧 worker 内存中。设计不能改变这三点。

C。从 B 到 A 需要用户确认的最小条件只有一项：现存 agent 如何走到首个支持升级的版本。可选为自然结束后在新版本下开新会话，或由用户选时间对现存 agent 各重启一次，或授权对其他路径做实验且结果未知。现存 agent 的待触发提醒随其 worker 的 `--timeout` 结束，不是单独决策。`active` 之后的有损接管不是进入 A 的条件，本设计已决定不纳入，用户可日后另行提出。

按用户指示，一致仅指可进入实施阶段，实施前停下等用户。本轮未运行命令，未修改文件，未实施、实验或操作任何 agent。

DONE
