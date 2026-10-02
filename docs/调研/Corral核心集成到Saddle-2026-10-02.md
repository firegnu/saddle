# Corral 核心集成到 Saddle：源码调研

日期：2026-10-02。状态：**调研结论与建议，尚非实施设计或迁移批准**。

用户要求：本轮不走任务流程；开一个 Claude Code 会话留给用户自行布置任务；主控调研最后一层 Corral infra 纳入 Saddle，使 Saddle 作为一个应用提供完整能力，同时保留原 Corral/corral-dispatch 独立使用。

本轮静态核对 Saddle `b4ab2f2`、Corral `6923da1`。没有改功能、构建、运行测试、切换后端、修改 Corral 仓库或真实配置/队列；没有选择遥测。新会话 `saddle/claude-1`（`cb7fea1096b7`）仅收到等待用户的指令，已回复就绪，不承担本次调研。

## 1. 结论

可行，符合此前“外围先收敛，Corral 最后迁入”的方向。建议在 Saddle 仓库内实现可独立构建和测试的 Rust agent-core，随 Saddle 发布，由统一入口启动。它是必备基础设施，不是依赖 TUI 存活的可选进程插件。

保留 Corral 现有的每 agent 独立后台 pen 进程。Saddle 界面退出，agent 仍运行；重新打开可以接回。不需要为统一产品另外设计一个中央 daemon。

原 Corral 仓库、独立命令及其中的 corral-dispatch 可以保持不动。Saddle 自带新的运行实现与已有 Dispatch 插件，日常使用不再依赖兄弟 checkout 或 Python Corral 安装。两套实现后续不会自动同步功能；原仓库的独立维护和新旧进程互操作要分开看待。

```text
Saddle 产品（同版本构建、安装、升级）
  ├─ TUI / headless agent 操作入口
  │    ├─ 可选遥测组合层 → Saddle 遥测
  │    └─ agent-core → 独立 pen → PTY → Claude Code / Codex / pi / omp
  ├─ Dispatch 插件：JEV 建议、分派技能
  └─ Drover 插件：Tasks 流转，经公开 agent 接口操作

原 Corral / corral-dispatch 仓库：独立保留，不依赖 Saddle
```

agent-core 不依赖 Tasks、Dispatch、JEV、遥测或 TUI。记录上下文仍由上层处理，不能下传成为 pen 的工作前提。外部 coding agent 程序及其登录仍由用户提供，不属于这次内置范围。

## 2. 当前还依赖 Corral 的地方

| 位置 | 当前行为 | 集成时需要调整 |
|---|---|---|
| `src/corral.rs`、`src/config.rs` | 配置一个外部程序，调用公开 JSON 命令 | 默认指向产品自带运行入口；保留显式外部后端的兼容方案 |
| `src/viewer.rs`、`src/pty.rs` | viewer PTY 执行 `corral attach` | 接入新入口；保留终端显示层，不同时重做终端引擎 |
| `src/agent.rs` | 可选遥测包装，仅支持 start/send/reply | 对接内置执行路径，保留现有参数、回执、取消与一次执行语义 |
| Drover `main.rs`、`api.rs`、`telemetry.rs` | 普通操作直接调用 Corral；记录时调用 Saddle 包装入口 | 记录与不记录两条路径使用同一个选定后端，继续只走公开接口 |
| Corral / dispatch 技能及项目指引 | agent 操作仍使用 Corral 命令 | 随 Saddle 提供匹配版本的操作指引；不静默覆盖用户管理的独立技能 |
| 安装与发布 | 本机部分程序/插件路径仍来自开发目录 | 发布物包含运行核心、hook 资源与官方插件包，完成脱离 checkout 的安装核验 |

`saddle agent` 目前不是完整的 Corral 替代命令；主入口只把它转交给采集包装层。既有 `--corral` 和配置项是**单个可执行文件路径**，不能直接把值改成字符串 `saddle agent`。需要明确程序路径与参数前缀，或者产品内的专用 helper；具体语法待设计。

插件已有通用 `SADDLE_HOST_BIN` 绝对路径，能复用它发现宿主。不要给 Drover 增加读取 agent-core 状态目录的特权。

## 3. 真正需要迁入的能力

Corral 不是一个 start/stop 包装。当前 Python 核心 23 个文件约 2,438 行，另有 pi/omp TypeScript hook；代码量较小不等于迁移只涉及少量行为。关键部分是：

| 能力 | 必须保留的语义 | 主要源码 |
|---|---|---|
| 身份、启动、登记 | 名称锁、实例身份、私有目录权限、启动回执、同名重建不混淆 | `spawn.py`、`registry.py`、`paths.py` |
| PTY 托管 | 脱离界面存活、进程退出回收、按 agent 类型分阶段停止 | `pen.py`、`agents/` |
| 输入和交付 | idle 判定、人类操作避让、文本确认、未知结果不补发 | `pen.py`、`cli.py` |
| 接入终端 | 首个接入者可写、后续只读、尺寸及写者交接、慢客户端不阻塞 agent | `attach.py`、`pen.py`、`protocol.py` |
| 终端模式 | bracketed paste、alternate screen、Kitty/modifyOtherKeys 等模式恢复 | `termmodes.py` |
| 状态与回复 | hook 事件、实例/会话过滤、事件版本、游标；不靠屏幕文本猜完成 | `events.py`、`hook.py`、agent 适配器 |
| 环境和 hook | 登录 shell 重建、父 agent 身份隔离、每次启动注入、不修改全局 agent 配置 | `env.py`、`agents/` |
| 延后发送 | `send --after` 的独立等待角色，绑定两个实例；pending 不等于成功发送 | `cli.py` |
| 公开契约 | CLI/JSON/退出码、读写边界、旧协议兼容、未知 agent 行为 | `docs/CONTRACT.md` |

公开接口还包括 keys/status/wait/reply/where/ls/read/attach/stop/guide 等，不应因当前采集只覆盖三个操作就缩小迁移范围。

### 实现形式建议

- 自带后台运行角色，例如 pen、hook、延后发送 worker，由产品内部启动；名称只是示意，不作为用户必须操作的新服务。
- 从独立 worker 入口建立进程会话、管理句柄，避免在多线程 TUI 内直接照搬 Python double-fork 后执行复杂逻辑。
- Claude/Codex 现有 hook 使用 `/usr/bin/python3 -I -S`。若目标是去掉 Python Corral 依赖，就需要一并替换 hook 执行器。pi/omp 的 TypeScript hook 是对方扩展资源，可以继续随产品提供。
- hook 使用与存活 agent 匹配的稳定路径/版本资源；不能升级时删掉仍在运行的 agent 要调用的 helper。
- Corral 会重建登录环境，父进程临时 PATH 不可靠；内置 helper 路径须在这个边界之后明确注入。保留父 agent 身份隔离。
- 先保持现有公开命令/子进程边界，减少对采集、取消、输出流的同时改动；是否改为进程内调用另行论证。

## 4. 独立性与兼容性必须分开

**原仓库不动，就能保留独立产品；但不能因此保证旧 Python CLI 可以安全操作新 Rust pen。** 公开契约只约束命令输出，不保证两套实现共享锁、登记文件、事件游标的内部格式。

现有 Corral protocol/event format 都是 v1，并具有版本拒绝逻辑，这是兼容设计的基础，但还不足以证明共享运行目录安全。本轮没有做互操作验证。

建议采用下面的目标和过渡原则：

1. 已有用户 agent 原地存活，不要求关闭、不迁移其 PTY、不重启其进程。
2. 过渡期保留公开旧 Corral 后端访问旧会话。不能在一次发送结果未知后改走另一个后端重发。
3. 新启动会话使用自带核心；同一会话后续操作必须固定后端与实例，列表、viewer、Drover、技能不能各自猜路径。
4. 不顺手迁移 `CORRAL_HOME`。若新旧实现共用它，先明确锁/登记/事件/游标的互操作约束并验证；兼容读写限于运行核心内，上层仍不读内部文件。
5. 如果共享目录不能在合理范围内安全实现，应在设计中明确隔离运行目录、聚合列表和名称冲突的方案，再让用户确认；不要悄悄建出两个互相看不见的 agent 世界。
6. 不覆盖当前独立 `corral` 命令来假装完成切换。是否提供兼容命令、旧 Python CLI 是否支持管理新 pen，需给出明确支持矩阵。

这里最需要下一步设计的是：**新旧 pen 共存期间，如何发现、选定后端和保证锁/身份一致，以及何时能够完全移除旧 CLI 的过渡依赖。** 它属于迁移桥接问题，不需要让原 Corral 反向依赖 Saddle。

原仓库内的 corral-dispatch 与 Saddle 已安装的同名技能也不能混为一谈：仓库可独立保留；全局同名安装目标仍存在所有权冲突，不能为了兼容自动改写用户的技能目录。

## 5. 建议的实施顺序（尚未派发）

1. 定接口与共存设计：完整命令映射、旧包装参数兼容、状态目录/版本边界、升级资源存活策略。
2. 在隔离运行目录实现 Rust 核心：登记与进程生命周期、PTY/终端、hook 状态、send/wait/after；复用公开契约场景作为验收依据，不把 Python 内部导入测试直接算成 Rust 通过。
3. 统一消费者：Saddle 看板/viewer/headless、Drover、技能使用同一个后端选择结果；遥测仍是上层可选组合，Tasks 流程不变。
4. 成套打包并验证升级：干净环境不需要另装 Corral/Python，退出 TUI 后 agent 存活，升级不破坏旧 agent/hook。之后才安排用户日常安装切换。

工作量主要集中在第 1、2 步，是底层运行机制迁移，不能按普通 UI 或薄插件接入估计。无需附带中央服务、远程管理、任务引擎、终端渲染重写或新 UI 设计。

以上是后续范围建议，不是本轮测试计划执行记录。本轮未证明 Rust 对等、混合运行或发布包可用；这些应在批准具体设计后使用隔离目录、自建测试 agent 验证，不拿用户会话做试验。

## 6. 阅读依据

- 先前产品方向：[统一仓库与运行入口调研](Saddle统一仓库与运行入口-2026-10-01.md)。该文部分外围现状已过期；当前遥测、Dispatch 插件及 Drover 已完成接入，以最新源码和 DESIGN 为准。
- 当前依赖边界：[DESIGN](../DESIGN.md)，尤其 Corral 不依赖遥测、插件消费通用接口的决定。
- Saddle 当前接点：`src/{main,agent,corral,viewer,pty,config}.rs`、`src/plugins/runtime.rs`、`plugins/drover/src/{main,api,telemetry}.rs`、`plugins/drover/README.md`。
- 原 Corral：`../corral/docs/CONTRACT.md`、`../corral/src/corral/` 的上述实现及 `../corral/corral-dispatch-skill/`。本轮仅读取。

## 7. 用户补充：迁移无感、保留命令习惯与底层独立性

用户进一步明确：“集成进来之后，我完全感受不到集成进来了，和现在的感觉一样。关闭saddle也可以attach到某一个agent或者重新打开saddle之后agent该跑还是跑状态不变”；要求调查 `corral start` 的迁移用法，并再次强调 Corral 是最底层 infra，不得反向依赖。

这收紧了第 4 节的方案空间：不能把要求用户手工选择两套 agent 世界、改用新名字或重启所有会话，作为满足本次目标的交付。技术兼容方案尚待验证，但这些产品约束已经明确。

### 7.1 生命周期：继续原会话，不是恢复一个替代进程

源码补充核对：`spawn.py` 通过独立会话和后台 pen 脱离调用者；pen 持有 agent 的 PTY，`attach.py` 仅连接 socket。attach 断开时 pen 的 `on_client` 只移除客户端。Saddle 的 viewer PTY 托管的是 attach 客户端，不是业务 agent；其销毁不能传播为停止 pen。

因此迁移目标是：

- 关闭/重开 Saddle 或退出 attach，都不重建、不停止 agent，原 instance、对话、工作目录、后台执行连续。
- 不运行 TUI 时也能 start/status/send/reply/attach/stop；不能改成依赖 `saddle ctl` 和一个活着的界面实例。
- 重新打开后查询同一实例的当前状态。状态可以因为工作自然完成而从 working 变 idle；“状态不变”是生命周期不被界面打断，不是冻结显示值。
- 保留多端 attach 的原规则：第一个接入者可写，后续只读，写者离开后按既有规则交接。不能因为多开一个窗口就抢输入或重新派发。

现有 `src/terminals.rs::restore` 保存/还原布局和 agent 引用，先放置占位内容，不会在这里重新 start agent；`src/viewer.rs` 接入时还核对实例。布局恢复和 agent 存活是不同职责，本次不顺手重新设计自动接入或界面恢复策略。

### 7.2 `corral start` 怎么用：建议保留原命令，而非迫使用户改习惯

建议 Saddle 安装包同时提供 TUI 主程序与独立的 Corral 兼容 CLI。一个产品可以包含多个可执行入口，用户只安装一次。独立 CLI 链接底层 Rust core，不链接 Saddle TUI、插件或遥测业务。

下面是**拟保留的原用法**，不是已经切换到 Rust 实现：

```sh
corral start my-project/main --cwd /absolute/project -- claude
corral status my-project/main
corral attach my-project/main
# Ctrl-] 离开接入；agent 继续运行。
```

参数、`--` 后原样传递、JSON、退出码、实例身份、send 确认和人类输入保护均按现有契约保持。`saddle agent --record-context … -- start/send/reply …` 仍是上层可选记录入口，普通 `corral start` 不因此开始遥测。是否再提供等价 `saddle` 子命令可以后定，不能取代原命令兼容要求。

依赖建议：

```text
Saddle TUI / 可选遥测组合 / 插件公开调用
                       ↓
                 Corral CLI 适配层
                       ↓
                 独立 Rust core
                       ↓
                  OS / PTY / agent
```

目录归 Saddle 仓库、随其安装发布，不等于底层依赖上层。Rust core 与 CLI 应能在不构建/启动 TUI、不加载插件、不打开遥测库的情况下工作。内部 pen/hook/after helper 也只属于底层运行包，不能回调 TUI 才能工作。

### 7.3 原命令路径与新旧混用：需要落实到安装和状态协议

本机 `~/.local/bin/corral` 目前软链接到原仓库 `corral/bin/corral`，脚本再加载该仓库 Python 源码。保持此链接就仍在使用原实现，不等于“Corral 已内置”；擅自换链接也不是安全迁移。

拟定部署方向是：先保留并验证原入口，新包准备好后在获准的部署步骤中备份、切换常用 `corral` 入口到随产品提供的 CLI。原仓库及它的 `bin/corral` 保持可单独运行；日常 PATH 选中的版本与仓库是否独立存在是两回事。研究期间不执行替换。

若要求旧 CLI 和新 CLI 都能管理同一批会话，必须覆盖以下组合，不能只证明新新组合：

| 客户端 | 既有 Python pen | 新 Rust pen |
|---|---|---|
| 原 Corral CLI | 原有行为基线 | 需验证，不能由 JSON 兼容推断 |
| 产品自带 Rust CLI | 无感接续的必要验证 | 新运行核心的必要验证 |

补读源码发现：`ls` 并非无副作用查询，遇到失联栏位会尝试获取锁并清理内部文件；`start` 同样依赖 flock 与锁文件 inode 核对；`status` 等查询还会写事件 cursor（当前 cursor v2、event format v1）。所以单纯实现 socket v1 不足以安全共存。锁生命周期、清理、meta/labels/exit、事件及 cursor 的读写行为必须成套兼容，或者由有证据的桥接方案隔离处理。

推荐优先论证保持现有会话命名空间和协议的互操作路径。当前可以确认没有架构上必须反向依赖的理由；不能在尚无 Rust 实现和互操作证据时宣称这四格已通过。如遇到必须修改原 Corral 的缺口，应先报告而不是先改原仓库。

### 7.4 下一步设计必须给出的证据

仅限与上述要求直接相关：CLI 兼容映射；旧会话接续与四格互操作方案；锁/事件格式和升级资源生命周期；在 TUI 完全不运行时 start/attach 的独立路径；关闭界面不停止 pen；所有消费者选择同一运行后端；核心构建依赖不含上层模块。这是待设计/验证清单，本轮没有执行混合会话测试，也没有停止用户界面或 agent。
