# 任务：Corral 通用升级独立交叉审查

2026-10-03，saddle/main 交给新建 saddle/dev-corral-upgrade-review（Codex，重：gpt-6-astra / xhigh；实际名字以公开回执为准）。
类型：其他（实现后的独立审查）。
路由：依本任务已判定的碰要害影响与 corral-dispatch 交叉审查规则，独立 Codex 重档；不重新开启设计或另派 agent。
你是被委派的审查者，不是实现者，不再向下派发。

## 背景与先读

实现者 saddle/dev-corral-upgrade-1 在 corral-live-upgrade 分支交付 `816358a`（实现）与 `ce6f675`（完成记录）。主控已核对同实例 idle/DONE、工作区干净、范围和完成记录，已开始静态核实与标准检查重跑，尚未宣布通过。

先读 AGENTS.md、docs/任务/Corral通用升级-实施.md、docs/Corral通用升级设计.md，及直接相关源码。设计已由用户批准，不重开已接受的首次旧版本限制、Observer 退出码缺失或不覆盖任意崩溃的范围；检查实现是否真实符合该设计。

## 审查位置与唯一可写文件

- 审查 worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/review-corral-live-upgrade，detached HEAD `ce6f675`。
- 代码范围：`git diff main...HEAD`。源码、资源、测试和正式设计全部只读，不修改、不提交、不切分支，不操作实现者 worktree。
- 唯一可写文件是本任务文件：`/Users/firegnu/Developer/personal_projs/saddle/docs/任务/Corral通用升级-独立审查.md`，在主仓库而非你的 detached worktree。只在末尾追加本轮审查意见。
- 不改原独立 Corral 仓库，不读真实 Corral 私有状态、对话或用户数据，不运行真实 agent，不升级或重启服务，不切命令入口，不操作 Tasks/遥测，不按名字或路径批量杀进程。

## 重点

1. pen 状态/fd/锁/连接与终端解析状态的完整交接；所有权与析构、备用接管、active 边界、Hold/recover、连续升级、旧版本能力判断和结果归属。
2. 正常繁忙输入输出及在途回执是否完整，fd 复用与客户端断开、agent 自然退出/stop 等生命周期是否引入真实回归。
3. 持久 after 的独占交接、阶段/期限和 request_id 证据，未知不重发；正常升级保留语义，不把有限事件匹配提升成精确交付。
4. 外部稳定 hook 入口、pi/omp 原生采集和 Rust 判定的语义兼容、v1/v2 混用；托管是否确实不依赖特定 agent 类型。
5. 公开 CLI 和成套包的集成范围、旧实例/旧 worker 边界，以及测试能否支持完成记录的断言。不要把隔离合成证据称为真实产品版本或真实 agent 验收。

## 验证预算

- 先静态核对 diff、既有测试及完成记录；不另写测试/脚本或扩大故障矩阵，不录屏。
- 主控负责最终标准 test/clippy 各一次；你不重复全套。确有疑点时可运行直接相关的现有 core 定向测试（含升级专项或协议/生命周期选择）及 `node crates/corral-core/tests/collectors.mjs`，只跑与疑点有关的一次，不循环跑绿。
- Cargo 共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，显式 `--target aarch64-apple-darwin`。所有测试只用自己生成的临时隔离数据，记下并清理自建测试进程，不读取或操作用户 agent。
- 不因为标准首轮失败已限定复跑绿就宣布首轮全套通过；主控正在核实最终候选。不要改变共享构建配置或覆盖任何不可变包。

## 输出

只在本文件追加「## 审查意见」：结论（可以合并 / 改完再合并），每项级别（必须改 / 建议改 / 可以不改）、代码位置、具体触发/后果与修正方向，最后对实现者取舍逐项表态。必须改仅限未达到已批准任务结果或触及进程/状态/并发等要害的真实缺陷；非目标的架构偏好和新增需求不挡合并。

命令全部前台完成。回复只写结论、条数和本任务文件路径，最后一行 DONE。不合并、不推送、不关闭任何 agent。

## 审查意见

2026-10-03，独立只读审查。候选：`ce6f6753f29570da3b9d84d2bcb7960c6ec379e1`，代码范围为 `git diff main...HEAD`；已读 AGENTS.md、实施任务及完成记录、专项设计和相关集成约束。

**结论：改完再合并。发现 1 项必须改，0 项建议改。** 以下问题来自静态控制流核对；本轮没有新增故障脚本或测试，不能把它称为已运行复现。通过的现有测试不覆盖该失败分支。

### 1. 必须改：恢复校验失败后不能重新建立 fd 身份基准并覆盖原快照

- **位置**：`crates/corral-core/src/pen/upgrade.rs:537-568`，尤其是第 564 行；同文件 `save()` 第 220-229 行、`validate()` 第 478-519 行，以及 Hold/recover 第 648-654 行。
- **具体触发**：新映像在 active 之前恢复时，某个继承 fd 仍然有效，但已经指向与快照不同的对象，例如目标初始化错误地关闭一个继承客户端 fd，并在该编号打开另一个 socket。此时 `validate()` 会在第 503-504 行正确返回 `inherited fd identity mismatch`。这正是现有恢复验证明确处理的失败输入，不涉及要求保护 active 后的任意崩溃。
- **代码路径与后果**：`resume()` 将验证失败和验证通过后的元数据写入失败汇入同一个错误分支；第 564 行无条件调用 `pen.save()`。该方法不是“只更新 upgrade 元数据”：它重新对当前全部 fd 做 `fstat`，重新取得锁身份，并重新写入 `schema: SCHEMA`。因此，只要当前 fd 均有效、目录可写，它就会把已经判为错误的 fd 身份写成新的正确基准。随后的旧映像回退继承同一组错误 fd，却会通过对新基准的校验，继续到 active 和流 I/O；客户端可能丢失或流被送到错误对象。若回退 exec 失败而留在 Hold，备用 K 仍持有原资源，但共享快照已经被改为 N 的错误身份，K 后续接管时反而会校验失败。原始 schema/fd 表校验失败也会被这次重新序列化抹去。Hold 的 recover 再调用同一个 `save()`，所以仅删除第 564 行仍不足以保证后续恢复不重新认可错误资源。
- **为何挡合并**：专项设计 §3、§4 要求保留精确快照，以原 fd/锁身份验证后才能越过 active；验证失败只能尝试符合该快照的旧映像恢复或进入 Hold。这条错误处理路径会撤销自身的身份校验，并破坏备用的恢复依据，属于进程资源与状态交接的真实缺陷。
- **修正方向**：区分“原始快照验证失败”和“验证成功后的提交/元数据失败”。保留原 `Snapshot` 的 schema、描述符身份、锁身份与流状态，失败记录及 recover 的 epoch/attempt/target 更新不得用当前未经验证的资源重建这些字段；若需要移除已经退出备用的管道角色，只能按明确的角色变更处理。在原始身份约束重新满足前，不得取得资源清理权或进入流 I/O。实现返工时应针对这条已定位路径验证：校验拒绝不会被后续保存/回退/recover 变成认可，原快照仍可供持有原资源的备用使用；本审查未自行增加该测试。

### 本轮核验与证据边界

以下命令均在前台等待完成；Cargo 共用指定 target 目录，显式选择 `aarch64-apple-darwin`。每项只运行一次：

| 检查 | 本轮实际结果 |
|---|---|
| `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test -p corral-core --target aarch64-apple-darwin --test upgrade` | 15 passed，0 failed，0 ignored |
| `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test -p corral-core --target aarch64-apple-darwin --test protocol raw_v2_events_keep_original_input_reply_and_retry_grace_with_v1 -- --exact` | 1 passed，0 failed，11 filtered out |
| `node crates/corral-core/tests/collectors.mjs` | 通过；Node 提示 TypeScript 资源未声明 package module 类型，未为此修改任何配置 |
| `git diff main...HEAD --check` | 通过 |

升级专项实际覆盖正常两次升级、繁忙输出顺序、半帧与终端解析状态、写者交接、人类保护、排队输入及回执、客户端 fd 复用、备用接管、Hold/recover、备用退出、active 禁止重放、持久提醒交接与证据缺失。其中 schema 用例是冻结前的 probe 拒绝，Hold 用例注入的是元数据 instance 不一致；它们均未验证上面指出的“恢复 fd 身份校验失败后重新保存”的分支。

未重复标准全套 test/clippy，最终标准结果由主控核实；实施者首轮全套失败及限定复跑的记录仍按原文保留，不能提升为首轮或最终候选全套通过。未重新打包或运行成套包检查；本轮仅静态核对打包变更和实施者记录。上述运行证据来自临时 HOME/CORRAL_HOME、合成程序/事件及同一构建复制的不可变临时路径，不是真实产品版本间升级或真实 agent 验收。现有夹具负责自身进程清理；结束后只读检查临时测试 Corral 可执行进程，未见匹配的残留，未向其他进程发信号。

### 对实施者取舍逐项表态

1. **可以不改——依赖与分层**：仅新增 serde derive，核心不反向依赖宿主、插件或任务业务；公开 CLI 接入和成套资源放在同一交付中，符合范围。
2. **可以不改——正常快照范围与退出码语义**：Pen/Client/Chunk/Terminal 的现有内存字段及绝对定时均进入快照；fd 与连接 order 共同关联回执，正常父进程使用 waitpid，Observer 退出码为 null。这些取舍成立；资源身份验证失败后的保存必须按第 1 项修正。
3. **可以不改——Owner 限制与 Observer 权限**：普通升级拒绝未收尾状态、未退出备用及 Observer；没有自动提升 Observer 清理权限或扩大其后续升级能力。保留已批准边界。
4. **可以不改——after 两阶段期限和确认含义**：waiting 与后续交付分别持久化期限，单次请求先记 sending；串行锁交接、accepted/written 证据及缺证据 unknown 不重发的方向符合设计。recent 有界，按文本/时间匹配不等于逐条精确确认或持久去重；不要求本轮增加接收端幂等协议。
5. **可以不改——稳定 helper 与原生采集器**：helper 按 CORRAL_HOME 建立并原子切换；pi/omp 采集事实，Rust 负责解释，现有 v1/v2 定向测试通过。已经加载的旧采集器不会被热替换，普通程序仍是 unknown / confirmed:false；托管升级路径没有按 agent kind 分流。
6. **可以不改——合成版本与证据限制**：两套路径来自同一构建，实施记录已明确其仅验证交接机制。真实 Claude/Codex/pi/omp 兼容冒烟仍未执行，不能据此声称全路径验收。
7. **可以不改——首次旧版本与故障窗口**：旧 pen/固定 helper/无记录 after 的首次过渡、快照至备用建立之间及 active 之后的无损限制按已批准设计保留。第 1 项位于已设计的 pre-active 验证/回退路径，不是在重新要求任意崩溃零损失。
8. **可以不改——检查与部署分离**：实施者如实保留了标准首轮失败、限定复跑与未执行项；本轮不要求重新消耗整套预算。未部署、未切真实入口、未修改原独立 Corral、未操作真实 agent/任务/遥测；后续验证、部署和首次过渡继续由主控按原授权安排。

本轮仅向主仓库本任务文件追加意见；detached worktree 的源码、资源、测试和正式设计保持只读，没有提交、合并、推送、再派发或关闭 agent。
