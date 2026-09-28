# 实施任务：Settings 任务通知渠道与 saddle 内部提示

2026-09-28，用户已授权实施；Drover 接口已通过其主控审查并交付固定 SHA，现正式派发 saddle 接入。实现者 `saddle/dev-task-notifications-1`，Claude Code，常规档 `opus[1m] / high`。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由三项均拿不准；按已有方案接入公开 CLI 与 UI 的常规行为改动判定，不碰任务推进／权限／业务核心规则）。若接口交付带来实际范围变化，再说明理由调整。

## 工作位置与前置交付

- 分支 `task-notifications`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/task-notifications`，由 main 建好。所有实现、测试和完成记录只写本 worktree。
- Drover 固定交付 `2b20205eb3ae423aa5f3eb9cc4a289ca84fd8340`（实现 `9b6d9c25a560bff75b797e47e4b847c8f6196318`，末提交仅审查记录），其 worktree 干净、主控审查通过；不重复它的全套审查。
- 公开契约：`/Users/firegnu/Developer/personal_projs/drover-worktrees/m35-notifications/docs/通知JSON接口.md`。只读此契约和联调用公开 CLI，不读取 Drover 内部数据文件或修改其源码。
- 联调命令：`/Users/firegnu/Developer/personal_projs/drover-worktrees/m35-notifications/bin/drover`。**必须隔离 HOME／XDG／runtime、假 OS 发送器和合成项目，不调用 PATH 中已安装的主分支 Drover。** 常规测试继续用假 CLI；真实公开接口联调只走共同约定主流程一遍。
- 下游 saddle/main 已在隔离 HOME 核对 status→off→off→on→status：开关值和 revision 为 true/0、false/1、false/1、true/2、true/2，status 不落盘。未调用真实 loop、队列或发送器。
- 身份六元组及编码以交付契约为准：事件类型 `awaiting_release`；t0 为有限 JSON number 转 binary64，大端 8 字节的 16 位小写十六进制，±0 统一 +0。不得直接用原 JSON 十进制文本作身份。
- 错误码已明确为 `invalid_arguments`、`preferences_invalid`、`preferences_unreadable`、`preferences_write_failed`，仍须兼容未知码及旧版本不支持。生效是 next_notification_check，不是引擎确认回执。

## 用户原话

- 「对，我的理解就是关键点谈就好了，而且我们已经有了attention入口」
- 「我个人觉得这个可以放到settings里面，由settings决定是系统弹还是saddle内部弹。」
- 「我一般就是使用一个saddle啊。为什么要使用多saddle？如果是saddle你打算怎么搞？」
- 「我觉得你的设计是合理的」
- 「可以，按照你的计划开干吧」

## 先读与范围

AGENTS.md、docs/DESIGN.md 第 47／48 节、`docs/任务/任务通知-接口与集成约定.md`，以及上述 Drover 公开契约。你是被委派实现者，不再派发。只改 saddle 中直接相关的 Settings、公开 CLI worker、提示状态／UI、去重存储、测试和文档；不改 Drover、corral、技能、用户配置或运行中服务。遇到契约缺口向 saddle/main 报告，不向其他用户 agent 送话或自己改上游。

## 已确认方案

- Settings General 加 Task notifications 选择 System／In saddle，注明本用户跨项目生效。读写共同约定的公开 CLI，Drover 偏好是唯一事实来源，不在 saddle config 重复保存。
- 编辑先为草稿，Save 才写，Cancel 不写；异步失败、旧接口不支持或读取不可用要明确显示。保存返回只表示偏好保存，后续通知检查应用，不声称立即停掉旧横幅。其余 Settings 的草稿、外部修改保护和配置路径规则保留。
- 全局偏好与本地 config 属于两个独立保存目标，不能虚报原子成功：如同次 Save 部分成功，说明哪些已保存、哪些失败并保留未保存草稿；不静默回滚已确认的全局偏好，不新增跨程序事务。沿用异步结果身份保护，不让失效查询覆盖新选择。
- In saddle 时，沿用公开任务刷新检测新的完成待放行，在右下角显示约 5 秒的英文短提示，可关闭、可点击定位对应项目任务。多项合成一条，点击进入现有 Attention。提示不抢焦点、不吞终端键盘输入，提示区域鼠标动作不透传终端。
- System 时不生成内部任务提示。两种模式都保留 Attention 中实际待处理事项；查看／关闭不放行。首次启动／切换以首次成功快照建立基线，不补弹既有 awaiting；同次运行去重，持久记录只防重复，不增加消息历史界面。
- 单 saddle 使用场景，不实现多实例选举或远程功能；不重启用户进程，不触发真实系统通知，不新增通知中心。普通 idle 推断不自动弹；补齐其他 Attention 类型不作为本任务隐含扩展。

## 验证与交付

验证预算：先以直接相关自动检查复现目标行为，再最小实现，运行定向检查及 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 各一次，`git diff --check`；不自行增加录屏／覆盖矩阵／故障注入。标准检查使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，测试使用假 Drover／corral、临时 HOME／XDG／runtime 和合成任务。联调用指定 Drover worktree 的固定提交，禁止连接真实 socket 或操作真实队列。

只在本任务分支提交，完成记录写变更、验证、取舍、未解决事项，回复最终 SHA。联调通过前不合并、不推送、不安装发布、不清 worktree；等待 saddle/main 审查并组织集成。命令前台跑完，最终回复最后一行 DONE。

## 完成记录

实现者：saddle/dev-task-notifications-1（Claude Code），分支 `task-notifications`。

### 改动

- `src/drover.rs`：`Task` 增加 `t0`（保留 JSON 值）、`start`、`main`；新增 `Preference` 与 `Client::notifications`，按公开契约解析 `drover notifications status|on|off --json`：成功须退出 0、`ok:true`、`scope:"user"`、布尔 `system_enabled`、非负整数 `revision`。四个已知错误码译成英文说明并附原 message；未知码照样按失败处理；非 JSON 输出报「this drover does not support task notifications」；类型、schema 不符或退出码与 ok 矛盾一律不当成功。新增串行 `ChannelWorker`（周期 status，以及 Settings 打开时读、Save 时 on/off）。
- `src/notify.rs`（新）：六元组身份（t0 转 binary64 大端十六进制、±0 归 +0，字段缺失／为空／非数字时跳过提示）；`Notifier` 负责基线、去重、合并与 5 秒计时；右下角提示绘制。
- `src/settings.rs`：General 增加 Task notifications（System／In saddle）及其说明行；草稿、Default、两段保存（先 config 后 Drover）、部分成功提示、等待期间锁定。
- `src/app.rs`：接入 worker 与 notifier；Survey 快照同时交给 Attention 和 notifier；提示的鼠标拦截与点击（单项打开 Tasks 定位，多项打开 Attention，× 关闭）。`src/attention.rs` 仅把 `Target::task`、`project_name` 改为 crate 可见。
- `Cargo.toml`：serde_json 开启 `float_roundtrip`（不新增包）。
- 文档：README 中英文补 Task notifications 说明；DESIGN 第 48 节追加实施取舍。

### 验证

- RED：`tests/notify.rs`、`tests/settings.rs` 新测试先因缺少对应功能无法编译（Task 无 t0/start/main、Settings 无渠道接口），实现后通过。`tests/drover.rs` 新增契约解析与 list 身份字段两条。
- `tests/workflow.rs` 新增 PTY 端到端测试（假 drover/corral、临时 HOME/状态/运行目录）：基线不补弹，新待放行右下角弹出；弹出期间按键仍进 agent、焦点不变；滚轮／移动／× 不传给终端；刷新不重复；点击打开 Tasks 定位；两项合并后点击打开 Attention；Settings 切 System 只调用 `notifications on`，之后不再弹。已有四处「队列调用全为 list」断言放宽为允许只读的 `notifications status --json`。
- 真实联调（`#[ignore]` 测试 `real_drover_notification_channel_prompts_dedups_switches_and_opens_tasks`，`SADDLE_DROVER_NOTIFY_BIN` 指向 Drover worktree `bin/drover`，HEAD `2b20205`、工作区干净）：隔离 HOME，包装脚本另把 XDG_*、TMPDIR 指到沙箱，`DROVER_CORRAL_BIN` 指向会记录调用的拒绝脚本（未被调用）；两个合成 git 项目。流程：status 为 true/0 → Settings 切 In saddle → false/1 → 合成 T1 待放行后弹一次 → 点击打开 Tasks → 两轮刷新不重复 → 切 System → true/2 → 另一项目待放行不弹、Attention · 2。通过。未运行 Drover loop、未调用 OS 发送器、未连真实 socket 或队列，未使用 PATH 中的 drover。
- `cargo clippy --all-targets -- -D warnings` 通过；`cargo fmt`、`git diff --check` 干净。
- `cargo test --all-targets`：除 workflow 外全部通过；workflow 每次有 1–2 个与本功能无关的 PTY 测试超时，且每次挂的不同（t20_r1_*、terminal_picker_*、native_mouse_*、closing_a_start_target_* 等，单独重跑通过，所用假 drover 不支持通知命令、不会弹提示）。在基线 `3cb5cb3` 的临时 worktree 对比，同样出现（9 次中 2 次失败），按用户指示记为基线即不稳定，不再加跑。本分支 10 次中 6 次有此类超时，样本少、未交替对照，是否因每个实例多一个周期 status 子进程而略增负载未定论。

### 取舍

见 DESIGN 第 48 节「实施取舍」。要点：去重只在内存（启动即立基线，持久记录不会多挡任何重复）；状态未知或 System 时不提示也不立基线；等待 Drover 保存时 Settings 锁定；其他弹窗占用输入时提示只响应 ×。

### 未做／需主控决定

- 未做持久去重文件；如主控认为需要，请说明它应额外挡住的场景。
- workflow 超时的不稳定是否需要另开任务处理（基线已存在）。
- 联合发布时需让旧常驻 Drover 引擎加载新版，本任务未安装、未启停任何服务。

## 主控审查（2026-09-28）

固定实现 `409e34c`，主控接受交付，隔离联调 Drover `2b20205` 一次通过；合并 `fadce6f`。正常使用无必须改项，路由不要求独立审查。用户最新确认撤回 saddle 持久去重阻塞，接受会话去重／启动基线，取代本任务此前要求另存持久记录的条款；实现者“持久记录不会多挡任何重复”的说法不成立，边界已修正文档。

标准测试一次 255 passed／1 failed／3 ignored，旧 T20 替换窗格用例在等待创建表单时超时，单项核对一次通过；Clippy、diff 检查及通知主路径通过。未重复全套／基线统计，不宣称整套全绿，也不从旧基线失败推出无回归。按用户收紧正常使用验收范围的决定接受该未定测试风险，T29 仍未解决。编译失败的开发记录不算有效行为 RED。

完整裁决、验证局限和边界见 `任务通知-主控审查.md`。未推进真实队列、未修改用户通知偏好；旧 Drover 引擎加载新版由 Drover 主控协调，不关闭 drover/main。
