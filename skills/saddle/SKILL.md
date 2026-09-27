---
name: saddle
description: 在运行中的 saddle 内新建普通终端、显示或创建 corral agent、安排 tab 与四向分屏，以及关闭显示。用于用户要求操作 saddle 工作区；不用于开发任务分派或推进 drover 队列。
---

使用公开 `saddle ctl`，不要启动第二个 TUI，也不要读 corral/drover 内部文件。

先运行 `saddle ctl inspect`。结果包含 `instance`、`active_tab`、`active_pane`、`tabs[].panes[]`、布局及 `caller.pane`。调用者定位优先匹配 `CORRAL_NAME` 和 `CORRAL_INSTANCE`；普通 shell 使用 saddle 注入的实例、Pane 和修订号。不要按名称猜身份，也不要把用户当前焦点当作自己。定位缺失、正在替换或不明确时停止依赖该定位的动作，说明错误。

需要选择实例时运行 `saddle ctl instances`，使用返回的实际 ID；多个实例时询问用户，不选择“最新”的实例。`--instance ID` 可用于 inspect 和 open，request 和 close 必须显式传入。显式使用 `--relative-to active` 才相对接收请求时的活动窗格；已知稳定 Pane ID 也可作为 relative-to。

开窗格的示例（用实际目录、名称和返回 ID 替换占位符）：

```sh
saddle ctl open --place right --shell
saddle ctl open --place tab --shell --cwd /absolute/project
saddle ctl open --place left --agent project/review
saddle ctl open --place down --name project/helper --cwd /absolute/project --role regular -- codex --yolo
```

`--place` 仅有 `tab|left|right|up|down`，默认 relative-to 为 self。三种内容互斥：shell、已有 agent、精确完整名称加 `-- PROGRAM ARG…`。已有 agent 已在别处显示时移动现有 Pane，不再次 attach。新 agent 的 role 默认 regular，可用 controller、implementer、reviewer；argv 直接传给公开 corral start，不经过 shell，不自动追加权限或模型参数。可显式传 `--prompt TEXT`，但用户只要求开一个 agent 不等于授权发送任务；不自行补 prompt，不委派开发任务，也不调用 drover next/done/go。

shell 默认取来源 Pane 的已知启动目录；没有时取当前 Tasks 项目，结果的 `cwd` 和 `cwd_source` 明示来源。不会追踪 shell 后来的 cd。命令默认保留输入焦点；仅当用户明确要求切换时加 `--focus`。

每次修改读取返回的 `instance`、`request_id`、`pane`、`revision`，随后查询：

```sh
saddle ctl request REQUEST --instance INSTANCE
```

`accepted` 仅代表接收；`state=starting|attaching` 仍在执行；`agent_created` 与 `pty` 分开记录，`complete` 表示显示步骤完成，不代表模型就绪。`failed|target_invalid|uncertain` 必须如实报告；目标关闭后创建的 agent 仍可能存在，继续查询原请求了解外部结果，不自动 stop 或重建。

可提前指定 `--request-id ID`。超时后保留原 ID：先查原请求，必要时用相同 ID、相同参数及调用者身份重试；绝不自动换新 ID 重发。相同 ID 不同参数会冲突。实例重启后旧请求不可用，不承诺跨重启恰好一次。`busy` 表示用户正在操作布局、表单或确认框，等该操作结束再重试。实例最多记录 256 次修改，满后拒绝新修改并保留原记录。

按明确意图区分关闭显示与停止 agent：

```sh
saddle ctl close --pane PANE --instance INSTANCE
saddle ctl close --tab TAB --instance INSTANCE
```

关闭 agent 显示仅断开 saddle 自有 attach。含运行 shell 时返回 `confirmation_required`、`targets` 和 `confirmation`，此时没有关闭任何目标。向用户说明列表中的 shell、目录及其前台任务会结束；获得对这些具体影响的确认后，使用原目标、原凭据和一个新的请求 ID 完成：

```sh
saddle ctl close --tab TAB --instance INSTANCE --confirmation TOKEN --confirm-shells --request-id CONFIRMED_REQUEST
```

旧凭据因目标变化失效时重新查询影响，不能绕过确认。不要把“关闭”猜成停止 agent；明确要求停止时核对公开 `corral status NAME` 的身份并按用户授权调用公开 `corral stop NAME`。saddle 没有 ctl stop、发键、读屏、运行脚本或任务调度命令。准确语法可查 `saddle ctl --help`（同样输出 JSON）。
