# Saddle 交接

更新：2026-10-04。主控 `saddle/main`，分支 `main`。前次交接 `3c8cf50` 已推送；本轮按用户要求重排 GPUI 前待办并同步评估文档，无产品代码修改。

最新补充：用户随后要求将 GPUI 前的任务按难易重排，并明确 T76 明天并行探索。已公开重排并回读 `T49 → T64 → T62 → T56 → T76`，其后顺序不变；18项 Pending、current/awaiting为空。四项不是 T76 前置门槛。此次仅评估排序，未派发、实现或修改任务正文；T77已结束trace未复用。详见 `docs/任务/GPUI前后-待办重估与排序-2026-10-04.md` 最上方更新；T49先判断是否必要，有必要则同任务内讨论并落地，不默认造新系统。

## 会话摘要

T77 已完成设计对齐、Drover UI 实施、主控审查、集成、合并清理和用户授权后的部署。交接现场确认宿主和 Drover 均已加载 `aab70c8`；公开任务状态为 Done，绑定 trace 已结束。本轮结束，等待用户后续指令，不启动下一任务。

## 完成的工作

- T77：突出 `Open in Telemetry ↗`，状态之后新增 `Look further`，提供正文内同源入口，整理关键信息顺序、弱化技术 ID。完整保留原始正文、关联、历史运行及全部轮次跳转；未改采集、查询、存储、协议或任务状态机。
- 提交：设计一致 `62560ac`，实现 `8988d58`，主控审查 `6643d32`，合并 `dc0f8f9`，空收尾 `2d25baf`，部署记录 `dfc8b2d`，均已推送。
- `t77-telemetry-ui` worktree/分支已清理，原 agent `saddle/dev-t77-design-1` / instance `0994bb2200d8` 随工作目录一并关闭。迟到提醒不得重复处理或重发实施。
- 此前 T66 分层 UI 回归约定、T74 Tasks 直接入口、T63 统一搜索和 Agents 头部菜单均已合并、推送、部署与清理；历史详情看各任务文档，不重新打开已确认设计。

## 当前安装与运行现场

- 安装入口 `~/.local/bin/saddle` → `~/.local/share/saddle/versions/aab70c8/bin/saddle`；Drover 登记 `versions/aab70c8/plugins/drover`。
- **已确认重启生效**：公开宿主实例 `5269a2eaf964917d`，宿主 PID **12296**、Drover PID **12299**；`lsof` 确认两者均加载 aab70c8 包。不再沿用“待重开”的旧状态。
- 从干净 aab70c8 release 构建宿主及 Drover，7.83秒、退出0；help、安装校验和及隔离 Drover initialize 通过。其他产物沿用旧包，配置、其他插件登记/启用/固定状态及 Corral 链接保持。
- 备份及部署证据：`~/.local/share/saddle/backups/t77-deploy-20261004-224400/`。旧包保留。后续源码提交只写文档，不需要再部署。
- 公开 `corral ls` 仅有主控 `saddle/main` / instance `9f8a73396d8a`；未重启或迁移主控 agent。
- 保留四个历史 worktree：`corral-live-upgrade-research`、`review-telemetry-design`（detached）、`t38-dispatch-study`、`t55-notification-flow`，不擅自删除。

## 验证与已知问题

- T77 定向 Drover lib/ui/ui_drover_polish：35项通过。全仓 Clippy、fmt/diff 通过。
- **全仓并非全绿**：全目标首次覆盖 694 passed / 14 failed / 9 ignored；首次在 app 停止后只续跑尚未执行目标，没有重跑整套。11项失败定向复核为10 passed / 1 failed，保留首次结果。
- 仍有4项未解决失败：宿主 `tests/app.rs` 的3项旧头部 Plugins/Settings/Telemetry 位置断言；`tests/drover_telemetry.rs::without_a_record_context_the_delivery_goes_the_plain_way_once` 期待 disabled，实际为 budget_exhausted。相关路径均无T77修改；后者根因未定位，不宣称修复。鼠标表单流程单项复核通过也不代表根因解决。
- 详情：`docs/任务/T77-遥测展示整理主控审查.md`；日志 `/tmp/saddle-t77-integration/`。没有删测试、放宽断言/超时或改串行来掩盖问题。不未经授权扩大到这些问题的修复。
- 本次实现者检查过合成画面与渲染测试，但未记录用户对实际视觉效果的专项确认；不能把部署成功当成视觉验收证据。

## 最新讨论与待续接边界

- 用户问“改动了什么，是否都从SQLite读取”：已说明遥测事件/元信息存SQLite，完整正文另存内容文件；任务状态/正文/运行历史来自Drover任务文件，仓库参考来自Git等路径。Drover通过Saddle公开查询读取遥测，不直接读SQLite。本次只改数据取回后的UI。
- 用户问“主控生成的任务书在哪看到”：当前路径为 Tasks → Open in Telemetry ↗ → 对应链路 → `Brief snapshot` → Enter / `Read full body`。T77设计快照事件 **#329**；实际实施交付快照 **#341**。快照独立于已删除worktree，仍可读取；仓库当前文档还包含后续完成记录，与派发时快照不同。
- 当前没有独立“任务书”直达入口。主控指出该层仍不够直观，但用户随后只要求 handoff+提交推送，**没有授权继续开发或登记新任务**。不要据此自动扩大T77或启动新任务。
- T77现场只读核验：`status=done`，run `97aa8e9954855bb2cbd15ee61e47d012`；本主控未执行 Submit/Accept/Return。trace `e588f2f5-cb18-4071-aa63-c093fc449032` 的 `closed_at=2026-10-04T14:52:06.636826Z`，capture_enabled=false。**本次记录链路已结束，不再追加或复用上下文。**
- 记录目录 `/tmp/saddle-t77-20261004/`；设计/实施和审查历史身份保存在 ids.json。`handoff-task.json`、`handoff-trace.json` 是本轮公开查询结果。不要把结束链路用于后续独立工作。
- 用户一般允许主控委派；T63的Codex-only、头部菜单的主控直接实现均为对应任务的选择。下一任务按当次要求和AGENTS执行。
- 按 `docs/UI回归.md` 选验证范围，已通过且代码未变不重复全量。Cargo共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 优先阅读与下一步

- `AGENTS.md`、`docs/UI回归.md`：开发、分派、分层验证及公开任务接口规则。
- `docs/任务/T77-遥测展示整理设计.md`、`docs/T77-遥测展示整理方案.md`：设计和对齐记录。
- `docs/任务/T77-遥测展示整理实施.md`、`docs/任务/T77-遥测展示整理主控审查.md`：实施、部署及验证限制。
- `docs/DESIGN.md`、`docs/UI设计语言.md`：长期设计与理由；`plugins/drover/README.md`、`docs/遥测使用.md`：操作及查询说明。
- `docs/任务/Agents头部工具菜单.md`、T63/T74实施文档、`docs/任务/T66-UI回归样例.md`：此前已交付工作。

暂无T77范围内未提交的产品实现。交接提交推送后停止，等待用户新指令；不自动修复遗留测试、不做任务书直达入口、不启动T76或其他待办。
