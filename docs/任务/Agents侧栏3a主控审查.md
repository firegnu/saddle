# Agents 侧栏 3a 主控审查

2026-09-27，主控 saddle/main。审查开发分支 `agents-panel-3a`，提交 `cb36c3e` + `c3c2346`，范围 `fc7e131..c3c2346`。按常规/改行为预算主控审查，不新增独立交叉审查。用户原稿在 docs/设计稿/agents-panel-3a/，解释与明确补充以 DESIGN 第40节为准。

## 首轮结论：改完再合并

- effort 已按用户原话恢复 R1 类型之后、状态之前，保留两列信号格及未知对齐，折叠/42 列/50 列检查、原三档字形和颜色断言均在；此项通过。
- 主控实跑 `cargo test --all-targets` 完整退出 0，workflow 58 passed、2 ignored，所有其他目标通过；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check fc7e131..HEAD` 通过。均在开发 worktree、共享 CARGO_TARGET_DIR、前台完成。
- 已读取开发者关于两次全套中偶发失败与 fc7e131 归档基线复现的记录。主控本次未复现，也未重新重复基线实验；保留“基线也可复现、根因未定位”的限制，不因本次绿就宣称竞态已修复。该问题不属于本轮 Agents 正常验收新增缺陷，暂不扩大修复范围，不再重复跑整套以碰到全绿。

## 必须修正的规格偏差

1. **R5 放得下的路径仍被无条件缩成末两级。** `src/ui.rs::agent_path` 从 `parts.len().saturating_sub(2)` 开始构造，即便可用宽度充足，`/tmp/team/project`（group 与 project 不同）也会渲染成 `…/team/project`。原稿第4节明确“若末段 == 分组名，只显示父目录；否则完整显示。超宽时从左侧截断，保留末段”。这是静态可确定的正常输入偏差。修正为先形成规格所要求的目录，再仅在超宽时从左侧缩短；同组父目录的截图缩写习惯可保留，但非同组且放得下必须完整。不修改公开 cwd 或 Git 查询。加一条直接渲染检查即可，不扩为路径清理功能。
2. **256 色终端近似色尚未落实。** 原稿第8节要求真彩色，256色终端取最近值；本提交仅新增 RGB 常量/配置，没有颜色能力选择或 RGB→256 转换（静态检索 src/theme.rs/ui.rs/main.rs，无对应实现）。在仅支持256色的终端上不应继续把 Agents 调色板全以真彩色发送。最小实现 Agents 区的近似色呈现，真彩色环境保持原RGB，保留其他区域与用户配置的语义，不引入全局主题系统/新调参UI/依赖。用合成能力输入或隔离环境作对应直接检查，不依赖用户终端。
3. **已连接 `[Attached]` 的显示状态被与可点击性混同。** 原稿第1节要求暗色、激活时正常色；当前 `connected` 分支设 enabled=false，统一禁用样式使已连接标签变暗。按连接状态点亮为正文色，但继续不生成重复 attach 点击目标；“已经连接”与“还能点击”是两回事。未连接的 Attach 继续沿用已有操作规则；不要改变会话/焦点语义。定向渲染与点击目标检查即可。

## 实现取舍裁定

- 同意 Agents 专用 `agents_*` 配色键，复用现有配置结构且隔离 Tasks/右侧终端，无需删成常量；不再添加其他设置。
- 同意默认 `s Sort`、名称序 `s Name` 以及 `z Fold/Expand`；保留对应切换语义。`[Attached]` 颜色按第3条修正，点击禁用保持。
- 同意边框内头部、固定底栏、字符格 gutter 与无法表示2px时不加空行，未知等待内容显示 ASK waiting for input、↑n 的真实ahead含义、binary/unknown 如实显示、未读标记用单格 •。
- effort 以用户明确要求为最终裁定，不再移位；主行固定列为两格信号柱保留空间，优先缩名称。
- 旧测试坐标/文案更新与默认状态序对应的 Harness 初始化调整保留；原输入路由/生命周期断言未削弱。不要为本次小修重做这些逻辑。

## 返工任务

交回原 `saddle/dev-agents-panel-1`，只处理上述三项规格偏差及直接回归，在原 worktree/分支提交。先读取本主仓库文件（只读），以及主仓库 DESIGN 第40节和原稿。纯视觉修正不伪造 RED；如涉及新的颜色能力分支，按 AGENTS.md 定向行为检查。仅运行直接相关检查及必要 Clippy/fmt/diff，不重复全套、录屏、基线多轮实验或新交叉审查。

继续原任务所有范围限制；不再委派，不操作现场 agent/TUI/socket/队列，不改 corral/drover、Git采集、生命周期/PTY/ctl，不 merge/push/release。主仓库文档由主控维护；本文件只读，完成记录写到自己分支原实施任务末尾。命令均设置共享 target 并前台完成，回复新 SHA，最后一行 DONE。


## 三处规格修正复核：可以合并

2026-09-27，核对原实现者 idle、attached=0、新回复 DONE，最新提交 `8872c5c`，只审 `c3c2346..8872c5c` 与直接相关变化；未重复主控已通过的全套标准检查，无独立交叉审查。

- 三项必须改均关闭：非同组路径放得下时完整显示、超宽才左侧缩短；Agents 专用 RGB 在无 COLORTERM=truecolor/24bit 时取最近 xterm 256 色；已连接 `[Attached]` 正文色且无重复 Enter 点击目标。effort 首行类型后/状态前及折叠呈现保持不变。
- 主控定向实跑：UI 39、layout_config 8、workflow 指定6项通过；app 原启动检查通过，新增颜色检查首次受主控环境 NO_COLOR=1 影响失败（输出确实全部无色），以 `NO_COLOR= cargo test --test app agents_colors_follow_the_terminals_announced_color_depth` 隔离禁色变量后通过。两次结果均保留，不称原命令全绿。`git diff --check fc7e131..HEAD` 通过；开发者本提交 Clippy/fmt 通过，未重复。
- 同意 COLORTERM 最小能力判定、只转换 Agents 专用 RGB、保留 ANSI/共享状态色与 effort 既有取色；同意为直接检查点击命中公开 Hits.buttons，以及 workflow 显式设置 COLORTERM 的环境隔离。未改生命周期、PTY、ctl、Git 采集或队列。
- **建议改（不阻塞）**：tests/app.rs 的 first_frame 可像 workflow harness 一样显式清空 NO_COLOR，避免禁色环境下颜色测试假失败。当前用受控环境已验证产品两条颜色路径，属于测试环境兼容性，不交回增加一轮产品返工；本轮保留，后续由用户决定是否另行处理。
- 首轮记录的 picker 偶发测试问题基线亦能复现，根因仍未定位，本次不重试整套、不宣称修复。无剩余必须改；按授权合并、推送、发布和清理，不操作 drover/T20。
