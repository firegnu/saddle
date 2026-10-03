# Corral 通用升级主控核实

2026-10-03。本轮已批准实施与审查集成，未批准真实部署或首次迁移。

## 候选与完成核对

- 实现者 `saddle/dev-corral-upgrade-1`，instance `7321e90e81f2`；公开状态 idle，完整回复末行 DONE。
- 候选 `ce6f675`：功能提交 `816358a9b23776da10f43d194910372c66294878` 加任务完成记录。实现 worktree 干净。
- 已读取完整回复、`docs/任务/Corral通用升级-实施.md` 完成记录及 diff。实现覆盖 pen/公开升级恢复入口、稳定 helper、pi/omp 原生采集与读取端判定、持久 after 和打包说明；未修改原独立 Corral 仓库。
- 主控已静态核对资源所有权、快照/fd 恢复、active/complete 边界、Hold/备用进程、request_id 与 worker 锁、采集事件兼容和包入口。最终通过结论仍须结合独立审查，不能以全套测试绿代替审查。

## 主控标准检查

在实现 worktree 对候选 `ce6f675` 各重跑一次，共用编译目录并显式指定 target，避免覆盖已安装或临时不可变产物。

```sh
CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test --all-targets --target aarch64-apple-darwin
CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo clippy --all-targets --target aarch64-apple-darwin -- -D warnings
```

- test：exit 0，641 passed、0 failed、9 ignored（66 条结果摘要）。
- clippy：exit 0，无警告。
- 原始日志：`/tmp/saddle-corral-upgrade-review.ito8C4/test-standard.log`、`/tmp/saddle-corral-upgrade-review.ito8C4/clippy-standard.log`。
- 这是主控对最终候选的一次全套绿；不覆盖实现者首次 test/clippy 失败历史。实现者的首次失败、限定复跑与修正仍保留在实施任务完成记录中。
- 本轮主控未重复固定包生命周期、专项故障矩阵或真实 agent 冒烟；这些已有隔离证据与未验证边界以实施记录为准。

## 独立审查进行中

- 独立 Codex 重档 `gpt-6-astra / xhigh`，`saddle/dev-corral-upgrade-review-1`，instance `dca43ca10e7b`，role=reviewer。
- detached worktree：`../saddle-worktrees/review-corral-live-upgrade`，HEAD `ce6f675`。
- 审查任务和唯一可写意见文件：`docs/任务/Corral通用升级-独立审查.md`。源码/测试只读；不重复标准全套，不操作用户 agent。
- 已通过公开 `corral send --after` 登记完成提醒，回执 pending。待核对同实例 DONE 与完整意见，再按技能预算处理必须改项；此时尚未批准合并。

## 实际边界

没有部署、切换真实命令入口、首次迁移、服务重启、操作用户 agent、Tasks 或遥测。旧 pen/旧 hook/旧 after 首次过渡限制，以及 Observer 退出码与 active 后崩溃边界仍然保留，不因隔离检查通过而改变。

## 主控审查：独立首审结果

已核对审查者同实例 `dca43ca10e7b`，公开 idle、完整回复 DONE；结论为 1 项必须改、0 项建议改。
主控核对 `resume/validate/save/hold` 控制流，认可「恢复校验失败后不能重新建立 fd 身份基准并覆盖原快照」；该问题属于已设计恢复路径中的资源身份缺陷，挡合并。
对其余八项取舍认可独立意见：保留既有分层、Observer/首次过渡/故障窗口与合成证据限制，不扩大本轮范围。
返工交回原实现者，任务 `docs/任务/Corral通用升级-返工1.md`；仅缺陷 RED→GREEN 与直接回归，不重复标准 test/clippy。完成后更新独立 detached 工作区到新提交，交原审查者限定复核。当前未合并。

## 主控核实：返工 1

核对实现者同实例 `7321e90e81f2`，idle/DONE；提交 `9648ff0`、`bd282db`，worktree 干净。改动仅 upgrade.rs、对应专项测试和实施完成记录。
源码改为持有完整 Snapshot 并原样保存身份基准；备用退出仅移除明确管道角色。Hold 控制资源单独核验。认可同一 socket 对端断开不应因权限位变化被视为资源替换，设备/inode/类型仍校验。
读取 `/tmp/saddle-corral-rework1.6GqKIm/` 的 RED/GREEN 和最终回归原始日志：初始缺陷真实失败、修正通过；相关 socket mode 误拒绝也有独立 RED/GREEN，最终 8 项集成、1 项单测通过。保留中间回归失败记录。
主控在 `bd282db` 仅重跑 `--test upgrade rejected_fd_identity`（2 passed）与 `--lib pen::upgrade::tests::descriptor_identity_survives_peer_disconnect -- --exact`（1 passed），使用指定共享 target 和显式目标；diff check 通过。未重跑标准套件或 clippy，不把旧候选的全套结果算作新候选全套结果。
原审查者 detached 工作区已更新至 `bd282db2339498b0f9589f99aafa2524c72a82e8`；交回限定复核第 1 轮，最终合并结论待其 DONE。

## 最终主控审查

2026-10-03，核对独立审查者同实例 `dca43ca10e7b` 的 idle/DONE、完整回复及「复核第 1 轮」：原必须改 1 项解决，剩余必须改 0 项、新增建议改 0 项，可以合并。
主控认可 Snapshot 保留、明确管道角色移除、Hold 控制资源身份与 socket 类型校验四项结论；结合前述主控定向复跑，批准合并候选 `bd282db`。
不追加第二轮复核、不重跑标准全套，不将修正前全套或旧临时包视为最终修正后的全套/发布包。真实部署、首次迁移和真实 agent 兼容冒烟仍未执行。

## 合并与清理实际结果

- 合并 `dab13d723f700b1558f3415c8817eb754d15bee6` 已成功推送 origin/main，未冲突、未改变已审候选源码。
- 清理前核对两名本轮 agent 均为原实例、idle、attached=0，两处 worktree 干净且提交已被 main 包含。
- 已不带 force 删除 `corral-live-upgrade` 和 `review-corral-live-upgrade` 两处 worktree，并以 `git branch -d` 删除已合并实现分支。
- 工作目录已删，一并关闭本轮实现者 `saddle/dev-corral-upgrade-1`（`7321e90e81f2`）及审查者 `saddle/dev-corral-upgrade-review-1`（`dca43ca10e7b`）；两次公开 stop 均返回 ok、exit_code 0。
- 历史调研/其他 worktree 保留；设计评估会话 `saddle/dev-live-upgrade-review-1`（`43b27dc12c46`）及主控 `saddle/main` 保留。没有操作用户 agent、部署或首次迁移。
