# T7：Agents 委派 effort 图标

2026-09-26，saddle/main 交给 saddle/dev-t7-effort（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规，交叉审查与影响面拿不准；新增公开元数据接入及显示，不改生命周期或核心规则，主控审查）。
你是被委派的 agent：照本文件做，不再开别的 agent。

## 先读

- AGENTS.md；docs/DESIGN.md 第 3、6、13、15 节的公开接口和 Agents 显示约定。
- src/corral.rs、src/ui.rs 的 Agent 数据接入及 agent_rows；tests/corral.rs、tests/ui.rs 的相关检查。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t7-effort，分支 t7-effort，已从 main 建好。
- 范围：Agents 的公开 labels 元数据读取、effort 图标显示、直接相关测试与文档、本任务记录。
- 所有 cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 要做的

用户先要求评估，随后将范围缩小为只显示委派时明确指定的 effort。corral/main 已完成公开标签接口（对方报告提交 ee45230）；用户现已通知 saddle 接着按派活流程实施，原评估等待阶段结束。

用户转达的已确认范围与要求：

> 只显示主控委派时明确指定的 effort；主控本身和用户手动开的 agent 没有标签，显示为未知或不显示图标。
>
> 读 corral ls / status 的 labels.effort，映射成类似 Wi-Fi 强度的图标（medium/high/xhigh 三档，其他值或没有这个键按未知处理）；不解析 argv、不读 corral 内部文件、不从终端输出推断。

公开接口约定：corral ls、status 的每个 agent 有 labels 对象，键值均为字符串；未传标签为 {}，同名重开不继承旧标签。旧版输出缺少 labels 也按未知。saddle 不推断默认档位，不将其称为运行时实际 effort、token 或负载。

这是现有 Agents 条目的小幅显示补充，图标位置、三档外观及未知采用不显示或明确未知，由你在现有布局和主题内简单决定；先写入 DESIGN 再实现。同步必要的中英文 README 说明。只用 effort，不扩展 model 展示或新的配置体系。

## 怎么算做完

> 只显示主控委派时明确指定的 effort；主控本身和用户手动开的 agent 没有标签，显示为未知或不显示图标。
>
> 读 corral ls / status 的 labels.effort，映射成类似 Wi-Fi 强度的图标（medium/high/xhigh 三档，其他值或没有这个键按未知处理）；不解析 argv、不读 corral 内部文件、不从终端输出推断。

验证预算：按项目轻量 TDD 对新增数据行为先取得真实 RED 再 GREEN；纯视觉不编造失败测试，以相关合成渲染检查观察结果。针对性检查及项目标准 cargo test --all-targets、cargo clippy --all-targets -- -D warnings 各一次，另做 git diff --check。只使用假 CLI 和合成数据，不做录屏、覆盖矩阵或缺陷注入；失败时可运行修复所需的相关检查。

## 不要做

- 不解析 argv、终端输出、任务文件来猜 effort，不读 corral 内部文件，不改 corral/drover 仓库。
- 不操作用户 agent，不改真实配置、队列或登记文件，不为测试启动真实 agent；不改现有状态判断、排序、接入和操作语义。
- 不开始 T8、T4、T2，不修无关问题，不引入复杂架构。
- 不按项目名或路径批量杀进程；自己起的进程只按记录的 PID 停止。
- 公开接口不足时报告具体缺口，不绕过边界。
- 不合并 main、不推送、不更新 HANDOFF；只在 t7-effort 分支提交。

## 做完

在本文件末尾追加「## 完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复同样列这些和需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：`Agent` 增加公开 `labels`（缺失按 `{}`），`Agent::effort()` 只把 `labels.effort` 为 `medium`/`high`/`xhigh` 的映射成三档，其余（无键、其他值、大小写不同、非字符串）为未知。`status` 的 labels 随其他字段覆盖 `ls`；跳过 status 的 starting/incompatible 条目用 `ls` 的 labels。Agents 主行在类型列（窄窗在名称列）后、状态前加 3 列 `▂▄▆` 信号图标，亮格数 1/2/3，亮格正文色、暗格树线色；未知留白对齐；整个列表都没有已知 effort 时不占列，原布局不变。先写 DESIGN 第 23 节，同步中英文 README。
- 验证了什么：tests/corral.rs 新增假 corral 用例，先以 `effort()` 桩取得断言 RED（`left: None, right: Some(Medium)`），实现后 GREEN。tests/ui.rs 新增合成渲染检查：无标签时无图标；160/80 列下三档亮格数与颜色、未知/其他值无图标、各行图标同列。`cargo test --all-targets` 全过，`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check`、`git diff --check` 干净。未启动真实 agent。
- 拿主意的地方：图标放主行类型列后固定列宽，便于纵向扫读；未知选「不显示」而非「?」，与「主控/手动开的 agent 没标签」的常态一致，不制造噪音；有已知 effort 时名称预留宽度 +4 列（33→37），只在需要时生效；effort 值区分大小写，按接口字面匹配；亮格用中性正文色，不复用状态语义色。
- 没做的事：不显示 model，不新增配置项/颜色字段，不在附加信息行加文字说明，不改状态、排序、接入和操作语义；未合并 main、未推送、未更新 HANDOFF。

## 主控审查

- 2026-09-26：可以合并。核对 45f95d7 全部 diff、完成记录及回复，只接入公开 labels.effort 并增加三档显示，符合已确认范围；未解析 argv/终端或读取内部文件，未改变已有状态、排序和接入语义。
- 主控复跑 cargo test --all-targets：79 passed、2 ignored；cargo clippy --all-targets -- -D warnings 和 git diff --check 通过。解析检查及 160/80 列合成渲染检查通过，无需返工。
- 同意图标位置、未知留白、全无已知值时不占列、有图标时名称少四列、精确小写匹配和中性正文色的取舍；未知不显示已获用户授权，无需再请用户决定。
- 同意不兼容 labels 整体为 null/非对象的畸形输出：公开契约保证对象，当前缺失字段可兼容，effort 的非字符串或其他值按未知；不为契约外输入增加处理。
- 无阻挡项；release 更新后需用户重启当前 saddle，当前运行实例不动。
