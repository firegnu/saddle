# 会话交接

更新：2026-09-29 晚。当前 `main`，本次交接前 HEAD／origin 为 `82286d1`，工作树干净。用户授权本次更新交接、提交并推送；不实施新任务，不操作队列、开关或其他仓库。

## 当前状态

- T47 已完成文档实现、审查、合并、清理和推送；最新公开 `drover list --json` 确认 T47 已在历史中，status=done，完成／放行时间相同。current=null、awaiting=null、paused=false、loop=false、gate=true。本主控本轮没有登记完成或放行；不要继续按旧交接把 T47 当成 Running，也不要补调 done。
- release 仍是 T45 编译的版本；T46、T47 只改文档，均未跑测试、clippy 或编译。
- 仅保留主仓库和历史 `../saddle-worktrees/t38-dispatch-study`（c15bc4d，未合入调研文档），工作树均干净；不要删除或合并历史调研分支。
- 用户正在收工。最新 corral ls 仅有 saddle/main 和 drover/main；用户说 Drover 正在提交推送，本主控不代操作或关闭它。其他主控已由用户退出。Attention 测试 agent `saddle/test-attention-input-1` 已按用户要求关闭，无自开实现／测试 agent 遗留。

## 今晚已查清的问题：完成登记遗漏

- 用户记得之前会进入 Awaiting release 并弹提示，记忆正确。历史交接提交 `ea267c2`（9 月 28 日 19:02）明确记载：主控执行 `drover done T35`，在 loop off 下进入待放行，用户看到通知并放行。T22／T23 等任务文档也有同类记录。
- 从 T36 交接 `5633a20`（9 月 28 日 20:10）开始，出现“不要调用 done/go/next”的限制；T36 当时留用户测试人工完成。T37 交接 `4edef81` 及后续继续不调用 done。主控把“不自动放行／派发下一项”扩大理解成“不登记当前任务完成”，漏掉了以前的收尾步骤。
- T36、T38 的公开 show 保留 completion_record.method=manual，解释它们后来仍出现 Awaiting；不能据此推断用户开过 Loop。用户明确从未开启 Loop。
- T47 现场：收尾记号、main 前进、分支合并均满足，CHECK_CMD 不适用，但当时仍 current=T47、awaiting=null。临时仓库／隔离 HOME／假 corral 的真实公开 CLI 对照确认：loop off 时引擎一跳不登记完成；loop on 且主控空闲时进入 Awaiting；直接 go 检查后完成并放行，不停留 Awaiting；loop off 下显式 done 且 gate=true 会停在 Awaiting。脚本仅临时证据 `/tmp/saddle-completion-probe.py`，不作为持久依赖。
- 通知只对新的 awaiting 提示（src/notify.rs）；这轮不是通知收到 Awaiting 后丢掉，而是没有登记出该状态。主控先归因 Loop off、建议改程序过早，已向用户纠正。不要重复该误判。
- **旧交接中的“不调用 done/go/next”不是永久通用规则。** 完成登记、用户放行、派发下一项是不同动作。也不要反向把所有项目都改成无条件调用 done：其行为受 gate／loop 影响，需遵守当轮授权及项目模式。本轮未改 AGENTS、共享技能或运行时代码；用户决定将可靠状态推进记成 T48，明天再修。

## 新待办与下一步

用户授权新增均已通过公开 add 入队，保持 Pending，尚未调查、设计、实施或委派。完整需求以公开队列正文为准，没有另写派发任务书。

- **T48：分离当前任务完成检查与自动派发，避免依赖主控登记完成。** 用户不认可长期完全依赖主控理解和记得执行命令；希望完成状态由确定性机制推进，仍由用户放行／启动下一项。讨论方向涉及 Drover，但具体技术方案和改动边界未定。用户说“明天修”，接手等其放行，不擅自开启 Loop 或修改上游。
- **T49：讨论改进项目跨会话交接与新 session 接续方式。** 用户认为每次都要提醒 handoff、提交推送、下次再读的方式繁琐；面向多项目讨论改进，未选定自动化或新系统。
- **T50：讨论跨项目总主控与通讯软件交互入口。** 用户希望“用户 ↔ 通讯软件 ↔ 总主控 → 各 repo 主控 → 执行 agent”。职责、授权、跨项目协调、通讯软件与集成位置待讨论；独立于 T49，不启动总主控、不接入账号、不发消息。
- 最新 Pending 顺序：T29 → T28 → T32 → T34 → T48 → T49 → T50。不要因 T48 计划明天处理而自动移位或派发。T29 仍仅记录偶发测试排查方向，未修复。
- Attention 的真实等待输入已验证：新开 Claude 测试 agent 调用 AskUserQuestion，公开 blocked；用户确认 Attention 出现，测试后已关闭。无需因 Awaiting 登记遗漏改 Attention。

## 最近已交付与验证边界

- **T47**：README Build and run 增加一句 `saddle --help` 帮助说明。实现 `29e5ad2`，合并 `24a9c54`，审查 `0ad6f4d`，收尾 `95c12e4`，交接／推送 `82286d1`。只核对文案和 diff；agent／分支／worktree 已清理。Dispatch `21125206edfc46428debf199d91915ae`，最终 note 已在实际推送成功后保存。任务及审查见 `docs/任务/T47-README命令行帮助.md`。
- **T46**：README 的 Dispatch selected 用法及禁用条件。实现 `aa7b912`，合并 `cf76dfe`，审查 `10b6a46`，收尾 `6ab5ec8`，最终推送 `4bb745f`。仅文档检查，环境已清理；Dispatch `7c4b208f9a2a4e93a5945f98abb9daae`。任务见 `docs/任务/T46-README选中派发说明.md`。
- **T45**：选中 Pending 按公开项目／pos／token 派发，失败不自动重发，缺接口字段禁用。实现 `e93b14d`、反馈修正 `4ee0e79`、最终推送 `80aa77f`，已清理并编译 release。主控标准测试 290 passed／1 failed／3 ignored；T25 PTY 退出断言单独一次通过，clippy 通过。返工 7 项定向检查通过，不将首轮写成全绿。Dispatch `5574d31b97324b619d77afb3070b4026`；详见 `docs/任务/T45-主控审查.md`。
- T37–T44 已落地，无需重复实现／审查。T43 标准套件非全绿及滚动检查修正、T38／撤回待办的验证边界保留在原任务审查文档；历史失败不等于已修复 T29。

## 接手先读与约束

- `AGENTS.md`，再读本次获授权任务；设计按 `docs/DESIGN.md`，Awaiting／通知历史证据见 `docs/任务/T36-主控审查.md`、`docs/任务/T37-主控审查.md` 和上面的历史提交。本交接未定新的系统设计。
- 按当前 corral-dispatch 技能做路由、任务类型、验证预算和阶段增量记录。已通过且相关代码未变不重复验证；完整派发任务书快照保留，最终 note 在实际收尾／推送结果确定后写。
- 记录器 `/Users/firegnu/Developer/personal_projs/dispatch-log/dlog`；主控按 AGENTS 先读其 USAGE。Saddle 可读取 Dispatch，未安装／无记录仍需正常工作；不补造历史、不读私人会话日志。
- 各 repo 主控退出前核查曾发现 Drover 有 4 个未推送提交及既存未跟踪 T27 文档，用户随后让它自行提交推送；此为旧快照，需由该主控确认最终状态。不要替其他 repo 提交或推送。
- 本次仅会话交接，不额外打任务收尾空提交、不修改项目规则、不开 agent、不启用循环、不自动放行或派发后续任务。
