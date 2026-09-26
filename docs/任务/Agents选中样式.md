# Agents：选中条目样式改进

2026-09-26，saddle/main 交给 saddle/dev-agent-selection（Claude Code，常规：opus[1m] / high，负责终端视觉样式）。
路由：常规 / 交叉审查不要 / 影响面：看得见（route.py：档位拿不准，交叉审查不要，看得见；主控按小范围视觉设计取常规）。
你是被委派的 agent，不再开 agent。当前只做第一阶段样式建议，不写功能代码；待主控通知再在同一 worktree 实现。

## 用户原话 / 验收

> 再排一个agent去修一下agents区域中被选中agent的样式的修改需求，现在的就是一个背景色。而且最左边还有一个小白色竖线，我觉得不好，白色数显是不是可以去掉。另外考虑一下，如果选中的那个北京最左边显示一个颜色长条会不会好看亦或者会乱。现在的截图如下

截图是用户提供的 `/Users/firegnu/Desktop/SCR-20260926-ouuj.png`，只读查看，不复制进仓库。这里的「白色数显」「北京」按上下文分别理解为白色竖线、背景；源码已核对首行选中标记为 `▎`。

## 先读

- AGENTS.md。
- docs/DESIGN.md 第 13、14 节及 Agents 样式相关补充；不要重开已定布局/数据规则。
- src/ui.rs 的 draw_agents / agent_rows 选中条目绘制，src/theme.rs 中已有语义色，相关 tests/ui.rs。
- 用户截图，重点看 selected main 的整块背景和仅首行最左侧的小白线。

## 工作位置与时序

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/agents-selection-style
- 分支：agents-selection-style，主控从 main 建好。
- 另一个 agent 正在 ../saddle-worktrees/t2-new-agent-form 改 New/Open，也涉及 src/ui.rs；不得在它合并前并行修改功能代码。当前唯一可写文件为本任务文件，在自己分支末尾追加建议并提交，不改其他 worktree 或主仓库。
- 第一阶段完成后保持 idle，等待主控将本分支接上合并后的 main，再明确通知进入第二阶段。不要自己合并/rebase、推送或提前实现。

## 第一阶段：看图给出具体建议

- 去掉用户不喜欢的首行小白色竖线。比较两个简单选择：只保留更明确但克制的背景/文字选中层级；或在选中条目背景最左侧加一条贯穿该条目所有可见行的柔和主题色竖条。
- 说明哪种更适合当前截图：选中识别、与树形连线/状态颜色/已接入标记的关系、是否显乱。给一个推荐，不扩展到全界面重新设计。
- 推荐方案要可直接实施：位置、宽度、长度、使用哪些已有颜色、Agents 有/无焦点时如何保持可辨识、长条与选中项多行/折行及裁剪的关系。可用紧凑文字线框或色块说明，不需要网页、图片生成或复杂原型。
- 复用已有主题色，不新增配置体系。不动 effort、agent 状态或 Git 摘要的语义色，不把选中与已接入混为一谈。

## 第二阶段边界（尚未授权开始编码）

收到主控明确实施通知后，只改选中条目视觉：src/ui.rs 对应绘制、必要的已有主题引用、直接相关渲染断言、DESIGN 中该样式规则和本任务完成记录。不修改选中/滚动/焦点/接入行为，不覆盖刚合入的 New/Open 改进，不顺手重构其他样式。

这是纯视觉，不要求伪造失败测试。预算为 git diff --check 加一项直接体现选中前后和多行条目的合成渲染检查；不跑全套、不录屏、不启动真实 agent、不做大矩阵。若需要 Cargo，使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做

- 不操作用户 agent 或真实队列，不读 corral/drover 内部文件，不改其仓库，不改用户配置，不更新 release。
- 不按名字/路径批量杀进程，不在其他 worktree 写文件；不合并到 main、不推送、不再委派。

## 本轮完成

在本文件追加「## 样式建议」并提交到 agents-selection-style：两种方案比较、明确推荐及理由、可实施细节。回复建议和提交 SHA，明确当前仅建议阶段，最后一行 DONE。所有命令前台完成。
