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

## 完成记录

2026-09-26，saddle/dev-t2-new-form 完成；待主控审查，本分支不合并、不推送。

### 改动

- New 默认当前 Tasks 项目和 Codex，可直接点击 Create agent；项目选择列表及 Edit path 分开呈现，另选目录不改变 Tasks 项目。可切 Codex／Claude，名称自动建议且支持手改；建议名带 `--unique`，手改名精确提交，实际创建名继续读取 start 返回结果。
- 四个文本框加入标签、框线、占位示例、焦点高亮及真实插入光标。支持点击定位、左右键、Home／End、Backspace／Delete、Ctrl-U 与光标处粘贴；中文宽字符第二格点击归到字符前，不破坏 UTF-8。首条消息支持多行、上下移动及 Enter 换行。
- Advanced 默认折叠，收纳完整命令、首条消息、六项打开位置及完整预览；明确选择 Codex／Claude 才重置命令，手填值标示 Custom command。收起／展开和无关焦点切换保留高级值。
- 小窗口随焦点／滚轮显示字段，底部固定创建／取消；无效字段就地提示，提交时定位到字段。New 打开时不显示底层 Viewer 光标。保留异步 start 接线、忙时编辑／提交保护及失败草稿。
- 修改 New 所需的 launch/app/ui，新增仅供 New 使用的 `src/launch_edit.rs`；更新相关检查、中英文 README 和 DESIGN 第 27 节本次修订取舍。

### RED → GREEN 与验证

命令使用共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，均前台等待结束。

1. `cargo test --lib launch::tests::text_edits_at_the_cursor_without_corrupting_wide_characters -- --exact`：RED 得到 `a中b文`，预期 `a中文b`，证明原实现忽略左移而末尾追加；加入插入位置编辑后 GREEN，继续验证中文删除、Home／End。
2. `cargo test --test workflow new_form_shows_bordered_inputs_and_click_positions_a_visible_cursor -- --exact --nocapture`：RED 为点击后终端光标仍隐藏；修复后 GREEN，并扩充为下面的完整可观察合成终端检查。
3. `cargo test --lib launch::tests::default_project_and_codex_can_create_without_typing_a_command -- --exact`：RED 为默认名称为空、无法构造调用；自动名称／默认命令实现后 GREEN。
4. 局部检查通过：项目与 agent 选择、手改名称保护、高级值折叠后保留、多行编辑及滚动点击、40×12 下四个输入框和创建／取消入口可达。假 CLI 工作流验证默认创建、选择其他项目及 Claude、Tasks 项目不变。原有工作流只调整新表单操作步骤，保留原断言。
5. 标准 `cargo test --all-targets` 一次通过：126 passed，2 ignored（原有要求安装 drover 的集成测试）；其中失败重试草稿、重复提交、关闭启动目标、隐藏 tab 启动及其他异步隔离回归全部通过。
6. 标准 `cargo clippy --all-targets -- -D warnings` 一次通过，无警告。最终 `git diff --check` 通过。

### 实际输入框／光标观察

通过真实 saddle 子进程、PTY、vt100 解析屏幕和假 corral/drover，在 140×40 合成终端直接观察：默认页显示 Project、● Codex／○ Claude、建议名称、Advanced、Create agent／Cancel；Name 输入框聚焦后为粗框 `┏…┓ / ┗…┛`。将内容改为 `a中b`，点击“中”的第二个终端格，光标可见并落在其前（该次屏幕 1 起算第 17 行、第 22 列）。再插入“文”、右移、Delete，界面得到 `a文中`；编辑阶段没有 start，点击 Create 后假 CLI 收到名称 `a文中` 和默认 `codex`，并显示返回的 `a文中-actual READY`。检查打印实际屏幕及 argv，可用上述单测命令复查。

另一路默认创建未手改名称，假 CLI 收到 `project-one/codex --unique -- codex`；从 Project 选 project-two、点击 Claude 后收到 `project-two/claude --unique -- claude`，Tasks 仍显示 project-one。

### 取舍与未做事项

- 保持终端原生样式与现有主题／按钮；只做四个 New 字段的局部编辑辅助，不搭通用表单框架，不新增依赖。
- 长行横向滚动、不软换行；多行消息按真实换行纵向滚动。Tab 字符显示四格。自动名目录末段仅保留 ASCII 字母、数字、下划线和连字符，其余替为连字符，空结果回退 project。
- Advanced 内预览可滚动；失败错误显示末尾最多三行以露出具体原因。小窗口验证限一项 40×12 自动检查，未做多尺寸矩阵或录屏。
- 未启动／操作真实 agent，未修改真实队列、用户配置、安装版本，未读 corral/drover 内部文件或仓库；测试只用临时目录与假 CLI。
- 未改 terminals/viewer/pty 状态机、其他表单、Tasks、tab/split 或 HANDOFF；未增加模型／effort／权限选择器。
- 无待主控裁决的设计分歧；按任务约定交主控审查，不合并、不推送。
