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
