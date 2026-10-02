# Saddle 交接

更新：2026-10-02。当前分支 main；本次交接前 HEAD 与 origin/main 均为 `8c3f157`，工作区干净。

## 会话摘要

单任务遥测结束边界已实施、审查、合并推送，并经用户授权完成日常部署。用户已重新打开 Saddle；宿主、Drover 和两份 dispatch 技能核验正常。当前没有正在实施或等待审查的任务，等用户下一项需求。

## 已完成

- 功能候选 `2c29d91`，合并 `d8e31f6`，代码收尾 `9231309`；部署记录 `5fca623`，部署收尾 `8c3f157`，均已推送。
- Saddle 增加正式 trace close、SQLite schema v2、v1 只读兼容、结束后拒绝追加及有限查询区间；合法旧 context 不妨碍业务执行一次。Drover 在 Accept/Return 后尝试 close，Submit 不 close。技能资源更新 revision 3。
- 主控与独立 Codex 重档审查通过，必须改0、建议改0。主控原标准441过1败5忽略、Clippy通过；唯一旧Theme测试预期适配后目标1绿，补遗漏24 targets共128绿；独立13项目标绿。不能改称一次全套绿。
- 附带只适配旧示例漏Theme参数和旧workflow默认颜色断言，未修改Theme生产行为。原始证据及逐项裁定已落实施核对/独立审查文档。
- 本项开发和部署未选择遥测，没有复用旧Theme trace，也没有关闭旧trace、操作Tasks队列或修改Corral。
- 本次实施、独立审查工作区及分支已清理，目录删除后关闭对应自建agent；设计会话此前也已清理，迟到提醒若not_found直接忽略。

## 日常部署现状

- 用户退出后，确认无Saddle实例和宿主/Drover进程，取得SQLite一致性备份，再成套替换宿主与Drover包。用户已重新打开，核验时实例 `693d73ccc278e385`。
- 安装源码 `bf056e1`；日常 `~/.local/bin/saddle` 仍链接到 `../saddle-worktrees/.target/release/saddle`。宿主SHA256：`5e1fecbd977ed311479abe2f9e92a451fea5b203dfec93dcb9271759d82d9c5d`。
- Drover包 `plugins/drover/dist/drover-plugin`，二进制SHA256：`58b85bffdb53314237153fdebe2c08b709b575525f3cbd8a0bcd7e998cfc765a`。实际进程存在，公开projects只读请求最终complete/result.ok=true。
- Claude Code、Codex两处corral-dispatch技能均 `revision=3 / owned_current`，逐文件匹配源码。主控已重读新版指引；其他已运行agent的上下文不会被安装强制刷新，下一次选择记录时须重新读同版本遥测操作.md。
- config.toml/plugins.toml逐字未变。最终只读核验时真实数据库仍v1；下一次正常写连接才事务升级v2。未为验证写真实库或close历史trace。公开settings get正常：enabled=true、generation=3。
- 长期备份：`~/.local/share/saddle/backups/telemetry-boundary-20261002-234002`，含旧宿主/完整Drover包/配置/技能/所有权，以及SQLite backup快照（v1，integrity ok）。
- 部署证据指针 `/tmp/saddle-boundary-deploy-path`，目录 `/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-boundary-deploy-7tjwbixl`。哈希、构建及安装/启动核验详见部署记录。
- 未发布远端release，未做真实任务端到端重验。旧二进制不能读写v2；回退需接受遥测不可用或恢复迁移前快照（会损失快照后的记录），不得自行回退。

## 当前规则与边界

- 口头要求记录只覆盖当前任务，含同任务返工、审查和实际收尾；主控完成交付且无待决事项后负责正式close。下一独立任务未要求则不记录，再要求新trace。阶段产出不是任务结束，closure note也不等于close。
- Tasks原项目默认和本次记录覆盖、手动Submit/Accept/Return流程不变；绑定run的trace由Drover终态结束。已结束trace不能重开，历史仍可读。设计理由与契约以docs/DESIGN.md及边界设计为准。
- 主控按AGENTS和corral-dispatch技能分派，用户明确要求自己做时照用户指令；不回退dlog/旧route.py。涉及Corral改动或反向依赖，先停相关部分告知用户。
- 纯UI任务严格限定显示范围，不擅改展示语言、不扩业务逻辑或验证预算。当前Theme已有Dune/Tide/Lagoon/Terminal；终端默认色跟随主题，显式ANSI/RGB保留。此前“Theme待实施/待重开”旧交接已过期。
- 不自动迁移Corral，不关闭用户agent，不推进真实任务队列。旧Drover/dispatch-log已退出日常链路，旧本地数据保留，不恢复服务。

## 保留会话与工作区

最新公开核对仅剩用户会话：`corral/main`（158b4aa04dd7）、`saddle-e2e-20261002-113520/main`（feb85d825521）、`saddle/main`（4ebbecf235f8）。不要送话或关闭其他用户会话。

保留worktree，不自行清理：

- `../saddle-worktrees/review-telemetry-design`：detached 5ddd544。
- `../saddle-worktrees/t38-dispatch-study`：c15bc4d。
- `../saddle-worktrees/t55-notification-flow`：3cc417d。

已关闭本次自建实施f99302444ec2、审查3459f08440da及原设计7f019dd5dae6，不恢复。

## 待办与已知限制

- 暂无本任务必须继续处理的工作。下一步等待用户，不自行追加测试或开发。
- 历史Drover两次budget_exhausted根因未知；一次合成探针仅定位shell入口后至宿主首回执前，本轮主控14项通过不等于原因修复。没有调整预算；取消路径独立审查仅静态核对。
- 既有非阻断03B S1：资源安装失败时skills保留原因提示可能遗漏；目录身份核对与rmdir非原子。04B S1：无MAIN_AGENT且准备失败时一行反馈可能误写sent the plain way，交付首行及not_sent/manual_text正确。不顺手扩修。
- 旧Theme trace `e72bb30e-3fc2-4f69-b910-36c20ee77636` 未操作；历史closure不自动转为正式结束。不得把它用于新任务。

## 优先阅读

1. `AGENTS.md`、`docs/DESIGN.md`、`docs/任务遥测接口契约.md`。
2. `docs/遥测单任务边界设计.md`、`docs/任务/遥测单任务边界-实施.md`。
3. `docs/任务/遥测单任务边界-实施核对.md`、`docs/任务/遥测单任务边界-独立交叉审查.md`。
4. `docs/任务/遥测单任务边界-部署记录.md`、`docs/遥测使用.md`、`plugins/drover/README.md`。

早期阶段01—05、UI整理和Theme细节已有对应任务/审查文档及Git历史，不再在HANDOFF重复过期安装状态。
