# 任务：03C 可选dispatch插件、路由与技能资源接入

2026-10-02，saddle/main交给新的Codex实现者（重档gpt-6-astra / xhigh）。
路由：重 / 交叉审查要 / 影响面：碰要害（JEV三项均如此，主控采纳）。
类型：功能变更
依据：03A与03B已审查通过，用户要求继续；按已批准设计完成03C，不切换真实消费者。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的实现者：照本文件做，不再开agent。

## 先读

- AGENTS.md；docs/DESIGN.md末尾dispatch职责和用户确认。
- docs/dispatch插件接口设计.md第2节依赖与现有接口、第4.4/4.5、第5节全文、第6.1/6.7、第7/8节03C、第9节用户决定。
- docs/调研/03C-ureq接口核对-2026-10-02.md；docs/任务遥测接口契约.md第5.1节。
- crates/core-plugin/src/lib.rs；src/plugins/{core,capture,cli}.rs的实际接口，src/main.rs组装根及资源/管理测试；不要重做03A/B。
- docs/任务/遥测03B-独立交叉审查.md末尾最终裁定（M1–M3关闭，S1非阻断暂不改功能，目录清理非原子边界保留）。
- 只读旧来源 `/Users/firegnu/Developer/personal_projs/corral/corral-dispatch-skill/{route.py,SKILL.md,项目AGENTS模板.md,README.md}`；HEAD/哈希见研究记录。

## 在哪里干活

- 分支telemetry-dispatch-plugin，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/telemetry-dispatch-plugin`。从03B审查候选597f48a叠放后合入主控文档；以主控派发时告知的基线为准。
- 允许新增plugins/dispatch及直接测试；修改根Cargo.toml/Cargo.lock、src/main.rs目录组装与必要直接测试；同步README、docs/遥测使用.md、docs/插件系统设计.md及本任务完成记录。插件仅依赖下层core-plugin及成熟通用库，不依赖宿主、Drover或Corral。
- 如通用框架现有接口确有阻断，先报告具体最小缺口，不直接修改03A/B代码；SDK/外部协议/遥测schema与公开行为不变。主控保留03A/B原实施与审查会话处理必要集成返工。

## 要做的

1. `plugins/dispatch`实现CorePlugin，ID dispatch，命令route且capture=Some(Route)，默认停用。仅在二进制组装根登记，宿主库不import插件业务。使用现有headless入口、启停/资源管理，不增加项目采用名单、项目开关、自动AGENTS修改或新界面。
2. 按接口设计5.1逐项移植旧route.py的请求常量、Python strip字符集、顺序/重复键/舍入、置信度/阈值/null兜底、model/usage透传、业务0/1及错误文本边界。完整解析JSON表示保留原键顺序及重复键后值首次位置；不启用serde_json全局preserve_order。只接受5.2已列边缘差异，不自行增加语义差异。
3. 用已核ureq3.4.2/rustls传输，固定生产URL/model，安全TLS、无重定向、分阶段10秒预算、最多2次尝试，重试规则保持5.1。局部传输适配可按研究记录隔离；测试端点/合成key只可通过内部注入，不提供生产环境或CLI换URL入口。读取上限16MiB，无key或非UTF8不发请求；不记录授权头/key，不把库错误Debug整包直接输出。
4. 使用现有Recorder和Completion，按4.4提供begin摘要/实际请求字节、版本/规则指纹、end完整解析response与stdout原字节suggestion；紧挨首次外部调用begin至多一次，重试仍同一operation。不向JEV发送任务书正文；任务书快照由宿主既有机制处理。shape失败仍保存已解析response；非2xx/前次429/截断/无效JSON/超大响应按既定gap分类。未请求采集不建库，遥测失败/关闭不改变业务，不能由插件直接构造遥测事件或访问Store。
5. 复制三份资源到plugins/dispatch/resources/corral-dispatch；模板逐字节不改。SKILL只改5.4指定的问路由命令和兜底，保留技能名、其余编排规则与旧dlog说明（05再处理）；README按5.4更新。停用/未知结果不自行启用、不重试、不回退route.py、不追加询问，依原项目规则/本次明确委派要求继续主控自行定档；完整配对回执判断按设计，不单看末行。资源include_bytes+revision/指纹，插件提供setup_note/setup_files；不要在宿主编码业务说明。
6. 在使用文档说明新入口、采集上下文与业务独立、资源启用/旧链接冲突，以及源码接入不等于真实安装/消费者已切换。将03B候选文档中过宽的“所有路径注明directory kept”收窄，保留S1事实和非原子限制，不顺手修S1功能。产品默认值维持原话注明来源、遥测初始关闭、允许标记晚交。

## 用户要求原话

- “你打算怎么怎么把这个corral-dispatch从corral中拆出来作为sdaale的plugin并且还能支持saddle接下来的遥测？”
- “不但要能支持接到saddle的遥测还要支持用户不要太折腾。”
- “第一版这个插件就在模块安装或者启动的地方写一个说明，让用户知道怎么接入想走主控项目的做法。”
- “如果涉及到要改corral或者发现依赖关系反转了，一定要告知我”。

## 验证预算

- 行为按轻量TDD先有效RED再GREEN，保存实际目标日志与RED测试补丁；编译/fixture失败不算行为RED。纯资源文案不用造RED。
- 限设计第8节03C关键验证：请求常量等价、排序/重复键/阈值和舍入定值、已批准重试/响应gap分类（假传输及回环TcpListener）、无key无begin无请求、摘要/任务书分离、合成key不进入记录回执、资源revision指纹与模板字节一致。完成与03A/B主路径集成一次即可，用临时资源/库，不做额外覆盖矩阵、真实录屏或产品JEV请求。
- 前台标准各一次：cargo test --all-targets；cargo clippy --all-targets -- -D warnings。先设置隔离HOME及XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_STATE_HOME/XDG_CACHE_HOME/XDG_RUNTIME_DIR，保留真实CARGO_HOME/RUSTUP_HOME，固定共享CARGO_TARGET_DIR=/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target，stdin=/dev/null（目标需正文时仅合成输入）。确保新workspace成员纳入标准测试。
- 已有无关失败可单项复跑一次，原失败保留，不声称根因修好；历史registry busy/工作流超时原因未明，不扩旁支。通过后无新改动不重复；返工仅目标/直接回归不全套clippy。git diff --check。

## 不要做

- **需要改Corral或反向依赖先停相关部分报告，不先改后报。** 不编辑Corral/dispatch-log仓库，不改变core Corral技能或实际两处全局链接，不安装技能，不改任一项目AGENTS。
- 不调用真实JEV/Corral进行产品验证，不读取真实密钥值或把它打印到日志；环境中即使有key也不可使用，测试必须合成值或显式移除。依赖下载允许，生产业务请求不允许。
- 不改Drover/Diff/外部SDK/协议/遥测schema，不迁移旧记录、不切消费者，不做04/05。不改原保留worktree，不关闭任何agent。
- 不release/build --release/install，不启动用户正在用的Saddle，不改~/.local/bin；现有安装指向共享target/release，不能覆盖。测试只用隔离HOME/数据，禁止真实队列操作。
- 不引入通用网络框架、后台服务、项目启用管理或额外恢复机制；若已定语义无法在现有接口实现，先报最小缺口。
- 不批量杀进程，只停自己记录PID的进程。不合并main、不推送、不清分支worktree；整阶段集成审查后由主控共同处理。

## 做完

本任务末尾追加完成记录：做了什么、实际RED/GREEN及标准检查、取舍/限制/未做项；在本分支提交。回复SHA与简短结果，命令都在前台跑完，全部完成后最后一行DONE。

## 完成记录（2026-10-02）

### 基线与范围

- 开始时分支 `telemetry-dispatch-plugin`、工作区干净，**当前 HEAD 基线为 `32b34fd21022cec62f3449dfad365d21c096d4e7`**：已从 03B 候选 `597f48affe8b2a5de3ae9a55744b28404a62aa51` 叠放并合入主控文档，以此 HEAD 为本轮增量起点，不把 597f48a 当本轮直接基线。
- 仅本分支新增 `plugins/dispatch` 和直接测试，修改根 Cargo 文件、`src/main.rs` 的目录一行及指定文档。03A/B 通用实现、协议/SDK、遥测 schema、Drover/Diff、Corral/dispatch-log 均未改；没有发现需要改 Corral、反向依赖或通用接口阻断。没有再开 agent。

### 实现结果

- `dispatch` 作为默认停用的 CorePlugin，`route` 声明 `Some(Route)`，只有二进制组装根引用。沿用 headless、管理页、启停、资源和 Recorder/Completion，插件不接触 Store 或自行构造遥测事件。
- 移植固定 URL/model/题目、Python strip、舍入/阈值/null、按响应顺序处理概率与 cross、重复键后值保留首次位置、model/usage 透传与业务 0/1。局部有序 JSON 不启用 serde_json 全局 preserve_order；规则指纹固定为 `sha256:8d4ec37d101d7d6c69d48df828d488b91b68fe751a80c783b6f34911484388b0`，整理/重试版本 `route-v1`。
- ureq 精确 3.4.2，仅 rustls（ring/WebPki），分阶段 10 秒、无总时限、无重定向、最多两次尝试。局部 Transport 适配只累计成功发出的 HTTP 头分隔符及已知请求体长度，未将 Reader 耗尽或 NextTimeout.reason 当成发送完成。响应流最多读取 16 MiB+1 字节判超限；网络诊断使用固定分类文本，不格式化库错误、授权头或 key。
- 首次外部调用前 begin 一次，重试共享 operation。保存实际请求字节、完整解析响应和 stdout 原字节；shape 失败保留解析响应，HTTP/网络/截断/解析失败/超限按约定 gap 分类。任务书仅由宿主快照保存，不进入请求。
- 三份资源 include_bytes，revision=1，资源指纹 `sha256:f51968bfdae9c5891afdc64a32c502db0db11fdf4c759e17ab75295739947918`；模板 SHA-256 `22339f4674469d1b8a5041086f4aa3e5a4b5e9d53f356f946dd1a06cc2f73384`，与旧来源逐字节一致。SKILL diff 仅第3节问路由命令及兜底，保留名称、其他编排规则和旧 dlog 说明；完整回执判定包含首字节、完整 LF 首末行、schema/final/UUID 与 call_id 配对。README 和插件接入说明同步，旧软链接冲突、源码接入与实际迁移分开说明。
- 使用文档收窄了“所有保留目录路径都会注明 directory kept”的过宽表述；03B S1 提示遗漏仍未修复，身份核对与 rmdir 的非原子边界保留。原话注明来源/未经独立核验、遥测初始关闭、有标记晚交的产品默认值保持。

### 实际验证及证据

所有测试/Clippy 命令均通过前台 runner 等待退出；隔离 HOME 和 XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_STATE_HOME/XDG_CACHE_HOME/XDG_RUNTIME_DIR，保留真实 CARGO_HOME/RUSTUP_HOME，固定共享 target，stdin=/dev/null（测试子进程仅合成正文）。外层显式移除 TYPESAFE_API_KEY，测试只内部注入合成值；仅本地回环 HTTP，无真实 JEV/Corral 请求。

证据目录：`/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-03c-8a69a7d0/`。包含 `baseline.txt`、`run.py`、各命令 `.json/.log`、`results.json` 和实际 RED 补丁。

| 检查 | 实际结果 / 日志 |
|---|---|
| 目录登记 RED→GREEN | `red-catalog.log`：目标因 unknown_plugin、退出2≠0失败；`green-catalog.log`：1通过。`red-catalog.patch` 为纯新增测试。 |
| 无 key RED→GREEN | `red-key.log`：错误文本 route unavailable≠约定 key 文本；`green-key.log`：1通过。`red-key.patch` 保留当时测试与可编译脚手架。 |
| 请求/排序/重复键/舍入 RED→GREEN | `red-route.log`：合成成功响应仍返回业务1，非编译/fixture失败；`green-route.log`：2通过。`red-route.patch` 保留当时测试及路由脚手架。请求期望从旧脚本 AST/literal 提取，未 import/执行旧脚本；舍入定值另以本机 Python round 核对。 |
| 429/529 重试 RED→GREEN | `red-retry.log`：仅1次尝试≠2；`green-retry.log`：3通过。`red-retry.patch` 为当时源码和新增测试快照。 |
| 网络阶段分类 RED→GREEN | `red-network.log`：发送前 reset 未标可重试；`green-network.log`：4通过。`red-network.patch` 保存该测试。 |
| 插件最终直接回归 | `cargo test -p saddle-dispatch-plugin`：**14通过，0失败**，`plugin-final-target.log`。含规则/资源固定指纹、模板、边界与舍入、无key/非UTF8无begin、两次尝试上限、假发送失败与实际发送完成标记、回环请求/429/529/重定向/非2xx/截断/无效JSON/超大响应/头与体超时。 |
| 03A/B/03C 主路径隔离集成 | `cargo test --test dispatch_plugin`：**3通过，0失败**，`integration-green.log`。真实插件清单接管理层及真实二进制默认停用/无key路径；成功采集由测试宿主编译同一份 route/json/rules 源码，仅 HTTP 边界注入合成响应。验证启用资源、外来旧链接不动、模板路径、未选采集不建库、宿主快照/来源/operation、摘要与任务书分离、response/suggestion原字节、记录及回执无合成key、停用及移除。未提供生产换URL入口。 |
| 标准测试（仅一轮） | `cargo test --all-targets`：**exit101，已执行96通过/1失败**，`standard-test.log`。既有 `core_plugins::management_switches_preserve_external_entries_and_enforce_reserved_ids_and_baseline` 在 `tests/core_plugins.rs:640` 报 `plugin registry busy; refresh and retry`；Cargo 随后停止，后续 target 未执行，不能说全套通过。新的 workspace/default-members 已包含 dispatch；其14项目标结果单独如上。 |
| 获准的单项复跑（仅一次） | 对上述用例 `--exact` 复跑：**1通过，exit0**，`registry-single-rerun.log`。未改该测试或框架，保留原失败；不声称锁竞争根因修好，不继续扩查/重跑全套。 |
| 标准 Clippy（仅一轮） | `cargo clippy --all-targets -- -D warnings`：**exit0**，`standard-clippy.log`，新成员已被检查。 |
| 文档/资源静态检查 | `git diff --check` 通过；模板与旧来源 byte diff 为空；SKILL 仅规定区域变化；生产源码引用 dispatch 仅 `src/main.rs`。 |

未计为行为 RED 的过程产物也保留：`loopback.log`/`loopback-green.log` 是 ConfigBuilder 私有类型的编译调整；`integration.log` 是测试 catalog 的静态生命周期调整。`boundaries-green.log`、`response-limit-green.log`、`response-limit-diagnose.log` 的超限断言失败来自假服务 accepted socket 继承非阻塞状态，大响应未发完；仅修 fixture 为阻塞 socket 后 `loopback-fixture-green.log` 4通过，生产读取仍用有界 as_reader。其 `red-response-limit.patch` 名称沿用当时文件名，**不作为有效行为 RED**。不将这些失败日志删除或冒充产品缺陷证据。

### 取舍、限制与未做项

- 现有接口足够，没有新增通用网络框架、后台服务、项目采用管理或恢复机制。HTTP 适配采用已批准的 ureq unversioned 公共接口并精确固定版本。资源/规则指纹规范及边缘差异均按既定设计，没有新增设计决定需主控裁定。
- 主路径宿主集成的成功 HTTP 为内部假传输，真实 ureq 单独用回环验证；不冒充真实 JEV/TLS 或代理环境验证。标准测试因上述既有锁竞争未全绿，此事实应带入独立审查。
- 没有发布/release/安装、改全局技能链接或项目 AGENTS、切消费者、迁移旧记录、实施04/05、操作真实队列、启动日常 Saddle、合并/推送/清理 worktree 或关闭任何 agent。其他 worktree 与03A/B保留会话未动；仅在本分支提交，交主控后续审查。

## 第一次限定返工完成记录（M1，2026-10-02）

### 基线、裁定与实现

- 已读主仓库 `docs/任务/遥测03C-独立交叉审查.md` 首轮审查及末尾“主控裁定与第一次限定返工”，接受 M1。此次从原分支干净 HEAD `1692be22eb1c516a433dcc1a774c683486766822` 继续；首轮叠放基线仍是前文的 `32b34fd21022cec62f3449dfad365d21c096d4e7`（03B 597f48a 加主控文档），没有切分支或操作其他 worktree。
- **纠正首轮完成记录的发送完成结论**：旧观察器在 TLS 之外，仅凭 `transmit_output` 成功推进明文计数不足以证明底层写入完成。锁定的 rustls `Stream::write` 会接收明文后隐藏 `complete_io` 的写错误，审查反例成立；旧 HTTP/假 Wire 绿测不能证明这一点。
- 修复仅在 dispatch 局部适配：用 ureq 公开 `ConnectProxyConnector → TcpConnector → RustlsConnector` 保留当前启用的传输顺序，在 TLS 下方增加 `WriteChecked`。每条连接仅保留写错误种类；TLS 返回成功后、提交 HTTP 进度和 `sent` 之前，显式传播被隐藏的错误。正常向上传播的错误保留原值，隐藏超时保留 timeout 分类，诊断不保存或格式化授权头/key。CONNECT 递归连接使用独立发送标志，避免代理握手误标目标正文完成；未增加代理能力或声称代理环境已验证。
- 真正完整写入后接收非超时错误仍不重试；两次尝试上限、begin 一次、首次 429/529 一秒退避、其他业务规则及 response gap 接线未改。固定生产 URL、分阶段预算、无重定向及生产 TLS 验证保持；新增断言检查生产配置未关闭证书验证。“发送完成”仍不等于服务端确认收到/处理。
- 公开接口足够，无需改 Corral、外部库源码、宿主/SDK/遥测 schema、03A/B 或依赖方向。只增加测试依赖 `rustls =0.23.45`，沿用锁文件中原有版本及 ring/std/tls12；Cargo.lock 仅将该既有包列入 dispatch 的依赖，不升级或新增库版本。README 同步真实实现与验证边界；C1–C5、03B S1 暂不修复及目录清理非原子边界继续保留。

### 实际 RED/GREEN 与直接回归

证据目录：`/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-03c-m1-uqcqtds1/`。包含本轮 `baseline.txt`、`run.py`、命令 `.json/.log`、`red-tls.patch`、`final.patch`、`results.json`。所有命令前台等待退出；每轮测试均隔离 HOME 及全部五个 XDG 目录，保留真实 CARGO_HOME/RUSTUP_HOME，固定共享 target，stdin=/dev/null，外层移除真实 TYPESAFE_API_KEY。

| 检查 | 实际结果与证据 |
|---|---|
| M1 有效 RED | `cargo test -p saddle-dispatch-plugin transport::tests::tls`：exit101，1通过/1失败。先添加审查同类合成 TLS 测试，生产适配仍为 1692be2 原文；真实 ureq/rustls 内存握手及 HTTP 头完成后，拒绝正文 TLS 记录。断言先确认写失败确实注入且正文交付为0，再检查尝试次数。失败显示 `(body=1955, delivered=0, sent=true, retry=false)`，实际1次≠期望2次；完整发送后接收 reset 的对照测试通过。`red-tls.log`、`red-tls.json` 与实施前保存的完整 `red-tls.patch` 对应，不是编译/夹具错误。 |
| 插件 GREEN（仅一轮） | `cargo test -p saddle-dispatch-plugin`：exit0，**17通过/0失败**，doc-tests 0项；`green-dispatch-unit.log/.json`。TLS 正文 BrokenPipe：每次交付0、sent=false/retry=true、两次尝试且 begin 一次；隐藏 TimedOut：同样最多两次、保留 `network: timeout`；完整正文交付后 ConnectionReset：sent=true/retry=false、一次尝试。三者均检查无状态退避、not_available gap、业务退出1及 stdout 无合成 key。既有 HTTP 状态/429/529/响应 gap/头体超时、请求整理、资源及 Recorder 目标同时通过。 |
| 直接集成 GREEN（仅一轮） | `cargo test --test dispatch_plugin`：exit0，**3通过/0失败**；`green-dispatch-integration.log/.json`。沿用原有真实登记/停用/无key和同源码业务采集集成，未扩大为其他目标。 |
| 静态检查 | `git diff --check`、限定 Rust 文件 rustfmt 检查通过；两份合成 TLS fixture 与 ureq 3.4.2 发行包对应文件逐字节一致，具体哈希及结果保存在 `results.json`。 |

TLS 测试使用 `plugins/dispatch/tests/fixtures/tls-{cert,key}.pem`，来源为 ureq 3.4.2 `src/unversioned/transport/testdata/{cert,key}.pem` 的公开夹具，和审查探针同源。内存服务端不调用 DNS/socket；只在该测试配置关闭夹具证书验证。GREEN 使用修复后的同一个 `Observe` 接入实际 `RustlsConnector`，未用假 TLS 代替目标行为。没有真实 JEV、真实 TLS 服务、真实凭据或 Corral 请求；HTTP 回环仅既有直接目标。生产 TLS 开启的配置断言不等于真实证书链/代理整链路验证。

### 验证限制与交付边界

- **标准测试未全绿的缺口仍独立保留**：实施首轮96通过/1失败，主控179通过/1失败/1忽略，分别在两个不同既有用例报 registry busy；单项通过不能证明根因修复，也不能将两者认定同根因。前文“锁竞争”措辞不代表已确认根因。本轮17+3项目标通过不替代标准全套，不自行豁免主控合并前裁定。
- 本轮未跑全套、Clippy、core_plugins、plugin_resources 或 plugins；没有历史 registry busy 扩查、重复绿测、额外覆盖矩阵或重建历史 RED。没有修改主仓库审查文档、其他 worktree、外部库源码或 Corral，没有再开 agent。
- 仅在原 `telemetry-dispatch-plugin` 分支提交本修复和记录；不合并/推送/清理/关闭，不 release/安装/启动日常 Saddle，不动真实遥测/队列、消费者、04/05。等待主控限定复核及独立处理标准验证缺口。

## 合并前集成验证限定修正完成记录（2026-10-02）

### 基线、范围与锁修正

- 已读主仓库 `遥测03C-集成验证修正.md`、标准验证缺口核查末尾主控裁定及独立交叉审查的补跑结果。从本分支干净 `6691200c7af1001c60fd50fc6bcc77bda6682596` 起步；M1保持关闭。本轮生产改动仅 `src/plugins/registry.rs`，另改局部锁测试、两个原失败用例、10项workflow的公共定位 helper/必要步骤及本记录；未改主仓库任务文件。
- Registry 成功取得原非阻塞排他锁后建立局部 `RegistryLock` guard。成功提交及 baseline/read/落盘等错误退出时，guard先调用 `LOCK_UN`，文件随后关闭，避免正常事务结束后仍被fork子进程的继承引用延长持有。稳定lock文件、baseline检查、临时文件/sync/persist和内存提交顺序均保留；未增加等待/重试、删除锁文件或通用锁框架。
- 解锁采用Drop中的最小best-effort调用，保持原写入结果，不把已持久化成功重新判为失败，不覆盖原业务错误；文件close仍随后发生。未新增对外解锁错误返回承诺，也不宣称极端系统错误/真实竞争从此不会busy。申请失败立即保存真实系统错误：仅 `ErrorKind::WouldBlock` 保留原busy文案，其他失败保留 `io::Error` 原因及固定上下文，不读取/打印配置内容。
- `tests/plugins.rs` 的stale add及过时remove、`tests/core_plugins.rs` 的stale core_enabled改为精确检查 `plugin registry changed; refresh first`。手工持锁仍精确验证busy，结束时显式 `LOCK_UN` 再drop；没有用串行测试或延时掩盖竞争。
- 新测试通过实际 `Registry::core_enabled → write` 验证成功/错误退出。唯一测试接缝是 `#[cfg(test)]`、线程局部且一次性消费的锁后hook，用本次真实锁fd启动自建 `/usr/bin/true`，以带5秒上限的屏障固定fork后的exec前窗口。父事务返回后、子进程尚未exec时再调用实际Registry写入，随后放行并等待子进程结束；不是在复制锁算法上GREEN。非busy分类测试对同一申请函数执行真实 `flock(-1)`，验证保留EBADF，不构造非法File/OwnedFd。

### 10项workflow逐项判断与适配

已逐项读取主控原 `saddle-03c-gap-controller-f9e9h7xt/saddle.log` 的失败位置/屏幕，并对照 Palette、管理页和窗口保护调用。下列均是过时测试假设或文本定位问题；限定目标验证后未发现必须修改生产UI的证据。Dispatch仍默认停用并正常显示，未隐藏注册项、跳过用例或放宽生命周期/布局断言。

| 目标 | 原失败原因与最小适配 | 保留的保护 / 最终结果 |
|---|---|---|
| `drover_plugin_opens_in_a_split_and_closes_only_its_view` | split picker/重开时默认首项成为Dispatch，Enter去管理页；明确选择公开action标题 `Tasks` 后Enter。 | 分屏两pane、关闭仅恢复原布局、合成队列不变、随后可重开overlay；通过。 |
| `drover_plugin_palette_form_and_background_lifecycle_never_use_old_cli` | 关闭任务视图后再开palette，原先默认首项可直接重开Drover的假设失效；选择 `Tasks`。 | 表单输入、退出后后台生命周期、旧CLI无业务写入断言全部保留；通过。 |
| `plugin_entry_opens_overlay_without_changing_layout_and_keeps_process` | 默认选中Dispatch而非fixture；`open_fixture_palette` 按可见标题选择Fixture Counter并等待选中标记。 | overlay几何/边框、focus、原tabs、保留键及只启动一次/计数状态保留；首轮通过。 |
| `plugin_manager_details_follow_the_visible_entries` | 原鼠标定位先命中Dispatch说明里的“Manage plugins”，不是footer按钮；先明确选择fixture，说明消失，再点原按钮。 | 管理页跟随所选ID，详情与最后可见entry相差2行的原断言保留；首轮通过。 |
| `plugin_manager_hides_underlying_cursor_but_keeps_directory_input_and_settings_cursor` | 最后一次打开管理页也误点同一说明文字；同一窄helper修正。 | 管理页隐藏cursor、目录输入显示cursor、返回Settings恢复cursor；首轮通过。 |
| `plugin_overlay_protects_host_actions_and_restores_viewer_focus` | 原Enter激活默认Dispatch；明确选择fixture。 | overlay期间ctl布局动作返回busy、宿主按键/点击受阻，Esc恢复viewer与active pane/tabs；首轮通过。 |
| `plugin_palette_empty_and_settings_are_not_replaced` | 无外部视图不再等于无注册项；明确检查Dispatch及Built-in Disabled，使用无匹配查询验证Enter不打开视图、focus/tabs不变，再走管理入口。 | Settings不能被背后Plugins替换、搜索/Enter不泄漏到agent终端、Esc恢复viewer；通过。 |
| `plugin_palette_switches_overlays_and_blocks_background_layout_writes` | 初始Enter误选Dispatch；先定位fixture，后续Other/Fixture搜索切换保留。 | palette期间ctl busy、关闭恢复原overlay、两个进程各启动一次、点击计数/原tabs保留、管理页跟随所选ID；首轮通过。 |
| `plugin_split_picker_cancels_opens_and_moves_one_live_view` | 默认首项假设，以及“当前pane不可再分到自身”时列表必为空的假设过时；明确选择fixture，当前视图已被过滤时查询 `test.entry` 并验证无匹配/Enter保持palette与原tabs。 | 取消不改布局、Move复用同一pane/进程、计数保留、关闭恢复agent输入、默认重开overlay；通过。 |
| `plugin_workspace_palette_reuses_panel_and_disable_blocks_open` | 默认首项及F5管理页默认行假设失效；明确选择fixture和管理页中的Entry fixture行，避开背景同名tab，停用后再次选择该项验证Enter不打开。 | 复用原panel/tabs、进程只启动一次、停用后不能打开；通过。 |

公共 `open_fixture_palette` 只被本轮失败目标使用；没有改整个Harness的点击规则。管理页行定位限定在Settings边框内，选择不依赖“第一行”。空搜索框的占位文字与非空查询分开判断，非空时检查无匹配状态、palette焦点及布局不变。

### 实际RED/GREEN、失败保留与限定回归

证据目录：`/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-03c-integration-fix-jodmed7i/`。含 `baseline.txt`、前台 `run.py`、各命令完整 `.json/.log`、RED补丁、`workflow-first.patch`、最终diff与汇总。每条命令等待退出；隔离HOME/全部五个XDG、保留真实CARGO_HOME/RUSTUP_HOME、固定共享target、stdin=/dev/null、外层移除TYPESAFE_API_KEY。仅合成材料、假Corral和自建短命进程。

| 检查 / 证据 | 实际结果 |
|---|---|
| 成功退出有效RED：`red-lock-lifecycle.log/.patch` | 未修实现的实际Registry成功返回后，子进程仍在exec前，第二次写返回busy，目标失败。首次并行运行共2失败；其中stale用例在夹具初始化的更早锁窗口报busy，**不将这条早期断言失败冒称错误退出的目标RED**。 |
| 错误退出有效RED：`red-lock-stale.log/.patch` | 夹具改为直接合成外部编辑以失效baseline，避免先取一个额外锁；只精确运行stale目标一次。确认首写返回changed后，下一次实际Registry写入仍因继承引用busy而失败，exit101。这是目标RED，没有用反复碰运气替代定位。 |
| 生命周期GREEN：`green-lock-lifecycle.log/.json` | 两个新目标2通过，均在子进程exec前完成下一次实际写入，确认最终状态及lock文件仍在。该RED/GREEN的 `cargo test --lib plugins::registry::tests::` 默认workspace命令也列出其他lib harness，但它们实际执行0项；随后所有命令显式 `-p saddle`，没有跑其他套件用例。 |
| 错误分类RED→GREEN：`red-lock-error.log/.patch`、`green-lock-error.log` | 将原申请代码机械提取为局部函数后，真实EBADF在旧逻辑被折叠为busy，缺失预期errno9，exit101；最小分类修正后该目标1通过。原WouldBlock文案由后面的手工持锁直接回归验证。 |
| 原plugins失败用例：`green-registry-original-plugins.log/.json` | `cargo test -p saddle --test plugins registration_defaults_disabled_and_concurrent_edit_is_not_overwritten -- --exact`：1通过，仅一轮。 |
| 原core_plugins失败用例：`green-registry-original-core.log/.json` | `cargo test -p saddle --test core_plugins management_switches_preserve_external_entries_and_enforce_reserved_ids_and_baseline -- --exact`：1通过，仅一轮。 |
| 10项workflow首轮：`workflow-ten.log/.json`、`workflow-first.patch` | 通过libtest多个完整名称加 `--exact`，仅选定10项，94项过滤；5通过/5失败，exit101。5个新失败分别来自本轮定位/期望错误：两项使用Drover名称而实际action标题为Tasks，两项有查询文字后仍等Search plugins占位符，一项点击背景同名tab而非管理页行。原始日志完整保留，不算新的生产UI缺陷。 |
| 修正后仅5项：`workflow-five-corrected.log/.json` | 对上述具体错误改定位/断言后，只选这5项精确运行一次，5通过/0失败，99项过滤。已通过的另5项未重跑；最终10项各有通过结果，没有workflow整套或无改动循环重跑。 |
| 静态核对 | 限定Rust文件rustfmt检查、`git diff --check`通过；dispatch/M1、生产UI/Palette/App及其他模块diff为空。 |

### 结论、未知与交付边界

- 本轮独立目标最终通过：新增锁目标3项、两个原用例2项、workflow指定10项，共15个不同目标。作用域解锁关闭的是已实证的继承引用窗口；原开发/主控两次busy没有历史errno/持锁者证据，**仍不能宣称原两次已证明同因**，尤其core_plugins原次原因继续按未知保留。
- 主控原补跑290通过/10失败/4忽略和原标准exit101均是保留的历史事实；本轮限定修正结果不篡改它们为原运行全绿，也不宣称重新执行了标准全套。未跑Clippy、标准全套、workflow整套或额外覆盖矩阵，未重开M1或已接受设计。
- 没有修改生产UI、dispatch传输、Corral/外部库、Drover业务、依赖方向、遥测schema及主仓库文档；没有发布/安装、真实服务/数据/队列、消费者切换、04/05、合并推送、清理worktree/agent。仅原分支提交，交主控和原独立审查者限定复核后再裁定合并。


## 阶段03最终审查与合并结论（2026-10-02）

本任务与03A/B/C串行集成整体通过主控及独立审查，已合并main，合并提交 `a1fd832e109936a940dec6d5b2fdd2b8aaac775f`。最终功能候选5df616d；03A0fa62cf、03B597f48a均在其祖先链中。主控完整验证裁定见 `docs/任务/遥测03C-独立交叉审查.md` 最后“主控最终裁定与阶段03合并放行”：原标准执行、补齐30遗漏target、发现问题的修正及定向复核共同形成放行依据，不冒称最终候选完整标准全绿。最后独立15目标全部通过，剩余必须改0；TLS M1及锁/10项workflow修正关闭，03B S1与非原子目录清理限制保留，历史两busy未证同因。未release/安装/操作真实数据队列或切消费者，不推进04/05。实际推送/清理/关闭结果由HANDOFF及dlog收尾记录承载。
