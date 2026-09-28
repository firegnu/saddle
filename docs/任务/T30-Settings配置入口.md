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
