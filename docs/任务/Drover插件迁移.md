# Drover 插件迁移

用户原话：“好的，我的问题没了，你开始吧”。已确认插件同时有界面和后台观察，关闭视图后台继续，停用才停止；不自动提交、接受或派发。主控亲自做，不委派。

## 已核实的仓库与实现

- Saddle：/Users/firegnu/Developer/personal_projs/saddle，当前 main 1de2016。本轮工作树 ../saddle-worktrees/drover-plugin，Dispatch e6105d3907a3410b8defbbbfe1d69576。
- Drover：/Users/firegnu/Developer/personal_projs/drover；实际 origin 为 https://github.com/firegnu/drover.git，不推测仓库地址。
- 已读 Drover AGENTS、最新HANDOFF、README、任务流转v2/通知契约及 bin/drover、bin/drover_core.py、bin/drover-board 的相关调用链。状态核心是 drover_core，CLI/watch/旧curses看板共用；无需在Rust里重写核心，也不需要替换现有CLI。
- Drover 现有未跟踪 docs/任务/T27-core-extraction 拆出核心逻辑与退掉Python看板界面.md 属于用户，保留。旧curses看板的删除不在本次迁移范围。

## 已确认源码归属

用户明确选择“插件暂放 Saddle 仓库”。新增 plugins/drover/ 独立Rust crate，通过公开SDK绘制，通过公开Drover CLI读取/写入。Drover仓库只读，不改CLI、核心、服务或其看板。Saddle主程序仅补通用接口并移除内建专用适配；该插件不依赖Saddle私有模块。

## 迁移清单与实施顺序

1. 迁移现有Rust Tasks列表、详情、添加/编辑/删除、项目选择、all pending、显式派发/提交/接受/退回确认、Links/Dispatch只读详情。尽量保留已有界面和令牌校验行为；仅通过CLI进行业务写操作。
2. 插件后台读取已登记项目及通知偏好，发布Attention快照；沿用公开notification_key、首次读取基线和无补弹语义。现有系统通知watch继续独立；插件不私读任务/偏好状态文件，不另启watch。
3. 仅因真实迁移缺口扩展通用协议：表单输入光标、打开时项目上下文、用户点击关联agent后的宿主导航、带目标通知点击。它们不能包含Drover任务类型或推进动作。关闭/失效/重新打开须拒绝旧请求，不允许插件越过身份验证接入另一个agent。
4. 移除宿主Tasks/专用通知设置、Drover轮询与Attention任务来源；Agent Attention保留。新建终端/agent的默认目录和项目候选不再依赖Drover是否安装。
5. 迁移并运行相关回归，加入插件未安装/禁用时宿主不调用Drover的检查；插件使用合成队列/假CLI验证，联调只使用隔离HOME。切换前确认功能、配置、旧来源撤除和回退产物，不能让两份来源同时通知。

本文件记录主控实现方案，不扩写用户原始验收原话。当前已实现插件与宿主解耦并完成主控审查；尚未操作真实队列、服务、配置、插件登记或其他agent。


## 主控实现与阶段证据

- 现有队列/详情/Links/Dispatch/CLI业务与纯业务测试迁入 `plugins/drover/`，只依赖公开SDK。宿主删除旧任务状态、Tasks入口、Drover轮询/通知偏好与任务Attention；旧queue表兼容加载但不生效。
- 新增通用 cursor、来源cwd、Esc返回、关闭视图、绑定用户输入的agent导航、带目标短提示。插件关闭视图保留后台、停用终止；现有Corral身份和附着校验保留。
- 游标越界/宽字续格检查先RED后GREEN；列表q关闭视图先RED后GREEN。实际插件可执行文件测试使用临时HOME和假CLI：表单光标、多行粘贴、返回、偏好保存/失败重试、后台Attention/新Awaiting通知均已通过阶段检查。
- 迁移PTY检查暴露SDK快速键入丢字、鼠标按下后重绘导致松开丢失、控制修饰键control误写ctrl，均先复现再修正。另发现通知关闭后松开泄漏到终端，已补手势拦截并复验。相同完整帧不重复增加frame_id，相同Attention内容不重复增加来源修订。
- 未安装Drover插件的宿主检查已通过，保留旧配置与项目登记也没有调用Drover。最终检查结果见下节，不能把阶段通过写成全套全绿。
- 主控没有委派或发送agent消息；上游Drover/Corral/corral-dispatch、真实队列、系统通知watch和当前运行窗口均未变。

## 最终主控审查与验证

- 已核对模块迁移、SDK/宿主能力协商与帧/输入约束、导航原实例与附着校验、Attention/通知会话隔离和基线规则。纯CLI业务模块直接搬迁；宿主不导入插件私有模块。源码搜索宿主 Drover 只剩帮助文案和旧配置兼容说明。主控审查通过。
- 标准检查：`cargo test --all-targets --no-fail-fast -- --test-threads=4` 为 **330 passed / 1 failed / 8 ignored**，日志 `/tmp/saddle-all-final.log`。唯一失败 `ctl_shell_creation_is_idempotent_preserves_focus_and_confirms_close` 的 close 请求返回 `instance_unavailable / Invalid argument (os error 22)`；原样单独复跑 **1 passed**，日志 `/tmp/saddle-ctl-final-recheck.log`。根因未确认，不归因为插件，也不宣称全套全绿、不扩修控制服务。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --all --check` 通过。SDK stdio隔离探针显式 **1 passed**；原已打包 Counter/Attention 二进制的兼容PTY组 **11 passed**。
- 新增实际进程测试 **7 passed**：表单光标/多行粘贴、关闭请求、偏好保存及失败重试、关闭视图后台观察、鼠标重绘松开、快速有序输入与重放拒绝、失焦取消手势。仅假CLI/临时HOME。
- T25退出测试此前仅等待终端READY就删除agent，未保证公开轮询已观察到接入。本轮去除宿主任务后台使启动更快，暴露此测试时序。测试现等待公开 ATT 接入数，再模拟消失；目标与最终全套均通过，生产退出逻辑未修改。
- 打包命令通过；release插件包与目标目录中的实际插件二进制SHA一致 `b0549c4b0eeb08dd4c0cdf700f52b178ce76b981c31a081120dc607e732bfeb6`。独立目标目录release宿主验证：插件PTY组 **11 passed**、任务/导航组 **7 passed**、新Awaiting通知及点击/关闭/偏好切换主路径 **1 passed**，全部临时HOME和假CLI；日志 `/tmp/saddle-drover-release-plugins.log`、`/tmp/saddle-drover-release-tasks.log`、`/tmp/saddle-drover-release-notify.log`。
- 增加workspace第二个可执行文件后，原 `cargo run -- --help` 出现二义性；显式 `default-run = "saddle"` 保持原启动方式。此配置用实际命令检查。
- 本次真实任务、系统通知watch、Drover/Corral/corral-dispatch源码与用户agent均未操作；插件不会自动登记或启用。发布记录和正式目录见 HANDOFF。
