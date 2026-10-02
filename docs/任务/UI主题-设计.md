# 任务：Theme 预置与颜色覆盖的最小设计

2026-10-02，saddle/main 交给新建 saddle/dev-theme-design（Claude Code，常规 opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（JEV档位拿不准，cross_review不要，impact看得见；主控按默认常规定档）。
类型：设计
依据：用户已放行Theme功能并指定现有主题名Dune，本轮收敛现有Colors页上的交互及兼容方案，不实现。
提示：围绕已知约束给出推荐及关键取舍；有实质备选时再比较，不凑方案数量。
你是被委派的agent，照本文件做，不再委派；无需采集遥测。

## 先读
- AGENTS.md；docs/任务/UI主题预置与颜色覆盖.md
- src/theme.rs、src/config.rs、src/settings.rs中配色/草稿/保存/冲突相关实现；直接相关测试按需阅读
- docs/DESIGN.md中Settings/颜色配置相关节
- /Users/firegnu/.agents/skills/better-colors/SKILL.md：只沿用相关配色原则，不展开无关全局审计

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/theme-design，分支theme-design。
- 只新增docs/UI主题设计.md，并在本任务文件追加完成记录；不改功能代码、测试或主仓库文件。

## 要做的
- 给出可直接评审的一份简短方案：现有Colors页顶部放Theme选择，下面颜色字段随主题变化、仍能覆盖；保留现有英文UI语言。给一张紧凑文字线框即可，不重设计整个Settings。
- 现有主题命名Dune。建议少量其他预置，说明风格与关键颜色即可，避免主题商城/导入导出/复杂继承。现有Theme语义字段和插件通用主题传递优先沿用。
- 明确切换主题时已有覆盖怎么处理、单项恢复默认的含义、Save/Cancel及预览行为、旧配置兼容的最小推荐。不要把建议写成用户已批准。
- 区分代码默认颜色与用户当前有效配色：脱敏的当前colors快照在 /var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-theme-telemetry-qtk_rdga/current-colors.json（只含颜色，不含其他配置）。Dune要保持当前观感，并说明Reset/ANSI颜色仍取决于终端；不偷偷把用户配置改成一份新的默认值。不要读取其他用户配置。
- 静态核对宿主与插件已有配色接口，说明哪些视图沿用主题，哪些嵌入终端颜色不由Saddle控制。不要为此扩展插件协议或业务。

## 怎么算做完
用户原话：
> 写一个ui theme，预置几种theme，选择的时候，下面color随theme变化，但是用户可以覆盖这个颜色。
> 现在的theme可以设为dune

本轮交付这些需求的设计方案，不声称功能已实现。验证预算仅git diff --check及方案对照实际接口的静态核对；不跑测试、Clippy、构建、真实终端或真实业务。需要实质新取舍在回复中列出。

## 不要做
- 不实现，不改界面语言，不扩大到其他UI重排/任务/队列/遥测逻辑。
- 不改Corral或反向依赖；发现此类需要就报告。
- 不安装、不release、不写真实配置/数据、不操作队列，不关闭用户agent，不按名字批量杀进程。
- 不合并、不推送，只在theme-design分支提交。

## 做完
在本文件追加完成记录：方案、核对、待主控或用户决定的事。回复列出提交和简短推荐。命令都在前台跑完，全部做完后，回复最后一行写DONE。

## 完成记录（2026-10-02，saddle/dev-theme-design）

**方案**：见 `docs/UI主题设计.md`，是推荐方案，未经用户批准，也没有实现。

- Colors 页顶部加 `Theme` 行（←/→ 切换），下面颜色字段不变。有效配色 = 预置 + `[colors]` 覆盖，行尾 `custom` 标出覆盖项。
- 预置：Dune（等于现在的 `Theme::default()`）、Tide（冷色中性，强调色和状态色不变）、Terminal（全部用 default/ANSI）。
- 存储：新增顶层键 `theme`，省略即 Dune；旧配置不迁移、不重写，观感不变。
- 切换主题（推荐 A）：等于原主题值的覆盖改为跟随新主题，其余覆盖保留。只改草稿，Cancel 可撤回。
- `Default Ctrl-D`：去掉这项覆盖，保存时删除这个键。预览、Save、Cancel、冲突处理沿用第 47 节。

**核对**（只做了静态核对）：

- 读了 `src/theme.rs`、`src/config.rs`、`src/settings.rs`（草稿、`defaults`/`saved`、Ctrl-D、`apply`、冲突重载）、`src/app.rs`（`apply_settings`、`plugins.theme`）、`src/plugins/mod.rs:280`、`crates/plugin-sdk`、`plugins/drover/src/plugin.rs` 的 `render`、`plugins/diff/src/app.rs`/`view.rs`、`src/terminal.rs` 的 `color`、`docs/插件协议.md`，以及 `docs/DESIGN.md` 第 8、17、18、40、47 节。
- 用户快照共 28 键：24 项与代码默认相同，4 项不同（`agents_bg`、`agent_selected`、`claude`、`codex`）。
- Tide 与 Dune 的 WCAG 对比度用脚本算过；Terminal 的取决于用户终端，没测。
- 按预算跑了 `git diff --check`；没跑测试、Clippy、构建或真实终端。

**待主控或用户决定**：

1. 切换规则选 A 还是 B。
2. 预置的名字和取值。
3. 用户当前的 4 项差异是否并入 Dune（推荐不并入）。
4. 顶层 `theme` 键的降级代价。
5. 批准后修订 DESIGN.md 第 8 节（"不增加多主题"）和第 47 节"恢复默认"的含义。
6. Drover 的 Tasks 状态色不跟随宿主主题，是现状，是否另立任务。
7. 实施时把 `config.toml` 示例里的颜色改成注释。

没有需要改 Corral 或反向依赖的地方。
