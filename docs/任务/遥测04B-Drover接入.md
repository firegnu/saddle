# 任务：04B 通用遥测跳转与Drover接入

2026-10-02，saddle/main交新的Claude Code实现者，重档opus[1m]/xhigh。
路由：重 / 交叉审查要 / 影响面：碰要害（JEV重、其余拿不准；主控因任务交付结果/禁止重复发送、取消与状态落盘边界判要/碰要害）。
类型：功能变更
依据：04A查询UI候选0261d51已通过主控核对，用户已批准04布局和项目默认关/本次覆盖，按设计串行完成04B。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的实现者，照任务做，不再开agent。此任务包含既有Drover页面控件接入，交Claude整体完成，避免界面与业务同时改相同文件。

## 先读

- AGENTS.md；docs/DESIGN.md末尾04批准；docs/遥测查询与Drover接入设计.md §5–7、§2焦点规则；docs/任务遥测接口契约.md §2.2/5.2/7和开关、来源约定；docs/遥测使用.md相关公开命令。
- docs/任务/遥测04A-主控审查.md最终结论及“外部预填控制字符留04B”事项，M1–M3已关闭不重做。
- crates/plugin-{protocol,sdk}及src/plugins/runtime.rs、src/app_plugins.rs中的agent.open模式，src/telemetry_view.rs的Page::open及筛选表单。
- plugins/drover/src/{core,drover,command,queue,plugin}.rs直接相关路径与README；先核现有任务身份/token/run、交付结果和状态持久化规则，不通读所有历史审查。

## 工作位置

- 分支telemetry-drover-integration，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/telemetry-drover-integration`。主控从04A的0261d51建好并合入main文档，实际基线见派发提示。
- 允许修改crates/plugin-protocol、crates/plugin-sdk、src/plugins/runtime.rs及必要通用接口、src/{app,app_plugins,telemetry_view}.rs的接线/筛选显示、plugins/drover内本任务业务/配置/页面及plugin.toml、直接相关测试；docs/{插件协议,遥测使用}.md、Drover README和本任务完成记录。
- 不改04A原worktree。04A实现者与分支继续保留，04B完成后整阶段集成审查再合并清理；不自行派发或清理。

## 要做的

1. 按设计§5.1实现通用`telemetry.open.v1`/`telemetry.open`与SDK调用：当前用户输入身份、每次输入一次、既有取消/超时/返回语义；参数为不透明BindingFilter，宿主无插件ID分支或Drover数据读取。筛选校验与Store相同，消息帧预算沿用协议，不能复制agent身份256字节限制。打开04A页面，关闭恢复发起处；不破坏agent.open/其他插件请求。
2. 宿主向所有进程插件提供`SADDLE_HOST_BIN`当前宿主可执行绝对路径，不携带记录上下文。插件通过公开Saddle CLI调用遥测与agent，不能链接宿主内部Store或读取SQLite/遥测业务文件。运行环境不可用如实返回采集状态，不自行寻找旧dlog替代。
3. Drover按已批方案：项目配置默认记录关闭，可保存项目默认选择；Dispatch selected旁提供本次覆盖，ctl dispatch支持可选record，未给覆盖时取项目默认。沿用现有项目/任务页面控件风格，任务详情有编号时“Telemetry ↗”传kind/scope/key（不带run，列出所有轮次），未编号不误关联；无需启用dispatch插件。该设置仅控制记录，不控制项目是否采用主控分派。
4. 已选择记录时，按契约§7和设计§5.2生成本次run/trace/controller_handoff身份，经公开CLI创建；总开关关闭/初始化不可用/预算耗尽在业务发送前按原路径交付，明确没有有效记录上下文。遵守300ms身份准备预算、不自动开启总开关。没有选择记录时保留原路径，不为交付创建遥测库。
5. 有效上下文作为独立附段加入原交付消息，临时上下文文件0600、`send_kind=initial`，经`Saddle agent --corral <原程序路径>`单次send；无MAIN_AGENT沿用manual_text。上下文不增加授权，不替换任务书、不把配置程序变成带空格的复合命令。
6. 该路径按契约独立进程组取消；起始字节0边界与末行终止同call_id配对后才采用executed判定，125/127数字本身不证明未执行。已启动agent后采集失败/超时/缺回执/错配/end保存失败都不得降级再发；保留Corral公开结果与现有confirmed/unconfirmed/rejected/unknown映射，未知不自动重发。采集与业务错误分开，不因遥测失败覆盖真实交付结果。
7. 先按现有Drover规则持久化任务状态，再append task.transition，记录真实from/to与business_committed_at。初始派发、后续提交审查/验收/退回都按完整项目/任务/run经公开查询找到对应trace，不猜最新链路、不在Drover数据新增trace_id；未采集不补造。操作结果保留delivery/record，另给telemetry状态；双存储不宣称原子。
8. 完成04A待接入事项：外部预填binding含控制字符时保留原始值，筛选框安全可见地显示，不静默丢失/改写、不错误查询另一项目；普通输入编辑与原筛选行为保持。同步文档说明可选记录、失败边界及旧入口尚未切换。

## 用户原话

- “当时做dispatch-log的原因就是要记录下每一个任务的流转状态。以便积累数据以便以后分析路由的正确性。”
- “另外这个遥测还可以独立关掉，也是一个需求”
- “如果涉及到要改corral或者发现依赖关系反转了，一定要告知我”
- 用户对主控展示的查询布局、Drover任务关联跳转及“项目默认记录关闭、派发时本次覆盖、宿主总开关仍控制采集”的方案回复：“批准”。

## 验证预算

- 目标行为按AGENTS轻量TDD先有效RED再GREEN；只针对本任务与直接边界，不造覆盖矩阵。保留真实目标输出及必要RED桩差异；编译/夹具错误不算RED。
- 合成Drover项目/队列和假Corral、临时HOME/XDG，验证公开接口与真实候选debug宿主的接线；不得调用真实agent或产品JEV。关键是单次交付/unknown不重发、关闭或初始化失败、业务落盘与采集失败分离、请求身份/导航返回、项目选择及04A关联筛选主路径。
- 前台标准各一次：`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`，共享 `CARGO_TARGET_DIR=/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target`。临时HOME和XDG_CONFIG_HOME/XDG_STATE_HOME/XDG_DATA_HOME/XDG_CACHE_HOME/XDG_RUNTIME_DIR，保留真实CARGO_HOME/RUSTUP_HOME，stdin=/dev/null、移除TYPESAFE_API_KEY。
- 疑似无关失败仅单项复跑一次，原失败与未执行目标保留，不宣称根因修好，不扩修。通过且无新改动不重复；返工只目标及直接回归，不重复全套/Clippy。git diff --check。

## 不要做

- 需要改Corral或反向依赖先停相关部分报告，不得先改后报；不改外部仓库/旧服务、存储schema、01/02/03已定采集协议或资源安装规则。现有公开能力不足先列最小缺口，不能藏在实现里变更契约。
- 不删旧Drover Dispatch/dlog入口、不导入旧日志，不切消费者或推进05；不统计评分/导出/自动分析，不让宿主管理项目分派名单或改项目AGENTS。
- 不release/build --release、安装、启动日常Saddle，不写真实技能链接/遥测数据/任务队列，不动用户agent/其他worktree，不批量杀进程。合成验证只停自己记录的进程。
- 不合并推送、清理worktree分支或关闭其他会话。04A与04B集成后由主控一起收尾。

## 做完

本分支提交，任务末尾追加实际改动/检查与日志/取舍/限制和未做事项。回复SHA、简明结果及需主控决定项；命令前台结束，全部完成后最后一行DONE。

## 完成记录（实现者，2026-10-02）

基线03f7282（含04A 0261d51与主仓库文档）。实施中一次API连接中断，续做时核对未提交改动与已跑命令后接着做，没有复位或丢弃改动；中断前只做过基线编译和耗时测量，无已通过检查被跳过。

### 实际改动

- **协议/SDK**：能力表加 `telemetry.open.v1`；protocol新增 `TelemetryFilter{kind,scope,key,run?}`（未知字段拒绝）。SDK `Context::open_telemetry` 只在输入回调内可用，与 `open_agent` 共用“每次输入一个导航”，排队前按既有 4 MiB 帧上限编码检查；`Event::TelemetryOpened`，20 秒无回复为 unknown，不重试。
- **宿主**：`runtime.rs` 处理 `telemetry.open`：当前会话最近输入、与agent.open共用输入高水位；筛选用 Store 同一规则 `BindingFilter::validate()`（由 `list_bound` 原有校验抽出，行为不变），不设256字节等额外限制、不改写控制字符；错误为 stale_input/invalid_filter/busy，未声明能力仍 unsupported。所有进程插件获得 `SADDLE_HOST_BIN=current_exe()` 绝对路径（取不到则删除继承值），无其他上下文。`app_plugins.rs` 在来源视图仍开且无其他对话框时以该筛选和清单显示名打开04A页并回 opened，否则 cancelled；页面打开期间覆盖插件失焦，关闭回原处。宿主无插件ID分支、不读Drover数据。
- **04A待接入项**：`telemetry_view.rs` 表单对含控制字符的预填值保留原值、以 `\x1b`/`\n` 可见转义显示并精确查询；该格输入/粘贴/删除即整格替换（有提示行），其余格编辑与原筛选行为不变。
- **Drover**：`.drover.conf` `TELEMETRY_RECORD=on|off`（缺省关）；`Record default`按钮/键R保存；Dispatch selected旁 `[x]/[ ] Record` 本次覆盖（换选任务回到默认，失败保留）；ctl `dispatch` 可选布尔 `record`，`list` 返回 `record_default`。新增 `telemetry.rs`：300 ms墙钟内经公开CLI建 trace（binding drover.task/规范化项目根/任务号/run）与 controller_handoff；失败（disabled/unavailable/host_unavailable/budget_exhausted等）在发送前走原路径并写原因。有上下文时消息末尾加独立附段（仅trace/dispatch/task/run，注明非任务内容非授权），0600临时上下文 `send_kind=initial`，`$SADDLE_HOST_BIN agent --corral <原值> --record-context … -- send` 单次执行；无MAIN_AGENT时附段进manual_text。`command::run_group` 独立进程组，取消/60秒超时 TERM→≤250 ms→KILL，信号均在回收组长前发出；退出且两路输出收齐才解析回执，字节0首行与末行同call_id配对才采信executed。配对executed=false→新状态 `not_executed`；executed=true按原映射；缺失/不配/超时/取消→unknown；入口启动后不回退直发、不重发。先写Drover start，成功后append task.transition（from/to真实值、business_committed_at=Drover记录时间）；提交/接受/退回按完整binding `telemetry list` 找到该轮trace再追加，无trace报not_recorded，不在Drover数据存trace_id。结果保留delivery/record并加 `telemetry`。直发与agent入口两条交付路径的子进程环境去掉 `SADDLE_HOST_BIN`。任务详情页签行对有编号任务显示 `Telemetry ↗`（kind/scope/key，不带run）；plugin.toml声明 telemetry.open.v1。
- **文档**：插件协议§11、遥测使用（插件跳转、Drover可选记录、旧入口未切换）、Drover README。

### 检查与日志

日志目录：`/private/tmp/claude-501/-Users-firegnu-Developer-personal-projs-saddle-worktrees-telemetry-drover-integration/8944a1f9-783a-4263-bfe7-f2942911793c/scratchpad`；隔离脚本 `iso.sh`（临时HOME与五个XDG、真实CARGO_HOME/RUSTUP_HOME、stdin=/dev/null、去掉TYPESAFE_API_KEY、共享target）。

- RED（桩可编译、断言失败）：`red-drover-telemetry.log` 8项（telemetry字段为Null等）、`red-drover-manual.log`、`red-drover-ctl.log`（record_default缺失）、`red-host-runtime.log`（telemetry.open返回unsupported、环境无SADDLE_HOST_BIN）、`red-view-control.log`（表单显示 `T1[31m`：吞掉ESC并隐藏换行后内容）、`red-drover-ui.log`（无按钮）、`red-e2e.log`（真实saddle+真实Drover插件中无 `[ ] Record`/`Telemetry ↗`）。桩差异：`red-stub/tracked.diff` 及当时的 `telemetry.rs`、测试文件。`red-drover-process.log` 是在等待新帧的 `see` 处超时，该等待写法本身有误（首帧已含标签不会再出新帧），GREEN前改为直接读当前帧；按钮缺失由ui RED独立证明，此项不计为严格的有效RED。
- GREEN：`green-drover-telemetry.log` 9/9（含未选不调用宿主/不建目录、disabled/无宿主/存储不可用各只直发一次、记录路径一次发送且正文全量可查、缺回执/不配对unknown不重发、真实入口无法启动Corral配对executed=false、Corral自身125/127为unknown、取消时进程组内Corral客户端被停、关闭总开关后流转报disabled与start写失败不补transition、默认值写入配置）、`green-host-runtime.log` 2/2、`green-view.log` 12/12、`green-drover-ui.log` 2/2、`green-drover-ctl.log`、`green-drover-process.log`、`green-e2e.log` 2/2（真实debug宿主+Drover插件+假Corral：跳转页、Esc回Drover、记录派发经agent入口一次送达并可经CLI查询）。e2e首轮第二次点击与Run details刷新竞争，按既有测试做法等详情加载后再点。
- 标准各一次：`cargo test --all-targets` exit 101（`std-test.log`）：根包32个目标412 passed/1 failed/5 ignored（主控据原日志纠正原写4），失败为未改动的 `pending_delete_button_confirms_names_the_task_and_can_be_cancelled`（测试读队列时插件正在写，`core::list` 报target_changed），单项复跑一次通过（`rerun-pending-delete.log`），不宣称根因已修。cargo因此未执行其余6个工作区包，随后对这些未执行目标各跑一次（`std-test-remaining.log`）：22个目标120 passed/0 failed。合计54个目标均执行一次。`cargo clippy --all-targets -- -D warnings` exit 0（`std-clippy.log`）。`cargo fmt --check`、`git diff --check` 通过。

### 取舍与限制

- 300 ms为两次CLI调用的墙钟预算（含进程启动）。实测新编译debug二进制在macOS首次执行约3.3 s（之后<10 ms），刚安装后的首次记录派发可能因此以budget_exhausted走原路径（无记录、业务照常）。
- 未向agent入口传 `--timeout-ms`：Drover 60秒总超时取消进程组，入口来不及写end，查询显示有begin无end；不重放。
- 宿主可执行文件本身无法启动时没有配对回执，按unknown处理（保守，不判未执行）。
- 提交/接受/退回在有宿主路径时都会只读调用一次 `telemetry list`（无库返回未初始化，不建库）。
- `Telemetry ↗` 与 `[ ] Record` 与现有 Dispatch selected 一样只能点击；`Record default` 有键R。
- `.drover.conf` 经现有 atomic_write 改写（与Drover既有写入一样新文件权限0600）。仅派发两条路径去掉 `SADDLE_HOST_BIN`，Drover其他corral只读调用照常继承环境。
- 未改Corral、外部仓库、存储schema或01/02/03协议；无依赖反转。未删旧Dispatch页签/dlog、未导入旧日志、未切换消费者或05；无release/安装/真实数据/产品JEV。

### 需主控决定

1. 是否接受300 ms含进程启动的墙钟口径（首次执行可能降级为无记录）。
2. 是否接受 `not_executed` 作为新的delivery状态值，及交付结果 `telemetry` 字段形状。
3. 上述未修的既有workflow竞态失败是否另行处理。

## 主控审查（初核，2026-10-02）

- 中断接续已完成，公开idle后取得完整DONE；候选81ccc7eec7b45a53d78ec4be39377b30405dee79干净。已核完成记录、diff、有效目标RED/GREEN及保存桩，未改Corral或发现反向依赖；BindingFilter仅提取同一校验供通用runtime使用，接受最小范围扩展。
- 主控一次标准533通过/0失败/5忽略、Clippy通过，diff check通过；日志saddle-04b-controller-fid61fg2。开发原workflow target_changed失败保留，单项复跑通过，主控本轮未复现但原次原因未确认；开发原日志忽略数为5而非完成记录的4。process RED夹具等待失败不算有效RED。
- 接受300ms含启动的准备预算、配对executed=false新增not_executed、独立telemetry状态、60秒组取消后未知不重发及预填整格替换显示取舍，详见docs/任务/遥测04B-独立交叉审查.md。开发debug首次启动耗时不外推为release事实。
- 下一步另开重档Codex detached独立审查04B与整阶段04；本轮初核不是最终批准，不合并清理。04A/04B实施会话worktree保留，不推进05。

## 主控最终审查（2026-10-02）

- 81ccc7e通过主控核对及重档Codex独立集成审查：必须改0、建议改1；七项取舍逐项认可，详见遥测04B-独立交叉审查.md末尾。04A0261d51与04B共同批准合并，功能代码未因审查改变。
- S1保留：无MAIN_AGENT且记录准备失败时，遥测提示误写sent the plain way；业务not_sent/manual_text与交付首行正确，不阻断、不自动扩修。本次主控仅合并文档两边追加记录并纠正忽略数。
- 主控候选一次标准533通过/0失败/5忽略及Clippy通过；独立仅静态/日志核对，不重复测试。原开发workflow失败原因仍未独立查证，不能把完成记录中的归因当确证。实际清理与推送见HANDOFF及dlog收尾记录。
