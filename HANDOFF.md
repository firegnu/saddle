# 交接

更新：2026-09-29，T40 已审查、合并、清理并重新编译 release。当前 main；合并 a226c88，收尾 6a0f9f5，随后提交本交接记录。

## 当前结果

- T40：Tasks 新增／编辑任务复用 Input，支持键盘光标移动、中间输入／粘贴／删除、鼠标定位及中文宽字符，多行正文可编辑；保存、取消和任务流程不变。
- 实现 5ca2a05；主控审查通过。目标缺陷有 RED→GREEN 记录；主控 cargo test --all-targets 一次 271 passed／0 failed／3 ignored，clippy 一次通过，diff 检查通过。本轮全绿不表示 T29 已修。
- 编辑正文的长行随光标横向滚动；查看任务页面仍自动换行。取舍已向用户说明，详见 DESIGN §53 及 docs/任务/T40-任务编辑光标.md。
- t40-task-editor 分支和 worktree 已安全删除；其 idle、attached=0 的自开实现者随工作目录一并关闭。只有 main worktree，其他用户主控保留。
- release 已重新编译成功，~/.local/bin/saddle 既有软链指向共享 .target/release/saddle；用户重启可用，本次没有替用户重启。未做真实界面手测，验证使用合成数据和自动检查。
- T39 已完成合并／发布／清理；当前已转入 T40，无需重复处理。T37 已完成发布并获用户确认放行。历史审查均在对应任务文件。

## 队列与下一步

收尾只读核对：current=T40、awaiting=null、loop=false、gate=true；pending 顺序 T38 → T29 → T28 → T32 → T34。没有执行 done／go／next，不自动推进。等用户验收和放行。

T38 的入口、范围、条数及持久化仍待讨论，不自动实施。T29 既有终端时序问题未修。

## 关键约束

- 核心稳定，使用公开接口，不顺手扩展 corral／Drover 或全局技能。
- 不干扰用户 agent，不自动关闭主控，不开启循环或替用户放行下一项。
- 按验证预算执行，不做统计重跑、基线对照或无关验证。
- T36 已完成两边发布与用户人工完成／放行，不重复处理。
- In saddle 通知偏好和 Hammerspoon 提醒保留；CCNotify 已授权移除及备份，不再操作。
