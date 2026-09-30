# Agents 顶部入口与状态分层

用户先询问截图中的排版和配色能否更好，同意如下建议后要求「可以。开始搞」：

```text
Agents · 4                   Plugins  Settings
Attention · 0
─────────────────────────────────────────────
```

Agents保持暖白加粗；Plugins/Settings同排、同色清晰，悬停提亮；Attention为0时弱化，有待处理事项时强调文字和数字。

主控亲自实施；Dispatch `3ffa43dbf0a6486c9203bfa3ebc0b72c`。范围仅宿主顶部布局与配色，窄窗口使操作入口移到状态下方，保留点击和现有快捷键。真实队列、agent、插件包与三处Diff演示改动不变。

实施检查：正常/窄宽度文字与点击范围、悬停/Attention颜色；现有入口点击、插件面板、设置保存/侧栏缩放和标准检查；发布候选验证后更新日常宿主，不强制重启窗口。

## 主控审查与验证

结论：Approve。改动限于顶部绘制、点击几何、相关预期与设计说明。正常宽度操作同排右对齐；52/30/20列侧栏分别验证标题同行、状态下方同行、操作分行，Attention保持第二行且不与入口重叠。Plugins和Settings同色，悬停提亮；Attention有条目时文字与计数共用状态色。既有入口处理和快捷键未改动。

- RED：旧实现的Settings仍在第二行、Plugins悬停未提亮，两个目标检查有效失败；日志 `/tmp/saddle-header-actions-red.log`。实现后目标检查通过。
- 相关回归：UI 27项、App 3项、Attention 4项通过，覆盖设置保存及侧栏调整；日志 `/tmp/saddle-header-actions-target.log`。
- 标准检查：`cargo test --all-targets -- --test-threads=4`，356 passed / 0 failed / 5 ignored；`cargo clippy --all-targets -- -D warnings`、fmt和diff检查通过。日志 `/tmp/saddle-header-actions-{all,clippy}.log`。
- 独立release候选：2项顶部布局/配色检查及1项真实PTY设置保存/侧栏调整通过；日志 `/tmp/saddle-header-actions-release-check.log`。候选SHA-256 `44f99c74dbf639e87e12b076870af7cff235061c606ef87ad31e5837eb75cce8`。

用户当前窗口的实际显示需重启后观察；自动化检查不等于已观察该窗口。未操作真实agent、队列、服务或插件包。
