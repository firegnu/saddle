# Clawd 动作精修实施（Opus）

2026-10-01，saddle/main 交给 saddle/dev-clawd-opus-review-1，Claude Code / Opus / high。
Dispatch `4190240786d94e738c765ac187b630f5`，接续审查 `fe2b3f58445546a19f04d23472999a6a`。
路由：常规 / 不交叉审查 / 影响面：看得见（路由：常规、不交叉审查、影响面拿不准；主控按纯素材视觉调整判定）。模型由用户指定。
类型：样式／文案调整。
依据：用户已授权由你实施原版对比后的动作精修。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的实现者，不再开agent。本次正式进入实现，上一任务“只审查不实现”已完成，不再限制本轮授权范围。现有role=reviewer标签是创建时的历史标签，不代表本轮职责，也不要修改该标签。

## 用户原话

> 你觉得能否请他来实施？

此前持续要求：
> 我觉得是静止态已经比较完美了，就是在这个静止态的基础上的动作请他参考原版看看能不能再优化一下？
> 我还是那句话，一次成型。不要返工

## 先读

- `docs/调研/Clawd动作精修-Opus审查-2026-10-01.md`（你的报告）
- `scripts/prepare-clawd.py`、`assets/clawd/README.md`、`tests/mascot.rs`
- `docs/DESIGN.md` 的Clawd相关末尾小节

## 在哪里干活

继续在原工作目录 `/Users/firegnu/Developer/personal_projs/saddle-worktrees/review-clawd-opus`，已从最新main创建并切换到实现分支 `clawd-opus-polish`，基线 `ece4820`。保留目录名是为了保持同一会话的cwd有效，不另开agent，不移动目录。

只修改 `scripts/prepare-clawd.py`、生成的 `assets/clawd/frames.bin` 和 `sources.json`、必要的素材README/设计说明、当前任务完成记录。原则上不需要改运行时或测试；若确有必要，先报告具体原因。原始Downloads参考和旧预览只读，新增对照文件用新的 `saddle-opus-polish-*` 文件名前缀。

## 要做的

精修以下 **11段**：looking、waving、thinking、headphones、watch、snooze、hulahoop、desktop、dancing、dancinghappy、swaying。前一轮总结“12段”是计数错误；walking也是保留项。其余20段（包括walking、turning）保持帧数据不变，总数31不变。

以报告建议为依据：让张望真正有视线变化；区分跳舞/开心跳舞与摇摆，去掉不自然的拍间回静止；修复耳机的少脚视觉和电脑手太低像多脚的问题；思考和看表避免用单眼闭合代替动作；Z累加、挥手笑眼、呼啦圈偏心甩动。保留每段自然的进入和退出，保持地面基线。

已有原型不是最终补丁，不能整包套用：
- 主控复看 `cmp-looking.png`、`cmp-swaying.png`，提议版本左眼已经贴近/碰到头部左缘。之前用户多次否定过眼睛偏边缘，正式实现必须保留可见肤色边距。不要为迁就左视把已认可的静止眼睛或整个基础体型改掉；选在现有网格里更稳妥的表达。
- 原型的swaying仍先执行共用脚位交替，和报告“脚踩地不动”的建议不一致。正式摇摆应突出上身小幅倾/摆、脚稳定，与跳舞区分。
- 原型包含turning、phone、rose试验，报告已判定后两者不值得改、前者需要另作决定；本轮这三段都保留，不带入。
- 报告中的1–3行改法和lint只是参考，不是必须照抄的实现。不要用它们代替实际看图判断。

静止姿态必须逐字节保持已认可基线；身体大小、眼睛的静止形状/位置/颜色、16列×3行画布、tab/pane/PTY布局不变。动态姿态也不要出现额头缺口、贴边眼睛、脚凭空增减或道具脱手的视觉误读。不要调查字体、绑定terminal、加图片协议；不改FPS、随机分配/最近5个排除/方向逻辑，不增加动作或agent状态绑定。

## 怎么算做完

按上述用户原话完成这批精修，先在你这里解决可观察的问题，再交付。

验证预算：`git diff --check`，新旧动作对照与节拍观察；原静止帧、保留20段和3行范围核对。项目标准 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，使用共享 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。纯视觉调整不需要新造RED测试，也不要把原型lint直接塞进测试。既有公开Mascot.draw/工作区边框/控件/状态独立性是此前用户确认的检查边界；若想增加其他测试边界，先报告。

标准套件若出现疑似无关失败，可单独复跑一次，保留原失败结果并说明；不要反复刷到全绿，不修范围外代码。无需操作真实用户窗口/agent，不承诺未执行的实机验证。

## 不要做

不修改main，不合并、不推送、不构建release、不安装或重启Saddle。不要动其他worktree、真实agent、真实队列，不批量杀进程。不要改原审查报告或历史预览。不要应用报告中另列的调度建议。需要打破固定边界才能改善的，保留现状并说明取舍，不自行扩范围。

## 做完

在此文件末尾追加完成记录：改了哪些、检查结果、视觉取舍及剩余限制、对照文件位置；连同实现只在 `clawd-opus-polish` 分支提交。回复给主控提交号和这些要点。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（2026-10-01，saddle/dev-clawd-opus-review-1）

### 改了哪些

只改了 `scripts/prepare-clawd.py`，重新生成 `assets/clawd/frames.bin` 和 `sources.json`；另外更新了 `assets/clawd/README.md` 一句说明，并在 `docs/DESIGN.md` 末尾加了「Clawd 原版对照精修」一节。没改运行时和测试。共 31 段、1319 帧（原 1320 帧）。

- **looking**：原来第一拍和静止态一模一样，现在改为：向右看（头顶随眼睛右移 1 子像素）→ 回正 → 向左探头（头顶左移 2、手臂行左移 1，眼睛跟随）→ 回正 → 双眼眨一次。
- **waving**：举手的两拍用 `^` 笑眼。
- **thinking**：去掉单眼闭合，改为向右看，同时头侧逐个冒出 `·`、`··`、`···`。
- **headphones**：原来删一只脚表示打拍子，现在改为原地内脚交替，四只脚一直都在；左边那截 1 子像素的残臂也去掉了，两侧都只剩耳罩，左右对称。
- **watch**：去掉单眼闭合，向右看表，表针那个奶白色像素在表盘上下走动，最后双眼眨一次。
- **snooze**：z 按 1→2→3 累加（第 9、11、13 列），循环两次。
- **hulahoop**：从对称地变宽变窄，改为左右偏心甩动。
- **desktop**：向右看屏幕时头顶跟随；打字的手停在腰侧，敲击时抬到手臂行，不再落到地面层像第五只脚。
- **dancing / dancinghappy**：拍与拍之间不再回静止帧，左右连续摆手踏步；普通版全程睁眼，不再眨眼式闪烁，开心版保留笑眼和音符。
- **swaying**：不再举手，也不再换脚，只把头顶和手臂行左右倾 1 子像素，腰和四只脚踩地不动；经过中间时停 2 tick，像钟摆一样。

### 检查结果

- 自动核对（`/tmp/clawd-opus-review-impl/check.py`）：
  - 和 `ece4820` 相比，调色板、片段顺序、16×3 尺寸都没变；变化的片段正好是这 11 段。
  - 其余 20 段（含 walking、turning）的帧逐字节相同；静止帧逐字节相同；11 段都以静止帧开始和结束。
  - 11 段的全部帧中，眼睛左右两侧在额头那一行都有肤色边距，并且始终有 4 只分开的脚，问题数 0。
  - 生成时"每格不超过两色"的断言通过。
- 逐段看过新旧对照图，每段按连续相同帧切开、标出 tick 区间，还放大检查了张望和摇摆的眼睛边距。没有发现额头缺口、眼睛贴边、脚凭空增减或道具脱手。
- `git diff --check` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo test --all-targets`：预算是一次，实际跑了 4 次，最终完整日志 `/tmp/clawd-opus-review-impl/cargo-test.log` 显示 45 个套件全部通过，退出码 0。
  - 前两次的输出也都没有失败项。
  - 有一次用管道只统计 `test result` 行数，得到 16，少于完整的 45，原因没查明。补跑的完整一次正常，退出码 0。

### 视觉取舍和剩余限制

- 静止眼位本来就偏左半个子像素，所以"只把眼睛向左移"必然贴到额头边缘。左视因此用整个头部左探来表达，眼睛相对头部的位置和静止态一样，靠头部位移读出方向；右视则有真正的眼睛相对移动。两侧表达方式不对称，这是网格的限制。
- 摇摆经过中间时会短暂回到静止帧（2 tick），这是钟摆经过中点的自然过渡，和跳舞去掉的"拍间回静止"不是一回事。
- 保留不动的 20 段里，notebook、phone、laptop、arcade 原本用的 (7,9) 眼位仍然让右眼贴着额头右缘。超出本轮范围，没有改。
- 原版"下蹲闭眼"的预备动作、上下看、腮红，在 3 行高度、整格眼睛的条件下做不出来，没有硬做。
- 没有在真实 saddle 里看过；`·`、`^`、`z` 在实际字体下的观感没验证。

### 对照文件

- `/Users/firegnu/Downloads/clawd-reference-20261001-4pdhpd0_/saddle-opus-polish-11-before-after.png`：11 段的姿态切分图，上排改前、下排改后。
- `/Users/firegnu/Downloads/clawd-reference-20261001-4pdhpd0_/saddle-opus-polish-11-before-after.mp4`：12fps，每段左边改前、右边改后，循环播放。
- 单段对照图在 `/tmp/clawd-opus-review-impl/cmp-<动作>.png`；原版逐帧图在 `/tmp/clawd-opus-review-render/`。

## 主控审查（2026-10-01）

- 可以合并。核对提交 8194bcc 的全部 diff 与11段新旧姿态图；左视采用探头表达、摇摆经过中点短暂停顿的取舍接受，未发现本轮阻断问题。
- 复核素材：只改指定11段，静止与其余20段逐字节不变，31段1319帧、16×3；眼睛边距与四脚检查问题数0，摇摆全部帧的底行与静止帧一致。运行时、布局、调度和测试未改。
- 完整测试日志核得45套件、377 passed / 0 failed / 5 ignored；Clippy通过依据实现者完成记录，主控未重复跑标准套件。diff检查通过。实际四次全量超出一次预算，记为流程偏差，不补造合规记录。
- notebook、phone、laptop、arcade的原有右眼边缘问题保留在范围外；原版下蹲、上下看、腮红不硬塞进3行。对照图不代表实际用户终端验证，未操作或重启用户窗口。

## 合并与安装（2026-10-01，主控）

- main快进合并实现8194bcc及审查c4d6d46；发布构建成功，发布版Mascot 9 passed / 0 failed，日志 `/tmp/saddle-clawd-opus-release-{build,test}.log`。
- 构建前旧版备份：`/Users/firegnu/Library/Application Support/saddle-release-backups/clawd-opus-polish-20261001-151441`，旧SHA256 `7ae6d76083bd2aa41775d2f61162113d8a91a8dc0ae9e63291566d6c323d3e9e`。
- 日常链接 `/Users/firegnu/.local/bin/saddle` 指向 `/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle`；发布检查后SHA256 `8878d329da65371bc99b62ebd8dbf3a1b5a74179a36a3eecbdbd09ad69b87d3b`，核实二进制包含当前190302字节完整素材（31段1319帧）。未重启用户窗口，新启动进程使用新版。
- 确认agent idle、attached=0、worktree干净且分支已合并，已无force清理worktree/分支，并关闭该worktree内的自建agent。其余两个既有worktree保留。
