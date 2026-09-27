# Agents 侧栏 3a 微调

2026-09-27，saddle/main 交给 saddle/dev-agents-panel（Claude Code，常规档 opus[1m] / high，role=implementer）。
路由：常规 / 交叉审查不要 / 影响面：改行为（route.py：tier 常规、cross_review 不要、impact 改行为）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 40 节；必要时读第 23/25/28/32 节中本任务涉及的 effort、Git 字段、选中、Tasks 入口既有约束。
- `docs/设计稿/agents-panel-3a/Agents Panel Spec.md` 全文和同目录 `agents-panel-3a.png` 图。必须实际查看图片；Markdown 优先于图片，语义核对以第 40 节为准。
- src/ui.rs 的 Agents 绘制/Tasks 入口、src/agents.rs、src/app.rs 的 Agents 输入与 View 接线、src/theme.rs，以及直接相关 tests。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/agents-panel-3a；分支 agents-panel-3a，从已包含本任务及设计的 main 建立。
- 仅改 Agents 展示与交互所需 src/ui.rs、src/agents.rs、必要的 src/app.rs/theme.rs/config.rs 和直接相关 tests、现有帮助文案/README/配置示例以及本任务完成记录。确有必要可局部拆分 Agents 渲染文件，不做相邻重构。
- DESIGN 的产品选择和原始设计稿由主控维护，不修改。主控可能更新设计补充，应先读取当前第 40 节；遇到未决设计冲突报告，不自行改产品规则。

## 要做的
按两份设计稿与 DESIGN 第 40 节完成左侧 Agents UI 微调，包括规格中的排序、折叠、动画、窄宽降级和对应鼠标/键盘操作。保持来源字段真实、选中与本机 attached 独立、各条目对齐，原有 attach/New/Stop/Tasks 入口继续使用既有操作语义。布局变化导致的点击区域和滚动同步在本任务范围内。

重要已核对语义：C 实际表示领先基准，显示 ↑n，不是 ↓n/pull。ATT 是公开连接数；本机标记依据 saddle 自有连接。用户明确要求 effort 保留现在的位置不变：R1 的 agent 类型/窄窗图标之后、状态之前，保留两列信号格，折叠时也显示。不得移到展开信息行或隐藏，固定列为它留空间、名称弹性截断，第 23 节三档字形/颜色/未知规则保持。TUI 不能表达 2px 按字符格最小适配，图片图下 Tweaks 不扩成产品面板。

## 怎么算做完
用户原话：
> cool，现在还有一个任务。重新对左侧agents区域按照对应的设计，进行ui的微调。给你的设计是下面两个文件：
> /Users/firegnu/Desktop/export/Agents\ Panel\ Spec.md
> [Image #1]
> 这两个文件就是微调设计稿。

用户补充原话：
> 保留，放在现在的位置不变，这个很重要

用户提供的 Markdown 规格第 9 节原文：
- 50 列下，示例数据（corral / drover / saddle×2）无任何折行。
- 注入 `+12847 -3291 ?128`，diff 整组移到下一行右对齐。
- 切换选中到 `saddle/main`，只有该条高亮，同组另一条不受影响。
- 42 列下，agent 列显示为图标，各列仍对齐。
- 5 种状态的点、颜色、排序符合第 3 节。

验证预算：排序/折叠等行为变化按 AGENTS.md 做有意义的定向 RED→GREEN；纯视觉变化不造假 RED，使用现有 TestBackend/合成数据检查直接可见结果。`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，另 fmt/diff 检查。新失败或新改动只追加相关检查。保留既有输入路由与生命周期断言，可修正直接受布局影响的坐标/文字与同步，不靠固定 sleep/重试掩盖问题。无需独立交叉审查、录屏、真实现场操作、覆盖矩阵或缺陷植入。Cargo 命令始终设置 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不改 corral/drover 仓库、协议、内部数据或真实队列，不执行 drover next/done/go。只用公开 CLI 和既有字段。
- 不改 src/git.rs 的采集与安全隔离、不做新 Git 查询，不改任务调度/agent 生命周期/PTY 信号/ctl 协议；如只是本机连接显示需要只读状态接线，可以最小调整 View。
- 不操作用户正在用的 agent、TUI、shell 或现场 socket。测试使用假 CLI、合成数据/PTY 与隔离 runtime。不要按项目名或路径批量杀进程，只回收自有且记下 PID 的测试子进程。
- 不顺带改右侧工作区、Tasks 弹窗及其他界面主题，不新建调参页或持久化设置系统。
- 不安装 skill、不构建 release、不修改 main/其他 worktree，不合并、不推送。只在自己分支提交。

## 做完
在本文件末尾追加「## 完成记录」并提交，写改动、实际验证（区分 RED/GREEN 与纯视觉验证）、取舍、未做的事和待主控决定事项。回复新 SHA，命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

实现者 saddle/dev-agents-panel，分支 agents-panel-3a。

### 改动
- `src/agents.rs`：新增 `Status`（waiting/error/stalled/working/starting/unknown/idle/exited，声明顺序即组内默认顺序），`Panel::status` 统一供排序与显示使用；公开 `blocked` 显示为 waiting，`error`/incompatible 为 error，`exited` 仅在公开状态真实返回时出现。`by_state` 改为 `by_name`：默认按状态排序、`s` 切到名称排序，同状态按名称，选择跟随 agent 名称。新增 `fold: Option<bool>`、`folded()`（未手动选择时 >5 个自动折叠）、`toggle_fold()`（本次运行内手动选择优先，不持久化）。
- `src/ui.rs` Agents 渲染按规格重写：边框内头部 `Agents · N` + 右侧 `Tasks · <状态> Tab`（Tasks 入口改为无括号文本、状态色沿用公开队列快照，仍可点击/Tab，打开时高亮不可点），分隔线；分组标题 `name/ ─── (n)`，组间空一行，条目间不留空；每条独立 gutter（选中橄榄 `┃`、未选极暗 `│`，均在同一列，不位移），选中整条底色 `#2b2621`（gutter 外）。R1 固定列：点 / 名称（弹性，`…` 截断，不再另起一行全名）/ agent 8 列（<50 列降为 2 列标识）/ 状态 9 列（working 前加紫色 braille，120ms 一帧；圆点 360ms 一帧）/ 时间 5 列右对齐（`⦿`=本 saddle 任一 pane/tab 正显示，`•`=未读完成回合；时间超 10h 取整、超 100h 用天/年，保证 ≤4 列）。R2 标题（等于分组名隐藏，单行截断）、R3 `ASK/ERR/DOING … · 时长`（状态色，时长暗色，右侧截断）、R4 `⎇ 分支 ↑n 基准 … +a -d ?u`（按显示宽度判定，放不下整组移到 R4b 右对齐，diff 内不折行；分支同名只显示 `⎇`）、R5 路径（末段等于组名只显示父目录并以 `/` 结尾；否则末两级；超宽先丢前面层级再从左截断保留末尾）、R6 `hash · ATT n · VIA via`（ATT>0 绿）。底栏贴底、上有分隔线：`↵ Attach`/`[Attached]`、`n New`、`s Sort`/`s Name`、`x Stop`（整组红，Stopping 时禁用）、`z Fold`/`z Expand`；按键字母橄榄粗体，点击区域随绘制结果生成，放不下时先缩间距再换行。滚动条放在右内边距列，滚动/跟随、命中行、滚动上下文底边标题照旧。
- effort：原两格信号柱原样保留（形状、配色、选中不变）。首版曾移到 R6 行右端，按用户后续补充改回 R1 原位置（见下方「用户补充修正」）。窄宽下没有标识的未知 agent 类型在 R6 前补类型名；pi/omp 仍用 `π` + 各自颜色区分。
- `src/app.rs`：Agents 焦点 `z` 切换折叠；View 新增 `local`（所有 tab/pane 中持有会话且正在显示的 agent 名），只读接线，不改 viewer/terminals。状态栏帮助加 `z Fold`。
- `src/theme.rs`/`config.toml`：新增 Agents 专用配色 `agents_bg/border/rule/faint/text/branch/dim/dimmer/accent/green/red/blue/yellow/purple`（默认值即规格第 8 节）；`agent_selected`、`claude`、`codex` 默认改为规格值（这三项原本只用于 Agents）。共享 `agent_*` 未改，Tasks 弹窗与右侧终端配色不变；stalled/starting 状态与 effort 图标沿用共享色。旧配置照常加载。
- README（中英）：Agents 描述、Git 行格式、配色说明、按键表（s/z/x）、示意图。

### 实际验证
- RED→GREEN（行为）：
  - `tests/agents.rs` 新增 `default_order_puts_agents_needing_people_first_and_s_switches_to_names`、`fold_defaults_on_above_five_agents_until_toggled`，并把原状态排序用例改为默认排序。先加最小桩（字段/方法存在但旧行为），三项按预期失败（顺序与折叠断言），实现后通过。
  - `tests/workflow.rs` 新增 `fold_toggles_by_key_and_bar_and_background_tab_agents_keep_the_local_mark`（z 键、点击底栏 `z Expand`、另一 tab 中显示的 p/a 仍有 `⦿`、只有公开 ATT 的 p/taken 没有）。临时把 `local` 改回仅活动 pane（旧行为）时在 `⦿` 等待处失败，恢复后通过。
- 纯视觉（TestBackend 合成数据，直接检查可见结果，不造 RED）：`tests/ui.rs` 新增规格第 9 节对应用例——50 列样例四个 agent 逐行对照设计且无折行/无意外截断；注入 `+12847 -3291 ?128` 后 diff 整组移到下一行右对齐；选中 saddle/main 只有该条整行底色与橄榄 gutter；42 列 agent 列为 `✳`/`>_` 且点/类型/状态/时间列跨组对齐；折叠；状态点/颜色/标签/R3 活动（五种 + stalled/starting/unknown）；braille 与圆点帧率。另用 `examples/ui_preview` 渲染合成数据目视核对。
- 受布局直接影响而修正的既有断言：Agents 坐标/文字（头部行、`‹Tasks›`→`Tasks · `、底栏文案、首条目行 3→5、树线→gutter、路径规则、Git 行格式、配色常量、宽字符续格背景）；workflow harness 启动后按 `s`+`kk` 切到名称序并选中首项，使原有「Enter 接 p/a」的输入路由/生命周期断言保持原意；app 冒烟测试等首帧同时出现头部与 Viewer 再断言（首帧增大后可能分多次读到）。输入路由与生命周期断言未删减。
- `cargo test --all-targets` 一次：除 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases`（第 2971 行等待 New agent 表单关闭超时）外全部通过；该用例单独连跑 3 次通过，workflow 全套再跑一次 58 passed / 0 failed。未定位根因，归入既有的偶发 workflow 观察，不称全套一次全绿。其后 clippy 修两处 lint，只复跑 `--test ui --test agents`。
- `cargo clippy --all-targets -- -D warnings` 通过；`cargo fmt --check`、`git diff --check` 通过。

### 取舍
- 头部放在边框内（按截图），边框保留用于焦点表达；Agents 区整体铺规格背景色。
- 规格 C/↓ 语义按 DESIGN 第 40 节：显示 `↑n 基准`，0 极暗、>0 黄；未知 `↑—`，不伪造零；binary 作为 `N binary` 放在 diff 组内。
- waiting 没有公开的待确认摘要，R3 如实显示 `ASK waiting for input`，不借 last_tool 编造。
- 排序按钮默认显示 `s Sort`（照设计稿），名称序时显示 `s Name`；折叠按钮显示下一步动作 `z Fold`/`z Expand`。`[Attached]` 在选中项已在活动 pane 显示时出现并置灰（不可点），否则为 `↵ Attach`。
- 未读标记沿用原语义，改为时间列内的 `•`（原 `new` 放不进 5 列）。
- TUI 无法表达 2px 内边距/选中 2px 竖线，按字符格处理，不加空行。

### 未做
- 未新增调参面板/持久化设置；未改 Git 采集、生命周期、PTY、ctl 协议、Tasks 弹窗与右侧工作区；未做 release、合并、推送。

### 待主控决定
- Agents 颜色做成了可配置的 `agents_*` 键（沿用 T1 配色可配置的做法）；若主控希望不暴露这些键，可改为常量。
- `s` 按钮文案（默认 `Sort`、名称序 `Name`）与 `[Attached]` 的「激活」解读请确认。

### 用户补充修正：effort 保留首行原位置
用户明确：「保留，放在现在的位置不变，这个很重要」（主控转达，见主仓库 c399688 的 DESIGN 第 40 节与本任务文件修正；本分支未 cherry-pick 该文档提交）。
- 改动：`src/ui.rs` R1 在 agent 类型列（窄窗为 2 列标识）之后、状态列之前放回两列信号格加 1 列间距；列表中有已知 effort 时所有条目都预留（未知留空对齐），全都未知时不占列；名称列让出 3 列继续弹性截断（50 列 19→16、42 列 17→14），类型/状态/时间固定列不变。折叠时首行照常显示。R6 恢复为纯 `hash · ATT n · VIA via`。第 23 节三档字形 `⣄⡀`/`⣴⡀`/`⣴⡇`、取色与未知语义未改。README 中英文同步。
- RED→GREEN：从基线 fc7e131 原样恢复 `delegated_effort_shows_strength_bars_and_unknown_stays_blank`、`selecting_an_agent_keeps_its_effort_icon_tier`（后者补断言图标确为 `⣴`，否则两边都是空格也会通过），新增 `effort_keeps_its_first_row_slot_when_folded_and_narrow`（7 个 agent 自动折叠，50/42 列下每个首行在固定列显示对应档位或空白、状态紧随其后、长名被截断；全无 effort 时不留列）。改代码前三项均因首行无图标失败，改后通过。
- 其后 `cargo test --all-targets` 一次：除 workflow `t20_r1_pending_new_pane_keeps_known_source_cwd_for_shell`（第 2892 行 ctl close 未返回 confirmation）外全部通过；该用例单独连跑 3 次通过。workflow 全套又跑 6 次：4 次全过，1 次同一用例失败，1 次 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases`（第 2971 行）失败；失败现场是「+ → New agent… → Cancel Esc」后新标签页的内容选择框仍开着。为判断是否本任务引入，把基线 fc7e131 用 `git archive` 导出到临时目录（未建 worktree、未动 main）跑 workflow 5 次：3 次失败，失败的正是上述两条（同在 2892/2971 行）以及 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal`。结论：基线已有的偶发问题，与 Agents 渲染改动无关，根因未定位，未修复，不称全套一次全绿。排查时临时给断言加了输出，已还原。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。

### 主控审查返工（依据主仓库 docs/任务/Agents侧栏3a主控审查.md，只读）
只处理审查指出的三项规格偏差；其余取舍已由主控裁定保留，未改。
1. **R5 路径放得下时完整显示。** `agent_path` 非同组目录从完整路径开始，只有超宽才先省去前面层级（`…/` 开头）再从左截字符、保留末段；末段等于组名时沿用截图的「父目录末级 + `/`」写法。不改公开 cwd、不动 Git 查询。
2. **256 色终端取最近值。** `theme::truecolor(COLORTERM)` 仅在 `truecolor`/`24bit` 时视为真彩色；否则启动时 `Theme::for_terminal(false)` 把 Agents 专用色（`agents_*`、`agent_selected`、`claude`/`codex`/`pi`/`omp`）中的 RGB 换成 xterm 256 色中最近的一格（6×6×6 立方与灰阶取距离更近者）。真彩色环境保持原 RGB；与 Tasks 等共用的颜色、用户写的 ANSI 颜色名不变。无新依赖、无新设置。workflow harness 显式设 `COLORTERM=truecolor`，原有 RGB 颜色断言不再依赖运行者环境。README 中英、`config.toml` 注释同步。
3. **`[Attached]` 已连接时用正文色，仍不可点。** 底栏控件新增 `lit`：已连接显示正文色，但不生成 Enter 点击目标；未连接的 `↵ Attach` 规则不变，不改会话/焦点语义。为让测试直接检查点击目标，`ui::Hits.buttons` 由 `pub(crate)` 改为 `pub`。

验证（只跑直接相关检查）：
- RED→GREEN（新的颜色能力分支）：`tests/layout_config.rs::agents_palette_takes_nearest_256_colors_unless_the_terminal_announces_truecolor`（COLORTERM 判定、已知色对应 234/143/167/253、Agents 专用色全部变成索引色、共用色和 ANSI 名保持）以及 `tests/app.rs::agents_colors_follow_the_terminals_announced_color_depth`（隔离运行 saddle 二进制：`COLORTERM=truecolor` 时首帧有 `48;2;29;26;22`；去掉 COLORTERM 时只有 `48;5;234`）。先用保持旧行为的桩函数跑，两条均在能力判定处失败；实现后通过。
- 纯视觉/命中（不造 RED）：`tests/ui.rs` 新增 `paths_show_in_full_when_they_fit_and_lose_leading_levels_only_when_too_wide`（50/42 列：`/tmp/team/project` 完整；长路径按宽度省层级；同组目录显示 `…/team/`）与 `attached_reads_in_text_color_but_offers_no_second_attach_click`（已连接时 `[Attached]` 为正文色且没有 Enter 点击目标；未连接时 `↵ Attach` 可点）。
- `cargo test --test ui --test layout_config --test app` 通过；workflow 只跑受影响的 `startup_colors…`、`buttons_require_release…`、`agents_reply_entry…`、`fold_toggles…`、`native_mouse_buttons…`、`stop_in_progress…`，均通过。未重复全套和基线多轮实验。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。


## 主控审查

2026-09-27：实现 cb36c3e，effort 原位修正 c3c2346，三处规格修正 8872c5c；主控最终结论可以合并，完整意见见《Agents侧栏3a主控审查.md》。effort 保留首行类型后、状态前，折叠仍显示；路径、256 色与 Attached 三项必须改已关闭，无剩余必须改，无独立交叉审查。

主控先前已跑标准检查通过（workflow 58 passed、2 ignored），本次仅定向复核 UI 39、配置8、workflow6项通过；app 启动检查通过，颜色检查在 NO_COLOR=1 环境先失败，清空该变量后通过。新增颜色测试未自行隔离 NO_COLOR 记为不阻塞建议；旧 picker 偶发失败基线亦可复现，根因未定位，未扩大范围。开发本提交 Clippy/fmt 通过。实现取舍已逐条裁定并写入审查和 DESIGN 第40节。

合并提交 2c884e7；不推进 drover/T20，发布和清理结果见下方。
