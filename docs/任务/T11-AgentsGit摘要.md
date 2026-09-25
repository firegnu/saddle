# T11：Agents Git 摘要

2026-09-26，saddle/main 交给 saddle/dev-t11-git（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查要 / 影响面：碰要害（路由：档位拿不准，按既定方案的实现采用常规；交叉审查要、影响面碰要害，涉及后台查询并发和生命周期）。
你是被委派的 agent：照本文件做，不再开别的 agent。

## 先读

- AGENTS.md；docs/DESIGN.md 第 3、6、19、23 节。
- src/corral.rs 的公开 Agent cwd 与刷新；src/app.rs 的后台更新和退出；src/command.rs；src/agents.rs、src/ui.rs 的条目和附加信息。
- tests/corral.rs、tests/ui.rs、tests/app.rs、tests/workflow.rs 中相关合成数据与程序生命周期检查。

## 在哪里干活

- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t11-agent-git，分支 t11-agent-git，已从 main 建好。
- 范围：本地只读 Git 摘要模块、必要 app/Agents 数据流、渲染、直接相关测试、设计与中英文说明、本任务记录。
- 所有 cargo 命令加 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 用户原话与授权

> 我想知道有没有提交以及diff情况，但是是不是只在主控的branch显示，你先调查给一个方案。家长确认后再排除agent去做

主控已调查并向用户展示以下完整方案，用户回复：

> 批准了

调查阶段结束，现已授权按下述方案实现，不需再次请求实施确认。

## 已批准的方案

每个 agent 都显示其公开 cwd 所在 worktree 的 Git 状态，不限于主控分支。在目录下面增加一行，窄窗自动折行，示意：

```text
dev-t12 · C2(main) · +18 -4 · ?1
```

- 分支：该工作目录的当前分支。
- C2(main)：开发分支比本仓库本地 main 多 2 个提交；main 则相对其配置的上游（如 origin/main），显示上游名称，表示尚未推送的提交数。没有明确基准显示 —，不猜测基准。这里是相对基准的提交差值，不是 agent 的历史提交数或本轮独有提交数。
- +18 -4：尚未提交的净增删行数，暂存与未暂存一起相对 HEAD 计算。提交后这部分归零，提交数随之变化。+ 绿色、- 红色，复用现有主题色。
- ?1：未跟踪文件数，不混入增删行数。
- 多个 agent 共用同一 worktree 时显示相同数字；数据属于该目录，不能归因到某个 agent。不同 worktree 分别统计，不能按仓库公共 Git 目录合并缓存。cwd 来自公开 corral ls，不从进程/终端推断 agent 后来 cd 到哪里。
- 本地公开 Git 命令后台读取、同 worktree 共享查询，约每 5 秒刷新；失败显示不可用，不影响 agent 状态刷新、界面输入或接入。只读本地数据，不自动 fetch。

## 要做的

- 按批准方案最小实现，先写 DESIGN 再改代码；选择具体数据结构和后台方式时沿用项目现有模式，保持简单。Git 结果与 corral 状态分开，超时/失败或旧结果不能覆盖错误目标。
- 非 Git 目录、已删除目录、无 HEAD 或 detached 等无法确定的字段如实显示，不冒充零值；数值含义遵循上面的定义，二进制等没有行数的情况不要编造。同步 README 说明必要的显示限制。
- 查询使用 Git CLI，不直接解析 .git；避免运行仓库配置的外部 diff/textconv 等辅助程序，不为只读摘要写索引、修改仓库或触发网络。测试使用临时合成仓库和假 CLI，不拿用户其他项目做实验。
- 保留既有 agent 信息、主题、effort、选择与接入交互；不新增 Git 写操作、diff 详情页或复杂配置体系。

## 怎么算做完

> 我想知道有没有提交以及diff情况，但是是不是只在主控的branch显示，你先调查给一个方案。家长确认后再排除agent去做

用户已批准「已批准的方案」全文，按该方案实现。

验证预算：按轻量 TDD 先取得针对目标行为的真实 RED，再最小实现到 GREEN；针对性检查及 cargo test --all-targets、cargo clippy --all-targets -- -D warnings 各一次，另做 git diff --check。按本次影响面补直接相关的查询取消/超时、共享与隔离、旧结果归属的边界检查；不要求覆盖矩阵、缺陷注入、录屏或额外证据目录。失败可运行修复所需检查。

## 不要做

- 不操作用户 agent，不改真实队列、登记、配置或其他项目文件；不读 corral/drover 内部文件，不改它们的仓库或接口。测试不启动真实 agent。
- 不 fetch/pull、不替用户提交推送、不读取或修改其他项目来试验；开发提交仅在自己的 worktree。
- 不声称数字证明某 agent 或某任务的提交归属；不扩成执行追踪或完成判据。
- 不开始 T4/T2，不修无关问题，不按路径/项目名批量杀进程，自己起的进程只按记录 PID 停止。
- 不合并 main、不推送、不更新 HANDOFF，只在 t11-agent-git 分支提交。

## 做完

在本文件末尾追加「## 完成记录」并提交：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复同样列这些和需主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
