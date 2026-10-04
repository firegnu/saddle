# Saddle 交接

更新：2026-10-04。当前分支 `main`；本次交接前 HEAD 为 `a3d5cfd`，已推送，工作区干净。本次只更新本文件，提交后继续等待用户操作，不自动推进队列。

## 会话摘要

用户断网后要求独自接手、不再委派；T67 已完成并部署。随后讨论并正式派发 T66，现已完成 UI 回归样例索引、运行入口与分层测试约定，合并、推送和清理均完成。

用户已确认本轮交付内容：T66 主要是文档及测试样例，未修改产品运行代码；验证时已编译相关测试和预览，**不需要发布编译、安装部署或重启 Saddle**。

## 完成的工作

- T66：实现 `77f5a4b`，合并 `de5e6b6`，清理后的收尾空提交 `1215793`，交接 `a3d5cfd`，均已推送。开发分支及 worktree `t66-ui-regression` 已安全删除，没有新建 agent。
- 新增 `docs/UI回归.md`：宿主、Settings、Agents、Drover、Diff 的已有场景/固定尺寸索引，四组 Cargo 命令、单例入口、预览和覆盖边界。AGENTS、README、DESIGN 的测试约定已同步。
- 只补一个 Diff 长路径/中文长代码绘制样例，复用同一 fixture 输出文本预览；既有测试、断言、并发参数和超时保留。
- 四组检查全部通过：宿主49、Settings51、Drover26、Diff9，共 **135 passed / 0 failed / 0 ignored**。自审加强路径可见断言后，仅复跑该单例，1 passed。
- Diff 定向 Clippy、格式/差异、文档链接及32个测试名检查通过。现有预览入口生成五份 SVG 与五份文本并核验，路径 `/tmp/saddle-ui-preview-t66/`。本轮未跑全量，无真实终端人工验收声明。
- 主控自审通过，无独立审查 agent。完整记录见 `docs/任务/T66-UI回归样例.md`；测试日志 `/tmp/saddle-t66-*.log`。

## 待完成与当前队列

- T66 实施暂无已知待完成工作。本次通过公开 Drover show 回读仍为 **Running**，run `b3db4bf30d9249bbfe7bb0e023a0322d`；回执 `/tmp/saddle-t66-handoff-status.json`。没有自动 Submit、Accept 或 Return，等待用户提交/验收或反馈。
- 任务正文末尾“仅更新待办，保持 Pending”是早先编辑阶段的旧说明；用户已明确“继续啊，这个任务我已经派发了”，不可再据此停止实施或退回 Pending。
- T76 GPUI 评估是此前讨论的下一项，尚未在本会话派发或实施；开始前以用户新指令和实时队列为准，不从旧排序推断授权。
- 既有时序测试问题没有搭车处理：T67 标准套件首轮673 passed / 14 failed / 9 ignored，14个失败项各一次限定复核通过；根因未定位，不能宣称默认并行全套稳定全绿。详见 T67 记录。T66 的135项通过不替代全量基线。

## 接手约束与现场

- 沿用用户“不再委派、独自完成”的安排，除非用户重新指定。设计和理由以 `docs/DESIGN.md` 为准；本文件只记录交接状态。
- 测试选择遵循 `AGENTS.md` 与 `docs/UI回归.md`：小改跑直接相关检查，公共 UI 改动跑相关组，跨模块行为变更/阶段集成/发布前跑全量；已通过且相关代码未改不重复全量。
- 共用 target：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。使用合成数据、临时目录和假 CLI，不接入或干扰用户 agent。
- 当前公开控制实例 `35766352fff8d4aa`，主控 `saddle/main` / Corral instance `9f8a73396d8a`。后续控制操作先重新确认实例；任务操作只走公开 `saddle ctl plugin` 和同实例 `saddle ctl request`，不能把 plugin_pending 当业务完成。
- 四个历史 worktree 保留：`corral-live-upgrade-research`、`review-telemetry-design`（detached）、`t38-dispatch-study`、`t55-notification-flow`。不擅自清理或关闭用户 agent。
- T67 安装入口本次核对仍指向 `~/.local/share/saddle/versions/d7da5b4/bin/saddle`；详细构建/部署记录见 T67 文档与 Git 历史 `678d399`。本次未重查运行宿主的加载路径，不沿用旧交接中的“待重开”判断。此后 T66 只有文档和测试变更，Source 领先 Installed 不表示有产品代码需要部署。

## T66 遥测续接

- trace `026876b7-e42a-47f0-a35f-5c0dac7a60ed`，controller handoff `edaf52b8-e5fe-42e2-b5ba-92fef070cdc0`，记录目录 `/tmp/saddle-t66-20261004/`。
- 自审记录分组 `5977a5e8-b822-4efc-9e96-5d2aaa0e0bdf`，passed event `7dc58f6d-6d48-45c4-92ee-6b9b526d06e3`；实际收尾声明 `bf87f9b3-0eb0-421c-ab84-0072a2061d9a` 已保存，记录实际合并、推送、清理及验证结果。
- 此 trace 绑定 T66 当前 run，主控不提前 close；由 Drover 在用户 Accept/Return 时关闭，Submit 不关闭。若有同任务反馈，先 show 核对绑定和关闭状态；不把这些 ID 用于 T76，也不因历史缺口补发业务。

## 优先阅读

- `AGENTS.md`、`docs/UI回归.md`：当前开发与验证入口。
- `docs/任务/T66-UI回归样例.md`：需求、范围、完成记录和自审。
- `docs/DESIGN.md`、`docs/UI设计语言.md`：设计依据。
- `docs/任务/T67-版本状态展示.md`、`docs/任务/T68-测试夹具修复.md`：前两项的实现与验证记录。
- `docs/任务/GPUI前后-待办重估与排序-2026-10-04.md`：后续任务评估；不是实施授权。
- `plugins/drover/README.md`：公开队列命令及状态边界。较早 UI 批次和部署细节可查 `docs/任务/` 与旧交接提交 `a3d5cfd`，不把旧状态当当前事实。

## 下一步

等待用户验收 T66 或提出反馈。若用户正式派发 T76，再读取任务正文并开展评估；无需为 T66 再编译部署或机械重跑全量。
