# 进度

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
