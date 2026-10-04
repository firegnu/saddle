# Tasks 入口设计

2026-10-04，T74。**状态：用户已批准（§4 四项均按推荐），实施及修订审查通过，已合并 main 并安装；重开 Saddle 加载新版。** 规则改动已写入 [DESIGN](DESIGN.md)「2026-10-04 Tasks 直接入口」。以下为批准时的设计正文；实施中的两处具体选择见文末“实施说明”。

用户原话：现在的每一个repo的tasks入口有点麻烦，需要先打开plugins，之后定位到tasks插件，然后才能打开任务。这个有点深。要重新设计一下。

## 1. 现状（以当前代码为准）

| 环节 | 现在的行为 | 依据 |
| --- | --- | --- |
| 打开 Plugins | 只能鼠标点 Agents 头部的 `Plugins`；Agents 焦点没有对应按键，底栏帮助也没有 | `src/app_plugins.rs` `plugin_launcher_event`；`src/app.rs` `panel_key` |
| 定位 Tasks | 面板先列内置插件（Dispatch），再按插件 ID 排序；默认选中第一行，即 Dispatch。Drover 行显示清单 action 标题 `Tasks`，需要 ↓ 或输入过滤后再 Enter/点 Open | `src/plugins/mod.rs` `palette_items`；`src/plugins/palette.rs` `update`；`plugins/drover/plugin.toml` |
| 打开时的项目 | 普通打开通过 `view.context.v1` 传来源 cwd：Agents 焦点用选中 agent 的 cwd，Viewer 焦点用当前 agent 窗格的 cwd；普通终端窗格不传（`None`） | `src/app_plugins.rs` `open_plugin_view_with_context`；`src/app.rs` `focused_agent_cwd` |
| 项目匹配 | Drover 用 `git rev-parse --show-toplevel --git-common-dir` 把来源 cwd 对应到已登记项目：同一 common dir 视为同仓库，优先 toplevel 相同者，否则取登记顺序第一个；只有该项目已有任务才切换；仅在列表页、无输入、非忙时切换 | `plugins/drover/src/drover.rs` `RepoTasks`；`plugins/drover/src/plugin.rs` `Event::Opened` |
| Attention/通知打开 | 带插件自己的目标，不用 cwd 覆盖 | DESIGN「Drover 插件迁移」 |

所以从某个 repo 到它的 Tasks，现在是：点 `Plugins` → 在列表中找到 `Tasks`（↓/输入/点击）→ Enter/Open → 若自动匹配没生效，再按 `c` 进项目选择器换项目。纯键盘无法打开。

## 2. 推荐方案：用户固定的头部入口 + 打开时按来源定项目

### 2.1 入口位置

在 Settings → Plugins（管理页）中，用户可把**一个**已登记插件的打开动作固定（Pin）到 Agents 头部。入口文字取该插件清单的 action 标题（Drover 为 `Tasks`），放在 Telemetry 同一行左侧；Plugins 和 Settings 位置不变。

正常宽度：

```
 Agents · 5              Tasks  Telemetry
 Attention · 2          Plugins  Settings
 ─────────────────────────────────────────
 saddle/main          ● Working
 ...
```

窄侧栏沿用现有换行兜底：`Tasks` 与 Telemetry 作为一组，同组一起换到下方并右对齐，不与标题、Attention 或列表重叠；绘制与点击使用同一矩形。没有固定插件时头部与现在完全相同。

最小退让：固定入口的标题在头部最多占 12 列，超出以 `…` 截断；连换行后都放不下时，先隐藏固定入口，Telemetry、Plugins、Settings 按现状保留。隐藏时仍可用按键（取舍 2）或 Plugins 面板打开。

可固定的范围：只有清单声明了可打开视图的插件（有 `panel.v1`，面板中会出现 Open/Switch 动作的那类）才能 Pin。内置 Dispatch 和只在后台运行的插件没有打开动作，Pin 按钮禁用并说明 `No view to open`，不会被当成 Tasks 式入口。

首次设置：没有默认固定，安装或升级后头部不会自动出现 `Tasks`。用户需要先做一次 Plugins → Manage plugins → 选中 Drover → Pin；在此之前访问路径与现在相同。

管理页现有操作为 Open panel / Disable / Restart / Add local… / Remove / Refresh / Back（`src/plugins/ui.rs`）。在 Restart 之后加一个按选中插件切换的操作（示意）：

```
 Open panel  Disable  Restart  Pin  Add local…  Remove  Refresh  Back
                               ↑ 已固定时为 Unpin；固定另一个插件会替换原固定
```

固定状态存于宿主自己的插件登记文件 `plugins.toml`（文件级一个 `pinned = "<plugin id>"`）。旧版 Saddle 读取时忽略此字段，但若旧版改写该文件会丢掉固定，需要重新固定；不影响插件本身。

### 2.2 从某个 repo 打开对应 Tasks 的操作路径

| 起点 | 操作 | 结果 |
| --- | --- | --- |
| Agents 列表选中 repo X 的 agent | 点头部 `Tasks`，或按 Agents 焦点下的固定入口键（见 §4 取舍 2） | Tasks 打开，顶部显示来源 X；Tasks 停在列表页且未在操作时，定位完成后切到 X 的项目（§2.5） |
| 正在 Viewer 里看 repo X 的 agent 窗格 | 鼠标点头部 `Tasks`（焦点仍在该窗格，来源取窗格） | 同上 |
| 同上，但用键盘 | Ctrl-] 回 Agents → 按键 | **来源是 Agents 列表中高亮的选中项，不是刚才看的窗格**。两者不同（例如用鼠标切过窗格）时会打开选中项的 repo；Tasks 顶部的来源行会显示实际用到的路径 |
| Tasks 已在工作区 tab/split 打开 | 同上 | 按现有规则聚焦已有视图，不新开 |

即：选中/聚焦 X 的 agent → 一次点击或一个键，前提是 Tasks 上次停在列表页、没有草稿或进行中的操作（否则保留现场，见 §2.5）。项目对不上时，仍可在 Tasks 内按 `c` 进 Projects 换项目。

推荐流程的提示文字：Agents 底栏写 `p Tasks (selected)`，让键盘用户知道用的是列表选中项。

### 2.3 目标项目如何确定

宿主只提供来源 cwd 快照，不读任务或项目登记；项目判断全部留在 Drover。

| 来源 | 推荐的结果 | 与现状的差别 |
| --- | --- | --- |
| cwd 在已登记项目的主目录 | 该项目 | 无 |
| cwd 在已登记项目的某个 worktree | 同一 git common dir 的登记项目，优先 toplevel 相同者 | 无 |
| 同一仓库登记了多个项目（如主目录和某 worktree 各登记一次），且没有 toplevel 相同者 | 按登记顺序取第一个 | 无；这类歧义罕见，Tasks 顶部的项目名可见，`c` 可改 |
| 已登记但还没有任何任务 | **仍切到该项目**（显示空队列） | 现在因“没有任务”不切换，会留在上一个项目，看起来像打开错了 repo；属 Drover 内部改动 |
| 匹配到已登记项目，但读取其队列失败 | 仍切到该项目，由列表页现有的读取错误显示原因，不显示成空队列 | 现在读取失败也返回 `None`，与“没有任务”混在一起；改动后定位只判断仓库，读取结果交给正常加载 |
| 确认没有匹配：来源和全部已登记项目的 git 查询都成功，但没有同仓库者 | 保留当前项目，来源行提示 `No added project matches <path> · c Projects → a Add project`（示意） | 现在静默保留 |
| 无法判断：来源不是 git 仓库、git 失败或超时、某个已登记项目查询失败 | 保留当前项目，来源行提示 `Couldn't match <path>: <reason> · showing <当前项目>`；不说“未接入” | 现在静默保留 |
| 普通终端窗格 | 传该窗格的启动目录（`source_cwd`） | 现在传 `None`；宿主小改，协议字段不变。限制：这是启动目录，shell 中 `cd` 后不跟随（见 §4 取舍 3） |
| Agents 焦点没有选中 agent | 不传 cwd，Drover 用当前/初始项目 | 无 |
| 从 Attention 或通知打开 | 用插件目标，不被 cwd 覆盖 | 无 |

Agents 列表按名字前缀分组（`saddle/` 等），组不等于 repo（同组 agent 可能在不同 worktree 或仓库），因此不以分组作为项目依据。

区分上表各行的方法只在 Drover 内部：把 `RepoTasks` 的结果从 `Option<String>` 改成插件内部的三种结果——匹配到项目、确认无匹配、无法判断（带原因）。`git::repository` 现在把非仓库、执行失败和超时都合并为 `None`，所以“不是 git 仓库”也归入“无法判断”，不单独声称未接入。宿主协议、`view.context.v1` 字段和 Saddle 代码都不变。

“Add project” 走现有路径：列表页按 `c` 进 Projects，再按 `a`（列表页的 `a` 是 Add task，不复用）。Projects 中 `a` 预填的是当前项目路径，用户需改成来源路径；本提案不新增专用动作，也不改预填。

### 2.4 插件不可用时

入口在固定期间始终占同一位置，状态不同只改变表现与点击结果：

| 插件状态 | 头部表现 | 点击/按键 |
| --- | --- | --- |
| Running 且有界面 | 正文色，悬停提亮 | 打开或切换到 Tasks |
| Starting / Stopping / Restarting / Disabled / Failed / Unresponsive / Unavailable | 次要文字色，不提亮 | 打开 Plugins 面板并预选该插件，由已有的说明行显示原因和 Manage 入口；不隐式启用、启动或重启 |
| 已从登记中移除 | 不显示，固定随之清除 | — |

Settings、确认框、New agent 等编辑界面打开时，沿用 Plugins 入口的忙碌规则，不打开、不丢草稿。

### 2.5 保留现场与异步定位

直达入口不覆盖 Tasks 里已有的工作。现状（`plugins/drover/src/plugin.rs`）：关闭视图时只清掉 detail 和 confirmation，项目接入表单（setup）和当前页面（Add/Edit/Delete/Projects/All pending 等）都保留；`Event::Opened` 只在无 setup、非 busy、处于 List 页时启动定位；定位结果回来时还要求 `input_revision` 未变，期间送达插件的按键、粘贴或鼠标输入会作废结果（实施时排除单纯鼠标移动，见文末）。

| 重新打开时 Tasks 的状态 | 行为 |
| --- | --- |
| List 页，无 setup，不在执行操作 | 启动定位。来源行显示 `From <path> · locating…`，项目名仍是当前项目，直到结果回来才切换 |
| 有 setup 表单、Add/Edit/Delete 等草稿、确认页、通知偏好（`N`），或其他非 List 页 | 不定位、不切换、不丢草稿（通知偏好现在不在 `Event::Opened` 的检查条件里，实施时补上）。来源行显示 `From <path> · kept your current page · project: <当前项目>` |
| 正在执行操作（busy，如派发、保存） | 不定位，不取消操作。来源行同上，提示仍是 `<当前项目>` |
| 定位中用户开始操作（`input_revision` 变化） | 结果作废，不切换。来源行改为 `From <path> · not applied · project: <当前项目>` |
| 定位中关闭视图，或从别的来源再次打开 | 关闭时丢弃；再次打开用新来源，旧结果不落到新的打开上 |

约束：

- 不为直达覆盖草稿、不自动取消正在执行的操作，也不把作废的结果留到下一次打开时再静默切换。下一次打开按那时的来源重新定位。
- 只要来源行没有显示已切换，Tasks 显示的就是标出的当前项目；用户不会把旧项目当成 X。来源行在用户切换项目、关闭视图或下一次打开时更新。
- git 查询每次最多 5 秒，已登记项目较多时定位可能需要数秒，期间 `locating…` 一直可见。
- 来源行和这些状态都在 Drover 画面内实现，不需要宿主新能力。

## 3. 改变的已批准规则

已批准（DESIGN「插件统一命令面板」）：插件不贡献常驻按钮，只保留一个固定 Plugins 入口。

建议改为：**插件仍不能自行增加主界面控件；用户可在管理页把一个插件的打开动作固定到 Agents 头部。** Plugins 入口和面板保持不变，作为其他插件的入口和异常说明处。

理由：原规则针对的是插件自行占用宿主外壳；固定由用户决定，宿主控制位置、数量和样式，插件清单无需新字段。Tasks 是日常高频入口，必须经过“打开列表再定位”正是用户指出的问题。

## 4. 需要用户确定的取舍

1. **是否接受 §3 的规则改动。** 不接受时的替代见 §5 的“方案 B”。
2. **键盘入口。** 推荐 Agents 焦点下用 `p` 打开固定入口（底栏显示 `p Tasks`，键名不随插件变化）；也可以不加按键，只用鼠标。终端焦点下按键都送终端，键盘路径都要先 Ctrl-]。
3. **普通终端窗格是否传启动目录作为来源。** 推荐传：在某 repo 开 shell 后直接点 Tasks 就能对上；代价是 `cd` 到别的 repo 后仍按启动目录匹配。不传则保持现状（终端窗格不跟随）。
4. **固定数量。** 推荐只允许 1 个：头部宽度有限，当前需求只有 Tasks。以后要固定多个，等 T76 决定前端后再定。

## 5. 备选方案

| 方案 | 做法 | 优点 | 代价 | 判断 |
| --- | --- | --- | --- | --- |
| A（推荐） | 用户固定的头部入口 + 按来源定项目 | 一次点击或一个键直达；不加协议；宿主仍不知道“任务” | 改一条已批准规则；头部多一个词 | TUI 过渡与长期语义都适用 |
| B：只改 Plugins 面板 | 给 Plugins 加 Agents 焦点按键；面板预选上次打开的插件 | 不改已批准规则 | 仍是“打开 Plugins → 确认”两步，用户指出的层级没有去掉 | 用户不接受规则改动时的退路 |
| C：每个 repo 一条入口 | 侧栏按 repo 列出项目，每个下面有 Tasks | 字面上最贴近“每个 repo 的入口” | 宿主需要插件持续发布项目列表，要新增类似 attention.v1 的入口快照能力；TUI 侧栏空间不够；与 T76 侧栏/项目组织高度重叠 | 不在 TUI 做；交给 T76 一起考虑 |

## 6. 与 T76 / T63 的分界

可复用到任何前端的访问语义：

- 用户（不是插件）决定哪个插件动作常驻；宿主只用清单已有的 action 标题。
- 打开时传来源 cwd 快照，由插件决定对应项目；Attention/通知目标优先于 cwd。
- 入口在插件不可用时保持位置，点击后解释原因，不隐式改变插件生命周期。

仅属于当前 TUI 的最小过渡：头部文字入口与换行规则、Agents 焦点按键、管理页 Pin 开关、终端窗格的启动目录来源。不重排 Agents 头部或侧栏，不做整体导航重构。

不吸收 T63：不增加通用搜索或操作目录；T63 以后可把固定入口作为其操作目录中的一项。不涉及 T76 的桌面实现；方案 C 留给 T76。

## 7. 实施范围（获批后，供拆任务参考）

- 宿主：`plugins.toml` 固定字段读写；管理页 Pin/Unpin（仅限有可打开视图的插件）；Agents 头部绘制与点击区域（与 Telemetry 同组换行，标题截断，放不下时先隐藏固定入口）；不可用时打开 Plugins 面板并预选；（取舍 2）Agents 焦点按键与底栏提示；（取舍 3）终端窗格来源 cwd。
- Drover：定位结果改为插件内部三态（匹配/确认无匹配/无法判断），已登记的空项目和读取失败的项目也切换；来源行显示 locating、已切换、未匹配、无法判断、保留现场、结果作废等状态；通知偏好打开时不定位。
- 协议、SDK、插件清单格式：不变。没有发现必须新增的接口。
- 验证建议：相关 UI 回归组（Agents 头部宽/窄、管理页）、Drover 项目匹配的单元测试（空项目、读取失败、确认无匹配、git 失败/超时、worktree）和保留现场测试（草稿、busy、定位中输入、关闭后重开）。

## 实施说明（2026-10-04）

- 管理页按钮写作 `Pin` / `Unpin`，不用 `Pin to header`：48 列窄窗下长名称会让按钮栏多换一行并挤掉详情；“固定到 Agents 头部”由详情行 `Pinned to the Agents header.` 和操作结果说明。
- 定位期间，按键、粘贴、点击、拖动、滚动作废结果；单纯鼠标移动（悬停）不算操作。宿主会把覆盖界面内的移动也转给插件，若移动也作废，点头部入口后把鼠标移进 Tasks 就会让直达失效。
- 来源行位置：列表页画在工具栏下的分隔线上（此时不画列表/详情分隔的 `┬`，与 Running action… 相同）；其他页面替换顶部的项目路径行；Projects 页在表头上方加一行；项目接入与通知偏好画在覆盖区第一行，被居中对话框遮住时不显示。
- 第一轮返工后来源行的实际格式为 `Showing <实际项目>[ · <结果>] · from <来源>`（上文 §2.3/§2.5 表中的 `From <path> · …` 为示意）。实际项目和是否切换放在最前；按实际宽度先保它们，其次结果原因，最后来源路径（剩余不足 8 列时省略来源）。项目接入与通知偏好在覆盖区第一行保留来源行，对话框画在其下，不会被覆盖。
- 匹配到已登记且有 `.drover.conf` 的项目时直接打开它的列表，即使队列读不出，错误由列表的 `Read failed` 显示；未登记或缺配置的目录仍走接入检查。只有列表确实显示目标项目后才记为已切换，否则显示 not applied。
