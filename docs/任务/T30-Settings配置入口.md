# 任务：T30 在 saddle 内提供 Settings 配置入口

2026-09-28，用户已手动下放 TASK T30，并再次确认以本任务书为准正式启动委派。队列派发正文仍为旧占位稿，本版已确认方案取代其中待讨论及不实施的限制。

你是被委派的实现者，照本文件做，不再开其他 agent。

## 用户需求与确认（原话）

- 「再加一个任务，我需要把config抽到saddele的一个setting功能里面。」
- 「接下来应该是T30了。这个需要你先和我共同设计下，之后再委派」
- 「我同意你的建议，但是入口放在哪，需要明确一下，其他的没有异议」
- 在主控展示 Agents 顶部第二行右侧的入口、窄栏换行及逗号快捷键后：「可以。更新任务书，然后开工吧！」
- 准备任务书时明确边界：「等一下，你只改任务书，我来委派」。主控据此只写任务书，未派发。
- 用户随后手动下放 TASK T30；主控核对旧正文并询问是否按新版任务书正式启动，用户回复：「对。这个应该是你刚才改写的吧？」。现按本任务书实施。

下列内容是用户确认的共同设计，不作为伪造的逐字验收原话。

## 目标与范围

只改 saddle。Settings 是现有 `config.toml` 的界面编辑入口，继续使用同一个配置文件，让用户在 saddle 内查看、修改和保存已有设置。

不新增模型、agent 启动命令或其他配置能力；不改 corral、drover、corral-dispatch、全局技能或分派流程。布局状态文件仍与配置文件分开。

## 已确认的入口

Settings 固定放在 Agents 顶部第二行右侧，与 Attention 同行；不随 agent 列表滚动。第一行继续放 Agents 标题和 Tasks。

```text
Agents · 6                  Tasks · Running
Attention · 2                    Settings
─────────────────────────────────────────
agent 列表……
```

- 点击英文 `Settings` 打开独立设置弹窗，沿用现有按钮与弹窗样式。
- 左栏太窄、Attention 与 Settings 放不下时，Settings 单独占下一行，避免重叠。
- 快捷键为 `,`，仅在 Agents 获得焦点时打开设置；终端输入中的逗号照常传给终端。
- 关闭设置后回到打开前的焦点。设置弹窗期间，agent 和终端继续运行，设置操作不传入终端。

## 已确认的设置界面

```text
Settings
Config: ~/.config/saddle/config.toml

[ General ] [ Colors ] [ Advanced ]

Sidebar width         [ 52          ]
Refresh interval      [ 1000     ] ms
Initial project       [ Automatic   ]

                       [Cancel] [Save]
```

图中的路径和数值只是示意，实际显示当前使用的配置路径和值。分组如下：

| 分组 | 内容 |
|---|---|
| General | 侧栏宽度 `left_width`、刷新间隔 `refresh_ms`、初始 Tasks 项目 `queue.cwd`；未指定项目表示 Automatic，沿用现有启动选择规则 |
| Colors | 现有 `[colors]` 字段，按通用界面、Agents、状态、agent 类型等用途分组，提供色块和颜色值；保留现有颜色格式与语义 |
| Advanced | corral 命令名或路径 `corral`、drover 命令名或路径 `queue.drover` |

- 已无显示作用的 `left_split` 不放入设置界面，保留旧文件兼容。
- 颜色编辑提供小范围效果预览，不在编辑草稿时实时改变整个界面。
- 每项可恢复默认值，仍需点击 Save 才保存。
- 沿用英文界面。表单编辑、页签切换和滚动沿用现有交互；具体尺寸、长路径呈现及字段排版在上述线框范围内适配。

## 保存、生效与失败行为

- 修改先保留在草稿中，点击 Save 才保存；Cancel 不改变配置。
- 颜色和侧栏宽度保存成功后立即在当前 saddle 生效；终端内 agent 输出仍保持自己的颜色。
- 刷新间隔、corral／drover 命令路径和初始项目标注 `Restart required`，下次启动生效；不因保存而切换当前项目或运行中的后台连接。
- 顶部显示实际保存路径。启动使用 `--config PATH` 时保存到该文件；否则沿用绝对 `XDG_CONFIG_HOME` 下的 `saddle/config.toml`，或默认 `~/.config/saddle/config.toml`。
- 文件不存在时仍正常使用默认值，首次保存创建文件及缺失目录；省略项继续沿用现有默认语义。
- 保留原文件注释和未编辑内容，不把设置保存变成整份配置的无关重写。
- 配置值沿用现有校验，错误明确显示；保存失败保留草稿，不把未保存值当作已生效。
- 检测配置文件的外部修改，冲突时提示重新加载，避免覆盖其他编辑。重新加载会涉及草稿取舍，应明确呈现，不静默丢弃草稿。

## 设计文档与先读

主控已将用户确认的方案同步到 `docs/DESIGN.md` 第 47 节。第 8 节原有启动加载规则由第 47 节补充 Settings 保存与生效行为；不重新打开已经确认的设计决策。

实施时先读：

- AGENTS.md。
- 本任务书及 DESIGN 第 47 节；原配置规则见 DESIGN 第 8 节，侧栏与 Attention 相关规则见第 40、42 节。
- `src/config.rs`、`src/main.rs` 的配置来源，`src/theme.rs` 与根目录 `config.toml` 的现有字段，`src/app.rs`／`src/ui.rs` 的焦点、弹窗和 Agents 顶部入口。

## 实施安排

- 路由：常规 / 交叉审查要 / 影响面：碰要害。route.py 的 tier、cross_review、impact 均为 null；主控按已定方案的界面实现判断为常规，配置持久化及外部修改冲突可能覆盖用户数据，因此安排独立审查。
- 实现者：Claude Code 常规档 `opus[1m]` / `high`，启动名前缀 `saddle/dev-t30-settings`，role=implementer。实际 unique 名称及基线记录在主仓库 HANDOFF。独立审查在实现和主控审查后安排。
- 分支 `t30-settings`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t30-settings`，从包含本任务定稿的 main 建立。只在此 worktree 修改相关 saddle 代码、必要测试、使用说明和本任务完成记录；不改主仓库 HANDOFF 或审查文件。
- Cargo 使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 验证预算

遵守 AGENTS.md 的轻量 TDD：目标行为先有有效 RED，再最小实现与 GREEN。定向检查限本次设置编辑／取消／保存、立即与重启生效的区分、文件保留与失败／外部修改冲突、入口及弹窗输入隔离等直接影响路径，由实现者选择具体检查。

标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，另做 fmt 和 diff 检查。失败后只补跑受影响检查，不重复无关全套，不要求录屏或覆盖矩阵。全部使用临时配置、状态目录、合成数据与假 CLI。

## 不要做

- 不读取、改写用户真实配置或布局，不操作用户真实 agent、队列或 saddle socket，不改上游内部文件。
- 不增加全局配置热监听、多主题体系、字体字号设置或其他未确认功能，不扩大到 T29／T28／T31。
- 不在保存时自动重启 saddle、agent 或已有终端，不安装发布或重启用户现场。
- 不按名称／路径批量杀进程；不合并、不推送，只在正式分配的分支提交。
- 需要改变已确认方案时报告主控，不自行扩展。

## 实施完成后

在本文件追加「## 完成记录」并在分配分支提交，简述改动、验证、取舍和未做事项，回复提交 SHA 及需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

实现者：saddle/dev-t30-settings（Claude Code），分支 `t30-settings`。

### 改动

- 新增 `src/settings.rs`：Settings 弹窗（General／Colors／Advanced 三页），草稿编辑、恢复默认、保存、外部修改冲突处理与绘制。`src/theme.rs` 把颜色解析抽成 `parse_color`，增加反向的 `color_name` 与按键名访问的 `Theme::named_mut`，原有颜色格式和报错文字不变。
- 入口：`src/ui.rs` 在 Attention 行右侧画 `Settings`（点击等同 `,`）；栏宽放不下「Attention · 99 loading…」加 Settings 时，Settings 单独占下一行，分隔线和列表随之下移一行。打开期间入口高亮，只有弹窗可点。
- `src/app.rs`：Agents 焦点下 `,` 打开；点击入口记住打开前的焦点（Agents 或 Viewer），关闭后回到该焦点。弹窗期间键盘、粘贴、鼠标全部交给 Settings，不进终端；终端中的 `,` 走原终端路由不受影响。保存成功后立即换用新颜色（按终端色深转换）和侧栏宽度，其余设置不动运行中的状态，状态栏提示需重启的项。`src/app_control.rs` 在 Settings 打开时对 ctl 请求返回 busy，与 Search 一致，避免焦点被外部请求改走。`src/main.rs` 把实际配置路径传给 app。
- 保存：新增依赖 `toml_edit 0.25`（与现有 toml 1 共用解析层，锁文件只多一个包）。只改被编辑的键，保留原值后的注释和键位置，其余文本原样；初始项目为 Automatic 时删掉 `queue.cwd`。整份结果再过一遍现有 `Config::parse` 校验。写入走同目录临时文件再替换；配置是符号链接时写到其目标并保留原文件权限。首次保存创建文件和缺失目录。
- 使用说明：README.md、README.zh-CN.md 增加 Settings 说明和快捷键行，修正「颜色只在下次启动生效」的说法；`--help` 增加 `,` 说明。

### 验证

- TDD：`tests/settings.rs` 先写 7 条行为测试，桩实现下全部因行为失败（RED），实现后通过；`tests/ui.rs` 新入口测试先失败后通过；`tests/app.rs` 新增 PTY 端到端测试（假 corral／drover，临时 HOME、配置和状态目录）：`,` 打开、编辑并保存后文件只改一处且侧栏宽度立即变化、焦点回到 Agents、Esc 不写文件。用临时注释掉「立即应用宽度」一行确认该测试会失败（42 vs 50）后恢复。
- 之后补充：无改动 Save 直接关闭不写文件、各页与冲突提示在 0–23×0–13 等窗口绘制不崩、经符号链接保存保留链接与 0600 权限（这三条在实现后补写，未单独做 RED）。
- 设计样例测试 `design_sample_fits_fifty_columns_without_wrapping` 第二行按第 47 节改为带 `Settings`。
- `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 通过；`cargo fmt --check`、`git diff --check` 干净。未接触用户真实配置、布局、agent、队列或 saddle socket。

### 取舍

- 页签快捷键用 F1／F2／F3（与 New 表单 F2／F3／F4 的写法一致），Tab／↑↓ 在当前页内选项，Ctrl-U 清空，Ctrl-D 恢复默认，Ctrl-S 保存，Esc 取消；滚轮移动选项（同 New 表单）。按钮沿用弹窗底部紧凑样式，左对齐：Default、Cancel、Save。
- 颜色项标签直接用配置键名（`bg`、`agents_bg`…），便于和 config.toml 对照；分组为 Interface、Agents panel、Status、Agent types。输入框左侧色块显示草稿颜色，无效值显示 `??`。Colors 页底部 4 行预览（Agents 栏、选中 agent、弹窗文字与按钮、状态色与回复格式）只用草稿颜色，不影响界面其余部分。
- 已编辑未保存的项前标 `•`。数值项前置检查为整数，颜色项按现有颜色格式检查并切到出错项所在页；其余沿用 `Config::parse` 的报错。显示值由解析后的配置生成（如 `reset` 显示为 `default`、`#E2835E` 显示为小写），未改动的项不写回，文件原文不受影响。
- 外部修改只在保存时比对（读取后的文件全文 vs 磁盘现状），不做持续监听。冲突时不写文件，给出三选一：Keep my edits（重读文件、保留已编辑项为草稿，需再次 Save）、Discard my edits（按文件现状重载）、Back（回到草稿不重载）。打开时文件无法读取或无效，只显示错误，可 Reload 或 Cancel，不提供编辑。
- 弹窗高度：General／Advanced 13 行、Colors 34 行、冲突／错误提示 18 行，均受窗口限制；路径过长从左侧截断保留文件名，`$HOME` 显示为 `~`。
- 无改动时 Save 等同关闭，不写文件、不显示「已保存」。

### 未做／需主控决定

- 未在 DESIGN.md 追加内容（任务书只允许改代码、测试、使用说明和本记录）；上述取舍如需进 DESIGN 第 47 节，请主控决定。
- 冲突检测只在 Save 时进行，打开期间不提示外部修改；若需要打开期间也提示，需要另行确认。
- 快捷键 F1–F3、Ctrl-D 为实现者选择，未经用户确认。

## 第 1 轮定向返工（R1／R2）

按主控「独立意见裁决与第 1 轮定向返工」只修两项，S1 未纳入，其他功能不变。基线 `857f154`。

- R1 重载失败后丢草稿：`Settings` 记下失败那次重载是否保留草稿（`keeping`），错误提示下的 Reload／Ctrl-R 按该选择重试，不再固定走丢弃。失败时不推进基线，已编辑项仍按原基线判定。首次打开出错和 Discard 的语义不变，都是 `keeping = false`。错误提示在确有保留草稿时加一句，说明 Reload 后草稿仍叠加在文件上、Cancel 才丢弃。
- R2 覆盖悬空符号链接：`write` 中 `canonicalize` 失败时，若配置路径本身是符号链接，就报保存失败（"symbolic link whose target cannot be resolved; the link is kept"）。Save 返回 Stay，保留链接和草稿，不再落回链接路径替换目录项。普通首次创建（路径不存在且不是链接）和有效链接保存照旧。
- RED／GREEN：`tests/settings.rs` 新增 `a_failed_reload_after_keep_my_edits_still_keeps_them_on_retry`（按审查探针合成：Keep → 文件无效 → 修复 → Ctrl-R，修复前得到 52 而不是 60），以及 `a_dangling_config_link_is_kept_and_saving_reports_failure`（修复前 Save 返回 Saved）。两条都先因目标缺陷失败，修复后通过。
- 限定回归：`cargo test --test settings`（12 条）、`cargo test --test app settings_open_with_comma_save_to_the_file_and_resize_the_sidebar_at_once -- --exact` 通过。`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 干净。未跑全套，未改主仓库，未动审查探针。

## 主控最终审查（2026-09-28）

- 主控与独立审查通过；首轮 R1/R2 经修复及同一审查者复核关闭，必须改 0。首轮取舍均获认可，详见 [主控审查](T30-主控审查.md) 与 [独立审查](T30-独立审查.md)。
- 主控首轮标准测试 242 passed／0 failed／2 ignored，Clippy／fmt／diff 通过；返工主控 Settings 12 项及 app 1 项通过，独立复核 Settings 12 项通过。合并后 src、tests、Cargo.toml、Cargo.lock 与审查提交一致，不重复无关全套。
- 实现 `85ead03` 已合入 main，合并提交 `5b88932`。保留非阻塞 S1：长配置路径可能挤掉单行保存错误的原因文字，失败本身仍明确、草稿保留；本轮不扩展处理，也不自动新增任务。

## 发布与清理（2026-09-28）

- 合并后 main 使用共享 target 的 `cargo build --release` 通过；已同步仓库 release。共享 `../saddle-worktrees/.target/release/saddle`、仓库 `target/release/saddle`、默认 `~/.local/bin/saddle` 三入口 SHA-256 一致：`61a169b9fb6db2faab8e35d999777c9053d6c2e2f8822bb8151d6edb7851bd7c`，默认入口仍链接共享 release，`--help` 已核对 Settings 入口。未重启正在运行的用户 saddle，下次启动使用新版本。
- 清理前再次核实两 agent 均 idle、attached=0，worktree 干净且最终提交已合入 main；实现和独立审查 worktree、实现分支均已删除，住在其中的两个自建 agent 一并关闭：`saddle/dev-t30-settings-1`（acef3e8b5939）、`saddle/dev-t30-review-1`（3bbf22741dd6）。原有用户 agent 保留，记录已落盘。
