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
