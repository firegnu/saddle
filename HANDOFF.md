# 会话交接

更新：2026-10-01。当前 main。核心遥测阶段01、02已审查、合并、推送并清理；尚未发布到日常安装版。用户已批准dispatch插件及必要通用框架能力，阶段03接口设计已派Claude，尚未派发功能实现。原03宿主必备JEV安排撤回。没有真实队列操作。

## 1. 会话摘要

遥测归Saddle核心，SQLite保存事件与关联、正文单独保存。阶段02完成公开 `saddle agent` 执行采集入口和Settings → General同一个Store总开关；02A/02B串行集成并共同收尾。既有TUI/ctl/Drover消费者尚未切换。

## 2. 已完成

- 阶段01：候选61014a0，合并465c4fb，收尾eb82212；存储、纯记录和headless查询。
- 阶段02：02A最终ae77967；02B初版baff5bf，经4dea76b/51d4c5f修正。整阶段合并3a7379e已推送，收尾空提交6414ea0；本交接及完成记录随后提交推送。
- 02A M1宿主回执误判、02B B1部分保存提示截断及返工旧值误报均经主控和原独立审查者关闭，最终新增阻断/建议0。
- 02B初版主控一次标准检查431 passed / 0 failed / 5 ignored，Clippy通过；返工后主控6项和独立21项直接回归通过，没有重复全量。独立审查还用合成检查核对其他设置页的消息、草稿和重试。
- 四个阶段02实施/代码审查worktree已安全移除，telemetry-agent-capture、telemetry-settings两分支已删除。对应dev-telemetry-agent-1、dev-telemetry-agent-review-1、dev-telemetry-settings-1、dev-telemetry-settings-review-1在idle/attached=0、目录删除后关闭。

## 3. 当前边界与悬项

- 无已知未关闭的阶段02阻断。02A S1发送参数换序仅历史非阻断建议；若无新的公开接口证据不扩展。
- 实现者历史workflow测试超时根因仍未知，单项/主控后续通过不能当作已修好。初始02B RED完整日志未保存，摘录只明确3项缺开关行为，第4项原因仅推断；B1两次RED/GREEN有完整日志。详细限制保存在审查文件，不重造历史。
- 未release/build --release、安装、接JEV/Drover、操作真实遥测/队列或切换消费者。已安装saddle仍是此前Clawd版，~/.local/bin/saddle指向共享.target/release/saddle；未构建release，后续构建前遵循原备份规则。
- 保留 ../saddle-worktrees/review-telemetry-design（5ddd544）、t38-dispatch-study、t55-notification-flow；设计Claude已按用户要求关闭，不恢复。
- 保留用户会话corral/main、dispatchlog/main及主控saddle/main；新开阶段03设计者见末尾。迟到的阶段02提醒查到not_found后忽略，不重建已收尾agent。
- 旧独立Drover已退役，现有plugins/drover仍保留旧Dispatch视图/dlog入口，阶段05再切换；不恢复旧服务或导入旧日志。

## 4. 约束与决定入口

设计以docs/DESIGN.md及docs/任务遥测接口契约.md为准。Corral无遥测依赖，宿主不反向依赖插件；如需改Corral或发现依赖反转，先停相关部分告知用户，不先改后报。初始总开关关闭；只采显式选中链路；原话逐字转录注明来源；晚交标记不改历史。

主控不写功能代码；按AGENTS、corral-dispatch及../dispatch-log/USAGE.md派发、审查、记录。每轮挂提醒；测试隔离HOME/状态，保留真实CARGO_HOME/RUSTUP_HOME，固定共享target。任务登记/放行/下一项分开，不自动推进真实队列。

## 5. 优先阅读与证据

- docs/任务遥测实施计划.md、docs/任务遥测接口契约.md、docs/DESIGN.md、docs/遥测使用.md。
- docs/任务/遥测02A-执行采集.md、遥测02A-独立交叉审查.md、遥测02B-设置总开关.md、遥测02B-独立交叉审查.md。
- 主控标准日志：/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-02b-controller-p4azt8gs。
- 主控最终6项：同临时目录父路径下saddle-02b-b1-r2-controller-ql6edm5n/target.log；独立21项及其他页检查：saddle-02b-b1-independent-_hkfuejz。
- 开发记录02A：ba09aea9613a48f6b6e6fb9ce22904b1；审查3906679e92ba45d2b634c3a43a75475d。02B：52441b99bb8e4d96955de374d94f65b5；审查4864f70f7a7b48868eadc3e82e6c87c1。完整意见在主仓库，不依赖已关闭会话。

## 6. 下一步

用户已同意修订方向：JEV路由归可选saddle-dispatch插件，主控做实际委派决定，Saddle遥测独立记录查询；没有插件仍可手动用Corral。已同步DESIGN、接口契约5.1、实施计划，并给旧设计/审查记录加上被替代说明。原03宿主路由方案不得继续照做。

下一步先形成dispatch插件集成与现场采集接入设计，澄清插件执行入口、固定采集凭据/开关在途语义、现场证据与声明的区分；方案未定，不直接派实现。查询界面不依赖dispatch，但本轮也未派发。阶段04保留独立查询/Drover关联入口，阶段05切消费者；Corral最后迁移。01/02成果保留，不重做，不把源码完成说成安装版或整链路已切换。


最新调查：已核实corral-dispatch包含SKILL.md、项目AGENTS模板.md、route.py、README；本机Claude/Codex软链接均指向Corral仓库资源，未修改。候选方案见docs/调研/Saddle-dispatch插件与技能资源接入方案-2026-10-01.md，包含无需TUI的可选core plugin入口、进程内通用采集接口、技能与模板版本化交付。现有command.v1主要依赖TUI且有正文大小限制，不能直接当作新路由入口。候选取舍尚待用户确认，未实现/派发；不改已安装技能、AGENTS、Corral或真实数据。下一步先确认候选结构及资源启用/安装边界，再定接口，不照原03方案开工。

最新决定：首版在插件安装或首次启用入口提供项目接入说明和AGENTS模板位置，由用户为希望默认主控分派的项目合入规则。Saddle不管理项目采用名单或开关，不自动改项目文件；全局skill安装不让所有项目默认委派，当次明确要求仍可使用。已同步DESIGN和候选方案；执行入口、资源安装及采集接口仍待设计，未派发实现或变更实际安装。

## 7. 当前在办：03插件接口设计

- 用户随后批准“新增dispatch插件，加上必要的通用插件基础能力；Corral不改”，并要求开始。skill随插件自动安装更新，无须单独操作；AGENTS接入仍仅说明/模板。详见DESIGN末尾。
- 任务：docs/任务/遥测03-插件接口设计.md；设计者saddle/dev-dispatch-plugin-design-1，instance 7cc3761a37cc，Claude opus[1m]/xhigh，role=implementer（本轮交付设计文档）。
- 分支dispatch-plugin-design，worktree ../saddle-worktrees/dispatch-plugin-design，基线8f66df0；只改docs/dispatch插件接口设计.md及任务完成记录，不合并推送、不写功能。
- dlog dispatch cba1e2fc7541408786a5ad4d8dd8a7f6。JEV重/交叉审查null/看得见；主控采纳重档，纯设计文档不另开代码交叉审查，后续行为实现重新定影响面。路由调用仅此次主控任务评估，不是产品JEV接入验证。
- 结束后先corral status，再经 ../dispatch-log/dlog reply --dispatch cba1e2fc7541408786a5ad4d8dd8a7f6 -- corral reply saddle/dev-dispatch-plugin-design-1 取完整回复并确认DONE。主控核具体接口、依赖方向、skill资源生命周期及首版项目说明，git diff --check/静态核对即可，设计不跑标准测试。working则重挂提醒。
- 用户批准的大方向内常规细节由主控核实；新增实质架构取舍列给用户。需改Corral或出现反向依赖先停相关部分告知。设计通过后再形成串行实施任务，不把设计交付当插件或整条遥测已实现。真实安装/消费者切换、04/05及原保留worktree不动。
