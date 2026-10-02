# Saddle 交接

更新：2026-10-02。main；阶段01—05已完成，日常安装已切换。用户已收到“可以重新打开Saddle”通知。不要自行开始Corral迁移。当前待办见下面最新条目。

## 最新：人工验收问题已汇总，等待用户统一放行修复

- 用户最新明确确认“测试全部通过了，就剩下这两个问题了”：本轮人工验收结束，只保留下列两项待修，不再要求继续可选测试或自动扩展验证。此为最终人工验收结论，以下分项/队列状态保留历史证据，不表示已获修复授权。
- 问题 1：Run details 未呈现已有审查/收尾证据；问题 2：退回原因已在 Drover 保存，却未进入 Telemetry 事件本身。集中记录 docs/任务/遥测人工验收问题汇总.md，问题 1 详细范围仍在 docs/任务/Run-details完成证据展示.md。只记录，未实施/派发/安装，不恢复自动 Submit。
- 用户确认 A—D、G 及 E 的显式记录临时委派通过；测试主控最新交接确认 F 两轮新 run/trace/会话分离并重新执行，均输出 5、1、0，但 Telemetry 退回原因缺失作为问题 2 保留，不标整项无缺陷通过。E 不记录支线、额外 JEV 缺 key 支线未确认。
- 已读取测试仓库外 records/问题记录.md 与 records/验收收尾.md，本次未另查真实队列。交接中最近状态 T1–T4 done，T5 新 run 134d43093b6cf5a04f3a35f8dbd069ad 仍 running；用户口头认可不等于平台 Done，Submit/Accept 由用户操作。
- 测试主控报告各轮自建实现者均关闭；测试仓库干净、main=acf4379，测试主控及其他既有 agent、全部数据保留。本主控未关闭会话或改变开关/队列。下文安装时“人工端到端测试尚未开始”为历史状态，以本节为准。

## 最新：Tasks 项目接入界面已安装，可重新启用 Drover

- 用户批准主控直接实施，不委派。功能 a860e09，合并 8a317e6 已推送。新增项目接入状态、Add project（目录浏览/短名/可选已有主控）、旧配置复用、项目 Settings 改绑/清空主控。业务和 UI 都归 Drover 插件；宿主生产代码、Corral、AGENTS.md 未改。
- 两项旧测试已校正：app 的 Settings 仍找顶部旧行号；workflow 的未登记旧配置现在须明确 Reuse & add。初次标准失败保留，补齐其余目标并定向修正后不同用例531过、5忽略；Clippy、diff check通过。详情 docs/任务/Tasks项目接入界面.md，日志 /tmp/saddle-project-setup-*.log。
- release已构建并用隔离 HOME/fake Corral完成 initialize/shutdown；候选 /tmp/saddle-project-setup-package-3755yq76，路径也存 /tmp/saddle-project-setup-package-path。bin SHA256 896077aefbc151e01db6c681cc6be01d1e435b16e0d5dce890996d621dbe3c05；清单沿用日常版，无能力变更。
- **已完成日常插件更新**。用户确认停用后，公开状态 enabled=false 且无运行中 saddle-drover；完整旧包备份 /Users/firegnu/.local/share/saddle/backups/tasks-project-onboarding-20261002-125905。原子替换 plugins/drover/dist/drover-plugin/bin/saddle-drover，哈希与候选一致，清单逐字保留。替换后仍 disabled，已通知用户在 Settings → Plugins 点 Enable；无需更新/重启宿主或停止 agent。新运行时界面和真实业务仍待用户人工验收。
- 本次工作区/分支已清理，无自建 agent；空提交已经完成。设计review、t38/t55保留。
- 人工端到端测试尚未开始：测试 repo /Users/firegnu/Developer/personal_projs/saddle-e2e-20261002-113520 已建；用户已建对应 main，instance feb85d825521，报告已读规则且 key 非空。未替用户登记/添加/派发任务。docs/遥测全链路人工验收.md 的 A2 已改成 Add project 界面步骤，插件已更新，重新 Enable 后按 A2 测试。
- 最新人工计划已按安装结果重写：从 A0 Enable 开始，跳过已完成 A1；A2/A2.1 验接入与设置取消/清空/恢复，G 验停用插件后宿主遥测和 agent 独立、重新启用保留数据。所有新增验收仍标未执行。

## 最新：顶部入口同行对齐已安装

- 用户截图指出首版额外占两行、Telemetry过低；确认修订为Agents同行右侧Telemetry，Attention同行右侧Plugins / Settings，两行右边缘一致，宽度不够才换行。不是全部左对齐。主控应用户要求直接修正，没有分派agent、路由或开启采集。
- 修正dde5504，合并de54a11已推送；现有UI目标27项通过，含两侧同行/右对齐及窄栏临界，diff --check通过，未重跑全仓标准/Clippy。详情docs/任务/界面入口两行布局.md。
- 首次合并遇短暂index.lock失败，锁随后自行消失，未删除锁；确认main未改变后合并成功。失败后那次构建仍旧代码，不作新产物证据；真正合并后release重新构建成功。
- 日常宿主已原子替换，SHA256 c361a08d3ce49c0f7e6c3fa7d0bf45ccded143e0d1cbf88bc1ead1c670de79f7，--help退出0。备份~/.local/share/saddle/backups/header-align-20261002-111624；未改Drover包或用户配置，未关闭运行中Saddle。用户重开界面即可看到修正。
- header-actions-align工作区/分支已清理，空提交ca25adf；本交接随后推送。既有保留worktree和用户agent不动。

## 本次完成

- 05A候选3c1a4ae通过常规主控审查，合并5e4205d；主控标准528过0败5忽略/Clippy通过。用户允许后删除实施worktree/分支，再关闭实现者0e03c0c56e19，收尾2e0ea6e；迟到提醒not_found忽略。
- 05B实际成套安装新宿主和Drover包。两处旧Corral技能链接备份移走，公开资源管理页安装revision2，两目标owned_current，dispatch已启用。本项目AGENTS切新指引，CLAUDE软链接保留，其他项目未改。
- 遥测总开关仍enabled=false、initialized=false，未写真实遥测数据/队列，未做产品JEV请求。未发布GitHub release。用户界面重开和真实业务验收不在已完成证据中。
- dispatch-log退役文档0a3f53b已推送，GitHub isArchived=true已读回。本地旧程序和全部旧数据保留，不迁移不删除。用户dispatchlog/main空闲会话保留，未送话/关闭；旧上下文不自动撤回。
- 部署记录提交4e6e5b8已推送，阶段05B收尾空提交ff847a5；本交接与空提交随后一起推送，以最终命令结果为准。

## 当前会话和工作区

- 仅用户corral/main、dispatchlog/main及主控saddle/main保留；03/04/05A实施审查会话已清理，不恢复。
- 保留review-telemetry-design（5ddd544 detached）、t38-dispatch-study（c15bc4d）、t55-notification-flow（3cc417d）。旧设计Claude已关闭。
- 本次只有文档/部署配置变动，没有主控功能代码；收尾目标工作区干净。后续接手先核git status。

## 保留限制

- 03B S1安装失败时skills目录保留原因提示可能遗漏；目录身份核对和rmdir非原子。
- 04B S1无MAIN_AGENT且准备失败时一行遥测反馈误写sent the plain way；交付首行和not_sent/manual_text正确。均为既有非阻断建议，不自动开修复。
- 历史标准套件失败/局部补测及未知根因保留在各阶段审查记录，不因后续绿测改写。05A全量是528/0/5，不追加真实平台验证。
- 技能更新不强制刷新旧agent上下文，不能声称所有外部旧消费者停止。已归档Drover旧仓库历史指引未改。

## 硬边界与下一步

- 等用户下一项需求。Corral迁移仍最后；需改Corral或发现反向依赖，先停相关部分告知用户。
- 遥测归宿主SQLite，dispatch为可选路由插件，Drover只提供任务和关联；总开关/每条链路选择独立。不因没有记录或记录失败重放业务。
- 后续主控按当前AGENTS和已安装corral-dispatch/遥测操作.md执行，不回退dlog/route.py，不自动启用遥测、不操作真实队列、不关闭用户agent。
- 备份保留：~/.local/share/saddle/backups/telemetry-05-20261002-103248。若需回退先协调退出界面，按05B记录恢复，不能强删用户修改。

## 优先阅读与证据

- docs/DESIGN.md、docs/任务遥测接口契约.md、docs/任务遥测实施计划.md、docs/遥测使用.md。
- docs/任务/遥测05A-主控审查.md、docs/任务/遥测05B-实际切换记录.md（实际哈希、安装/恢复步骤、归档结果）。
- 主控05A日志：/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-05a-controller-_m4xy86k。
- 05B构建/安装核对：同父目录saddle-05b-stage-hrgnwvmo，含installed-status.json、installed-telemetry.json、archive-result.json、final-check.json；原用户layout及旧链接另在installer子目录备份。
- 旧dlog05A记录13dc01e67825482d86390ad956ef6c3f已实际收尾，不再续写或补造到新库。
