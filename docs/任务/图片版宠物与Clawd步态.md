# 任务：接续图片版宠物并修正 Clawd 走路姿态

2026-10-03，saddle/main 交给新建的 saddle/dev-pet-images（Claude Code，常规：opus[1m] / high；实际带后缀名称以 Corral 回执为准）。
路由：常规 / 交叉审查不要 / 影响面：改行为（Saddle dispatch route：常规、不要、改行为）。本任务未选择遥测记录，不创建 Tasks run。
类型：功能变更
依据：用户现在授权接续图片版宠物，并一并纠正像素版 Clawd 的走路姿态。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- AGENTS.md。
- HANDOFF.md「宠物包」及末尾共享编译目录说明。主控迁移状态已有更新：现在 saddle/main 是 Rust Corral 托管的新会话，旧 Claude 已关闭，不能恢复或操作它。
- docs/DESIGN.md「Clawd 自由巡游与网页动作」「Clawd 精选小幅动作」「宠物包与第二只宠物」。
- assets/pets/README.md、assets/pets/clawd-sources.json，以及直接相关的 src/mascot.rs、tests/mascot.rs。

## 在哪里干活
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/pet-images，分支 pet-images，从 main 的 d24965f 建立。
- 范围：宠物素材、宠物渲染及其必要的终端集成、相关测试和设计说明；依赖调整限图片版所需。

## 要做的
- 接续已约定的图片版：Kitty 图形协议，支持的终端默认图片，其余自动退回方块版；先做原创橘猫，Clawd 使用 claude.dev 的参考动画，只做现有 3 行高度放得下的。参考来源在 clawd-sources.json；需要时自行读取原参考。
- 保持现有位置、16 列 × 3 行区域、pane/PTY 几何和宠物选择机制。用户日常用 Ghostty 和 Metalterm。
- 修正像素版 Clawd 左右行走：相对宠物自身位置，上身与头保持稳定，只让腿动；整体仍正常来回移动。不要把要求理解成宠物不能平移，也不要因此冻结其他非行走动作。
- 用户的新要求替代设计里「迈步姿态把头顶抬高八分之一格，做出身体起伏」。先将本次获准的图片版阶段及步态纠正写入 docs/DESIGN.md，再实施。

## 怎么算做完
用户原话：
> 开一个claude code让他继续做图片那一个版本的宠物去。另外现在像素版本的clawd有一个问题也请他去修一下，就是来回走路的姿势，上身不能动，只要腿动，现在改的clawd头也在动。

## 验证预算
- 改行为部分按 AGENTS.md 轻量 TDD：先运行针对性检查取得目标行为的 RED，再最小实现与 GREEN；纯素材/视觉修改不伪造失败测试，以直接显示检查验证。
- 标准检查各一次：cargo test --all-targets；cargo clippy --all-targets -- -D warnings；另做 git diff --check。命令前设置 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。
- 不覆盖正在安装的版本或共享 release 来演示。需要启动固定候选时用不可变临时成套包或显式 native target 路径；不要把共享顶层 binary 被覆盖的结果当作本候选证据。
- 无关失败最多单独复跑一次，保留首次失败。不要自行扩大成测试矩阵或重复全套。

## 不要做
- 不改 Corral、Drover、遥测业务，不操作真实任务、其他 agent 或原 Corral 仓库；不采集本任务遥测。
- 不增加 Python 产品代码/运行时依赖，不扩充未要求的宠物或设置。
- 不切换安装，不重启用户界面，不操作用户正在使用的终端；不能实测的显示效果如实说明。
- 不按项目名或路径批量杀进程；自起进程只按记录的 PID 清理。
- 不合并 main、不推送，只在 pet-images 分支提交。重大方案若超出已确认范围，停下报告。

## 做完
在本文件末尾追加「完成记录」并提交：做了什么、验证结果、取舍、未完成事项。回复注明提交号和需要主控决定的事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
