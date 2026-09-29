# 交接

更新：2026-09-29，T39 已审查、合并、编译 release 并清理。当前 main；合并 5f99074，收尾 e624e3d，随后提交本交接记录。

## 当前结果

- T39：Tasks 顶部明确显示 Queue: 状态，默认 Task text 标题前显示所选任务自身状态，复用既有文字和颜色；未改状态判定、流程或异步逻辑。
- 实现固定 ca2be4f，主控审查通过。相关渲染测试通过，clippy／fmt／diff 检查通过；主控按纯展示预算只核对 diff，没有重跑套件。
- 实现者超出预算重跑 workflow，T29 相关 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases 用例仍时过时不过；已记录流程偏差，没有扩入 T39 或追加验证，不称整套全绿。详细记录见 docs/任务/T39-Tasks状态标识.md。
- release 已重新编译成功；~/.local/bin/saddle 的既有软链指向共享 .target/release/saddle，新产物已确认包含 Queue:。用户重启即可看见，本次没有替用户重启。
- t39-status-labels worktree／分支已安全删除，自开实现者随工作目录一并关闭；调查 agent 此前已按用户要求关闭。其他用户主控保留，只有 main worktree。
- T37 此前也已按用户要求编译 release，用户已确认放行，任务闭环。先前交接里“未编译／待放行”已过时；设计与审查仍见 §51 和 T37 任务记录。

## 队列与下一步

本次收尾只读核对：current=T39、awaiting=null、loop=false、gate=true；pending 顺序为 T38 → T29 → T28 → T32 → T34。没有执行 done／go／next，不自动推进；等用户验收与放行。

T38 仍只记录近期操作结果方向，入口、范围、条数和是否持久化待讨论，不自动实施。T29 既有终端时序问题仍未解决。

## 关键约束

- 核心稳定，新需求优先公开接口；不顺手扩展 corral／Drover 或全局技能。
- 不干扰其他用户 agent，不自动关闭主控，不启动循环或替用户放行下一项。
- 验证遵守任务预算，纯展示修改不要再顺带跑 workflow、做基线统计或重复整套。
- T36 已在两边完成合并、发布、清理和用户人工完成／放行，无需重复处理。
- In saddle 通知偏好与 Hammerspoon 提醒保留；CCNotify 已授权移除及备份，不再操作。
