# New Agent：主控固定 main，普通 agent 自由命名

## 最新状态：角色方案已确认（2026-09-27）

用户已回复「对」，确认默认主控、名称锁定 main；选普通 agent 后自由填写名称。继续在 b015f42 上补齐角色选择，不另开分支或 agent。旧方案已静态初审，尚未主控最终标准检查或合并；后续按本任务最新范围审查。

2026-09-27，saddle/main 交给 saddle/dev-exact-name（Codex，常规档：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由档位拿不准，主控选常规；交叉审查不要，影响面改行为）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 34 节。
- src/launch.rs 及直接相关现有测试。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/new-agent-exact-name，分支 new-agent-exact-name（从 main 建好）。
- 只动 New 的角色/名称选择及必要布局、直接受影响的测试及中英文 README、本任务完成记录。

## 要做的
按设计第 34 节，在 New 普通表单增加 Controller / Regular 角色选择，沿用现有描边按钮，放在名称输入框前。默认 Controller，名称显示只读 main；Regular 的名称允许编辑。角色与 Codex/Claude 独立，不自动开启派发或修改队列配置。沿用 b015f42 的精确名称规则：不加 --unique，重名通过公开 CLI 失败反馈并保留草稿，不新增编号、重试或改接策略。保持现有表单风格和短窗口可达性。

## 怎么算做完
> 现在有一个问题，new agent的时候，如果不改默认的main，但是出来的agent的名字是main-1.你来调查一下。
>
> 关键是现在有些工作流依赖主控的名字是main
>
> 能不能这样，有一个选项。选主控就是写死main，如果不是就可以随意写名字。你觉得如何？

用户已确认：默认主控、名称锁定 main；选普通 agent 后自由填写名称。

验证预算：新增角色行为的定向自动检查先 RED 再 GREEN，运行受增量影响的表单/工作流检查、Clippy 和 `git diff --check`；旧方案标准检查已跑，本轮不重复全套。命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不改变派发流程、Codex YOLO 默认、Claude 默认、打开位置；除角色/名称控件所需布局外，不改 UI 样式或其他功能。
- 不操作现有 main-1 或任何真实 agent、队列和其他仓库；测试使用假 CLI/合成数据。
- 不按项目名或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。
- 已知 full_workflow 旧终端鼠标坐标失败不扩修；其他范围外问题记录报告。

## 做完
本文件末尾追加完成记录并提交：改动、验证、取舍、未做事项。回复带提交 SHA 和待主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（2026-09-27）

- 根因与改动：`Form::args()` 对未手改名称追加 `--unique`，使默认 `main` 交给 CLI 自动编号。现已删除这处分支，默认和手工名称都精确传递；名称提示统一为 `Exact name · edit freely`。同步中英文 README，修正旧的项目/agent 建议名说明。
- 测试改动：现有默认名称测试验证精确 argv、预览与名称提示；项目/Claude 切换测试和假 CLI 工作流的默认提交预期移除 `--unique`。保留手改名称保护和公开返回名处理的原有检查。
- 验证：以下 Cargo 命令均加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，逐条在前台等待结束。
  - 初次误用 `cargo test --bin saddle launch::tests::default_project_and_codex_create_exact_main_without_typing_a_command -- --exact`，运行 0 个测试，不计作验证。
  - RED：`cargo test --lib launch::tests::default_project_and_codex_create_exact_main_without_typing_a_command -- --exact`，1 个失败；实际 argv 多出 `--unique`，符合目标缺陷，退出码 101。
  - GREEN：`cargo test --lib launch::tests::`，6 个通过，包括默认精确名称、预览及手工名称保护。
  - `cargo test --all-targets`，执行一次，退出码 101；此前各测试目标通过，workflow 为 36 通过、1 失败、2 忽略。默认 New 的假 CLI 提交、手工名称、预览及失败草稿保留测试通过。唯一失败为任务已注明的 `full_workflow_routes_input_switches_safely_and_survives_disappearance`：旧鼠标坐标发送后，等待 `input p/a 1b5b3c303b333b324d` 超时；未扩修或重跑全套。
  - `cargo clippy --all-targets -- -D warnings`，执行一次，通过。
  - `git diff --check`，通过。
- 取舍与未做：遵循设计第 34 节，无需变更设计；沿用现有 CLI 失败反馈与草稿保留，不新增重试、编号或接入已有 agent 的策略。不改派发、命令默认值、打开位置或 UI 样式；未操作真实 agent/队列、未构建 release、未合并或推送。
- 待主控：审查本分支，决定后续合并；已知 full_workflow 旧鼠标坐标问题另行安排。

## 角色方案增量完成记录（2026-09-27）

- 本轮确认：用户确认 Regular 首次名称为可编辑的 `main`，切到 Controller 显示锁定 `main`，切回 Regular 恢复此前填写的名称草稿。
- 改动：名称前新增 Role 的 Controller / Regular 描边按钮，默认 Controller；主控名称不进入可编辑焦点，不接受点击编辑、清空、字符键或粘贴。Regular 沿用原名称输入和校验，角色切换保存其输入状态；项目和 Codex/Claude 切换不覆盖角色或名称。名称继续精确提交，不加 `--unique`。
- 布局与说明：复用现有按钮、输入框、焦点和滚动方式，只为 Role 增加四行表单高度；支持 Tab、方向键、Enter/空格和 F6/F7 选择角色，短窗口仍通过焦点滚动访问控件。同步中英文 README。
- 定向 RED → GREEN：先增加 `controller_name_cannot_be_edited_by_click_keys_or_paste`，以 `cargo test --lib launch::tests::controller_name_cannot_be_edited_by_click_keys_or_paste -- --exact` 运行，旧实现实际名称为 `renamed`、预期 `main`，退出码 101；实现后同一命令 1 个通过。
- 增量验证：以下 Cargo 命令均加共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，逐条前台等待结束。
  - `cargo test --lib launch::tests::`：8 个通过，覆盖只读名称、普通名称草稿与校验、项目/工具切换、精确 argv/预览、描边样式和 40×12 短窗口输入可达性。
  - `cargo test --test workflow -- new_form_shows_bordered_inputs_and_click_positions_a_visible_cursor new_agent_ placement_cancel_and_escape_never_attach_and_new_cancel_keeps_the_draft closing_a_start_target_keeps_the_created_agent_available_without_attaching starting_in_a_hidden_tab_preserves_focus_and_exit_detaches_every_tab`：6 个通过，使用假 CLI；覆盖默认主控创建、普通名称编辑及角色往返切换、失败草稿、取消重开、关闭启动目标和隐藏标签启动。
  - `cargo clippy --all-targets -- -D warnings`：通过。
  - `git diff --check`：通过。
- 取舍与未做：角色仅限制 New 名称，没有新增派发或队列配置；保留 b015f42 的精确名称语义、公开返回名和错误处理。未改变 YOLO、Claude 命令默认值或打开位置，未操作任何真实 agent/队列，未重跑全套、未扩修已知 full_workflow 问题、未构建 release、未合并或推送。
- 待主控：审查增量提交并决定合并；没有待确认的实现规则。
