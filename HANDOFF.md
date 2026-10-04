# Saddle 交接

更新：2026-10-04。T67 已在原 Claude worktree 由主控独自接手完成、合并并清理；用户随后授权编译部署，已安装，待正常重开 Saddle 加载。T68 下方待放行描述是历史快照，不再占用队列；其余任务不自动推进。

## 当前：T67 已部署，待重开 Saddle 加载

- 用户断网恢复后明确要求独自完成、不再委派。主控直接接手 Claude 原 worktree，保留原候选，补来源路径校验并自审。实现 `85b84d0`，main 合并 `073ddf5`，清理后收尾空提交 `9fed67a`；功能/取舍/验证和自审记录在 `docs/任务/T67-版本状态展示.md`，设计已同步 `docs/DESIGN.md`。
- Updates 区分 Source、Installed、Running、Agents；构建记录须匹配程序 SHA-256，源码只认记录的绝对工作树根和分支。Diagnostics 显示完整构建/路径及缓存检查时间，未知如实展示；原小点、Upgrade all、回执核验及不重发语义保留。用户随后授权「编译+部署吧」，已部署下述宿主包；当前窗口仍加载旧程序，正常重开后生效。
- 有效 RED：相对 `.` 错读启动仓库；最小修复后目标 51 项通过。标准验证补齐全部 74 目标，首轮 **673 passed / 14 failed / 9 ignored**；14 个失败项各一次限定复核全通过（protocol 1，upgrade 4，agent_capture 6，plugins 2，workflow 1；后四组串行）。不宣称默认并行全套一次全绿，时序失败根因未定位。标准 Clippy `--all-targets -- -D warnings` 一次 exit 0；格式、差异、合成打包元数据核验通过。
- 日志与原候选备份 `/tmp/saddle-t67-takeover-20261004/`；首次接手检查 `/tmp/saddle-t67-target-inherited.log`。只用临时仓库、合成包、假公开命令及合成 Buffer，未做真实跨版本终端验收。
- 开发 worktree/分支 `t67-version-status` 已在干净且确认合并后安全移除；原 Claude `saddle/dev-t67-version-1` 已因中断退出，现场无此 agent，无需再 stop。四个历史 worktree 原样保留，公开 Corral 列表只余新主控 `saddle/main`（instance `9f8a73396d8a`）。
- T67 仍用原 run `eb8bde08924afc0223def93851fc0232`，未重派、Submit/Accept/Return 或启动下一项。主控只完成代码交付与审查报告，队列交由用户提交/验收。新宿主 instance `73227dc3db2cbe6d`，公开查询回执在接手日志目录。
- 同一 trace `27a60724-2bc1-40d0-8001-118f68b6bad2`：接手授权 `3149f30e-bd60-4384-9089-af3824f52e74`；主控自审记录分组 `f0903d97-12ef-450d-9f28-2e440a46df42` / passed event `7d44e3b3-4e13-47f9-83ac-7be06755ad86`。这是自审记录身份，没有新 agent 或独立审查。第一次 trace 级 review 因 scope 不符拒绝后改为 review 分组成功保存，仅纠正记录，未重做业务。原 Claude 未取得最终回复，不伪造回复或早期 RED。绑定 Tasks 的 trace 留待用户验收/退回关闭。
- 部署：干净 main `d7da5b4` 的宿主 release 构建通过，新包 `~/.local/share/saddle/versions/d7da5b4`，入口已原子切换；BUILD.txt 的 source/main/revision 与 Saddle SHA-256 核验通过，入口 `--help` 成功。Corral、Diff、Drover 产物沿用旧包，命令链接、插件注册和主配置不变。旧包保留，备份与安装回执在 `~/.local/share/saddle/backups/t67-deploy-20261004-164920/`。
- 运行回读：宿主 PID 86118 仍加载 `efea116/bin/saddle`，主控仍为原 instance `9f8a73396d8a`；未重启宿主或 agent。下一步正常关闭并重开 Saddle 后查看 Updates。此后的部署记录提交只改文档，版本页按提交数可能显示 Source 领先 Installed，不表示功能代码未部署。其余任务不自动放行或派发；本轮 trace 继续等用户验收结束。

## 当前：T68 修复完成，等待用户放行

- 夹具修复 `ee4ad30`，用户批准的 UI 测试类型别名 `d90bbaf`，最终主控复核 `7cbeb12`，main 合并 `75a1ee0`，收尾 `669ad6b`。任务记录 `docs/任务/T68-测试夹具修复.md`；先前诊断报告 `docs/调研/T68-测试失败根因与回归判断-2026-10-04.md` 保留原阶段证据与未知项。
- 修复仅测试侧：两组夹具复用小型 Rust 启动器，以 exec 调解释器，避免新脚本首次执行成本；保留产品预算和原业务/回执/超时/取消/回收断言，新增受控慢启动用例。UI 告警仅提取等价类型别名。产品源码、原公共夹具不改，无安装部署需要。
- 有效 RED：agent 1 项、Drover 9 项；修复后串行 agent 24/24、Drover 15/15。主控标准默认并行全套 74目标、683 passed / 0 failed / 9 ignored；类型别名后目标 UI 7/7、标准 Clippy `--all-targets -- -D warnings` exit 0，差异检查通过。等价别名后未重复全套。
- 实现者早期全套有 viewer 超时，限定复跑及主控全套均通过；保留偶发失败事实，不声称修复了它的未知根因。macOS 底层首次启动原因和 release 冷启动仍未外推。日志 `/tmp/saddle-t68-fixture-fix-20261004-fHUMoq/`、主控 `/tmp/saddle-t68-fix-review-20261004/`。
- 开发 worktree/分支 `t68-fixture-fix` 已安全清理，自建 `saddle/dev-t68-fix-1`（instance `d67e8bb598fb`）idle/attached=0 后随目录关闭。公开列表只留主控，四个历史 worktree 保留；重复完成提醒不重做。
- T68 公开最终核对仍 `awaiting_release`，run `62df4a09a0d3976795853fdec8651038`。本修复阶段没有 Accept/Return/重复 Submit/新派发；用户说修完后自行放行。
- 同一 trace `65382f38-6fc9-4ef5-a9a4-90dff5df9f54`：修复 implementation `2d27fa30-7231-4818-80f3-1bd3f7a01d40`，补充修正 `4aa0b847-7f10-4522-b5db-6540335b45e8`，最终 review `4b1a0fc4-9fe0-4322-af80-bce10602801a` / passed event `6d34a615-cddf-4131-b8fc-9a53794c6c27`。reply/审查已存；上下文与后续真实推送收尾记录在 `/tmp/saddle-t68-fix-20261004/`。trace 绑定 Tasks，留待用户 Accept 后由 Drover 关闭，不由主控提前 close。
- 修复初次遥测决策引用被拒后，启动入口配对 executed=false/unreadable_context；核对未创建 agent 后修正记录、另作有效启动。两次回执均保留，不冒充未知结果重发或完整无缺口历史。
- 前阶段诊断 `3951269` 已随 `ad41fb5` 合并，诊断开发目录和 agent 当时已清理。旧目录 `/tmp/saddle-t68-20261004/`、诊断实验 `/tmp/saddle-t68-diagnosis-20261004/` 保留，旧派发身份不用于后续独立任务。

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
