# 会话交接

更新：2026-09-30。用户已授权并完成 Drover + Saddle 简化流程联合切换。用户自行重启了 Saddle，主控核对新版进程、公开任务状态，并启动通知观察器。真实 T57 未提交或接受，插件设计未开始。

## 当前状态

- Saddle main 已合并：2a6862c；主控审查 c97270a；空提交收尾 c939fd1。本交接与发布记录随最终文档提交推送，确切 SHA 见 git log 和 Dispatch 05458465051541c4a9c2cc908dc9fe3e 收尾 note。无遗留本轮实现改动。
- Drover main 已发布 e0d8118（合并 b02f142，收尾16ef806），origin/main 已核对一致。Drover 无关未跟踪 T27 文档保留。
- 实际 Saddle 入口 ~/.local/bin/saddle → ../saddle-worktrees/.target/release/saddle；release 构建成功，SHA-256 e63d8e5ab76012e1b67e34b9232636a5ee3067972ea2382771f147a1d6d9dd76。用户重启后实例 5d8f7242dcc4636b、PID63344，已核对可执行路径。
- 服务 dev.drover.loop 已改为 `drover notifications watch`，launchctl running、PID83246；旧推进 PID46666 已退出。保留的 loop 名字不是自动推进功能。
- 切换后公开 list schema2：T57 Running、awaiting=null，T55 Pending；Pending 正文/顺序及45条历史状态/时间与切换前一致。Drover 主控还核对三个登记项目任务事实一致，真实任务存储未改。

## 验证与限制

- Drover 旧 gate=false 后 go 的兼容返工已复审；新 accepted 严格要求 Awaiting，不伪造旧接受。
- Saddle 主控标准检查292 passed / 1 failed / 3 ignored，Clippy通过。失败为既有 picker 用例 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases，独立 target 的 main基线亦在同一步失败；未修 T29，不能写全绿。
- 隔离联合主路径通过，主控复跑1 passed：A退回保留未合并分支，B提交后出现测试PTY内部提示/Attention，接受后Done，无自动派发。不是用户桌面现场通知测试；发布期间不为验证通知推进真实T57。
- 备份目录：/Users/firegnu/Library/Application Support/saddle-release-backups/20260930-135645。含旧数据/配置/服务、旧Saddle二进制、公开切换前后状态和核验。新事件不兼容旧二进制写操作，回退须协调版本与日志。

## 保留的工作

- 本轮 Saddle drover-schema2 和 Drover task-flow-simplification 分支/worktree 均已清理；自开 saddle/dev-drover-schema2-1 随工作目录删除已关闭。drover/main 保留，其他用户 agent 未停止。
- T55 的 t55-notification-flow（3cc417d）与 T38 的 t38-dispatch-study（c15bc4d）保留，不能为解除完成阻挡而合并/删除。
- T57 两行测试文档早已交付、推送和 Git 收尾；旧检查被 T55 分支挡住。现在新流程已上线，但尚未调用真实 done/go。
- T51 当前pane、T52 Shift+Enter 已交付并由用户放行。其他 Pending 不自动启动。

## 下一步

等待用户决定是否提交 T57 以观察真实通知。新版流程必须明确 dispatch-pending → done提交验收 → 用户go接受，Running/Awaiting可退回；没有Loop自动提交/派发。此次版本发布不等于T57完成登记，不自动放行或派发下一项。

Drover可选插件、Corral独立组合的设计等用户后续讨论；本轮没有改 Corral或corral-dispatch。开发、审查、worktree方式保持。

## 先读与记录

AGENTS.md、dispatch-log/USAGE.md；docs/DESIGN.md §61；docs/任务/Saddle-Drover联合发布.md及对应主控审查/隔离联调记录；Drover主仓库docs/任务流转JSON接口.md。真实操作仅走公开CLI。

Dispatch：Drover 0ec134ce246c420aabb6654254ddcf97；Saddle 05458465051541c4a9c2cc908dc9fe3e。Drover第一阶段回调若晚到，先核对已处理，不重复安装或启动服务。
