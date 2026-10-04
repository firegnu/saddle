# Saddle 交接

更新：2026-10-04。用户已认可设计语言 HTML，要求按清单逐页调整，后授权可独占文件的 UI 并行分派，并明确要求继续修正、集成和实际画面预览。本轮不走 Tasks 队列。

## 当前状态

- 主目录 `main` 保留此前直接完成的 Settings 未提交候选（DESIGN、settings、plugins/ui、ui 与 tests/settings）及未跟踪设计语言 HTML；不要覆盖、丢弃或夹带提交。
- 独立集成分支/worktree：`ui-integration-preview` / `../saddle-worktrees/ui-integration-preview`。已纳入上述 Settings 精确副本、宿主弹窗 `f704c1b`、Telemetry `1f68d2b`、Drover `59f91f1` 与修正 `b1573bf`。源码无合并冲突；任务书合并保留实现与审查记录。
- 三组主控审查已完成。Drover 极矮窗口新增帮助挤掉正文的问题已由原实现者修正，集成画面确认 80×10 正文恢复。其余候选取舍与验证边界见任务书；不宣称全套通过。
- 33 页实际 Ratatui 合成画面 HTML：`file:///Users/firegnu/Developer/personal_projs/saddle-ui-preview/2026-10-04-first-batch/index.html`。可切换 Settings、Tasks、宿主弹窗、Telemetry、Search 四主题。临时生成器与日志在 `/tmp/saddle-ui-integration-preview/`；只使用临时配置和合成数据。预览不等于真实终端/手机 SSH 验收。
- 记录：`docs/任务/UI并行-分组与派发-2026-10-04.md`、三份组任务书及 `docs/任务/UI第一批-集成与预览-2026-10-04.md`。

## 下一步

- 当前已经推进到具体画面回看；按用户针对画面的反馈继续同任务微调。不要再次停在泛泛的“可以继续”；不需重复询问已授权的修正、集成、审查操作。
- 外观回看后按项目规矩完成合并、推送、适用的安装验收与清理；此时尚未把候选合入 main，也未安装/升级真实实例。主目录 Settings 副本须在安全核对一致性后处理，不能强行 checkout 覆盖。
- Diff、Settings 剩余项、公共状态栏/Toast/notice/终端和 Agents 公共绘制尚在后续清单，留待串行/后续批次。本批结束不等于整份 UI 清单完成。
- T73 仅展示，T74 导航改造与移动端能力不搭车；不推进 Tasks 队列、不打开本轮未选择的遥测。

## 保留 agent 与 worktree

- `saddle/dev-ui-drover-1` → `ui-drover-polish`；`saddle/dev-ui-dialogs-1` → `ui-host-dialogs`；`saddle/dev-ui-reading-1` → `ui-telemetry-polish`。实现完成后空闲保留，供画面反馈与返修。不要影响其他用户 agent。
- Drover 修正提醒 `fa520d88-d267-448d-b195-e3805c0aa9c0` 已主动读取状态/回复并处理；重复到达时不重做。
- 历史 `corral-live-upgrade-research`、`review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow` 原样保留。

## 验证与运行边界

- 本轮仅局部展示与集成检查，最终数量/结果见集成记录。此前全套在 agent_capture 出现 4 项失败，根因未核实；历史 T68 测试问题不在本批搭车修复。
- 本轮未安装、未升级 Corral、未停止或重启用户 agent，已运行版本不能按候选提交号推断。
- 设计语言与具体取舍以 `docs/UI设计语言.md`、整理清单及集成分支 `docs/DESIGN.md` 为准，交接不重新决策。
