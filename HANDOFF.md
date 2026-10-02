# Saddle 交接

更新：2026-10-02。main；阶段01—05已完成，日常安装已切换。用户已收到“可以重新打开Saddle”通知。不要自行开始Corral迁移。当前待办见下面最新条目。


## 最新：用户批准单次任务遥测边界，进入接口设计

- 用户明确要求：本次任务从委派到完成收尾均记录，随后自动停止；下一项任务未再次要求则不记录，再要求新trace。规则已写docs/DESIGN.md末尾，替代下面“尚无新增授权”的旧状态。
- 本轮先核定结束接口/写入边界/旧库兼容及Tasks人工验收衔接，任务docs/任务/遥测单任务边界-设计.md，仅设计无实现。路由一次重/交叉null/看得见，采用Claude opus[1m]/xhigh，主控静态审查，不测试构建扩审计。产品边界已批准，不重复问；实质新取舍另报。
- 本项工作本身未要求记录，不复用Theme trace、不操作真实遥测/队列。当前功能尚未改变；不得把closure说明称为已封闭。
- 已派saddle/dev-telemetry-boundary-design-1，instance7f019dd5dae6，分支/worktree telemetry-task-boundary-design，基线24c04bd。结束先status再公开corral reply确认DONE；working重挂提醒。交付docs/遥测单任务边界设计.md及完成记录，主控核后再形成实施任务；现有保留目录及用户agent不动。

## 最新：Lagoon墨绿主题已合并安装，待用户选择

- 用户参考图精确底色#0c1616，新增第四预置Lagoon/lagoon，顺序Dune/Tide/Lagoon/Terminal；主背景和Agents背景同参考色，原三主题及覆盖/渲染机制保持。功能0b77408，合并7c55a97，审查0c77ea5，清理空提交4a869d2。任务docs/任务/Lagoon墨绿主题.md含完整配色及核对。
- 开发记录直接主题目标2+5+1通过；公开read尾部只能核一项完整test result，主控未冒称核过所有原日志，未重跑套件/全套Clippy。正文/底色14.24、正文/选中9.76、按钮文字/焦点10.75主控独立计算一致；真实终端观感待用户。原始公开输出/tmp/saddle-lagoon-public-read.txt。
- theme-lagoon工作区/分支已删，idle/attached0且目录删除成功后关闭自建saddle/dev-theme-lagoon-1 instanceb6bb523c6e90。用户agent、设计review/t38/t55保留；迟到not_found提醒忽略。
- release构建及日常入口哈希/--help通过，SHA256 0fd905e605610abfb3b3a70c511b6dbcee79d372e812d4c4b53888534ad94b8a；备份/Users/firegnu/.local/share/saddle/backups/theme-lagoon-20261002-220045，安装记录/tmp/saddle-lagoon-install.json，日志/tmp/saddle-lagoon-build.log。真实配置前后哈希一致，未替用户切换主题、未动插件包/队列/Corral或停止当前进程。**重新打开Saddle → Settings → Colors → Theme → Lagoon → Save**。
- trace e72bb30e-3fc2-4f69-b910-36c20ee77636，dispatch6ebd0a70-a081-4c93-8670-d4d4102b7873；review83473441-4ed7-4915-b1cc-9d764ca02f66已stored，原telemetry目录lagoon-closure.json/receipt记录实际收尾。
- 遥测边界讨论尚无新增实现授权：独立新任务未显式要求则不记录；原任务补充沿原trace，新独立记录任务新trace。目前closure不是硬封存、任务边界由主控判断。本轮Theme后续沿原trace已告知用户，不自行扩展完成状态功能。


## 最新：右侧终端默认配色已合并安装，待用户重开

- 用户批准Tide右侧默认背景/文字跟随Theme。功能ec84452，合并80876ce，清理空提交62190b7；任务docs/任务/Theme终端默认配色.md含完成与主控审查。两处渲染传有效Theme，Tide bg/text为#151b23/#dce4ee；Dune/Terminal及显式索引/RGB/动态调色板颜色保留。无PTY/ColorRequest/解析器/Corral/业务改动。
- 直接terminal六项通过，公开原输出已核/tmp/saddle-theme-terminal-public-read.txt；主控仅静态/diff check，无套件重复或全套Clippy。游标及dim/bright默认名维持旧行为，未做终端协议同步；真实界面效果待用户。
- theme-terminal-colors工作区/分支已删除，idle/attached0且目录删除成功后关闭自建saddle/dev-theme-terminal-1 instance1fbad82a5aa3；用户agent及设计review/t38/t55保留。迟到提醒not_found忽略。
- 日常宿主已构建安装，源62190b7，SHA256 5cf134ea847740bad8a7a7c7717160f011713e83f694aa450b9decb10e55d32b，--help/哈希通过；备份/Users/firegnu/.local/share/saddle/backups/theme-terminal-20261002-215135，记录/tmp/saddle-theme-terminal-install.json，日志/tmp/saddle-theme-terminal-build.log。真实配置哈希不变，插件包/队列未改，未停止运行中Saddle。**重新打开Saddle后生效**，当前已选Tide且无bg/text覆盖即会显示冷色默认区域。
- trace同下，新dispatch c545a60c-2707-40cc-b2f0-1f0a04c3beee，review 00048168-ce21-412b-bcf4-1b802ff1c0c0已stored；原telemetry目录viewer-closure.json/receipt记录实际收尾。用户另问口头遥测结束标志，已说明closure为主控声明、目前无正式trace已完成状态；未授权扩展该功能，不自行实现。


## 最新：Theme已合并收尾并安装，待用户重开

- Dune/Tide/Terminal及Colors顶部Theme、整套草稿切换、单项覆盖/Default、Save/Cancel、旧配置观感兼容已实施审查通过。功能63e8b3c，合并d1d5855；文档0a991c3，清理空提交fc6f97e。具体取舍见docs/UI主题设计.md和docs/任务/UI主题-主控核对.md。
- 主控原一次标准542过12败5忽略exit101、Clippy通过；返工M1最后颜色键独立注释保留修复，原12项旧UI测试适配后定向补齐。本轮settings20+plugin_resources3+workflow9=32通过，日志/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-theme-r1-controller-eo87ijwc。未重复全套/Clippy，不称原全套绿。开发原始RED日志缺失和此前超预算重复标准的事实保留，不补造。
- theme-presets工作区及分支已删除；确认idle/attached0且目录删除成功后，saddle/dev-theme-1 instancee641723be7ef已关闭。设计agent/worktree此前已清，不恢复。现有用户agent corral/main、saddle-e2e-20261002-113520/main、saddle/main保留；review-telemetry-design、t38、t55工作区保留。
- trace e72bb30e-3fc2-4f69-b910-36c20ee77636；采集目录/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-theme-telemetry-qtk_rdga。返工review ad231119-2857-415d-9281-32581f45e8e2已stored；closure.json/receipt记录实际推送及收尾结果。迟到实施提醒查not_found可忽略，不重派。
- 用户随后回复“继续啊”，已完成日常release构建安装（源0711dc4）。构建exit0、日常入口哈希与--help核验通过，SHA256 1b2ec6aa845d83a7f31196ed0403ed3c8b6941448f721dd26447caf2ff315c78；备份/Users/firegnu/.local/share/saddle/backups/theme-20261002-213950，安装记录/tmp/saddle-theme-install.json、构建日志/tmp/saddle-theme-build.log。真实配置前后哈希一致，未改插件包/队列/Corral，未停止当前Saddle或任何agent。
- **用户重新打开Saddle后生效**：Settings → Colors → Theme选择Dune/Tide/Terminal；选不同主题会替换整套草稿覆盖，再可单项自定义，Save应用、Cancel撤回。已有配色保持，真实终端体验待用户。安装遥测见原采集目录installation-note.json/receipt。不自动开始其他功能或Corral迁移。


## 最新：Settings → Plugins 整理已安装

- 本轮主控直接纯UI调整，不委派不遥测，保持英文。宽屏插件列表与详情左右分栏，窄屏上下分区；列表状态对齐、选中高亮，详情按资源/接入说明/技术路径分组且可滚动，底部操作固定。Add local路径字段加框。原按钮顺序/可用条件/插件生命周期和资源操作逻辑不改。
- Dispatch仅setup_note和Template标签改英文；技能与模板正文、资源revision、路由/遥测/注册业务不改。功能da68f28、合并9a0f81c已推送，记录docs/任务/Settings插件管理页整理.md。
- 两项直接UI检查通过，覆盖极小尺寸、48×24/80×24/140×44、完整说明滚动、内外部详情及Back焦点；必要release与diff check通过，未全仓/Clippy。日志/tmp/saddle-plugin-settings-{verified,build}.log，合成终端预览已检查；真实桌面复看待用户。
- 日常宿主已备份并原子更新，SHA256 26dfc9c5568c631646c1486802738a863f538f8f818ce8442bba9ec6146d90d5；备份~/.local/share/saddle/backups/plugin-settings-20261002-183820，记录/tmp/saddle-plugin-settings-install.json，哈希/--help通过。Drover包/配置/数据未改，未关闭运行中进程。**重新打开Saddle生效。**
- 工作区/分支已清理，空提交f75c5b1已收尾；旧设计review/t38/t55保留。等用户UI复看，不启动主题任务或扩大范围。


## 最新：Drover其余子页面整理、Telemetry英文已恢复

- 用户将本轮只读检查改为“界面可以改，不能改任何逻辑”，随后再次指出Telemetry未经批准汉化。主控直接实现，不委派不遥测。新增/编辑任务、三种流转确认、删除、路径、Help、结果、All pending居中限宽，移除无关队列工具栏；确认身份长行折行，Help/结果/确认/Links加滚动提示；目录/接收者选择器加位置和完整选中值。主列表/Run details和业务状态/动作未改。
- Telemetry界面中文恢复英文，用户正文/任务书/回复/退回原因不翻译；公开query返回的六类宿主中文说明仅UI显示转换，接口/存储/采集/分页不变。功能e79e553、合并1243e06已推送，完成记录docs/任务/Drover子页面检查与Telemetry语言纠正.md。
- Drover UI23/lib7/Links1及Telemetry17个不同目标最终通过，必要宿主/插件release及diff check通过。过程保留旧边框/长句换行断言失败，调整后目标通过，不称一次全套绿。未全仓/Clippy；日志/tmp/saddle-secondary-*.log与/tmp/saddle-telemetry-english-*.log，真实桌面复看待用户。
- 已备份并原子安装宿主dc0e36e8cb76637031d9645cc9834928ddcadd799bb5800d8b1ed60e8e9f6cb7、Drover fdd3224363fd31aaddd1f25d2e587f91bd84aaad43b259f87f7b1eb6347942e0；backup ~/.local/share/saddle/backups/drover-secondary-telemetry-english-20261002-181604，记录/tmp/saddle-secondary-ui-install.json；manifest/配置/数据未改，未关闭运行中进程。**用户需重新打开Saddle加载Telemetry英文；Drover如保留旧进程，停用再启用。**
- 本轮worktree/分支清理、空提交收尾已完成；不自动开始主题或Corral迁移，旧设计review/t38/t55保留。之后继续纯UI窄范围、英文展示，等待用户复看。

## 最新：Projects / 项目表单 / 通知页面已整理安装

- 用户截图指出少量内容铺满全屏、操作与字段分离。主控直接纯UI调整，保持英文，不委派不遥测：Projects居中限宽并移除无关队列工具栏，名称/状态接收者分列，完整路径保留；添加/设置/选择器紧凑居中，字段边框、只读标记、就近操作；通知明确单选标记与底色，Save/Cancel在底部。宿主外框与业务逻辑未改。
- 功能dfef084、合并c5360bd已推送，相关26个不同目标通过（project_setup2/UI22/通知保存失败重试1/表单布局1），48×24、80×24、160×50布局检查；必要插件release/diff check通过，未全仓/Clippy。首次Projects测试扩展引用私有字段编译失败，修正后通过，不计功能RED。日志/tmp/saddle-project-dialogs-*.log，完成记录docs/任务/Tasks项目与通知页面整理.md。
- 日常Drover二进制已原子替换，SHA256 352fcf3703c3a9ccdad7eebcb4e71f12441c868a67ee8acc87578c80e26e4e2c；完整包备份~/.local/share/saddle/backups/tasks-project-dialogs-20261002-180730，记录/tmp/saddle-project-dialogs-install.json。manifest/宿主/配置/任务数据未改，未关闭运行中进程；用户需Settings → Plugins停用再启用Drover，无需重启Saddle。
- 本轮工作区和分支已清理、空提交收尾完成；等用户复看，不扩大UI范围。主题功能仍待全部UI调整结束，旧设计review/t38/t55保留。

## 最新：Run details顶部主状态已安装

- 用户要求一眼看出任务是否需要人工处理，现有展示不丢。英文Run details标题下新增底色/粗体主状态及动作提示：YOUR REVIEW NEEDED表示本次最新收尾报告可读需审阅，AWAITING YOUR ACCEPTANCE表示已提交待验收，ACCEPTED表示已验收，COMPLETION UNCONFIRMED表示无可读收尾报告；加载/刷新失败不拿旧报告显示待批准。没有把报告出现当作成功，也不改变人工提交/验收。
- 生产仅加plugins/drover/src/detail.rs展示，原节点/报告/旧轮退回/仓库参考保留；不新增查询、不改业务状态/通知/采集，不委派不遥测。功能be1ba02、合并9a6ff56已推送。目标有效RED后GREEN，detail3+UI22共25项通过、必要插件构建/diff check通过；未全仓/Clippy/workflow，日志/tmp/saddle-status-banner-*.log。
- 日常Drover二进制已原子更新，SHA256 35fa66190ba0cd83e85fd21a816ed46e8f53e7c71d06a1f525728799e27ff01b；完整包备份~/.local/share/saddle/backups/tasks-status-banner-20261002-175908，安装记录/tmp/saddle-status-banner-install.json。manifest/宿主/配置/真实任务数据未改，未关闭进程；用户需停用再启用Drover生效。
- 本轮worktree/分支已清理，空提交d4c8ed1。等用户复看；保持英文、窄验证范围。主题仍是全部UI调整完成后的待办，旧设计review/t38/t55保留。

## 最新：已纠正Tasks展示语言，保持英文

- 用户指出只要求UI调整，未授权改展示语言。上一轮把页签/节点提示改成中文属实施越界；已恢复英文Task text / Run details / Links / Telemetry及所有新增摘要文案，保留批准的布局和只读节点功能，不翻译用户正文/报告/原因。以后UI调整沿用现有语言，不能从中文讨论推定语言切换授权。
- 功能2920cde、合并7b96b49已推送。27项相关检查通过（lib5/UI22）、必要插件release及diff check通过，未重复workflow/全仓/Clippy。生产差异仅字符串和rustfmt格式，判断与查询不变。日志/tmp/saddle-ui-language-{check,build}.log；详见Tasks运行概览与页签整理任务末尾。
- 日常Drover二进制已备份并原子更新，SHA256 54cce20d6778e257c0af7a9fe954f873d40a85aa893c3846940e6ebb1f90b510；备份~/.local/share/saddle/backups/tasks-ui-language-20261002-175525，记录/tmp/saddle-ui-language-install.json。manifest/宿主/用户配置/真实数据未改，未关闭任何进程；用户停用再启用Drover生效，无需重启Saddle。
- 本轮worktree/分支已清理，空提交be0cb82。等待用户复看，不新增UI范围；主题功能仍待所有UI调整完成。旧设计review/t38/t55保留。

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
