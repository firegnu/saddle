# UI 主题：预置与颜色覆盖（设计稿，待评审）

2026-10-02，saddle/dev-theme-design 起草。依据 `docs/任务/UI主题预置与颜色覆盖.md` 与 `docs/任务/UI主题-设计.md`。本文是推荐方案，**尚未经用户批准**，功能也未实现。

用户原话：

> 写一个ui theme，预置几种theme，选择的时候，下面color随theme变化，但是用户可以覆盖这个颜色。
> 现在的theme可以设为dune

## 1. 推荐结论

- 现有 Colors 页（`Colors F2`）最上面加一行 `Theme`，下面 41 个颜色字段原样保留，分组、标签、色块和预览都不变。
- 有效配色 = 所选预置 + 用户覆盖。`[colors]` 里写了的键就是覆盖，没写的键跟随主题。
- 现有观感命名为 **Dune**，取值就是代码里现在的 `Theme::default()`。没有 `theme` 键的配置都按 Dune 处理。
- 另外推荐两种预置：**Tide**（冷色深底）和 **Terminal**（全部用终端默认色 / ANSI）。
- 不做主题商城、导入导出、自定义主题文件或多层继承；不扩展插件协议。

## 2. 线框（Colors 页）

外框、页签、按钮栏沿用现有 Settings；只多了 `Theme` 一行、行尾的 `custom` 标记和一条切换提示。下例是这位用户的配置在草稿里切到 Tide 时的样子（见第 6 节）：

```text
┏ Settings ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ Config: ~/.config/saddle/config.toml                                     ┃
┃ General F1  Colors F2  Advanced F3  Diagnostics F4  Plugins F5           ┃
┃                                                                          ┃
┃•Theme              [Tide          ] ←/→ switch                           ┃
┃                                                                          ┃
┃ Interface                                                                ┃
┃•bg                 ██ [default       ]                                   ┃
┃•focus              ██ [yellow        ]                                   ┃
┃  …                                                                       ┃
┃ Agents panel                                                             ┃
┃ agents_bg          ██ [default       ] custom                            ┃
┃ agents_border      ██ [#4b5462       ]                                   ┃
┃ agent_selected     ██ [#302a23       ] custom                            ┃
┃  …                                                                       ┃
┃ Preview · unsaved colors                                                 ┃
┃ Status  ● Working   ○ Idle   ? Waiting   ! Error                         ┃
┃ Text    Normal   Muted   Code   Heading                                  ┃
┃ Tide: 24 colors now follow the theme; 4 keep your overrides.             ┃
┃                          [Default Ctrl-D] [Cancel Esc] [Save Ctrl-S]     ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

- `Theme` 行：←/→（以及 Space/Enter）在 Dune → Tide → Terminal 之间循环，用法同现有开关行的 `Space/Enter toggle`。
- 颜色行输入框显示**有效值**：有覆盖就显示覆盖值，没有就显示所选主题的值。行尾 `custom`（muted）表示这一项是覆盖。
- 行首 `•` 仍表示"与已保存状态不同"。覆盖变成跟随主题、或反过来，都算改动，即使颜色值没变。
- 提示用现有的 message 行，只在切换主题后出现。

## 3. 预置

三套都按深色终端设计。`bg`、`overlay`、`text` 等共用字段三套都保持 `default`/ANSI，不给整个界面刷一块固定底色，否则会和 Viewer 里 agent 自己的终端底色对不上。四个类型色（`claude`/`codex`/`pi`/`omp`）是品牌标识，不算主题色，三套相同；只有 Terminal 把 `pi` 改成 `default`。

| 预置 | 风格 | 关键颜色 |
|---|---|---|
| **Dune**（默认） | 现状：暖沙色 Agents 栏，其余跟随终端 | 共用字段为 `default`/ANSI；`agents_bg #1d1a16`、`agents_text #e8dfcc`、`agents_dim #8c8374`、`agents_accent #bdb86a`、`agent_selected #2b2621`；状态色 `agent_working #7fb4ee`、`agent_idle #9cbd80`、`agent_blocked #e6b566`、`agent_stalled #e79b65`、`agent_error #ef8174`、`agent_starting #b0a1d8`。完整取值即 `src/theme.rs` 的 `Theme::default()` |
| **Tide** | Dune 的冷色版：中性色换成蓝灰，强调色和状态色不变 | `agents_bg #161a1f`、`agents_border #4b5462`、`agents_rule #2d343e`、`agents_faint #323a45`、`agents_text #dce2ea`、`agents_branch #c0c8d3`、`agents_dim #838d9a`、`agents_dimmer #6c7684`、`agent_selected #222932`；其余字段同 Dune |
| **Terminal** | 不用 RGB，完全跟随终端调色板；适合非真彩色终端或想用终端主题统一观感的人 | 共用字段同 Dune；`agents_bg/text/branch = default`，`agents_dim = gray`，`agents_dimmer/border/rule/faint = dark_gray`，`agent_selected = dark_gray`，`agents_accent = yellow`；状态色为 `blue`/`green`/`yellow`/`light_red`/`red`/`magenta`（working/idle/blocked/stalled/error/starting），`agents_*` 状态同理，`agents_purple = magenta` |

配色核对（按 better-colors 的原则；WCAG 对比度是对实际绘制背景用脚本算出来的）：

- Tide 只换中性色的色相（34° → 213°），各层级的对比度和 Dune 基本一致。在 `agents_bg` 上：text 13.41（Dune 13.09）、dim 5.19（4.63）、dimmer 3.80（3.35）、border 2.28（2.26）。在选中底 `agent_selected` 上：dim 4.36（Dune 4.00）。
- Tide 不另选强调色。试过青色 `#6fc2bd`（176°），但它和 `connected = cyan` 撞色，违反"一种颜色一个含义"，所以沿用 Dune 的 `#bdb86a`。
- Terminal 的对比度**取决于用户终端的调色板，没测**。ANSI 里没有橙色，所以 stalled（`light_red`）和 error（`red`）色相接近；状态本身还有字形和文字区分，不只靠颜色。

## 4. 存储与旧配置兼容（最小方案）

```toml
theme = "tide"           # dune | tide | terminal；省略 = dune
[colors]                 # 只写要覆盖的颜色；省略的跟随主题
agents_bg = "default"
```

- 新增顶层键 `theme`，不放进 `[colors]`，这样 `[colors]` 仍然只装颜色，`Theme` 的字段和语义不用动。名字未知时报错，沿用第 8 节"解析失败不静默回退"的规则。
- `[colors]` 的语法和校验不变，含义从"覆盖代码默认值"变成"覆盖所选主题"。没有 `theme` 键时主题就是 Dune，而 Dune = 现在的代码默认值，所以**所有旧配置升级后观感不变，文件也不改**。
- 不做迁移，不重写文件，不往文件里补一份"新默认值"。只有用户在 Settings 改了主题并保存，才写 `theme` 键：保存只写改过的键，切回 Dune 不算改动。
- 降级：写了 `theme` 的文件，旧版 saddle 读不了，因为 `Config` 用了 `deny_unknown_fields`。这和以前任何新键的情况一样，不另做处理。
- 实施时代码要能分出"文件里有这个键"和"取的是默认值"：现在 `Theme` 用 `serde(default)` 反序列化，读完就分不出了。所以覆盖要按 `Option<Color>` 逐键保存，再和预置合成有效 `Theme`。合成后传给界面和插件的 `Theme` 结构不变。
- 仓库根目录的 `config.toml` 示例现在列出了全部颜色。照抄的话每个键都会变成覆盖，切主题时什么都不变。实施时建议把颜色行改成注释示例。

## 5. 交互规则

**切换主题时已有覆盖怎么处理（推荐 A）**：在草稿里从主题 X 切到 Y：

- 覆盖值**等于 X 的值**的键，视为"没有真正改过"，改为跟随 Y；
- 和 X 不同的覆盖原样保留，继续标 `custom`；
- 没有覆盖的键直接显示 Y 的值。

比较的是解析后的颜色，所以 `reset`/`default`、`#FFFFFF`/`#ffffff` 算相等。这些变化只发生在草稿里，带 `•` 标记，会出提示（"N colors now follow the theme; M keep your overrides."），Cancel 能全部撤回，Save 时把转为跟随的键从 `[colors]` 删掉。

为什么不推荐 B（"所有覆盖一律保留，另给一个全部重置的按钮"）：这位用户的文件里显式写了 28 个颜色，其中 24 个和 Dune 完全一样。按 B，切主题时这 24 项全都钉住不动，正好违背"选择的时候，下面color随theme变化"。A 的代价是：用户刻意把某项写成和当前主题一样的值，切主题后它也会跟着走；不过草稿里看得见，可以 Cancel，也可以再改回来。**A 和 B 怎么选要用户定**（第 8 节）。

**单项恢复默认（`Default Ctrl-D`）**：

- 在颜色行上：去掉这项覆盖，改为跟随草稿里的主题，输入框显示主题的值；保存时从 `[colors]` 删除这个键。这项只有是覆盖时按钮才可用。以前的做法是把代码默认值显式写回文件，以后不再这样。
- 在 `Theme` 行上：切回 Dune，按上面的切换规则处理。
- 按钮文字保持 `Default Ctrl-D`，不改界面语言。

**编辑**：在颜色行上输入任何内容，这一项就成为覆盖，值等于主题值也一样。`Ctrl-U` 清空后是无效颜色，和现在一样保存时会报错；空值不当作"跟随主题"，跟随主题只能用 Ctrl-D，避免两种入口语义不同。

**预览、Save 与 Cancel**：沿用第 47 节。

- 草稿里的主题和覆盖只体现在色块和 `Preview · unsaved colors` 中，不实时改整个界面。
- Save 成功后立即在当前 saddle 生效（走现有 `apply_settings`）。插件会在下一帧收到新的五个语义色。
- Esc/Cancel 丢掉主题和所有颜色草稿。
- 冲突检测、Keep my edits / Discard my edits 不变：`Theme` 是普通的已编辑字段，会和其他已编辑项一起列在冲突提示里。
- 颜色无需重启。

## 6. 代码默认颜色与用户当前有效配色

- **代码默认颜色**：`src/theme.rs` 的 `Theme::default()`，即本方案的 Dune。
- **用户当前有效配色**：Dune 叠加用户 `[colors]` 里的 28 个键。核对来源是脱敏快照 `current-colors.json`，按"它就是文件里的 `[colors]` 表"理解：快照有 28 项，`Theme` 有 41 项，缺的 13 项正好是除 `agents_bg` 以外的 `agents_*`。
  - 24 项和代码默认值相同。
  - 4 项不同：`agents_bg = default`（代码 `#1d1a16`，即 Agents 栏用终端底色）、`agent_selected = #302a23`（代码 `#2b2621`）、`claude = #d97757`（代码 `#e2835e`）、`codex = #8ed9c1`（代码 `#79d4b4`）。
- 推荐**不把这 4 项并入 Dune**。它们是这位用户自己的选择，留在文件里作为覆盖，用户看到的效果和现在完全一样；Dune 本身仍等于代码默认值，没有配置文件的人看到的也不变。
- 观感取决于终端：`default` 是终端的默认前景/背景，ANSI 色名用的是终端调色板。所以 Dune、Terminal 的这些部分在不同终端里看起来不一样，Saddle 不负责也不保证。RGB 字段在没有声明真彩色时：`agents_*`、`agent_selected` 和类型色由 `Theme::for_terminal` 转成最近的 256 色；`agent_working` 等共用状态 RGB 不转换。这一点是现状，本方案不改。

## 7. 哪些视图跟随主题（静态核对）

| 视图 | 颜色来源 | 跟随主题与覆盖 |
|---|---|---|
| Agents 栏、终端窗格边框/标题、各弹窗（Settings、New、Search、Placement、Attention、遥测页等）、按钮 | 宿主 `config.colors`（`src/app.rs`、`src/ui.rs` 等） | 是 |
| 插件面板（Drover、Diff） | 宿主每帧调 `plugins.theme`，只传 `text/muted/background/accent/error` 五个语义色，变化时发 `theme` 事件（`src/plugins/mod.rs:280`，`docs/插件协议.md`） | 只有这五个色跟随 |
| Drover 的其他颜色（Tasks 状态色等） | 插件自带的 `plugins/drover/src/theme.rs` 默认值，在 `render` 里只用五个语义色覆盖 | **不跟随**，覆盖 `agent_working` 等也不影响 Tasks。这是插件化之后就有的现状 |
| Diff 的语法高亮 | 插件固定用 `base16-ocean.dark` | 不跟随 |
| Viewer 里 agent 的终端输出 | RGB 原样输出；索引色/命名色先查程序用 OSC 设的调色板，没设就按索引交给外层终端画；默认色交给终端（`src/terminal.rs` 的 `color`） | **不受 Saddle 控制** |

按本轮约束，插件协议和 Drover 都不扩展。要让 Tasks 状态色也跟随主题，得另开任务决定协议或插件的改法（第 8 节）。

## 8. 待用户或主控决定

1. 切换主题时覆盖怎么处理：推荐 A（等于原主题值的覆盖改为跟随新主题），备选 B（全部保留，另加批量重置）。
2. 预置名单与取值：Dune / Tide / Terminal，名字和 Tide、Terminal 要不要做都可以改。
3. 用户当前的 4 项差异是否并入 Dune：推荐不并入，继续作为覆盖。
4. 存储用顶层 `theme` 键，接受写入后旧版不能读这一降级代价。
5. 批准后需要修订 `docs/DESIGN.md` 第 8 节（"不增加多主题、热加载或继承"）和第 47 节"每项可恢复默认值"的含义；本轮按约束不改那份文档。
6. Drover 的 Tasks 状态色不跟随宿主主题（现状）：本轮不处理，要不要另立任务请决定。
7. 实施时把仓库根目录 `config.toml` 示例里的颜色行改成注释（第 4 节）。
