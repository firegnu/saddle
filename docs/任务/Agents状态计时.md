# 任务：Agents 计时与当前状态一致

2026-10-03，saddle/main 交给新建 saddle/dev-state-timer（Codex，常规：gpt-6-astra / high；实际带后缀名字以 Corral 回执为准）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由三项均拿不准，主控按状态展示修复确定；不改变生命周期状态机）。未选择遥测，不创建 Tasks run。
类型：Bug 修复
依据：working 行末时间不断在 0s/1s 间跳；用户批准将计时改成当前状态持续时间。
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的 agent：照本文件做，不再开其他 agent。

## 先读
- AGENTS.md。
- src/ui.rs 的 agent_rows、activity、short_time，src/agents.rs，src/corral.rs。
- 若公开时间字段不足，读取 crates/corral-core/src/events.rs、cli.rs 和相关契约测试。
- docs/DESIGN.md 的 Agents 相关规则。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/agent-state-timer；分支 agent-state-timer，从 main d24965f 建立。
- 范围：Agents 状态计时、公开状态字段消费、直接相关测试与设计说明；必要时在本仓库 crates/corral-core 增加最小公开状态起点字段。不修改原独立 Corral 仓库。
- 另一实现者正在 pet-images 分支做宠物。你不动宠物素材、mascot、图片渲染集成或 app 绘制循环。src/ui.rs 仅改 Agents 行/活动计时相关位置；docs/DESIGN.md 仅增加本次状态计时说明。主控负责顺序集成两条分支，不合并未完成宠物分支。

## 要做的
用户确认的显示语义：
- working：本轮已工作多久，持续递增。
- idle：本轮结束后已空闲多久。
- waiting：进入等待输入状态后过了多久。

同一行时间不能随终端输出/动画重绘归零。若详情活动行也显示时间，避免相同状态出现互相矛盾的含义。

已有 working 起点为公开 turn_started；现有 last_event_at 是最后事件时间，不保证是进入状态的时间，不能直接冒充。选最小可靠实现；必要的公开状态起点扩展保持向后兼容、底层无反向依赖，不改变 Corral 状态流转。确实缺少可信起点时如实显示未知，不捏造准确时长。先更新 docs/DESIGN.md，再实现。

## 怎么算做完
用户原话：
> 但是现在就是agent在working的时候在0和1之间来回切换。

用户对上述三种状态计时方案的批准原话：
> 就是这个意思。开干吧

## 验证预算
- 按 AGENTS.md 先加自动检查，确认因目标计时问题失败，再最小实现、GREEN 及直接相关回归。
- 标准检查各一次：cargo test --all-targets；cargo clippy --all-targets -- -D warnings；git diff --check。若改 corral-core，增加该 crate 的直接相关测试一次。
- 共享 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。并行宠物构建会覆盖同名顶层 binary；本任务所有 cargo 命令显式使用 --target aarch64-apple-darwin，确保宿主调用型测试使用该候选路径。不要覆盖正在安装的不可变版本目录。
- 无关失败最多单独复跑一次；保留首轮失败，不扩展覆盖矩阵、不重复全套。

## 不要做
- 不改宠物功能、任务流转、遥测业务、agent 状态判定或警示阈值。
- 不操作真实任务队列，不创建/停止/发送用户 agent，不读 Corral 私有状态文件作为宿主实现；测试用合成数据。
- 不切换安装，不重启界面，不修改共享发布入口。
- 不按名字/路径批量杀进程；只清自己起且记录 PID 的进程。
- 不合并 main、不推送；只在本分支提交。若需要改变已批准业务语义，先停下报告。

## 做完
在本文件末尾追加完成记录并提交：原因、改动、验证结果、取舍和未完成项。回复附提交号。命令都在前台跑完，全部做完后，回复最后一行写 DONE。


## 完成记录（2026-10-03，被委派实现者）

- 原因：Agents R1 行末从 `last_output` 计算无输出时长，终端持续输出会反复回到 0s／1s；DOING／ASK 原来统一从 `turn_started` 计算，导致同行与详情含义不同，waiting 也错误包含等待前的工作时间。
- 改动：先补设计说明。working（含原 working 的 stalled 警示）使用公开 `turn_started`；idle／waiting 使用新增可空公开 `state_started`，R1 与活动行复用同一时长。没有可信起点显示 `—`。仅改 Agents 计时相关渲染和字段消费，不改 `src/agents.rs` 的状态判定、警示阈值或 app 绘制循环。
- 底层：本仓库 corral-core 在事件归约后仅于状态实际改变时记录起点，`status` 加字段；事件格式和状态流转不变。派生 cursor 从 2 升到 3，遇到旧缓存重放原事件，防止沿用旧客户端留下的过期扩展字段；启动期公开状态覆盖为 starting 时不暴露 idle 的起点。重复 PermissionRequest／Notification／Stop、compact、子会话事件和重复读取均不重置当前状态起点。
- RED：working 渲染回归期望 `30s`、实际 `1s`；状态公开契约期望起点 `10`、实际 null；idle／waiting 渲染回归期望 `25s`、实际 `0s`，均先于对应实现运行并因目标缺陷失败。底层初次 GREEN 尝试另发现测试误把仅列元数据的 `ls` 当作完整 status，已将该断言改为重复读取公开 status，没有扩大 ls 接口。
- GREEN／直接回归：working 单测通过；公开状态契约单测通过；`cargo test --target aarch64-apple-darwin -p saddle --test ui` 30 项通过；`cargo test --target aarch64-apple-darwin -p corral-core --test protocol` 11 项通过。覆盖输出／动画刷新、等待与空闲的新起点、旧字段缺失、重复事件、子会话过滤、旧 cursor 重建及原有 UI 样式。公开 Client 字段消费检查随全套通过。
- 标准检查各一次，均设置 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`：`cargo test --target aarch64-apple-darwin --all-targets` 通过（609 passed，0 failed，9 ignored）；`cargo clippy --target aarch64-apple-darwin --all-targets -- -D warnings` 通过；`git diff --check` 通过。日志：`/tmp/saddle-agent-state-timer-all-targets.log`、`/tmp/saddle-agent-state-timer-clippy.log`。所有命令前台等待结束；使用指定 target 下候选二进制，测试只用合成数据与隔离目录。
- 取舍／边界：旧 Corral 缺少状态起点时 idle／waiting 如实未知；working 仍可使用原有公开起点。9 个既有 ignored 检查（外部旧版客户端兼容性、安装形态及独立插件示例）未额外运行，不声称已验证。没有安装切换、界面重启、真实队列／用户 agent 操作，也未改宠物分支相关内容。实现范围无未完成项；仅提交本分支，等待主控审查与集成。

## 主控审查（2026-10-03）

- 结论：可以合并。核对 2df04b8 的完整 diff，变更限计时展示、最小公开字段与相关文档/测试；状态判定、警示阈值、宠物和任务业务未改。
- 同意状态变化时记录 state_started、派生 cursor 升版后重放、缺失起点显示未知的取舍；同状态事件不会重置 idle/waiting，working 始终沿用本轮 turn_started，行末与活动行一致。
- 主控在同一候选 worktree、共享目标目录的显式 aarch64-apple-darwin target 复跑标准测试：609 passed、0 failed、9 ignored；Clippy -D warnings 与 diff 检查通过。日志 /tmp/saddle-agent-state-timer-review-all-targets.log、/tmp/saddle-agent-state-timer-review-clippy.log。
- 未验证已忽略项，未切换安装或重启实际宿主；当前安装 7755bb8 尚不包含本修复。无阻断意见，不增加交叉审查。
