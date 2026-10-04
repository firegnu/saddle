# Saddle 交接

更新：2026-10-04。用户认可设计语言和 33 页实际绘制预览，要求确认未破坏逻辑后合并部署。第一批 UI 已完成审查、合并、推送、安装与开发目录清理；本轮未走 Tasks 队列或开启遥测。

## 当前待办排序已重估（2026-10-04）

用户已自行将完成的 T73 Drop，并注明队列外完成。之后要求登记 GPUI 桌面化方向，已新增 T76；又授权重估全部任务并排序。本次通过公开 Drover list/show 读取 23 条完整 Pending，再 move 10 次，完整回读全部任务内容和顺序。全部仍 Pending，current/awaiting 均为空，未派发/启动/放行。

当前顺序：T68 → T67 → T66 → T76 → T53 → T49 → T74 → T63 → T62 → T64 → T65 → T59 → T69 → T61 → T70 → T75 → T71 → T56 → T32 → T60 → T34 → T50 → T72。

建议先收口测试基线、版本事实、已有 UI 样例，再尽早做 T76 GPUI 评估/原型；这不是全面迁移已获批，也不把前三项当原型必须等待的技术门槛。T53/T49 及 T59→T69 可提前但不阻塞桌面路线验证。完整逐项就绪度、返工成本及依赖判断见 `docs/任务/GPUI前后-待办重估与排序-2026-10-04.md`。

T72 用户确认指“递归自我改进”，其具体改进对象/评价/授权和回退边界未定；含义写入评估文档，原任务正文未改。本轮仅调顺序，所有实施仍等用户触发。读取默认 list 带历史时曾遇 incomplete control message；用公开分页参数跳过历史页后待办读取成功，根因尚未确认，勿当队列不存在或重发写操作。

## 当前：UI 收尾已完成并部署（2026-10-04）

用户批准的两批 UI 和后续收尾均已落地。收尾宿主 `d98cc0a`、Diff 返工 `abd7e6c` 主控审查通过；独立集成 `35f2e2e`，main 合并 `efea116`，收尾 `bae7004`。当前安装 Saddle/Diff 为 `~/.local/share/saddle/versions/efea116`，Drover 注册仍为 `5f39f88`，Corral 不变。未重启运行中的宿主，正常重开加载。

- 新增收尾：Agents 窄栏名称优先，时间再状态文字退让，状态符号形状/行高/操作保留；插件可见截断按钮焦点修正；Diff 明确浅色 RGB 背景可读性适配，#909090 语法分类合并返工已关闭。
- 宿主目标检查实现者报告 2 passed；Diff 实现者 lib 6 passed，主控针对复核 1 passed；发布构建与安装回读通过。本轮未重跑全套/Clippy，旧基线失败继续单列。
- 两个收尾实现者与开发/集成 worktree、分支已按规清理，完成提醒已处理；重复提醒不重做。四个历史 worktree 原样保留。Tasks 队列与遥测未动。
- 保留：Reset 实际背景未知、极矮窗整行按钮不可见的公共布局边界、其余清单明确保留项。不存在当前必须返工项，不自动推进其他任务。
- 记录：`docs/任务/UI收尾-集成与部署-2026-10-04.md`；清单 `docs/调研/UI整理清单-2026-10-03.md` 已逐项更新。备份与验证日志：`/Users/firegnu/.local/share/saddle/backups/ui-final-20261004-140225`。
- 下一步：用户正常重开后查看本轮微调，有反馈再处理；没有反馈则本轮 UI 整理告一段落，等待用户触发下一项。

## 第二批历史完成记录（2026-10-04）

用户批准剩余 UI 整理及 Agents 限定方案，三组并行实现和主控审查均已完成。宿主 `29357c3`、Settings `8735ff8`、Diff 返工 `60f7bf1` 统一集成；34 列 Staged+error 必须改项已关闭。main 合并 `6189921`，收尾 `3b2f6b6`。

- 集成展示检查宿主 7、Settings 4、Diff 1 项通过；Diff 34 列专项复核另有 1 passed。源码独占范围和业务边界通过审查；Agents 3a、行高、入口顺序与行为保持。未重跑全套/Clippy；此前旧基线 12 项失败仍保留，不能宣称全套绿。
- 第二批当时安装包 `~/.local/share/saddle/versions/6189921`：Saddle 与 Diff 更新；Drover 注册仍为 `5f39f88`，Corral 入口不变。发布构建与安装回读通过，主配置 hash 未变；备份 `/Users/firegnu/.local/share/saddle/backups/ui-second-batch-20261004-132008`。未重启当前宿主或主控，正常重开 Saddle 加载本批。
- 实际绘制预览：`file:///Users/firegnu/Developer/personal_projs/saddle-ui-preview/2026-10-04-second-batch/index.html`，31 幅合成 Buffer。预览、原始帧与日志独立保存，不等同真实终端/手机 SSH 验收。
- 三个 ui2 开发 worktree/分支与 ui2-integration 已安全清理，对应三个实现者已随目录关闭；完成提醒全部处理，重复提醒忽略。四个历史 worktree 原样保留。
- 既有极窄 Agents 名称/Attention 截断、极窄按钮焦点线索属于建议观察项，没有扩展本批范围。后续依用户实际反馈继续小修，不自动启动其他任务。
- 详细记录：`docs/任务/UI第二批-集成与预览-2026-10-04.md`，分组与三份任务书保留所有完成/返工/审查记录。本批未推进 Tasks 队列，未开启遥测。

## 第一批与圆点调整的历史部署记录

- `main`：UI 合并 `5f39f88`，收尾空提交 `85a780b`；之后仅补部署与交接记录。Settings 原未提交候选已核对后安全提交 `0452406`，设计语言 HTML 已入库。
- 新安装包：Saddle 入口指向 `~/.local/share/saddle/versions/4ac4691`（更新圆点微调），Drover 注册仍为 `5f39f88`。Corral、Diff、其他插件和主配置未改；旧不可变包保留。
- **第一批部署当时的运行快照（非当前状态）**：当时核验宿主 PID 72000 仍加载 `a542931`，已有 Drover 进程仍加载 `711ab18`。正常关闭并重开 Saddle 即可加载新版，不需要退出主控或其他 agent；不要擅自杀进程。
- 第一批包含 Settings 家族、Drover、宿主弹窗和 Telemetry 展示整理。80×10 Add/Edit 正文被帮助行挤掉的问题已修正；workflow 旧文字定位已适配，业务断言保留。
- 预览：`file:///Users/firegnu/Developer/personal_projs/saddle-ui-preview/2026-10-04-first-batch/index.html`。独立文件未随 worktree 删除。实际 Ratatui 合成画面，不冒充真实终端或手机 SSH 验收。

## 验证与已知问题

- 完整标准套件原始结果 645 通过、21 失败、9 忽略；9 项 UI 文字定位已适配。明确重建宿主和 Drover 后完整 workflow **101 通过、0 失败、4 忽略**。Clippy `--all-targets -- -D warnings`、release 构建与 diff 检查通过。
- 剩余候选 agent_capture 3 项、drover_telemetry 9 项失败，全部在干净旧基线 `9f721d0` 复现，相关业务源码和测试未改；不宣称全套绿或旧故障已修复。不搭车扩展成遥测修复任务。
- 共用 target 的基线构建曾让后续 workflow 读到旧宿主产物；此次已明确重建集成宿主/Drover 后验证通过。后续跨 worktree 切换时核对实际产物，避免把缓存结果当候选运行证据。
- 部署备份、BUILD.txt、installed.json 与验证日志：`~/.local/share/saddle/backups/ui-first-batch-20261004-115727/`。详细证据见 `docs/任务/UI第一批-集成与预览-2026-10-04.md`。

## 清理与保留

- 第一批三个 UI 开发分支/worktree、集成与 detached 基线 worktree 均已安全移除；对应三个 `saddle/dev-ui-*-1` agent 随工作目录关闭。第一批清理时公开 corral ls 仅余主控 `saddle/main`，instance `0ed54c671118`；原宿主 instance `ab0d3e34a167ecbe` 保留。
- 旧 UI 完成提醒已处理；关闭 agent 后重复提醒不触发重做、重派或队列放行。
- 历史 `corral-live-upgrade-research`、`review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow` 四个 worktree 原样保留。

## 下一步

- 用户正常重开 Saddle 后回看本批实际界面，有反馈则继续对应 UI 小修。部署成功与运行窗口已更新应分别核验。
- Diff、Settings 剩余 H18/H19、公共状态栏/Toast/notice/终端和 Agents 公共绘制现已按上方第二批范围完成并部署；本批不包含额外产品能力或其他队列任务。
- 保持设计语言与整理清单两份文档。设计与批准取舍看 `docs/DESIGN.md`、`docs/UI设计语言.md` 和 `docs/调研/UI整理清单-2026-10-03.md`；分组和各组审查见 `docs/任务/UI并行-分组与派发-2026-10-04.md`。T74 导航改造、移动端能力不搭车，不推进 Tasks 队列。

## Settings 更新圆点微调（2026-10-04）

用户看到更新提示后反馈「这个小红点有点小。你自己调整一下吧」。本次主控直接改为加粗实心圆 `●`（原 `•`），仍单列、原色、原位置；更新检测与 Settings 命中区域不变。实现 `e23fd89`，合并 `b25fa21`，收尾 `4ac4691`，开发 worktree/分支已清理，无新委派。

现有 UI 检查 31 项通过（含 52/30/20 列的更新提示）、宿主 lib Clippy 与 release 构建通过；纯视觉小改不制造失败测试、不重跑刚完成的全套，旧基线失败状态沿用上文。部署 `4ac4691` 仅替换 Saddle，配置/注册表 hash 未变，入口和 `--help` 回读成功；备份与日志在 `~/.local/share/saddle/backups/update-dot-20261004-125029/`。本次未重启正在使用的宿主或任何 agent，正常重开 Saddle 后加载。
