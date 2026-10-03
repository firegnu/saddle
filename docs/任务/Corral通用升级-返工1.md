# Corral 通用升级返工 1：保留恢复身份基准

2026-10-03，saddle/main 交给原实现者 saddle/dev-corral-upgrade-1（Codex 重档 gpt-6-astra / xhigh）。本任务属于已批准实施的必要返工，不另行扩大设计或授权。

你是被委派的实现者，不再派发。仍在 `/Users/firegnu/Developer/personal_projs/saddle-worktrees/corral-live-upgrade`、分支 `corral-live-upgrade` 工作；原任务的范围、禁令和正式设计继续适用。

## 先读与主控判断

- 原实施任务和专项设计 §3、§4。
- 主仓库 `/Users/firegnu/Developer/personal_projs/saddle/docs/任务/Corral通用升级-独立审查.md` 的「审查意见」第 1 项；此文件不在你的分支上，按绝对路径读取，不修改。
- 主控已核对候选 `ce6f675` 源码，认可该必须改项：resume 的 validate 失败后调用 Pen::save，会用当前未通过验证的资源重建 schema、fd 与锁身份；Hold/recover 也有同一路径。它可能撤销校验并破坏备用的原快照，是已设计 pre-active 故障路径的真实缺陷。

## 本轮要做的

修复上述唯一必须改项及修正直接引入的问题：失败记录和 recover 的升级元数据更新必须保留原始快照身份约束与流状态；未经原约束验证的资源不能因保存而被认可，不能取得清理权或进入流 I/O；仍持有原资源的备用应保有原快照的恢复依据。明确角色变更（如已退出备用的管道移除）不应重建其余身份基准。

修正方向以独立意见为参考，具体最小实现由你决定。不重开既定设计，不扩大到任意崩溃恢复、首次旧 agent 迁移或 Observer 后续升级。不顺手修无关问题。

## 验证预算与交付

- 按 AGENTS.md 为该缺陷建立可运行的针对性检查，先确认因缺陷出现真实 RED，再最小修正到 GREEN；选择直接相关恢复/备用/Hold 回归。如何构造证据由你判断。
- 只运行此次修正检查与直接相关回归，以及 `git diff --check`；不再运行 workspace 标准全套、clippy、重新打包、真实 agent 冒烟或扩大故障矩阵。主控已对修正前候选完成标准 test/clippy 一次且通过，不能称为修正后的全套结果。
- Cargo 继续使用共享 CARGO_TARGET_DIR 并显式 `--target aarch64-apple-darwin`；仅临时 HOME/CORRAL_HOME、合成程序与隔离固定二进制。不得覆盖已安装或既有不可变包，不读取真实 Corral 私有状态，不操作用户 agent。
- 只在实现分支提交源码/测试和原实施任务文件末尾「返工 1 完成记录」；不要改本任务书、主仓库、审查工作区，不合并、不推送、不清理 worktree 或关闭 agent。
- 记录根因、修正范围、RED/GREEN 的实际命令/结果/日志、直接回归和剩余边界。若需要改变已批准设计或超预算，先报告，不自行扩展。
- 命令都在前台跑完，全部做完后，回复提交号、结果及取舍，最后一行写 DONE。
