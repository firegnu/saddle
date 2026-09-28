# T36：委托 Drover 主控实现人工确认完成

2026-09-28，saddle/main 交给现有 drover/main。用户先以「可以的」确认 DESIGN 第 49 节方案，随后明确「这个任务已经处于下方状态了」（结合对话，指已下发）。现正式进入实施，不再等待第二次下发；此前评估稿“不立即实施”与“等待手动下发”的阶段限制已被本次指示取代。

## 先读与职责

- 你仓库 AGENTS.md，以及 `/Users/firegnu/Developer/personal_projs/saddle/docs/DESIGN.md` 第 49 节。
- T36 评估报告仅作为根因与现状参考，不将其中每个讨论点扩为新功能。
- 你仍是 Drover 主控：先固定最小公开契约，按本仓库流程派 Codex 实现，再负责审查及按路由安排独立审查。模型／强度／验证预算由路由与本仓库规则确定，记录实际结论；不是让你自己写功能代码。公开接口细节在已批准语义内可直接确定后实施，无须另等一轮确认；若必须改变已确认产品流程再报 saddle/main。

## 已批准的结果与范围

- 新增公开人工完成操作，让用户对当前具体任务运行填写原因并确认完成。支持未满足 Git 条件或未执行／失败的验收条件，不声称这些检查通过；不为人工确认重跑 CHECK_CMD。记录人工确认方式、原因、时间及当时检查结果／未执行状态，list／show 可读，旧历史不得倒填自动通过。
- 目标须绑定项目和具体任务运行，不能只按 T 编号；确认目标已变或无法可靠读取／写入时拒绝，不改成处理下一任务。具体命令、身份字段、成功／错误 JSON 及错误码由你在公开契约中明确，供 saddle 直接消费。
- 人工完成固定进入 Awaiting release，即使 gate 关闭也等用户放行；不调用 go／next，不改变 loop／pause，不停止 agent，不改 Git 文件、分支或提交。保留现有自动检查及普通 go 行为，自动引擎不得自行走人工覆盖路径。
- 不做任务类型系统、分支归属／永久忽略协议、核心提取重构、通用审计系统或历史迁移，不修改 corral、saddle、全局技能或无关 T27。saddle 后续只通过你的公开接口实现确认弹层和结果展示。

## 验证与交付阶段

按路由和你项目规则给实现者明确预算；在隔离 HOME、合成项目及假 corral／通知发送器中验证直接行为与必要回归。标准检查各一次，不做重复基线统计或无关矩阵；失败如实报告、定向处理。不要为 T36 调用真实任务的 done／go／next 或新人工完成命令，不改真实配置、通知偏好、队列或服务。

审查通过后交付固定 SHA、公开契约路径、可执行命令绝对路径、实现者／worktree、路由审查结论和未解决项。**本阶段停在待联调：不合并、不推送、不安装发布，不清开发／审查 worktree。** saddle/main 收到审查通过的固定交付后，安排 saddle 消费方，再组织隔离联调；通过后协调两边落地。

派发后可先报实际 agent／模型档位／分支路径及契约异议；区分派发进展与完成交付。挂好你方完成提醒，接续审查；允许你通过 corral 向 saddle/main 发送本任务进展及最终交付，这是本次跨项目协调授权。不要关闭 drover/main 或其他用户主控。

命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## Drover 派发进展（2026-09-28，尚未实现交付）

Drover 主控回报并经 saddle/main 只读核对：契约提交 `a292b2a` 仅包含公开接口及实施任务两份文档，不是功能完成 SHA。契约路径为 `/Users/firegnu/Developer/personal_projs/drover-worktrees/m36-manual-complete/docs/人工完成JSON接口.md`。

- `show` 新增 `manual_completion.target_token`，消费者只原样回传，不解析／组装；新命令为 `complete-manually Tn --target-token TOKEN --reason REASON --json`，在同一项目调用。
- 成功固定 awaiting_release；`task.completion_record` 保留人工确认和检查快照，旧历史不倒填。target_changed／state_busy 等明确报错，不回退普通 go。既有自动判据及普通 go 语义保持；与 DESIGN §49 无契约异议。具体字段及错误码以固定公开文档为准。
- 实现者 `drover/dev-manual-complete-1`，instance `465fd011c92e`，Codex `gpt-6-astra / xhigh`；分支 `m36-manual-complete`，worktree 同上述目录。核对时 working，不读取回复当作 DONE。
- 路由三项未定，Drover 主控依据身份并发和核心放行确定重／碰要害／独立审查要；后续审查由该主控安排，完成提醒已挂。

当前只记录派发，不启动 saddle 消费方。等待实现和独立审查完成后交固定 SHA；两边 worktree 留待隔离联调，不合并发布，不操作真实队列、配置或服务，不关闭 drover/main。

## Drover 主控初审通过、独立审查中（2026-09-28）

Drover 主控回报实现 `b50d36091d54085ba56beba6768fb28f215fe6cf` 已完成、初审通过；标准 CLI／criteria／show 16 项／list JSON 各复跑一次通过，主控记录 `ef0a7233554f52316ef0c396a02779deef1afadf`。saddle/main 已核对两个提交存在，未重复上游测试；通过结论和测试结果为 Drover 主控交付的阶段记录。

独立审查者 `drover/dev-manual-review-1`，instance `99562bc7194c`，Codex gpt-6-astra/xhigh，worktree `/Users/firegnu/Developer/personal_projs/drover-worktrees/review-m36-manual-complete`，detached HEAD `ef0a723`；核对时 working，重点身份并发及放行边界。范围和公开契约无变更／异议，Drover main 仍 `46d7699`。

当前不是最终交付。等待独立审查通过后的固定 SHA，再启动 saddle 消费方；未合并发布或操作真实状态，不清任何开发／审查 worktree，不关闭 drover/main。

## 最终交付接收（2026-09-28）

固定 `0e4d031d3a570ef00e66b4df2f1e97b191f6607f`，实现 b50d360、审查点 ef0a723。已核对最终 HEAD、开发／审查工作区干净，最终提交与 ef0a723 的 bin／tests 无差异，仅文档归档；主控与独立审查均通过（必须改 0、建议改 0）。独立 10 项通过和标准检查结果由 Drover 审查记录提供，本主控未重复执行。公开契约已读，无消费方异议。

公共写命令非阻塞互斥忙时退出 4，这是新增操作与原写操作之间的协调；正常 go／自动判据语义保留。该锁只协调新版 CLI，旧版已启动写进程的切换留待联合落地核对，不在此操作服务。保留 Drover 两个 worktree／agent 以待联调。下一步按 `T36-saddle人工完成实施.md` 派消费方，独立审查后只走一次隔离主流程。
