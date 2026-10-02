# Tasks 运行概览与页签整理

用户批准按所展示方案直接实施。范围：页签和独立遥测入口；当前run关键节点只读摘要；人工推进状态、完整报告与历史退回原因展示顺序。主控自己做、不委派、不遥测。主题预置仍待所有UI调整完成后再做。

只使用假宿主和合成数据检查只读投影、未知结果/缺记录、当前与旧run隔离，以及直接相关UI导航/宽窄布局。必要插件构建，不跑全仓测试或Clippy；不改Corral、任务状态机、采集或真实队列。

## 完成与验证

- 右侧三项改为单行可换行页签，选中项为强调色/粗体/下划线；遥测跳转单独靠右，窄屏换行。键盘Tab/Shift-Tab、原t/Enter、鼠标与正文阅读位置保持。
- 首屏显示本次各类最新事件与序号/时间/条数、最新主控审查/收尾声明、人工提交/验收及下一步；下方保留全部报告正文、身份/链接、运行记录与旧轮退回原因、仓库参考。并非聚合整阶段成功，未解析自然语言正文做结论。
- 查询仍通过宿主公开遥测按精确run绑定与固定事件上限分页，不增加外部调用；只保留额外4类事件的摘要。不改采集、存储、宿主、Corral、状态机或通知，未操作真实任务。
- 新摘要展示与查询丢弃执行事件两项目标有有效RED，日志/tmp/saddle-overview-red-display.log、/tmp/saddle-overview-red-query-valid.log；早先扩展查询夹具的seq超出其固定上限，已纠正，早先日志不作有效查询RED。完整初始差异/tmp/saddle-overview-red.patch。
- 53个直接相关目标通过：插件lib 5、链接6、队列导航20、UI21，加首屏/窄屏全文/页签点击目标1。命令结果边界含超时、无end、排队、回复缺正文/关联未证实、路由缺建议、非零退出；本轮不继承旧轮提交/验收，旧原因全文保留。日志/tmp/saddle-overview-related.log、/tmp/saddle-overview-firstscreen.log。
- 首次UI运行18过3败，均为原英文/圆点/按钮文案与CJK续格样式断言；按批准新页签更新预期与宽字符定位，不删除交互或状态语义检查；最终上述相关目标全过。没有跑全仓测试或Clippy。
- 真实终端视觉待用户复看；自动验证使用合成数据、假宿主与TestBackend，不访问真实任务/遥测库。日常插件安装证据及重载要求见HANDOFF。

- 补核真实宿主/插件协议上的直接入口：插件遥测跳转/拒绝1、旧log退役视图1、宿主遥测往返1，以及7个受页签选择器影响的定向workflow，共10项通过，累计63个不同目标。未跑整套workflow。已同步其余同名页签选择器，不声称执行所有相关文件的全部用例。
- 定向workflow首次6过1败（仍点击旧Links），改为关联资料后该单项通过；旧log视图单项首次因直接拼接CJK续格导致文字多空格失败，改为读取协议文本span后通过。失败原日志保留，不声称一次全绿。日志/tmp/saddle-overview-{plugin-nav,host-nav,workflow-selectors,links-selector,legacy-view,legacy-view-final}.log。
- 必要插件release构建完成（/tmp/saddle-overview-build.log），候选在/tmp/saddle-run-overview-stage-path指向的目录；隔离HOME/假Corral的initialize/shutdown通过。未重建或替换日常宿主；插件manifest及真实配置保持。

## 展示语言纠正

用户指出上版未经授权改成中文。恢复该轮改动的所有产品文案为英文，保留页签布局与只读摘要；不翻译用户正文、报告或退回原因，不改判断/查询/任务流转。纯文案纠正不制造RED，只验证直接相关展示/节点文案与必要插件构建。

纠正验证：插件lib 5项、UI 22项通过（/tmp/saddle-ui-language-check.log）；首屏、窄屏、选中样式与点击保留。生产3个文件去除字符串并规范化rustfmt空白/尾逗号后与纠正前一致，无新增判断；初次未规范化尾逗号的检查失败仅为格式差异，已核diff。必要release构建通过，未重复workflow、全仓测试或Clippy。

## 顶部主状态（2026-10-02）

用户仍无法一眼看出是否需要批准，要求保留当前全部信息，只重点突出状态。Run details标题下增加高对比底色/粗体标题/直接操作提示：Running且本次最新收尾报告可读时YOUR REVIEW NEEDED，awaiting_release为AWAITING YOUR ACCEPTANCE，done为ACCEPTED，Running缺可读报告为COMPLETION UNCONFIRMED。加载/刷新失败不显示旧报告为待批准，区分CHECKING STATUS / STATUS UNAVAILABLE。所有文案英文；报告出现不代表运行成功，不根据自然语言判断成功或自动提交。其他详情全部保留，不新增查询、业务逻辑、通知或采集。

状态目标实现前有效失败（/tmp/saddle-status-banner-red.log），实现后3项detail目标和22项UI通过，含首屏位置/强调色/底色/粗体、完整报告和旧轮隔离、缺报告/不可读/查询失败/刷新过期。测试仅这些直接相关检查，不跑全仓或Clippy；必要插件release构建、diff check通过。日志/tmp/saddle-status-banner-{check,ui,build}.log。安装/备份及重载要求见最新HANDOFF。
