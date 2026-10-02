# Saddle 交接

更新：2026-10-02。当前main；阶段04已通过审查、合并并完成工作区/会话清理。未发布或安装；05已获用户批准，05A现已派发。

## 本次完成

- 04A：宿主独立Telemetry/t入口，列表/筛选→详情→完整正文，固定事件上限与当前摘要分开。候选0261d51此前已通过主控审查。
- 04B：通用telemetry.open.v1跳转和SADDLE_HOST_BIN；Drover项目默认记录关、本次覆盖、经公开CLI单次交付与业务落盘后状态声明；任务详情关联跳转；预填控制字符保值安全显示。候选81ccc7eec7b45a53d78ec4be39377b30405dee79。
- 原04B一次API连接中断后由原会话接续，不是返工；完整DONE已核。重档Codex独立审查可以合并，必须改0/建议改1/可不改7，主控逐项认可。
- 主控候选标准533通过/0失败/5忽略、Clippy通过。独立审查只读源码与既有日志，未另跑测试；本次最终核对未重复标准。合并后与候选的差异只有文档。
- 合并4ad70a3已推送；冲突只在04B任务末尾追加记录，保留双方记录并纠正开发日志忽略数。收尾空提交7eadfef；本交接随下一条文档提交推送。
- 01存储/查询、02执行采集/总开关、03通用插件扩展及可选dispatch插件此前已落main并收尾；没有改Corral或反向依赖。

## 保留问题与验证限制

- 04B S1非阻断未修：无MAIN_AGENT且准备记录失败时，反馈遥测行误写“sent the plain way”，交付首行及not_sent/manual_text正确。已告知用户，不自动开文案返工。
- 开发原标准workflow pending_delete_button_confirms_names_the_task_and_can_be_cancelled报target_changed；单项复跑通过、主控全量未复现，原次原因仍未知，不称已修。原32目标412过1败5忽略（原完成记录误写4已纠正），补22目标120过0败；不是开发原次全绿。
- process RED曾因夹具等待新帧失败，不算有效RED；其他目标有真实RED/GREEN及桩留档。独立没有实测完整60秒超时/背压/SDK20秒超时，不扩大证据。
- 300ms身份准备含启动，失败只能在业务尝试前无上下文降级；60秒取消后可能缺end，未知不重发。开发debug首次启动耗时未由主控复测，不能推论release表现。
- 03B S1失败时skills保留原因提示可能遗漏，仍非阻断未修；目录身份核对与删除非原子。03原历史测试失败/补测与未知原因均留在03审查文件，不因04全绿抹去。

## 清理与现存会话

- 已无force删除telemetry-query-ui、telemetry-drover-integration、review-telemetry-drover-integration三个worktree；两个实施分支均git branch -d删除。
- 每个自建agent核idle/attached0且目录删除后才关闭：saddle/dev-telemetry-query-ui-1（2be45b11bd4d）、saddle/dev-telemetry-drover-1（aca3e300a88d）、saddle/dev-telemetry-drover-review-1（e3860ca07333）。迟到提醒查not_found后忽略，不恢复。
- 公开corral ls仅余用户corral/main、dispatchlog/main及主控saddle/main；用户会话未操作。03六个旧实施审查agent/目录及04设计agent此前均已清理，勿恢复。
- 保留../saddle-worktrees/review-telemetry-design（5ddd544）、t38-dispatch-study（c15bc4d）、t55-notification-flow（3cc417d）。旧设计Claude已关闭。

## 下一步与硬边界

- 04已收尾。用户明确“好的，那就开始吧”，05获准启动；先05A仓库内替换，后备份/安装/技能与指引切换，再核消费者退役旧工具。旧数据不导入不删除，不恢复旧Drover独立服务。
- 遥测归宿主SQLite，dispatch为可选路由插件，Drover经通用公开接口关联；无需dispatch/Drover也可用独立遥测。具体设计与理由以docs/DESIGN.md及契约为准，不重新讨论已批准布局/记录选择。
- **需改Corral或发现依赖反转，先停相关部分告知用户，不能先改后报。** Corral迁移仍最后。Saddle不管理项目是否采用主控分派，不自动改项目AGENTS。
- 主控不写功能代码；按AGENTS/corral-dispatch和../dispatch-log/USAGE.md记录路由/派发/审查。合并、release、任务登记、队列放行/派发是不同授权。
- 未release/build --release、安装、写真实技能/链接、遥测数据/任务队列、调用产品JEV或切消费者。~/.local/bin/saddle仍指向共享.target/release/saddle的旧日常版本；release构建前遵循备份规则，源码已合并不等于日常版本已升级。

## 优先阅读与证据

- docs/DESIGN.md、docs/任务遥测接口契约.md、docs/任务遥测实施计划.md、docs/遥测查询与Drover接入设计.md、docs/遥测使用.md。
- docs/任务/遥测04A-主控审查.md；遥测04A-查询界面.md与遥测04B-Drover接入.md完成/最终记录；遥测04B-独立交叉审查.md末尾最终裁定。阶段03历史意见留03A/B/C各独立交叉审查文档。
- 主控标准日志：/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-04b-controller-fid61fg2（test.log、clippy.log、results.json）。04A旧标准saddle-04a-controller-32zq_vbj，定向saddle-04a-r1-controller-ub8k8xlk，同一父目录。
- 04B开发证据：/private/tmp/claude-501/-Users-firegnu-Developer-personal-projs-saddle-worktrees-telemetry-drover-integration/8944a1f9-783a-4263-bfe7-f2942911793c/scratchpad。
- 本次清理原始记录：/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-04-close-hmxm_ybf/actions.json。
- dlog：04A f4e92b504dec41f59ce2062a5e83f571；04B 936889ea33ad4146bd967c25da4ae6d9；04B独立审查0a1223662d11441ba1b703e6dc3f71e5。裁定已记录，实际收尾在推送完成后另记。

## 当前派发：05A仓库消费者替换

- saddle/dev-telemetry-consumers-1，instance0e03c0c56e19，Codex gpt-6-astra/high、role=implementer；分支telemetry-consumer-cutover，worktree ../saddle-worktrees/telemetry-consumer-cutover，基线b6bfb04。任务docs/任务/遥测05A-仓库消费者替换.md，dlog13dc01e67825482d86390ad956ef6c3f。
- JEV三项null，主控常规/交叉审查不要/改行为：删旧只读消费者，按已有契约组合技能记录流程，不改模型或核心采集。先核目标RED/GREEN与diff/取舍，主控一次标准test/clippy；不自动扩成独立审计，发现具体要害才说明原因调整。主控不写功能代码。
- 目标：移除Drover旧Dispatch日志视图和dlog执行链，保留Dispatch selected动作与Telemetry跳转；旧--dispatch-log值接受但弃用不执行以兼容清单；随dispatch分发同版本遥测操作指引并更新revision。没有新增UI布局，无需再确认布局。
- 本轮不改AGENTS/全局skill/真实安装，仍由现用dlog记录本阶段。两个全局链接现指向Corral/corral-dispatch-skill，后续只迁移链接本身不写Corral源码。顶层项目指引盘点另外命中已归档Drover和dispatch-log本仓库，不能据此声称所有外部会话停止旧采集；不自动改用户项目或关用户agent。
- 回来先status，working重挂；idle经dlog reply取完整DONE，核完成记录与一次标准。返工交原实现者，仅定向直接回归，每轮挂提醒。需改Corral或依赖反转先停告知。
- 05A不release/build --release/package安装/真实遥测或队列/产品JEV/外部归档，不动当前日常版本；审查后主控按已批05顺序另做备份切换与退役，不把05A源码完成说成05全部完成。04B S1/03B S1不顺手扩修，旧设计review/t38/t55保留。

## 最新：05A已合并，05B准备中

- 05A3c1a4ae已通过主控审查，无返工/额外交叉审查；主控标准528通过0失败5忽略/Clippy通过，日志saddle-05a-controller-_m4xy86k。合并5e4205d已推送。主控审查docs/任务/遥测05A-主控审查.md；开发原RED/GREEN及指南探针均已核，首轮探针夹具失败不算绿。
- 原实现者0e03c0c56e19 idle但attached1，日常Saddle正显示此agent；已问用户允许清理/关闭还是暂保留，答复前不清目录或关闭。分支/worktree尚存，未空提交收尾，不能声称05A生命周期收尾完成。
- 05B已完成备份（~/.local/share/saddle/backups/telemetry-05-20261002-103248）；正构建显式host target路径的release产物，不覆盖当前默认release/saddle。日志saddle-05b-stage-hrgnwvmo，实际切换记录docs/任务/遥测05B-实际切换记录.md。二进制/插件/技能/AGENTS都尚未实际切换，外部仓库未归档。
- 接续先核构建与用户清理答复，再按切换记录顺序核产物/协调一次重启/资源管理页安装与owned_current核对/主控指引/旧消费者退役。不用再派功能agent、不重跑标准；需要改Corral或依赖反转先停告知。
