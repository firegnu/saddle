# Tasks 入口设计

2026-10-04，T74 设计阶段。**状态：提案，待用户确认。** 未实施，不修改已批准的 [DESIGN](DESIGN.md)；若获批，实施时先把选定的规则改动写入 DESIGN 再改代码。

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

管理页现有操作为 Open panel / Disable / Restart / Add local… / Remove / Refresh / Back（`src/plugins/ui.rs`）。在 Restart 之后加一个按选中插件切换的操作（示意）：

```
 Open panel  Disable  Restart  Pin to header  Add local…  Remove  Refresh  Back
                               ↑ 已固定时为 Unpin；固定另一个插件会替换原固定
```

固定状态存于宿主自己的插件登记文件 `plugins.toml`（文件级一个 `pinned = "<plugin id>"`）。旧版 Saddle 读取时忽略此字段，但若旧版改写该文件会丢掉固定，需要重新固定；不影响插件本身。

### 2.2 从某个 repo 打开对应 Tasks 的操作路径

| 起点 | 操作 | 结果 |
| --- | --- | --- |
| Agents 列表选中 repo X 的 agent | 点头部 `Tasks`，或按 Agents 焦点下的固定入口键（见 §4 取舍 2） | Tasks 覆盖界面打开并显示 X 的项目 |
| 正在 Viewer 里看 repo X 的 agent 窗格 | 鼠标点头部 `Tasks`（焦点仍在该窗格，来源取窗格） | 同上 |
| 同上，但用键盘 | Ctrl-] 回 Agents → 按键 | 来源变为 Agents 选中项；Ctrl-] 返回时选中项通常就是刚看的 agent，但不保证 |
| Tasks 已在工作区 tab/split 打开 | 同上 | 按现有规则聚焦已有视图，不新开 |

即：选中/聚焦 X 的 agent → 一次点击或一个键。项目对不上时，仍可在 Tasks 内按 `c` 换项目。

### 2.3 目标项目如何确定

宿主只提供来源 cwd 快照，不读任务或项目登记；项目判断全部留在 Drover。

| 来源 | 推荐的结果 | 与现状的差别 |
| --- | --- | --- |
| cwd 在已登记项目的主目录 | 该项目 | 无 |
| cwd 在已登记项目的某个 worktree | 同一 git common dir 的登记项目，优先 toplevel 相同者 | 无 |
| 同一仓库登记了多个项目（如主目录和某 worktree 各登记一次），且没有 toplevel 相同者 | 按登记顺序取第一个 | 无；这类歧义罕见，Tasks 顶部的项目名可见，`c` 可改 |
| 已登记但还没有任何任务 | **仍切到该项目**（显示空队列） | 现在因“没有任务”不切换，会留在上一个项目，看起来像打开错了 repo；属 Drover 内部改动 |
| 不是 git 仓库或未登记 | 保留当前项目，并在 Tasks 顶部显示一行提示：`Opened from <path> · not a Tasks project · a Add project`（示意） | 现在静默保留上一个项目；属 Drover 内部改动 |
| 普通终端窗格 | 传该窗格的启动目录（`source_cwd`） | 现在传 `None`；宿主小改，协议字段不变。限制：这是启动目录，shell 中 `cd` 后不跟随（见 §4 取舍 3） |
| Agents 焦点没有选中 agent | 不传 cwd，Drover 用当前/初始项目 | 无 |
| 从 Attention 或通知打开 | 用插件目标，不被 cwd 覆盖 | 无 |

Agents 列表按名字前缀分组（`saddle/` 等），组不等于 repo（同组 agent 可能在不同 worktree 或仓库），因此不以分组作为项目依据。

### 2.4 插件不可用时

入口在固定期间始终占同一位置，状态不同只改变表现与点击结果：

| 插件状态 | 头部表现 | 点击/按键 |
| --- | --- | --- |
| Running 且有界面 | 正文色，悬停提亮 | 打开或切换到 Tasks |
| Starting / Stopping / Restarting / Disabled / Failed / Unresponsive / Unavailable | 次要文字色，不提亮 | 打开 Plugins 面板并预选该插件，由已有的说明行显示原因和 Manage 入口；不隐式启用、启动或重启 |
| 已从登记中移除 | 不显示，固定随之清除 | — |

Settings、确认框、New agent 等编辑界面打开时，沿用 Plugins 入口的忙碌规则，不打开、不丢草稿。

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

- 宿主：`plugins.toml` 固定字段读写；管理页 Pin/Unpin；Agents 头部绘制与点击区域（与 Telemetry 同组换行）；不可用时打开 Plugins 面板并预选；（取舍 2）Agents 焦点按键与底栏提示；（取舍 3）终端窗格来源 cwd。
- Drover：已登记的空项目也切换；来源未匹配时显示提示行。
- 协议、SDK、插件清单格式：不变。没有发现必须新增的接口。
- 验证建议：相关 UI 回归组（Agents 头部宽/窄、管理页）、Drover 项目匹配的单元测试（空项目、未登记、worktree）。
