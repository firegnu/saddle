# Saddle 交接

更新：2026-10-02。main；阶段01—05已完成，日常安装已切换。用户已收到“可以重新打开Saddle”通知。没有待执行的开发派发、提醒或安装步骤；不要自行开始Corral迁移。

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
