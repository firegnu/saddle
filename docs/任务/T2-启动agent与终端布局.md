# T2：内置启动 agent、终端标签页与分屏

2026-09-26，saddle/main 交给 saddle/dev-t2-terminal（Codex，重：gpt-6-astra / xhigh；用户明确指定 Codex）。
路由：重 / 交叉审查要 / 影响面：碰要害（route.py：重、要、碰要害；多 PTY 输入路由与异步启动/关闭生命周期）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话 / 验收

> 现在用户要开一个新的corral托管的一个agent，用户需要在saddle外部的terminal中进入repo所在的文件夹并输入corral start ....我觉得不是很方便，我想在sddle中内置这个功能，但是如何交互如何设计入口，我还没想清楚。主控先和家长把设计聊好并取得一致了。再委派agent出去干。顺便可以讨结合这个功能讨论一下现在的agent区域可以split   right，down，up, left这种功能以及agent区域的开tab的能力，我的理解这了这两个功能应该是有关联的，先讨论，等设计明了了，再派出agent去实现

主控展示推荐草图并说明 tab 管一组分屏、命令表单和关闭只断开显示后，用户回复：

> 委派codex去做吧。

## 先读

- AGENTS.md。
- docs/DESIGN.md 第 27 节（本轮授权方案），第 3–7、13 节的现有交互约束；冲突处以第 27 节为准。
- src/app.rs、src/viewer.rs、src/pty.rs、src/layout.rs、src/ui.rs、src/input.rs、src/corral.rs 及对应测试，只按需要读。
- 公开 `corral start --help`、`corral guide`；不读 corral/drover 仓库或内部数据文件。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t2-terminal-layout
- 分支：t2-terminal-layout（主控从 main 建好）。
- 修改范围：本任务需要的 src/、tests/、Cargo.toml/Cargo.lock（确有依赖需要时）、README.md/README.zh-CN.md（以仓库实际文件名为准）、docs/DESIGN.md 第 27 节和本任务完成记录。不要修改 HANDOFF.md 或其他任务文件。
- 所有 Cargo 命令使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 要做的

实现 DESIGN 第 27 节：New 表单调用公开 corral start；右侧 tab 每个包含一组可四向分屏的终端；已有 agent 可接入活动窗格、在新 tab 或分屏打开，已打开的跳到现有位置；关闭显示不停止 agent。保持左侧功能和已修复的 Tasks 原文/编辑入口。

按既有风格实现必要的可点击入口、焦点和帮助说明。普通实现取舍自行决定并在完成记录说明；如果必须改变已授权交互方案或需要其他仓库新增接口，停下来报告，不自行扩展。

## 验证预算

- 按 AGENTS.md 的轻量 TDD：先添加或定位针对目标行为的自动化检查，运行得到确因功能缺失而失败的 RED，再最小实现到 GREEN。
- 重点边界覆盖多窗格输入归属、异步启动/接入期间切换或关闭目标、关闭 tab/窗格/退出只断开 attach。按实际实现选择检查，不做无关覆盖矩阵或缺陷注入。
- 完成后运行一次项目标准检查：`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`（均带共享 target），以及 `git diff --check`。不反复跑已通过的全套。
- 使用假 corral、临时目录和合成数据；不启动真实 agent，不录屏，不做真实队列操作。命令在前台跑完。

## 不要做

- 不操作用户现有 agent，不发送消息、按键、停止或接入打字；不读 corral/drover 内部文件，不改其仓库。只读 ~/.drover/projects 的既有授权不扩大。
- 不新增会话恢复、布局持久化、shell 会话、自动 worktree/自动派活、独立模型/权限选择器，不改 Tasks 业务规则，不修无关遗留问题。
- 不按项目名或路径批量杀进程；自己起的进程只按记录 PID 停止。
- 不合并到 main、不推送、不更新已安装 release。只在 t2-terminal-layout 分支提交。

## 做完

在本文件末尾追加「## 完成记录」并提交：做了什么、验证结果与 RED/GREEN、实现取舍、没做的事，各几句话。回复只写这些、提交 SHA、有没有要主控决定的事。全部做完后，回复最后一行写 DONE。

## 完成记录

- 已实现 New 原生启动表单（当前 Tasks 目录、已登记目录轮换／手填、名称、带参数命令、可选多行首条消息、六种打开位置、完整可滚动调用预览）。显式提交后异步调用公开 corral start，使用返回 name 再做 status 检查并接入；失败保留草稿，忙碌时禁用重复提交。Agents 的 Enter／点击接入当前窗格，Open 菜单可新 tab 或四向分屏，已打开的 agent 跳回现有位置。
- 新增内存中的 tab／递归等分布局，每个窗格独立 Viewer／PTY；可点击标签、标题、内容、关闭控件，键盘／粘贴只进入活动窗格。标签切换保留接入，关闭窗格／tab／退出只断开自有 attach。稳定窗格 ID 与修订号隔离启动／接入结果；目标关闭、替换或重新选择原 agent 后，旧结果不会重定向。后台启动完成不抢 Tasks 表单焦点。Tasks 原文与编辑入口回归通过，Queue 唯一改动是将已有文本换行函数设为 crate 可见供新表单复用，业务规则未变。
- RED/GREEN：`terminal_tabs_and_splits_route_input_and_close_only_owned_attaches` 初次因 Open 菜单缺失失败，`new_agent_previews_exact_arguments_and_keeps_failed_draft` 初次因 New 表单缺失失败，均在实现后通过。自查补到的 `reselecting_the_displayed_agent_cancels_an_inflight_replacement` 先观察到 A 被断开、字符 41 进入 B，再修正请求失效逻辑转绿；打开菜单的 Ctrl-] 也先复现为误选向上分屏，再优先处理 Ctrl-]/Ctrl-5 并限制纯数字选择，原检查转绿。未做缺陷注入。
- 验证：带共享 target 的 `cargo test --all-targets` 已通过（115 passed、2 ignored，原有 installed-drover 手动集成项未运行）。随后自查时序修复的受影响目标 `cargo test --test workflow --test viewer --test terminals` 通过（31 passed、2 ignored，新增第 116 个自动化检查）；最后菜单 Ctrl-] 修复的完整 tab／split／输入／关闭目标检查再次通过。`cargo clippy --all-targets -- -D warnings`、`cargo fmt --all --check`、`git diff --check` 通过。已有 UI 断言仅随新增按钮换行、右侧新增一行 tab 和异步 PTY 创建更新；异步流程检查等待可见的表单完成状态后再点击。布局合成检查覆盖四个方向、关闭合并、嵌套及 160×48／120×36／80×24／极小尺寸。
- 实现取舍：新增 `terminals.rs` 管布局与请求归属，`launch.rs` 管 New／Open 原生交互，保留单 Viewer 和现有静态预览入口。命令通过 shell-words（原 lock 已有的成熟库，增加直接依赖）拆 argv，不经 shell、不追加权限参数；目录以 Ctrl-P 轮换登记项。提交时预留显示目标，失败可留下空窗格；最后一个 tab 关闭后保留一个空 tab。过窄／过矮时暂时只绘制一支，放大恢复；后台 PTY 保留最后尺寸。细节、键位及理由已写入 DESIGN 第 27 节，中英文 README 已同步。
- 全部使用假 corral／drover、临时目录与合成数据；只读取公开 corral 帮助。未启动真实 agent、未操作用户 agent 或真实队列、未读 corral／drover 内部数据，未录屏、未更新安装版本，未改 HANDOFF 或其他任务文件。未合并、未推送；仅在 t2-terminal-layout 提交。无须主控新增设计裁决，交主控审查。

## 第一轮返工记录

2026-09-26，按主控认可的交叉审查必须改 1、2，在原分支基于 c35daa2 修复；主仓库审查文件只读。

- 必须改 1：窗格统一提供 `input_session`，只有没有待处理目标、已有具名当前 session 且没有退出中的 attach 时才允许输入。键盘、粘贴、鼠标都沿 App 原有 `focused_session` 入口调用此判断。重复选择等待 status 的 B 仍可定位原窗格，但不会把输入发给保留着的 A；重新选择真正已连接 A 时取消替换的行为和原有断言保留。
- 必须改 2：`reserve` 同步调用 Viewer 的 `cancel_pending`，清除旧 pending 并更新代次；PTY worker 携带创建时的代次，结果过期时中断自有 attach 并保留所有权直到回收，不将其登记为当前 agent 或可输入目标。过期失败结果也不会覆盖当前状态。取消待接入工作本身不打断正常保留的现有 session；关闭、重选和 agent 消失沿同一失效机制处理。
- RED → GREEN：新增 workflow 回归先记录到 `input p/a 5a`、发给 B 的粘贴和鼠标字节也落到 A，输入判断修复后通过。新增 pending 阶段回归先记录 `attach p/a → detaching p/a → detached p/a → attach p/b`（此时已预留 C），修复后 A 回收期间不再启动 B，C 随后正常接入。新增未接收 spawn 的同项回归先记录 `target=Some(1), input=true, reaped=false`，修复后即使 C 的 status 失败，旧 B 也不再是目标／输入 session，并完成回收。三个检查均先因目标缺陷失败，再通过；用假 CLI 事件和待退出文件控制阶段，未以产品延时掩盖问题，也未强制延迟 OS spawn。
- 增量验证：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test --test workflow --test viewer --test terminals` 通过（34 passed、0 failed、2 ignored；忽略项仍是既有 installed-drover 集成测试），其中包含保留的重选 A、tab／split、关闭与表单焦点回归；同一共享 target 的 `cargo clippy --all-targets -- -D warnings` 及 `git diff --check` 通过。没有重复全部标准测试，没有修改或删除任何已有断言。
- 8 项已认可实现取舍全部保留，未改布局、New 交互、命令参数或 Tasks 业务；未改主仓库审查文件、设计或其他任务文件。仅用假 CLI、临时目录、合成数据，全部命令前台完成；无真实 agent／队列操作、无新委派、无合并或推送。无新增待裁决事项，交主控增量复核。

## 主控审查

- 初审标准检查 116 passed、2 ignored，Clippy 与差异检查通过；独立交叉审查发现待接入输入误路由、过期接入未取消两项必须改，主控认可后交回原开发者。
- `46d8abb` 修复两项，主控增量检查 34 passed、2 ignored、Clippy 与 diff 检查通过；原审查者在该 SHA 复核原两份探针及过期 spawn 回收检查，三项均通过，无剩余阻挡或新增建议。
- 结论：通过，已合并 main。tab 布局、等分、窄窗折叠、后台尺寸、直接 argv、目录轮换、失败预留空窗格、最后空 tab 八项取舍均认可；详细证据和逐项裁决见 `T2-主控与交叉审查.md`。
- 按用户授权继续推送和清理收尾，公开 T2 标为完成待用户放行，不自动 go/next。真实终端观感由用户体验。
