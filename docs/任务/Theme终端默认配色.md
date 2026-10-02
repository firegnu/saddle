# 任务：右侧终端默认配色跟随Theme

2026-10-02，saddle/main交新Claude Code（常规opus[1m]/high）。
路由：常规 / 交叉审查不要 / 影响面看得见（JEV常规/不要/影响拿不准；主控判仅显示默认颜色映射，无业务或终端协议变化）。
类型：样式／文案调整
依据：用户看到Tide左冷右暖截图后批准“那就改改试试”。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的agent，不再委派，不记录遥测。

## 先读
- AGENTS.md，docs/UI主题设计.md末尾本轮补充，docs/DESIGN.md末尾本轮补充。
- src/terminal.rs渲染及颜色映射、src/ui.rs和src/terminals.rs两个终端绘制入口、src/theme.rs Tide。

## 在哪里干活
worktree /Users/firegnu/Developer/personal_projs/saddle-worktrees/theme-terminal-colors；分支theme-terminal-colors。
允许改上述四个源文件与直接测试、必要说明及本任务完成记录。不改PTY/输入/进程/agent生命周期/存储/任务与遥测业务。

## 要做的
- 右侧终端默认背景和默认前景跟随当前有效Theme的bg/text。只替换“未显式指定”的默认颜色；ANSI索引、RGB、agent通过终端调色板明确设置的颜色仍按现有方式显示。反色/选中/代码块等既有样式语义保留。
- 用现有bg/text，不增加新配置项、开关或协议。两个已有绘制入口一致，已打开终端随Save后的主题重绘；默认色映射不改解析器中的原始输出或历史内容。普通shell沿用同一Screen渲染，不另造agent专用机制。
- Tide现有bg/text仍是default，接线后仍不会变。因此仅将Tide的bg/text设为与其冷色Agents区协调的明确RGB（建议沿现有agents_bg #151b23、agents_text #dce4ee），其他色不顺手重设计。Dune与Terminal的default行为保持，用户显式bg/text覆盖按现有配置生效。
- 色块及Preview/Save-Cancel流程沿用。保持英文，不加页面，不碰插件五色协议；agent完全显式铺色仍不会被强行改色。
- 仅渲染范围：Screen::process及ColorRequest/default_rgb查询回执、PTY通信保持原逻辑，不扩成终端主题协议同步；此轮不承诺应用会根据新主题重新计算自身配色。遇到必须改这些机制或Corral的情况先报告。

## 怎么算做完
用户原话：
> 那就改改试试？[Image #1] 现在切换到tind theme是这样。

批准提议：默认背景和文字色跟随Theme，agent明确指定颜色保留，不改输出内容或运行逻辑。
验证预算：git diff --check；一组直接渲染检查覆盖默认色跟随、显式色保留、从Tide换回default。可复用tests/terminal.rs做这一组，不要求制造RED，不跑全套/clippy、不做录屏/覆盖矩阵，不跑真实agent。所有cargo使用CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 不要做
不写真实配置/数据/队列，不安装release，不关闭或操作用户agent，不改Corral或反向依赖；不改展示语言/无关UI，不增加标准验证或重跑旧Theme全套。
不合并不推送，仅在本分支提交。发现超范围需求先报告，不自行扩展。

## 做完
本文件末尾写完成记录：改动、实际验证、取舍、未完成事项。命令前台完成，回复最后一行DONE。主控审查后再构建安装供用户体验。

## 完成记录（2026-10-02）

改动：
- src/terminal.rs：`Screen::render`增加当前Theme参数；渲染时仅对未被程序设定的默认前景/背景（alacritty的Foreground/Background，动态调色板为空时）回退到`theme.text`/`theme.bg`。ANSI索引、RGB、OSC 10/11等调色板设置的颜色照旧优先；反色、选中、历史等样式不变。`Screen::process`、ColorRequest/`default_rgb`回执、解析器内容与PTY均未动。
- src/ui.rs、src/terminals.rs：两个终端绘制入口都传入当前有效Theme（每帧取，Save后的主题即重绘；普通shell同路径）。
- src/theme.rs：Tide的bg/text设为#151b23/#dce4ee（同agents_bg/agents_text），注释同步；其他色、Dune与Terminal不变。
- tests/terminal.rs：既有render调用补Theme参数；新增一组直接渲染检查。

实际验证：
- `git diff --check`、`cargo fmt --check`通过。
- `cargo test --test terminal`：6 passed。新测覆盖Tide下默认前景/背景跟随（含空白格）、显式256色/RGB保留、反色仍为REVERSED、OSC 11设置的背景优先于主题、OSC 111复位后换回Dune默认得到Reset。
- 未跑全套测试/Clippy、未录屏、未跑真实agent（按预算）。

取舍：
- 只映射Foreground/Background两个默认名；Cursor、Dim/Bright默认名等仍为原Reset回退，避免超出“默认背景和文字”。
- Tide的bg/text是共享色，宿主其余使用`base()`的区域也会随之变为这对冷色；这是任务指定的改法，非truecolor终端下照原规则不降级共享色。

未完成事项：无。主控审查后构建安装供用户体验。

## 主控审查（2026-10-02）：通过

候选ec84452，status idle/attached0，公开reply末行DONE，采集首尾配对stored。已核全部diff及完成记录：仅四个指定源文件、直接测试和任务完成记录；两个绘制入口逐帧传当前Theme，显式RGB/索引/动态调色板优先，默认Foreground/Background才回退bg/text。解析、ColorRequest/default_rgb、PTY、输入及生命周期未改。diff --check通过。
公开corral read输出已核terminal六项通过，含默认空白格/显式颜色/反色/OSC背景优先和复位后Dune；原文存/tmp/saddle-theme-terminal-public-read.txt。主控按看得见预算不重跑套件、全量或Clippy。额外fmt check只读，无新增改动。
接受Tide共享bg/text让其他base区域同步变冷，这是复用现有主题语义的预期；Cursor及Dim/Bright默认名保持旧回退，不扩终端调色协议。无阻断，无交叉审查。实际终端体验留用户重开后检查；本结论不冒充桌面截图验收。
