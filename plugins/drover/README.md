# Drover 插件

Drover 是一个完整的可选 Saddle 进程插件：同一进程提供任务界面、任务数据读写、状态流转和通知。无需安装旧 Drover CLI、Python 程序或独立 watch。Corral 仍负责 agent，Saddle 核心提供可选遥测；插件只通过公开命令与通用导航能力访问。05A 已移除旧 Dispatch 日志页和 dlog 执行依赖；2026-10-02 已与新版宿主成套更新日常安装，见 [切换记录](../../docs/任务/遥测05B-实际切换记录.md)。

## 安装与生命周期

在仓库根目录打包：

```sh
CARGO_TARGET_DIR="$HOME/Developer/personal_projs/saddle-worktrees/.target" ./plugins/drover/package.sh
```

Saddle → Plugins → Manage plugins 添加生成的完整目录 `plugins/drover/dist/drover-plugin`，启用后从 Plugins 选择 Drover。目录须同时包含 `plugin.toml` 和 `bin/saddle-drover`。更新前停用，更新后重新启用。

关闭面板只关闭视图，后台继续观察；停用插件或退出 Saddle 停止观察与通知。重开恢复数据、建立通知基线，不补弹此前已等待的任务。每个用户的数据只允许一个 Drover 插件进程持有，第二个实例会明确失败，避免重复通知和并行所有者。插件不自动派发、提交、接受或执行检查命令。

保留 `~/.drover/projects` 登记目录、各项目 `.drover.conf` 的 `HANDOFF_DIR`、`queue.md`、`tasks.state` 和通知偏好。旧事件只解码，不重写或补造接受记录。所有写操作持有任务锁，提交/接受/退回校验运行身份与令牌；Git 分支和检查记录只作参考，不阻挡状态流转。

需要覆盖命令或初始目录时，在包中 `plugin.toml` 的 `[view]` 之前填写：

```toml
args = ["--corral", "corral", "--cwd", "/absolute/project", "--refresh-ms", "2000"]
```

默认从 PATH 查找 corral。旧清单中的 `--dispatch-log <值>` 仍兼容解析，但已弃用，值不展开、不读取、不执行；新清单不再提供，不自动修改用户清单。初始项目选择已登记的启动目录，否则第一项；打开时可跟随当前 agent 仓库。旧 Saddle `[queue]` 配置已不生效，旧 `--drover` 参数移除。重新打包会覆盖包清单，应保留本机自定义 args。

## 界面操作

保留添加/编辑/排序/删除、项目选择、All pending、任务详情、Links。显式派发将所选 Pending 交给配置的 `MAIN_AGENT`；未配置时登记 Running 并提供手动发送文本。没有自动发送下一项。

Running → Submit for review → Awaiting release → Accept → Done。退回 Pending 需要原因和确认工作已停止。各任务的提交不要求其他任务分支合并。删除仅作用于 Pending，保留 Dropped 历史。派发送达与运行登记分开报告，无法确认时不自动重发。

### 可选遥测记录与跳转

是否记录派发由 Drover 按项目保存：`.drover.conf` 的 `TELEMETRY_RECORD=on|off`，缺省为关；顶部 `Record default`（键 `R`）切换并立即保存。Dispatch selected 旁的 `[x] Record`/`[ ] Record` 只改这一次派发，换选任务后回到项目默认。它只决定是否请求记录，不改变项目是否采用主控分派；Saddle Settings 中的 Telemetry recording 总开关仍决定能否采集，Drover 不会开启它。

选择记录时，Drover 通过 Saddle 为插件进程提供的 `SADDLE_HOST_BIN` 调用公开 `saddle telemetry`/`saddle agent`：先在 300 ms 内建立 trace 与 controller_handoff 身份；建立不成（总开关关闭、存储不可用、没有宿主路径、超预算）时在发送前照原样直接 `corral send`，并注明没有记录上下文。有上下文时，交付消息末尾附一段只含 trace_id/dispatch_id/task/run 的记录上下文（不是任务内容，也不构成授权），经 `saddle agent --corral <原程序> --record-context … -- send` 只发一次，并在独立进程组中运行以便取消。宿主回执配对且 executed=false 才算未发送；其余按 Corral 原结果映射，缺回执/超时/取消为未知；已启动后绝不改走直接发送或自动重发。先保存任务状态，再追加 task.transition；之后的提交/接受/退回按该轮完整 binding 查到 trace 后追加，没有 trace 就不补。结果的 `delivery`/`record` 不变，另有 `telemetry` 字段。细节见 [遥测使用](../../docs/遥测使用.md)。

有任务号的任务详情页签行右侧有 `Telemetry ↗`，在 Saddle 查询页打开该任务所有轮次（宿主需支持 telemetry.open.v1；无需 dispatch 插件）。未编号待办不显示。旧 Dispatch 日志页已移除；Task text、Run details、Links仍可切换、刷新和返回，Dispatch selected 业务动作保留。旧 dispatch-log 数据不迁移、不删除，仅保留供离线历史查看。

单任务边界版本中，Accept/Return 业务落盘、尝试追加 transition 后，插件用已解析的同 run trace 调用 `saddle telemetry trace close`，独立预算300 ms；Submit 不结束。流转结果增加 `telemetry.close:{status}`，与外层 transition 的 status 分开：stored/duplicate 表示结束，失败如实报告；找不到 trace/查询失败或 Submit 为 not_attempted，不新建、不重试、不重放业务、不影响状态成功。Return 原因先随 transition 尝试写入，失败后 close 仍尝试，遥测可能永久缺这条原因；Drover 原始退回历史保留。关闭不证明任务成功，Run details 和通知语义不变。

同 run 内实现/审查返工继续同 trace；Return 后新 run 按既有项目默认/本次覆盖决定是否记录。主控对绑定 Tasks run 的链路只写 closure 声明，不提前 close。宿主、Drover 和技能 revision 3 须成套交付；本次源码实施未安装或迁移真实库，部署前备份及旧二进制限制见[遥测使用](../../docs/遥测使用.md)。

`N` 打开通知偏好，System/In Saddle 互斥，`Ctrl-S` 保存。系统通知由插件内的工作线程调用 macOS osascript；内部通知通过 Saddle 的通用通知接口显示。首次观察和偏好切换只建基线。Esc 返回子页面，列表 Esc/q 关闭视图，Ctrl-] 回 Agents。关联 agent 仍由宿主核实原始 instance 后打开。

Run details 顶部主状态栏突出人工处理：`YOUR REVIEW NEEDED` 表示本次有可读收尾报告需你核对后提交，`AWAITING YOUR ACCEPTANCE` 表示已经提交待验收，`ACCEPTED` 表示已验收；缺少可读报告时为 `COMPLETION UNCONFIRMED`，不会推断 Agent 仍在运行。刷新失败时状态不可用，旧数据明确标注。下方全部记录保留。

Run details 的 **Controller reports · this run** 按当前项目、任务和 run 读取宿主遥测中已有审查/收尾报告，显示声明的结果与正文。首屏另显示本次各类消息交付、路由、Agent启动/回复的最新记录与条数，和人工提交/验收状态、下一步提示；完整报告先于原始运行和仓库参考信息。各类最新事件可能来自不同委派或迟到结果，完整顺序到遥测核对。命令退出0、回复已读取、主控声明都不自动证明任务成功。它不判定任务成功，不自动 Submit/Accept；无记录或查询失败显示未知。退回原因在退回业务保存后随 task.transition 记录，可在 Telemetry 选中该事件后打开 reason 正文。更新前的旧遥测事件不会自动补写原因。

## 主控命令

主控通过运行中的 Saddle 转交命令；Saddle 只处理通用插件请求，不解释任务业务。先 `saddle ctl instances` 选择承载 Drover 的实例。调用示例：

```sh
saddle ctl plugin --instance INSTANCE --plugin drover --method list \
  --params '{"project":"/absolute/project"}' --request-id UNIQUE_READ_ID
saddle ctl request UNIQUE_READ_ID --instance INSTANCE
```

首次返回 `state: plugin_pending`，用同一实例查询到 `complete`，业务结果在 `result`。同一请求 ID 和相同参数只返回原回执，不重复执行；参数变化返回冲突。刷新使用新的请求 ID。不能把首次 accepted 当业务成功。断连/停止/超时是 uncertain/result_unknown，先读取实际状态，不能自动重发。没有运行中的 Saddle 或未启用插件时调用失败。回执沿用宿主每实例最多256条限制，不静默淘汰；容量满时拒绝新命令，旧回执仍可查询。

| method | params（除全局项外均需绝对 project） |
|---|---|
| projects | 无，读取登记项目 |
| register | project、name（小写字母/数字/短横线）、可选 main_agent；创建配置并登记，不派发 |
| notifications | 无为读取；system_enabled 为保存用户偏好 |
| list | 可选 pending_offset/history_offset（从0开始）；两组各最多20条摘要，并返回 total/offset、queue_token、record_default |
| show | id；读取完整任务和仓库参考信息 |
| add | title、可选 body |
| edit | pos（从1开始）、title、body、读取时的 queue_token |
| move | pos、to（均从1开始）、queue_token |
| drop | pos、queue_token |
| pause / resume | 禁止/允许显式派发；不会自动派发 |
| dispatch | pos、所选任务 actions.dispatch-pending.target_token、可选布尔 record（本次是否记录，省略取项目默认）；结果另含 telemetry |
| submit | id、所选 Running 的 actions.done.target_token（submit/accept/return 结果另含 telemetry） |
| accept | id、所选 Awaiting 的 actions.go.target_token |
| return | id、actions.return-to-pending.target_token、reason、work_stopped:true |

命令参数/结果最多48 KiB；列表只给摘要，正文按 show 获取。过大的单任务结果明确返回 result_too_large，任务数据不会被截断或改写。令牌过期要重新读取并确认；禁止默默替换令牌重试。初次切换后应重新读取令牌。

## 从独立 Drover 切换

先备份原程序入口、launchd 配置、Saddle 配置和任务数据，完成新旧投影只读对比。然后停用指定的 `dev.drover.loop`，移走其自启动入口及旧 drover/drover-board 命令链接，更新 Saddle 和插件包，登记启用插件，重启 Saddle。不要同时保留两个观察/写入入口。旧仓库与数据保留；回退须先停用新插件，再恢复旧程序与服务。切换本身不派发、提交或接受真实任务。

## 在 Tasks 中接入项目

打开 Tasks → 项目选择器（c）→ Add project（a）。输入绝对目录，或点 Browse 浏览目录后选 Use directory；Check 显示接入状态。新项目确认短名，并可从 Choose agent 选择已有接收主控，Save 后进入任务列表。主控留空表示手工交付，不是配置错误。目录已接入时 Open 进入现有项目；发现旧配置时 Reuse & add 沿用原配置、绑定和任务数据。登记项目配置异常仍保留在列表，显示原因，不覆盖初始化。

Projects → Settings（s）可改默认接收主控；No receiver 清空绑定。Cancel 不保存，外部配置改变则需要 Reload 后再保存。这里的修改只影响以后明确发起的派发，不更改已有运行记录。不创建或发送 agent，不自动推进任务，不管理 AGENTS.md。新接入会将 .drover.conf 加入 .gitignore；该步失败会显示已接入但忽略规则未更新的提示。停用插件保留项目和任务数据。

插件新增公开命令（project 为绝对路径）：project-info 只读检查，返回 state（new/existing/registered/unavailable）及配置 token；project-save 使用 token，新建还需 name，已登记项目可设置 main_agent，未登记旧配置原样沿用。project-browse 只列指定目录的子目录，project-agents 只经 Corral ls 列出现有 agent；不会扫描所有项目或创建 agent。

### Bundled agent runtime

When launched by Saddle, the default agent program is its resolved absolute `SADDLE_AGENT_BIN`. Explicit `--corral PROGRAM` still wins; standalone invocation without the variable uses `corral` on PATH. This changes executable selection only, not task transitions or recording choices.
