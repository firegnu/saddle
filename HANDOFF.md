# Saddle 交接

更新：2026-10-03。用户准备关闭当前 Codex 主控和 Claude 调研会话，要求先归档双方结论、更新交接、提交并推送。**下一会话先核对状态并讨论；没有授权实现、迁移或部署无缝升级。**

## 本轮结果与接续入口

- 本机每次重新部署 Saddle/Corral 后，让全部现存 agent（主控、普通、后台）自动由最新核心托管，同时保留进程、上下文、在途工作、输入输出、连接、身份和完成提醒，这是用户目标。新界面连接旧 pen、结束后 resume 都不算满足目标。
- 主仓库 `docs/调研/现存agent无缝接管可行性.md` 已汇总 **Claude 调查结论、主控 Codex 复核结论、源码/系统依据及未验证点**。开头的结论对照和主控限定优先于保留的原调查正文中的较强措辞。任务与完成/审查记录在 `docs/任务/现存agent无缝接管调研.md`。
- Claude 原调查提交 `460d03b`，主控复核 `f0bdc90`。本轮将文档归档到 main，不是实施分支合并；只改文档，没有改源码、测试或正式 DESIGN，没有运行接管原型、迁移或部署。
- 当前公开机制内没有可用的首次无缝过渡路径；原地 exec 有系统依据，可作为优先验证候选，但正式路线未定。忙碌跳过、旧 hook、失败/回退及 after 超时尚未解决，不能宣布完整可行。
- 用户本轮要求提交并推送。最终提交号及远端一致性以 `git log -3`、`git status -sb`、`git ls-remote origin refs/heads/main` 核对；不要把本文件的写入当作推送已成功的证据。

## 两个会话及保留工作区

- 主控：`saddle/main`，调查快照 instance=`faec2c2b00cb`，agent PID=99462，pen PID=98891，仍加载 `versions/7755bb8/bin/corral`。这是 Rust Corral，不是旧 Python 核心。
- Claude：`saddle/dev-live-upgrade-1`，instance=`3d4043d744ca`，opus[1m]/xhigh；最近公开状态 idle、attached=0。工作区 `../saddle-worktrees/corral-live-upgrade-research`、分支 `corral-live-upgrade-research`，提交 `f0bdc90`，保留供追溯。主仓库已有完整文档，不依赖会话存活才能接续。
- 用户表示将关闭两者；本轮不代为 stop，不宣称已关闭。下一会话先 `corral ls/status` 核对，不复用上面的 PID/状态推断。用户关闭会话不等于批准其他 agent 重启或批准迁移方案。
- 另保留历史 worktree：`review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow`。不要顺手清理。

## 当前部署与最近功能

- 最近成套安装为 `~/.local/share/saddle/versions/00af4f2`，两命令入口和 Drover/Diff 注册已切换；四程序哈希、插件回读、隔离 release 退出重开测试 1 项通过。没有由主控重启真实 Saddle；实际加载路径需接续时核对。
- 部署记录：`docs/任务/卡皮巴拉-部署记录.md`；备份 `~/.local/share/saddle/backups/capybara-00af4f2-20261003-124250`。旧包仍有 pen/hook 引用，不可删除。部署只切入口不会改变存活 pen。
- 已集成部署：Agents 状态计时；图片宠物及 Blocks Clawd 仅腿动；Display Auto/Blocks；A 描边大头橘猫；Capybara 图片与 Blocks 两版。默认宠物仍 Clawd，Display 默认 Auto。对应任务书均在 `docs/任务/`。
- Capybara：实现 `ea665b3`，审查 `0776995`，合并 `33c00d7`，收尾 `eda201b`；预览 `docs/调研/卡皮巴拉宠物.md`。41 图片姿态、35 Blocks 姿态、七段动画，既有四套未变。该实现分支/worktree 已清理，实现者已关闭。
- Capybara 主控标准首轮 496 passed、1 failed、9 ignored；既有 workflow 鼠标任务表单失败，限定复跑 1 passed，补其他目标 128 passed，合计 625 项通过、9 忽略；不是一次全套绿。Clippy 通过，首次失败原因未证实，证据 `/tmp/saddle-capybara-review-*.log`。尚未在真实 Ghostty/Metalterm 完成卡皮巴拉视觉验收。
- A 图片猫及更早改动的验证/取舍见对应任务书，不重新开启已完成工作；部署历史也在各任务部署记录中。

## 核心与操作约束

- Rust Corral 已集成到 `crates/corral-core`，原 Corral 仓库只读保留。宿主、插件只通过公开接口交互。关闭 Saddle 不停 agent，重开接回同实例是已有能力；这不代表热替换 pen 已实现。
- Rust 集成验证和原始失败记录见 `docs/调研/Corral核心Rust集成-实施核验.md`；部署记录见 `docs/调研/Corral核心Rust部署记录-2026-10-03.md`。只测过合成 agent 的产品生命周期，不能称真实 Claude/Codex/pi/omp 全路径验收。
- 主控遵守 AGENTS.md：功能开发用 corral-dispatch 派发，自己审查集成。现有用户 agent 只读，不擅自送话、按键、停止；不读 Corral 私有状态文件，不批量杀进程。
- 本调研未选择遥测、未创建或操作真实 Tasks run。历史队列快照不当作当前状态，不自动派下一任务。
- 构建共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；需固定二进制时用不可变包或显式 `--target aarch64-apple-darwin`，避免共享顶层产物被覆盖。纯文档归档验证用 diff/引用检查，不运行 Cargo 全套。

## 下一步

1. 读 AGENTS.md、本文和调查报告；核对 main/远端、工作区和公开 agent 状态。
2. 向用户汇报并继续讨论无缝升级的未决问题；没有后续授权则停在讨论。
3. 用户选定方向并明确授权后，才改正式设计、安排隔离验证或实现。不得把报告中的实验建议自动当作下一项执行任务。

正式设计入口：`docs/DESIGN.md`、`docs/Corral核心Rust集成设计.md`。本轮的理由与双方意见保存在调查报告中。
