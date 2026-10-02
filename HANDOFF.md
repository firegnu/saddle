# Saddle 交接

更新：2026-10-02。main；阶段01—05已完成，日常安装已切换。用户已收到“可以重新打开Saddle”通知。不要自行开始Corral迁移。当前待办见下面最新条目。

## 最新：Tasks 页签与运行概览已安装，待用户重载 Drover

- 用户批准主控直接整理右侧页签并补只读节点摘要；不委派、不遥测。功能3f619e3、合并f565244已推送。任务说明/运行概览/关联资料用下划线标选中；打开遥测独立靠右，窄屏换行。首屏展示本次各类最新消息/路由/Agent启动和回复记录、主控审查/收尾声明、人工提交/验收与下一步；下方保留完整报告、身份、运行/历史退回原因及仓库参考。
- 仅Drover展示及其公开查询投影改变：复用既有精确run/固定upper_seq查询，不加外部请求，不解析自然语言推断成功，不改任务状态机/通知/自动推进、遥测采集/存储、宿主或Corral。多次委派与迟到记录不合成为整阶段成功；完整链路到Telemetry看。
- 63个不同相关目标最终通过（原53+直接宿主/插件入口10），必要插件release构建/隔离HOME假Corral初始化关闭/diff check通过。未全仓测试/Clippy。保留原RED、旧文字与CJK续格/旧Links选择器的中间失败；目标修正后通过，不称一次全套绿。证据docs/任务/Tasks运行概览与页签整理.md及/tmp/saddle-overview-*.log。
- **仅日常Drover二进制已原子替换**，SHA256 433b6d3f748ebcadcd1c36de9240a89f6e1bc14dec1de72c533d03102fb16079；完整包备份~/.local/share/saddle/backups/tasks-run-overview-20261002-175048，安装记录/tmp/saddle-run-overview-install.json。manifest逐字未变，宿主/技能/配置/真实数据未改。未关闭Saddle、Drover或agent；旧进程仍运行，**用户需在Settings → Plugins把Drover停用再启用，加载新界面**，无需重启宿主。
- 本轮worktree/分支已清理，空提交893dd04；旧设计review/t38/t55保留。下一步等用户界面复看；主题预置与Colors覆盖仍为全部UI调整完成后的待办，尚未实施。

## 最新：正文阅读窗口已留边，主题功能仅记待办

- 用户确认全文也使用居中大窗口；主控直接修改绘制区域，最多160列，正常尺寸左右至少2列、上下1行留边，极小尺寸收起对应留边。阅读、滚动、换行、校验和返回逻辑未改；不委派、不遥测。
- 功能e8c2db2、合并af99633；既有Telemetry界面17项通过，必要release构建及diff check通过，未跑全仓测试/Clippy。日志/tmp/saddle-reader-layout-{test,build}.log。
- 日常宿主已原子更新，SHA256 91b7a465ba651aa060f90824c6fdd25c358e29c4dd676a70463ce2bc0e898645；哈希/--help通过。备份~/.local/share/saddle/backups/telemetry-reader-20261002-173523，安装记录/tmp/saddle-reader-layout-install.json。未关闭Saddle或agent、未改插件/数据/配置；用户重开查看。
- 本轮worktree和分支已清理，空提交73d55fd；保留设计review/t38/t55。
- 用户新需求记在docs/任务/UI主题预置与颜色覆盖.md：预置几种主题，选择时下方Colors随主题变化，允许逐项覆盖。**等全部UI调整完成后再做，目前仅记录，未实现**；下一步继续等用户UI反馈，不自行开始主题或Corral迁移。

## 最新：Telemetry 查询界面已整理并安装

- 用户批准按展示建议直接改UI，不委派、不遥测、不改变业务逻辑；纯UI只做直接相关验证。功能ae171cb，合并ccd6360已推送。生产仅改src/telemetry_view.rs；采集/SQLite/命令接口/Drover/Corral未改，真实任务数据未操作。
- 列表/详情最大160×36居中，列表随条目收缩；项目(scope)/类型选择、任务或链路搜索只筛选已加载记录，原精确binding查询保留。事件可读名称、来源与选中底色；正文按钮醒目、首份正文两行预览、无正文明确提示，技术详情v展开原字段。多正文、全文阅读、固定事件上限/当前摘要分别标时、返回与控制字符安全显示保留。
- 20个不同的直接相关检查通过（展示17、模态入口/插件返回2、异步过期结果1），必要release构建及diff check通过；没有跑全仓测试或Clippy。新增筛选/正文入口有效RED后GREEN；旧文案/全屏位置/默认展开字段断言按批准UI更新，语义检查保留。证据docs/任务/Telemetry查询界面整理.md、/tmp/saddle-telemetry-ui-*.log。
- **日常宿主已安装**，SHA256 2a23c69a0f95fa3ffb7eba45f7c751a81122c4eb55a4bd99ec93aeceecd2c064；哈希与--help核验通过。备份~/.local/share/saddle/backups/telemetry-ui-20261002-172441，记录/tmp/saddle-telemetry-ui-install.json。未关闭运行中Saddle或agent，未改插件包/技能/配置；用户重开界面看效果，真实终端复看待用户。
- telemetry-ui-layout工作区/分支已清理，无自建agent，空提交收尾完成；设计review/t38/t55保留。下一步等用户UI反馈，不自行扩大到Run details或Corral迁移。

## 最新：Plugins 入口弹窗整理已安装

- 用户明确主控直接实现、不委派、不遥测；按已批准线框只调整 Plugins 弹窗，未改管理页或业务流程。搜索边界、名称/状态/操作分列、Built-in 跟随名称、暖色选中底色、尖括号按钮、一行说明和紧凑高度已落地；窄屏保留键盘操作。
- 功能 f1807f2，合并 ce62879 已推送。标准首次415过/1败/5忽略，唯一失败是workflow旧版连写 Built-in · Disabled 的预期；更新后单项通过，补余下122项及examples通过，共538个不同用例通过5忽略，不能称一次全套绿。全量Clippy及变更后workflow定向Clippy通过。详见docs/任务/Plugins弹窗布局.md、/tmp/saddle-palette-*.log。
- **宿主已安装**：候选SHA256 7bc87dfd2f558cbe5f352e8c6ae040393fc87e85b889cc68de3695208513dc58，原子替换日常宿主且哈希/--help核对通过。备份 ~/.local/share/saddle/backups/plugins-palette-20261002-165514；记录 /tmp/saddle-palette-install.json。Drover包/技能/配置未改，未关闭运行界面或agent；用户重开Saddle看效果，真实界面复看待用户。
- plugins-palette-layout工作区/分支已清理，无自建agent；空提交收尾完成。保留既有设计review/t38/t55。下一步等用户逐个界面反馈，不自行开始Run details新改版或Corral迁移。

## 最新：两项验收问题已修复合并并安装，可重开 Saddle

- 用户授权主控直接修复，不委派。功能9f8630a、候选记录b757804，合并55201bf已推送。Run details 增加本次run审查/收尾报告，经宿主公开遥测查询；通用task.transition允许可选reason正文，Drover在退回业务落盘后采集原因并由现有正文查看器展示。任务状态机/人工Submit-Accept-Return不变，不改Corral、不反向依赖、不自动补写旧事件。
- 有效RED/GREEN已核；标准414通过1失败5忽略（workflow鼠标测试resize后超时，原因未确认），该单项复跑通过，补其余工作区122通过，合计537个不同用例通过5忽略；Clippy通过。不能称一次全套绿，未扩修旧测试。日志/tmp/saddle-e2e-fixes-*.log，详见docs/任务/遥测验收问题修复.md。
- release宿主/Drover已构建，隔离启动检查通过；成套候选/tmp/saddle-e2e-fixes-stage-vm6scrnf，manifest.json保存候选和现有安装哈希，路径文件/tmp/saddle-e2e-fixes-stage-path。构建在共享target显式架构目录，未覆盖日常安装。宿主候选489e9743…，Drover候选fd71367f…，插件清单原样保留。
- **日常安装已完成**：用户确认退出后，ctl instances为空且无运行宿主/Drover；旧宿主和完整插件包备份 /Users/firegnu/.local/share/saddle/backups/telemetry-e2e-fixes-20261002-162434。两二进制原子替换，哈希与候选一致、插件清单未改；安装后宿主只读查询和隔离插件启动/关闭通过。记录/tmp/saddle-e2e-fixes-install.json。可通知用户重新打开Saddle，真实界面复看待用户；Corral agents和真实队列/数据未操作。旧事件原因不自动回填。
- 实施worktree/分支已清理，无自建agent；空提交收尾已完成，后续交接提交推送以工具结果为准。保留设计review/t38/t55。
- 本轮人工验收用户最终确认完成，原问题证据保留docs/任务/遥测人工验收问题汇总.md。测试主控收尾时T1–T4 done、T5新run仍running是历史读回，本主控未操作真实队列；不把口头验收当平台Done。用户agent/测试数据保留。下文安装时“人工端到端测试尚未开始”为旧状态，以本节为准。

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
