# 会话交接

更新：2026-09-30。Drover 已从 Saddle 内建功能迁成可选进程插件；主控亲自完成，没有委派。代码已合并推送，日常 release 文件已更新，但没有强制重启用户窗口，也没有替用户登记/启用插件。当前用户窗口仍可能是旧版，不能当作新版已运行。

## 当前交付

- 实现 `ff79aa6`、合并 `425ec55`、空提交收尾 `ae072dc`；本轮 `drover-plugin` 分支/worktree已清理。Dispatch `e6105d3907a3410b8defbbbfe1d69576`。任务、主控审查和发布证据：`docs/任务/Drover插件迁移.md`。
- 插件源码按用户选择暂放 `plugins/drover/`。原队列/CLI/详情/Links/Dispatch/通知规则迁入独立Rust crate，仅依赖公开SDK；插件具有界面和后台观察，关视图继续后台，停用停止并撤下来源，不自动派发/提交/接受。
- 宿主不再读取Drover登记项目、不调用Drover CLI、不保留Tasks入口和专用通知设置。Agent Attention保留，插件条目通过通用来源接口汇总。旧 `[queue]` 表兼容加载但不再生效；新agent候选来自公开Corral cwd和启动目录。
- 通用SDK新增光标、打开时cwd、动态Esc返回、用户输入绑定的agent导航、关闭视图和带目标通知。原instance/附着校验保留；导航新tab保护已有终端。快速输入和鼠标反馈重绘检查已补齐，旧Counter/Attention二进制兼容。
- 上游Drover地址已实查：`/Users/firegnu/Developer/personal_projs/drover`，origin `https://github.com/firegnu/drover.git`。上游源码/CLI核心/通知watch、本机真实队列/偏好、Corral/corral-dispatch和用户agent均未改。

## 日常版本与安装

- 入口 `~/.local/bin/saddle` 仍链接共享 `.target/release/saddle`。新SHA-256：`40671d1c538b29e5e0fd77ebc653b338ccef20dd35129bcb640bf66951c02ac4`，是已做release隔离PTY检查的同一二进制；实际入口 `--help` 通过。
- 旧程序与发布元数据备份：`/Users/firegnu/Library/Application Support/saddle-release-backups/drover-plugin-20260930-204507`。旧SHA `d92ff67177a1d6143264ae57f50e1661694a87a20d6d4db6c228eda3ea9dfc8a`。插件尚未登记，回退不涉及任务数据迁移。
- 正式目录：`/Users/firegnu/Developer/personal_projs/saddle/plugins/drover/dist/drover-plugin`。包含 `plugin.toml` 和 `bin/saddle-drover`，插件SHA `b0549c4b0eeb08dd4c0cdf700f52b178ce76b981c31a081120dc607e732bfeb6`。
- 本机PATH没有dlog，正式包的args已显式保留用户旧配置中的 `/Users/firegnu/Developer/personal_projs/dispatch-log/dlog`。源码清单通用默认仍为PATH查找。再次运行package.sh会覆盖清单，需保留此参数；详见插件README。
- 用户下一步：重启日常Saddle → Plugins → Manage plugins → Add local，填写上述完整目录 → Read manifest → Add disabled → Enable。关闭管理页，从Plugins选择Drover。没有Drover插件时仍可独立管理Corral agent和终端。
- 首次观察已有Awaiting只建基线、不补弹旧任务。观察新通知要使用之后的新Awaiting事件；用户未授权任何新测试任务或真实队列推进。

## 验证与残留

- 最终标准套件 **330 passed / 1 failed / 8 ignored**（`--test-threads=4`），Clippy和fmt通过。唯一失败：`ctl_shell_creation_is_idempotent_preserves_focus_and_confirms_close` 的close请求返回 `instance_unavailable / Invalid argument (os error 22)`；原样单独复跑 **1 passed**。根因未确认，不能写全套全绿，不扩大修控制服务或T29。
- T25退出检查的测试前提修正：等公开轮询确实观察ATT接入后再模拟agent消失，避免终端READY先到造成观察缺失；生产退出逻辑未改。目标和最终全套该项均通过。
- 实际插件进程合成检查7 passed；旧Counter/Attention兼容PTY组11 passed；SDK stdio隔离探针1 passed。
- release候选与正式包：插件PTY组11 passed、任务/导航组7 passed、新Awaiting通知/点击/关闭/偏好切换1 passed。均临时HOME、假CLI，无真实agent/任务。发布复制同字节产物，未改运行服务。
- 日志：`/tmp/saddle-all-final.log`、`/tmp/saddle-clippy-final.log`、`/tmp/saddle-ctl-final-recheck.log`、`/tmp/saddle-drover-release-{plugins,tasks,notify}.log`；完整记录见任务文档。跨worktree共享target时核对构建来源，勿把旧缓存/零匹配当验证。

## 保留上下文

- 用户长期方向：Corral/corral-dispatch可独立；Saddle是其GUI操作台，外壳归Saddle；功能以可选进程插件加入。统一Plugins面板管理状态/打开，插件不增加常驻侧栏按钮。路线已批准，不重开Wasm/Extism讨论。
- 原Claude讨论agent `saddle/dev-plugin-design-1` 已按用户要求关闭。用户要求主控后续亲自做，不再委派；没有新agent需要关闭。
- Counter正式目录 `examples/counter-plugin/dist/counter-plugin` 与Attention demo `examples/attention-plugin/dist/attention-plugin` 保持原样。旧demo用于合成演示，Drover插件才是真实任务来源。
- T38 `t38-dispatch-study`（c15bc4d）和T55 `t55-notification-flow`（3cc417d）工作树保留；不可为了清理而合并/删除。Drover仓库未跟踪T27核心拆分任务文档属于用户，不动。
- 任务流转v2此前已联合发布：done提交验收，go接受，next派发，各任务独立，Git检查作参考。T57/T58文档已交付并被用户接受；不重复推进。其他Pending不自动开始。
- Drover CLI/notifications watch继续独立，本轮不重写或删除它们。插件不是新的任务推进服务。

## 先读

`AGENTS.md`、`dispatch-log/USAGE.md`、`docs/DESIGN.md` §62及Drover迁移小节、`docs/插件协议.md` §10–11、`plugins/drover/README.md`、本轮任务文档。设计理由留设计文档；本交接只留状态和下一步。
