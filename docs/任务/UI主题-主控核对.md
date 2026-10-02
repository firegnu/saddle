# UI主题设计主控核对

2026-10-02，候选2c42a0b（基线ad67337）。公开status idle，带遥测reply末行为DONE。只改设计稿/任务完成记录，worktree干净，diff --check通过；未测试构建。

已核：Config现有deny_unknown_fields、Theme默认/41字段、Settings草稿及只保存编辑键、插件五个主题字段、Drover自身默认色、Diff固定语法主题。设计不需改Corral，无反向依赖。

结论：方案尚未定稿或批准，不派实现。

- 可采用：Colors顶部Theme行、custom标记、现有Save/Cancel与预览、Dune默认兼容旧配置、单项Default恢复跟随所选主题。主题键/覆盖合成是最小配置扩展，旧版不识别新theme键的降级限制必须保留。
- 必须纠正文档：Terminal宣称完全无RGB，但同时保留claude/codex/omp的RGB品牌色，自相矛盾。实际选择全ANSI时须明确完整字段映射；不声称已验证渲染对比度（当前仅声明值计算）。
- 待用户选择：作者A按数值等于旧预置来去除覆盖，会把有意设置的相同值也清掉；B保留全部覆盖，又可能让换主题几乎不变。主控推荐更直接的产品行为：选择主题即在草稿中载入整套新配色，之后逐项覆盖，Save才生效，Cancel恢复；升级本身不重置现有颜色。此为新提议，未经批准。
- 预置主控建议：Dune保留当前基线，Tide为较完整的冷色配色，Terminal跟随终端。当前稿Tide只改Agents中性色，效果局部；待用户看过线框与方向后合并一次修订，不反复扩审计。
- 插件目前只有text/muted/background/accent/error跟随；Tasks独有状态色、Diff语法色、agent终端内容不属于宿主已有覆盖范围。此限制先明示，不自动改协议。

后续：给用户简明线框和切换选择；答复后一次性给原设计者修订，再核定稿。用户agent、真实配置和队列未动。

## 用户批准后的定稿

用户回复“可以的。如果没有其他问题就实施吧”。主控已按批准的整套草稿载入规则定稿docs/UI主题设计.md并更新DESIGN；Terminal矛盾通过全部字段ANSI规则消除，Tide覆盖现有公共界面色；接口限制保留。设计通过，未实现，不跑测试构建。原A/B提议作废，不继续让原设计者往返。

## 实施首轮核对（候选5c06809，进行中）

- 原实现者idle、attached0，公开reply最后DONE且遥测首尾配对stored。分支只改theme/config/settings、直接测试和文档，未改Corral/插件协议/业务；diff check通过。
- 旧配置Dune+显式覆盖、Theme行/全套草稿替换、Default删覆盖、Save/Cancel主要实现与批准方案一致。原测试“配置有全颜色示例”和F2直接首颜色假设已作必要适配。设计结构取舍可接受；不把存在的覆盖按数值相等消掉。
- 接受Tide bg/overlay/text继续default，其余公共语义色与Agents整体变冷；Terminal所有字段均ANSI/default，无RGB。静态对比度计算未独立重跑，不声称实际终端测过。
- M1必须改：src/settings.rs remove()删除最后一个键时，没有next/previous可附着，已提取的独立注释直接丢弃；实现完成记录也承认“表内键全部删除时，上方注释不保留”。选择主题清全部覆盖及单项Default是正常路径，违反已批准“保留无关配置和注释”。需保留独立注释，即使[colors]最终为空；被删键本行说明可随键删除，不需扩成通用TOML格式重写器。必须由原实现者最小修正，目标RED/GREEN及直接回归，不再全套/Clippy。
- 开发者标准未绿：plugin_resources3、workflow9失败；开发者擅自重复全套超出预算，不接受“没改这些文件”作为无关证明，原失败保持。主控正在运行一次cargo test --all-targets --no-fail-fast及Clippy，no-fail-fast只为保留所有target结果，不重复全套。开发RED原始日志尚未取得，当前只有自述和完成记录。


### 主控标准检查及集中返工范围

主控一次全量（--no-fail-fast）542通过、12失败、5忽略，exit101；失败与开发报告的3项plugin_resources、9项workflow一致。日志/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-theme-controller-hek2ddkh/test.log，Clippy结果以results.json为准。

已逐项看失败断言/渲染：plugin_resources仍查未截断名称、旧`> `选中符、旧内联ID/状态、首屏包含全部详情/CLI退路；新分栏布局截断长名、详情滚动，截图可见数据存在。workflow两项Drover等旧`─ Add task`/`─ All pending ─`标题（新边框为`┏ …`）；七项Plugins等旧`Changes here apply immediately.`及用其消失作异步屏障，而当前文案是`Changes apply immediately.`。上述生产布局均在Theme基线前存在，本轮src/plugins和plugins/drover未改，Theme默认保持旧值；静态及截图支持旧测试假设过时。未运行main基线，不把本轮当全套通过，也不泛称所有历史失败同因。

限定原实现者：M1保留独立注释；并只对这12项失败中已证实的旧布局/文案/等待条件最小适配测试，使原业务含义继续受检查，不改插件或Drover生产代码。不删除断言、不跳过、不加大超时。如果发现真实生产问题先报告。返工只目标和直接回归，不全套/Clippy、不重建原RED；补已有原始验证日志路径，缺失如实说明。
