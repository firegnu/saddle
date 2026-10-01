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

## 主控审查（初核，2026-10-02）

- 中断接续已完成，公开idle后取得完整DONE；候选81ccc7eec7b45a53d78ec4be39377b30405dee79干净。已核完成记录、diff、有效目标RED/GREEN及保存桩，未改Corral或发现反向依赖；BindingFilter仅提取同一校验供通用runtime使用，接受最小范围扩展。
- 主控一次标准533通过/0失败/5忽略、Clippy通过，diff check通过；日志saddle-04b-controller-fid61fg2。开发原workflow target_changed失败保留，单项复跑通过，主控本轮未复现但原次原因未确认；开发原日志忽略数为5而非完成记录的4。process RED夹具等待失败不算有效RED。
- 接受300ms含启动的准备预算、配对executed=false新增not_executed、独立telemetry状态、60秒组取消后未知不重发及预填整格替换显示取舍，详见docs/任务/遥测04B-独立交叉审查.md。开发debug首次启动耗时不外推为release事实。
- 下一步另开重档Codex detached独立审查04B与整阶段04；本轮初核不是最终批准，不合并清理。04A/04B实施会话worktree保留，不推进05。
