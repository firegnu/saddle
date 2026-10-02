# Drover 子页面检查与 Telemetry 语言纠正

用户授权：检查全部Drover界面，后明确界面可改，禁止改任何业务逻辑；继续主控直接做、不委派不遥测、保持英文。随后要求纠正Telemetry未经批准的汉化。

## 检查范围与处理

已读Drover全部Page绘制分支（List/Text/Details/Links、Projects、Project path、Add/Edit/Delete、Confirm三种操作、All pending、Help、Feedback），Setup三个模式和Notifications绘制。主列表/Run details/新项目与通知面板保留既有结构。

- 旧子页面统一复用全屏队列工具栏，内容和操作分散：调整为居中限宽面板、保留上下文与就近操作。
- Help/结果/确认/Links长内容没有明确溢出提示：补滚动条，子页面保留PgUp/PgDn提示；滚动状态与按键逻辑不改。
- 确认页项目/任务长行直接截断：按现有字符宽度规则折行；退回原因和work-stopped选择保留原控制逻辑。
- 目录/接收者列表缺少位置提示、长路径难区分：显示位置/总数，目录行突出名称，底部展示完整选中值；选择行为不变。
- Telemetry界面文案恢复英文，保留原始中文材料。API的说明字符串不改，只在显示层转换已知宿主说明；不改遥测读取、分页、采集、SQLite或业务状态。

## 验证

只做相关合成/隔离验证与必要构建。Drover UI 23项与lib7项通过；Links错误显示定向检查日志/tmp/saddle-secondary-links.log。子页面48×24、80×24、180×50，既有确认点击/编辑鼠标/滚动/状态颜色检查保留。

Telemetry展示17个不同目标最终通过（全目标16过1因英语长句换行的旧单行断言失败，定向调整折行断言后该项通过；保留完整句意及中文用户正文检查）。最初Drover两项因新边框/折行的旧断言失败；中间检查还发现结果页不能省略不同于旧正文的最新反馈，已保留不同反馈，仅合并相同文本重复展示。以上过程失败不作为业务RED或一次全套绿。

日志/tmp/saddle-secondary-ui-{test,final,layout,verified,build}.log，/tmp/saddle-telemetry-english-{test,final,wrap}.log。构建使用独立target triple路径，不覆盖运行文件；未跑全仓测试/Clippy，未操作真实队列/代理。真实终端复看待用户。
