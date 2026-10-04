# Agents 头部工具菜单

2026-10-04，用户授权 saddle/main 直接实现，不委派，不使用此前 T63 的 task trace。

## 用户原话与已确认方案

“左侧agents区域的最上面的右边4个链接，感觉很不优雅。有什么好的ui上面的修改吗？先讨论。”

主控推荐顶部保留 Tasks 直达，Plugins、Telemetry、Settings 收进 `⋯` 菜单；用户明确：“我接受你的推荐。你直接上手吧。不用委派了。”

## 范围

只调整宿主头部入口及菜单导航，沿用原插件固定/可用性、管理、设置和遥测路径。主设计同步入口位置，既有测试调用改走菜单。不扩展插件协议、任务功能或自动启停，不部署或操作任务状态。

## 完成记录

- 原 `88f74a9` 基线创建 `agents-header-menu` worktree，由主控直接实施。
- 顶部呈现标题与 Tasks/⋯，下行只保留 Attention；窄列按内容宽度换行，固定入口超长继续截断、不可用继续弱化。菜单沿用主题，支持鼠标同一行按下/抬起、上下键/Enter/Esc、外侧关闭；更新提醒同时保留在 ⋯ 和 Settings 项。
- 原插件弹层的输入保护保持：菜单内 Settings/Telemetry 此时不可用，Plugins 可继续切换插件视图；菜单阻止后台布局写操作，关闭返回原输入焦点。
- 有效 RED：新增真实伪终端流程 `header_more_menu_opens_tools_and_restores_terminal_focus` 在旧实现因找不到 ⋯ 失败，`/tmp/saddle-header-menu-red.log`；实现后同例通过，随后补入外侧点击检查。
- 验证范围按局部入口改动选择：宿主 UI 组、菜单单元检查与 workflow（共用打开 Tasks/Plugins 的夹具改走新入口，因此这一目标集中跑一次）；未跑全仓 `cargo test --all-targets`，没有改插件业务、存储或协议。
- 菜单单元检查 2 passed；宿主 UI 组首次 49 passed / 2 failed，两个新布局预期修正后限定复核 2 passed（fixture 实际为一个 Agent、新布局在 52 列已可同行，换行样例收窄至 40 列）；合计 51 项最终通过。
- workflow 首次 **104 passed / 2 failed / 4 ignored**。新增菜单流程已通过；`plugin_overlay_protects_host_actions_and_restores_viewer_focus` 新增 Esc 后紧接按键被解释为组合键，改为等待菜单关闭后发送后续输入，不改断言或超时。另一个 `closing_a_start_target_keeps_the_created_agent_available_without_attaching:1868` 旧流程失败未改实现或测试。两项失败和最终菜单流程只做定向复核，共 **3 passed / 107 filtered out**。保留首轮结果，不宣称 workflow 首次全绿或旧时序根因修复。
- 自审：固定插件路径不变，菜单选择调用现有导航；键盘/粘贴/鼠标归菜单消费，外侧按下关闭后消费剩余手势；插件弹层和设置草稿保持原保护；更新提醒仍可见。没有独立审查或委派。
- 全部验证使用临时配置、假 CLI 与合成数据，共享 target；日志 `/tmp/saddle-header-menu-*.log`。画面依据 TestBackend 固定帧与伪终端流程，未声称日常终端人工视觉验收。

- `cargo clippy -p saddle --all-targets -- -D warnings` 最终通过；首轮发现新 UI 测试遗留的未用 import，删除后重验。`cargo fmt --check`、`git diff --check` 通过。源码审查与上述范围检查通过，按项目流程合并推送；本轮未 release 构建或部署。
