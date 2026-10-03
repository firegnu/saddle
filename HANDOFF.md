# Saddle 交接

更新：2026-10-03。Corral 通用升级实施已审查通过、合并推送并清理本轮工作区，收尾空提交 `a148d2b`。**未部署，未授权现存旧 agent 首次迁移；本轮实现工作已完成，等待用户另行安排部署或首次过渡。**

## 当前完成状态

- 正式设计：`docs/Corral通用升级设计.md`；主设计已追加入口，旧核心设计已注明新旧规则边界。
- 任务与实际完成记录：`docs/任务/Corral通用升级-实施.md`；主控核实、独立审查同目录。候选 `bd282db`，合并 `dab13d7` 已推送；收尾 `a148d2b`。最终交接提交及远端一致性以 `git log -3`、`git status -sb`、`git ls-remote origin refs/heads/main` 核对。
- 最新一致记录：`docs/调研/Corral通用升级-实施前一致记录.md`。评估会话 `saddle/dev-live-upgrade-review-1`，instance `43b27dc12c46`，只读设计已完成，保留，不关闭。
- 当前主控 `saddle/main`，公开 instance `19185812ef0e`，role=controller；此前主控/Claude 实例已不在公开列表，旧 PID 不复用。开始/完成与实际 agent 名字以公开回执核对。
- 目标适用首个具备升级协议版本之后：原地 exec、通用资源交接、稳定 hook、进程内采集与判定分离、持久提醒。现存旧 pen/适配器/旧 after 首次过渡仍未解决，不自动停、重建或 resume。已知故障窗口仍可能丢会话。
- 检查：主控对修正前 `ce6f675` 标准 test 641 passed、0 failed、9 ignored，clippy 通过。最终 `bd282db` 返工直接回归由实现者验证 8 项集成及 1 项单测，主控重跑缺陷 2 项集成及 1 项单测通过。没有重跑修正后的全套/clippy；初次失败与真实 RED/GREEN 保留，不混报版本。
- 独立 Codex 重档首审发现 1 项必须改；`9648ff0` 修正后，第 1 轮限定复核同实例 DONE，可以合并，剩余必须改 0、新增建议改 0。主控认可，无待处理审查项。
- 本轮实现及 detached 审查 worktree、已合并 `corral-live-upgrade` 分支已安全删除；工作目录已删，一并关闭实现者 `saddle/dev-corral-upgrade-1`（`7321e90e81f2`）与审查者 `saddle/dev-corral-upgrade-review-1`（`dca43ca10e7b`），两次 stop 均 ok/exit 0。公开列表仅保留主控和上述设计评估会话，两者均在主仓库。
- 实施仅隔离合成验证，未操作用户会话、真实安装、Tasks 或遥测；本链路未选择遥测。旧临时固定包基于 `816358a`，不包含后续身份修正，不能作为最终部署包使用。真实 Claude/Codex/pi/omp 冒烟未执行。

## 上轮调研归档历史

- 本机每次重新部署 Saddle/Corral 后，让全部现存 agent（主控、普通、后台）自动由最新核心托管，同时保留进程、上下文、在途工作、输入输出、连接、身份和完成提醒，这是用户目标。新界面连接旧 pen、结束后 resume 都不算满足目标。
- 主仓库 `docs/调研/现存agent无缝接管可行性.md` 已汇总 **Claude 调查结论、主控 Codex 复核结论、源码/系统依据及未验证点**。开头的结论对照和主控限定优先于保留的原调查正文中的较强措辞。任务与完成/审查记录在 `docs/任务/现存agent无缝接管调研.md`。
- Claude 原调查提交 `460d03b`，主控复核 `f0bdc90`。本轮将文档归档到 main，不是实施分支合并；只改文档，没有改源码、测试或正式 DESIGN，没有运行接管原型、迁移或部署。
- 当前公开机制内没有可用的首次无缝过渡路径；原地 exec 有系统依据，可作为优先验证候选，但正式路线未定。忙碌跳过、旧 hook、失败/回退及 after 超时尚未解决，不能宣布完整可行。
- 用户本轮要求提交并推送。最终提交号及远端一致性以 `git log -3`、`git status -sb`、`git ls-remote origin refs/heads/main` 核对；不要把本文件的写入当作推送已成功的证据。

## 上轮会话快照及保留工作区

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

- Rust Corral 已集成到 `crates/corral-core`，原 Corral 仓库只读保留。宿主、插件只通过公开接口交互。关闭 Saddle 不停 agent，重开接回同实例是既有能力；通用升级代码本轮已合并，但现行安装仍是旧版本，不能据此声称运行中的 pen 已升级。
- Rust 集成验证和原始失败记录见 `docs/调研/Corral核心Rust集成-实施核验.md`；部署记录见 `docs/调研/Corral核心Rust部署记录-2026-10-03.md`。只测过合成 agent 的产品生命周期，不能称真实 Claude/Codex/pi/omp 全路径验收。
- 主控遵守 AGENTS.md：功能开发用 corral-dispatch 派发，自己审查集成。现有用户 agent 只读，不擅自送话、按键、停止；不读 Corral 私有状态文件，不批量杀进程。
- 本调研未选择遥测、未创建或操作真实 Tasks run。历史队列快照不当作当前状态，不自动派下一任务。
- 构建共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；需固定二进制时用不可变包或显式 `--target aarch64-apple-darwin`，避免共享顶层产物被覆盖。纯文档归档验证用 diff/引用检查，不运行 Cargo 全套。

## 下一步

1. 本轮实现与审查集成已完成，不再派发、重跑已完成检查或消费晚到提醒重复工作。
2. 等待用户安排真实兼容验证、部署和现存旧 agent 首次过渡；未获授权不安装、不切入口、不重启或迁移。
3. 若后续获准部署，需从包含 `9648ff0` 的最终源码生成新的不可变成套包；原临时包不含返工，不能直接发布。部署和首次过渡分别核对，不把入口切换等同旧 agent 已接新核心。
4. 保留历史调研工作区与设计评估会话，不顺手清理。

正式设计入口：`docs/DESIGN.md`、`docs/Corral核心Rust集成设计.md`。本轮的理由与双方意见保存在调查报告中。
