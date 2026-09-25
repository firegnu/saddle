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

## 完成记录

- 做了什么：先在 DESIGN 第 25 节写下实现方式，再新增 `src/git.rs`（只读 Git CLI 摘要 + 后台 Poller）。每个 agent 目录行下多一行 `分支 · C<n>(<基准>) · +增 -删 · ?未跟踪`，窄窗按显示宽度折行，`+` 绿（agent_idle）、`-` 红（agent_error）。结果存在 Agents 面板的独立表 `Panel.git`（按公开 cwd 作键，与 corral 数据分开），`absorb_git` 只收当前名单里仍在用的 cwd，并删掉已不在名单里的条目。App 在每次 corral 刷新后把去重的 cwd 交给 Git Poller（名单变化时立即重查，否则约 5 秒一轮），退出时取消并等线程结束。`command.rs` 加了带环境变量的 `run_with_env`，原 `run` 行为不变。中英文 README 补了显示含义、限制和 Git 2.41 要求。
- 验证了什么：先写 tests/git.rs、tests/agents.rs、tests/ui.rs 的新检查，用能编译的空实现拿到真实 RED（都是断言失败），实现后转 GREEN。覆盖：临时合成仓库里 main 相对上游 / 开发 worktree 相对本地 main 的提交数、暂存加未暂存的增删行、二进制单列、未跟踪数；同一 worktree 的根目录和子目录只查一次，同仓库两个 worktree 分开统计（用记录调用的 git 包装脚本数出来）；非 Git、已删除、相对路径显示不可用；无 HEAD、detached、没有 main、main 没有上游时对应字段为未知而不是 0；仓库配置的 clean/process filter、textconv、diff 驱动、diff.external、fsmonitor 都没被运行，索引字节不变（另手工确认去掉这些保护时 clean filter 确实会跑，说明测试能抓到问题）；Poller 跟随目录切换，git 卡住时 drop 在 2 秒内返回，取消的一轮不上报结果；晚到的旧目录结果不会显示在已换目录的 agent 名下；渲染、共享 worktree 同数字、颜色、折行、紧贴路径行。`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`git diff --check` 均通过。原多条目排版测试的「每个 agent 4 行」按批准方案改为 5 行。
- 拿主意的地方：
  - 读不出来的值显示 `—`（`C—`、`+— -—`、`?—`），detached 显示 `HEAD detached`，没取回显示 `git …`，整目录不可用显示 `git unavailable`。
  - 二进制文件显示为 `N binary`，不计入行数。
  - 没有 cwd 的 agent 不显示 Git 行。
  - 同一 worktree 的判定用 `rev-parse --show-toplevel`，子目录也算同一 worktree，统计范围是整个 worktree。
  - 所有 worktree 在一个后台线程里依次查，每条命令超时 5 秒。
  - 为了不跑仓库配置的 filter，用了 `--attr-source=<空树>`，因此需要 Git ≥ 2.41。代价是 `.gitattributes` 里的 `binary`/eol 设置不参与这次统计，改由 Git 按内容自动判断二进制。
  - 部分克隆不补取缺失对象，用的是环境变量 `GIT_NO_LAZY_FETCH=1`，旧版 Git 会直接忽略。
- 没做的事：Git 写操作、diff 详情页、配置项（git 程序固定从 PATH 找 `git`，刷新间隔固定 5 秒）；不跟随 agent 之后 cd 到的目录；`$GIT_DIR/info/attributes` 和用户全局 attributes 里的 filter 仍按 Git 规则生效（已在 DESIGN 注明）；没做真实界面录屏或手动启动检查。

## 第一轮返工完成记录（交叉审查必须改 1–4）

- 1 全部 attributes 来源的外部 filter：去掉 `--attr-source=<空树>`，attributes 照常读取。diff 前用 `git config -z --name-only --get-regexp ^filter\.` 列出所有配置层定义的 filter 驱动，每个驱动以 `-c` 清空 clean/smudge/process 并设 `required=true`。无论规则来自工作区 `.gitattributes`、`.git/info/attributes` 还是 `core.attributesFile`，Git 都报 filter 失败而不启动程序，这一轮增删行显示未知。不改仓库配置或 attributes 文件。回归：三种来源 × clean，外加 info × process，标记文件不出现、索引字节不变；不受 filter 影响的改动照常计数。
- 2 禁止 lazy fetch 的能力门禁：所有 Git 调用（包括第一步定位 `rev-parse --show-toplevel`）都带 `--no-lazy-fetch`，最低版本改为 Git 2.45。更旧的 Git 把它当未知选项拒绝，定位即失败，整行显示 `git unavailable`，不降级。原来的 `GIT_NO_LAZY_FETCH` 环境变量和 `--show-object-format` 都已去掉。回归：
  - 假旧版 Git 拒绝该选项时，结果为不可用。
  - 本地 `file://` promisor 的部分克隆缺 HEAD blob、工作区已改：摘要显示增删未知，blob 仍缺失。该 promisor 可达，允许补取的话会成功，所以测试能区分。
- 3 继承环境隔离：`command::run_with_env` 改为 `run_without_env`（带要去掉的变量名），Git 调用去掉当前进程的全部 `GIT_*` 变量。corral/drover 仍走原 `run`，环境不变。回归放在单独的测试二进制 tests/git_env.rs（只有一个测试，避免改进程环境影响别的测试）：`GIT_DIR`/`GIT_WORK_TREE` 指向 A、另设 `GIT_INDEX_FILE` 和 `GIT_TRACE` 时，A、B 仍各读各的分支，B 的未跟踪数正确，trace 文件没被写。
- 4 内建属性语义：恢复读取 attributes 后，`*.txt text eol=crlf` 已提交、只改时间戳的文件不再报增删，`*.asset binary` 计为 binary。回归即此夹具。
- 验证：先补上面这些检查。四项都拿到真实 RED：旧实现分别报 `+3 -3`（原预期 `binary 1`）、filter 夹具仍计数且会跑钩子、假旧 Git 仍返回摘要、受污染环境下 B 读成 `branch-a`。修复后 GREEN。
  - 通过：`cargo test --test git --test git_env --test agents --test ui`，以及因 command.rs 改动顺带跑的 `--test corral --test drover`；`cargo clippy --all-targets -- -D warnings`；`git diff --check`。
  - 按预算没重跑无关全套。
  - 测试夹具里，filter 用例先把 a.txt 设成旧时间戳并刷新索引。否则 Git 对与索引同一秒写入的文件（racy）要重读内容，需要 filter，增删行在触碰 a.txt 前就是未知。这属于 Git 的正确行为，已写进 DESIGN。
- 取舍：
  - 被阻止的 filter 让整个 diff 未知，不按文件剔除，不改成原始字节口径。
  - 纯改名按路径全删全增，不做 rename 检测（主控裁决）。
  - 慢仓库延长整轮刷新周期，已写入 DESIGN 和 README，结构仍是单线程（主控裁决）。
  - 进程组、包装器后代的清理没做（建议改 2，非返工条件）。
  - DESIGN 第 25 节和两份 README 已同步。
