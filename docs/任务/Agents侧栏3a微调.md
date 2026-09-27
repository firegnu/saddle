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

重要已核对语义：C 实际表示领先基准，显示 ↑n，不是 ↓n/pull。ATT 是公开连接数；本机标记依据 saddle 自有连接。effort 暂按第 40 节放到展开信息行；不得擅自删掉旧的委派强度信息。TUI 不能表达 2px 按字符格最小适配，图片图下 Tweaks 不扩成产品面板。

## 怎么算做完
用户原话：
> cool，现在还有一个任务。重新对左侧agents区域按照对应的设计，进行ui的微调。给你的设计是下面两个文件：
> /Users/firegnu/Desktop/export/Agents\ Panel\ Spec.md
> [Image #1]
> 这两个文件就是微调设计稿。

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
