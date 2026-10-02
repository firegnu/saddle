# 任务：新增参考图墨绿色Lagoon主题

2026-10-02，saddle/main交新Claude Code，常规opus[1m]/high。
路由：常规 / 交叉审查不要 / 影响面看得见（JEV档位null、交叉不要、影响null；主控按现有框架新增纯配色预置判定，不改业务）。
类型：样式／文案调整
依据：用户提供纯色截图，要求新增同色主题。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的agent，不再委派，不记录遥测。

## 先读
AGENTS.md、src/theme.rs预置、src/settings.rs的Preset::ALL使用、docs/UI主题设计.md末尾补充；/Users/firegnu/.agents/skills/better-colors/SKILL.md只用配色相关指导，不扩审计或依赖。

## 在哪里干活
分支theme-lagoon；worktree /Users/firegnu/Developer/personal_projs/saddle-worktrees/theme-lagoon。
范围src/theme.rs，以及新增预置确需的最小Settings标签/测试/README/config.toml示例更新。本任务完成记录；不要动终端渲染或业务。

## 要做的
- 新增第四个英文预置Lagoon，配置名lagoon。参考图/Users/firegnu/Desktop/SCR-20261002-symr.png，主控已提取1154×668所有像素均RGB(12,22,22)，即#0c1616；以此为主背景和Agents背景，右侧默认终端色沿已有机制自然跟随。
- 配齐现有字段的墨绿中性色层次，正文、弱文字、边框/分隔、选中/焦点、弹窗协调；状态色保留可区分的既有语义。只用现有字段与十六进制表示，不增加token层、开关、依赖或主题机制。
- 保留Dune/Tide/Terminal的所有配色与兼容行为。沿现有Preset::ALL让Settings可选；主题顺序Dune/Tide/Lagoon/Terminal。检查枚举列举文案及示例同步，英文不汉化。
- 用户覆盖、切换整套草稿、Save/Cancel、默认色渲染逻辑完全沿用。不要自动给用户选Lagoon，不写真实配置。

## 怎么算做完
用户原话：
> [Image #1] 这个颜色的也弄一个主题。我也比较喜欢这个颜色

验证预算：git diff --check、一组直接预置/Settings检查（可在现有tests/layout_config.rs或tests/settings.rs加入最小目标后只跑目标名称）和声明颜色主要正文/选中文字对比度数值核对。不制造RED，不全套Clippy/录屏/真实agent测试。对比度只称计算值，不称真实终端验收；不要加永久对比度框架。所有cargo命令CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 不要做
不改其他主题、布局/语言/业务/终端协议/Corral或插件协议；不写真实配置/队列/数据，不安装release、不关用户agent。不得超预算重跑全套或追加审计。若确需跨范围先报告。
不合并不推送，只在分支提交。

## 做完
本文件末尾追加完成记录：改动、实际检查结果/对比度数值及其前景背景、取舍、未完成项。命令前台完成，最后一行DONE。主控核对后构建安装让用户体验。
