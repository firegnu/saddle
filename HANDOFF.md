# 交接

更新：2026-09-29，T37 已审查、合并并清理。当前 main；合并 9ebd457，收尾空提交 3ebd736，随后提交本交接与审查落盘记录。

## 当前结果

- T37：Settings → Diagnostics F4 提供只读诊断，手动刷新／复制摘要；具体设计与取舍见 docs/DESIGN.md §51。
- 实现 36bc598、摘要修复 a24da03。主控发现配置解析错误携带原值，一次返工后复核通过。审查见 docs/任务/T37-主控审查.md；任务记录见 docs/任务/T37-只读诊断入口.md。
- 主控标准测试各一次：cargo test --all-targets 为 265 passed／2 failed／3 ignored；clippy 通过。后续只复核相关 diagnostics／settings 共 21 项通过；原两个失败用例 t25_exited_original_status_is_not_reattached 和 terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal 各单跑一次通过。不能改写为整套全绿，T29 既有时序问题未修。
- t37-diagnostics worktree／分支已安全清理；实现者 idle、attached=0 后随工作目录一并关闭。只剩 main worktree。其他用户主控 corral/main、drover/main、globalmesh/main、saddle/main 保留。
- 本次未发布 release、未更新已安装二进制、未重启运行中的 saddle；真实剪贴板写入没有现场验证。

## 队列与下一步

本次收尾只读核对：current=T37、awaiting=null、loop=false、gate=true；pending 顺序为 T38 → T29 → T28 → T32 → T34。没有执行 done／go／next，不自动推进。T37 等用户验收及后续队列操作；如果用户要在已安装版本使用，另行构建发布，不把已合并视为已部署。

T38 仍仅记录“近期操作结果”，入口、范围、条数与是否持久化待讨论，不据此开始实现。其余待办按用户指示处理。

## 关键约束

- 保持核心稳定，新需求优先使用公开接口，只改 saddle；接口缺口先说明，不顺手扩展 corral／Drover 或全局技能。
- 不干扰其他用户 agent，不自动关闭主控；不启动循环或替用户放行下一项。
- 验证按任务预算，不追加统计重跑、覆盖矩阵或真实 agent 操作。历史 T29 用例失败不能因本次单跑通过而记为修复。
- T36 已在两边完成合并、发布、清理和用户人工完成／放行，无需重复处理。历史详情见 docs/任务/T36-主控审查.md。
- 既有通知 In saddle 偏好保持；Hammerspoon 提醒保留，CCNotify 已授权移除及备份，不再操作。
