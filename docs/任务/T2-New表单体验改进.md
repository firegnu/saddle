# T2 后续：New agent 表单体验改进

2026-09-26，saddle/main 交给 saddle/dev-t2-new-form（Codex，常规：gpt-6-astra / high；沿用用户 T2 指定 Codex）。
路由：常规 / 交叉审查不要 / 影响面：改行为（route.py：常规，交叉审查拿不准，影响面拿不准；主控判断：只改表单显示/编辑/参数选择，不改 PTY 并发和生命周期）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话 / 验收

> 我看了一下你的实现，我觉得有些和我想象的有点出入。先从new agent开始，我都不知道怎么去操作那个弹出的对哈框。

主控提出「选项目 → 选 Codex / Claude → 点击创建」，名称自动生成并可修改，输入框、选项、按钮明确，完整命令放高级设置，用户回复：

> 继续吧。

用户随后强调：

> 还有输入框之类的体验，现在的很不好，不注意看都不知道是个输入框。而且点击输入框连个光标都没有

## 先读

- AGENTS.md。
- docs/DESIGN.md 第 27 节末尾「T2 用户体验修订：New 表单」；仅按需要参考同节已有行为。
- src/launch.rs 及 src/app.rs、src/ui.rs 的 New 接线，相关测试。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t2-new-agent-form
- 分支：t2-new-agent-form，主控从 main 建好。
- 只改 New 所需的 src/launch.rs、src/app.rs、src/ui.rs，必要的局部编辑辅助模块及 lib 声明、对应 tests/、确需的 Cargo 依赖、中英文 README、本任务完成记录和 DESIGN 第 27 节本次修订下的实现取舍。不要改 HANDOFF 或旧审查/任务记录。
- Cargo 命令一律 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 要做的

按设计修订完成可直接操作的 New 表单：明确的项目选择与路径编辑、Codex/Claude 选项、自动建议且可修改的名称，Advanced 收纳原有命令/首条消息/打开位置/预览；只在显式创建时走原有 start 请求接线。

输入框体验优先：框线、标签、占位示例、焦点高亮和可见插入光标；点击定位、键盘移动、插入/删除和粘贴正常，中文宽字符对齐，首条消息支持多行。沿用主题和英文界面，保持原生终端风格。小窗口也要能访问字段和创建/返回入口；不要为这些局部字段搭通用表单框架。

具体简单实现取舍自行记录；如需改 tab/split、PTY 生命周期或扩展其他仓库接口，先报告，不自行扩大范围。原本所有启动失败保留草稿、重复提交保护、异步目标隔离都保留。

## 验证预算

- 按 AGENTS.md：先加入直接体现本次输入/表单行为的自动化检查，确认因目标缺失而 RED，再最小实现到 GREEN。已有流程测试可调整交互步骤适应新表单，保留其原有行为断言，不删除回归。
- 对当前界面做一条直接可观察的合成终端检查：看输入框轮廓、点击后的光标和实际编辑结果，并从默认选择走到假 CLI 创建。说明所见结果；不只验证最终能执行 start。无需录屏、真实 agent 或多尺寸覆盖矩阵。
- 完成后标准检查各一次：共享 target 的 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`，以及 `git diff --check`。不做无关测试或缺陷注入。

## 不要做

- 不启动或操作真实 agent，不修改真实队列、用户配置或安装版本；不读 corral/drover 内部文件或其仓库。测试使用假 CLI、临时目录和合成数据。
- 不改 src/terminals.rs、src/viewer.rs、src/pty.rs 的接入状态机；不重做其他弹框、tab、split、Tasks，不加模型/effort/权限独立选择器，不新增配置/持久化/自动 worktree 功能。
- 不按项目名/路径批量杀进程，自己启动的进程按记录 PID 停止。
- 不合并、不推送，只在 t2-new-agent-form 提交。

## 做完

在本文件追加「## 完成记录」并提交：改了什么、RED/GREEN 与验证、实际输入框/光标观察、取舍、未做事项。回复这些内容和提交 SHA、是否有待主控裁决的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
