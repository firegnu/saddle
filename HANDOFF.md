# 交接

## 2026-09-26：T5 pending 编辑与调整次序完成（最新状态）

- T5 已由 saddle/dev-t5-pending-1 完成，两轮主控审查后合并到 main。选中 Pending 可 Edit e 编辑标题/正文，Move up u / Move down d 调序，Ctrl-S 保存、Esc 取消；失败/刷新保留草稿，成功后保持选中。非 pending 无入口，首末方向禁用。设计及公开接口限制见 DESIGN 第 20 节。
- 第一轮主控复跑发现新增 PTY 测试提前发送选择按键，开发者用 64 字节分块重现并修正屏幕同步，未改产品逻辑；原断言保留。开发者最终完整标准测试 73 passed、2 ignored，Clippy 通过；主控增量复核 workflow 17 passed、2 ignored，Clippy 与 diff 检查通过。两轮结论见 docs/任务/T5-主控审查.md。
- release 已构建并更新本仓库 target/release/saddle，重启生效；未重启用户当前界面，未改用户配置。开发分支/worktree 已清理，工作目录已删，一并关闭 saddle/dev-t5-pending-1；其他 agent 未动。已补收尾空提交 7518d18。
- 当前写入前会通过公开快照核对 pending 列表，变化则拒绝并保留草稿；drover 尚无公开版本获取入口，检查与写入之间仍存在竞态。若以后消除该限制，须另行授权扩展 drover，不能读取内部文件绕过。
- 下一步等用户放行，待办顺序 T4 → T2；T4 核查并补齐完成后的状态/过程详情，T2 tab/split 仍待补充。不自动开始下一件。
- --help 的 r reply、Queue 普通列表 PgUp/PgDn、drover 完整历史修复的远端发布情况仍未在本任务处理。

## 2026-09-26：T3 Tasks 状态文字颜色区分完成

- T3 经用户确认解除占位限制，已由 saddle/dev-t3-colors-1 实现、主控审查并合并到 main。Current/Running 蓝，Awaiting 琥珀，Pending 紫；History 标题中性，历史行 Done 绿、Failed 红、Dropped 橙，未知值原样中性显示。配置沿用 T1 现有字段，无需修改用户配置；设计和理由见 DESIGN 第 21 节。
- 开发者完成 UI 16 项、配置检查和 Clippy；主控检查完整 diff 与渲染检查，diff 检查通过，按纯视觉预算不重跑套件。release 构建通过，已更新 target/release/saddle；用户下次重启生效，本轮未重启运行中的界面。
- t3-task-colors 分支及 worktree 已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t3-colors-1，其他 agent 未动。已补收尾空提交 286054f，主控审查见任务文件。
- 下一步按用户调整的顺序为 T4（核查并补齐完成后的状态与过程详情）、T2（tab/split，仍待用户补充）。T3 收尾后等待放行，不自动开始下一件。T4 应先核查公开接口与显示差异，不预判遗漏原因。
- 旧遗留 --help 的 r reply、Queue 普通列表 PgUp/PgDn、drover 完整历史修复的远端发布情况未在 T3 处理。

## 2026-09-25：T1 启动配置与颜色配置完成

- T1 已由 saddle/dev-t1-config-1 实现，主控审查并合并到 main。27 项颜色通过启动配置进入界面，已有布局、刷新、命令路径配置保留；使用方式见 README 配置节和根目录 config.toml，设计见 DESIGN 第 8 节。
- 主控复跑标准检查：64 项通过，2 项依赖真实 drover 的既有测试忽略；Clippy 无警告，diff 检查通过。release 已构建，并更新本仓库 target/release/saddle；没有重启用户当前界面，下次启动加载新配置。
- 本机原先没有默认配置，现已创建 ~/.config/saddle/config.toml，内容与仓库默认示例逐字一致。程序支持绝对路径 XDG_CONFIG_HOME 和 --config 优先覆盖，省略项仍用当前默认值。
- 开发 worktree 和 t1-config 分支已清理；工作目录已删，一并关闭本主控开的 saddle/dev-t1-config-1。未关闭其他 agent，未改 corral/drover 仓库或服务。已补「收尾: T1 启动配置与界面颜色配置完成」空提交。
- 下一步等用户：T2「支持 tab 和 split」、T3「Tasks 状态文字颜色区分」已通过公开 CLI 加入队列，均为占位，详细内容由用户后续补充；补充前不设计或实现，不自动放行下一件。
- 仍悬着的旧事项：--help 残留 r reply、Queue 普通列表 PgUp/PgDn 不翻页，以及 drover 完整历史修复的远端发布情况。T1 不扩展处理。

## 2026-09-25 晚：接入主控分派和 drover（新主控先读本节）

- **开发方式变了**：从这里起，`saddle/main` 是主控，只拆任务、写任务文件、审查、合并，不自己写功能代码；规矩见 `AGENTS.md`「开发方式（主控分派）」，分派按 corral-dispatch 技能做。旧的 `saddle/main` 是亲手写代码的开发者，已收尾关掉，本节以下是它留下的交接。
- **drover 已接上**：交接目录 `~/.drover/saddle`，`.drover.conf`（不进 git）填了 `MAIN_AGENT=saddle/main`、`TASK_FILE_DIR=docs/任务`、`DONE_MARK=收尾`，`CHECK_CMD` 留空（不跑验收命令），放行模式（每件做完等用户 `drover go`）。队列目前是空的，任务由用户加。
- **仓库状态**：main 已推送到 origin，和远端一致（推送含旧会话的交接提交 `432471d` 和本次 AGENTS / HANDOFF 改动）；下面「本次交接提交仅保留本地」一句已过时。本文件由 `docs/PROGRESS.md` 改名而来。
- **约定**：每件活一个分支和 worktree（`../saddle-worktrees/<分支>`），所有 worktree 共用编译目录 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；标准测试 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`；合并后推送，收尾打「收尾: 」空提交，再更新本文件。
- **可以接着做的**：见下一节「已知问题与接续事项」（帮助文案里残留的 `r reply`、Queue 列表 PgUp/PgDn 不翻页等）。做不做、先做哪件，等用户给任务。

## 2026-09-25：旧会话收尾，交给新主控（接手先读本节）

### 当前状态

- 用户通过 corral/main 转达：后续改为主控分派开发，另开新主控，本会话结束。本轮只记录交接并提交，不开始新功能、不推送、不创建或关闭 agent。
- 三窗格、Agents 树形信息/配色、Queue 英文控件/状态/空态、鼠标滚动及完整历史接入均已完成；用户已反馈当前效果满意。没有做到一半的实现或未提交开发改动。
- 公开仓库为 https://github.com/firegnu/saddle。此前已推送 main 至 `7943443`，读回远端 SHA 与本地一致；英文 README 为默认首页，顶部可切中文。本次交接提交仅保留本地。
- `target/release/saddle` 已构建。最近完整默认套件为 59 项通过；之后增加的完整历史跨仓库集成测试已单独通过，现在有 2 项依赖已安装 drover 的测试默认忽略。clippy、fmt 和 release 构建已有通过记录。本轮仅更新文档，检查 diff，不重跑行为测试。
- drover 修复为 `3e66d1a`（收尾 `22de37d`）：JSON 取消十条限制，普通 list 摘要保持；saddle 验收提交为 `26cbd35`。真实公开接口核对过 22 条历史，40 条合成历史经真实 CLI 进入 saddle 后鼠标可到两端。没有修改 corral 的任何代码。

### 已知问题与接续事项

- `src/main.rs` 的 `--help` 仍列着 `r reply`，但 Agents 的 r 和 Reply 按钮已按用户要求隐藏/停用；这是帮助文案遗留，底层回复功能仍保留。本轮不扩展修改。
- Queue 普通任务列表的 PgUp/PgDn 仍更新 `scroll`，列表绘制使用 `top`，因此不能翻列表；详情/反馈分页正常。用户明确以鼠标/触控板为主，先前轮次有意未修改此项，鼠标滚动已验证正常。
- 本会话首次公开发布时核对：GitHub 上 drover 尚未包含完整历史修复，远端版本可能仍只返回十条；本机既有软链指向已修复代码。没有获得推送 drover 的授权，新主控如要发布该依赖修复，应先确认最新远端及对应授权。
- 主题可配置、其他平台兼容性均未实现/验证，不属于已承诺待完成项。自动测试使用假 agent；不把合成终端测试当作对所有真实 agent 应用的完整兼容性保证。
- 仓库 `AGENTS.md`（`CLAUDE.md` 链接到它）目前仍写“直接在 main 上开发”。新主控接手后需按用户最新决定落实主控分派流程与相应仓库规则；本轮不擅自设计流程或启动分派。继续遵守不干扰现有 agent、测试隔离及明确授权后才推送的边界。

以下章节为历次记录；其中“未推送”“无未完成项”等表述是当时状态，以本节汇总为接手基准。

## 2026-09-25：英文首页、中文 README 与首次公开发布

- 用户明确授权创建 GitHub public repo 并推送；公开仓库为 https://github.com/firegnu/saddle，origin 已配置。本次发布范围仅 saddle，不推送 drover 或 corral。
- README.md 为默认英文首页，顶部链接 README.zh-CN.md，中文版链接回英文；整理功能、构建/安装、入门操作、配置、快捷键与验证方式，移出首页的历史实现细节仍保留在 docs。
- 两种语言的命令和配置逐字一致，本地链接与 Markdown 代码块检查通过，`cargo build --release --locked` 通过。本轮仅改文档，不重复运行行为测试；Gitleaks/TruffleHog 检查 Git 历史未发现凭证。
- 明确记录依赖版本边界：GitHub 上 drover 的 JSON 接口尚有限制十条的版本，完整历史需要其修复版；没有把本地未推送的依赖修复当作已公开发布。
- AGENTS.md 的“没有远端”旧说明已更新为仅在用户明确授权时推送，后续发布仍需对应授权。

## 2026-09-25：真实截图下历史仍不可滚动（已修复接口截断）

- 用户再次确认 e6d2e36 的历史底边没有解决问题，上一轮关于历史区已完成的结论不足。
- 只读核对真实公开接口：drover 项目的 `drover list --json` 仅返回 T22…T13 共 10 条历史，未返回历史总数或分页字段；`drover list --json --all` 被拒绝为用法错误。saddle 展示了收到的全部 10 条，截图视口容得下这些行，最大滚动偏移为 0；不能把这一现象继续当作滚轮事件失效来修。
- 用户明确授权检查/修改 drover 后，确认 JSON 分支的 `fin[-10:]` 是截断根因。已在 drover main 修复为返回全部历史（3e66d1a，收尾 22de37d），普通 list 摘要、JSON 顺序和字段保持。saddle 沿用相同公开命令即可读取全量，不增加内部文件依赖。
- 两条真实 RED→GREEN：drover 的 11 条合成历史检查捕获最早一条丢失；saddle 的真实 CLI + 40 条临时历史整程序测试捕获接口只有 10 条，修复后完整读取、鼠标立即移动视口、正文/滚动条可到两端，合成状态文件不变、无真实 agent。新增测试默认忽略，用 `SADDLE_DROVER_BIN=… cargo test --test workflow installed_drover_complete_history -- --ignored` 单独运行。
- drover CLI 标准回归、saddle 定向跨仓库验收、clippy -D warnings、fmt 和差异检查均通过。只读核对真实公开 JSON 已返回 T1–T22 共 22 条；安装命令是既有软链，当前 saddle 自动刷新即生效，无需重新安装或重启。本轮未改真实任务数据、未操作服务/agent；两个仓库 main 本地提交，未推送，无遗留实现项。

## 2026-09-25：Queue 英文、状态配色与历史区边界（完成）

- Queue 的分组、计数、状态、空态、帮助、项目选择/路径、新增表单、详情与操作反馈、底栏输入目标统一英文；任务内容、路径、原因和公开 CLI 输出保留原文。
- 顶部保留 Manual/Auto，单独显示 Running / Awaiting / Ready / Idle / Paused，不再把空队列标成运行中。运行蓝、等待/暂停琥珀、就绪/空闲/完成绿、失败红、放弃橙，沿用 Agents 强调色；状态加粗，Loop on 绿色、off 弱化，未知任务状态中性原样。反馈依据命令结果区分成功/失败，执行中蓝色。
- RED→GREEN：只有历史记录、没有 current/awaiting/pending 时缺少空态提示；现在显示 No active tasks，完全空队列补充 No history yet，读取中/失败不误报空队列。
- 用户确认历史问题是区域下方大片留白，而非缺失旧记录。历史区固定底边延伸至任务按钮上方，显示可见范围/总数和 End 标记；短列表保持紧凑行距，长列表仍可滚到两端。只读核对 drover 公开 list 返回 10 条历史、0 条活动任务；不读取内部文件，不改真实队列。
- 59 项全量自动测试通过，1 项真实 drover 测试继续忽略；clippy -D warnings、fmt、release 构建及 diff 检查通过。验证英文界面保留中文源数据、各状态颜色、短历史底边、空态、加载/失败边界和鼠标滚动；检查三尺寸合成字符网格与弹层交互。
- DESIGN 第 21 节、README 已同步；main 小步提交、未推送。新版 target/release/saddle 已重建，需要重新启动；未操作真实 agent、未重启用户界面。本轮无未完成项。

## 2026-09-25：Queue 紧凑按钮与鼠标滚动（完成）

- Queue 的项目/队列/任务和弹层按钮统一为 Agents 的单行英文无底色样式；收紧项目头部，给任务列表更多空间，保留禁用、指针反馈与释放执行规则。
- 用户明确问题是鼠标/触控板，本轮没有修改 PgUp/PgDn。根因是滚轮被转换为上下选择，选中项没到视口边缘时内容不移动；现改为直接滚动视口，正文和滚动条都响应，保持任务选择与正常刷新后的滚动位置，显式选择时恢复跟随。
- 两项 RED→GREEN：独立 PTY 连续滚轮后第一项仍可见；末项可见时滑块仍未到底。修正滚动事件路由和最大偏移范围，任务文字预留滚动条列。回归覆盖上下两端、刷新保持、列表外滚轮不生效、选择恢复跟随及无 drover 写操作。
- `cargo test --all-targets`：55 项通过，1 项真实 drover 测试继续忽略；clippy -D warnings、fmt、release 构建、diff 检查通过。检查三尺寸合成字符网格及原生按钮/表单/弹层点击，没有启动真实 agent 或重启用户界面。
- DESIGN 第 20 节与 README 已同步，直接在 main 小步提交、未推送；新版 `target/release/saddle` 需重新启动才生效。本轮无未完成项。

## 2026-09-25：Agents 滚动到底与多 agent 排版（完成）

- 用户确认问题发生在鼠标滚轮/触控板。两个独立 RED：内容最后一行可见时滑块末尾仍为轨道；滚轮位于滚动条列时无法滚到最后一个 agent。最小化为 2 个 agent/小视口，并保留 13 个 agent 场景。
- 根因与修复：Ratatui ScrollbarState 的位置范围按可滚动偏移设置为 总行数−视口行数+1，原实现误传总行数；右侧滚动条列不在文本 list 的命中范围内，现将正文和滚动条一起接收滚轮。两项复现均 GREEN；集成测试还验证滚回顶部、正文滚动、刷新后不跳回、无 attach 副作用。已在 edf733a 单独提交。
- 多条目排版：宽窗按同组名称长度利用主行空余列，claude-demo-1 等名称可一次完整展示；确实放不下才保留补充行。类型在主行完整可见时，第二行不重复类型；窄窗和超长未知类型仍在身份行保留。每条之间加不可接入的空白行，树线连续；路径逐项保留并弱化，其他信息不删除、不折叠、不移到选中项详情。
- 新增保留全部字段、长名称只出现一次、窄窗保留类型和空白行无点击目标的 Buffer 检查；原有信息、选择、状态、按钮/弹层回归保留。53 项全量测试通过，1 项真实 drover 隔离测试继续忽略；clippy -D warnings、fmt、release 构建、diff 检查通过。
- README、DESIGN 第 19 节已同步，Rust 预览例子新增多 agent 场景；检查了合成字符网格，实际终端观感待用户反馈。main 小步提交、未推送，未操作任何真实 agent，也未重启用户 saddle；release 已重建。本轮无未完成项。

## 2026-09-25：Agents 计数、状态重点与短实例 ID（完成）

- 总数固定在顶部 Agents · N；repo 根行右侧对齐 (N)，按完整组计数，长名称省略保留数量。滚动归属/隐藏数移到底边，顶部总数不被覆盖，标题提亮加粗。
- Agents 选中底色从终端较重的 bright black 改为截图适配的薄暖灰 #302a23；仅 Agents 使用，Queue 选择样式不变。Codex 标识/类型名改薄荷青 #8ed9c1 并加粗，Claude 保持陶土橙。
- 状态加粗，使用独立真彩色：working 蓝、idle 绿、blocked 黄、stalled 橙、error 红、starting 紫，避免 ANSI 主题使语义色不明显；默认正文、边框等继续跟随终端。
- 第二行实例 ID 缩为前 6 位；完整数据及停止确认仍保留原 ID，命令身份判断不变。
- 新增计数检查先复现旧标题缺少总数，再通过多 repo/多 agent、状态排序、长名称/窄窗和空列表检查；渲染检查覆盖每种状态的颜色/加粗、Codex 强调、短 ID 及确认中的完整 ID。50 项全量测试通过，1 项真实 drover 测试保持忽略；最终标题样式另经 ui 检查通过，clippy -D warnings、fmt、release 构建、diff 检查通过。
- 已同步 README 和 DESIGN 第 18 节，检查三尺寸合成字符网格；具体终端字体与观感仍待用户目视反馈。main 小步提交，无推送；未操作真实 agent，未重启用户 saddle。release 已重建，本轮无未完成项。

## 2026-09-25：Agents 英文文案、隐藏回复入口与终端主题（完成）

- Agents 的状态、空列表、未读标记、停止确认、底栏说明及连接提示改英文；agent 标题、工具名、路径、输出/错误原文不翻译。Queue 自有文案尚未调整。
- 隐藏 Reply/Hide 按钮，停用 Agents 的 r。保留回复读取、渲染和滚动代码；已有渲染测试继续直接开启 show_reply 验证内容。Queue 刷新和 Viewer 输入路由不变。
- 共享主题改用终端默认色和 ANSI 16 色，选中背景为 bright black；树线在选中项上使用 ANSI white 保持可见。品牌色和 Viewer 内容色不变，不新增配置。Queue 的共享控件也采用这一主题。
- RED→GREEN：新增合成 PTY 测试先观察到 r 打开 Last reply，修改后验证入口消失、r 不打开面板且不调用 reply；补充英文文案保留中文源数据、终端色和英文停止确认的 Buffer 检查。默认背景让部分空格由终端清除指令产生，修正测试点击助手将空单元格还原为空格并跳过双宽续格，实际点击坐标保持一致。
- 全量 48 项测试通过，1 项真实 drover 隔离测试保持忽略；clippy -D warnings、fmt、release 构建和 diff 检查通过。三尺寸合成字符网格已检查；导出的 SVG 使用示例 ANSI 色，实际颜色由用户 Terminal 调色板决定，未把预览当成实际终端配色验收。
- README、DESIGN 第 17 节已同步，main 小步提交，未推送。未启动或操作真实 agent，未重启当前 saddle；release 已重建，重新运行生效。本轮无待实现或待拍板项。

## 2026-09-25：Agent 类型品牌标识（完成）

- 主行类型名加入同色字符标识：✳ claude 陶土橙、>_ codex 白、π pi 白、π omp 紫。以官方网页/资源为配色依据，OMP 用渐变的代表性紫色；单行字符是近似标识，不引入图标字体、图像协议或依赖。
- 仅调整主行类型样式，状态、连接、选中背景和附加信息颜色不变；未知类型保持中性原名。宽窗让出两列名称宽度容纳标识，长名称仍在后续行完整显示；中窄窗口保持原有布局。
- 纯视觉修改以 Ratatui Buffer 验证四种类型/未知类型的前景色、选中/非选中底色及状态/连接标记；检查合成三尺寸网格。全量 46 项测试通过、1 项真实 drover 测试保持忽略；clippy -D warnings、fmt、release 构建及 diff 检查通过。
- README、DESIGN 第 16 节已同步。直接在 main 小步提交，无推送，无真实 agent 操作。release 已重建，用户当前运行实例不重启。本轮无待拍板或未完成项。

## 2026-09-25：Agents 紧凑按钮、选中底色与 repo 树（完成）

- Agents 按钮改为单行英文轻量边界（如 ‹Reply r›），无底色，窄窗按两列换行；共享指针命中、松开执行和悬停/按下规则保持。Queue 与弹层仍用原有三行按钮。
- 选中 agent 的主行和附加信息整行恢复 #232931 底色；面板其余区域继续使用终端背景。repo 青色、身份信息蓝色、路径与树线弱化、标题正文色。
- SOURCE 改为 VIA，去掉 DIR/TITLE 标签但保留内容；路径只显示末两级（长路径前缀 …/）。每个 repo 下以 ├─/└─ 连接多个 agent，附加信息延续竖线；repo 根滚出视口时标题保留顶部条目所属 repo。
- 新增多 repo、多 agent 渲染检查，先复现旧字段和完整路径，再验证状态排序前后的树枝、整条选中底色、窄窗换行和滚动归属。已有回复检查确认每项信息不被 Last reply 替换；按钮检查同时覆盖普通与紧凑样式的透明底色及指针状态。
- `cargo test --all-targets`：45 项通过，1 项既有真实 drover 隔离测试保持忽略；最终渲染改动再次通过 ui/buttons 检查。clippy -D warnings、fmt、release 构建、diff 检查通过。合成三尺寸字符网格已检查；真实终端字体观感待用户目视反馈。
- 已同步 README、DESIGN 第 15 节；直接在 main 提交，未推送。未启动或操作真实 agent，未重启用户当前 saddle。`target/release/saddle` 已重建，退出旧进程后重新运行生效。本轮无待实现或待拍板项。

## 2026-09-25：按用户习惯修整 Agents 和按钮（完成）

- Agents/Queue 使用终端默认背景，选中项改用竖线和文字强调；按钮正常/悬停/按下均不填底色。所有按钮改英文标签、圆角细边框和三行高度；窄窗按行重排，四按钮超宽时采用两列，边框也可点击。危险操作仍为红色且保留确认。
- 每个 agent 的 KIND/INST/ATT/SOURCE/DIR/TITLE/DOING 放回各自条目下，长内容折行；r 展开独立 Last reply，不替换附加信息。PgUp/PgDn 在回复打开时翻回复，关闭时翻列表；鼠标仍可分别滚动两个区域。
- Queue 仅适配无底色按钮及其高度，操作语义不变；读取失败时把空间让给完整错误，不继续占用禁用的项目动作按钮。
- RED→GREEN：新增渲染检查先复现非选中条目附加信息缺失，再验证 r 打开前后两个 agent 的字段都在各自条目中。透明背景、英文轮廓、指针状态、命中区域和窄窗重排使用 Buffer 检查；合成 PTY 保留所有操作回归。
- `cargo test --all-targets`：44 项通过，1 项既有真实 drover 隔离集成测试保持忽略；clippy -D warnings、fmt、release 构建、diff 检查通过。已更新 README 和 DESIGN 第 14 节，重新生成三尺寸合成网格供检查。
- 直接在 main 小步提交、未推送；没有启动或操作真实 agent。`target/release/saddle` 已重建，退出旧 saddle 后重新运行生效。本轮无未完成项。

## 2026-09-25：Claude Design 界面重设计（完成）

- 基准为用户提供的 Claude Design 方案，技术栈不变。设计差异与真实数据边界已写入 DESIGN 第 13 节；直接在 main 小步提交，未推送。
- 统一语义配色、全行选中底色、独立连接标记、琥珀焦点和输入目标底栏。按钮有主次/危险/禁用样式、悬停/按下反馈；只在同一目标松开才执行，拖出取消并捕获余下鼠标事件，不泄露到 Viewer。
- Agents：宽窗两行扫描、中窄窗单行；完整字段集中在可滚动详情，回复在详情区展开。停止为原生确认弹层，执行中阻止重复提交。
- Queue：项目名/路径/项目选择/刷新置顶，项目操作与任务操作分开；分组计数、中文长标题省略、固定右侧状态。项目、目录、详情、新增、帮助、反馈均为原生覆盖层；不虚构登记项目的读取状态。
- 响应布局：≥140 列使用配置宽度，100–139 列最多 44 列，<100 列最多 34 列并收为 Agents/Queue 标签；保留选择、滚动及唯一 Viewer 连接。
- 弹层阻止背景点击。Ctrl-] 暂回 Agents 保留 Queue 页面/草稿，Tab 返回继续；Esc 取消。延迟的接入结果不抢走 Queue 或停止确认的焦点。自动刷新保留原数据与选择，切换项目才清空。
- RED→GREEN：响应布局、按钮释放执行、按钮拖出后无鼠标泄露、延迟 attach 不抢表单输入、停止中不重复提交、暂停编辑后草稿保留。整程序 12 项回归通过，全部使用假 CLI 和独立 PTY。
- 全量 `cargo test --all-targets`：42 项通过，1 项依赖本机 drover 的既有隔离集成测试保持默认忽略，本次未重跑。`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、release 构建及 diff 检查通过。
- 新增 `cargo run --example ui_preview -- /tmp/saddle-ui-preview`：直接导出 Ratatui 合成 Buffer 的 SVG/文本（160×48、120×36、80×24 及项目/表单/停止/回复）。已检查字符网格、颜色属性、CJK 状态列与极小尺寸无越界；浏览器阻止本地 SVG 页面访问，未绕过限制，因此没有把浏览器像素检查记为通过。
- 已更新 README。可执行文件 `target/release/saddle` 已重建；正在运行的旧进程需要退出后重新启动。没有启动或操作任何真实 agent，也没有修改真实项目/全局配置。真实终端字体与观感仍由用户目视验收。
- 本轮无待实现项，无需用户拍板。

## 2026-09-25：按钮视觉修整

- 仅调整共享按钮渲染：去掉方括号和亮蓝底，改为低对比度深灰底、浅色操作名、弱化快捷键；增加按钮间距和换行留白。操作和快捷键保持不变。
- 纯视觉修改，以独立 PTY 的原生按钮点击、表单、停止确认和窄窗重排回归作为验证，不编造失败逻辑测试。7 项 workflow 测试、clippy、fmt、release 构建通过；依赖真实 drover 的测试本次未重复运行。

## 2026-09-25：修复 Queue 项目入口，加入原生按钮

- 根因已复现：从 saddle 目录执行 `drover list --json` 返回“尚未接入 drover”；原实现只读启动目录，没有沿用登记项目列表，而且完整错误被截在页脚、状态仍显示读取中。
- 用户已明确授权只读 `~/.drover/projects`。默认优先 `queue.cwd`、已登记的启动目录、登记首项；无登记才尝试启动目录。点击「项目」从原生列表切换，支持刷新登记清单和手动目录；不读取 `.drover.conf` 或任务内部文件，不自动初始化仓库。
- 读取失败在内容区折行展示完整错误，支持滚动，禁用过期状态上的写操作；成功后清除读取错误。切换项目会丢弃旧 worker 的结果，执行操作时禁止切换。
- Agents 新增接入、回复、排序、停止及确认/取消按钮。Queue 新增项目、刷新、详情、新增、放行、下一件、暂停/恢复、循环、帮助按钮；表单可点击字段、保存/取消。窄窗按钮自动换行，全部由 Rust/ratatui 绘制。
- 自动检查：32 项通过，1 项真实 drover CLI 隔离测试默认忽略、已单独执行通过。clippy（拒绝警告）、fmt、release 构建、diff 检查通过。
- RED→GREEN：长路径错误不再截断/假装加载；从未接入目录启动可自动加载登记项目；鼠标切换后动作只作用于选中项目；表单和停止确认可点击。回归还覆盖缩窄窗口后的按钮命中、失败时阻止过期动作、登记缺失/去重/读取错误及手动修正目录。
- 实际只读核对：登记的 drover 项目公开 `list --json` 正常返回。没有修改登记清单或真实项目，没有启动、接入或操作真实 agent。新版可执行文件为 `target/release/saddle`，已运行的旧进程需退出后重新打开。

## 第 1 步（原生 UI 修订版）：实现与自动验收完成

2026-09-25，直接在 main 分步提交，未推送。按用户后续要求，原来的「Queue 先嵌入 drover board」方案已经作废。

### 当前实现

- Agents：ratatui 原生面板。公开 ls/status 每秒后台刷新；项目分组、状态排序、完整字段和窄屏折行、选中项保持、未读标记、滚动条/隐藏数量、回复分页、x/y 停止确认。
- Queue：ratatui 原生列表、详情、帮助、操作反馈、标题/正文新增表单。公开 `drover list --json` 提供 mode、paused、current、awaiting、pending、history；g/n/p/l/a 通过公开 CLI 执行放行、下一件、暂停/恢复、循环和新增。失败保留新增草稿，执行中不重复提交。
- Viewer：唯一使用 PTY 的窗格，仅运行公开 `corral attach`。Rust/alacritty_terminal 解析、ratatui 绘制颜色、中文、光标、鼠标和粘贴；切换等待旧 attach 退出，再接入最新选择。已有其他 attach 时拒绝接入，agent 消失后不自动换人。
- 所有 saddle UI 都是 Rust。生产代码没有 `board` 或 Python UI 调用；不启动外部编辑器。测试中的 Python 文件只是假的 CLI/字节流边界，不作为 UI 实现。
- Queue 的 `queue.command` 已删除并在解析时拒绝；新配置为 `queue.drover` 和可选 `queue.cwd`。未指定 cwd 时从 `~/.drover/projects` 选择默认项目（用户已授权只读该清单）。
- TOML 配置、三窗格缩放、焦点路由、后台命令取消/超时、终端退出恢复、README 和 `--help` 已同步。

### 验证

全部通过：

- `cargo test --all-targets`：32 项通过；依赖本机 drover 的 1 项默认忽略，已单独执行通过。
- `SADDLE_DROVER_BIN=/Users/firegnu/.local/bin/drover cargo test --test workflow installed_drover_cli -- --ignored`：真实公开 CLI 在隔离 HOME/临时项目中驱动原生 Queue，验证正文、暂停/恢复、循环开关、多行中文新增和缩放。不调用外部看板，不启动真实 agent。
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `cargo build --release`：可直接运行 `target/release/saddle`。
- `git diff --check`
- 生产调用点检查：仅 `src/viewer.rs` 调用 `Session::spawn`；无 `board`、`python`、`queue.command` 调用。

已有合成原型 `cargo run --example compare_parsers` 对比 alacritty_terminal 与 vt100 的中文、真彩/256 色、光标、清屏和滚动区；选择前者的理由是终端查询的事件回传能力。

RED→GREEN 包括布局、配置、公开 JSON、刷新选中项保持、输入路由/编码、PTY 输入、鼠标坐标和网格绘制；整程序回归覆盖接入/切换顺序、ATT、状态变化/新增/消失、回复、停止确认与取消、输出压力和退出恢复。原生 Queue 新增 RED→GREEN 覆盖公开数据/动作、选择/新增状态机、拒绝外部 UI 配置、禁止 Queue 按键进入 PTY。

### 边界与记录

- 按用户约束，未启动、attach、输入或停止任何真实 agent；真实 Claude Code 显示与真实 corral 生命周期未验证，不用合成测试代替该结论。
- 本机 Terminal GUI 控制被工具拒绝，没有绕过限制；交互验收在独立 PTY 中完成，没有录屏。
- 核对 CLI 时误将 `drover init --help` 当成帮助命令。确认创建时间及精确内容后，已删除它意外创建的 saddle 配置、空的 `--help` 交接目录和末尾新增登记项，并恢复 `.gitignore`；后续所有 drover 写入测试都使用隔离 HOME/临时项目。
- 无待用户拍板的实现问题。原生 Queue 只显示公开 JSON 已有字段；更细的判据数据若将来需要，应由 drover 增加公开 API，saddle 不读取内部文件补齐。
