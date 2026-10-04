# 任务：Telemetry界面按既有设计语言细调

2026-10-04，saddle/main 交给新建 saddle/dev-ui-reading（Claude Code，常规：opus[1m] / high）。实际名字和 instance 以公开 start 回执为准。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：档位拿不准，主控选常规；交叉审查不要；影响面看得见）。
类型：样式／文案调整
依据：用户认可设计语言 HTML 后要求逐界面调整，随后明确授权评估可并行项并委派；本轮只做展示整理。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md；其中主控分派、合并与收尾由主控负责，你不执行。
- docs/UI设计语言.md 第 2–4 节。
- docs/调研/UI整理清单-2026-10-03.md 第 2 节、第 4 节 H21、第 5/7 节。
- 只读视觉参考：/Users/firegnu/Developer/personal_projs/saddle/docs/UI设计语言.html。用户认可其基本方向，不表示其中全部尺寸已定稿。
- docs/DESIGN.md 中与你实际改动相关的现有批准条目。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/ui-telemetry-polish；分支 `ui-telemetry-polish`。
- 只改：`src/telemetry_view.rs` 的绘制、展示文案和必要的命中区域几何，以及本任务文件。
- 如确需增加或调整直接展示检查，只能放入你独占的 `tests/ui_reading_polish.rs`（Drover 则为 `plugins/drover/tests/ui_drover_polish.rs`）；纯视觉调整不要求制造 RED。
- 所有 cargo 命令使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。共享编译目录有锁时正常等待，不另开 target、不杀进程。
- 同时有其他组改不同 UI 文件；不改它们的文件。主目录保留未提交 Settings 候选，不读取为运行基线、不覆盖、不替主控提交。你的产品代码基线为 `9f721d0`，任务书提交只增加文档。

## 要做的
- 在原有列表、详情和正文阅读层级中整理列对齐、标签和值、技术元数据和正文的信息层次。
- 摘要更易读，重复提示与强调减少；原始正文、技术标识、来源和错误仍能通过原有入口完整读取。
- 已有操作提示准确，空、加载、失败、未知和陈旧语义保持；不把截断或缺内容写成成功。
- 不改分页/查询/筛选/异步请求/正文加载/返回链路；不改宿主公共状态栏。

共同约定：产品界面沿用英文；用户正文、报告和原因保持原文。同类组件统一，焦点、选中、主操作分清；沿用既有主题角色、按钮组件和显示列宽工具。不扩主题协议、不新增共享框架，不改任何第三方插件契约。

## 怎么算做完
用户验收原话：
> 按照设计语言走以及不破坏任何逻辑代码

验证预算（看得见）：`git diff --check`，再选一条能直接看出本次结果的现有绘制测试或合成绘制检查。只做这些；不跑全套、不跑真实 agent/真实用户数据，不强制截图或另建证据目录。觉得预算不够先报告，不自行扩大。

## 不要做
- 不改状态机、运行身份、权限、数据模型、持久化、查询/分页、异步生命周期、任务派发、agent 生命周期、按键映射、默认焦点、Tab 顺序或危险确认。移动绘制时同步原操作的点击区域，不能换动作或可用条件。
- 冻结公共 `src/theme.rs`、`src/buttons.rs`、`src/ui.rs`、`src/app*.rs`、`src/settings.rs`、`src/plugins/*`，以及 Drover 的 theme/buttons 与 SDK/协议。你的允许范围与此冲突时，以本任务列出的具体 UI 文件为准，仍只改展示部分。
- 不改主设计、设计语言、整理清单或其他人的任务书；确需记录的设计取舍放在本任务完成记录，由主控统一回写。跨文件公共组件缺口、业务缺陷或尚未批准的大改停在该点报告，继续其余独立可做项。
- 不触碰 Tasks 队列，不开启遥测；不安装、不部署、不升级、不操作任何真实 agent，不修改用户配置和真实数据库。
- 不按项目名或路径批量杀进程；只能停止自己启动且记下 PID 的进程。
- 全套测试背景：主控此前全套停在 agent_capture，4 项失败且根因未核实；不可称全套绿，不搭车修复。
- 不合并到 main，不推送。只在 `ui-telemetry-polish` 分支提交允许范围。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复只写这些，加上有没有要主控决定的事和提交号。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

**做了什么**（只改 `src/telemetry_view.rs` 绘制/文案，另加 `tests/ui_reading_polish.rs`）
- 列表：时间与类型列定宽对齐（Task/Ad hoc 同列，类型文字 inert 转义）；时间、类型、绑定尾部弱化，标签为正文；选中行改为“▸ 标记 + 选中底色 + 字重”，不再用焦点色。底部选中摘要改成对齐的 `Label / Scope / Coverage` 标签列（标签弱化），`coverage_start` 字段名改为 `Coverage  from …`，“trace recording on/paused/ended”原文保留。
- 详情：时间线表头按行的实际列宽生成，修正 Source 与 Bodies / gaps 错位；seq/时间/来源弱化，事件名为正文；选中事件同上用底色+字重，选正文时保留该事件 ▸ 标记（身份不丢）、不加底色。“Now (queried …)”查询时间弱化，Dispatch 行去掉与底栏重复的 “◂ Tab ▸”，当前 dispatch 用底色+字重。详情面板按行分层：事件标题加粗，时间/哈希/链接/字节数等技术元数据弱化，读取失败（预览、目标 trace）用 danger，正文与状态文字照旧。
- 按钮：只有面板主操作（Read full body / Choose body / Read selected body）保留强调；Technical details 和列表顶部 Project/Type/Search 改为普通按钮。区域、键位、可用条件未变。
- 提示：同页回车统一写 `Enter`；去掉与面板按钮重复的底栏“↵ Read full body”、计数后的“· r Refresh”、各处行内“r Retry/r Refresh”，改为底栏在失败时把 r 标成 Retry（同一动作）；`n Next page` 仅在确有下一页可读时出现；o/i 在已打开对应面板时标为 “Event detail”（按键本就切回）；去掉 ‹Operations› 等多余括号，“Bodies 1 ↵”去掉逐行 ↵。
- 正文阅读层：sha/字节数弱化，“hash verified · 编码”保持正文；行号计数改弱化色。Recording intervals 行把 on/off 放前面对齐。

**验证了什么**
- `git diff --check` 通过。
- 现有绘制测试 `cargo test --test telemetry_view`：18 项全过（空、加载、失败、未初始化、陈旧上界、截断/缺正文、转义、窄/宽布局、鼠标点击主按钮均在内）。
- 新增合成绘制检查 `tests/ui_reading_polish.rs`（1 项，通过）：表头与行列对齐、选中行底色+字重、未选中无底色、主/次按钮字重区分、元数据弱化。
- 未跑全套与 clippy（按预算）；只对改动的两个文件跑了 rustfmt。

**拿主意的地方**
- “late · Late submission …”的重复字样保留：现有测试以小写 late 为断言，改它需动别人的测试文件。
- 选中统一用 `agent_selected` 底色 + 粗体（列表、时间线、选择器、body 选择、当前 dispatch），焦点色只留给主操作与底栏键名；四套主题下观感未实际渲染确认。
- 宿主状态栏文案 `status()` 未动。

**没做的事**
- 筛选表单（kind/scope/key/run）布局未调；真实终端截图与主题对比度未验证；未改主设计/设计语言文档。

## 主控审查

- 2026-10-04，审查提交 `1f68d2b`；公开 status 为 idle、attached=0，reply 以 DONE 结束，分支工作区干净。结论：代码审查通过，未发现必须返工项；本轮按提醒停在审查，未合并、推送或安装。
- 范围只有 `src/telemetry_view.rs` 展示、独占检查文件和本任务书；未修改查询/分页、异步读取、输入处理、持久化或冻结公共文件。列对齐、元数据弱化、选中标记/底色/字重和主次按钮符合设计语言，未发现逻辑越界。
- 主控核对 diff 与 `git diff --check` 通过；实现者报告现有 Telemetry 18 项和新增展示检查 1 项通过，主控按看得见预算未重跑套件。针对删除重复读全文提示的疑点，用临时合成数据对比基线与候选在 80×9 的实际 Ratatui buffer，两者面板读全文按钮均可见，未复现入口丢失；记录在 `/tmp/saddle-telemetry-review/{before,after}.log`。不据此声称全部尺寸或全套测试通过。
- 本页选中样式和 Enter 文案可接受，沿用设计语言的既有方向；不据此扩大为所有组件的唯一画法，也不替换已有批准变体。“late · Late submission”重复可留待后续文案整理，不挡本轮审查。
- 四套主题真实终端观感与对比度尚未确认；筛选表单布局未调整。保留分支/worktree 与 idle 实现者，供后续画面回看和集成；不把代码审查通过写成用户已确认最终外观。
