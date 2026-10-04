# 第二批 UI：Settings内页

2026-10-04，saddle/main 交给新建 saddle/dev-ui2-settings（Claude Code，常规 opus[1m] / high）；实际名称/instance 以公开回执为准。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：档位、影响面拿不准；主控按纯绘制定常规/看得见；交叉审查不要）。
类型：样式／文案调整
依据：用户认可上一批后接受剩余任务及 Agents 小幅整理方案，明确要求可并行就直接委派实施。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md；主控分派、审查、合并由主控负责，你不执行。
- docs/UI设计语言.md 第 2–3 节。
- docs/调研/UI整理清单-2026-10-03.md 第 4 节：H18、H19/H20 与 X6 的 Plugins 错误配色剩余项；保留第一批 H16/H17 外壳与字段帮助。
- docs/DESIGN.md 对应组件的已批准规则与末尾第一批 UI 记录。第一批已落地，不能重做或回退。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/saddle-worktrees/ui2-settings`，分支 `ui2-settings`，产品基线 `7643ac0`。
- 只改 `src/settings.rs`、`src/plugins/ui.rs`、`src/plugins/palette.rs` 的绘制与显示文案，以及独占检查文件 `tests/ui_second_settings.rs`。 以及本任务书。
- 三组独占文件；不改其他组源码/测试，也不改 DESIGN、设计语言、整理清单、HANDOFF。需要记录的具体取舍写完成记录，主控统一回写。

## 要做的
- 整理 Diagnostics/Updates 内部报告的标签列、正文/状态/技术信息层次；不同页面主操作按用途保留，不把 Refresh 权重机械统一。
- Plugins 管理页与 Add local 的焦点、主操作、错误表达清楚；沿用已有错误角色，内部 F(i) 命中编号不能显示成不存在的快捷键。
- 启动面板保留获批布局和提示行；启用与打开语义保持区别，不复制管理页整套说明。
- 保留六页外壳、草稿/校验/保存/冲突、复制报告、更新检测/升级、插件安装/启停生命周期。不要因新增对齐裁掉原报告或失败原因。

## 怎么算做完
用户验收原话：
> 按照设计语言走以及不破坏任何逻辑代码

本轮启动原话：
> 好的接受你的建议，开始做吧，你依旧看看是否并行委派多个agents去做上面的任务。如果可以，那就直接开干

验证预算（看得见）：`git diff --check`，再选一条直接展示检查命令（可以包含相关绘制场景）；仅合成/临时数据。纯视觉不制造失败测试；若修正明确的显示缺陷，在同一针对性检查中保留修正前后观察。不要跑全套/Clippy/真实 agent，不扩大覆盖范围。已过时的跨组工作流文字断言只报告位置，不擅自改共享 tests/workflow.rs 或削弱业务断言。

本批 cargo 一律通过 `python3 /tmp/saddle-ui2-cargo.py <cargo 参数>` 前台运行；脚本对整个命令加锁并设置项目共享 CARGO_TARGET_DIR，避免并行 worktree 相互覆盖产物。不要绕过锁，不另开 target、不杀别人的编译进程。若脚本不存在报告，不自行改用别的编译目录。

## 不要做
- 不改业务状态、持久化、数据、权限、输入/PTY、按键映射、默认焦点、Tab 顺序、危险确认、异步生命周期、查询/分页/升级语义。移动已有绘制时仅同步相同动作的命中几何，不换动作或可用条件。
- 不修改公共 theme/buttons、插件 SDK/协议、Corral、Drover、Updates/Diagnostics 数据采集和实际动作；不增加公共组件框架。宿主组若需局部共用状态栏绘制可以放自己独占文件，不改其他组调用接口/签名。
- 不新增能力/键位，不将 Unknown/Stale/失败改成成功，不隐藏证据。保留 Settings 加粗实心更新圆点与显示条件。
- 不读取真实私人数据，不操作用户 agent、任务队列、遥测，不安装/部署，不开启未选择的记录。不使用 stop/send/keys 操作其他实例。
- 不按项目名/路径批量杀进程；只停自己记录 PID 的测试进程。
- 第一批候选采集/遥测 12 项失败已在旧基线复现，背景见集成记录；不搭车修复或宣称全套绿。
- 不合并、不推送，只在本分支提交。范围外的共用缺口、行为缺陷或重大视觉取舍停在该点报告，继续独立可做项。

## 做完
在本文件末尾追加完成记录：改了什么、直接检查结果、具体视觉取舍、未做或保留项。记录实际证据，不用源码检查冒充真实终端观感。回复提交号及有无主控待定项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（saddle/dev-ui2-settings，2026-10-04）

### 改了什么
- `src/settings.rs`：Diagnostics 与 Updates 改用同一套报告排法（`report_lines`）：两格缩进 + 16 列弱化标签列 + 值列，值按词换行回到值列下；标签超出标签列时独占一行、值从下一行值列开始，不截断标签或值。节标题用 bright + 粗体（Updates 原用 focus 色），节前空一行；Updates 第一条 Heading 视为页面状态行（正文色粗体，后空一行）。值的状态色不变（Good 正文、Action unread、Bad danger、Unknown muted）。Diagnostics 主操作仍是 Refresh，Updates 仍是 Upgrade all；按钮、按键、滚动、复制摘要、数据来源未改。
- `src/plugins/ui.rs`（管理页 / Add local）：
  - 焦点与主操作分开：原来 Tab 焦点直接画成 primary；现在焦点 = 焦点色 + 粗体下划线标签（禁用按钮保持 dim 但仍带下划线，焦点落在禁用项上也能看见）。管理页按所选插件操作，不设单一主操作；Add local 未读到 manifest 时 Read manifest 为主操作，读到后 Add disabled 为主操作。
  - 内部 `F(i)` 只用于命中编号：这些按钮没有键，原公共按钮把标签最后一个词按“键位”弱化（`Open panel` 的 `panel`、`Add local…` 的 `local…`、`Read manifest` 的 `manifest` 看起来像快捷键）；现对本页标签统一着色，不显示任何 F 编号。
  - 错误角色（X6）：管理页消息原用 focus 色；现在注册表错误与操作失败用 danger，Disabled/Added 等结果提示用 muted（与 Settings 消息一致）。Add local 的读取/添加失败用 danger。新增仅用于展示的 `error` 标记，随 message 一起设置。
  - 列表选中改为 底色 + 粗体 + `›` 标记，正文色；`›` 在列表有焦点时为焦点色，焦点移到按钮后变 muted，选中底色保留。On 列 `!`（冲突）与 Runtime 的 Failed/Unresponsive/Unavailable 用 danger。详情标题由 focus 色改为 bright 粗体；宿主自己写入详情的问题行（Resources unavailable、not installable、ID 冲突）用 danger。
  - Add local：字段标题保留 “Plugin directory”，占位改为示例 `/path/to/plugin`（原占位重复字段名）；manifest 名称行加粗、`Program:` 标签弱化、权限警告保持正文色、“Adding does not start the plugin.”/入门提示弱化。
- `src/plugins/palette.rs`（启动面板）：布局、提示行、默认选择、动作判断不变。打开类动作（Open/Switch/Move）保持焦点色，内置插件的 Manage 改为正文色，以区别“打开视图”与“进入管理”；Disabled 仍显示 `—` 与启用指引。选中失败类插件时其原因行用 danger（原为 muted 帮助色）。底部 Manage plugins/Close 的焦点加粗下划线，不只靠颜色。
- `tests/ui_second_settings.rs`：4 个合成场景展示检查（Updates/Diagnostics 同列与长标签、管理页焦点/主操作/错误与提示、Add local、启动面板）。

### 直接检查结果
- `git diff --check`：无输出。
- `python3 /tmp/saddle-ui2-cargo.py test --test ui_second_settings`：
  - 修正前（只加检查、未改源码）：4 个全部失败，且都落在目标缺陷上——`local… looks like a key`（Reset vs Gray）、启动面板 `managing is not opening`（Yellow == Yellow）、Add local 字段名出现 2 次、Updates 仍为 `Saddle: Current` 无标签列。
  - 修正后：`test result: ok. 4 passed; 0 failed`，编译无警告。另修过两处检查自身的查找错误（提示行也含 “Read manifest”；列表名被截为 `Synthetic built-…`），不涉及源码。
- 画面来自 TestBackend 缓冲区和合成数据（120×50 / 110×40 / 100×30 / 80×24），只在默认主题下核对颜色；不代表真实终端、其他主题或手机 SSH 观感。

### 具体视觉取舍
- 标签列固定为 Diagnostics 原有的 16 列，两页完全同列；较长的 agent 名（如 `saddle/dev-…`）在 Updates 中会占两行（名一行、值一行），换取同列和不截断。
- Diagnostics 值的换行由逐字符改为按词（与 Updates 原有方式一致），超长单词仍按字符拆分。
- 焦点用下划线 + 粗体作为非颜色线索；未改公共 `buttons.rs`，焦点位置用本文件内按标签查找已绘制按钮的方式定位（标签被极窄宽度截断时不画焦点线索，按钮本身照常）。
- Updates 状态行是否需要按“需处理/全部生效”上色未做：状态行文字本身已说明，且不在本组改 `updates.rs` 的数据结构。

### 未做 / 保留项 / 待主控
- 未跑全套与 Clippy（按验证预算）。静态核对：`tests/updates.rs`、`tests/diagnostics.rs`、`tests/plugin_resources.rs`、`tests/plugins.rs`、`tests/workflow.rs` 中涉及这些页面的文字断言（“Changes apply immediately.”、“Search plugins”、“Open panel”、“Read manifest”、“corral recover p/held”、“checking…” 等）文本均未改动；未发现需要改共享测试的断言，但未实际运行，请主控在集成时跑相关测试确认。
- 管理页焦点落在“列表”时，列表本身只有 `›` 颜色变化作为焦点线索，未加边框（保留获批布局）。
- 公共按钮把“标签最后一个词”当键位着色的规则在其他无键按钮处可能有同类表现（本组只处理自己三文件）；是否在公共层修正由主控决定。
