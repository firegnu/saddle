# 任务：显示英文 agent 工作职责

2026-09-27，saddle/main 交给 saddle/dev-duty-title（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由档位、影响面拿不准；交叉审查不要。主控判断仅扩展显示映射及派发文档，选看得见）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 36、37 节。
- src/corral.rs 的 Role、src/ui.rs 的 pane_title、tests/ui.rs 的角色标题检查。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/agent-duty-title，分支 agent-duty-title（从 main 建好）。
- 只改角色标签显示映射及直接相关测试、必要的中英文 README、本任务完成记录。

## 要做的
按设计第 37 节，在现有 Controller / Regular 基础上支持公开 role=implementer、role=reviewer，显示英文 Implementer / Reviewer 加 agent 名。保留旧标签、未知值及空窗格回退规则。项目派发标签约定已由主控写入 AGENTS.md，不必再改。

## 怎么算做完
用户原话：
> 我想显示的是主控，实现者，审查者类似这种。你看看好不好实现？不好实现就算了。
>
> 那就做吧
>
> 不是汉语哦，就是对应的英语

验证预算：`git diff --check` 加一条直接验证角色标题渲染的现有自动检查（可扩充该检查）。仅显示/文档改动，不要求 RED，不跑全套或真实终端录制。Cargo 命令使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不修改 New 的角色控件、创建行为、命名、命令默认值、终端生命周期、输入或派发操作。
- 不改 corral/drover 仓库、全局技能、其他项目规则、真实 agent/队列或用户配置。不补旧 agent 标签，不根据名称猜角色。
- 不扩修已知 full_workflow 旧鼠标坐标基线问题，不扩大验证预算。
- 不按项目名或路径批量杀进程；不合并、不推送、不构建 release，只在本分支提交。

## 做完
本文件末尾追加完成记录并提交，写明改动、验证、取舍与未做事项。回复带提交 SHA 和待主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录（saddle/dev-duty-title，2026-09-27）

### 改动
- `src/corral.rs`：`Role` 增加 `Implementer`、`Reviewer`，公开标签 `role=implementer` / `role=reviewer` 解析为对应职责；仍只认精确小写值，不看名字。
- `src/ui.rs` `pane_title`：`Implementer` → `Implementer · 名称`，`Reviewer` → `Reviewer · 名称`；Controller / Regular / 无效或无标签 → Agent / 空窗格 → Viewer 规则不变。
- `tests/ui.rs` `viewer_title_shows_the_public_role_label_or_agent_without_guessing`：原先 `reviewer` 期望 `Agent`，改为期望 `Reviewer`；新增 `implementer` → `Implementer`，未知值改用 `tester` → `Agent`；大小写不符 `Controller` → `Agent`、空标签 → `Agent` 保留。
- `README.md` / `README.zh-CN.md`：New 段落补一句其他入口带 `role=implementer` / `role=reviewer` 标签的 agent 显示 `Implementer · 名称` / `Reviewer · 名称`。

### 验证
- `git diff --check`：通过。
- `CARGO_TARGET_DIR=… cargo test --test ui viewer_title_shows_the_public_role_label_or_agent_without_guessing`：1 passed。
- 按预算未跑全套测试、clippy 或真实终端录制。

### 取舍
- New 的角色控件、创建参数、命名等未动；`Role` 新增的两个变体只用于读取标签显示，New 不会产生它们。
- 空窗格 `Viewer` 回退沿用原实现，本次未改该路径，也未为它另加检查。

### 未做 / 待主控决定
- 未跑 `cargo clippy --all-targets -- -D warnings` 与全套测试（超出验证预算）；合并前是否补跑由主控定。
- AGENTS.md 派发标签约定已由主控写入，未改。
