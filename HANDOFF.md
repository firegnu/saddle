# Saddle 交接

更新：2026-10-04。用户认可设计语言和 33 页实际绘制预览，要求确认未破坏逻辑后合并部署。第一批 UI 已完成审查、合并、推送、安装与开发目录清理；本轮未走 Tasks 队列或开启遥测。

## 第二批正在实施（2026-10-04）

用户接受剩余 UI 整理及 Agents 小幅方案，明确授权可并行则直接开干。按独占文件已派出三个 Claude Code opus[1m]/high 实现者：`saddle/dev-ui2-host-1`（ui2-host，公共提示+Agents/终端）、`saddle/dev-ui2-settings-1`（ui2-settings，Settings 剩余内页）、`saddle/dev-ui2-diff-1`（ui2-diff）。起点任务书提交 `85ed2fe`；具体 instance、完成提醒 request_id、范围见 `docs/任务/UI第二批-分组与派发-2026-10-04.md` 和三份组任务书。

- Diff 返工 `60f7bf1` 已复核通过：34 列 Staged+error 同时可见，主控针对性绘制检查 1 passed，唯一必须改项关闭；初版与返工提醒均已处理。已建立独立 `ui2-integration`，本轮纳入 Diff，保留原 agent/worktree 供集成反馈。Settings `8735ff8` 已审查通过、无必须返工项，本轮纳入 ui2-integration；其完成提醒已处理。宿主组仍待完成。第二批尚未合并 main/部署。完成提醒到达后按组读取 status/reply、检查 diff 与展示结果，原 agent 同任务返工，再集成；不要只停在复述 DONE。不走 Tasks 队列，不开启遥测。
- 公共区域与 Agents 同组避免 ui.rs/terminals.rs 交叉写；Settings、Diff 可独立并行。保留 Agents 3a、行高、入口顺序和业务行为，不重做卡片。所有边界见任务书。
- cargo 必须用 `/tmp/saddle-ui2-cargo.py`，对整条命令串行加锁并设置共享 target，避免检查期间另一 worktree 替换产物。三组只跑一条直接展示检查，不跑全套/Clippy；跨组旧文本断言报告给主控，不能削弱业务断言。
- 上一批与红点调整已收尾；以下当前安装/历史验证记录保留，第二批未完成时不能覆盖成已部署。

## 第一批与圆点调整的已部署状态

- `main`：UI 合并 `5f39f88`，收尾空提交 `85a780b`；之后仅补部署与交接记录。Settings 原未提交候选已核对后安全提交 `0452406`，设计语言 HTML 已入库。
- 新安装包：Saddle 入口指向 `~/.local/share/saddle/versions/4ac4691`（更新圆点微调），Drover 注册仍为 `5f39f88`。Corral、Diff、其他插件和主配置未改；旧不可变包保留。
- **当前窗口尚未热更新**：核验时宿主 PID 72000 仍加载 `a542931`，已有 Drover 进程仍加载 `711ab18`。正常关闭并重开 Saddle 即可加载新版，不需要退出主控或其他 agent；不要擅自杀进程。
- 第一批包含 Settings 家族、Drover、宿主弹窗和 Telemetry 展示整理。80×10 Add/Edit 正文被帮助行挤掉的问题已修正；workflow 旧文字定位已适配，业务断言保留。
- 预览：`file:///Users/firegnu/Developer/personal_projs/saddle-ui-preview/2026-10-04-first-batch/index.html`。独立文件未随 worktree 删除。实际 Ratatui 合成画面，不冒充真实终端或手机 SSH 验收。

## 验证与已知问题

- 完整标准套件原始结果 645 通过、21 失败、9 忽略；9 项 UI 文字定位已适配。明确重建宿主和 Drover 后完整 workflow **101 通过、0 失败、4 忽略**。Clippy `--all-targets -- -D warnings`、release 构建与 diff 检查通过。
- 剩余候选 agent_capture 3 项、drover_telemetry 9 项失败，全部在干净旧基线 `9f721d0` 复现，相关业务源码和测试未改；不宣称全套绿或旧故障已修复。不搭车扩展成遥测修复任务。
- 共用 target 的基线构建曾让后续 workflow 读到旧宿主产物；此次已明确重建集成宿主/Drover 后验证通过。后续跨 worktree 切换时核对实际产物，避免把缓存结果当候选运行证据。
- 部署备份、BUILD.txt、installed.json 与验证日志：`~/.local/share/saddle/backups/ui-first-batch-20261004-115727/`。详细证据见 `docs/任务/UI第一批-集成与预览-2026-10-04.md`。

## 清理与保留

- 三个 UI 开发分支/worktree、集成与 detached 基线 worktree 均已安全移除；对应三个 `saddle/dev-ui-*-1` agent 随工作目录关闭。第一批清理时公开 corral ls 仅余主控 `saddle/main`，instance `0ed54c671118`；原宿主 instance `ab0d3e34a167ecbe` 保留。
- 旧 UI 完成提醒已处理；关闭 agent 后重复提醒不触发重做、重派或队列放行。
- 历史 `corral-live-upgrade-research`、`review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow` 四个 worktree 原样保留。

## 下一步

- 用户正常重开 Saddle 后回看本批实际界面，有反馈则继续对应 UI 小修。部署成功与运行窗口已更新应分别核验。
- Diff、Settings 剩余 H18/H19、公共状态栏/Toast/notice/终端和 Agents 公共绘制现已按上方第二批范围授权实施；本批不包含额外产品能力或其他队列任务。
- 保持设计语言与整理清单两份文档。设计与批准取舍看 `docs/DESIGN.md`、`docs/UI设计语言.md` 和 `docs/调研/UI整理清单-2026-10-03.md`；分组和各组审查见 `docs/任务/UI并行-分组与派发-2026-10-04.md`。T74 导航改造、移动端能力不搭车，不推进 Tasks 队列。

## Settings 更新圆点微调（2026-10-04）

用户看到更新提示后反馈「这个小红点有点小。你自己调整一下吧」。本次主控直接改为加粗实心圆 `●`（原 `•`），仍单列、原色、原位置；更新检测与 Settings 命中区域不变。实现 `e23fd89`，合并 `b25fa21`，收尾 `4ac4691`，开发 worktree/分支已清理，无新委派。

现有 UI 检查 31 项通过（含 52/30/20 列的更新提示）、宿主 lib Clippy 与 release 构建通过；纯视觉小改不制造失败测试、不重跑刚完成的全套，旧基线失败状态沿用上文。部署 `4ac4691` 仅替换 Saddle，配置/注册表 hash 未变，入口和 `--help` 回读成功；备份与日志在 `~/.local/share/saddle/backups/update-dot-20261004-125029/`。本次未重启正在使用的宿主或任何 agent，正常重开 Saddle 后加载。
