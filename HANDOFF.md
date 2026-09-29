# 交接

更新：2026-09-29。当前 main。用户要求 saddle/main 与 drover/main 直接完成 Running 撤回 Pending，不走任务派发流程；真实误派发 T29 留给用户实验。

## 本轮结果与待部署

- Saddle 实现 `9a65ba9`，合并 `302a7cc`，收尾 `6725382`；分支／worktree 已清理，没有新开 agent。main 的 release 已重新编译成功（`/tmp/saddle-return-release.log`），既有 `~/.local/bin/saddle` 软链无需修改；用户尚需重启 Saddle 查看入口。
- 选中 Running → `Return to pending…` → 原因 + `Work has stopped` → 确认。同编号正文回待办首位，队列暂停，Run details 显示撤回历史；按钮不停止 agent。独立令牌、项目与返回校验，失败须刷新重确认，不调用 drop/add/go/next 回退。设计见 DESIGN §55，验证见 `docs/撤回待办验证.md`。
- 新功能定向检查通过；Saddle 标准套件 282 passed／1 failed／3 ignored，失败为原 picker 用例（tests/workflow.rs:2294），单独一次通过；clippy 通过。不把全套记为全绿，不在此实现 T29 排查。
- 隔离联调通过：实际 Saddle Panel／Client 调实际 Drover CLI，临时 Git/HOME/XDG、合成任务，无真实 agent／队列操作；验证编号正文顺序、暂停、历史、重放拒绝和不派下一项。
- Drover 由 `drover/main` 完成并合入 main，最新回复 SHA `fdcd70d35c36a80915e4d285a13562036112c883`，尚未推送。对方报告 11 项专项、相关回归和两项变异检查通过；CLI 软链已加载新代码。未改 corral、dispatch-log 或共享技能。对方仍开着，勿自动关闭。
- **待用户授权重载常驻服务**：Drover 主控确认旧 PID 37103 仍加载旧逻辑；公开部署方式为 `launchctl kickstart -kp "gui/$(id -u)/dev.drover.loop"`。重启立即检查全部登记项目，可能执行验收／完成／派发，pause 不能挡住当前任务完成检查。对方读到三个项目 loop off，但重启前要复核；本轮未执行重启、未改任何项目开关。详见 Drover `docs/撤回JSON接口.md` 的部署段。
- 本次后续文档提交并推送仅涉及 Saddle；提交／远端状态以 Git 为准。真实 T29 实验尚未执行。

## 此前 T38 结果

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

本轮公开只读核对：current=T29、awaiting=null、loop=false、gate=true、paused=true；未执行 done/go/next/drop/return-to-pending 等真实队列写操作。T29 是用户误发的排查任务，本轮没有开始其排查或派实现者。

下一步处理上面的常驻服务部署确认；用户重启 Saddle 后自行用 T29 做撤回实验。不要把当前新功能开发成果记成 T29 已完成，也不要为了测试自动撤回或推进它。

旧 t38-dispatch-study 分支／worktree 按用户要求保留，未合入的调研文档仍可能影响后续 branches_merged，不自动处理。T37／T39／T40 已完成，不重复处理。

## 约束

- 不扩大范围、不读上游内部文件、不自动推进队列或启用循环。
- 复核按已经记录的预算，不重复全套、统计重跑或修无关问题。
- 不干扰其他用户 agent，不自动修改全局采集指令或用户配置。
