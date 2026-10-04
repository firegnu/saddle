# Saddle 交接

更新：2026-10-04。主控 `saddle/main`，分支 `main`。T77 已完成源码合并与清理；本次交接随后随 main 推送。

## 当前优先续接：T77 已落地，未部署

- 用户授权主控与 agent 达成设计一致后持续落地。T77 实现 `8988d58`，主控审查记录 `6643d32`，合并 `dc0f8f9`，空收尾 `2d25baf`。
- 仅整理 Drover 任务详情：突出 `Open in Telemetry ↗`，状态之后新增紧凑 `Look further`，支持正文同源链接，调整关键信息顺序并弱化技术 ID。原始内容、历史信息、全部轮次筛选保留；采集、查询、存储、宿主和任务状态未改。
- 主控 T77 范围审查通过，**全仓并非全绿**：全目标首次覆盖 694 passed / 14 failed / 9 ignored；11 项失败定向复核为 10 passed / 1 failed。仍有 3 项宿主 app 旧头部入口断言失败，及 `without_a_record_context_the_delivery_goes_the_plain_way_once` 的 budget_exhausted/disabled 不符。后者根因未定位，不宣称修复；相关源码均无 T77 修改。本任务不搭车修复这些问题。全仓 Clippy、fmt/diff 通过。
- 细节见 `docs/任务/T77-遥测展示整理主控审查.md`，日志 `/tmp/saddle-t77-integration/`。首次因 app 失败停止后，只续跑未执行目标；没有重跑整套，未删测试、放宽断言/超时或改串行。
- `t77-telemetry-ui` worktree 与分支已安全删除；原 agent `saddle/dev-t77-design-1` / instance `0994bb2200d8` idle、attached=0 后，随工作目录一并关闭。不要处理迟到提醒或重发实施。
- **未部署**：日常安装仍是 `b9ff8e0`，不能把源码合并当成界面已更新。后续部署只按用户新指令执行。本轮未 Submit/Accept/Return、未关闭 task trace、未派发下一项；T77 仍待用户验收操作。
- 本任务记录目录 `/tmp/saddle-t77-20261004/`；trace `e588f2f5-cb18-4071-aa63-c093fc449032`，run `97aa8e9954855bb2cbd15ee61e47d012`，实施 dispatch `97ef21ba-0239-463b-be45-4b73d5b31ac5`，主控实施审查 dispatch `378a4b14-54a7-430b-94c8-5cfcf6ae2bb1`。属于 task trace，由用户任务验收/退回流程结束，主控不 close。

下方为前一轮历史交接，涉及“本轮”和待办状态时以上方 T77 最新说明为准。

## 会话摘要

今天完成 T66 分层 UI 回归约定、T74 Tasks 直接入口、T63 统一搜索，以及最后的 Agents 头部工具菜单。源码均已合并推送，对应工作区已清理；产品已部署，交接现场确认当前 Saddle 已加载最新菜单版 `b9ff8e0`。用户接受 Tasks 缺席时右侧只保留 `⋯` 的布局，本轮结束，等待新指令。

## 已完成与验证

| 工作 | 交付与提交 | 验证记录 |
| --- | --- | --- |
| Agents 头部菜单 | 用户批准并要求主控直接实现、不委派；顶部保留 Tasks + ⋯，Plugins/Telemetry/Settings 收进菜单。实现 `0447361`，合并 `c37e262`，空收尾 `f34dd1b`，部署记录 `ac4eb43` | 菜单 2 项、宿主 UI 51 项最终通过。workflow 首次 104 passed / 2 failed / 4 ignored；两项失败及最终菜单流程定向复核 3 passed。Saddle all-targets Clippy、fmt/diff通过，未跑全仓测试 |
| T63 统一搜索 | 用户指定 Codex；搜索已有 Agent 名/cwd 末级项目名、插件标题/ID和六个设置页，不增加 Drover 登记项目/单条任务搜索。实现 `2a36dbd`，合并 `40bdf5c`，空收尾 `e5940dc` | 主控审查通过，无独立交叉审查。集中全量首次 702 passed / 1 failed / 9 ignored；唯一失败单项复核通过。全量 Clippy、fmt/diff通过 |
| T74 Tasks 入口 | Settings → Plugins 固定一个插件入口，Agents 焦点 `p` 打开；项目识别、读取错误及草稿边界经返工/独立复核。实现 `e6edaca`，返工 `439c5e3`，合并 `a62eecf` | 首次覆盖合计 697 passed / 2 failed / 9 ignored；两项失败限定复核通过，Clippy/fmt/diff通过 |
| T66 UI 回归约定 | 现有样例索引、运行入口与分层验证约定，补一个 Diff 合成样例；无产品运行代码变化。实现 `77f5a4b`，合并 `de5e6b6`，空收尾 `1215793` | 相关四组 135 passed；Diff 定向 Clippy及预览检查通过，未跑全量 |

上述失败首轮结果均保留，不能把单项复核通过写成首次全量全绿。头部菜单的插件弹层测试补了 Esc 后等待关闭，断言/超时不变；另一项启动时序失败未改。T63 的 `native_mouse_buttons_cover_forms_and_stop_confirmation:851` 限定复核通过但根因未定位，不搭车修复。详情及原始命令见对应任务文档；日志 `/tmp/saddle-header-menu-*.log`、`/tmp/saddle-t63-integration/`、`/tmp/saddle-t74-integration/`。

## 当前安装与运行现场

- 安装入口 `~/.local/bin/saddle` → `~/.local/share/saddle/versions/b9ff8e0/bin/saddle`。从干净 `b9ff8e0` release 构建宿主，help与SHA-256校验通过；后续源码提交仅为文档，无须为文档重新部署。
- **交接现场已确认重启生效**：公开宿主实例 `890a0adae29f8d05`，PID **28742**；`lsof` 显示加载 `versions/b9ff8e0/bin/saddle`。不再沿用此前“待重开”的状态。
- 本次只更新宿主；Drover仍沿用T74产物，其他插件、配置、登记及Corral链接保持。旧安装包保留，最近备份日志 `~/.local/share/saddle/backups/header-menu-deploy-20261004-213528/`。
- 公开 `corral ls` 仅有主控 `saddle/main` / instance `9f8a73396d8a`。T74两个agent、T63原Codex均已按流程关闭；头部菜单未创建agent。
- 本轮新增worktree/分支均清理。四个历史worktree保留：`corral-live-upgrade-research`、`review-telemetry-design`（detached）、`t38-dispatch-study`、`t55-notification-flow`；不擅自删除。

## 待完成与授权边界

- 本轮实现和部署**暂无已知待完成工作**。没有遗留功能改动待提交；本次文档提交推送后停止，等待用户反馈或下一项。
- 用户最后已确认：未安装/卸载/未固定时不显示 Tasks，右侧仅 ⋯；已固定但停用时保留弱化入口。完整设计以 `docs/DESIGN.md` 最末节和 Tasks 设计为准，不重新打开已认可的布局决策。
- 未自动 Submit/Accept/Return、关闭任务 trace 或派发下一项；真实队列状态本次未查询，不能沿用旧 Running/Pending 快照。T76等历史待办不是实施授权。
- 用户一般允许主控委派；T63明确只用Codex，最后头部菜单明确主控直接做、不委派。后续按当次用户指令和AGENTS执行，不把本次例外扩为永久规则。
- 验证依 `docs/UI回归.md` 按范围选取；已通过且代码未变不重复全量，不删测试、放宽断言或超时。所有Cargo共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 任务记录续接

以下均是历史任务绑定，不是新授权；主控未关闭这些 task trace。若用户续接同任务，先公开查询当前状态与 `closed_at`，不猜最新run、不重复迟到提醒、不重放业务。

| 任务 | trace_id | run | 记录目录 |
| --- | --- | --- | --- |
| T63 | `16b9ad30-24fa-46c1-b07b-f3ea48e8656f` | `1aceb478636a305c942d7b155e57bc79` | `/tmp/saddle-t63-20261004/` |
| T74 | `0a8a12d6-8ea4-477a-b00c-1e1fe470919c` | `5e378db26877a85fefe4684199d17910` | `/tmp/saddle-t74-20261004/` |
| T66 | `026876b7-e42a-47f0-a35f-5c0dac7a60ed` | `b3db4bf30d9249bbfe7bb0e023a0322d` | `/tmp/saddle-t66-20261004/` |

T63实施dispatch `c337173f-6d84-40f8-b8ca-4656c120b640`、审查dispatch `f7bb95d9-f4fb-41a8-91eb-c9400f163f93`；实施提醒 `80909340-c726-40a5-bf36-e433293901e2` 已处理，实际收尾与部署记录已保存。头部菜单是后来独立口头任务，没有复用T63遥测或操作Tasks。

## 优先阅读与下一步

- `AGENTS.md`、`docs/UI回归.md`：开发、验证和公开任务操作约定。
- `docs/任务/Agents头部工具菜单.md`：最新实现、检查、部署记录。
- `docs/任务/T63-统一搜索入口实施.md`、`docs/任务/T74-Tasks入口实施.md`、`docs/任务/T74-Tasks入口独立审查.md`：已交付功能及验证限制。
- `docs/任务/T66-UI回归样例.md`：分层测试与样例整理结果。
- `docs/DESIGN.md`、`docs/UI设计语言.md`：设计与理由；`plugins/drover/README.md`：公开队列接口。

等待用户新指令或反馈，不自行启动下一项。
