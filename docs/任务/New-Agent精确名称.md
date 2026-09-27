# New Agent：主控固定 main，普通 agent 自由命名

## 最新状态：角色方案已确认（2026-09-27）

用户已回复「对」，确认默认主控、名称锁定 main；选普通 agent 后自由填写名称。继续在 b015f42 上补齐角色选择，不另开分支或 agent。旧方案已静态初审，尚未主控最终标准检查或合并；后续按本任务最新范围审查。

2026-09-27，saddle/main 交给 saddle/dev-exact-name（Codex，常规档：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由档位拿不准，主控选常规；交叉审查不要，影响面改行为）。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- docs/DESIGN.md 第 34 节。
- src/launch.rs 及直接相关现有测试。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/new-agent-exact-name，分支 new-agent-exact-name（从 main 建好）。
- 只动 New 的角色/名称选择及必要布局、直接受影响的测试及中英文 README、本任务完成记录。

## 要做的
按设计第 34 节，在 New 普通表单增加 Controller / Regular 角色选择，沿用现有描边按钮，放在名称输入框前。默认 Controller，名称显示只读 main；Regular 的名称允许编辑。角色与 Codex/Claude 独立，不自动开启派发或修改队列配置。沿用 b015f42 的精确名称规则：不加 --unique，重名通过公开 CLI 失败反馈并保留草稿，不新增编号、重试或改接策略。保持现有表单风格和短窗口可达性。

## 怎么算做完
> 现在有一个问题，new agent的时候，如果不改默认的main，但是出来的agent的名字是main-1.你来调查一下。
>
> 关键是现在有些工作流依赖主控的名字是main
>
> 能不能这样，有一个选项。选主控就是写死main，如果不是就可以随意写名字。你觉得如何？

用户已确认：默认主控、名称锁定 main；选普通 agent 后自由填写名称。

验证预算：新增角色行为的定向自动检查先 RED 再 GREEN，运行受增量影响的表单/工作流检查、Clippy 和 `git diff --check`；旧方案标准检查已跑，本轮不重复全套。命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 不要做
- 不改变派发流程、Codex YOLO 默认、Claude 默认、打开位置；除角色/名称控件所需布局外，不改 UI 样式或其他功能。
- 不操作现有 main-1 或任何真实 agent、队列和其他仓库；测试使用假 CLI/合成数据。
- 不按项目名或路径批量杀进程。
- 不合并、不推送、不构建 release；只在本分支提交。
- 已知 full_workflow 旧终端鼠标坐标失败不扩修；其他范围外问题记录报告。

## 做完
本文件末尾追加完成记录并提交：改动、验证、取舍、未做事项。回复带提交 SHA 和待主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
