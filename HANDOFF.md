# 交接

更新：2026-09-29。当前 main，T38 发布交接已提交并推送至 `aaf2079`。本次在项目 `AGENTS.md` 加入主控使用 dispatch-log 的说明；`CLAUDE.md` 是指向它的符号链接，无需重复编辑。用户准备重启主控验证。

本次提交范围仅 `AGENTS.md` 与本交接文档，提交前无其他 saddle 工作区变更；只改文档，检查 diff，不重复代码测试。提交、推送结果以 Git 当前状态为准。

## 当前结果

- Tasks 详情为 Task text | Run details | Dispatch | Links。Dispatch 通过独立记录器的 ls/show/cat 只读展示派发、返工、审查和全文；缺少记录器／数据、读取失败或不兼容时如实显示，其他任务功能继续使用。
- 实现 `bf9cb6a`，契约修正 `da39a9d`。JEV 建议读取对象 verdict（null 是 uncertain），corral 回复数字 at 单独显示来源时间，不与观察时间混同。
- 主控首轮标准套件 277 passed／1 failed／3 ignored；失败为控制 socket 用例 `control_socket_is_private_and_slow_clients_do_not_block_terminal_input` 的 inspect 返回缺 instance，单独一次通过，原因尚未定位。不记成全套全绿，也不直接认作 T29 根因。clippy 通过；返工后只复跑相关 6 项，全通过。
- release 已在 main 编译成功，`~/.local/bin/saddle` 原有软链指向共享 `.target/release/saddle`。用户随后提供截图，确认已能在 Dispatch 页看到 T38 的真实派发、回复、返工与审查列表；截图未验证各条全文。自动检查使用合成记录和假 CLI。
- 实现分支 t38-dispatch-view／worktree 已安全删除，`saddle/dev-t38-dispatch-view-1` 因工作目录已删一并关闭。旧 t38-dispatch-study 分支／worktree 按用户要求保留（c15bc4d 的调研文档未合入）。不要自动删除它或关闭其他用户 agent。

## 记录器与使用

- dispatch-log 独立仓库由 `dispatchlog/main` 负责；此前核对版本 `9cd295a`。随后按用户要求新增 `agents模版.md` 并补充其 HANDOFF，这两份文档尚未提交，不包含在本次 saddle 提交中。未改记录器代码、corral、drover 或共享技能。
- 真实派发 ID：`973aed4655fa419fa4d78e0176d7085a`。JEV B 输入／解析返回、决定、任务书快照、回复、返工及审查均通过记录器保存。查询走 `/Users/firegnu/Developer/personal_projs/dispatch-log/dlog show <id>`，不读取私人会话日志。
- saddle 读取配置为现有 `[queue]` 下的 `dispatch_log = "/Users/firegnu/Developer/personal_projs/dispatch-log/dlog"`；用户已成功看到记录，无需再次配置。本轮未改用户配置，也未安装全局入口。Settings／Diagnostics 未新增这个配置项。
- 项目主控后续先读 `/Users/firegnu/Developer/personal_projs/dispatch-log/USAGE.md`，按说明记录路由、派发、回复及审查收尾。仅主控负责采集，实现者和审查者无需重复采集；仍遵守原有授权与放行流程。新会话读取 AGENTS／CLAUDE 后沿用，已有会话需重新读取。不是后台自动采集，绕过记录器的操作和未记录历史不会自动补齐。
- 打开时读取一次，Refresh r 重读；全文 Esc 返回原条目。设计与取舍见 DESIGN §54；主控审查见 `docs/任务/T38-主控审查.md`，任务书见 `docs/任务/T38-单任务派发记录.md`。

## 队列与下一步

上次公开只读核对：current=T38、awaiting=null、loop=false、gate=true、paused=false；本次文档修改未重新查询队列。未执行 done/go/next，等待用户放行。旧调研分支未合入，可能继续挡住 Drover 的 branches_merged；如要处理需由用户决定，不为自动完成清理它。

下一步由用户重启主控，验证它能读取项目采集说明，并在后续获授权的任务中使用记录器。不要为了测试自行派发或推进真实队列。

T37／T39／T40 的代码和 release 已完成，历史审查在对应任务文件，不重复处理。T29 时序问题未修；本次控制 socket 单例失败保留记录，不扩入 T38。

## 约束

- 不扩大范围、不读上游内部文件、不自动推进队列或启用循环。
- 复核按已经记录的预算，不重复全套、统计重跑或修无关问题。
- 不干扰其他用户 agent，不自动修改全局采集指令或用户配置。
