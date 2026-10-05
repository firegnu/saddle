# dispatch 插件及通用接入接口设计（遥测03）

> 已退役（2026-10-05）：Saddle 已删除遥测、Drover 和整个插件系统。本文只保留设计/验证历史，不再作为当前操作规矩或安装指引；旧磁盘数据不读、不删。当前功能与命令见 [README](../README.zh-CN.md)，派发路由由 ranch 提供。

2026-10-01。状态：主控静态复核通过，可按第8节串行实施；管理页线框须在界面实施前由用户确认，HTTP库具体版本在路由实施前核实。按任务《遥测03-插件接口设计》基于源码静态核对写成；没有实现、编译、运行测试、执行旧 route.py、发 HTTP 请求或改动任何安装。下文“已有”指当前源码中的能力，“新增/建议”均未实现。依据为 DESIGN.md 末尾“dispatch 可选插件与遥测职责重新确认”“dispatch 首版项目接入说明”及其后续授权、[接入调研](调研/Saddle-dispatch插件与技能资源接入方案-2026-10-01.md)、[遥测接口契约](任务遥测接口契约.md)。

## 0. 结论

- dispatch 作为同版本编译进 Saddle 的 **core plugin**：业务、路由规则、skill/模板资源和接入说明都在 `plugins/dispatch/`；宿主只提供“内置插件目录 + 启停 + 无 TUI 命令入口 + 受控采集接口 + 资源安装器”五项通用能力。现有外部进程插件、协议、SDK、Drover、`saddle ctl plugin` 都不改。
- 依赖无环：`saddle → saddle-dispatch-plugin → saddle-core-plugin(新接口 crate)`，`saddle → saddle-core-plugin`。插件不依赖宿主应用 crate；宿主库模块也不 import dispatch，只有 `src/main.rs` 组装目录（第 2 节）。
- 无 TUI 入口：`saddle plugin run dispatch route`。stdin 是摘要，stdout 是与 route.py 等价的业务 JSON，退出码是业务码；宿主状态与采集结果走 stderr 起止配对回执，所有路径（含 125 拒绝）都输出同一对边界，规则与 `saddle agent` 相同；宿主拒绝固定为 125（第 3 节）。
- 采集复用阶段 01/02 的 `Store::check_operation / prepare_operation / record_begin / record_end` 和进程内 `Capture`；插件只拿到一个只能 `begin` 一次、不返回任何状态的 `Recorder`，不接触 ID、generation、producer 或 evidence_kind（第 4 节）。
- 路由逐项保持 route.py 的请求、整理、阈值、舍入、重试和 null 兜底语义，列出少数接受的边缘差异（第 5 节）。
- skill 在用户**启用**插件时自动安装；TUI 启动时只升级本插件拥有且未被改过的旧版本；停用不删 skill；“Remove resources”只删自己拥有且未改的文件。现有两处软链接被当作外来同名目标，永不改写，留给阶段05（第 6 节）。
- 用户已确认停用后的skill行为（第9节D1）；已知设计阻断M1/M2经限定修订关闭。未发现需要修改Corral的接口；未实施的行为仍待代码验证。

## 1. 现状与缺口（代码定位）

| 位置 | 已有 | 对本设计的意义 |
|---|---|---|
| `src/main.rs:5-14` | 只分发 `agent`、`telemetry`、`ctl` 三个 headless 子命令 | 需加 `plugin` 子命令 |
| `src/plugins/registry.rs:8-112` | 外部插件清单：相对 `executable`、`required_capabilities`、可选 view/action；`Manifest::read` 要求可执行文件存在 | 内置插件没有目录/可执行文件，不能伪装成这种清单 |
| `src/plugins/registry.rs:115-125,167-182` | `plugins.toml` 为 `{version=1, plugins=[{id,directory,enabled}]}`；新增默认停用；写入有 flock + baseline 比对 | 可加可选 `core` 表承载内置插件启停；`File`/`Entry` 没有 `deny_unknown_fields`，旧版本读取时忽略新表 |
| `src/app.rs:254-259` | 登记文件固定为 `config 所在目录/plugins.toml`，`Manager::open` 启动全部 enabled 外部插件 | headless 入口须用同一路径规则；内置插件不在 TUI 中起进程 |
| `src/plugins/runtime.rs:164-220`、`src/plugins/mod.rs:520-554`、`src/app_control.rs:93-112,226-238`、`src/control_cli.rs:101-108` | `command.v1`：经运行中 TUI 的控制 socket 转给已运行插件进程，参数/结果各 ≤48 KiB，120 s 后 result_unknown | 依赖运行中的 TUI 和插件进程，不能当无 TUI 路由入口 |
| `crates/plugin-protocol`、`crates/plugin-sdk` | 外部进程插件的线协议与 Ratatui SDK | 内置插件不需要也不应依赖（避免 dispatch 拉入 ratatui） |
| `src/telemetry/capture.rs:6-37,97-140,142-279,314-322,369-470` | `OperationInput`；`check_operation` 在关闭时也校验关联；`prepare_operation` 固定 generation、暂存任务书快照与正文；`record_begin` 原子写快照+begin；`record_end_before` 在 begin 缺失时补 operation、沿用 begin 链接并可发布原暂存快照 | 全部可直接复用；`kind="route"` 已被接受 |
| `src/telemetry/validate.rs:242-321,385-500` | route.begin 需 summary/request 正文与 router_model；route.end 的 outcome 沿用进程结果词表，exited/0 视为成功并要求 response/suggestion 或 gap；begin 缺失时 end 可沿用 begin 链接 | 不改 schema，只需固定 route 的 outcome 映射（4.4） |
| `src/telemetry/blobs.rs:79` | 正文只按绝对文件路径暂存，单正文上限 16 MiB | 内存中的摘要/请求/响应需先写私有临时文件；`src/agent.rs:191-219` 已有同样做法 |
| `src/agent.rs:298-311,341-412` | 125 拒绝回执、关联错误才拒绝执行、存储错误只成 gap、起止边界回执 | 插件入口照搬这些语义 |
| `~/.claude/skills/corral-dispatch`、`~/.agents/skills/corral-dispatch` | 均为指向 `../corral/corral-dispatch-skill` 的软链接（只读核对 `readlink`） | 安装器必须视为外来目标，不能改 |

没有现成的：内置插件目录、内置插件启停存储、无 TUI 插件命令、插件可用的采集接口、技能资源安装/所有权记录。外部插件的 `SADDLE_HOST_BIN`（契约 §2.2 拟议）也尚未实现，但本设计不需要它，仍留给阶段04 Drover 接入。

## 2. 模块与依赖

```text
src/main.rs（组装根，唯一引用 dispatch crate 的地方）
   │  core_catalog = [&saddle_dispatch_plugin::PLUGIN]
   ├──► saddle（lib，宿主）
   │      plugins::core      目录查找、启用检查、run 编排、回执
   │      plugins::cli       `saddle plugin status|run`
   │      plugins::capture   Recorder 的宿主实现 → telemetry::{Store,Capture}
   │      plugins::resources 资源安装器与所有权记录
   │      plugins::{registry,mod,ui,palette} 加 core 表与管理页行
   │      telemetry、agent   阶段01/02，不改 schema 与行为
   │      （saddle 包依赖 saddle-core-plugin，plugins::capture 实现其 Recorder）
   └──► saddle-dispatch-plugin（plugins/dispatch）
          route（JEV 请求/整理/重试）、资源（include_bytes）、接入说明
            └──► saddle-core-plugin（crates/core-plugin，新：只有 trait 与类型，无宿主依赖）

不变：saddle-plugin-protocol / saddle-plugin-sdk / plugins/drover / plugins/diff / 外部 Corral（零依赖，dispatch 不调用 Corral）
```

- Cargo 包级别：`saddle → {core-plugin, dispatch}`，`dispatch → core-plugin`，无环。dispatch 不依赖 `saddle`，因此无法构造遥测事件、读 Store 或选择来源。
- 宿主库模块只接收 `&'static [&'static dyn CorePlugin]`（`app::run` 与 `plugins::cli::run` 多一个参数）。测试注入假插件，宿主核心不出现 dispatch 名称或业务判断。这是二进制入口处的编译期组装，不是宿主功能依赖插件：遥测、查询、`saddle agent` 在 dispatch 停用或不存在时行为不变。
- 不建议把 trait 放进 plugin-sdk：SDK 依赖 ratatui 且面向外部进程；也不建议把 telemetry 拆成独立 crate 再给插件直接用——那会让插件绕过宿主拿到 Store。

### 2.1 `saddle-core-plugin` 接口草案

```rust
pub struct Manifest {
    pub id: &'static str,            // 符合 registry::valid_id；外部登记不得占用
    pub name: &'static str,
    pub version: &'static str,       // env!("CARGO_PKG_VERSION")
    pub commands: &'static [Command],
    pub resources: &'static [Resource],
    pub setup_note: &'static str,    // 插件提供的接入说明纯文本
    pub setup_files: &'static [SetupFile], // 宿主把它解析成已安装的绝对路径
}
pub struct Command { pub name: &'static str, pub capture: Option<Operation> }
#[non_exhaustive] pub enum Operation { Route }            // 只开放遥测已有的 route
pub struct Resource { pub kind: ResourceKind, pub name: &'static str,
                      pub revision: u32, pub files: &'static [ResourceFile] }
#[non_exhaustive] pub enum ResourceKind { AgentSkill }
pub struct ResourceFile { pub path: &'static str, pub bytes: &'static [u8] }
pub struct SetupFile { pub label: &'static str, pub resource: &'static str, pub path: &'static str }

#[non_exhaustive] pub enum Begin { Route(RouteBegin) }
pub struct RouteBegin { pub router_model: String, pub router_version: Option<String>,
                        pub rules_version: Option<String>, pub summary: Vec<u8>, pub request: Vec<u8> }
#[non_exhaustive] pub enum End { Route(RouteEnd) }
pub struct RouteEnd { pub response: Captured, pub suggestion: Captured }
/// 取得的完整正文，或取不到的原因（宿主映射为遥测 gap，不用其他字节顶替）。
pub enum Captured { Bytes(Vec<u8>), Missing(Missing) }
#[non_exhaustive] pub enum Missing { NotAvailable, Unrecognized, TooLarge }

pub trait Recorder {
    /// 至多一次，紧挨在外部副作用之前调用。无返回值：业务不得依据采集结果分支。
    fn begin(&mut self, begin: Begin);
}
pub struct Call<'a> { pub command: &'a str, pub stdin: &'a mut dyn std::io::Read,
                      pub recorder: &'a mut dyn Recorder }
pub struct Completion { pub exit_code: u8, pub stdout: Vec<u8>, pub end: Option<End> }
pub trait CorePlugin: Sync {
    fn manifest(&self) -> &'static Manifest;
    fn run(&self, call: Call<'_>) -> Completion;
}
```

`Begin/End` 用封闭枚举而不是任意 JSON payload，对应遥测 v1 的封闭事件表，不能用通用 payload 绕过校验。以后真有第二种受控采集再加枚举项，本轮不预留其他扩展点。插件不写 stdout/stderr（返回字节由宿主写），stderr 只属于宿主回执。

## 3. 官方 core plugin：登记、启停与无 TUI 入口

### 3.1 目录与启停存储

- 目录：编译期常量列表，由 `src/main.rs` 传入。core plugin 的“安装”就是随 Saddle 发布；用户第一次“启用”才是资源安装和接入说明的触发点（第 6 节）。
- 启停存储：同一个 `plugins.toml` 增加可选表，保持 `version = 1`：

  ```toml
  [core.dispatch]
  enabled = true
  ```

  `File` 加 `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")] core: BTreeMap<String, CoreEntry{enabled: bool}>`，读写沿用现有锁和 baseline。缺表或缺项 = 停用（默认停用，和外部插件“添加后默认停用”一致）。未知 core id 原样保留，不报错。
- 兼容：旧二进制读取时忽略 `core` 表；但旧二进制写登记（启停外部插件）会丢掉该表，回到新版后 dispatch 呈停用。这是安全方向的退化，写入使用说明，不做额外迁移。
- ID 冲突：`Registry::add` 拒绝与内置 ID 相同的外部插件（错误“plugin ID reserved by a built-in plugin”）；内置 ID 列表由 Manager 传入，registry 不 import 插件。若旧登记里已有同 ID 外部条目，外部条目照旧工作，内置插件显示 `Conflict`，`run` 返回 125 `plugin_conflict`。

### 3.2 CLI

```text
saddle plugin [--config PATH] status [ID]
saddle plugin [--config PATH] run ID COMMAND [--record-context /abs/context.json] [--brief-file /abs/brief.md]
```

- `--config` 与 TUI 相同，只用于定位 `plugins.toml`；默认 `config::default_path()` 的目录。参数用 `args_os` 解析，路径必须绝对且为 UTF-8，重复选项拒绝。v1 命令不接受位置参数（不设 `-- ARG...`）。
- `status`：只读。输出一行 JSON：内置插件的 id/name/version/enabled/state（enabled|disabled|conflict）、命令及其采集类型、每个资源的 revision、内容指纹、各目标路径与分类（6.3）、`setup_note` 和解析后的 `setup_files`；外部插件只列 id/directory/enabled 与清单可读与否（运行状态属于各 TUI 实例，不在这里报告）。退出码：0 成功；1 登记或资源记录不可读；2 参数或未知 ID。不写任何文件。
- `run`：
  1. 解析参数；ID 不在目录中：若是已登记外部插件，返回 125 `external_plugin`（提示用运行中 Saddle 的 `saddle ctl plugin`）；否则 125 `unknown_plugin`。
  2. 只读 `plugins.toml`。读失败 125 `registry_unavailable`（无法确认启用即不执行）；未启用 125 `plugin_disabled`；ID 冲突 125 `plugin_conflict`。不自动启用、不回退到旧脚本。
  3. 命令不存在 125 `unknown_command`；命令没有声明采集却给了 `--record-context` 时 125 `capture_not_supported`。`--brief-file` 无 `--record-context` 时与 `saddle agent` 一样接受但不读取。
  4. 有记录上下文时按 4.3 先校验；关联无效 125，存储关闭/不可用只记 gap 继续。
  5. 进入 `plugin.run`（`catch_unwind` 包住；起始边界已在入口写出，见下），把 `Completion.stdout` 原样写 stdout，按 4.3/4.4 写 end，最后写终止回执，以业务码退出。

以上任一步的 125 拒绝都在已写出的起始行之后补写终止行。

上下文文件与 `saddle agent` 同一格式（`src/agent.rs:18-29`）：`schema_version=1`、`trace_id`、`dispatch_id`、可选 `basis_event_ids`、`previous_brief_event_id`（须同时给 `--brief-file`）。route 不接受 `decision_event_id` 与 `send_kind`（路由先于主控决定；遥测也不允许 route.begin 引用 decision），出现即 125 `invalid_context_combination`。文件读取限制（O_NOFOLLOW、普通文件、64 KiB）照搬 agent。

**stderr 回执（与 `saddle agent` 同一起止匹配规则）**：除 `--help` 外，每次 `run`——成功、业务失败、125 拒绝（含参数解析失败）、126 内部失败——都输出一对宿主行，前缀 `saddle-plugin: `：

```text
saddle-plugin: {"schema_version":1,"call_id":"<本次 UUID v4>","final":false}
saddle-plugin: {"schema_version":1,"call_id":"<同一 UUID>","final":true,"plugin":"dispatch","command":"route","executed":true,"outcome":{"kind":"exited","exit_code":0},"error":null,"operation_id":"…","begin":"stored","end":"stored","gaps":[]}
```

终止行字段与 `src/agent.rs` 的回执同形（平铺 operation_id/begin/end/gaps，另加 plugin、command）；拒绝时 executed=false、outcome=null、error={code,message}。不含正文、key 或请求头。

宿主写出规则：

1. call_id 在入口生成，不传给插件或 JEV。起始行随即写出，是本次 stderr 的字节 0，早于参数解析、启用检查和 `plugin.run`；125 拒绝路径没有业务，起始行之后直接写终止行。
2. 两行都用 02A `process::write_receipt` 同样的有界写法：poll 可写后每次至多 512 字节，不给继承来的 fd 设 O_NONBLOCK；每行有固定等待上限（实施初值 1 s，可按合成验证调整），超时或写错误即放弃该行，不重试。回执写出从不无限阻塞业务。
3. 起始行没能完整写出时，业务照常执行一次（与 02A“业务仍可执行”一致；回执从不阻止、推迟到无限或重放业务），但本次不再写终止行，避免在残缺首行后追加可被误读的内容。
4. 终止行写不出或只写出一部分时同样放弃：不篡改已知退出码（业务码或 125/126），不重跑插件，不补写。stdout 写失败（如 EPIPE）只在终止行 gaps 加 `output/write_failed`，退出码不变。
5. 插件本身不写 stderr；第三方库若意外写出，只会夹在两行之间，按下面规则不影响判定。

调用方（包括按 skill 行事的主控）的判定规则与 [遥测使用](遥测使用.md) 中 `saddle agent` 的规则相同：等进程退出、stdout/stderr 收齐后，只取 stderr 字节 0 起的第一条完整 LF 行作起始边界（前缀、合法 JSON、schema_version=1、final=false、合法 UUID call_id），只取最后一条完整 LF 行作终止回执（同前缀、合法 JSON、final=true、call_id 与首行相同），不向前或向后搜索其他行。两者都匹配才采信 executed、outcome、error 与采集状态；缺失、截断或不匹配一律为“未知”，不能凭退出码（包括 125、126）断言未执行、被拒绝或插件未启用，也不据此重跑。业务结果只看 stdout 与退出码本身，不依赖回执。

**采集状态（begin/end）优先级**，取值沿用 agent 回执的 not_requested、stored、duplicate、disabled、unavailable、invalid，另加 not_reached：

1. 125 拒绝：与 `agent.rs` 的拒绝回执一致，begin/end=not_requested、operation_id=null。
2. 未给 `--record-context`：not_requested。
3. 预检 `check_operation` 已判 disabled/unavailable：begin/end 固定为该值；之后插件有没有调用 begin 都不改写。
4. 已请求且预检可采集，但插件没调用 begin：begin/end=not_reached。它只表示没走到操作起点（例如没有 key），不用于其他情况。
5. 插件调用了 begin：begin 为 prepare_operation/record_begin 的结果；prepare 失败时 end 与 begin 相同（同 `agent.rs`），有 Capture 时 end 为 record_end 的结果。

**退出码**：

| 码 | 含义 | executed |
|---|---|---|
| 0–124 | 插件业务码原样返回；dispatch route：0 = `ok:true`，1 = `ok:false`（同 route.py） | true |
| 125 | 宿主在进入插件前拒绝（参数、未知、停用、冲突、登记不可读、上下文/关联无效） | false |
| 126 | 进入插件后宿主内部失败：panic，或插件返回 ≥125 的非法码；业务结果未知，stdout 可能为空 | true |
| 128+n | 被信号终止，没有终止回执；结果未知 | 未知 |

退出码 126/127 也可能来自 shell（不可执行、找不到 `saddle`），125/126 是否真是宿主拒绝或插件内部失败，只以配对回执为准；表中 executed 列指配对回执里的值。

业务结果与采集结果严格分开：采集失败从不改变 stdout 或退出码，也没有“为补记录再跑一次”的选项。宿主对内置命令只做名称查找和一次同步调用，不提供队列、状态机、多步工作流或常驻服务。

### 3.3 停用与在途请求

| 发生的事 | 对已在执行的 run | 对之后的新 run |
|---|---|---|
| 在 TUI 停用 dispatch | 不中断（各 run 是独立进程，启用只在开始时检查一次），照常返回与记录 | 125 `plugin_disabled` |
| 遥测总开关或链路开关关闭 | 业务继续；尚未提交的 begin/end 被拒（disabled） | 不采集 |
| 关闭后又重开 | 旧 Capture 的 generation 已变，end 仍被拒，不复活 | 新调用可采集 |
| 替换 Saddle 二进制 | 运行中的进程用旧代码完成 | 新进程用新代码 |
| 调用方 kill / Ctrl-C | 进程终止，无终止回执；JEV 可能已收到请求；遥测可能只有 begin | 不自动重试 |
| 插件 panic | 126；若已有 Capture，宿主写 `outcome=unknown` 的 end | — |

v1 不装信号处理器、不提供总超时选项：路由单次进程短，route.py 也没有；调用方自己的超时 = 结果未知，按 skill 规则不重试。

### 3.4 TUI 管理页最小改动

- Manager 把内置插件作为额外行列出（建议排在外部插件前），Runtime 列显示 `Built-in`，State 为 Enabled/Disabled/Conflict。内置行可用动作：Enable/Disable、Sync resources（已启用时）、Remove resources（已停用且有自己拥有的资源时）、Refresh、Back；Open panel、Restart、Remove（登记）不适用。外部插件行和现有动作不变。
- 详情区显示资源状态、插件提供的接入说明与模板路径（6.7）。窄窗口可能截断，完整内容由 `saddle plugin status dispatch` 输出。
- Plugins 搜索面板：内置插件一行，状态如 `Built-in · Enabled`；Enter 打开管理页并选中该行（复用 `select_plugin`），不新增视图类型。也可以只在管理页列出；推荐列出，避免用户找不到入口。
- 线框（用户确认样子后再实现）：

```text
  Name                     Enabled  Runtime
> Saddle Dispatch          Yes      Built-in
  Drover                   Yes      Running

ID: dispatch · Built-in 0.1.0 · Command: saddle plugin run dispatch route
Skill corral-dispatch r1: Claude Code installed · Codex conflict (existing link → …/corral-dispatch-skill, unchanged)
<插件接入说明 3–4 行>
模板：/Users/<u>/.claude/skills/corral-dispatch/项目AGENTS模板.md
```

### 3.5 与外部插件和 ctl 的共存

外部插件的清单、进程、协议、SDK、Attention/通知与 `saddle ctl plugin`（command.v1）一律不变。`saddle ctl plugin --plugin dispatch` 仍会因 TUI 中没有该进程而失败；`saddle plugin run` 不运行外部插件。两个入口各管一类，不互相回退。

## 4. 通用采集接口

### 4.1 宿主提供与插件持有

| 宿主提供（`src/plugins/{core,capture}.rs`） | 插件持有（`plugins/dispatch`） |
|---|---|
| 命令行解析、启用检查、上下文文件读取与关联校验 | 读取 stdin 摘要、key（`TYPESAFE_API_KEY`）、构造请求 |
| `Recorder` 实现：内部持有 `Store`、`OperationInput`、`Capture`（含固定 generation 与暂存字节） | 决定何时调用 `begin`（紧挨首个网络请求之前）以及提供哪些正文 |
| producer（`saddle.plugin.<id>`，由目录 ID 生成）、evidence_kind、operation/event ID、outcome | 路由规则、HTTP 重试、整理输出、`Completion.end` 的 response/suggestion |
| 把内存正文写成 0600 临时文件交给 `prepare_operation`（复用 `agent.rs` 做法，可提成 crate 内共享函数，不改 Store 公开签名） | 不能取得 Store、generation、凭据或任何 ID |
| stdout 写出、end 记录、stderr 回执、退出码 | 不写 stdout/stderr |

### 4.2 数据流

```text
主控 ──stdin 摘要──► saddle plugin run dispatch route [--record-context C] [--brief-file B]
宿主：生成 call_id、写 stderr 起始边界 → 解析 → 启用检查 → (有 C) check_operation：无效→125（写终止行）；关闭/不可用→Recorder=Inactive
插件：读摘要、strip → 无 key：stdout ok:false，退出1，不调用 begin（capture=not_reached）
插件：构造请求 JSON 字节（只含摘要）
插件 ─Recorder.begin(Route{summary, request, router_model, router/rules_version})─► 宿主
      宿主：prepare_operation（读 B 快照、暂存正文、固定 generation）→ record_begin（快照+begin 同一事务）
插件 ──POST 请求（不含 B、不含任何遥测 ID）──► JEV ──HTTP 响应──► 插件
插件：最终 2xx 响应解析为完整 JSON 值 → 整理 → Completion{exit_code, stdout=业务 JSON, end=Route{response=完整解析响应表示, suggestion=stdout}}
宿主：写 stdout → record_end(outcome=exited(exit_code)) → stderr 终止回执 → exit(exit_code)
主控：读结果，自己定模型家族/强度/预算；需要时用现有 `saddle telemetry append` 提交 controller.decision（based_on 指 route.begin/end）
```

任务书只在本地快照，从不进入请求；JEV 只收到摘要，这是原有行为，采集不改变它。API key 只在插件内部拼 Authorization 头，不进入 `Begin/End`、回执或错误文本。

### 4.3 时序与失败处理

| 情况 | 处理（依据） |
|---|---|
| 未给 `--record-context` | 不打开遥测库、不建目录；begin/end=not_requested |
| 关联 ID 不存在、跨 trace、字段无效 | 执行前 125（与 `agent.rs:347-359` 相同，关闭时也校验，`capture.rs:97-140`） |
| 总开关/链路关闭、库未初始化或不可用 | 路由照常；begin/end 记 disabled/unavailable |
| begin 前出错（无 key、摘要不是 UTF-8） | 不建 operation，capture=not_reached；主控可在决定事件里写“路由不可用”，不伪造 begin |
| `prepare_operation` 失败 | 无 Capture；end 不再尝试；不在 end 时重新取 generation 补交 |
| `record_begin` 失败但 Capture 在 | 路由照常；end 时由 `record_end` 判定 begin 缺失：建 operation、标 `begin_missing=true`，沿用调用前固定的 based_on/uses_brief，若原 generation 仍有效则以 `publication_phase=end` 发布调用前暂存的任务书原字节（`capture.rs:395-470`、`validate.rs:302-310`，契约 §4 第5步）；summary/request 属于 begin 正文，不补造 |
| 调用中关闭或关闭后重开 | `check_capture` 比较开始时的全局与链路 generation，不一致即 disabled（`capture.rs:314-322`）；不换新凭据 |
| `begin` 被调用两次 | 第二次忽略，回执 gap `capture/unrecognized`；业务不受影响 |
| 插件返回后 | 有 Capture 才写 end；回执状态按 3.2 的优先级，not_reached 只用于“已请求、预检可采集、但没调用 begin” |
| panic | 有 Capture 时写 `outcome=unknown`，response/suggestion 记 gap not_available；退出 126 |
| 被杀/超时 | 没有 end；查询显示 begin 无 end = 结果未知；不重试 |

锁等待沿用 Store 现有 100 ms 预算，begin 位于网络请求之前，最多增加这一预算加磁盘 IO。

### 4.4 route 事件映射（不改 01 schema）

| 事件 | 内容 |
|---|---|
| route.begin payload | `router_model="jev-1.13.0"`；`router_version="route-v1"`（移植版本常量，路由整理/重试语义变化时手动递增）；`rules_version="sha256:<规则常量的规范 JSON 指纹>"`（TIERS、题目、阈值、模型名，自动计算） |
| route.begin 正文 | `summary` = strip 后的摘要 UTF-8 字节；`request` = 实际发送的 HTTP 请求体字节（重试发送同一份） |
| route.begin 链接 | based_on → 需求/提议/授权/复用证据/controller.summary；uses_brief → 本 operation 的快照；无 uses_decision |
| route.end outcome | 业务返回时 `{"kind":"exited","exit_code":<业务码>}`（0 成功、1 失败）；panic 为 `unknown`。HTTP 超时属于业务失败（exit 1），不是 `timed_out` |
| route.end 正文 | `response` = **完整解析响应的表示**（契约 §5.1、接入方案 §4/§7）：仅当最终一次尝试返回 2xx 且整个响应体解析为 JSON 成功时取得；把解析得到的整个 JSON 值（含 answers、model、usage 及整理未用到的字段）按文档中的键顺序紧凑序列化为 UTF-8 JSON，重复键按 Python dict 语义（后值替换、保留首次位置），数值取解析后的值（i64/u64 内整数精确，其余为 f64）。它是解析结果的规范表示，既不是 HTTP 原始字节，也不是整理建议；不截断。保留键顺序，使存下的响应足以复核 level 平局和 cross 遍历顺序。整理（shape）失败时仍保存这份已取得的解析响应。`suggestion` = 本次写到 stdout 的业务 JSON 原字节（成功为三项建议，失败为 `ok:false`），即主控实际看到的输出 |
| response 取不到时 | 只记 gap，不用其他字节顶替：网络失败或最终尝试为非 2xx → not_available；2xx 但响应体读取中断 → not_available；2xx 但 JSON 无效 → unrecognized；响应体超过 16 MiB 读取上限或表示超过正文上限 → too_large。重试前那次 429/529 的响应体、任何 HTTP 错误体、截断或部分响应体都不作为 response 保存；HTTP 错误的前 300 字节只出现在 `suggestion`（stdout 的 error 文本）里。成功（exited/0）时 response 与 suggestion 都必有 |

主控已有的 controller.summary（purpose=route）、controller.decision、controller.note 仍走现有纯记录命令，本设计不加新事件类型。

### 4.5 受控现场采集与普通声明

- **受控现场采集**（`execution_observed`）：仅限编译进同一 Saddle 构建、在目录里声明了 `capture: Some(Route)` 的内置插件命令，通过 `saddle plugin run` 执行时由宿主写入。可信边界是“同一份经审查的构建”，与 `saddle agent` 相同；它不认证调用者身份（任何本地进程都能调用该命令，但记录的确实是 Saddle 自己执行的路由）。若将来内置插件改成从构建外加载，这一前提失效，须重新设计。
- **普通声明**：外部进程插件、主控和任何其他调用方只能用 `saddle telemetry append` 提交 `plugin_statement`/`controller_statement`，记为 unverified；不能取得 Recorder，也不能产生 route.* 或 agent.* 事件。没有跨进程 token，没有放宽来源规则。
- `saddle agent` 的 agent.* 采集不对插件开放；`Operation` 只有 `Route`。

## 5. 路由迁移范围（route.py 静态核对）

核对对象：`../corral/corral-dispatch-skill/route.py`，Corral HEAD `6923da1`，SHA256 `d4ab1cc8…e548167a`，工作区无改动。只读源码，没有执行。

### 5.1 保持不变的语义

| 项 | route.py 行为 | Rust 移植要求 |
|---|---|---|
| 输入 | stdin 全部读入，`str.strip()` | 读全部字节，须为 UTF-8；按 Python `str.isspace` 字符集去首尾空白（Rust `char::is_whitespace` 之外还包括 U+001C–U+001F）；空摘要照常发送 |
| key | 只读 `TYPESAFE_API_KEY`；缺失或空串 → `{"ok":false,"error":"TYPESAFE_API_KEY is not set"}`，退出 1，不发请求 | 相同文本、相同退出码；key 先于请求检查 |
| 请求 | POST `https://api.typesafe.ai/v1/systemone`；`Authorization: Bearer <key>`、`Content-Type: application/json`；体为 `{"state":{"task_summary":…},"model":"jev-1.13.0","questions":{tier, cross_data_model, cross_concurrency, cross_security_privacy, cross_core_rules, visible, doc_only}}`，题目原文照抄 | 用带字段顺序的类型序列化，保持上述键顺序；URL 和模型为常量，生产代码不提供环境/参数覆盖（防止把 key 发往他处） |
| 超时 | 每次尝试 socket 超时 10 s，无总时限 | 每次尝试的连接、等待响应、读取响应体各 10 s；不新增总时限 |
| 重试 | 最多两次尝试：首次遇到 HTTP 429/529 → 等 1 s 重试；首次遇到连接阶段错误（urllib 包成 URLError，含 DNS、拒绝、TLS、发送失败）或读超时（Python ≥3.10 的 socket.timeout 即 TimeoutError；本机 python3 为 3.12）→ 立即重试；第二次失败报错。其他 HTTP 状态、等待响应时连接被重置（不在 URLError/TimeoutError 内）、响应体不完整、JSON 解析失败都不重试 | 按阶段分类：请求完整发出前的错误与任何超时可重试；请求发出后的非超时错误不重试；状态码规则同左。用合成服务器逐项覆盖 |
| 错误输出 | `{"ok":false,"error":"<类名>: <信息>"}`，HTTP 错误含状态码和响应体前 300 字节（替换非法 UTF-8） | 保留 `HTTP <code>: <前300字节>`、`network: …`、key 文本；类名前缀是 Python 产物，不复刻。error 文本只供诊断，不是契约 |
| tier | 概率键 `int(k)` 映射 轻/常规/重，值 `round(v,3)`；level 取**舍入后**最大值，平局取响应中先出现者；`confidence >= 0.8`（**未舍入**）才给 verdict，否则 null；score/confidence 输出舍入 | 必须保留响应中的键顺序：用自定义反序列化得到有序列表，不能启用 serde_json `preserve_order`（会改变整个宿主二进制的 JSON 行为） |
| cross_review | 遍历全部以 `cross_` 开头的答案（按响应顺序），值舍入；top = 舍入后最大值；`>=0.8` 要、`<=0.2` 不要、其余 null；输出 `{verdict, 各题}` | 同左，比较用**舍入后**值 |
| impact | `visible = round(max(visible, doc_only), 3)`；cross=要 → 碰要害；否则 visible≥0.8 → 看得见；visible≤0.2 且 cross=不要 → 改行为；其余 null | 同左 |
| 其余 | `model`、`usage` 原样透传（缺失为 null）；任何字段缺失或类型不符 → `ok:false`；输出一行 JSON，`ensure_ascii=False`；成功退出 0、失败 1 | 同左 |
| 舍入 | Python `round(x,3)`：对二进制值正确舍入、恰好平局取偶 | 用对二进制值正确舍入的十进制转换再解析（例如 `format!("{:.3}")`→`parse`，其平局行为须由测试确认），不能用 `(x*1000).round()/1000`；定值测试覆盖二进制恰好平局值（0.0625、0.1875 等奇数/16）和十进制看似平局的值（0.0005、0.2675），期望值由实现任务按 Python 规则推导 |

### 5.2 接受的边缘差异

请求/输出按 JSON 值等价判定，不追求字节相同：Python 的 `json.dumps` 默认带空格分隔且请求转义非 ASCII，Rust 输出紧凑 UTF-8；User-Agent 不同；整数概率在 Python 保持整数、Rust 输出浮点（数值相等）。Python 接受的非标准概率键（如 `"01"`、负索引 `"-1"`）、`NaN` 字面量、布尔当数值等，Rust 一律按无效响应 `ok:false`。摘要非 UTF-8 时 Python 视 locale 可能抛未捕获异常（无 JSON、退出 1）或带代理字符照发；Rust 一律输出 `ok:false`、退出 1、不发请求。重定向：urllib 会把 POST 的 301/302/303 改成 GET 并继续携带 Authorization；Rust 版不跟随重定向，当作不重试的 HTTP 错误，避免把 key 转发到别处。响应体读取设 16 MiB 上限，超出按无效响应处理。

### 5.3 仍属主控，不进入插件或宿主

模型家族（Claude/Codex/pi/omp）、实际模型与强度、验证预算、是否推翻路由、是否交叉审查的最终决定，以及“项目写了不用路由时不调用”。插件只给三项建议和 null；宿主遥测只保存。

### 5.4 skill 文本的最小改动

资源从 Corral `6923da1` 复制到 `plugins/dispatch/resources/corral-dispatch/`：

- `项目AGENTS模板.md`：逐字节不改（SHA256 `22339f46…c2f73384`），保留全部必须在加载 skill 前生效的规则。
- `SKILL.md`：只改第 3 节“问路由”的命令与兜底条目，其余（含 frontmatter 名称 `corral-dispatch`、分工表、档位、第 7 节派发记录器）不动，阶段05 另行处理。兜底条目按用户已确认的9.1 D1写：

  ```text
  echo "<摘要>" | saddle plugin run dispatch route
  - 等命令结束、输出收齐再判断。路由结果只看退出码和 stdout：退出码 0 且 stdout 是 "ok": true 的 JSON，照用各项 verdict（null 照旧自己定）。
  - 其他任何情况都是路由不可用：各项自己按下面的规则定；不要重试，不要自己启用插件，不要改用旧 route.py，不要问用户。
  - 在路由行写不可用原因时，只认 stderr 的完整配对回执：第一行是 `saddle-plugin:` 起始行（final=false），最后一行是同一 call_id 的 final=true 终止行。配对成功才照终止行的 error.code 写原因（plugin_disabled 写「插件未启用」）；回执缺失、截断或不配对，只写「路由不可用，原因未知（退出码 N）」，不能凭退出码说插件未启用或没执行。
  - 需要记录这次路由时加 --record-context <绝对路径> [--brief-file <任务书绝对路径>]。记录状态同样只认配对终止行的 begin/end，否则记为未知；不论记录成功与否都不重跑路由。
  ```
- `README.md`：安装/卸载节改为“随 Saddle 启用 dispatch 插件自动安装”，key 与“不用路由”说明保留；`route.py` 不再随资源分发。

阶段05 切换前，Corral 仓库里的旧副本仍是现用来源；期间若旧副本有改动，要手工同步到新资源，否则会分叉（见 9.3）。

## 6. 技能与模板资源生命周期

### 6.1 资源声明与版本

dispatch 声明一个 `AgentSkill` 资源 `corral-dispatch`，文件为 `SKILL.md`、`项目AGENTS模板.md`、`README.md`，用 `include_bytes!` 编进二进制，与路由代码同版本，不需要用户另管文件或版本配对。Saddle 包版本长期是 `0.1.0`，不能区分构建，因此资源用整数 `revision` 排序、用内容指纹（各文件路径与 SHA256 的规范列表再取 SHA256）识别。dispatch crate 加一个固定 `(revision, 指纹)` 的测试：改资源而不递增 revision 会失败。

安装器是通用的：只认 `AgentSkill` 类型和清单里的名称与文件；资源名与相对路径须是普通名称（无绝对路径、`..`、符号链接），不含任何 dispatch 文本或判断。v1 只有内置插件能声明资源；外部插件清单不支持，需要时另升清单版本。

### 6.2 目标与所有权记录

- 目标（与旧 README 实测的两家一致）：`$HOME/.claude/skills/<名称>`（Claude Code）、`$HOME/.agents/skills/<名称>`（Codex）。`~/.claude` 或 `~/.agents` 不存在时该目标记 `skipped(agent_absent)`，不替用户创建 agent 主目录；`skills/` 子目录可创建。v1 不读取 `CLAUDE_CONFIG_DIR` 等重定位变量，不宣称支持其他 agent。
- 所有权记录：`$XDG_STATE_HOME/saddle/plugin-resources.json`（与遥测相同的状态目录规则，目录 0700），配 `plugin-resources.lock`（flock 非阻塞，忙则本次结果 `busy`）。每个目标一条：plugin、resource、agent、绝对路径、revision、`files{相对路径: sha256}`、可选 `pending`（进行中的新 revision 与文件表）、时间。临时文件 + fsync + rename 写入。
- 记录是按用户的，不随 `--config` 区分。

### 6.3 目标分类（只用 lstat/O_NOFOLLOW，不跟随符号链接）

| 分类 | 判定 |
|---|---|
| absent | 路径不存在，且没有记录 |
| missing | 有记录，路径不存在（用户删了或安装中断） |
| foreign | 有东西但没有记录：符号链接（附 readlink 目标）、普通文件、或别人的目录 |
| owned_current | 有记录；所有记录文件为普通文件且哈希等于记录（或 pending）；revision 与内容等于随包版本 |
| owned_outdated | 同上但记录 revision < 随包 revision |
| newer | 记录 revision > 随包 revision（较新的 Saddle 装过） |
| incomplete | 有 `pending`，目标内容既不完整等于旧文件集也不完整等于 pending 文件集（安装/升级中断留下的新旧混合或缺文件） |
| modified | 有记录（无 pending 或已排除 incomplete），但任一记录文件缺失、变成链接或哈希不符；或同 revision 内容不同 |

“完整等于某个版本”指：该版本文件集里的每个路径都是普通文件且哈希一致，且只属于另一版本的路径不存在。按整套文件判断，不逐个文件拼凑。

目标目录里用户新增的额外文件不算修改，始终保留；新版本若要写同名文件则该目标记 modified。

### 6.4 触发点与动作

| 触发 | absent | missing | owned_outdated | owned_current | newer | modified / incomplete / foreign |
|---|---|---|---|---|---|---|
| 用户在管理页 Enable（首次或再次） | 安装 | 重新安装 | 升级 | 不动 | 不降级，报告 | 不写，报告冲突 |
| Sync resources（已启用时） | 同 Enable | 同 Enable | 同 Enable | 同 Enable | 同 Enable | 同 Enable |
| TUI 启动（仅已启用的内置插件） | 不动 | 不动，报告 | 升级 | 不动 | 不动 | 不动，报告 |
| Disable | 不动 | 不动 | 不动 | 不动 | 不动 | 不动 |
| Remove resources（已停用时） | — | 删记录 | 删自己的文件 | 删自己的文件 | 删自己的文件 | 不动，报告 |
| `saddle plugin status/run`、ctl、路由调用 | 只读或不涉及 | | | | | |

要点：

- 首版触发点固定为 **用户显式 Enable / Sync resources** 和 **TUI 启动时对已启用插件的自有旧版升级**。普通打开 Saddle 只会更新 Saddle 自己写入且未被改过的文件，从不创建新目标、不覆盖外来或被改过的内容。headless 命令不写技能目录。v1 不提供 headless enable：启用会改用户主目录下的全局技能，属于用户在界面上的决定；skill 也明确不让 agent 自己启用。
- 启用先写登记再处理资源：登记写失败则什么都不做；资源部分失败时插件仍是启用状态，逐目标回执，可点 Sync resources 重试。
- 停用保留 skill：启停是可逆开关；删除会和用户修改冲突，也无法让已加载 skill 的会话忘掉内容。停用后 skill 调用路由得到 125 `plugin_disabled`，按 5.4 的兜底继续（此行为见 9.1 D1）。
- “卸载”= 停用后 Remove resources：只删记录在案且未改过的文件，删完空目录才 rmdir；SKILL.md 最先删除，使目录立即不再被当作技能。项目 AGENTS、用户文件、外来目标和旧软链接都不碰。
- 多个 Saddle 构建并存时绝不自动降级（TUI 启动与 Enable 都不降），避免开发版和日常版来回改写。

### 6.5 写入过程与回执

- 安装：确认父目录状态 → 写 `pending` 记录 → `mkdir` 目标（已存在即并发冲突，按 foreign 处理）→ 每个文件先写同目录临时名（以 `.` 开头且不叫 SKILL.md）再 rename，**SKILL.md 最后写** → 记录定稿。失败时删除自己创建的文件与目录，撤 pending。不在技能根目录放整目录的暂存副本，避免 agent 把暂存目录当成重复技能。
- 升级：在锁内重新核对目标完整等于旧文件集 → 写 `pending` → 逐文件“临时名 + rename”替换，旧版本多出的自有文件在哈希仍等于旧记录时删除，SKILL.md 最后 → 定稿。
- 同一轮内的失败回滚只处理能证明属于本轮的文件：当前哈希等于本轮刚写入的字节，才恢复为内存中的旧字节（升级）或删除（安装、新增文件）；哈希对不上的文件不覆盖、不删除，原样保留并在回执里列出，结果记 failed。目录只在空时 rmdir。
- 记录先于目录变动写入。下次分类时：目录不存在 → missing（显式 Enable/Sync 时重装）；目标**完整等于** pending 文件集 → 视为新版本已装好并定稿；完整等于旧文件集 → 撤 pending，按旧版本处理；两者都不完整等于 → incomplete。incomplete 不会被标成 owned_current，启动和 Enable/Sync 都不自动修补，只报告“安装/升级未完成”及不一致的文件；处理方法是用户自行检查后删除该目录，再 Sync resources 按 missing 重装。v1 不提供更多恢复手段，不引入事务引擎。
- 用户在核对与替换之间的极短窗口里编辑文件仍可能被覆盖，文档注明。
- 回执（管理页消息和测试断言用同一结构）：

```json
{"plugin":"dispatch","enabled":true,"trigger":"enable",
 "resources":[{"name":"corral-dispatch","revision":1,"targets":[
   {"agent":"claude-code","path":"/Users/u/.claude/skills/corral-dispatch","before":"foreign",
    "result":"conflict","reason":"symlink","detail":"-> /Users/u/Developer/personal_projs/corral/corral-dispatch-skill"},
   {"agent":"codex","path":"/Users/u/.agents/skills/corral-dispatch","before":"absent","result":"installed"}]}]}
```

  `result`：installed / updated / unchanged / removed / conflict / skipped / busy / failed。界面摘要如“Enabled. Skill: Claude Code conflict (existing link, unchanged) · Codex installed. Nothing was overwritten.”。

### 6.6 当前两处旧软链接

两处都是 foreign/symlink：启用 dispatch 时报告冲突并保留原链接，旧会话继续读 Corral 仓库里的 route.py 版 skill。本轮与阶段03实现都不改它们。阶段05 单独执行：确认链接目标 → 记录到切换任务 → 只删链接本身（不带结尾 `/`）→ Sync resources 安装 → `status` 核对两目标 owned_current → 回退方法为停用后 Remove resources 再 `ln -s` 恢复。阶段05 还需先发布含 `saddle plugin` 的日常二进制，再切换 skill，否则新 skill 调用不存在的命令只会走兜底。

### 6.7 首版项目接入说明

显示位置：管理页 dispatch 详情区（启用后一直可见，可回看）、Enable 结果消息提示“见下方接入说明”、`saddle plugin status dispatch` 的 `setup_note/setup_files`。不在每次路由调用时重复，不提供项目开关、项目名单或自动改项目文件。

插件提供的文案草案（纯文本）：

```text
Dispatch 提供路由命令 `saddle plugin run dispatch route`，并随插件安装 corral-dispatch 技能（状态见上）。
技能装好不会让所有项目都改成主控分派。要让某个项目默认这样做：打开下面的模板，把第1节复制进该项目的 AGENTS.md，替换尖括号里的项目参数，并处理和已有规则的冲突。不采用的项目不用改；某次对话里明确要求走分派流程也可以。
路由需要环境变量 TYPESAFE_API_KEY，没有时主控自己判断，不影响委派。是否记录遥测由 Settings → General 单独控制。
```

宿主在说明下渲染 `setup_files`：`模板：<已安装目标中的 项目AGENTS模板.md 绝对路径>`（多个目标都列；全部未安装时写“模板：未安装（见上方技能状态）”）。宿主只做路径替换，不解析文案。

## 7. 框架缺口与建议改动文件

| 缺口 | 建议文件 |
|---|---|
| 内置插件接口 | 新 `crates/core-plugin/{Cargo.toml,src/lib.rs}`；根 `Cargo.toml` 加 workspace 成员与依赖 |
| 内置启停存储与 ID 保留 | `src/plugins/registry.rs`（`core` 表、读写、保留 ID）；`src/plugins/mod.rs`（Manager 内置行、状态、启停） |
| 无 TUI 入口 | `src/main.rs`（`plugin` 分发、帮助文字、组装目录）；新 `src/plugins/cli.rs`、`src/plugins/core.rs`；`src/app.rs` 的 `run` 接收目录 |
| 受控采集 | 新 `src/plugins/capture.rs`；`src/agent.rs` 的临时正文函数可提为共享（行为不变）；遥测 Store/schema 不改 |
| 资源安装器 | 新 `src/plugins/resources.rs`；`src/plugins/mod.rs`（启动升级）；`src/plugins/ui.rs`（内置行、Sync/Remove resources、说明区）；`src/plugins/palette.rs`（内置行进入管理页） |
| dispatch 插件 | 新 `plugins/dispatch/{Cargo.toml,README.md,src/lib.rs,src/route.rs,src/transport.rs,resources/corral-dispatch/*}`；HTTP 客户端建议 `ureq`（阻塞、rustls，成熟且活跃维护；不引 tokio），根证书/加密后端在实现时确认无额外系统依赖，不提供关闭 TLS 校验的选项 |
| 文档（批准后） | DESIGN.md 新小节；契约 §5.1 指向本文；`docs/遥测使用.md` 增 plugin run 采集；`docs/插件系统设计.md` 增内置插件；README 命令帮助 |
| 不改 | `crates/plugin-{protocol,sdk}`、`plugins/{drover,diff}`、`src/telemetry` 的 schema 与公开行为、`saddle ctl`、Corral 仓库与已安装技能 |

## 8. 串行实施拆分

三步串行、分支叠放，集成审查通过后一起合并（框架两步只有测试用假插件作消费者，接口可能在 dispatch 接入时还要调整）。每步都只用临时 HOME/XDG_* 与合成数据，不碰真实技能目录、遥测库、JEV 或队列。

| 步 | 交付 | 文件范围 | 关键验证 |
|---|---|---|---|
| 03A 内置插件框架与 headless 入口 | core-plugin crate；registry `core` 表与保留 ID；`saddle plugin status/run`；回执与退出码；Recorder→遥测适配；提供管理层所需的非可视目录/状态接口，不改界面 | 第7节前三、四行（不含 resources） | 假内置插件：未启用/冲突/登记损坏均 125 且不进插件；业务码与 stdout 透传；panic→126；旧登记兼容与旧二进制写回后默认停用；route 采集的 stored/disabled/重开后旧 end 被拒/begin 缺失 end 恢复；采集状态优先级（未请求、预检 disabled/unavailable 不被改写为 not_reached、仅可采集未 begin 才 not_reached）；未给上下文时不建遥测目录。回执：成功、业务失败、参数错误与各类 125、126 都有字节 0 起始行和同 call_id 的末行终止回执；用不读取的合成 stderr 管道验证起始行写不出时业务仍只执行一次、不写终止行、退出码不变，终止行写不出时有界返回且退出码不变；stdout EPIPE 只加 gap；任何回执失败都不重跑插件 |
| 03B 资源生命周期 | 资源安装器、所有权记录、启动升级、status资源段；集成管理页内置行、Enable/Disable、Sync/Remove resources、接入说明与模板路径渲染（线框确认后实施界面） | 第7节资源安装器行及管理页/Plugins面板 | 第 6.3 各分类与 6.4 动作矩阵；符号链接与外来目录永不写；用户额外文件保留；中途失败回滚；锁忙；不降级；模拟两处旧软链接只报告冲突 |
| 03C dispatch 插件 | 路由移植、ureq 传输、资源复制与 SKILL/README 最小改动、接入说明、目录登记、route 事件映射 | 第7节 dispatch 行与 `src/main.rs` 目录一行 | 请求 JSON 与 route.py 常量等价；5.1 整理/舍入/顺序/阈值边界定值；response 为完整解析表示（保留键顺序与重复键语义、shape 失败仍保存、非 2xx/前次 429/截断/无效 JSON 均不作为 response 而记对应 gap）；重试矩阵（假传输 + 本地 TcpListener）；无 key 不发请求不 begin；请求不含任务书；记录与回执中搜不到 key；资源 revision 指纹测试；模板逐字节一致 |

建议主控路由时注意：03A 涉及来源证据完整性，03B 会写删用户主目录文件，03C 移植核心判定规则，都可能落在“碰要害”。阶段04（独立查询界面、Drover 关联）和阶段05（发布、切换软链接与 skill、退役 route.py/dlog 依赖）保持原计划，不并入本阶段。

## 9. 需要决定的事项与阻断检查

### 9.1 用户已确认的决定

- **D1 停用后的 skill 行为（用户已同意）**：停用只拦新的路由调用，已装skill与项目规则保留；主控确认plugin_disabled后说明“插件未启用”，自行判断档位并继续项目规则或当次用户已要求的分派流程，不暂停询问、不自行启用、不回退旧route.py。没有分派要求的项目不因保留skill自动委派。用户在设计者修订期间回复“同意你的建议”，主控已于main提交712848a记录，本文由主控据此同步。

### 9.2 主控可直接确认的推荐（常规细节）

内置插件列入 Plugins 搜索面板（3.4）；v1 不提供 headless enable（6.4）；TUI 启动自动升级自有旧版（6.4）；response 按契约存完整解析响应的表示、suggestion 存 stdout 原字节（4.4，主控核对 M1 后修订，不再采用原始 HTTP 字节）；不跟随重定向（5.2）；`ureq`（第7节）；说明文案用中文。管理页新增的说明区按项目惯例先给用户看线框（3.4）。

### 9.3 阻断检查

- 不需要改 Corral，也没有依赖反转：dispatch 不调用 Corral；Corral 不知道 dispatch 或遥测；插件不依赖宿主 crate；宿主库不 import 插件。
- 未发现阻断 03A–03C 的矛盾。需要随批准同步的文字：DESIGN.md 与契约里旧称“同进程 `saddle dispatch route`”的段落、契约 §5.1“接入待设计”和“当前只能按 controller.note 提交”的现状描述。
- 悬项：阶段05 前存在 Corral 旧副本与 Saddle 新资源两份 skill，旧副本改动需手工同步；阶段05 前在日常版启用 dispatch 只会报告两处软链接冲突，这是预期，不是失败；`SADDLE_HOST_BIN` 仍未实现，与本阶段无关，留给阶段04。

## 10. 静态核对记录

- 读取：AGENTS.md；DESIGN.md §62 与末尾遥测/dispatch 各节；接入调研；遥测契约全文；实施计划；`src/main.rs`、`src/lib.rs`、`src/app.rs:245-270`、`src/plugins/{mod,registry,runtime,ui}.rs` 相关段、`src/control_cli.rs`、`src/app_control.rs:93-250`、`crates/plugin-protocol/src/lib.rs:275-300`、`crates/plugin-sdk/src/lib.rs:145-200,420-450`、`src/telemetry/{mod,capture,model,cli,storage,blobs,validate,events}.rs` 相关段、`src/agent.rs`、根 `Cargo.toml`（无 panic=abort，`catch_unwind` 可用）。
- 旧资源只读：`../corral/corral-dispatch-skill/{SKILL.md,项目AGENTS模板.md,route.py,README.md}`；`git -C ../corral status --short corral-dispatch-skill` 为空，HEAD `6923da1ee0c766468c31e1f577fb8370b8b6da77`；四个文件 SHA256 与调研记录一致（SKILL `84b0a8cf…`、模板 `22339f46…`、route.py `d4ab1cc8…`、README `f93d2401…`）。目录内另有 `__pycache__/`，未读取、未执行。
- `readlink` 确认 `~/.agents/skills/corral-dispatch` 与 `~/.claude/skills/corral-dispatch` 都指向该目录；未修改。
- 未执行：cargo 构建/测试、route.py、HTTP、Corral/JEV/队列/遥测数据访问、安装或链接切换。上文所有“验证”均为后续实现应做的检查，没有一项已验证。

## 11. 主控核对修订（2026-10-01）

依据主仓库 `docs/任务/遥测03-插件设计主控核对.md`，只改本文与任务完成记录：

- **M1**：撤回 abd1d3c 中“response=最后收到的 HTTP 响应体原字节（含非 2xx）”。response 恢复为契约要求的完整解析响应表示，与整理建议和 HTTP 原始字节都区分；固定了 shape 失败、解析失败、HTTP 失败和重试各次响应的归属（4.4），接口改为 `Captured::{Bytes, Missing}` 以便如实记 gap（2.1）。没有发现必须改用原始字节的理由。
- **M2**：3.2 写明所有路径（含 125 拒绝和参数错误）都有配对起止边界，起始行早于解析与插件执行，两行有界写出，起始行或终止行写不出时业务不受阻、不重放、退出码不变；调用方判定规则与 `saddle agent` 相同。5.4 的 skill 指令改为只按配对回执写原因和记录状态，缺失即未知。8 节 03A 加入对应直接验证项。来源模型不变。
- **S1**：6.3 新增 incomplete，按整套文件判断“完整等于某个版本”；6.5 写明同轮回滚只动哈希可证明为本轮写入的文件，崩溃后混合状态只报告、不标 owned_current、不自动修补。
- **S2**：3.2 给出采集状态优先级，与 `agent.rs` 一致；4.3 相应改写。
- 设计者本轮完成时D1尚待同步；随后主控按用户已确认决定更新第9.1节及5.4，D1不再待问。

主控收敛补记：2c7db0d增量仅两份文档，工作区干净、git diff --check通过；M1完整解析响应与缺失原因、M2起止回执/拒绝/背压规则、S1整版本恢复、S2状态优先级均经静态核对接受。03A界面项移到03B，先做无界面框架；实施次序仍串行集成后一起合并。此为设计结论，不是运行验证通过。
