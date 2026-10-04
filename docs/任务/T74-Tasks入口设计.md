# T74：各 repo 的 Tasks 入口设计

2026-10-04，saddle/main 委派 Claude Code，常规档 opus[1m] / high。
路由：常规 / 交叉审查不要 / 影响面看得见（路由 tier.verdict=null，档位拿不准；主控按限定范围的设计文档选常规）。
类型：设计
依据：T74 已正式派发，先确定具体入口形式，本阶段不实施。
提示：围绕已知约束给出推荐及关键取舍；有实质备选时再比较，不凑方案数量。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户要求

用户原话：现在的每一个repo的tasks入口有点麻烦，需要先打开plugins，之后定位到tasks插件，然后才能打开任务。这个有点深。要重新设计一下。

目标：重新设计各 repo 的任务入口，让访问对应项目 Tasks 的路径更直接，减少打开 Plugins 后再定位 Tasks 插件的操作层级。具体入口形式留待设计时确定。

原待办中的“仅加入待办，不设计、不开发或派发”是登记阶段说明，本次正式派发已触发设计。用户最新明确恢复 agent 委派，主控仍为 saddle/main。

## 先读

- AGENTS.md；docs/DESIGN.md §62（尤其“插件统一命令面板”）及“Agents 顶部入口两行布局”“Tasks 项目接入界面”。
- docs/UI设计语言.md；docs/插件入口与界面接入设计.md；plugins/drover/README.md 的界面和项目接入说明。
- docs/任务/GPUI前后-待办重估与排序-2026-10-04.md 中 T74/T63/T76 的范围。
- 按需阅读 src/ui.rs、src/agents.rs、src/app.rs、src/plugins/ 以及 plugins/drover/ 的现有导航、来源 cwd 和项目选择逻辑。以实际代码为准，不把旧方案当当前界面。

## 工作位置与产出

worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t74-tasks-entry-design，分支 t74-tasks-entry-design。
只写本任务文件及 docs/Tasks入口设计.md；后者明确标注“提案，待用户确认”，不直接修改已批准的 DESIGN。

基于现有界面提出一个推荐方案，用简短文字线框说明入口位置，以及从某个 repo 打开对应 Tasks 的实际操作路径。说明目标项目如何确定、已有多 repo / worktree 场景是否有歧义、插件不可用时入口怎样表达。指出需用户选择的关键取舍；若确有实质备选再比较，不堆选项。

主控设计约束：保留宿主通用插件边界，任务核心和项目数据仍由 Drover 拥有，不为直接入口让宿主读任务业务文件。此前已批准“插件不贡献常驻按钮、只保留固定 Plugins 入口”，本任务正重新讨论访问路径；建议改变此规则时明确列出改变及理由，不暗中当作已批准。考虑 T76 尚待 GPUI 评估，区分能复用的访问语义与当前 TUI 最小过渡，避免默认先做完整导航重构。沿用产品英文，不把中文讨论当汉化授权；不吸收 T63 通用搜索或 T76 桌面实现。

## 本阶段验收与验证

用户目标原文：重新设计各 repo 的任务入口，让访问对应项目 Tasks 的路径更直接，减少打开 Plugins 后再定位 Tasks 插件的操作层级。具体入口形式留待设计时确定。

验证预算：文档差异检查、核对方案引用的现有入口/代码依据；不跑测试、编译、截图矩阵或真实 agent 操作。只提交设计候选，不宣称功能已实现。没有额外产品验收点。

## 不要做

- 不改产品或测试代码，不操作真实队列/配置，不部署、不重启，不合并 main、不推送。
- 不对用户 agent 发消息、按键或停止；不用私人 Corral 状态文件。
- 不按项目名或路径批量杀进程。
- 不自行扩展依赖或协议；如有必要的接口缺口，只说明最小缺口与影响交主控审查。

## 完成时

在本文件追加完成记录：产出、核对、关键取舍和未做事项；仅在本分支提交。回复简要说明推荐方案及需用户确定的点。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

遥测由主控处理：trace 0a8a12d6-8ea4-477a-b00c-1e1fe470919c，run 5e378db26877a85fefe4684199d17910；设计候选不是整项任务收尾。

## 完成记录（设计阶段，2026-10-04）

产出：`docs/Tasks入口设计.md`，标注“提案，待用户确认”；未改 DESIGN、产品或测试代码。

推荐：用户在 Settings → Plugins 管理页把一个插件的打开动作固定（Pin）到 Agents 头部，Drover 显示为 `Tasks`，与 Telemetry 同行；点击（或可选的 Agents 焦点按键）直接打开，来源 cwd 由 Drover 按 git common dir 对应到已登记项目，worktree 归到所属仓库。插件不可用时入口保留并弱化，点击打开 Plugins 面板预选该插件看原因，不隐式启用。协议、SDK、清单格式不变。

核对依据（只读代码）：Plugins 仅鼠标可开（`src/app_plugins.rs` plugin_launcher_event，`src/app.rs` panel_key 无对应键）；面板默认选中首行内置 Dispatch，Drover 行标题来自清单 action `Tasks`（`src/plugins/mod.rs` palette_items、`src/plugins/palette.rs`）；来源 cwd 只取 agent，普通终端为 None（`src/app.rs` focused_agent_cwd）；Drover 的 RepoTasks 匹配逻辑及“仅有任务才切换”（`plugins/drover/src/drover.rs`、`plugin.rs` Event::Opened）；登记文件结构可容纳新增字段（`src/plugins/registry.rs`）；管理页现有按钮（`src/plugins/ui.rs`）。

关键取舍（待用户确定）：①是否把已批准的“只保留固定 Plugins 入口”改为“插件不能自加控件，用户可固定一个”；②Agents 焦点按键 `p` 加不加；③普通终端窗格是否以启动目录作为来源；④固定数量限 1。备选：只改 Plugins 面板（不改规则但层级仍在）；每 repo 侧栏入口（需新增插件入口快照能力，交 T76）。

未做：未实施、未测试/编译/截图，未操作真实队列、配置或 agent，未合并 main、未推送；未吸收 T63/T76 范围。验证仅为 `git diff --check` 与上述代码核对。

## 复核修订记录（2026-10-04，依据 `T74-Tasks入口设计复核.md`）

仍只改 `docs/Tasks入口设计.md` 与本记录，保留单个顶部固定入口方向，未扩产品范围。

- 项目识别（复核 1）：不再根据 `None` 声称未接入。设计改为 Drover 内部把 `RepoTasks` 结果分成匹配 / 确认无匹配 / 无法判断（带原因）三种；git 失败、超时、非仓库及任一登记项目查询失败都算无法判断，只提示“Couldn't match … · showing <当前项目>”。定位只判断仓库，已登记空项目与队列读取失败都切换，失败由列表页已有读取错误展示，与空队列区分。Add project 改用现有路径 `c` → Projects → `a`，说明列表页 `a` 是 Add task、Projects 的 `a` 预填当前项目路径。公共协议不变。
- 保留现场（复核 2）：新增 §2.5，按 `Event::Opened` 的启动条件和 `input_revision` 作废规则列出 List 页、草稿/setup/确认/通知偏好/其他页、busy、定位中输入、关闭重开时的行为；来源行始终标出来源路径和当前实际项目，未切换时不让旧项目被当成 X；不覆盖草稿、不取消操作、不把作废结果留到下次静默切换。§2.2 结果列加了前提；Ctrl-] → `p` 明确使用 Agents 高亮选中项，底栏提示 `p Tasks (selected)`。
- 建议项：Pin 只对有可打开视图的插件可用，内置 Dispatch/后台插件禁用并说明；标题最多 12 列截断，放不下时先隐藏固定入口、不挤掉其他入口；没有默认固定，需先在管理页 Pin 一次。

核对：`plugins/drover/src/plugin.rs`（Event::Opened 条件、lookup 的 input_revision 校验、Closed 只清 detail/confirmation、preferences 不在检查条件内）、`plugins/drover/src/drover.rs`（RepoTasks）、`plugins/drover/src/git.rs`（失败/超时均为 None，5 秒超时）、`plugins/drover/src/queue.rs`（列表 `a`=Add task，`c`=Projects，Projects 内 `a` 预填当前项目）。`git diff --check` 通过；未测试、编译，未改 DESIGN 或产品代码，未合并推送，未操作任务状态。
