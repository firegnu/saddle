# Working 动画与 effort 区分

2026-09-27，saddle/main 交给 saddle/dev-working-mark（Claude Code，轻档 sonnet / medium，role=implementer）。
路由：轻 / 交叉审查不要 / 影响面：看得见（route.py：tier拿不准、cross_review不要、impact看得见；主控定轻：删除一处既有动画，视觉规则已经明确）。
你是被委派的 agent，不再委派。

## 先读

AGENTS.md、docs/DESIGN.md 第40节末尾「Working 动画与 effort 的视觉区分」，src/ui.rs 状态行绘制、tests/ui.rs 相关渲染断言。

## 范围与目标

worktree /Users/firegnu/Developer/personal_projs/saddle-worktrees/working-mark；分支 working-mark。
仅改 src/ui.rs、直接受影响的 tests/ui.rs 与 README 中英文及本任务完成记录。去掉 working 文字前的紫色盲文动画，保留首列蓝色圆点旋转和 working 文字。effort 原位置、字形、颜色、未知留空和折叠显示保持，状态列与时间列仍对齐；无需删配置项或顺带重构。

## 用户确认

「我确认了。就是那个紫色的动画引起的。不working的时候，就正确显示effort了」。此前主控建议去掉 working 前的紫色动画，保留最左侧转动圆点，effort 位置和样式不动；按 DESIGN 第40节执行。

## 验证预算

纯视觉：git diff --check，加直接观察上述显示结果的渲染检查；运行直接受影响的 UI 检查即可，不伪造 RED，不跑全套/交叉审查/录屏/缺陷植入。共享 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target，命令前台完成。若旧动画专属断言需要调整，保留圆点旋转及 effort 的原断言，不删相邻无关检查。

## 边界

不改 agent 生命周期、输入、状态判定、模型参数或 effort 解析；不新增模型名显示。不改原稿、DESIGN、其他代码/测试或配置；不操作用户 agent/现场TUI/socket/真实队列，不读 corral/drover 内部文件，不改其他仓库。不批量杀进程。不 merge/push/release，不写 main，只在本分支提交。

## 做完

本文件末尾追加完成记录，写改变、验证与取舍，提交并回复 SHA。全部命令前台结束，回复最后一行 DONE。
