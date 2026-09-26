# 交接

## 2026-09-26：右侧标签按钮纠正（以本节为准）

- 用户澄清只要缩小尺寸，保留原描边样式。已撤回上轮误改的单行方括号：恢复三行圆角描边、无填充，去掉横向多余留白，新增入口只显示 +。agent 名称及长名截断保持；当前 tab 顶边居中有 ● 标记，并提亮边框、加粗名称，悬停不添加选中标记。
- 本轮仍按用户授权直接修改，不委派、不操作队列。渲染/点击几何与受影响的切换、移动、取消检查通过；release 更新后重新运行 saddle 生效。设计见 DESIGN 第 30 节末尾。T14 已由用户放行，不重开，不执行 go/next。

## 2026-09-26：T14 已审查、发布与清理（以本节为准）

- 用户授权实施的弹框内部描边与素雅 Tab 已完成：开发 384339d，合并 b35ae28，主控审查 c93646b。New/Split 内部按钮描边无填充，底栏统一紧凑，Tab 三行且 × 独立命中。取舍与验证详情见 docs/任务/T14-按钮样式实施.md、DESIGN 第 30 节。
- 纯视觉预算：主控看完整 diff，直接检查 New 和 Tab/Split 合成渲染/命中两条均通过，未重跑套件；开发报告受影响检查与 Clippy 通过。功能状态机未改。旧 Queue/Stop 偶发问题仍未定位，不扩本任务。
- release 已重建并同步共享 target 与仓库 target/release/saddle，SHA-256 e8fbdbec3ce79abb09d296b146ba485bd808862fae203a3318b36ac6c1943039。默认 saddle 指向共享 release；用户当前实例未重启，重新运行生效。
- t14-outlined-buttons worktree/分支已正常删除；工作目录已删，一并关闭自有 saddle/dev-t14-buttons-1。当前无自有开发/审查 agent，无未收尾工作树；收尾空提交 22ca0c6，发布记录 d69f9cf。其他用户 agent 未操作。
- 公开 drover done T14 已核对通过（退出 8 为放行模式正常等待）：main 前进、分支已合并、收尾记号 22ca0c6；未配置额外 CHECK_CMD。T14 已完成待用户放行，下一步等用户体验反馈，不自动 go/next。原 T2 已放行不重开，T13 不启动。Agents diff 展示按用户最新决定保持现状，不另加任务。

## 2026-09-26：T14 已获实施授权并委派（以本节为准）

- 用户明确「不要待办，开干吧」，覆盖旧正文“先记录/只探索”。公开队列已核对 T14 current/doing，loop=false、gate=true；不需再次 next 或改写运行中条目，原 T2 不操作。
- 设计与任务 b153681：DESIGN 第 30 节、docs/任务/T14-按钮样式实施.md；盘点文件保留历史并追加实施状态。已开自有 saddle/dev-t14-buttons-1（Claude Code opus[1m] / high），worktree ../saddle-worktrees/t14-outlined-buttons，分支同名，完成提醒已挂好。
- 路由常规/不交叉，影响面拿不准；主控定纯视觉，预算 diff + 直接渲染/命中检查和受尺寸影响的必要现有断言，不跑全套或矩阵。功能/PTY 状态机不变；旧 Queue/Stop 偶发问题不扩处理。
- 下一步开发 DONE 后按预算主控审查，必要返工仅增量；通过后合并推送、更新 release、清理自有 worktree/分支/agent、空提交及交接，再核对 T14 完成并停待用户放行，不自动 go/next。当前尚未实现完成或发布。

## 2026-09-26：T14 样式盘点完成，仍待实施（以本节为准）

- 用户体验位置优先布局后表示不错，要求今天先探索 T14 涉及哪些现有弹出面板，并补充右侧 Tab 要做成素雅、不抢内容的按钮。
- 源码盘点和文字建议已写 docs/任务/T14-按钮样式盘点.md，明确需改的 New 内部按钮、Split 方向，以及新增 Tab 条；Queue 底栏和候选列表可保留，Stop 确认底栏有一个现有样式例外。方案仍为建议，尚未做渲染验收或改代码；具体方案和三行 tab 多占两行的成本见该文件。
- 已通过公开 drover edit 将用户新增的 Tab 要求及盘点链接补入 T14，保持原要求和 pending 状态，未 next/go、未操作已放行 T2。没有启动开发 agent 或更新 release。
- 下一步等用户确认具体样式后再按项目流程实施 T14。之前位置优先布局已合并收尾，Queue/Stop 偶发检查的未定位观察继续保留，不在 T14 扩展处理。

## 2026-09-26：位置优先布局合并、发布与收尾完成（以本节为准）

- 用户批准的右侧 Split/+ Tab 先选位置再选 agent 已落地；左侧旧 Show 移除，已打开项明确 Move here 并移动现有会话，取消不留空布局。最终开发 73b3d88，经主控和独立 Codex 增量复核通过，合并 e0a9164、收结 8141474 已推送。
- 两项阻挡均关闭：取消测试等待旧屏幕问题、候选刷新导致点击误接其他 agent。最后交叉六条定向工作流通过，主控 lib/terminals 与相关工作流、Clippy/diff 通过；未重复旧任务及整套。完整证据、逐项裁决和取舍见 docs/任务/终端布局主控与交叉审查.md、终端布局位置优先.md；设计见 DESIGN 第 29 节。
- 范围外观察仍未定位：未改动 Queue/Stop 用例在主控混合 workflow 检查中曾于 Ctrl-] 等待失败，单独复跑通过，交叉未发现本次增量直接关联。没有把该次混合检查说成全绿或将偶发问题记为已修复；用户另行要求时再调查，不扩大本任务。
- release 从合并后 main 构建成功；共享 target 与仓库 target/release/saddle 已同步，SHA-256 同为 5b0a78539fd70928a0523296dbac828271a03e960c43a49356d6c12679bf9103。默认 saddle 指向共享 release；当前用户实例未重启，重新运行生效。
- 开发 pane-placement、detached review-pane-placement 两个 worktree 与开发分支均已清理；工作目录已删，一并关闭自有 saddle/dev-pane-placement-1、saddle/dev-pane-review-1。收尾空提交 5d0f13f；当前仅 main worktree，没有等待结果的本任务 agent。其他项目 agent 未操作。
- 本任务已完成，下一步等用户体验反馈。T14「弹框内部按钮改为紧凑描边样式」只记录待办，未实施；本轮没有操作队列。原 T2 已放行，不重开、不执行 go/next。历史章节的开发中/阻挡状态均被本节覆盖。

## 2026-09-26：点击身份修复主控通过，第一轮交叉增量复核（以本节为准）

- 开发 73b3d88 在 placement 局部绑定按下目标名字，松开换人则取消；主控仅审 48a5a48 后增量，通过。lib 6、terminals 6 及受影响候选/取消/移动回归通过，Clippy/diff 通过。
- 混合 workflow 35 passed、1 failed、2 ignored；失败为未改动且不打开 placement 的 Queue/Stop 用例 Ctrl-] 等待，单独复跑通过，根因未定。已如实记录为范围外偶发问题建议另查，不能称全套全绿；交叉者仅核对增量关联性，不扩旧任务。
- detached ../saddle-worktrees/review-pane-placement 已实际更新至 73b3d88；通知原 saddle/dev-pane-review-1（Codex 重档）第一轮交叉增量复核唯一必须改 1 及修复引入问题。主仓库 docs/任务/终端布局主控与交叉审查.md 有完整证据、裁决及预算。
- saddle/dev-pane-placement-1 原开发 worktree/分支保留。尚未合并发布；复核通过后按规矩合并推送、release、清理两个 worktree/分支及自有 agent、空提交和交接。
- New/Agents 旧任务不重审，T14 仅待办；原 T2 已放行，不操作队列、不执行 go/next。

## 2026-09-26：交叉初审发现候选点击目标漂移，增量返工中（以本节为准）

- 独立审查 48a5a48 结论改完再合并：交叉必须改 1 为按下候选行后遇 ls 刷新，原处松开会选中换位后的另一个 agent。假 CLI 探针实际出现错误 attach 和输入；主控核对后接受阻挡，9 条可以不改全部接受，无建议项。
- 详细证据、逐项裁决和第二轮开发返工预算见 docs/任务/终端布局主控与交叉审查.md。原主控取消同步问题已关闭；本轮只修候选点击身份，保留其他已审取舍。
- 开发者 saddle/dev-pane-placement-1 留在 ../saddle-worktrees/pane-placement 修复；审查者 saddle/dev-pane-review-1 留在 detached ../saddle-worktrees/review-pane-placement（仍 48a5a48）等待。没有合并、发布或清理开发/审查环境。
- 下一步开发 DONE 后主控仅审增量和受影响检查，通过后先 checkout 审查 worktree 到新 SHA，再交原审查者增量复核交叉必须改 1。这是交叉后的第一次返工，交叉复核尚未开始；通过再按规矩合并推送、release、清理、收尾和交接。
- New/Agents 旧任务不重审，T14 仅待办；原 T2 已放行，不操作队列、不执行 go/next。

## 2026-09-26：位置优先布局返工复核通过，进入交叉审查（以本节为准）

- 开发修复 48a5a48 仅在取消回归增加等待草稿消失，保留所有原断言；主控只审增量，workflow 35 passed、2 ignored，diff 检查通过，必须改 1 已关闭。此前 Clippy 和其他测试目标通过，不重复全套。
- 审查任务与逐项裁决在 docs/任务/终端布局主控与交叉审查.md；已开独立 saddle/dev-pane-review-1（Codex 重档，gpt-6-astra / xhigh）并挂完成提醒，detached ../saddle-worktrees/review-pane-placement 固定 48a5a48，唯一可写主仓库该审查文件。开发者 saddle/dev-pane-placement-1 和 pane-placement worktree/分支保留等待可能返工。
- 下一步收到交叉审查 DONE 后逐项裁决，认可必须改交原开发者；复核只看增量并先更新审查 worktree SHA。通过后合并推送、release、清理自有环境、空提交和交接。尚未合并/发布。
- New/Agents旧任务不重审，T14仅待办；原 T2 已放行，不操作队列、不执行 go/next。

## 2026-09-26：位置优先布局主控发现一项检查失败，返工中（以本节为准）

- 开发 4e9d1fa 已完成，主控初审范围/取舍通过，但最终标准套件 133 passed、1 failed、2 ignored；取消与 New 草稿回归等待 + Tab 候选弹框超时，单独复跑通过，时序根因尚未定位。Clippy 与 diff 检查通过。
- 逐项裁决和返工预算见 docs/任务/终端布局主控与交叉审查.md。原开发者 saddle/dev-pane-placement-1 在 ../saddle-worktrees/pane-placement、分支 pane-placement 只修这项并写增量记录；主控已接受原失败保留空位及 Tab N/+ Tab 文案，不为这两项扩需求。
- 下一步收到返工 DONE 后只复核增量和受影响检查，主控通过后再安排独立 Codex 重档交叉审查。尚未合并、发布或清理开发环境，不把单次重跑成功当修复。
- 原 T2 已放行，不操作队列、不执行 go/next；T14 弹框描边按钮仅待办，不在本轮实施。New 与 Agents 样式旧任务不重审。

## 2026-09-26：用户批准位置优先布局，已委派开发（以本节为准）

- 用户认可 New，指出 Show 在鼠标点选即接入的流程中没有实际作用；已批准右侧 Split/顶部加号先选位置后选 agent 的方案。设计与任务提交 039b8d8；具体交互和理由见 DESIGN 第 29 节、docs/任务/终端布局位置优先.md。
- 已开自有 saddle/dev-pane-placement-1（Claude Code，opus[1m] / xhigh），worktree ../saddle-worktrees/pane-placement，分支 pane-placement。本轮涉及已有会话移动与异步归属，路由重；主控定碰要害、需独立交叉审查。
- 下一步：开发 DONE 后读状态、回复、任务完成记录，按预算主控审查及一次标准检查；通过后另开 Codex 重档在 detached review worktree 独立审查。逐项裁决，有必须改项交原开发者返工并仅增量复核；通过后合并推送、更新 release、清理自有 worktree/分支/agent、空提交和交接。
- New/Show 和 Agents 样式旧任务已完整收尾，无需重复审查。当前实现尚未完成、未发布；保留现有 New 和 Agents 样式。原 T2 已放行，不操作队列、不执行 go/next。其他项目 agent 不是本任务所有，不干扰。

## 2026-09-26：Agents 选中样式合并收尾完成（以本节为准）

- 第二阶段开发 ff728a2 已实现去掉 Agents 首行短竖线、选中名称亮色加粗；主控纯视觉审查通过，无必须改或新增建议项。审查提交 7729b85，功能和记录已推送 main；详细完成/审查记录见 docs/任务/Agents选中样式.md，设计理由见 DESIGN 第 28 节。
- 按预算核对 diff 和开发合成渲染证据：非末项、多行折行、两种焦点下的空格/背景/树线/空行/名称样式检查通过，diff 检查通过；没有重跑全套或 Clippy。New/Show 不重复审查，Queue 和交互未改。
- release 构建通过；共享 target 与仓库 target/release/saddle 已同步，SHA-256 均为 d00c8efb11c1a9f882b6692b5e34cdcfa2989c8b7204522c440e855993980f2c。默认 saddle 使用共享 release，用户当前实例未重启，重新运行即可体验 New/Show 和选中样式全部改进。
- agents-selection-style worktree/分支已清理；工作目录已删，一并关闭自有 saddle/dev-agent-selection-1。收尾空提交 f32a374。New/Show 的自有开发环境此前已清理，当前仅 main worktree，没有等待结果的自有开发/审查 agent。
- 本轮反馈改进全部完成，暂无已知待完成工作；后续等待用户体验反馈，不自动排新任务。原 T2 已放行，本轮未操作队列、不执行 go/next。历史章节中的“开发中/等待”均已被本节覆盖。

## 2026-09-26：New/Show 已合并收尾，接续 Agents 样式（以本节为准）

- New 输入框轮廓、点击光标与编辑、默认项目/agent 创建及 Show 用途/取消一致性已完成；开发最终 d02094c，主控按预算审查通过，合并 b6f4c10、审查 8998486 已推送。最终全套 128 passed、2 ignored；Clippy、diff 检查通过，另直接核对合成 PTY 的中文点击光标/编辑屏幕及三弹框 Cancel 渲染。详细记录在 docs/任务/T2-New表单体验改进.md。
- release 已从合并后的 main 构建；默认 saddle 指向共享 target release，仓库 target/release/saddle 同步且 SHA-256 一致。未重启用户当前界面，重新运行 saddle 生效。
- t2-new-agent-form worktree/分支已删除，住在其中的自有 saddle/dev-t2-new-form-1 一并关闭；收尾空提交 19f86e5。原 T2 已由用户放行，不重开、不操作该队列条目，不执行 go/next。
- 尚待完成：Agents 选中样式。saddle/dev-agent-selection-1（Claude Code opus[1m] / high）保留在 ../saddle-worktrees/agents-selection-style，已由主控 rebase 到 main f90b838（原建议 9a12077 变为 5f5ee68），追加实施通知 bf4f0bc，向同一 agent 送达第二阶段实施要求并挂完成提醒。现在等待实现结果，不能把建议阶段 DONE 当实现完成。
- 实施预算：纯视觉，git diff --check + 一项合成渲染检查；不跑全套、不另开交叉审查。实现完成后主控看 diff 和记录，通过再合并推送、更新 release、清理该 worktree/分支/agent、空提交和交接。任务 docs/任务/Agents选中样式.md；设计理由见 DESIGN 第 28 节。

## 2026-09-26：追加 Agents 选中样式，先建议后串行实施（以本节为准）

- 用户截图反馈选中 agent 只有背景色，首行最左侧的小白竖线不好看；要求另排 agent 去掉它，并评估选中块左侧主题色长条会不会更清楚或显乱。新任务 docs/任务/Agents选中样式.md，先做只读建议阶段。
- New 已由 saddle/dev-t2-new-form-1 提交 d78ad30（126 passed、2 ignored，待主控核验），同一 agent 正在处理已排队送达的 Open 用途/Cancel 一致性补充；两部分完成后再一起主控审查，不能仅据 New 的旧 DONE 合并。
- Agents 样式建议阶段已由 saddle/dev-agent-selection-1（Claude Code，opus[1m] / high）提交 9a12077，目前 idle；位于 ../saddle-worktrees/agents-selection-style、分支 agents-selection-style，仅任务文档变化。主控采纳方案 A：去掉白色短线、保留整块背景、选中名称加粗；长色条与树线叠加显乱，暂不加。Queue 超出本次范围，不要求用户为此另作决定。设计理由已入 DESIGN 第 28 节。
- 两项都涉及 src/ui.rs，待 New/Open 合并后，主控先将样式分支接上最新 main，再通知原样式 agent 实施方案 A，按纯视觉预算验证；不要将建议阶段 DONE 当实现完成。两名 agent 均已挂提醒，重复提醒先核对这里的已处理状态。
- New/Open 完成后还要继续 Agents 样式，不能因前一项收尾而遗漏。原 T2 已放行 History；不自动操作队列或重开 T2。

## 2026-09-26：同轮追加 Open 易用性与弹框一致性（以本节为准）

- 用户在 New 改进进行中补充：Open agent 用途晦涩，Esc/Cancel 与其他弹框的设计语言不一致。范围现扩展为同一轮 New + Open 交互改进，New 输入框/光标优先要求保留；tab/split 接入语义和状态机仍不改。
- 主控核对到 Open 的大块三行按钮与 Queue 紧凑底部工具栏不同；设计已追加至 DESIGN 第 27 节，补充任务为 docs/任务/T2-NewOpen交互补充.md。要求用途文案和动作分组清楚，复用现有 Cancel Esc 控件与行为。
- 原开发 agent saddle/dev-t2-new-form-1 正在运行，补充安排在其本轮结束后送达，继续原 t2-new-agent-form 分支/worktree。主控收到较早的 DONE 时须核对补充任务是否已执行，未完成 Open 补充前不能合并或收尾；避免旧完成提醒提前触发合并。
- 队列原 T2 已放行在 History，current/awaiting/pending 均空；本次继续为已授权反馈改进，不改队列。

## 2026-09-26：T2 用户反馈，New 表单改进开发中（以本节为准）

- 用户实际体验后表示 New 不知如何操作，并明确输入框不显眼、点击没有光标；已同意继续简化为选项目/选 Codex 或 Claude/创建，名称自动生成可改，完整命令收进高级设置。范围只含 New，不扩展 tab/split 或其他弹框。设计见 DESIGN 第 27 节末尾用户体验修订，任务见 docs/任务/T2-New表单体验改进.md，准备提交 8a2b4ea。
- 已新开 saddle/dev-t2-new-form-1（Codex，gpt-6-astra / high），在 ../saddle-worktrees/t2-new-agent-form、分支 t2-new-agent-form 开发。路由常规；交叉/影响面拿不准，主控按局部表单行为判为改行为、不做交叉审查，不改已修复的 PTY 生命周期。
- 下一步收到 DONE 后看 diff/完成记录并按预算做主控审查，特别核对输入框轮廓、点击光标和编辑的可观察结果。通过后按规矩合并推送、更新 release、清理自有 worktree/分支/agent、补收尾。
- 用户已放行原 T2，公开 CLI 确认 T2 在 History/done 且有放行时间，current=null、awaiting=null、pending=[]，loop=false、gate=true。本次作为后续体验改进继续，不重开或改写已放行 T2，不执行 go/next；尚未发布。当前只有上述开发 agent 和 saddle/main。

## 2026-09-26：T2 合并与清理完成（以本节为准）

- T2 已实现 saddle 内 New 启动 agent、右侧终端 tab 和四向分屏；开发 c35daa2、返工 46d8abb，经主控和独立交叉复核通过，合并 ebea609，功能与审查已推送 main。详细设计见 DESIGN 第 27 节，完成和审查记录见 docs/任务/T2-启动agent与终端布局.md、T2-主控与交叉审查.md。
- 两项阻挡均关闭，无新增建议：待接入输入误路由、过期 pending/spawn 未取消。初审标准检查 116 passed、2 ignored；返工主控增量 34 passed、2 ignored；交叉复核三项通过。Clippy、差异检查与 release 构建通过，不重复已完成全套。
- 本机共享 target release 与仓库 target/release/saddle 已更新并核对 SHA-256 一致；默认 saddle 命令解析到共享 target 的 release。当前实例未重启，用户重新启动 saddle 后体验 New n、Open o、tab 与分屏；真实终端观感仍待用户查看。
- 开发与 detached 审查 worktree、t2-terminal-layout 分支全部清理；工作目录已删，一并关闭自有 saddle/dev-t2-terminal-1、saddle/dev-t2-review-1。当前仅 main worktree 和 saddle/main agent。收尾空提交 ad47b7b。
- `drover done T2` 核对通过（退出码 8）：收尾记号、main 前进、分支合入检查通过；drover 未配置 CHECK_CMD，测试依据为上方实际 Cargo 记录。公开列表确认 current=null、awaiting=T2/done、pending=[]，loop=false、gate=true；等待用户放行，未执行 go/next。没有等待开发或审查的工作。
- 既有无关建议保持原状，不在本轮扩展。历史章节中的占位、审查阻挡、未合并等均为当时状态，不覆盖本节。

## 2026-09-26：T2 第一轮返工主控通过，交叉增量复核中（以本节为准）

- 开发修复 `46d8abb` 已处理待接入输入隔离与旧 pending/spawn 取消；原开发 agent saddle/dev-t2-terminal-1 空闲，保留原 worktree 等结果。
- 主控仅审 c35daa2..46d8abb，增量检查 34 passed、2 ignored，Clippy 与 diff 检查通过。裁决及取舍见 docs/任务/T2-主控与交叉审查.md，无新增范围。
- detached 审查 worktree ../saddle-worktrees/review-t2-terminal-layout 已实际更新到 46d8abb，交原 saddle/dev-t2-review-1 仅复核必须改 1、2 和修复引入的问题；这是第一轮交叉复核。
- 下一步看复核结论，通过后合并、推送、清理两个 worktree/开发分支及自有 agent，补收尾和交接。尚未合并、更新安装版本或标记完成；T2 仍 current，不自动 go/next。

## 2026-09-26：T2 交叉审查发现两项阻挡，返工中（以本节为准）

- 独立审查 c35daa2 结论「改完再合并」：必须改 1 是重复选择等待 status 的 B 后可能把输入给旧 A；必须改 2 是等待 A 断开期间改选 C 后，已过期 B 仍接入。主控核对探针与调用路径，接受两项；8 项实现取舍仍通过，无建议项。
- 详细证据、主控逐项裁决和返工预算见 docs/任务/T2-主控与交叉审查.md。尚未合并、发布、标记完成。
- 原开发者 saddle/dev-t2-terminal-1 在 ../saddle-worktrees/t2-terminal-layout 修这两项并补回归；原审查者 saddle/dev-t2-review-1 保留，detached worktree ../saddle-worktrees/review-t2-terminal-layout 仍为 c35daa2。
- 下一步收到开发 DONE 后，主控看增量并验证受影响检查，将审查 worktree 更新到新 SHA，请原审查者增量复核。复核通过后按项目规矩合并推送和清理收尾，不自动 go/next。

## 2026-09-26：T2 主控通过，独立交叉审查中（以本节为准）

- 开发提交 c35daa2 已完成 New 表单、终端 tab 和四向分屏；saddle/dev-t2-terminal-1 空闲，保留 ../saddle-worktrees/t2-terminal-layout 等待审查或返工。
- 主控核对 diff、范围和取舍通过，最终代码标准测试 116 passed、2 ignored，Clippy 与差异检查通过。取舍逐项裁决与审查要求见 docs/任务/T2-主控与交叉审查.md；尚未合并、发布或标记完成。
- 已启动独立只读审查 saddle/dev-t2-review-1（Codex，gpt-6-astra / xhigh），detached worktree ../saddle-worktrees/review-t2-terminal-layout 固定在 c35daa2。审查者只向主仓库上述审查文件追加意见，其他文件只读。
- 下一步：审查 DONE 后核对状态和回复、逐项裁决；有必须改项交原开发者返工，复核只看增量；通过后按 AGENTS.md 合并推送、清理两个 worktree/开发分支及自有 agent、补收尾并更新交接。T2 仍 current，等待用户放行的边界保留；不自动 go/next。

## 2026-09-26：T2 已委派开发（以本节为准）

- 公开队列当前为 T2「支持saddle内部打开corral start」，用户已补充需求并在讨论方案后明确「委派codex去做吧」。旧章节中 T2 占位、等待补充的状态已过时；本轮未操作队列推进。
- 设计及理由见 docs/DESIGN.md 第 27 节，任务文件为 docs/任务/T2-启动agent与终端布局.md；准备提交 `b5366ee`。当前 main 保留管理文档，功能开发在 t2-terminal-layout 分支。
- 自有开发 agent：saddle/dev-t2-terminal-1（Codex，gpt-6-astra / xhigh），worktree：../saddle-worktrees/t2-terminal-layout。路由为重 / 交叉审查要 / 碰要害。
- 下一步：开发回复 DONE 后核对状态、完成记录和 diff，主控按预算跑标准检查，再开 detached worktree 做独立 Codex 只读交叉审查。通过后按 AGENTS.md 合并、推送、清理、补收尾；本轮尚未完成实现、审查或发布。
- 本轮只开上述开发 agent；saddle/main 保留。既有非阻挡建议沿用下方记录，不扩展本任务。

## 2026-09-26：主控结束前交接（以本节为准）

### 当前状态

- 用户体验 Tasks 详情后反馈：「cool,基本满足我要求了。」本轮功能与文档均已完成，用户准备结束当前主控；没有等待开发者或审查者的工作。
- 本次公开 CLI 核对：T4、T12 都已完成并放行，已在 History；current、awaiting 均为空。loop=false、gate=true。待办仅 **T2：支持 tab 和 split**，仍是占位，用户补充具体内容前不开始设计、实现或自动 Next。
- 当前分支 main，交接编辑前 HEAD 为 `b94ca4d`，工作区干净；本次只更新本交接文件并提交推送。历史章节中的“待放行”“未推送”“待目视确认”等是当时状态，不覆盖本节。
- 只有 main worktree，全部自有开发/审查 worktree、分支和 agent 已清理。交接时 corral/main、drover/main、saddle/main 分别在各自主仓库；用户将自行结束当前 saddle 主控。下次先公开 `corral ls` 核对，不凭旧记录停止其他 agent。

### 最近完成与验证

- T4：Tasks 列表/详情共用区域，右侧终端保持原样，详情打开即查、查询返回后约 5 秒刷新、返回列表停止。实现 `643233e`，合并 `88e0815`，收尾 `145098c`；主控 102 passed、2 ignored，clippy 和差异检查通过，交叉审查三个定向检查通过。release 已构建并原子更新 `target/release/saddle`，用户已体验并反馈基本满意。
- T12：简短中文使用示例 `docs/任务详情使用示例.md`，实现 `e3e6bfa`，合并 `0c342c5`，收尾 `4bc1173`；主控对照现有功能核对、diff 检查通过。纯文档未运行 cargo、未构建 release，无需为本轮文档或交接再次重启。
- 既有 T1/T3/T5/T6/T7/T8/T9/T11 和 effort 间距/配色微调均已落地。设计及理由以 `docs/DESIGN.md` 为准，任务完成及审查记录在 `docs/任务/`，不重复实施。

### 接手顺序与悬项

1. 先读 AGENTS.md；委派前重新加载 corral-dispatch / corral 技能，按当前路由和 labels 规则派活。用公开 CLI 核对队列和 agent，再等用户补充 T2 或提出新任务。
2. T4 两条非阻挡建议仍保留：归属测试成功样本错配、routing=null 原因文案过于具体。用户未要求另做，不自动扩展；runner 的特殊包装器后代限制和逐项裁决见 `docs/任务/T4-主控与交叉审查.md`。
3. 旧记录的 pending 快照预检查与写入间竞态、--help 的 r reply、Queue 普通列表 PgUp/PgDn、历史帮助流程测试偶发失败均非本轮新增阻挡；需要处理时先核实现状。旧“drover 完整历史发布待确认”已被后续记录取代：公开 JSON 已验证返回完整历史，T4 schema 1 也已交付，本机可用。
4. current 退回 pending 已按用户决定不做。不要据 T8 原始双需求重新启动此项。

重要入口：`docs/任务详情使用示例.md`、`docs/任务/T4-任务状态与过程详情.md`、`docs/任务/T4-主控与交叉审查.md`、`docs/任务/T12-任务详情使用示例.md`、`docs/DESIGN.md` 第 26 节。开发只走公开 corral/drover CLI；标准测试及共享 target 约定见 AGENTS.md。

以下保留历史交接，供查提交和旧背景；不作为当前队列状态。


## 2026-09-26：T12 任务详情使用示例完成

- 使用示例已新增在 `docs/任务详情使用示例.md`，说明打开详情、状态与上次验收、查询返回后约 5 秒刷新，以及 Esc/Back 返回。开发 `e3e6bfa`，主控审查通过，合并 `0c342c5`，实现和审查已推送 main；任务记录见 `docs/任务/T12-任务详情使用示例.md`。
- 对照现有说明、界面标签及刷新接线核对，diff 检查通过。仅文档，未运行 cargo、交叉审查或构建 release；已安装 release 仍为 T4 版本，本轮无需重启。
- `t12-detail-example` worktree/分支已清理，工作目录已删，一并关闭自有 `saddle/dev-t12-doc-1`。收尾空提交 `4bc1173`。用户的 corral/main、drover/main、saddle/main 保留在各自主仓库。
- T4 已由用户放行；本轮 T12 用于体验新详情功能，`drover done T12` 已核对通过（退出码 8），current 为空、awaiting 为 T12/done，完成待用户放行；未执行 go/next。待办仅 T2，仍待用户补充，不开始设计或实现。
- T4 的两条非阻挡建议及既有悬项沿用下方记录，本轮无新增阻挡。


## 2026-09-26：T4 Tasks 状态与过程详情完成

- T4 开发 `643233e` 经主控及 Codex 只读交叉审查通过，无必须改项，合并 `88e0815`，已推送 main。点击任务在 Tasks 区域内切换详情，Back/Esc 返回；右侧终端保持原样。打开即查询，之后每次返回约 5 秒再刷新，返回列表停止详情查询。支持已记录的状态、判据、上次验收、Git 进展与开始/完成/放行记录。
- drover schema 1 已交付且本机公开 show 命令可用；saddle 只消费公开接口。设计见 `docs/DESIGN.md` 第 26 节，完成记录见 `docs/任务/T4-任务状态与过程详情.md`，逐项裁决见 `docs/任务/T4-主控与交叉审查.md`。
- 主控完整标准测试 102 passed、2 ignored，clippy、差异检查通过；交叉审查三个定向检查通过。三条非阻挡建议中，文档校验承诺已收窄；测试成功夹具与目标错配、routing=null 原因文案两项保留建议，后续是否另做由用户决定。本轮没有返工。
- release 构建完成，已原子替换并校验 `target/release/saddle`；用户重启后生效，当前实例未重启。真实终端字体和操作观感待用户查看。本 release 也包含已完成的 effort 间距与按档位配色调整 `6829963`。
- 两个 T4 worktree、`t4-task-details` 分支均清理；工作目录已删，一并关闭自有 `saddle/dev-t4-details-1`、`saddle/dev-t4-review-1`。收尾空提交 `145098c`。当前只有 main worktree；仍开着 corral/main、drover/main、saddle/main，各在对应主仓库，未操作其他用户 agent。
- `drover done T4` 已核对通过（退出码 8）：current 为空、awaiting 为 T4/done，完成待用户放行；未执行 go/next。pending 仅 T2（tab/split），仍待用户补充，禁止开始设计或实现。
- 其他既有悬项沿用下方记录；current 退回 pending 按用户决定不做。T4 没有阻挡项，既有 runner 的特殊包装器后代限制见审查文件，不在此扩展实现。


## 2026-09-26：T11 Agents Git 摘要完成

- T11 已按批准方案完成，开发提交 `2b60ab1` 经主控与 Codex 两轮增量交叉复核通过，合并提交 `a61ab3a`，实现及审查记录已推送 main。每个 agent 按公开 cwd 展示分支、相对基准提交数、未提交增删与未跟踪文件数。设计和限制见 `docs/DESIGN.md` 第 25 节，完成记录见 `docs/任务/T11-AgentsGit摘要.md`，完整裁决见 `docs/任务/T11-主控与交叉审查.md`。
- 必须改 1–4 均已关闭，最终复核无阻挡。初审标准测试 89 passed、2 ignored；第一轮主控相关回归 40 passed；第二轮 Git/git_env 8 passed、交叉复核子模块定向测试 1 passed，clippy 与 diff 检查通过。
- release 构建完成，已原子替换并校验 `target/release/saddle`，用户重启后生效；未重启当前运行实例。真实终端观感待用户查看。
- 开发和 detached 审查 worktree、`t11-agent-git` 分支均已清理；工作目录已删，一并关闭自有 `saddle/dev-t11-git-1`、`saddle/dev-t11-review-1`，其他用户 agent 未动。收尾空提交 `48194ac`，当前仅 main worktree。
- `drover done T11` 已核对通过，T11 完成待放行（退出码 8，等待用户 `drover go`）；待办顺序仍为 T4 → T2，不自动开始。T4 核查任务历史/过程详情；T2 仍待用户补充。
- 尚存事项沿用前轮：pending 快照检查与写入间竞态；旧 --help r reply、Queue PgUp/PgDn、drover 完整历史修复发布情况；既有帮助流程测试曾偶发失败，未调查原因。current 退回 pending 已按用户决定不做。T11 已接受的限制与建议详见审查文件，不自动扩展实现。

## 2026-09-26：T9 effort 选中显示与图标优化完成

- T9 两项均已完成：saddle/dev-t9-effort-1 的 5ca733f 经主控审查后合并。选中不再提亮 effort 暗格；图标改为三档点阵阶梯 ⡄⡀⡀ / ⡄⡆⡀ / ⡄⡆⡇。原因与方案见 DESIGN 第 23 节，审查见 docs/任务/T9-effort选中显示与图标优化.md。
- 主控标准检查：83 passed、2 ignored，Clippy、diff 检查通过。release 已更新 target/release/saddle，重启生效，当前运行实例未重启。合成渲染已验证，真实终端字体观感由用户查看。
- 开发 worktree、t9-effort-polish 分支已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t9-effort-1，其余用户 agent 未动。收尾空提交 efd4cfb。
- 下一步将 T9 标为完成待放行；最新待办顺序 T11 → T4 → T2，不自动开始。T11 是 agent 区域显示提交/diff 信息的调查方案，须用户确认后才委派实现；T2 仍待补充。
- 尚存事项沿用前轮：pending 快照检查与写入间竞态；旧 --help r reply、Queue PgUp/PgDn、drover 完整历史修复发布情况；既有帮助流程测试先前偶发失败原因未调查，本轮通过。current 退回 pending 已按用户决定不做。

## 2026-09-26：T8 pending 删除完成

- T8 已按用户最终决定「只做删除」完成：saddle/dev-t8-delete-1 的 74194bf 经主控审查后合并。Queue 选中 Pending 可点 Delete x，弹层确认 y、取消 Esc；移出待办，History 保留 Dropped。设计见 DESIGN 第 24 节，审查见 docs/任务/T8-pending删除与current退回评估.md。
- current 退回 pending 已由 drover/main 只读评估，用户明确暂不做；没有实现，不自动另开任务。T8 仅按删除功能验收及收尾。
- 主控标准检查：82 passed、2 ignored，Clippy、diff 检查通过。release 已更新 target/release/saddle，用户重启生效，当前运行实例未重启。
- 开发 worktree、t8-pending-delete 分支已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t8-delete-1，其他用户 agent 未动。收尾空提交 8638fd9。
- 下一步将 T8 标为完成待放行；公开队列最新顺序 T9 → T4 → T2，等待用户推进。T9 已记录「Claude high effort 未选中时两格，点击选中后像满格」的 bug，尚未排查。T2 仍待补充。
- 尚存事项：pending 写入前公开快照核对与实际写入间的竞态限制仍在；旧 --help r reply、Queue PgUp/PgDn、drover 完整历史修复发布情况；既有帮助流程测试先前偶发失败原因未调查，本轮通过。

## 2026-09-26：T7 Agents 委派 effort 图标完成

- T7 已由 saddle/dev-t7-effort-1 完成（45f95d7），主控审查通过并合并 main。Agents 从公开 labels.effort 显示 medium/high/xhigh 三档信号图标；缺失或其他值不显示。设计见 DESIGN 第 23 节，审查与取舍见 docs/任务/T7-Agents委派effort图标.md。
- 主控复跑标准检查：79 passed、2 ignored，Clippy、diff 检查通过。release 已构建并更新 target/release/saddle，重启生效，当前运行实例未重启。已有无标签 agent 不会凭空出现图标，后续委派需显式带 effort 标签。
- 开发 worktree 与 t7-effort 分支已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t7-effort-1。其余用户 agent 未动，已补收尾空提交 3047741。
- 下一步将 T7 标为完成待放行，等待用户操作；最新待办顺序 T8 → T4 → T2，不自动开始。T8 是 pending 删除及未运行 current 退回 pending 的需求（后者先评估）；T2 仍待补充。
- 尚存事项沿用前轮：T5 快照预检查与执行间的竞态限制；旧 --help r reply、Queue PgUp/PgDn、drover 完整历史修复发布情况；既有帮助流程测试先前偶发失败原因未调查，本轮通过。T7 无阻挡项。

## 2026-09-26：T6 所有项目 pending 汇总完成

- T6 已由 saddle/dev-t6-pending-1 完成，主控审查、合并。Queue 底部 All pending A 打开只读弹层，按登记项目显示 pending 任务位置、id、完整标题，可滚动、r 刷新、Esc 关闭。界面方案见 DESIGN 第 22 节；不在此弹层编辑或调序。
- 每次打开/刷新重读授权的 ~/.drover/projects，再后台调用各项目公开 list --json；加载、空队列和失败分别显示，失败不遮掉其他项目。只在打开/刷新时读取，不自动轮询所有项目；无真实队列或配置改动。
- 主控复跑标准检查：77 passed、2 ignored，Clippy、diff 检查通过；release 已构建并更新 target/release/saddle，重启生效，当前界面未重启。接受大写 A 快捷键及窄窗按钮多占一行的设计取舍，审查已写入任务文件。
- 开发 worktree 与 t6-all-pending 分支已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t6-pending-1。其他 agent 未动，已补收尾空提交 434a2d7。
- 下一步等待用户放行，待办顺序仍为 T4 → T2。T4 核查完成状态/过程详情，T2 tab/split 仍待补充，不自动开始。
- 尚存事项：T5 公开快照预检查与执行间的竞态限制；旧 --help r reply、Queue PgUp/PgDn、drover 完整历史修复发布情况。本次开发者曾遇到既有帮助流程测试偶发失败，主控本轮通过，未调查其原因。

## 2026-09-26：T5 pending 编辑与调整次序完成

- T5 已由 saddle/dev-t5-pending-1 完成，两轮主控审查后合并到 main。选中 Pending 可 Edit e 编辑标题/正文，Move up u / Move down d 调序，Ctrl-S 保存、Esc 取消；失败/刷新保留草稿，成功后保持选中。非 pending 无入口，首末方向禁用。设计及公开接口限制见 DESIGN 第 20 节。
- 第一轮主控复跑发现新增 PTY 测试提前发送选择按键，开发者用 64 字节分块重现并修正屏幕同步，未改产品逻辑；原断言保留。开发者最终完整标准测试 73 passed、2 ignored，Clippy 通过；主控增量复核 workflow 17 passed、2 ignored，Clippy 与 diff 检查通过。两轮结论见 docs/任务/T5-主控审查.md。
- release 已构建并更新本仓库 target/release/saddle，重启生效；未重启用户当前界面，未改用户配置。开发分支/worktree 已清理，工作目录已删，一并关闭 saddle/dev-t5-pending-1；其他 agent 未动。已补收尾空提交 7518d18。
- 当前写入前会通过公开快照核对 pending 列表，变化则拒绝并保留草稿；drover 尚无公开版本获取入口，检查与写入之间仍存在竞态。若以后消除该限制，须另行授权扩展 drover，不能读取内部文件绕过。
- 下一步等用户放行，待办顺序 T4 → T2；T4 核查并补齐完成后的状态/过程详情，T2 tab/split 仍待补充。不自动开始下一件。
- --help 的 r reply、Queue 普通列表 PgUp/PgDn、drover 完整历史修复的远端发布情况仍未在本任务处理。

## 2026-09-26：T3 Tasks 状态文字颜色区分完成

- T3 经用户确认解除占位限制，已由 saddle/dev-t3-colors-1 实现、主控审查并合并到 main。Current/Running 蓝，Awaiting 琥珀，Pending 紫；History 标题中性，历史行 Done 绿、Failed 红、Dropped 橙，未知值原样中性显示。配置沿用 T1 现有字段，无需修改用户配置；设计和理由见 DESIGN 第 21 节。
- 开发者完成 UI 16 项、配置检查和 Clippy；主控检查完整 diff 与渲染检查，diff 检查通过，按纯视觉预算不重跑套件。release 构建通过，已更新 target/release/saddle；用户下次重启生效，本轮未重启运行中的界面。
- t3-task-colors 分支及 worktree 已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t3-colors-1，其他 agent 未动。已补收尾空提交 286054f，主控审查见任务文件。
- 下一步按用户调整的顺序为 T4（核查并补齐完成后的状态与过程详情）、T2（tab/split，仍待用户补充）。T3 收尾后等待放行，不自动开始下一件。T4 应先核查公开接口与显示差异，不预判遗漏原因。
- 旧遗留 --help 的 r reply、Queue 普通列表 PgUp/PgDn、drover 完整历史修复的远端发布情况未在 T3 处理。

## 2026-09-25：T1 启动配置与颜色配置完成

- T1 已由 saddle/dev-t1-config-1 实现，主控审查并合并到 main。27 项颜色通过启动配置进入界面，已有布局、刷新、命令路径配置保留；使用方式见 README 配置节和根目录 config.toml，设计见 DESIGN 第 8 节。
- 主控复跑标准检查：64 项通过，2 项依赖真实 drover 的既有测试忽略；Clippy 无警告，diff 检查通过。release 已构建，并更新本仓库 target/release/saddle；没有重启用户当前界面，下次启动加载新配置。
- 本机原先没有默认配置，现已创建 ~/.config/saddle/config.toml，内容与仓库默认示例逐字一致。程序支持绝对路径 XDG_CONFIG_HOME 和 --config 优先覆盖，省略项仍用当前默认值。
- 开发 worktree 和 t1-config 分支已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t1-config-1。未关闭其他 agent，未改 corral/drover 仓库或服务。已补「收尾: T1 启动配置与界面颜色配置完成」空提交。
- 下一步等用户：T2「支持 tab 和 split」、T3「Tasks 状态文字颜色区分」已通过公开 CLI 加入队列，均为占位，详细内容由用户后续补充；补充前不设计或实现，不自动放行下一件。
- 仍悬着的旧事项：--help 残留 r reply、Queue 普通列表 PgUp/PgDn 不翻页，以及 drover 完整历史修复的远端发布情况。T1 不扩展处理。

## 2026-09-25 晚：接入主控分派和 drover（新主控先读本节）

- **开发方式变了**：从这里起，`saddle/main` 是主控，只拆任务、写任务文件、审查、合并，不自己写功能代码；规矩见 `AGENTS.md`「开发方式（主控分派）」，分派按 corral-dispatch 技能做。旧的 `saddle/main` 是亲手写代码的开发者，已收尾关掉，本节以下是它留下的交接。
- **drover 已接上**：交接目录 `~/.drover/saddle`，`.drover.conf`（不进 git）填了 `MAIN_AGENT=saddle/main`、`TASK_FILE_DIR=docs/任务`、`DONE_MARK=收尾`，`CHECK_CMD` 留空（不跑验收命令），放行模式（每件做完等用户 `drover go`）。队列目前是空的，任务由用户加。
- **仓库状态**：main 已推送到 origin，和远端一致（推送含旧会话的交接提交 `432471d` 和本次 AGENTS / HANDOFF 改动）；下面「本次交接提交仅保留本地」一句已过时。本文件由 `docs/PROGRESS.md` 改名而来。
- **约定**：每件活一个分支和 worktree（`../saddle-worktrees/<分支>`），所有 worktree 共用编译目录 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；标准测试 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`；合并后推送，收尾打「收尾: 」空提交，再更新本文件。
- **可以接着做的**：见下一节「已知问题与接续事项」（帮助文案里残留的 `r reply`、Queue 列表 PgUp/PgDn 不翻页等）。做不做、先做哪件，等用户给任务。

## 2026-09-25：旧会话收尾，交给新主控（接手先读本节）

### 当前状态

- 用户通过 corral/main 转达：后续改为主控分派开发，另开新主控，本会话结束。本轮只记录交接并提交，不开始新功能、不推送、不创建或关闭 agent。
- 三窗格、Agents 树形信息/配色、Queue 英文控件/状态/空态、鼠标滚动及完整历史接入均已完成；用户已反馈当前效果满意。没有做到一半的实现或未提交开发改动。
- 公开仓库为 https://github.com/firegnu/saddle。此前已推送 main 至 `7943443`，读回远端 SHA 与本地一致；英文 README 为默认首页，顶部可切中文。本次交接提交仅保留本地。
- `target/release/saddle` 已构建。最近完整默认套件为 59 项通过；之后增加的完整历史跨仓库集成测试已单独通过，现在有 2 项依赖已安装 drover 的测试默认忽略。clippy、fmt 和 release 构建已有通过记录。本轮仅更新文档，检查 diff，不重跑行为测试。
- drover 修复为 `3e66d1a`（收尾 `22de37d`）：JSON 取消十条限制，普通 list 摘要保持；saddle 验收提交为 `26cbd35`。真实公开接口核对过 22 条历史，40 条合成历史经真实 CLI 进入 saddle 后鼠标可到两端。没有修改 corral 的任何代码。

### 已知问题与接续事项

- `src/main.rs` 的 `--help` 仍列着 `r reply`，但 Agents 的 r 和 Reply 按钮已按用户要求隐藏/停用；这是帮助文案遗留，底层回复功能仍保留。本轮不扩展修改。
- Queue 普通任务列表的 PgUp/PgDn 仍更新 `scroll`，列表绘制使用 `top`，因此不能翻列表；详情/反馈分页正常。用户明确以鼠标/触控板为主，先前轮次有意未修改此项，鼠标滚动已验证正常。
- 本会话首次公开发布时核对：GitHub 上 drover 尚未包含完整历史修复，远端版本可能仍只返回十条；本机既有软链指向已修复代码。没有获得推送 drover 的授权，新主控如要发布该依赖修复，应先确认最新远端及对应授权。
- 主题可配置、其他平台兼容性均未实现/验证，不属于已承诺待完成项。自动测试使用假 agent；不把合成终端测试当作对所有真实 agent 应用的完整兼容性保证。
- 仓库 `AGENTS.md`（`CLAUDE.md` 链接到它）目前仍写“直接在 main 上开发”。新主控接手后需按用户最新决定落实主控分派流程与相应仓库规则；本轮不擅自设计流程或启动分派。继续遵守不干扰现有 agent、测试隔离及明确授权后才推送的边界。

以下章节为历次记录；其中“未推送”“无未完成项”等表述是当时状态，以本节汇总为接手基准。

## 2026-09-25：英文首页、中文 README 与首次公开发布

- 用户明确授权创建 GitHub public repo 并推送；公开仓库为 https://github.com/firegnu/saddle，origin 已配置。本次发布范围仅 saddle，不推送 drover 或 corral。
- README.md 为默认英文首页，顶部链接 README.zh-CN.md，中文版链接回英文；整理功能、构建/安装、入门操作、配置、快捷键与验证方式，移出首页的历史实现细节仍保留在 docs。
- 两种语言的命令和配置逐字一致，本地链接与 Markdown 代码块检查通过，`cargo build --release --locked` 通过。本轮仅改文档，不重复运行行为测试；Gitleaks/TruffleHog 检查 Git 历史未发现凭证。
- 明确记录依赖版本边界：GitHub 上 drover 的 JSON 接口尚有限制十条的版本，完整历史需要其修复版；没有把本地未推送的依赖修复当作已公开发布。
- AGENTS.md 的“没有远端”旧说明已更新为仅在用户明确授权时推送，后续发布仍需对应授权。

## 2026-09-25：真实截图下历史仍不可滚动（已修复接口截断）

- 用户再次确认 e6d2e36 的历史底边没有解决问题，上一轮关于历史区已完成的结论不足。
- 只读核对真实公开接口：drover 项目的 `drover list --json` 仅返回 T22…T13 共 10 条历史，未返回历史总数或分页字段；`drover list --json --all` 被拒绝为用法错误。saddle 展示了收到的全部 10 条，截图视口容得下这些行，最大滚动偏移为 0；不能把这一现象继续当作滚轮事件失效来修。
- 用户明确授权检查/修改 drover 后，确认 JSON 分支的 `fin[-10:]` 是截断根因。已在 drover main 修复为返回全部历史（3e66d1a，收尾 22de37d），普通 list 摘要、JSON 顺序和字段保持。saddle 沿用相同公开命令即可读取全量，不增加内部文件依赖。
- 两条真实 RED→GREEN：drover 的 11 条合成历史检查捕获最早一条丢失；saddle 的真实 CLI + 40 条临时历史整程序测试捕获接口只有 10 条，修复后完整读取、鼠标立即移动视口、正文/滚动条可到两端，合成状态文件不变、无真实 agent。新增测试默认忽略，用 `SADDLE_DROVER_BIN=… cargo test --test workflow installed_drover_complete_history -- --ignored` 单独运行。
- drover CLI 标准回归、saddle 定向跨仓库验收、clippy -D warnings、fmt 和差异检查均通过。只读核对真实公开 JSON 已返回 T1–T22 共 22 条；安装命令是既有软链，当前 saddle 自动刷新即生效，无需重新安装或重启。本轮未改真实任务数据、未操作服务/agent；两个仓库 main 本地提交，未推送，无遗留实现项。

## 2026-09-25：Queue 英文、状态配色与历史区边界（完成）

- Queue 的分组、计数、状态、空态、帮助、项目选择/路径、新增表单、详情与操作反馈、底栏输入目标统一英文；任务内容、路径、原因和公开 CLI 输出保留原文。
- 顶部保留 Manual/Auto，单独显示 Running / Awaiting / Ready / Idle / Paused，不再把空队列标成运行中。运行蓝、等待/暂停琥珀、就绪/空闲/完成绿、失败红、放弃橙，沿用 Agents 强调色；状态加粗，Loop on 绿色、off 弱化，未知任务状态中性原样。反馈依据命令结果区分成功/失败，执行中蓝色。
- RED→GREEN：只有历史记录、没有 current/awaiting/pending 时缺少空态提示；现在显示 No active tasks，完全空队列补充 No history yet，读取中/失败不误报空队列。
- 用户确认历史问题是区域下方大片留白，而非缺失旧记录。历史区固定底边延伸至任务按钮上方，显示可见范围/总数和 End 标记；短列表保持紧凑行距，长列表仍可滚到两端。只读核对 drover 公开 list 返回 10 条历史、0 条活动任务；不读取内部文件，不改真实队列。
- 59 项全量自动测试通过，1 项真实 drover 测试继续忽略；clippy -D warnings、fmt、release 构建及 diff 检查通过。验证英文界面保留中文源数据、各状态颜色、短历史底边、空态、加载/失败边界和鼠标滚动；检查三尺寸合成字符网格与弹层交互。
- DESIGN 第 21 节、README 已同步；main 小步提交、未推送。新版 target/release/saddle 已重建，需要重新启动；未操作真实 agent、未重启用户界面。本轮无未完成项。

## 2026-09-25：Queue 紧凑按钮与鼠标滚动（完成）

- Queue 的项目/队列/任务和弹层按钮统一为 Agents 的单行英文无底色样式；收紧项目头部，给任务列表更多空间，保留禁用、指针反馈与释放执行规则。
- 用户明确问题是鼠标/触控板，本轮没有修改 PgUp/PgDn。根因是滚轮被转换为上下选择，选中项没到视口边缘时内容不移动；现改为直接滚动视口，正文和滚动条都响应，保持任务选择与正常刷新后的滚动位置，显式选择时恢复跟随。
- 两项 RED→GREEN：独立 PTY 连续滚轮后第一项仍可见；末项可见时滑块仍未到底。修正滚动事件路由和最大偏移范围，任务文字预留滚动条列。回归覆盖上下两端、刷新保持、列表外滚轮不生效、选择恢复跟随及无 drover 写操作。
- `cargo test --all-targets`：55 项通过，1 项真实 drover 测试继续忽略；clippy -D warnings、fmt、release 构建、diff 检查通过。检查三尺寸合成字符网格及原生按钮/表单/弹层点击，没有启动真实 agent 或重启用户界面。
- DESIGN 第 20 节与 README 已同步，直接在 main 小步提交、未推送；新版 `target/release/saddle` 需重新启动才生效。本轮无未完成项。

## 2026-09-25：Agents 滚动到底与多 agent 排版（完成）

- 用户确认问题发生在鼠标滚轮/触控板。两个独立 RED：内容最后一行可见时滑块末尾仍为轨道；滚轮位于滚动条列时无法滚到最后一个 agent。最小化为 2 个 agent/小视口，并保留 13 个 agent 场景。
- 根因与修复：Ratatui ScrollbarState 的位置范围按可滚动偏移设置为 总行数−视口行数+1，原实现误传总行数；右侧滚动条列不在文本 list 的命中范围内，现将正文和滚动条一起接收滚轮。两项复现均 GREEN；集成测试还验证滚回顶部、正文滚动、刷新后不跳回、无 attach 副作用。已在 edf733a 单独提交。
- 多条目排版：宽窗按同组名称长度利用主行空余列，claude-demo-1 等名称可一次完整展示；确实放不下才保留补充行。类型在主行完整可见时，第二行不重复类型；窄窗和超长未知类型仍在身份行保留。每条之间加不可接入的空白行，树线连续；路径逐项保留并弱化，其他信息不删除、不折叠、不移到选中项详情。
- 新增保留全部字段、长名称只出现一次、窄窗保留类型和空白行无点击目标的 Buffer 检查；原有信息、选择、状态、按钮/弹层回归保留。53 项全量测试通过，1 项真实 drover 隔离测试继续忽略；clippy -D warnings、fmt、release 构建、diff 检查通过。
- README、DESIGN 第 19 节已同步，Rust 预览例子新增多 agent 场景；检查了合成字符网格，实际终端观感待用户反馈。main 小步提交、未推送，未操作任何真实 agent，也未重启用户 saddle；release 已重建。本轮无未完成项。

## 2026-09-25：Agents 计数、状态重点与短实例 ID（完成）

- 总数固定在顶部 Agents · N；repo 根行右侧对齐 (N)，按完整组计数，长名称省略保留数量。滚动归属/隐藏数移到底边，顶部总数不被覆盖，标题提亮加粗。
- Agents 选中底色从终端较重的 bright black 改为截图适配的薄暖灰 #302a23；仅 Agents 使用，Queue 选择样式不变。Codex 标识/类型名改薄荷青 #8ed9c1 并加粗，Claude 保持陶土橙。
- 状态加粗，使用独立真彩色：working 蓝、idle 绿、blocked 黄、stalled 橙、error 红、starting 紫，避免 ANSI 主题使语义色不明显；默认正文、边框等继续跟随终端。
- 第二行实例 ID 缩为前 6 位；完整数据及停止确认仍保留原 ID，命令身份判断不变。
- 新增计数检查先复现旧标题缺少总数，再通过多 repo/多 agent、状态排序、长名称/窄窗和空列表检查；渲染检查覆盖每种状态的颜色/加粗、Codex 强调、短 ID 及确认中的完整 ID。50 项全量测试通过，1 项真实 drover 测试保持忽略；最终标题样式另经 ui 检查通过，clippy -D warnings、fmt、release 构建、diff 检查通过。
- 已同步 README 和 DESIGN 第 18 节，检查三尺寸合成字符网格；具体终端字体与观感仍待用户目视反馈。main 小步提交，无推送；未操作真实 agent，未重启用户 saddle。release 已重建，本轮无未完成项。

## 2026-09-25：Agents 英文文案、隐藏回复入口与终端主题（完成）

- Agents 的状态、空列表、未读标记、停止确认、底栏说明及连接提示改英文；agent 标题、工具名、路径、输出/错误原文不翻译。Queue 自有文案尚未调整。
- 隐藏 Reply/Hide 按钮，停用 Agents 的 r。保留回复读取、渲染和滚动代码；已有渲染测试继续直接开启 show_reply 验证内容。Queue 刷新和 Viewer 输入路由不变。
- 共享主题改用终端默认色和 ANSI 16 色，选中背景为 bright black；树线在选中项上使用 ANSI white 保持可见。品牌色和 Viewer 内容色不变，不新增配置。Queue 的共享控件也采用这一主题。
- RED→GREEN：新增合成 PTY 测试先观察到 r 打开 Last reply，修改后验证入口消失、r 不打开面板且不调用 reply；补充英文文案保留中文源数据、终端色和英文停止确认的 Buffer 检查。默认背景让部分空格由终端清除指令产生，修正测试点击助手将空单元格还原为空格并跳过双宽续格，实际点击坐标保持一致。
- 全量 48 项测试通过，1 项真实 drover 隔离测试保持忽略；clippy -D warnings、fmt、release 构建和 diff 检查通过。三尺寸合成字符网格已检查；导出的 SVG 使用示例 ANSI 色，实际颜色由用户 Terminal 调色板决定，未把预览当成实际终端配色验收。
- README、DESIGN 第 17 节已同步，main 小步提交，未推送。未启动或操作真实 agent，未重启当前 saddle；release 已重建，重新运行生效。本轮无待实现或待拍板项。

## 2026-09-25：Agent 类型品牌标识（完成）

- 主行类型名加入同色字符标识：✳ claude 陶土橙、>_ codex 白、π pi 白、π omp 紫。以官方网页/资源为配色依据，OMP 用渐变的代表性紫色；单行字符是近似标识，不引入图标字体、图像协议或依赖。
- 仅调整主行类型样式，状态、连接、选中背景和附加信息颜色不变；未知类型保持中性原名。宽窗让出两列名称宽度容纳标识，长名称仍在后续行完整显示；中窄窗口保持原有布局。
- 纯视觉修改以 Ratatui Buffer 验证四种类型/未知类型的前景色、选中/非选中底色及状态/连接标记；检查合成三尺寸网格。全量 46 项测试通过、1 项真实 drover 测试保持忽略；clippy -D warnings、fmt、release 构建及 diff 检查通过。
- README、DESIGN 第 16 节已同步。直接在 main 小步提交，无推送，无真实 agent 操作。release 已重建，用户当前运行实例不重启。本轮无待拍板或未完成项。

## 2026-09-25：Agents 紧凑按钮、选中底色与 repo 树（完成）

- Agents 按钮改为单行英文轻量边界（如 ‹Reply r›），无底色，窄窗按两列换行；共享指针命中、松开执行和悬停/按下规则保持。Queue 与弹层仍用原有三行按钮。
- 选中 agent 的主行和附加信息整行恢复 #232931 底色；面板其余区域继续使用终端背景。repo 青色、身份信息蓝色、路径与树线弱化、标题正文色。
- SOURCE 改为 VIA，去掉 DIR/TITLE 标签但保留内容；路径只显示末两级（长路径前缀 …/）。每个 repo 下以 ├─/└─ 连接多个 agent，附加信息延续竖线；repo 根滚出视口时标题保留顶部条目所属 repo。
- 新增多 repo、多 agent 渲染检查，先复现旧字段和完整路径，再验证状态排序前后的树枝、整条选中底色、窄窗换行和滚动归属。已有回复检查确认每项信息不被 Last reply 替换；按钮检查同时覆盖普通与紧凑样式的透明底色及指针状态。
- `cargo test --all-targets`：45 项通过，1 项既有真实 drover 隔离测试保持忽略；最终渲染改动再次通过 ui/buttons 检查。clippy -D warnings、fmt、release 构建、diff 检查通过。合成三尺寸字符网格已检查；真实终端字体观感待用户目视反馈。
- 已同步 README、DESIGN 第 15 节；直接在 main 提交，未推送。未启动或操作真实 agent，未重启用户当前 saddle。`target/release/saddle` 已重建，退出旧进程后重新运行生效。本轮无待实现或待拍板项。

## 2026-09-25：按用户习惯修整 Agents 和按钮（完成）

- Agents/Queue 使用终端默认背景，选中项改用竖线和文字强调；按钮正常/悬停/按下均不填底色。所有按钮改英文标签、圆角细边框和三行高度；窄窗按行重排，四按钮超宽时采用两列，边框也可点击。危险操作仍为红色且保留确认。
- 每个 agent 的 KIND/INST/ATT/SOURCE/DIR/TITLE/DOING 放回各自条目下，长内容折行；r 展开独立 Last reply，不替换附加信息。PgUp/PgDn 在回复打开时翻回复，关闭时翻列表；鼠标仍可分别滚动两个区域。
- Queue 仅适配无底色按钮及其高度，操作语义不变；读取失败时把空间让给完整错误，不继续占用禁用的项目动作按钮。
- RED→GREEN：新增渲染检查先复现非选中条目附加信息缺失，再验证 r 打开前后两个 agent 的字段都在各自条目中。透明背景、英文轮廓、指针状态、命中区域和窄窗重排使用 Buffer 检查；合成 PTY 保留所有操作回归。
- `cargo test --all-targets`：44 项通过，1 项既有真实 drover 隔离集成测试保持忽略；clippy -D warnings、fmt、release 构建、diff 检查通过。已更新 README 和 DESIGN 第 14 节，重新生成三尺寸合成网格供检查。
- 直接在 main 小步提交、未推送；没有启动或操作真实 agent。`target/release/saddle` 已重建，退出旧 saddle 后重新运行生效。本轮无未完成项。

## 2026-09-25：Claude Design 界面重设计（完成）

- 基准为用户提供的 Claude Design 方案，技术栈不变。设计差异与真实数据边界已写入 DESIGN 第 13 节；直接在 main 小步提交，未推送。
- 统一语义配色、全行选中底色、独立连接标记、琥珀焦点和输入目标底栏。按钮有主次/危险/禁用样式、悬停/按下反馈；只在同一目标松开才执行，拖出取消并捕获余下鼠标事件，不泄露到 Viewer。
- Agents：宽窗两行扫描、中窄窗单行；完整字段集中在可滚动详情，回复在详情区展开。停止为原生确认弹层，执行中阻止重复提交。
- Queue：项目名/路径/项目选择/刷新置顶，项目操作与任务操作分开；分组计数、中文长标题省略、固定右侧状态。项目、目录、详情、新增、帮助、反馈均为原生覆盖层；不虚构登记项目的读取状态。
- 响应布局：≥140 列使用配置宽度，100–139 列最多 44 列，<100 列最多 34 列并收为 Agents/Queue 标签；保留选择、滚动及唯一 Viewer 连接。
- 弹层阻止背景点击。Ctrl-] 暂回 Agents 保留 Queue 页面/草稿，Tab 返回继续；Esc 取消。延迟的接入结果不抢走 Queue 或停止确认的焦点。自动刷新保留原数据与选择，切换项目才清空。
- RED→GREEN：响应布局、按钮释放执行、按钮拖出后无鼠标泄露、延迟 attach 不抢表单输入、停止中不重复提交、暂停编辑后草稿保留。整程序 12 项回归通过，全部使用假 CLI 和独立 PTY。
- 全量 `cargo test --all-targets`：42 项通过，1 项依赖本机 drover 的既有隔离集成测试保持默认忽略，本次未重跑。`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、release 构建及 diff 检查通过。
- 新增 `cargo run --example ui_preview -- /tmp/saddle-ui-preview`：直接导出 Ratatui 合成 Buffer 的 SVG/文本（160×48、120×36、80×24 及项目/表单/停止/回复）。已检查字符网格、颜色属性、CJK 状态列与极小尺寸无越界；浏览器阻止本地 SVG 页面访问，未绕过限制，因此没有把浏览器像素检查记为通过。
- 已更新 README。可执行文件 `target/release/saddle` 已重建；正在运行的旧进程需要退出后重新启动。没有启动或操作任何真实 agent，也没有修改真实项目/全局配置。真实终端字体与观感仍由用户目视验收。
- 本轮无待实现项，无需用户拍板。

## 2026-09-25：按钮视觉修整

- 仅调整共享按钮渲染：去掉方括号和亮蓝底，改为低对比度深灰底、浅色操作名、弱化快捷键；增加按钮间距和换行留白。操作和快捷键保持不变。
- 纯视觉修改，以独立 PTY 的原生按钮点击、表单、停止确认和窄窗重排回归作为验证，不编造失败逻辑测试。7 项 workflow 测试、clippy、fmt、release 构建通过；依赖真实 drover 的测试本次未重复运行。

## 2026-09-25：修复 Queue 项目入口，加入原生按钮

- 根因已复现：从 saddle 目录执行 `drover list --json` 返回“尚未接入 drover”；原实现只读启动目录，没有沿用登记项目列表，而且完整错误被截在页脚、状态仍显示读取中。
- 用户已明确授权只读 `~/.drover/projects`。默认优先 `queue.cwd`、已登记的启动目录、登记首项；无登记才尝试启动目录。点击「项目」从原生列表切换，支持刷新登记清单和手动目录；不读取 `.drover.conf` 或任务内部文件，不自动初始化仓库。
- 读取失败在内容区折行展示完整错误，支持滚动，禁用过期状态上的写操作；成功后清除读取错误。切换项目会丢弃旧 worker 的结果，执行操作时禁止切换。
- Agents 新增接入、回复、排序、停止及确认/取消按钮。Queue 新增项目、刷新、详情、新增、放行、下一件、暂停/恢复、循环、帮助按钮；表单可点击字段、保存/取消。窄窗按钮自动换行，全部由 Rust/ratatui 绘制。
- 自动检查：32 项通过，1 项真实 drover CLI 隔离测试默认忽略、已单独执行通过。clippy（拒绝警告）、fmt、release 构建、diff 检查通过。
- RED→GREEN：长路径错误不再截断/假装加载；从未接入目录启动可自动加载登记项目；鼠标切换后动作只作用于选中项目；表单和停止确认可点击。回归还覆盖缩窄窗口后的按钮命中、失败时阻止过期动作、登记缺失/去重/读取错误及手动修正目录。
- 实际只读核对：登记的 drover 项目公开 `list --json` 正常返回。没有修改登记清单或真实项目，没有启动、接入或操作真实 agent。新版可执行文件为 `target/release/saddle`，已运行的旧进程需退出后重新打开。

## 第 1 步（原生 UI 修订版）：实现与自动验收完成

2026-09-25，直接在 main 分步提交，未推送。按用户后续要求，原来的「Queue 先嵌入 drover board」方案已经作废。

### 当前实现

- Agents：ratatui 原生面板。公开 ls/status 每秒后台刷新；项目分组、状态排序、完整字段和窄屏折行、选中项保持、未读标记、滚动条/隐藏数量、回复分页、x/y 停止确认。
- Queue：ratatui 原生列表、详情、帮助、操作反馈、标题/正文新增表单。公开 `drover list --json` 提供 mode、paused、current、awaiting、pending、history；g/n/p/l/a 通过公开 CLI 执行放行、下一件、暂停/恢复、循环和新增。失败保留新增草稿，执行中不重复提交。
- Viewer：唯一使用 PTY 的窗格，仅运行公开 `corral attach`。Rust/alacritty_terminal 解析、ratatui 绘制颜色、中文、光标、鼠标和粘贴；切换等待旧 attach 退出，再接入最新选择。已有其他 attach 时拒绝接入，agent 消失后不自动换人。
- 所有 saddle UI 都是 Rust。生产代码没有 `board` 或 Python UI 调用；不启动外部编辑器。测试中的 Python 文件只是假的 CLI/字节流边界，不作为 UI 实现。
- Queue 的 `queue.command` 已删除并在解析时拒绝；新配置为 `queue.drover` 和可选 `queue.cwd`。未指定 cwd 时从 `~/.drover/projects` 选择默认项目（用户已授权只读该清单）。
- TOML 配置、三窗格缩放、焦点路由、后台命令取消/超时、终端退出恢复、README 和 `--help` 已同步。

### 验证

全部通过：

- `cargo test --all-targets`：32 项通过；依赖本机 drover 的 1 项默认忽略，已单独执行通过。
- `SADDLE_DROVER_BIN=/Users/firegnu/.local/bin/drover cargo test --test workflow installed_drover_cli -- --ignored`：真实公开 CLI 在隔离 HOME/临时项目中驱动原生 Queue，验证正文、暂停/恢复、循环开关、多行中文新增和缩放。不调用外部看板，不启动真实 agent。
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `cargo build --release`：可直接运行 `target/release/saddle`。
- `git diff --check`
- 生产调用点检查：仅 `src/viewer.rs` 调用 `Session::spawn`；无 `board`、`python`、`queue.command` 调用。

已有合成原型 `cargo run --example compare_parsers` 对比 alacritty_terminal 与 vt100 的中文、真彩/256 色、光标、清屏和滚动区；选择前者的理由是终端查询的事件回传能力。

RED→GREEN 包括布局、配置、公开 JSON、刷新选中项保持、输入路由/编码、PTY 输入、鼠标坐标和网格绘制；整程序回归覆盖接入/切换顺序、ATT、状态变化/新增/消失、回复、停止确认与取消、输出压力和退出恢复。原生 Queue 新增 RED→GREEN 覆盖公开数据/动作、选择/新增状态机、拒绝外部 UI 配置、禁止 Queue 按键进入 PTY。

### 边界与记录

- 按用户约束，未启动、attach、输入或停止任何真实 agent；真实 Claude Code 显示与真实 corral 生命周期未验证，不用合成测试代替该结论。
- 本机 Terminal GUI 控制被工具拒绝，没有绕过限制；交互验收在独立 PTY 中完成，没有录屏。
- 核对 CLI 时误将 `drover init --help` 当成帮助命令。确认创建时间及精确内容后，已删除它意外创建的 saddle 配置、空的 `--help` 交接目录和末尾新增登记项，并恢复 `.gitignore`；后续所有 drover 写入测试都使用隔离 HOME/临时项目。
- 无待用户拍板的实现问题。原生 Queue 只显示公开 JSON 已有字段；更细的判据数据若将来需要，应由 drover 增加公开 API，saddle 不读取内部文件补齐。
