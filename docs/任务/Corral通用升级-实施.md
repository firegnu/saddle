# 任务：实现 Corral 通用升级

2026-10-03，saddle/main 交给新建 saddle/dev-corral-upgrade（Codex，重：gpt-6-astra / xhigh；实际名字以公开 start 回执为准）。
路由：重 / 交叉审查要 / 影响面：碰要害（路由模型 jev-1.13.0：重，交叉审查与影响面拿不准；主控依据进程资源所有权、并发交接与在途数据判为碰要害，需要独立审查）。
类型：功能变更
依据：用户确认一致方案主要改 Rust Corral 内核、少量公开消费者接入，明确回复“批准”实施；实际部署和首次过渡另行确认。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的实现者，照本文件做，不再开其他 agent。本链路未选择遥测、不创建 Tasks run。

## 先读

- AGENTS.md。
- docs/DESIGN.md 的“Corral 通用升级”一节。
- docs/Corral通用升级设计.md（本轮完整正式方案）。
- docs/Corral核心Rust集成设计.md §2、§3、§4、§6、§8.1 的分层与兼容约束。
- 直接相关源码及现有 crates/corral-core/tests；最新讨论需要追溯时读 docs/调研/Corral通用升级-实施前一致记录.md。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/corral-live-upgrade。
- 分支：corral-live-upgrade，从主控的设计/任务提交建立。
- 范围：crates/corral-core 的实现、资源、测试；必要的 Cargo.toml/Cargo.lock；直接相关公开消费者兼容、scripts/package.sh 与说明；本任务完成记录。核心不反向依赖宿主、插件、遥测或任务业务。
- 正式设计已先落盘。常规实现细节可记录；如遇必须改变已批准行为或实质架构的情况，先向主控报告，不自行降目标。不要改主仓库工作区或其他历史 worktree。

## 要做的

按专项设计完成首个支持升级版本所需的集成能力：

1. 通用 pen 的正常原地升级、完整在途状态、资源所有权、备用进程与 Hold 恢复；能力、身份和结果通过公开接口可观察。
2. Corral 公开 upgrade/recover 与批量结果，正确区分 accepted、pending、complete、failed、unknown 和旧版无能力；不停止旧 pen 兜底。
3. 未来外部 hook 稳定入口，以及 pi/omp 采集器与可升级判定分离，同版本兼容 v1/v2 事件。普通程序与已知 agent 共享托管升级机制。
4. 新 after 持久记录、独占持有者交接、request_id 的发送事实；sending 无法证实时 unknown，不重发。保持原有等待、空闲、人类保护、超时和确认含义。
5. 仅必要的公开消费者/帮助/成套资源接入；打包仍只产出新目录，不切真实链接。所有有集成关系的部分在本分支交付，不把 pen 单独完成当作全链路完成。

边界：这是未来支持协议版本之间的能力。现存旧 pen/固定适配器/旧内存 after 的首次过渡未解决；报告 needs_restart/不支持，不擅自重启或伪造无缝。部分故障窗口无保护如实报告；正常升级不能静默丢输入、连接或提醒。

## 怎么算做完

用户原话：
> 这个方案不能绑定某一些agent。

用户在确认主要修改 Corral 内核及“正式设计 → 隔离实现 → 合成验证 → 主控审查，实施与部署分开”的实施顺序后回复：
> 批准

具体授权范围及已有取舍以专项设计为准；不自行补充用户验收要求或把未测能力写成通过。

## 验证预算

- 遵循 AGENTS.md 的行为 RED→GREEN；目标检查先可运行且因缺失行为失败，编译错误不算 RED。针对进程、流、恢复、提醒和事件适配的直接相关边界检查在碰要害预算内自行选择，命令前台完成。
- 直接相关检查，加标准各一次：`cargo test --all-targets --target aarch64-apple-darwin`；`cargo clippy --all-targets --target aarch64-apple-darwin -- -D warnings`；`git diff --check`。均用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。
- 新测试与夹具不含 Python；默认全部使用临时 HOME/CORRAL_HOME、合成程序/事件和不可变临时包，不依赖真实 coding agent 或用户数据。生命周期测试不得绑定会被重编译覆盖的共享顶层二进制。
- 标准首轮如有疑似无关失败，最多限定复跑一次，保留首次结果。返工仅跑修复相关检查与直接回归，不重跑整套预算。未完成真实 agent 兼容验证须明确记录，不能冒充全路径验收。

## 不要做

- 不部署、不安装或改用户技能/配置/命令链接，不升级、重启、停止、注入或输入现存 agent，不操作真实任务队列或遥测。
- 不读取 Corral 私有真实状态、对话或终端内容；测试状态只能是本任务隔离生成的数据。不改原独立 Corral 仓库及任何旧版本包。
- 不加入中央服务、有损恢复、自动重发、旧 Python 兜底、无关 UI 或相邻优化。
- 不按路径/项目名批量杀进程；只清理自己记下 PID 的隔离测试进程。
- 不合并 main、不推送，只在 corral-live-upgrade 分支提交。不要自行删 worktree 或停 agent。

## 做完

在本文件末尾追加完成记录并提交：实现与提交号、真实 RED/GREEN 和标准检查结果、必要取舍、已知限制、未执行项目。若实验证伪关键机制且无法在既定设计内解决，报告具体阻塞，不用 stub/skip/弱化断言宣布完成。回复给出提交号、结果及需主控决定事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
