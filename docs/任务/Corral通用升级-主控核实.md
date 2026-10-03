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
