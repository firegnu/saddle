# Drover 插件完整替换

用户原话：“drover我完全想替换掉啊”。确认：“是，全部由插件接管”；“接受，生命周期跟随 Saddle”。主控亲自实施，不委派。

## 范围

上一轮只完成界面/观察接入的插件化。本轮将任务核心、数据读写、项目登记、通知偏好与系统通知迁入Rust插件。旧Drover程序、CLI和独立watch退役；不再依赖Python Drover、不保留第二个任务引擎。Saddle退出停止观察和通知，数据保留；外部主控的任务调用经运行中Saddle转给插件。

保留已交付界面和简化流转规则。Corral/corral-dispatch、dispatch-log仍是独立infra，不修改。插件源码仍暂放Saddle仓库。

## 实施与验证

1. Rust核心承接已有queue.md/tasks.state及项目配置和登记，保留历史事实。延续既有数据路径，避免为换程序而重写全部历史；新的运行数据也由插件管理。用临时合成项目检查旧事件兼容、独立转换、令牌过期/并发锁、明确派发结果、编辑与通知偏好；先复现外部CLI缺失时当前插件不能读取的失败。
2. UI工作线程直接调用同一核心。系统/内部通知由插件按偏好选择，停用或Saddle退出即停；验证不启动旧CLI/watch。
3. Saddle新增通用插件命令转发，插件声明能力；主控读取/提交使用这一入口。宿主不解析任务规则。明确实例/请求身份、超时结果不自动重放，停用/重启不能把旧请求发给新进程。
4. 隔离验证完成后备份真实旧数据与服务配置，只读对比新旧任务投影；停用旧launchd watch并撤下旧命令入口，更新插件和使用文档。真实Pending/Running/Awaiting/历史不自动推进；不删除旧repo或数据备份。

Dispatch：7785984ca0bc4ff186b50014fc89263b。分支/worktree：drover-native-core。

## 主控审查与验证

- 主控亲自实现，没有新建或操作其他 agent。插件内部 Rust 核心承接旧数据，CLI 适配和构造旧命令参数的死代码已移除；Corral 与 dispatch-log 接口不改。宿主只新增通用 command.v1 转发。
- 原失败复现：没有旧 Drover CLI 时原 Client 读取失败；大正文使原命令列表超过48 KiB。新实现目标检查通过，列表改为分页摘要，展示令牌仍在任务锁内验证。
- 合成验证覆盖明确派发/退回/提交/接受、旧 gate=false/go 事实、错误 accepted 拒绝、陈旧/ABA令牌、并发锁、送达与记录分开、未发送/不确认/拒绝、不重发，以及一个用户只能有一个插件所有者。
- SDK/真实进程验证覆盖主控请求去重与冲突、停止后结果未知、不跨 session 重放、界面草稿/焦点保留、关闭视图保持观察、两类通知互斥、重启/偏好基线、单次系统调用。使用临时 HOME/任务和假 Corral/osascript，没有发送真实系统通知。
- 旧 CLI 专用断言改成原生数据及公开 Corral 验证；不再保留需要安装已退役 CLI 的4个可选测试。宿主界面场景继续保留，历史滚动 fixture 使用不与当前选中项重合的编号，Links 点击等待异步详情刷新；没有修改宿主选择器和点击保护规则。
- 标准套件第一轮最终结果330 passed / 0 failed / 4 ignored；Clippy通过。删除旧参数构造死代码后再跑交付套件（见后续交付记录）。日志 `/tmp/saddle-native-final-all.log`、`/tmp/saddle-native-final-clippy.log`。
- release隔离主控入口1 passed、内部通知1 passed、插件进程9 passed。日志 `/tmp/saddle-native-release-{ctl,notify,process}.log`。没有编造“肉眼看到了系统通知”。
- 一次性只读差分探针将3个真实项目旧CLI结果与新Rust投影比较，除不透明操作令牌及新分页令牌外语义精确一致：Drover 5待办/24历史，JB 0待办/1历史，Saddle 8待办/50历史；均无Running/Awaiting。探针不留在常规测试内，未写任务数据。日志 `/tmp/saddle-native-migration-read.log`。
- 保留的工程限制：通用ctl沿用每实例256条回执上限，满时拒绝，不静默淘汰或重放；单请求/结果48 KiB，过大详情明确失败。系统通知跟随Saddle，仍受macOS通知设置控制。

交付检查：`cargo test --all-targets -- --test-threads=4` **330 passed / 0 failed / 4 ignored**；`cargo clippy --all-targets -- -D warnings`、fmt、diff检查通过。最终日志 `/tmp/saddle-native-delivery-{all,clippy,build}.log`。现有Counter/Attention旧二进制对release宿主的兼容组 **3 passed**（`/tmp/saddle-native-release-compat.log`），未改demo。

主控审查结论：范围符合完整插件替换，无阻断项。进入联合安装，真实队列保持原样；用户当前Saddle窗口不强退，重启后加载新版。

### 安装验证发现的 ctl 读取缺陷

正式插件读取真实项目时复现 ctl 的 `instance_unavailable / Invalid argument (os error 22)`。最小socketpair验证：对端存活时设置读取超时成功，发送响应后关闭时设置SO_RCVTIMEO失败，但缓冲响应仍可正常读取。原ctl在每次读取前重设超时，因响应/关闭时序产生错误；不是任务引擎或插件退出。

新增“对端关闭后读取完整响应”测试先稳定RED，改用poll等待可读并保留原两秒总截止时间后GREEN；不完整响应仍拒绝。相关ctl集成4 passed。日志 `/tmp/saddle-control-closed-peer-{red,green}.log`、`/tmp/saddle-control-regression.log`。未更改任务状态、重试策略或其他控制操作。

修复候选通过真实三项目只读入口复验，调用正式安装的插件包、使用隔离宿主和假Corral，没有打开或操作真实agent。任务状态/数量与备份一致，队列/事件/配置/暂停文件字节均未变；隔离宿主已正常退出。日志 `/tmp/saddle-native-probe-fixed.log`。
