# 任务：实现Dune及预置主题、单项颜色覆盖

2026-10-02，saddle/main交给新建saddle/dev-theme（Claude Code，常规opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（JEV三项一致，主控照用）。
类型：功能变更
依据：用户已批准Theme线框、Dune/Tide/Terminal，以及切换载入整套配色后再逐项覆盖，明确授权实施。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的agent：照本文件做，不再开agent，不需要记录遥测。

## 先读
- AGENTS.md
- docs/UI主题设计.md（已定稿，取代原设计稿A/B切换方案）
- docs/DESIGN.md第8、47节和末尾Dune主题决策
- src/theme.rs、src/config.rs、src/settings.rs及直接相关测试
- /Users/firegnu/.agents/skills/better-colors/SKILL.md：只应用本次配色所需部分，不能据此扩审计或启动agent

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/theme-presets；分支theme-presets，已从main创建。
- 范围：src/theme.rs、src/config.rs、src/settings.rs，必要且最小的配置调用点适配，config.toml示例、直接相关测试/文档和本任务完成记录。

## 要做的
- 沿用现有Colors页与英文，顶部添加Theme选择（已批准线框见设计）。预置Dune/Tide/Terminal；颜色字段仍能单独编辑并标custom。键盘和鼠标均沿用现有控件方式。
- Dune保留现有代码默认值。旧配置无theme时观感不变，不迁移、不自动写回用户配置。Tide是完整协调的冷色主题，覆盖公共区域和Agents颜色；Terminal所有字段只用终端默认/ANSI，不以RGB冒充全终端配色。
- 明确选择不同主题，在草稿中载入整套预置并清除草稿颜色覆盖，然后允许逐项修改。升级或只选择当前主题不清掉既有覆盖。颜色Default恢复跟随当前主题。完整行为依设计，不再自行改回“相等值自动剔除”等初稿策略。
- 保留Save/Cancel、预览、失败草稿和外部配置冲突处理；只保存编辑的主题/颜色覆盖增删，保留无关配置和注释。跟随主题和显式覆盖须可区分，最终渲染Theme/现有插件五色接口沿用。
- 配置示例展示theme及注释的覆盖项，避免照抄产生全覆盖。更新最短必要使用说明；不重排其他页面。

## 怎么算做完
用户原话：
> 写一个ui theme，预置几种theme，选择的时候，下面color随theme变化，但是用户可以覆盖这个颜色。
> 现在的theme可以设为dune
> 可以的。如果没有其他问题就实施吧

批准的是docs/UI主题设计.md所述布局及整套草稿切换规则。验证预算：目标行为先定位/补自动检查，确认目标RED后最小实现；直接相关检查及项目标准cargo test --all-targets、cargo clippy --all-targets -- -D warnings各一次。所有命令使用CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。纯视觉部分不制造RED，做最接近的布局核对。失败保留原结果，无关失败最多单项复跑一次并报告；不追加全套/覆盖矩阵/故障植入/真实终端录制。需要新增预算先报告。

## 不要做
- 不改Corral，不增加反向依赖；若需要停下报告。
- 不改插件协议、Tasks状态色或Diff语法高亮；这两项不跟随是已知边界。Viewer中agent终端颜色不属于本功能。
- 不改任务/遥测/路由业务，不自行汉化，不修改用户真实配置、技能、数据或队列。
- 不安装、不release、不重启用户Saddle/agent；不按名称或路径批量杀进程。
- 不扩主题商城/导入导出/复杂继承，不做无关重构，不合并main、不推送；只在theme-presets提交。
- 遇到未批准的实质设计变化先报告，不另起研究或委派。

## 做完
在本文件末尾追加完成记录：实际改动、验证结果、取舍与未完成事项。报告真实RED/GREEN和失败，不把夹具/编译失败当有效RED。写出提交与需主控决定的事项。命令在前台跑完；全部做完后回复最后一行写DONE。
