# 任务：实现 Corral 通用升级

2026-10-03，saddle/main 交给新建 saddle/dev-corral-upgrade（Codex，重：gpt-6-astra / xhigh；实际名字以公开 start 回执为准）。
路由：重 / 交叉审查要 / 影响面：碰要害（路由模型 jev-1.13.0：重，交叉审查与影响面拿不准；主控依据进程资源所有权、并发交接与在途数据判为碰要害，需要独立审查）。
类型：功能变更
依据：用户确认一致方案主要改 Rust Corral 内核、少量公开消费者接入，明确回复“批准”实施；实际部署和首次过渡另行确认。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的实现者，照本文件做，不再开其他 agent。本链路未选择遥测、不创建 Tasks run。

## 先读

- AGENTS.md。
- docs/DESIGN.md 的“Corral 通用升级”一节。
- docs/Corral通用升级设计.md（本轮完整正式方案）。
- docs/Corral核心Rust集成设计.md §2、§3、§4、§6、§8.1 的分层与兼容约束。
- 直接相关源码及现有 crates/corral-core/tests；最新讨论需要追溯时读 docs/调研/Corral通用升级-实施前一致记录.md。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/corral-live-upgrade。
- 分支：corral-live-upgrade，从主控的设计/任务提交建立。
- 范围：crates/corral-core 的实现、资源、测试；必要的 Cargo.toml/Cargo.lock；直接相关公开消费者兼容、scripts/package.sh 与说明；本任务完成记录。核心不反向依赖宿主、插件、遥测或任务业务。
- 正式设计已先落盘。常规实现细节可记录；如遇必须改变已批准行为或实质架构的情况，先向主控报告，不自行降目标。不要改主仓库工作区或其他历史 worktree。

## 要做的

按专项设计完成首个支持升级版本所需的集成能力：

1. 通用 pen 的正常原地升级、完整在途状态、资源所有权、备用进程与 Hold 恢复；能力、身份和结果通过公开接口可观察。
2. Corral 公开 upgrade/recover 与批量结果，正确区分 accepted、pending、complete、failed、unknown 和旧版无能力；不停止旧 pen 兜底。
3. 未来外部 hook 稳定入口，以及 pi/omp 采集器与可升级判定分离，同版本兼容 v1/v2 事件。普通程序与已知 agent 共享托管升级机制。
4. 新 after 持久记录、独占持有者交接、request_id 的发送事实；sending 无法证实时 unknown，不重发。保持原有等待、空闲、人类保护、超时和确认含义。
5. 仅必要的公开消费者/帮助/成套资源接入；打包仍只产出新目录，不切真实链接。所有有集成关系的部分在本分支交付，不把 pen 单独完成当作全链路完成。

边界：这是未来支持协议版本之间的能力。现存旧 pen/固定适配器/旧内存 after 的首次过渡未解决；报告 needs_restart/不支持，不擅自重启或伪造无缝。部分故障窗口无保护如实报告；正常升级不能静默丢输入、连接或提醒。

## 怎么算做完

用户原话：
> 这个方案不能绑定某一些agent。

用户在确认主要修改 Corral 内核及“正式设计 → 隔离实现 → 合成验证 → 主控审查，实施与部署分开”的实施顺序后回复：
> 批准

具体授权范围及已有取舍以专项设计为准；不自行补充用户验收要求或把未测能力写成通过。

## 验证预算

- 遵循 AGENTS.md 的行为 RED→GREEN；目标检查先可运行且因缺失行为失败，编译错误不算 RED。针对进程、流、恢复、提醒和事件适配的直接相关边界检查在碰要害预算内自行选择，命令前台完成。
- 直接相关检查，加标准各一次：`cargo test --all-targets --target aarch64-apple-darwin`；`cargo clippy --all-targets --target aarch64-apple-darwin -- -D warnings`；`git diff --check`。均用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。
- 新测试与夹具不含 Python；默认全部使用临时 HOME/CORRAL_HOME、合成程序/事件和不可变临时包，不依赖真实 coding agent 或用户数据。生命周期测试不得绑定会被重编译覆盖的共享顶层二进制。
- 标准首轮如有疑似无关失败，最多限定复跑一次，保留首次结果。返工仅跑修复相关检查与直接回归，不重跑整套预算。未完成真实 agent 兼容验证须明确记录，不能冒充全路径验收。

## 不要做

- 不部署、不安装或改用户技能/配置/命令链接，不升级、重启、停止、注入或输入现存 agent，不操作真实任务队列或遥测。
- 不读取 Corral 私有真实状态、对话或终端内容；测试状态只能是本任务隔离生成的数据。不改原独立 Corral 仓库及任何旧版本包。
- 不加入中央服务、有损恢复、自动重发、旧 Python 兜底、无关 UI 或相邻优化。
- 不按路径/项目名批量杀进程；只清理自己记下 PID 的隔离测试进程。
- 不合并 main、不推送，只在 corral-live-upgrade 分支提交。不要自行删 worktree 或停 agent。

## 做完

在本文件末尾追加完成记录并提交：实现与提交号、真实 RED/GREEN 和标准检查结果、必要取舍、已知限制、未执行项目。若实验证伪关键机制且无法在既定设计内解决，报告具体阻塞，不用 stub/skip/弱化断言宣布完成。回复给出提交号、结果及需主控决定事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（2026-10-03，被委派实现者）

实现提交：`816358a9b23776da10f43d194910372c66294878`（实现 Corral 通用升级与持久提醒交接）。集成实现已交付本分支，等待主控与独立审查；没有合并、推送或部署。

### 交付内容

- pen 的 snapshot schema 1、目标预检、原地 exec、完整流/终端/定时/身份/元数据快照、fd 角色与锁 inode 验证、Owner/Handoff/Observer 清理规则、双管道备用、active 边界、Hold/recover 和同 epoch attempt。状态公开实际 exe、能力、custody、保护状态和结果。保留 proto 1，核心无宿主/插件/业务依赖。
- 公开 `upgrade [--all|NAME] [--exe PATH]`、`recover`；批量分别报告 pen、helper 和持久提醒，旧 pen 为 needs_restart，回执不明为 unknown，不停止旧 pen 兜底。旧 pen 上由新协议建立的持久提醒仍独立交接，不能把它们误判为旧内存 worker。
- 每个 CORRAL_HOME 的稳定 helper 入口；pi/omp 只采集 v2 原生事实，Rust 读取端负责输入原文关联、回复、去抖和重试宽限，兼容 v1/v2。普通程序仍为 unknown / confirmed:false，升级本身不按 kind 分支。
- 新 after 持久记录与永久锁 inode、串行持有者交接、绝对等待/交付期限；`send` request_id 的 accepted/written 事实和 `after NAME --request-id ID` 公开查询。sending 有接受证据时持续观察原请求；证据缺失为 unknown，不重发。正常交接完成不等于提醒已交付，结果同时保留 phase。
- 必要的指南、随包技能资源与 UPGRADING.md；打包加入采集器和专项设计，仍只创建新目录。既有 Saddle/插件公开调用边界无需改动，没有新增宿主私有状态读取。

### RED → GREEN 与直接回归

以下 RED 都是可编译、可运行检查中的行为失败；实施中的编译错误没有计入 RED。

| 检查 | 实施前/修复前的真实 RED | GREEN |
|---|---|---|
| 公开升级保留身份和半包连接 | upgrade 返回 unknown or missing command | 最终专项通过 |
| 排队输入事实与回执 | recent 的 request_id 为 null | accepted/written、原连接回执与退出码通过 |
| 稳定外部 helper | 注入的是固定版本真实路径 | 稳定入口及切换后原命令执行通过 |
| v2 读取与旧事件混用 | unsupported event format | 原文确认、回复、重试宽限及 v1 混用通过 |
| 持久提醒公开结果 | pending 回执缺 request_id | after 查询及普通程序 confirmed:false 通过 |
| fd 复用后的在途回执 | 新半包连接被旧回执关闭，BrokenPipe | 回执关联改为 fd + 连接 order，跨升级通过 |
| recover 记录写入失败 | status 仍为 accepted | failed/last_error 可见，再次 recover 成功 |
| 已接受提醒的回执等待过期 | 错误转成 unknown | 保持 sending 观察，随后只写入一次 |
| 旧 pen 与新持久提醒混用 | 提醒 worker 仍在旧 exe | pen 保留 needs_restart，记录型 worker 独立交接 |

- 最终 `cargo test -p corral-core --target aarch64-apple-darwin --test upgrade`：**15 passed，0 failed，0 ignored**。还覆盖繁忙输出逐字节顺序、两个接入者、半帧输入、终端解析半序列和模式、人类保护、写者交接、schema 拒绝、备用接管、Hold 原 epoch 恢复、备用丢失、active 后禁止重放、提醒期限/证据丢失/不重复发送。
- 直接协议与生命周期回归：`--test protocol --test lifecycle` 共 **20 passed**；后续返工仅跑相关升级专项与 core clippy，没有重跑整套预算。
- `node crates/corral-core/tests/collectors.mjs`：pi/omp 合成扩展 API 通过；采集器保留原生事实、无判定定时器，写入失败不影响调用者。没有运行真实 pi/omp。
- 两次检查夹具时序失败也保留在记录中：Hold 查询可能在冻结前已成为旧连接，后改为有界等待新的控制连接；提醒交接回执可先于新 worker 的下一次阶段判断，后改为有界等待 unknown。业务断言（连接连续、原 request_id、文本只有一次写入）没有删弱。一次失败遗留的隔离 pen/备用/cat 已核对精确 PID 并清理，夹具加入自身 PID 清理；最后未发现本任务测试 pen 遗留。

### 标准检查：保留首轮结果

所有 Cargo 命令都使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，目标为 `aarch64-apple-darwin`，命令均等待前台完成。

1. `cargo test --all-targets --target aarch64-apple-darwin` 首轮 **退出 101**。在 fail-fast 前汇总 **510 passed、1 failed、9 ignored**。唯一失败是未改动的宿主 `tests/workflow.rs:851` 中 `native_mouse_buttons_cover_forms_and_stop_confirmation`，界面把标题/正文连到一起，未等到队列中的分行文本；该测试使用既有假 Corral，与新 pen 无直接调用关系。
2. 按预算只对该用例限定复跑一次：`cargo test -p saddle --target aarch64-apple-darwin --test workflow native_mouse_buttons_cover_forms_and_stop_confirmation -- --exact`，**1 passed**。这不是首轮全套通过，也不是最终 HEAD 的整套重跑；首轮 fail-fast 后未执行的目标不补称已验证。
3. `cargo clippy --all-targets --target aarch64-apple-darwin -- -D warnings` 首轮 **退出 101**：本次新增代码的 unnecessary_unwrap 和 nonminimal_bool 两处告警。已修正；最终限定 `cargo clippy -p corral-core --all-targets --target aarch64-apple-darwin -- -D warnings` **通过**。没有重跑整套 workspace clippy，不宣称它已全绿。
4. `git diff --cached --check` **通过**，包括新增文件；任务完成记录追加后再次检查相应文档差异。

日志保留在本机临时目录 `/tmp/saddle-corral-upgrade-checks.BE7R1b/`：`test-first.log`、`workflow-rerun.log`、`clippy-first.log`、`clippy-core-final.log`、`upgrade-final.log`，以及 `upgrade-handover-observation-race.log`。这些是本轮证据路径，不是发布内容。

### 成套包与消费者

- `sh -n scripts/package.sh` 通过；实际执行打包成功，检查必需二进制、说明和两份采集器齐全，并验证对已有输出目录拒绝覆盖、文件哈希不变。
- 最终包：`/tmp/saddle-corral-upgrade-checks.BE7R1b/product-final`。BUILD.txt 标记 revision 为 `816358a9b23776da10f43d194910372c66294878`、working-tree 为 clean；没有安装、注册插件或切换任何真实入口。最终打包日志为 `package-final.log`。
- 使用该包的 saddle/corral 显式运行原有可选成套检查 `tui_exit_and_reopen_preserve_agent_identity_and_runtime -- --ignored --exact`，**1 passed**（`product-final.log`）。它在临时 HOME/CORRAL_HOME 内使用合成 cat，验证 TUI 关闭和重开不改变实例/agent PID，不是真实 agent 冒烟。

### 实现取舍、限制和后续

- 新依赖仅加入 workspace 已有的 serde derive。快照包含全部 Corral 自有解析/队列状态；内核 PTY 模式随同一 fd 保留。恢复结构在验证及 active 边界前不取得自动关闭 fd 或杀 agent 的清理权，普通父进程用 waitpid，备用用非父进程观察与 PTY 退出，后者退出码为 null。
- 普通 upgrade 遵循设计的 Owner 限制；备用接管保持 Observer，不擅自提升清理权限。标记/备用尚未收尾时不允许下一轮。Observer 后续开启新升级轮次没有擅自扩展为新的设计能力。
- after 保留原先“等待被等待者结束/更换/超时，再等待接收者可交付”的两阶段超时含义；每段期限持久化后不因交接重置。request_id 的 recent 证据有界，事件确认仍不构成同文本多提醒的精确逐条确认或接收端持久去重账本。
- 生命周期测试从同一新构建复制出两套不可变临时路径，按实际 exe 区分，验证的是首个支持协议版本的交接机制；没有声称已经验证任意真实产品版本间升级。无新增 Python 代码或夹具。
- 现存旧 pen、固定 helper 和无记录 after 的首次无缝过渡仍未解决；快照到备用建立之间、active 之后的崩溃没有无损保证。没有用 stub、skip 或重启伪装这些能力。
- **未执行**：真实 Claude/Codex/pi/omp 兼容冒烟、真实安装/入口更新、首次过渡、真实 agent/任务/遥测操作、修改原独立 Corral 仓库、主仓库/其他 worktree 写入、合并和推送。
- **需主控决定/继续**：主控和独立审查；若需要最终整套 workspace 全绿，另行给检查预算；真实兼容验证、部署与首次过渡仍需按既定授权边界另行安排。没有实验证伪而被隐瞒或降低的已批准正常升级机制。

## 返工 1 完成记录

2026-10-03，被委派的原实现者。依据主控的《Corral通用升级-返工1》及独立审查唯一必须改项，在原 `corral-live-upgrade` 分支完成限定修正。实现提交：`9648ff0`（修复 Corral 恢复时重建快照身份基准的问题）；本节单独提交作证据记录，等待主控复核。

### 根因与修正范围

- 原 `resume` 在校验失败后丢掉 `Snapshot` 外层身份依据，再调用 `Pen::save` 对当前 fd/锁重新采集；fallback、Hold/recover 会因此认可错误资源，并覆盖备用仍需使用的原依据。
- `resume`、Hold 和 standby 现在保留完整 `Snapshot`。失败记录、recover 的 epoch/attempt/target 更新通过 `Snapshot::save` 持久化，不重建 schema、描述符身份、名称锁身份或流状态。仅初始 live pen 的两次交接保存采集身份。
- standby 接管及 Hold 确认备用退出时，明确移除 control/alive 两个管道角色和对应描述符；其余基准保持原值。Hold 使用 listener 或取得管道关闭权前，校验 schema/角色表及该资源的原身份；未通过的 master/客户端不取得清理权，不进入原流 I/O。
- 修正中直接暴露一个相关误拒绝：macOS 上原 socket 对端断开后，`fstat.st_mode` 的权限位会变化，原来再次采集身份掩盖了此变化。定向实测中设备号/inode 不变，mode 从 49590 变为 49152。身份校验使用原设备号、inode 和 `S_IFMT` 文件类型；保存的完整 mode 不重写。同类型 socket 替换仍拒绝。失败信息保留具体 fd 和原/实际 stat 值，便于定位。
- 源码/测试范围仅 `crates/corral-core/src/pen/upgrade.rs`、`crates/corral-core/tests/upgrade.rs`；未改协议 schema、正式设计、CLI 接口、适配器、after 或打包。

### 实际 RED → GREEN 与直接回归

所有命令前台等待结束，Cargo 均使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，显式 `--target aarch64-apple-darwin`。以下命令列省略共同的环境前缀；完整日志目录为 `/tmp/saddle-corral-rework1.6GqKIm/`。

| 检查/实际命令 | 实际结果与日志 |
|---|---|
| `cargo test -p corral-core --target aarch64-apple-darwin --test upgrade rejected_fd_identity_survives_fallback_recover_and_standby -- --exact`，先加测试、未改实现 | **RED，退出 101**；编译运行成功后，断言发现仍带 `inherited fd identity mismatch` 的回退已变为 `custody: owner`、`state: none`。`red.log` |
| 同一命令，保留 Snapshot 的初次修正后 | **GREEN，1 passed**；验证 fallback/recover 拒绝、原快照不变、备用从原资源恢复冻结连接。`green.log` |
| `cargo test -p corral-core --target aarch64-apple-darwin --test upgrade -- rejected_fd_identity hold_recovers hold_reports recover_record_failure pre_io_crash standby_never` | **首轮直接回归 6 passed / 1 failed，退出 101**；原 `hold_reports_lost_standby_and_can_recover_unprotected` 被 socket mode 动态变化误拒绝。`regression.log`，未将该轮写成通过 |
| `cargo test -p corral-core --target aarch64-apple-darwin --test upgrade hold_reports_lost_standby_and_can_recover_unprotected -- --exact`，增加 fd/stat 错误详情后 | **限定定位复跑仍失败，退出 101**；同 fd 的设备号/inode 不变，mode 权限位改变。`pipe-diagnosis.log` |
| `cargo test -p corral-core --target aarch64-apple-darwin --lib pen::upgrade::tests::descriptor_identity_survives_peer_disconnect -- --exact` | **RED → GREEN，退出 101 → 0，最终 1 passed**；先证明同一 socket 对端关闭后的误拒绝，再核实类型位校验通过且 `dup2` 替换成另一 socket 仍拒绝。`socket-mode-red.log`、`socket-mode-green.log` |
| `cargo test -p corral-core --target aarch64-apple-darwin --test upgrade -- rejected_fd_identity hold_recovers hold_reports recover_record_failure pre_io_crash standby_never public_upgrade_keeps`，最终候选 | **8 passed / 0 failed，退出 0**；`regression-final.log` |
| `git diff --check`、源码/测试提交前 `git diff --cached --check` | 通过 |

两个新增集成用例仅在隔离 exec 映像中用有效文件替换一个继承客户端 fd，备用仍持有原 socket：

- 比较失败记录和 recover 后快照的全部非 upgrade 字段，确认 schema、描述符、锁身份与完整流状态未改变；校验拒绝未被 fallback/recover 豁免，未出现 active，错误资源未被写入，原客户端保持冻结。
- 有备用时，终止本测试失败映像后，备用按原快照接管，保留 agent PID/instance，恢复两条原连接及请求回执。
- 备用先退出时，确认 `hold_unprotected`；recover 只移除原管道描述符，其他快照字段不变，错误 fd 仍被拒绝。

最终 8 条集成回归还包括正常原地升级与半包连接、原 Hold 成功恢复、备用退出后的成功恢复、recover 记录写入失败后重试、pre-active 崩溃接管、active 标记禁止重放。额外 1 条 unit 检查覆盖同 socket 对端变化与同类型资源替换；不是另跑全套升级或故障矩阵。

### 验证边界与未执行项

- 全部使用临时 HOME/CORRAL_HOME、合成 cat/脚本以及同一构建复制的固定临时二进制；不代表真实产品版本间升级或真实 coding agent 兼容验收。夹具只清理本次记录的 PID；结束后只读检查临时测试可执行路径，未见匹配残留。
- 按返工预算，没有运行 workspace 全套、clippy、重新打包或真实 agent 冒烟。主控任务书已说明修正前候选标准 test/clippy 通过，该结果不属于修正后验证。
- 没有新增任意崩溃恢复、旧 agent 首次迁移或 Observer 后续升级；原实施边界继续有效。没有部署、真实 agent/任务/遥测操作、再派发、主仓库/审查 worktree 写入、合并或推送。
- 后续仅待主控按本次修正范围复核；未擅自启动额外全套检查或部署步骤。
