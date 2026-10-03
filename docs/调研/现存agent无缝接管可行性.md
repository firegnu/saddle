# 现存 agent 无缝接入最新 Corral：Claude 调查与主控复核结论

日期：2026-10-03。调研者：saddle/dev-live-upgrade-1（Claude Code，被委派，只读调研）。
基线：分支 `corral-live-upgrade-research`，从 main `bd2066e` 创建，加任务文件 `6a51ba8`。
范围：本机自用；不涉及发行、Homebrew、外部用户。本文只给可讨论的结论和建议；没有实现、部署、迁移，没有运行接管原型，也没有改正式设计。

本文于 2026-10-03 按用户要求归档到主仓库，供两个会话关闭后接续。Claude 调查提交为 `460d03b`，主控复核提交为 `f0bdc90`；下文保留原调查正文及其依据，较强措辞以主控复核的限定为准。用户本次授权整理文档、更新 HANDOFF、提交并推送，没有选定实现路线，也没有授权迁移或部署。

## 接续时先读：双方结论与当前裁定

用户目标：修改 Saddle 或其中 Corral 并重新部署后，全部现存 agent（包括主控、普通、后台）自动接入最新 Corral，保留原进程、上下文、在途工作、输入输出、连接、身份和完成提醒，不要求用户逐个退出、交接或重建。

| 问题 | Claude Code 的调查结论 | 主控 Codex 的复核结论 |
|---|---|---|
| 当前重新部署能否让现存 agent 换核心 | 不能；旧 pen 没有交接入口，新 CLI 只是连接旧 pen | 源码和进程快照支持这一事实；界面更新不代表托管核心更新 |
| 当前旧会话首次过渡 | 建议自然结束，或支持交接的版本上线后重启一次 | 当前公开机制内没有找到无缝路径；两种建议均不满足原目标，也都未获用户选择。不能扩大为所有系统手段均已证明不可能 |
| 后续升级路线 | 推荐 pen 原地 exec，保存状态并继承 PTY、锁、socket 等描述符；预检和恢复失败回退 | 系统原语有依据，可优先验证；尚不能宣布完整方案可行，更没有实现。忙碌跳过、旧 hook、after 和失败语义仍须解决 |
| 其他路线 | 新 pen 接收描述符会失去父子关系；常驻 holder 抗崩溃，但新增一层 | 保留为备选比较，未选定；holder 自己的升级和在途状态也需要处理 |
| 工作量 | Claude 粗估中等、核心几百行及相应测试 | 仅粗估，未验证的生命周期问题可能改变工作量，不作为排期承诺 |

主控最终建议：先补齐完整设计，再考虑隔离实验；目前不直接进入正式实现。不降低「全部 agent」「在途工作连续」要求，不把旧 hook 长期保留或忙碌会话被跳过当成默认接受的结果。

尚未验证：macOS 实机切换时 PTY/连接是否连续，输入输出和回执是否不丢不重，切换耗时与请求超时，after 是否正确且只触发一次，恢复/回退失败时能否保住会话，以及真实 Claude/Codex 的反应。下文 §6 只是实验建议，没有执行。

下一会话先读 AGENTS.md、HANDOFF.md 和本文，再与用户讨论以上缺口。用户准备关闭这两个会话不等于批准现存全部会话重启，也不等于批准任何升级路线；进程信息都是调查时快照，接续时按公开命令重新核对。

## 主控核对补充（2026-10-03）

本报告可作为讨论材料；下面的限定优先于正文中「可行」「首次无解」「一定」等较强措辞，不代表方案获准实施。

- 已核实当前协议没有升级/状态导出入口，7755bb8 到 00af4f2 的 pen/hooks/state 源码未变。公开 `corral where saddle/main` 和进程路径再次确认主控 instance=faec2c2b00cb、agent PID=99462、父 pen PID=98891，pen 仍加载 7755bb8，命令链接指向 00af4f2。因此在当前公开机制、本轮禁止注入或改旧包的边界内，没有找到可用的首次无缝过渡路径；这不是对所有可能的系统手段作不可能性证明。保留旧会话或重启/resume 都不满足原始目标，尚未选择任何一种。
- Apple execve、SETEXEC、addinherit_np 文档支持「保留 PID 和指定描述符」这个系统原语，不等于本项目已证明完整无缝升级。尤其 §4 的忙碌时跳过不能保证全部 agent 完成升级；需继续解决在途状态与持续繁忙会话，不能当作用户已接受的范围缩减。
- 旧 hook 的绝对路径是当前绑定事实；仅升级 pen 仍有旧 helper、after/attach 代码驻留。路径固定不等于所有兼容桥接思路均不可能，但本轮没有设计或验证它们，也不允许改旧包。旧 hook 不能仅列成可选后续，就宣称所有核心已更新。
- §5.3 第 6 项「回退也失败时留在旧代码」不成立：exec 成功后旧映像已经消失，必须成功执行旧版本恢复入口才算回退。预检失败、系统调用返回错误、新映像恢复失败、新映像崩溃是不同阶段；SETEXEC 的具体失败分支能否保持所有资源也需隔离验证，不能一概保证原样继续。
- after 的连续性不只取决于 socket 是否保留；响应延迟、超时和状态一致性同样影响提醒。CLI 请求有 30 秒超时，ls 单 pen 为 5 秒；当前 after_worker 忽略 turn_end 错误，目标 status 错误会退出。需在隔离验证中覆盖，不能直接列为已排除阻断。
- 本轮仅核对公开状态、源码差异和官方资料，git diff --check 通过；未执行原型或运行测试，切换耗时、输入输出/回执完整性、真实 Claude/Codex 连续性及故障恢复均未验证。实现行数仅是调研者粗估，不作为工期承诺。原地 exec 可保留为优先验证候选，正式方案尚未定案。

证据标记：

- **【源码】** 本仓库源码或 git 历史可以直接看到。
- **【快照】** 本轮只读命令在 2026-10-03 取得，之后会变。
- **【系统】** 有官方系统资料支持（xnu 开源 man 页/内核源码、本机 macOS man 页、POSIX、man7），但本项目还没有实验验证。
- **【推测】** 合理推断，没有直接证据。
- **【未知】** 需要实验或外部信息才能判定。

## 0. 结论摘要

1. **现在做不到，只部署新版也不行。** 当前每个存活的 pen（`7755bb8` 到 `00af4f2`，这几版的 `pen.rs`、`hooks.rs`、`state.rs` 字节完全相同）都没有交出 PTY、导出状态或换程序的入口。部署新版之后，只有新 CLI 和新界面能通过不变的 proto 1 去连旧 pen，agent 本身仍由旧 pen 进程托管，代码不变。这正是任务排除的「新界面连接旧 pen」。【源码】
2. **首次过渡：现存主控在不重启的前提下无法被接管。** 主控 `faec2c2b00cb` 的 pen（快照 PID 98891，`versions/7755bb8`）只能按两种方式处理：要么一直留在旧 pen 上直到自然结束，要么等第一个支持交接的版本上线后，由用户挑时间重启一次（这一次会断）。其他现存 pen 也一样，包括本调研 agent 自己的 pen（00af4f2）。以后的部署才可能无缝。【源码＋系统＋快照】
3. **以后的部署可行，推荐「pen 原地 exec 到新版程序」。** macOS 上，exec 保留 PID、父进程、进程组、控制终端和信号屏蔽；没设 close-on-exec 的描述符原样保留。所以 PTY master、监听 socket、名称锁、已连接客户端都能跨过切换，agent 仍是同一个 pen 的子进程，退出码也拿得到。内存状态写成文件交给新程序。另外两条路线：把描述符移交给另一个新进程，在 macOS 上会丢掉父子关系和退出码（没有 subreaper）；常驻托管层最抗崩溃，但要新增一层并改设计。【系统，未验证】
4. **每个 agent 有几样东西终身留在旧路径。** agent 启动参数里写死的 hook 程序绝对路径、它的环境变量（`CORRAL_NAME/INSTANCE/EVENTS/HOME`）、agent 进程本身。因此在 agent 活着期间，instance、状态目录布局、events v1 格式和旧版本目录都必须保持。【源码＋快照】
5. **量级中等，没有新常驻服务。** 改动集中在 pen（状态序列化、恢复入口、升级请求）、CLI（批量升级命令）和部署步骤，另需先改 DESIGN。真正的阻断点：第 2 条（首次无解）；exec 成功后新程序如果直接崩溃，agent 会收到 SIGHUP，这段风险窗口不能降到零；macOS 实机上还没有验证过。

## 1. 当前真实机制与状态归属

### 1.1 进程拓扑

```text
corral start（短命） ─spawn+setsid─▶ corral __pen（pen；start 退出后被 launchd 收养，PPID=1）
                                        │ spawn + setsid + TIOCSCTTY
                                        ▼
                                      agent（会话首进程，控制终端 = PTY slave）
                                        └─ agent 自己的子进程（MCP、工具…）

Saddle TUI ─每个窗格一个自有 PTY─▶ corral attach NAME ─unix socket─▶ pen
corral status/send/reply/ls/stop ─短连接─▶ pen sock；并读写 $CORRAL_HOME/NAME/ 下的 meta/labels/events/cursor/exit
agent ─hook：<启动时版本目录>/bin/corral __hook EV─▶ 追加 events
corral send --after ─spawn+setsid─▶ corral __after（独立轮询 worker）
```

源码依据：`pen.rs:20-35`（pen 以 `__pen` 加 setsid 启动）、`pen.rs:659-686`（agent 的 setsid、TIOCSCTTY、信号复位）、`lib.rs:41-63`（私有模式都绑定 canonicalize 后的真实程序路径）、`viewer.rs:194-221`（Saddle 窗格里跑的是 `corral attach`，先核 instance）、`cli.rs:431-460`（after worker）。

快照（只读 `corral ls`、`corral where`、`ps`）：

- 主控 `saddle/main`：instance `faec2c2b00cb`，kind codex，agent PID 99462（`/usr/local/bin/codex`，PPID 98891，PGID 99462，tty ttys002）。pen 是 PID 98891，PPID 1，命令 `/Users/firegnu/.local/share/saddle/versions/7755bb8/bin/corral __pen`。codex 下面还有 node、MCP、`codex-code-mode-host` 等子进程，各自独立 PGID。
- 主控 agent 启动参数里的 hook 程序，按路径模式只抽取匹配部分：`/Users/firegnu/.local/share/saddle/versions/7755bb8/bin/corral __hook`。
- 入口：`~/.local/bin/{saddle,corral}` → `versions/00af4f2/bin/…`。本调研 agent（`saddle/dev-live-upgrade-1`）的 pen 是 PID 60300，跑的是 `versions/00af4f2`。
- 这与主控之前核对的 98891 / 7755bb8 / 99462 一致；快照会变，不能当成永远有效。

### 1.2 状态归属与部署后的去向

| 状态 | 归属 / 位置 | 源码依据 | 只切链接后 | 接管时必须处理 |
|---|---|---|---|---|
| pen 代码 | pen 进程映像（版本目录里的真实路径） | `pen.rs:20-35`、`lib.rs:41-44` | **留旧**：运行中的进程不会换代码 | 换映像（exec）或换进程 |
| agent 子进程与父子关系 | pen 持有 `std::process::Child`，用 `try_wait` 回收并写 exit.json | `pen.rs:151`、`437-442`、`576-582` | 不变 | 要拿真实退出码，就得保住父子关系 |
| PTY master | pen 持有；FD_CLOEXEC；运行时 O_NONBLOCK | `pen.rs:636-653`、`744` | 不变 | 最后一个 master 描述符一关，内核就向 agent 会话首进程发 SIGHUP（见 §3.1 依据），所以全程必须有人持有 |
| PTY slave / 控制终端 | agent 会话 | `pen.rs:665-671` | 不变 | 不注入就无法从外部更换 |
| 内核里的 PTY 状态（termios、窗口大小、缓冲） | 内核，跟着 master 存活 | — | 不变 | master 不断就保留 |
| 监听 socket `NAME/sock` | pen 持有 listener；Drop 时删掉 sock 文件 | `pen.rs:655-658`、`185` | 不变 | 描述符连续，新连接就在 backlog 里排队；路径一断，CLI 报 not_found |
| 名称锁 `NAME/lock`（flock） | pen worker 栈上的 `_lock`；Rust std 用 O_CLOEXEC 打开 | `pen.rs:592`、`state.rs:86-147` | 不变 | 锁一放，`ls` 就会清理这个目录（`state.rs:259-274`） |
| 已连接客户端 | pen 内存 `clients`：fd、没解析完的输入、待发输出（≤8 MiB）、模式、行列、先后顺序 | `pen.rs:128-136`、`157`、`529-539` | 不变 | 保留 fd 和状态；否则 Saddle 窗格显示 “attach exited… Select an agent on the left to reconnect” 并需要手动重选（`viewer.rs:259`），`corral attach` 不带 `--wait` 就直接退出（`attach.rs:187-193`） |
| 写者 | pen 内存 `writer` | `pen.rs:168`、`242-255`、`300-312` | 不变 | 序列化 |
| 输入队列与发送确认 | pen 内存 `input`（剩余字节、延时、回执客户端 fd）、`next_input` | `pen.rs:145-149`、`160-161`、`545-561` | 不变 | 在途 send 的回执绑定在客户端 fd 上 |
| 人类输入保护、最近发送 | `human`、`recent`（最多 10 条摘要） | `pen.rs:173-174`、`279-281`、`320-327`、`342-349` | 不变 | 丢了会影响 `human_active` 拒绝和 `last_input_source`（`cli.rs:141-158`） |
| 输出环、最后输出时间 | `ring`（512 KiB）、`last_output` | `pen.rs:158-159`、`521-527` | 不变 | 丢了 `read` 会缺内容，`idle_for` 会错 |
| 终端模式 | pen 内存 `Terminal`：标题、DEC 模式、kitty 栈、keypad、modify、截断的转义序列 | `terminal.rs:15-23` | 不变 | 丢了 `bracketed_paste` 会判错（`cli.rs:177-181`），多行 send 可能提前提交；attach 回放的模式也会错 |
| 停止流程 | `stopping/stop_steps/stop_at/stopped_by` | `pen.rs:162-164`、`175`、`189-219` | 不变 | 停止中不应升级 |
| meta / labels / exit 文件 | 状态目录 | `pen.rs:624-627`、`725-728`、`576-582` | 不变 | meta 里的 `version` 是 `CARGO_PKG_VERSION`，即 `0.1.0`（`crates/corral-core/Cargo.toml:3`），分不出是哪次构建 |
| events / cursor | events 由 hook 追加；cursor 由 CLI 读取方写（status 也写） | `hooks.rs:8-47`、`events.rs:159-209` | 新 CLI 下次调用就生效。注意 cursor schema 已从 2（7755bb8）升到 3（2df04b8 起），新旧读取方会互相判失效、从头重放：结果正确，只是多算 | 文件不随 pen 变 |
| hook | 写死在 agent 启动参数里：claude 是 `--settings` 内联 JSON，codex 是 `-c hooks.*`；pi/omp 的扩展文件在状态目录 | `hooks.rs:57-117`；快照 | **留旧**：agent 活多久，就调用旧版本 helper 多久 | 无法更换；旧版本目录必须保留，events v1 必须一直兼容 |
| agent 环境变量 | `CORRAL_NAME/INSTANCE/EVENTS/HOME` | `environment.rs:170-177` | **留旧** | instance 和路径必须保持 |
| after 提醒 worker | 独立 setsid 进程，跑的是启动它那一刻的 CLI 版本 | `cli.rs:431-460`、`572-602` | **留旧**，直到自然结束 | 查被等待方失败会被忽略，于是提前提醒；查目标方失败，worker 直接退出，提醒静默丢失（`cli.rs:580-584`）。所以 socket 不能中断 |
| Saddle 宿主 | 独立 TUI；窗格里是同包 `corral attach` 子进程；轮询 ls/status | `viewer.rs:194-221`、`agent_program.rs:4-6`、`corral.rs:104-140` | 重启 Saddle 后才用新代码；不重启，旧宿主就继续用旧 CLI 连 pen | 宿主和 pen 可以各自重启：已有测试 `crates/corral-core/tests/product.rs:181` 验证关闭再重开 TUI 后 PID/instance 不变 |
| `saddle ctl` 自身身份 | agent 环境里的 `CORRAL_NAME + CORRAL_INSTANCE`；宿主按公开列表核对 | `control.rs:38-49`、`app_control.rs:24-36` | agent 通过 PATH 调用的 `saddle` 链接，切换后立即是新版 | instance 一变，agent 里的 ctl 就失效 |
| Drover 链接、插件 | Drover 记录 instance；宿主传入 `SADDLE_AGENT_BIN` | `plugins/drover/src/links.rs:127-147`、`src/plugins/runtime.rs:379-380` | 跟着宿主重启 | instance 必须保持 |

### 1.3 只切链接就更新 vs 留在旧进程或旧路径

- **切链接后下次调用即新版**：所有短命 CLI 调用（start/status/send/reply/ls/stop…）、之后新建的 pen 与 agent、新起的 after worker、agent 里调用的 `saddle ctl`。
- **要重启宿主才更新**：Saddle TUI、Drover/Diff 插件进程、窗格里的 `corral attach` 子进程（它们用的是宿主同包的 corral，`agent_program.rs:4-6`）。
- **留在旧进程或旧路径**：每个存活 pen 的代码；它的 agent 的 hook helper、环境变量、启动参数；已经在跑的 after worker 和外部 attach 客户端。

## 2. 首次过渡：现存旧 pen 没有迁移协议

事实：

- `git diff 7755bb8 HEAD -- crates/corral-core/src` 只改了 `cli.rs`、`events.rs`（状态计时和 cursor 3）；`pen.rs`、`hooks.rs`、`state.rs` 与 7755bb8 字节相同。所以所有现存 pen 的行为一样。【源码】
- 旧 pen 只响应 socket 请求 `status/attach/read/send/keys/stop`，未知操作回 `bad_op`（`pen.rs:297-380`）。没有 exec、交出描述符或导出状态的入口。【源码】
- master 显式设了 FD_CLOEXEC（`pen.rs:651`）。锁、listener、客户端描述符由 Rust std 默认设 close-on-exec（本机 rust-src，rustc 1.96.0：`library/std/src/sys/fs/unix.rs:1360` 用 O_CLOEXEC；`sys/net/connection/socket/unix.rs:94-97`、`259-267` 在 macOS 上 socket/accept 之后调用 `set_cloexec`）。【源码】
- 旧 pen 启动之后不再 exec，也不加载任何代码。SIGHUP 被忽略（`pen.rs:743`）；其他信号按默认处理就是终止，终止后内核关掉 master，agent 收到 SIGHUP。正常返回或 panic 展开时，`Drop` 用 SIGKILL 杀掉 agent 进程组（`pen.rs:177-187`）。【源码＋系统】
- macOS 没有从外部复制别的进程描述符的接口：Linux 的 `pidfd_getfd` 是 Linux 5.6 专属，还需要 ptrace 级权限；macOS 也没有 `/proc/PID/fd` 可以重开。`/dev/ptmx` 每次打开都是一对新的 PTY，不能重开已有的 master。打开 slave（快照里的 `/dev/ttys002`）只会和 agent 抢输入，属于禁止的强行夺取。把 agent 的终端换掉要靠 ptrace 注入（reptyr 一类，Linux 专属），既被任务禁止，在 macOS 上也不适用。【系统】

结论：**只部署新版不能接管现存旧 pen。** 主控 98891/99462，以及其他所有现存 pen，都会一直由各自的旧 pen 进程托管到 agent 结束。新 CLI 和新宿主还能操作它们，只是因为 proto 1 没变，而这就是「新界面连接旧 pen」。

可选的处理方式（由用户决定）：

- (a) 让现存会话在旧 pen 上自然走完。代价：在它们结束之前，新版本必须继续和旧 pen 保持 proto 1 兼容，`versions/7755bb8` 这类旧目录不能删（主控的 hook 指向它）。
- (b) 第一个支持交接的版本部署后，用户挑一个空闲时间点，把这些会话重启一次（按 HANDOFF 接续，或者 resume）。这一次会断，以后不再需要。

## 3. 可行路线比较（只针对首个支持交接的版本之后）

### 3.0 先排除的

- 只在 Linux 上可用：`pidfd_getfd`、`PR_SET_CHILD_SUBREAPER`（Linux 3.4）、CRIU、reptyr（ptrace）、systemd 文件描述符仓库。macOS 都没有。
- 任务明确不算数的：新界面连接旧 pen；结束 agent 再 resume。

### 3.1 A：pen 原地 exec

做法：pen 收到升级请求 → 等到静默点 → 把内存状态写进状态目录里的私有文件 → 保留 master、listener、锁、客户端和状态文件的描述符 → 以恢复模式 exec 新版本目录里的 `corral` → 新映像读状态、重建结构，继续原来的 poll 循环。

系统依据：

- Apple execve(2)：没有设 close-on-exec 的描述符在新映像里保持打开，并且 “unaffected”；新映像继承进程 ID、父进程 ID、进程组 ID、控制终端、信号屏蔽；被忽略的信号继续被忽略；execve 失败会返回 −1 给调用者。【系统】
- flock(2)：“Locks are on files, not file descriptors”，描述符不关，锁就在。【系统】
- 所以 agent 仍是同一个 PID 的子进程，新映像可以 `waitpid` 拿到真实退出码。切换期间 agent 如果退出，会成为僵尸，等新映像回收，状态不丢。【系统，未验证】
- Apple 扩展 `POSIX_SPAWN_SETEXEC`：posix_spawn 表现为 “a more featureful execve”。配合 `POSIX_SPAWN_CLOEXEC_DEFAULT` 和 `posix_spawn_file_actions_addinherit_np`，可以只继承列出的描述符，并在新映像里清掉 FD_CLOEXEC。这样旧映像不用先改自己的描述符标志，spawn 失败就原样继续。【系统】
- pen 主循环是单线程 poll（`pen.rs:435-567`），不涉及多线程 exec 的问题。【源码】

切换窗口内的输入输出：

- master 不关，PTY 不会挂断。agent 的输出暂存在内核 PTY 缓冲里；缓冲满了 agent 写操作阻塞，会短暂停顿但不丢数据。【推测，需实验】
- 新连接在 listener backlog 里排队（Rust std 在 Apple 平台上 listen backlog 传 −1，源码注释说明会被截为默认 128，`os/unix/net/listener.rs:81-92`、`:106`）。已连接客户端的数据留在 socket 缓冲里。CLI 请求超时 30 秒（`state.rs:166-168`），`ls` 每个 pen 5 秒（`state.rs:244`）。【源码】
- 窗口时长估计在毫秒到几十毫秒之间。【未知，需测】

失败语义：

- exec 之前的检查失败，或 execve/posix_spawn 返回错误：原进程继续，会话无损。【系统】
- 新映像能运行但不接受这份状态：可以再 exec 回旧路径。旧版本目录本来就因为 hook 要保留。这要求从第一个支持交接的版本起，每一版都带恢复入口。【建议】
- **新映像在 exec 之后直接崩溃**（abort、段错误、被 kill）：进程消失 → master 关闭 → agent 收到 SIGHUP → 会话丢失。这个风险只能用预检缩小，不能归零。它和今天「pen 一崩溃 agent 就没了」是同一类风险（`pen.rs:177-187`）。
- 预检：exec 之前，先用新程序以只读 probe 模式解析一份状态副本，报告它支持的状态 schema。不通过就不升级。【建议】

依据链：xnu 的 `ptcclose`（primary 端关闭）调用 `l_modem(tp, 0)`（`bsd/kern/tty_dev.c:588`）；`ttymodem` 在掉线分支里对会话首进程 `psignal(..., SIGHUP)`（`bsd/kern/tty.c:2032-2042`）。termios(4) 的 Modem Disconnect 节写的是同一语义。agent 由 `setsid + TIOCSCTTY` 成为会话首进程（`pen.rs:670`）。【系统＋源码】

### 3.2 B：把描述符移交给另一个新 pen 进程

做法：旧 pen 起一个新 pen，经 unix socket 用 SCM_RIGHTS 传 master、listener、锁、客户端描述符和状态；新 pen 确认后，旧 pen 退出，且不能走 `Drop`。

- macOS unix(4) 支持 SCM_RIGHTS。收到的是 dup 出来的描述符，“Per-process descriptor flags … are not passed”，要自己重设 CLOEXEC；描述符号会变，客户端和回执的对应关系要重新映射。【系统】
- **父子关系丢失**：agent 的父进程仍是旧 pen。旧 pen 一退出，agent 就被系统进程收养（POSIX `_exit`；快照里 pen 自己 PPID=1 就是同一现象）。macOS 没有 subreaper，新 pen 无法 `waitpid`。kqueue `EVFILT_PROC` 的 `NOTE_EXIT` 能观察非子进程退出，但 `NOTE_EXITSTATUS` 写明 “Valid only on child processes”，所以退出码拿不到，stop 回执和 exit.json 里的 `exit_code` 只能是 null。如果让旧 pen 留下来专门收尸，旧代码就长期驻留，每升级一次多一层。【系统】
- agent 的 PPID 变成 1 之后，Claude/Codex 会不会有异常反应。【未知】
- 旧 pen 必须绕过 `Drop`：它会删 sock 文件、SIGKILL agent（`pen.rs:177-187`）。【源码】
- 优点：在旧 pen 放手之前，新 pen 可以拿真实描述符完整初始化，失败时旧 pen 原样继续。提交前的回滚能力最强。

### 3.3 C：常驻会话托管层

做法：每个 agent 配一个极小、协议冻结的 holder。它负责 fork agent、持有 master 和锁、`waitpid`，并能把 master 复制给逻辑进程。可升级的 pen 逻辑作为另一个进程连到 holder 上。

- 优点：逻辑进程崩溃或升级都不会关掉 master，agent 不会挂断；父子关系在 holder 上，退出码保留；升级失败可以重启旧逻辑。
- 限制：holder 自己还是换不了代码，要换只能走 A。终端模式、在途输入这些状态，仍要在新旧逻辑进程之间交接，逻辑进程崩溃时它们会丢（bracketed paste 判定可能出错）。每个 agent 多一个进程。这是新架构，要改 DESIGN：「pen 持有 agent PTY、socket 和名称锁」（`docs/Corral核心Rust集成设计.md:70`），以及 §6「不热替换旧 PTY」（同文件 `:106`）。现存 agent 同样迁不进来。

### 3.4 对照

| | A 原地 exec | B 描述符移交 | C 托管层 |
|---|---|---|---|
| macOS 可用 | 是【系统】 | 是【系统】 | 是（自己写，不依赖 tmux/zellij） |
| pen PID 与 agent 父子关系 | 都保留 | pen PID 变；agent 被 launchd 收养 | 由 holder 保留 |
| 真实退出码 | 保留 | 丢失 | 保留 |
| 客户端连接 | 保留，描述符号不变 | 保留，需要重新映射 | 保留，经逻辑进程交接 |
| 提交前失败 | 原样继续 | 原样继续，验证更充分 | 原样继续 |
| 提交后新代码直接崩溃 | 会话丢失 | 会话丢失 | agent 存活，状态降级 |
| 旧代码残留 | 无（hook 除外） | 无，或留一个收尸进程 | holder 永久留着 |
| 改动面 | pen 内部 | pen 内部加两阶段协议 | 新增一层并改设计 |
| 能接管现存会话 | 不能 | 不能 | 不能 |

## 4. 推荐路线（建议，不是设计）

推荐 **A：原地 exec，加预检与回退**。要点：

1. pen 新增请求 `upgrade`，proto 仍为 1。旧 pen 对它回 `bad_op`（`pen.rs:380`），所以可以无副作用地探测。CLI 新增一个批量命令（名字待定，例如 `corral upgrade --all`），遍历 `ls`：主控、普通 agent、宿主没显示的后台 agent 都在里面。逐个返回 upgraded / needs_restart（旧 pen）/ busy / failed。
2. 静默条件：不在停止流程中；输入队列为空；没有处在握手中或等待回执的客户端。等不到就放弃这一个，绝不强行切。
3. 目标程序由新 CLI 传入：canonicalize 之后的绝对版本路径。pen 先以子进程方式 probe 它：解析状态副本，核对状态 schema 和契约版本。
4. 状态内容：§1.2 里 pen 内存的全部字段，加上自己旧程序的路径（用于回退）。写进状态目录的 0600 文件，以继承描述符的方式交给新映像。
5. exec 用 `posix_spawn(SETEXEC | CLOEXEC_DEFAULT)` 加 `addinherit_np`，只继承 master、listener、锁、客户端、状态文件、日志。失败就原样继续。
6. 恢复：在完全恢复之前，新映像不要构造那个会杀 agent 的 `Drop`。恢复失败就 exec 回旧路径。成功后改写 meta 里的构建标识，建议写 git revision 或 BUILD 哈希，而不是只写 `0.1.0`。
7. 部署：切完命令链接之后执行批量升级；`bin/corral` 哈希没变就跳过。Saddle 宿主照旧由用户重启，窗格会按已记住的 instance 重新接上。
8. 兼容规则：pen 要继续接受旧 CLI、旧 attach、旧 after worker 的 proto 1；agent 活着期间，events v1 和状态目录布局不变；还有 hook 引用的旧版本目录不删。
9. 可选的后续：新建 agent 的 hook 要不要改成指向稳定链接（见 §5 的取舍 5）。

## 5. 实现量级、阻断点与需要用户决定的取舍

### 5.1 量级（推测）

- pen 状态序列化/恢复、自己的 `waitpid` 封装：约 300–500 行。
- 升级请求、probe、spawn-exec、回退：约 150–250 行。
- CLI 批量命令及结果：约 100 行。部署记录、脚本步骤：少量。
- 测试（两个临时版本目录之间升级、故障注入）：约 400–600 行。
- 合计中等，一个分支能做完，不需要常驻服务。动手前要先改 DESIGN §4、§6、§8.1。

### 5.2 真正的阻断点

1. 现存 pen 无法接管：硬阻断，首次一定要断一次（§2）。
2. exec 之后新程序直接崩溃就会丢会话：风险窗口不能为零（§3.1）。
3. agent 写死的 hook 路径和环境改不了：协议、目录布局要长期兼容，旧版本目录会越积越多（§1.2）。
4. macOS 实机上一项都还没验证：PTY 不挂断、客户端连续、窗口时长、Claude/Codex 对短暂输出阻塞的反应（§6）。

不算阻断的：Saddle 宿主重启（已经支持接回同一实例）；after worker（listener 连续就不受影响）；cursor 混版重放（结果正确，只是多算）。

### 5.3 需要用户决定

1. 现存会话：在旧 pen 上自然走完，还是在第一个支持交接的版本上线后重启一次？哪个时间点？
2. 路线：A（推荐）、C（更抗崩溃，但新增一层并改设计）、B（丢退出码）。
3. 触发方式：部署步骤里显式执行，还是让 pen 自动察觉入口变化？
4. agent 忙的时候：只在静默点升级（可能跳过忙的 agent，下次再试），还是连在途输入一起交接（更复杂）？
5. hook 路径：保持绝对版本路径（hook 终身不升级，目录永不复用），还是改成稳定链接（hook 随部署升级，但链接切换的一瞬间会出现新 hook 配旧 pen 的组合）？
6. 回退也失败时怎么办：放弃这一个、让它留在旧代码上，还是重试？
7. 设计文档 §4、§6「不热替换旧 PTY」的表述要改写。

## 6. 下一步最小隔离验证（建议，本轮未执行）

用原型分支构建两个临时版本目录 A、B（构建号能区分）。在隔离的 `HOME`、`CORRAL_HOME`、`SADDLE_RUNTIME_DIR` 下，用合成 agent（每 10 ms 打印一个递增计数，记录自己的 PID/PPID/PGID/控制终端，收到 SIGHUP 时写标记文件）。先接一个 attach 客户端，让它开启 bracketed paste 模式，挂一个 `send --after` 和一个长 `wait`，然后触发 A→B 升级。要证明：

1. agent 的 PID、PPID（等于 pen PID）、PGID、会话、控制终端都不变；没有收到 SIGHUP；计数在 attach 流和 `read` 里连续，不缺不重。
2. pen PID 不变，可执行文件变成 B，meta 构建标识更新；instance、started、labels 不变。另开紧密循环跑 `ls`，确认从未出现 starting 或目录被清理；另一个紧密循环 connect，确认从未被拒。
3. attach 客户端不断线，写者身份和窗口大小保留；窗口期间输入的内容只交付一次；在途 send 只收到一个回执；`recent_sends`、`human_active`、`bracketed_paste` 都保留。
4. after worker 只在真正的轮次结束时触发；`wait` 结果正确。
5. 故障注入：目标程序不存在或不可执行 → 报错，现状不变；目标拒绝这份状态 → 回退到 A（PID 不变，agent 存活）；目标 exec 后立即 abort → 记录 agent 收到 SIGHUP，用来量化 §5.2 第 2 条的风险。
6. 升级之后 stop 返回真实退出码（跨 exec 的 `waitpid` 有效）。
7. 测出切换窗口时长。
8. 合成场景都通过之后，再用自建的 `saddle/test-*` Claude/Codex 会话各跑一次，确认 PTY 输出短暂阻塞和窗口期对真实 agent 无副作用。不碰用户的会话。

## 7. 依据清单

源码（本仓库 `6a51ba8`）：`crates/corral-core/src/pen.rs`、`state.rs`、`cli.rs`、`hooks.rs`、`environment.rs`、`events.rs`、`attach.rs`、`terminal.rs`、`lib.rs`；`src/viewer.rs`、`src/corral.rs`、`src/control.rs`、`src/app_control.rs`、`src/agent_program.rs`、`src/plugins/runtime.rs`；`plugins/drover/src/links.rs`；`scripts/package.sh`；`crates/corral-core/tests/product.rs:181`；`docs/Corral核心Rust集成设计.md` §4、§6、§8.1、§10。各处行号见正文。旧实现用 `git show 7755bb8:<path>` 和 `git diff 7755bb8 HEAD -- crates/corral-core/src` 核对。

官方系统资料（xnu 固定在提交 `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea`；本机 `man` 页内容一致）：

- execve(2)：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man2/execve.2#L114-L165>
- kqueue(2) `EVFILT_PROC`、`NOTE_EXITSTATUS`：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man2/kqueue.2#L544-L550>
- unix(4) SCM_RIGHTS：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man4/unix.4>
- posix_spawnattr_setflags(3) `POSIX_SPAWN_SETEXEC`、`POSIX_SPAWN_CLOEXEC_DEFAULT`：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man3/posix_spawnattr_setflags.3#L106-L130>
- posix_spawn_file_actions_addclose(3) `addinherit_np`：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man3/posix_spawn_file_actions_addclose.3>
- flock(2)：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man2/flock.2#L103>
- termios(4) Modem Disconnect：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/man/man4/termios.4#L705>
- xnu `ptcclose`：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/tty_dev.c#L534-L590>；`ttymodem` 掉线发 SIGHUP：<https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/tty.c#L2032-L2042>
- POSIX `_exit`（子进程改由实现定义的系统进程收养）：<https://pubs.opengroup.org/onlinepubs/9799919799/functions/_exit.html>
- Linux 专属对照：pidfd_getfd(2) <https://man7.org/linux/man-pages/man2/pidfd_getfd.2.html>；PR_SET_CHILD_SUBREAPER <https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html>
- Rust std 的 close-on-exec 默认（本机 rust-src，rustc 1.96.0）：`library/std/src/sys/fs/unix.rs:1360`、`library/std/src/sys/net/connection/socket/unix.rs:94-97`、`:259-267`、`library/std/src/os/unix/net/listener.rs:81-92`、`:106`。

Apple 开发者站上的在线 man 归档（developer.apple.com）本轮网络连不上（TLS 错误），所以改用上面 Apple 官方开源的 xnu 源和本机 macOS 自带 man 页。

本轮只读命令：`corral ls`、`corral where saddle/main`、`ps -o … -p 99462/98891/60300`、按父进程列出主控 agent 的子进程、`ps -o command= -p 99462 | grep -o '<hook 路径模式>'`（只输出匹配的 hook 路径）、`ls -l ~/.local/bin/{saddle,corral}`、`versions/{7755bb8,00af4f2}/BUILD.txt` 前三行。没有读取 Corral 私有状态目录，没有向任何 agent 送话、按键或 attach。
