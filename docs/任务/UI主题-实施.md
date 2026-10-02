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

## 完成记录（saddle/dev-theme，2026-10-02）

### 实际改动
- src/theme.rs：新增Preset（dune/tide/terminal，未知值报错）、Overrides（只含[colors]写出的键）和Theme::with；Dune即原Theme::default()；Tide、Terminal为完整41色。Theme不再直接反序列化，[colors]由deserialize_overrides逐键校验（未知键、无效颜色均报错并带键名）。
- src/config.rs：新增顶层theme与overrides；colors为有效配色（主题+覆盖），渲染、for_terminal及插件五色调用点不变。
- src/settings.rs：Colors F2顶部新增Theme行（‹ Dune ›，←/→、Space/Enter、点击切换；点‹为上一个）。每个颜色记“跟随主题/显式覆盖”；输入改动即成覆盖（等于主题值仍是覆盖），覆盖行标custom。选另一主题：草稿载入整套配色、清除全部颜色覆盖、给出提示；选当前主题无变化。颜色Default=恢复跟随（保存时删[colors]键）；Theme行Default=按切换规则回Dune（已是Dune时不动）。Save只写theme及增删的颜色键；删除键时上方注释行移到相邻键。草稿标记、预览、失败草稿、Keep/Discard冲突处理沿用。
- config.toml：增加theme = "dune"；颜色全部改成注释示例并说明只取消注释要改的项。README：Settings与配置说明各补最短用法。docs/DESIGN.md、docs/UI主题设计.md：状态改为已实施待审查，记录实施取舍。
- 测试：tests/layout_config.rs新增1项、tests/settings.rs新增4项；改动2项旧测试（F2现在先选中Theme行需多按一次Down；colors_and_commands改用自带夹具，因示例颜色已注释掉）。

### Tide关键值与核对（WCAG对比度，静态计算，非真实终端验收）
- Agents：bg #151b23，text #dce4ee（13.50，Dune 13.09），branch #bfcad8（10.43），dim #8595a9（5.66，Dune 4.63），dimmer #6b7a8f（3.96，Dune 3.35），accent #86c1e6（8.89），选中底#212b38上text 11.16、dim 4.68。状态色在agents_bg上6.05–9.49，均不低于Dune对应项的量级。
- 共享：border #435166，muted #8d9bb0，dim #56657a，focus #86c1e6，connected #6ccfb4，working #7a9cf0，danger #e06c75，unread #c38ae8；focus底上input_text #0e141b为9.51。bg/overlay/text保持default（理由见DESIGN）。
- 色相：focus 203°、working 223°、connected 164°，彼此≥20°；状态/危险/agent类型保持Dune的语义色相。

### 验证
- RED（先加脚手架使测试可编译，失败均为断言失败）：layout_config新测在“等于主题值的覆盖应被记录”处失败；settings 4项新测因无theme字段（value("theme")为None）/无Theme行失败。
- GREEN：cargo test --test settings --test layout_config 全过（19+10）；相关diagnostics、ui也全过。格式化后两项复跑仍过。
- 布局核对：TestBackend 80×36渲染Colors页，Theme行在Interface上方、‹与颜色输入[对齐、覆盖行显示custom、切换Tide后提示两行显示，颜色输入宽度因右侧custom/提示列由40缩到36。
- cargo clippy --all-targets -- -D warnings：通过。cargo fmt --check：通过。
- cargo test --all-targets：失败。第一次在tests/plugin_resources.rs停下（3项失败：外部插件行“Synthetic external”、“Enabled. skill-a r2”缺失），单项复跑一次结果相同；为覆盖后续目标又以--no-fail-fast跑了全套（同一命令里误多跑了一遍以列失败名），另见tests/workflow.rs 9项失败（Drover/插件交互等待画面，如“Invalid task event JSON”、插件面板）。其余目标全过，含theme相关的startup_colors_reach_agents_queue_controls_and_leave_viewer_colors_alone。本次未改src/plugins、Drover或workflow相关代码，这12项判断与本改动无关，但未在main基线上对照验证。

### 取舍与未完成
- Tide保留终端默认背景（见DESIGN）；若要宿主整体刷冷色底，需要另定Viewer默认背景如何处理。
- Tide共享区RGB在非truecolor终端不降级（沿用现有规则）。
- 删除颜色键时，键行自身的行尾注释随键删除；表内键全部删光时，原上方注释也不保留。
- 保存了theme键的配置，旧版Saddle因deny_unknown_fields无法读取（设计已列为降级限制）。
- Tasks状态色、Diff语法高亮、Viewer终端颜色仍不跟随主题（已知边界）。

### 需主控决定
- plugin_resources 3项、workflow 9项失败是否需在main上对照确认（本任务预算内未做）。
- Tide是否接受bg/overlay/text跟随终端的取舍。
