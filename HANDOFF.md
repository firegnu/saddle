# Saddle 交接

更新：2026-10-03。Corral 通用升级实施已收尾（`a148d2b`），部署记录已提交推送（`2b9a728`）。**新包 `711ab18` 已在真实宿主、插件和新主控中运行；当前主控具备升级协议能力。新建替换已完成，不声称旧实例首次无缝迁移。** 本轮只读核对后，用户要求更新本交接、提交推送，然后停止，等待其提问。

## 当前完成状态

- 正式设计：`docs/Corral通用升级设计.md`；主设计已追加入口，旧核心设计已注明新旧规则边界。
- 任务与实际完成记录：`docs/任务/Corral通用升级-实施.md`；主控核实、独立审查同目录。候选 `bd282db`，合并 `dab13d7` 已推送；收尾 `a148d2b`。最终交接提交及远端一致性以 `git log -3`、`git status -sb`、`git ls-remote origin refs/heads/main` 核对。
- 最新一致记录：`docs/调研/Corral通用升级-实施前一致记录.md`。评估会话 `saddle/dev-live-upgrade-review-1`（`43b27dc12c46`）已按用户明确要求关闭；本轮公开列表仅有新主控。
- 当前主控 `saddle/main`，公开 instance `935562a4613c`，role=controller，agent PID `51609`、pen PID `51035`；旧主控 `19185812ef0e` 已不在公开列表。PID 和实例均为本轮快照，接续时重新通过公开命令核对。
- 目标适用首个具备升级协议版本之后：原地 exec、通用资源交接、稳定 hook、进程内采集与判定分离、持久提醒。现存旧 pen/适配器/旧 after 首次过渡仍未解决，不自动停、重建或 resume。已知故障窗口仍可能丢会话。
- 检查：主控对修正前 `ce6f675` 标准 test 641 passed、0 failed、9 ignored，clippy 通过。最终 `bd282db` 返工直接回归由实现者验证 8 项集成及 1 项单测，主控重跑缺陷 2 项集成及 1 项单测通过。没有重跑修正后的全套/clippy；初次失败与真实 RED/GREEN 保留，不混报版本。
- 独立 Codex 重档首审发现 1 项必须改；`9648ff0` 修正后，第 1 轮限定复核同实例 DONE，可以合并，剩余必须改 0、新增建议改 0。主控认可，无待处理审查项。
- 本轮实现及 detached 审查 worktree、已合并 `corral-live-upgrade` 分支已安全删除；工作目录已删，一并关闭实现者 `saddle/dev-corral-upgrade-1`（`7321e90e81f2`）与审查者 `saddle/dev-corral-upgrade-review-1`（`dca43ca10e7b`），两次 stop 均 ok/exit 0。历史工作区未清理。
- 实施阶段未部署；之后已另获部署授权并完成安装，见下节。未操作真实 Tasks 或遥测。旧临时固定包 `816358a` 不含返工，不再使用；真实 Claude/Codex/pi/omp 全路径冒烟未执行。

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

- 当前成套安装为 `~/.local/share/saddle/versions/711ab18`，包含恢复身份修正；两命令入口和 Drover/Diff 注册已切换。四程序哈希、插件回读、隔离 release 退出重开 1 项通过；隔离新建合成实例公开 status 确认新版 exe 及 upgrade/recover/snapshot=1，测试实例已清理。
- 本轮部署记录：`docs/任务/Corral通用升级-部署记录.md`；私有备份 `~/.local/share/saddle/backups/corral-upgrade-711ab18-20261003-191430`。config.toml 未变；两处 Corral 技能同步新版并 dry-run same。旧包保留，不删除。
- 本轮真实运行核对：两命令入口均指向 `711ab18`；`lsof` 确认 Saddle PID `50936`、Drover PID `50938`、Diff PID `50939` 及主控 pen PID `51035` 的实际映像均来自该包。四程序 SHA256 与 BUILD.txt 一致，公开 plugin status 确认 Drover/Diff 注册指向该包且 enabled、manifest_readable。
- 新主控 `935562a4613c` 的公开 status：exe 为 `711ab18/bin/corral`，capabilities 的 upgrade/recover/snapshot 均为 1，custody=owner，upgrade.state=none、attempt=0、last_error=null。进程参数确认 Codex hooks 使用稳定 helper 入口。具备后续公开升级能力，但本轮未执行 upgrade/recover，未验证真实客户端跨版本连续性。
- 本轮核对前工作区干净，main 与远端均为 `2b9a728`；本次仅更新 HANDOFF，不修改源码、不重跑 Cargo、不操作真实 Tasks 或遥测、不重启或停止 agent。交接提交及推送结果仍以 Git 回读为准。
- 部署记录：`docs/任务/卡皮巴拉-部署记录.md`；备份 `~/.local/share/saddle/backups/capybara-00af4f2-20261003-124250`。旧包仍有 pen/hook 引用，不可删除。部署只切入口不会改变存活 pen。
- 已集成部署：Agents 状态计时；图片宠物及 Blocks Clawd 仅腿动；Display Auto/Blocks；A 描边大头橘猫；Capybara 图片与 Blocks 两版。默认宠物仍 Clawd，Display 默认 Auto。对应任务书均在 `docs/任务/`。
- Capybara：实现 `ea665b3`，审查 `0776995`，合并 `33c00d7`，收尾 `eda201b`；预览 `docs/调研/卡皮巴拉宠物.md`。41 图片姿态、35 Blocks 姿态、七段动画，既有四套未变。该实现分支/worktree 已清理，实现者已关闭。
- Capybara 主控标准首轮 496 passed、1 failed、9 ignored；既有 workflow 鼠标任务表单失败，限定复跑 1 passed，补其他目标 128 passed，合计 625 项通过、9 忽略；不是一次全套绿。Clippy 通过，首次失败原因未证实，证据 `/tmp/saddle-capybara-review-*.log`。尚未在真实 Ghostty/Metalterm 完成卡皮巴拉视觉验收。
- A 图片猫及更早改动的验证/取舍见对应任务书，不重新开启已完成工作；部署历史也在各任务部署记录中。

## 核心与操作约束

- Rust Corral 已集成到 `crates/corral-core`，原 Corral 仓库只读保留。宿主、插件只通过公开接口交互。新版已安装，但关闭 Saddle 不会停止/升级旧 agent；重开接回旧实例不能当作核心升级。未来支持协议的实例通过经授权的公开 upgrade 流程切换，不是仅重开 UI。
- Rust 集成验证和原始失败记录见 `docs/调研/Corral核心Rust集成-实施核验.md`；部署记录见 `docs/调研/Corral核心Rust部署记录-2026-10-03.md`。只测过合成 agent 的产品生命周期，不能称真实 Claude/Codex/pi/omp 全路径验收。
- 主控遵守 AGENTS.md：功能开发用 corral-dispatch 派发，自己审查集成。现有用户 agent 只读，不擅自送话、按键、停止；不读 Corral 私有状态文件，不批量杀进程。
- 本调研未选择遥测、未创建或操作真实 Tasks run。历史队列快照不当作当前状态，不自动派下一任务。
- 构建共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；需固定二进制时用不可变包或显式 `--target aarch64-apple-darwin`，避免共享顶层产物被覆盖。纯文档归档验证用 diff/引用检查，不运行 Cargo 全套。

## 下一步

1. 本轮实现与审查集成已完成，不再派发、重跑已完成检查或消费晚到提醒重复工作。
2. 新主控、宿主及插件已完成本轮只读核对。按用户要求，本交接提交并推送后立即停止，等待用户提问；不自行启动升级、测试、派发或清理。
3. 后续真实客户端全路径兼容验证另行安排；本次新建替换不解决旧 agent 无缝首次迁移。旧 Tasks runs 不自动转移，旧包不删除。
4. 保留历史调研工作区，不顺手清理。设计评估会话已由用户要求关闭。

正式设计入口：`docs/DESIGN.md`、`docs/Corral核心Rust集成设计.md`。本轮的理由与双方意见保存在调查报告中。
