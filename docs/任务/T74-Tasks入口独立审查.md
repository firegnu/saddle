# T74：Tasks 入口实施独立审查

2026-10-04，saddle/main 委派新的 Codex（重档 gpt-6-astra / xhigh，role=reviewer）。你是被委派的独立审查者，不再委派。

## 背景与固定对象

用户已批准单个固定插件入口、Agents 焦点 p、终端启动目录来源，以及停用保留/移除清除固定；同一 T74 已获准实施。原 Claude 完成 `e6edaca`，未合并/部署。主控已读实现 diff 和公开 DONE 回复；固定字段沿用 registry 锁与并发保护、宿主只传 cwd 的分工符合要求。Pin 短文案适应窄窗口、悬停不作废定位两项取舍原则认可，仍需核对没有破坏行为。

- 审查 worktree：`/Users/firegnu/Developer/personal_projs/saddle-worktrees/review-t74-tasks-entry-design`，detached `e6edaca`。
- 实现差异：`git diff ec33dec..e6edaca`；设计阶段文档见 `docs/Tasks入口设计.md`，完整目标与验证记录见 `docs/任务/T74-Tasks入口实施.md`。
- 本任务文件位于主仓库 `/Users/firegnu/Developer/personal_projs/saddle/docs/任务/T74-Tasks入口独立审查.md`，不在审查 worktree；**只允许向本文件追加审查意见**。审查 checkout 只读，不改文件、不提交、不切分支，不改实现者文件。
- 先读 AGENTS.md、docs/UI回归.md、上述实施任务书、设计及对应差异。不重新设计已批准的入口形式。

## 重点

1. 固定登记的持久化、并发修改保护和启停/移除语义；未安装/未启用、无视图/内置插件不可固定，其他正常登记写入不丢 pin，不隐式启停。
2. 鼠标与 Agents p 是否使用正确来源，Viewer 按键是否透传，已开工作区/覆盖视图和不可用插件是否保持合理焦点/页面；宿主不得依赖 Drover 业务文件。
3. Drover 异步定位与输入修订、关闭再开、新来源、Attention/通知的隔离；草稿/确认/通知偏好/busy 不被覆盖或延后静默切换。空项目和读取失败、匹配失败与确认无匹配不混淆。
4. 用户能否辨认实际显示项目，尤其定位失败/保留草稿时的来源行，在现有常用窗口中不能因长文案让关键状态不可见。主控注意到来源字符串较长、部分页面只有一行且在居中表单外绘制，请核实正常操作是否会违背明确提示实际项目的约定，不凭猜测认定缺陷。
5. 新增/修改测试是否覆盖主路径并保留原有断言；实现记录是否与代码吻合。实现者报告 workflow 整组 `native_mouse_buttons_cover_forms_and_stop_confirmation:851` 持续失败、基线 ec33dec 同处2/2失败、单例通过；另一个 closing_a_start_target 项一次偶发失败。该声明尚未由主控重跑确认，不据此宣称全套绿或自动判回归。

## 验证预算与边界

- 主体为静态核查。必要时选择直接涉及上述疑点的现有单例/目标检查或最小隔离验证；复用合成数据、临时目录、假CLI，所有Cargo加共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。
- 不跑全量测试、全量Clippy、整个workflow，也不重复实现者已通过的所有UI组。全量由主控在阶段集成集中跑一次；不要搭车诊断/修复既有时序测试，不扩展主题/设备/窗口组合矩阵。原始失败须保留，若疑似偶发只限定复核一次。
- 不读真实队列/配置，不调用真实agent或部署/重启，不访问私人Corral状态文件。使用现有日志如有；无日志时标为实现者声明，不编造RED/GREEN实测证据。临时自建材料放 `/tmp/saddle-t74-review/`，不污染审查checkout。

## 输出

向本文件追加「## 审查意见」：先给“可以合并 / 改完再合并”结论，然后每条写必须改/建议改/可以不改、文件位置、实际触发与影响、证据、最小改法。必须改仅用于用户目标/实施约定未达到或真实并发、数据损坏等缺陷；罕见组合或范围外问题列建议改，不扩任务。对两项实现取舍表态，列实际运行检查及结果，不能把局部通过写成全量通过。

命令在前台跑完。回复仅写结论和问题条数、必要未决项，最后一行 DONE。不要推送/合并或操作Tasks状态；遥测由主控处理。

## 审查意见

**改完再合并。必须改 2 条，建议改 0 条，可以不改 5 条。**

2026-10-04，独立 Codex 审查。固定对象为 `e6edacaee0208ced286227679cb2d7b83e67d72b`，差异为 `ec33dec..e6edaca`；以下源码位置均指该固定提交。已读 AGENTS.md、UI 回归、实施任务、批准设计和对应差异，不重开入口设计。以静态核查为主，补充直接相关现有单例及 `/tmp/saddle-t74-review/` 内的合成进程验证。

### 必须改

**R1：常用窗口中的草稿页无法辨认实际项目，接入表单还会覆盖整条来源提示。**

- 位置：`plugins/drover/src/plugin.rs:35-48,953-975`；`plugins/drover/src/queue.rs:1353-1374,1540-1546`；相关布局 `plugins/drover/src/project_setup.rs:309`。
- 实际触发：项目 A 的 Add task 页保持打开，经 Ctrl-] 关闭视图、选项目 B 的 agent 后再开固定入口，按约定保留 A 的草稿。在正常的 80×24 宿主窗口中，Tasks 覆盖区内尺寸为 62×17（按 `src/app_plugins.rs::overlay_area` 的 4/5 大小及外框计算）。以实际插件可执行文件发送等价的关闭、来源、打开事件，画面来源行只剩 `From …view/case-5kxtgxvk/project-beta · kept your current`，全帧完全没有 `project-alpha`。这不是极小窗口或异常长项目名；来源已被正常截到 32 列，实际项目仍被固定放在句尾挤掉。保存继续作用于 A，用户却只能看见来源 B，未达到设计 §2.5 的实际项目提示约定。
- 补充证据：同一草稿在 120×36 宿主对应的 94×26 内区可以显示 `showing project-alpha`；但项目接入表单在这个常用尺寸高 26 行，其边框从第 0 行开始绘制，覆盖预先画在该行的来源提示，重开时完全没有 `From`/保留现场状态。列表的 NoMatch、Unknown 来源行在 94 列也会截掉 `showing`，不过列表另有项目选择器，不能将这些列表画面单独误报为“完全看不到当前项目”。
- 证据材料：`/tmp/saddle-t74-review/draft-host80x24.txt`、`draft-host120x36.txt`、`setup-host120x36.txt`，及同名 `.json` 原始协议帧；`no-match-host120x36.txt`、`unknown-host120x36.txt` 保留列表对照。`probe.py::visual` 运行成功；首次等待完整 `kept your current page` 因文字真的被裁断而超时，原始输出保留于 `probe-initial-failure.txt`，随后仅将探针等待条件改为可见前缀，产品代码未改。
- 最小改法：在实际可用宽度内优先保住实际项目和未切换状态，再缩短来源与原因；草稿/接入/偏好页把提示放进不会被表单覆盖的保留行。无需重排入口或扩展窗口矩阵。补一条上述正常窗口的保留草稿断言，检查实际项目名仍可见，并检查接入表单重开提示；仅检查 `From` 或 `kept your current page` 不足。

**R2：匹配到已登记但队列读取失败的项目，仍落入接入表单，没有切到该项目的列表错误状态。**

- 位置：新增调用 `plugins/drover/src/plugin.rs:548-552`，及其既有下游 `plugin.rs:253-280`、`plugins/drover/src/project.rs:29-33`、`plugins/drover/src/core.rs:410-424`。
- 实际触发：A/B 均登记且 Git 可读，B 的 `.drover.conf` 有效，但 B 的队列不可读；当前显示 A，从 B 打开入口。合成验证将 B 的 `data/queue.md` 设为目录，稳定制造读取错误。RepoTasks 确实匹配 B，然而 `Request::Project` 先调用 `project::inspect`，后者通过 `Snapshot::load` 读取队列；读取失败使状态成为 `unavailable`，于是打开 `Add project` 并提前返回，尚未替换 `panel.project`/worker。
- 影响与证据：实际帧出现 `Add project`、`Project unavailable; existing configuration will not be overwritten.` 和 `Cannot read …/project-beta/data/queue.md: Is a directory (os error 21)`，没有 B 的 `Read failed` 列表。取消表单后仍显示 A 的任务，来源行变为 `From …/project-beta · showing project-alpha`。因此“已登记读取失败仍切换、由列表正常报错”的批准行为未实现；不是静默修复或数据损坏，但应在合并前补足该明确约定。错误文本能够展示，不能据此写成正确切换。
- 证据材料：`/tmp/saddle-t74-review/matched-unreadable-queue-host120x36.txt`、`matched-unreadable-after-cancel.txt` 及同名协议帧；`probe.py::unreadable`。首次验证已复现表单路径，后续只为确认取消后的实际项目补跑该探针，没有扩展失败矩阵。
- 最小改法：把“已登记且应打开”与“队列能否成功读取”分开；本次定位确认的已登记项目应建立其列表/worker，让读取错误进入现有 `panel.read_error`。未登记项目继续走既有接入流程；不要简单移除所有接入检查，也不要改写损坏的数据。只有确实切换/显示目标后才采用相应来源结果。补一条已登记目标读取失败仍显示目标项目及列表错误、且不打开接入表单的进程检查。

### 可以不改

1. **固定持久化及生命周期分离可以不改。** 位置：`src/plugins/registry.rs:183-208,216-313`、`src/plugins/mod.rs:352-378`。正常启停、增加其他插件、内置开关写入均传递 pin；写路径沿用文件锁、锁内 baseline 比较、临时文件持久化，移除时按剩余登记过滤 pin。无登记/内置/无视图不能由 Manager 固定；停用保留且不隐式启动。独立 registry 单例和固定入口 workflow 单例通过，包括 stale writer 拒绝、移除清除、Disabled 解释页。最小改法：无需修改。

2. **宿主入口、来源和焦点复用可以不改。** 位置：`src/app.rs:1744-1777`、`src/input.rs::Focus::route`、`src/app_plugins.rs:138-181,421-476`、`src/ui.rs:145-239`。点击取打开前焦点，Agents 的 p 使用列表选中项；Viewer 先路由到终端，不受新 p 绑定拦截；普通终端取 `source_cwd` 启动目录。宿主只传 cwd，未引入 Drover 文件读取。已有 workspace 视图复用，同一 overlay 不重复打开；不可用时预选 Plugins 行，不启停；Settings 等忙碌界面仍先消费输入。独立头部布局、固定入口、Agents/Viewer 项目来源和终端来源单例通过。后两项经既有 Plugins 路径验证公共打开函数，不能声称覆盖全部固定入口×焦点组合；workspace/Viewer p 透传及 Attention/通知不传 cwd 是静态链路核查。最小改法：无需修改。

3. **异步定位现场保护可以不改。** 位置：`plugins/drover/src/plugin.rs:538-561,633-637,706-715,891-933`、`drover.rs::RepoTasks::drop`。草稿/确认/通知偏好/setup/busy 被 `locatable` 排除；输入取消 lookup，新打开/关闭/Attention/通知清除旧 lookup，旧结果不会排队等到操作结束后再切换。4 个相关现有进程测试通过；另以延迟 Git 的合成进程验证“B 定位中关闭，改从无匹配来源重开”，等待旧查询有时间返回后仍为 A、最终显示 NoMatch，材料为 `closed-reopened-new-source.txt`。确认页/busy 和显式 Attention/通知在本轮只做静态核查，不称独立实测覆盖。R1 的提示可见性和 R2 的切换下游另列，不能由这些保护检查通过抵消。最小改法：无需改取消/保留机制。

4. **认可 Pin/Unpin 短文案，可以不改。** 位置：`src/plugins/ui.rs:146-149,321-331,720-723`。按钮名称缩短未改变可固定条件或状态操作；成功反馈与详情解释 Agents header，固定入口单例实测 Pin、Unpin 状态、停用和移除正常。保持短文案符合已认可的窄栏取舍；本轮未重跑 48 列管理页，原布局比较仍属实现者记录。最小改法：无需恢复长按钮文案。

5. **认可悬停不作废定位，可以不改。** 位置：`plugins/drover/src/plugin.rs:706-715`。唯一豁免为 `mouse` 的 `move`；按键、粘贴、按下/释放、拖动和滚动仍走取消路径。独立运行 `input_while_locating_discards_the_result_but_hovering_does_not` 通过，证明按键取消且单纯移动后仍切到目标。最小改法：无需扩大取消范围；不把头部点击后移动鼠标当作新编辑意图。

### 测试及实施记录核对

既有断言没有通过删除或放宽来掩盖本次缺陷：两处 Tab 次数随 Pin 增加一站；workflow 的空项目预期改成切换是批准行为变化。新增测试覆盖固定、空项目、worktree、来源无法判断、保留草稿/偏好和输入取消等主路径，但 `process.rs:778-801` 只查保留提示/草稿，没有断言常用窄窗中实际项目可见；`process.rs:763-774` 的 unreadable 是来源非 Git，不是匹配目标队列读取失败。这分别漏掉了 R1、R2。

实施记录关于“来源行写明实际项目”及“读取失败走列表错误”的表述需随修复核正。目前不能照录成已完成；其余本轮核查的入口及保留语义与代码一致。没有取得实现前运行日志，本轮不认证实现者 RED/GREEN 原始证据。

本轮实际运行如下；所有 Cargo 命令前均设置 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，均已等到命令退出：

| 命令（省略同一 CARGO_TARGET_DIR 前缀） | 结果 |
| --- | --- |
| `cargo test -p saddle --test plugins a_pin_outlives_other_registry_writes_and_leaves_with_its_registration -- --exact` | 1 passed |
| `cargo test -p saddle --test ui_final_host a_pinned_entry_joins_telemetry_wraps_with_it_and_gives_way_first -- --exact` | 1 passed |
| `cargo test -p saddle-drover-plugin --test process opening_ -- --nocapture` | 3 passed |
| `cargo test -p saddle-drover-plugin --test process input_while_locating_discards_the_result_but_hovering_does_not -- --exact` | 1 passed |
| `cargo test -p saddle --test workflow pinned_tasks_entry_is_set_once_opens_with_p_and_stays_while_disabled -- --exact` | 1 passed |
| `cargo test -p saddle --test workflow tasks_open_on_the_focused_agents_repository_even_without_tasks -- --exact` | 1 passed |
| `cargo test -p saddle --test workflow a_terminal_gives_its_launch_directory_as_the_tasks_source -- --exact` | 1 passed |
| `python3 /tmp/saddle-t74-review/probe.py` | 最初因提示截断等待超时；探针调整后退出 0，复现 R1/R2，并通过关闭重开取消检查。此处退出 0 表示复现断言成立，不表示实现满足验收 |
| `python3 -c 'import sys; sys.path.insert(0,"/tmp/saddle-t74-review"); import probe; probe.unreadable()'` | 退出 0；补证取消错误接入表单后仍为 A |
| `git diff --check ec33dec..e6edaca` | 通过 |

共 9 个现有自动化测试通过；合成协议帧验证只使用上述两种常用内区尺寸，没有扩大窗口/主题矩阵。探针启动的是本地插件可执行文件，所有项目、配置、队列和假 Corral/osascript 均在 `/tmp/saddle-t74-review/`，不使用真实 agent。探针子进程均已退出。UI 证据来自实际渲染协议帧，不是人工终端截图，也不声称真实设备观感验收。

没有运行全量测试、全量 Clippy、整个 workflow，未重复所有 UI 组。`native_mouse_buttons_cover_forms_and_stop_confirmation:851` 的整组失败、基线同处 2/2、单项通过及 `closing_a_start_target…` 偶发失败仍为**实现者声明，未在本轮独立复跑或确认归因**；既不据此判 T74 回归，也不称全套绿，阶段集成由主控处理。

审查 checkout 始终保持 detached 固定 HEAD 且无文件改动；仓库内仅向本审查文件追加意见。未提交、切分支、委派、合并、推送、部署、重启或操作 Tasks 状态。

## 主控核验（第一轮）

2026-10-04：已核对 `saddle/dev-t74-review-1` / `80d99546d75c` 为 idle，公开回复 DONE、结论改完再合并。主控对照批准设计 §2.4/§2.5、实施任务书、固定候选源码和审查者保存的协议渲染帧，确认 R1/R2 均未达到既定约定，交回原 Claude 定向修订；未另跑探针或重复测试。认可五项“可以不改”的结论和两项实施取舍。全量保留到修订复核通过后的阶段集成一次；既有 workflow 失败归因尚未独立确认。
