# Saddle 交接

更新：2026-10-04。当前分支 `main`。当前 T63 已审查、合并并清理；主控仍为 saddle/main。T74 已完成部署和重启验证，下方 T66/T74 为此前完成记录。

## 当前：T63 已合并并清理，待用户验收/部署指令

- 用户批准轻量统一搜索，并指定 Codex；实现 `2a36dbd`，主控审查记录 `aac20cb`，合并 `40bdf5c`，清理后空提交 `e5940dc`。本交接提交随后推送 origin/main，以实际推送回执为准。
- 入口仍为 Agents 焦点 `/` 或现有 Search 点击；查 Agent 名、已有 Agent 的 cwd 末级项目名、插件标题/ID及 General/Colors/Advanced/Diagnostics/Plugins/Updates 六页。项目搜索返回 Agent，没有 Drover 登记项目或单条任务查询。
- Tasks 复用普通插件入口，不要求 Pin；未安装无条目，停用/失败/无响应显示状态并转管理，正常时打开/切换原视图。刷新与激活均检查可用性，不自动启停。T74 Pin、Plugins/Settings及终端透传保持。
- 主控按任务书审查，无阻塞项；路由不要独立交叉审查，未另派。集中全量一次 **702 passed / 1 failed / 9 ignored**；唯一失败 `native_mouse_buttons_cover_forms_and_stop_confirmation:851` 限定复核一次通过。未修改断言/超时/并行参数，不能称首次全量全绿或根因已修复。全量 Clippy、fmt、diff通过，日志 `/tmp/saddle-t63-integration/`。
- 合并后核对产品代码与受检候选一致，差异仅主控文档。原 worktree/分支 `t63-unified-search` 已安全删除；原 Codex `saddle/dev-t63-design-1` / `bd7efad2d99a` 在 idle、attached=0、clean和已合并核对后随工作目录删除一并关闭。公开列表只剩主控；四个历史 worktree 保留。
- 本轮没有 release 编译、安装部署、Submit/Accept、关闭 task trace或派发下一项；已安装产品仍为 T74 `29c94d4`。需要本功能在用户当前 Saddle 生效，仍须用户触发部署。
- T63 trace `16b9ad30-24fa-46c1-b07b-f3ea48e8656f`，run `1aceb478636a305c942d7b155e57bc79`，controller_handoff `86502084-b6b3-4153-aa91-75756da5eaaa`；绑定Tasks，不由主控关闭。任务状态未操作，当前业务状态需按公开接口查询。
- 实施dispatch `c337173f-6d84-40f8-b8ca-4656c120b640`，主控审查dispatch `f7bb95d9-f4fb-41a8-91eb-c9400f163f93`；公开reply operation `2bb6ecaa-f52f-452b-9f11-b0f2c057693a`，静态审查 `b788668c-12c5-482e-bbc2-2f4166d150b8`，集成审查记录预备ID `40fd53c0-560d-40bf-9b1e-f31103d1f5e9`（以append回执为准）。记录目录 `/tmp/saddle-t63-20261004/`；实施提醒 `80909340-c726-40a5-bf36-e433293901e2` 已处理，迟到提醒不重复执行。

## 上一项：T74 已合并、安装并重启验证

- 用户已批准同一 T74 实施。实现 `e6edaca`，两项返工 `439c5e3`；原独立 Codex 定向复核 R1/R2 通过、阻塞0，主控阶段集成完成。合并提交 `a62eecf` 已推送；本节随收尾更新。
- 入口：Settings → Plugins 给 Tasks 设置 Pin 后，Agents 头部出现 Tasks，Agents 焦点按 p 可打开；Viewer 按键透传。普通终端使用启动 cwd。停用保留入口并解释不可用，移除清除固定。没有默认自动固定。
- R1 已修：保留草稿时优先显示实际项目和未切换状态，接入/通知偏好表单不覆盖提示。R2 已修：已登记项目队列读取失败仍切到该项目列表显示 Read failed，不误入接入表单。详见设计和实施记录。
- 标准全量命令只跑一次，在 workflow 失败后停止：555 passed / 2 failed / 9 ignored。只补跑未执行的插件目标142 passed、宿主examples退出0；首次覆盖合计 **697 passed / 2 failed / 9 ignored**。两项失败各限定复核一次均通过，未改代码/断言/超时。全量Clippy、fmt、diff检查通过。
- 两项失败为 `mouse_wheel_scrolls_queue_history_immediately_and_reaches_both_ends:1257` 与 `native_mouse_buttons_cover_forms_and_stop_confirmation:851`。不能称默认并行全套稳定全绿；后一项与实现者旧基线报告相同，但主控未重跑旧基线确认归因。没有搭车修复时序测试。完整日志 `/tmp/saddle-t74-integration/`。
- 主控确认合并后代码与受检候选一致，差异只有主控文档。两个 T74 worktree 和分支已安全清理；原 Claude `saddle/dev-t74-design-1` / `0f81bd0e76e9`、Codex `saddle/dev-t74-review-1` / `80d99546d75c` 在 idle、未attach、clean及祖先检查后随工作目录删除一并关闭。公开 corral ls 只剩本主控，四个历史worktree保留。
- 用户随后明确“编译+部署”。已从干净 main `29c94d4` release 构建并安装 Saddle + Drover 到 `~/.local/share/saddle/versions/29c94d4`，入口及 Drover 登记目录已切换。config、其他插件、启用状态及 Corral 命令链接保持；旧包与备份保留。没有改 Tasks 状态、Submit/Accept 或关闭 trace。
- trace `0a8a12d6-8ea4-477a-b00c-1e1fe470919c` 继续开放，绑定 T74 同run；不由主控关闭。记录目录 `/tmp/saddle-t74-20261004/`；原审查/返工/复核提醒均已处理，后续迟到提醒不重复执行。
- 最后复核 dispatch `91cb9789-6afd-4c81-b95f-109baece7230`，主控passed事件 `aceb863e-63a3-4d94-b7aa-61eedea39304`。审查报告 `docs/任务/T74-Tasks入口独立审查.md` 保存各阶段真实证据和集成判断；源阶段任务书已随合并保留。
- 用户指出入口任务耗时过长，后续坚持缩小范围、避免重复检查；本次未追加新需求。T76等下一项未获本轮实施授权。

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

- 用户最新已恢复委派；主控负责拆分、派发、审查、合并和收尾。设计和理由以 `docs/DESIGN.md` 为准；本文件只记录交接状态。
- 测试选择遵循 `AGENTS.md` 与 `docs/UI回归.md`：小改跑直接相关检查，公共 UI 改动跑相关组，跨模块行为变更/阶段集成/发布前跑全量；已通过且相关代码未改不重复全量。
- 共用 target：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。使用合成数据、临时目录和假 CLI，不接入或干扰用户 agent。
- T66 结束时公开控制实例 `35766352fff8d4aa`（已被上方 T74 新实例取代），主控 `saddle/main` / Corral instance `9f8a73396d8a`。后续控制操作先重新确认实例；任务操作只走公开 `saddle ctl plugin` 和同实例 `saddle ctl request`，不能把 plugin_pending 当业务完成。
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

等待用户验收、部署指令或同任务反馈。T63已完成主控审查和一次集中集成，不重复全量；不自动推进Tasks状态、关闭trace或派发下一项。
