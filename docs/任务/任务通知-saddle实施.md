# 实施任务：Settings 任务通知渠道与 saddle 内部提示

2026-09-28，用户已授权实施；本任务等待 Drover 接口实现及其主控审查通过后再派发。家：Claude Code，常规档 `opus[1m] / high`；worktree 与 agent 名在正式派发时补齐，不得提前启动。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由三项均拿不准；按已有方案接入公开 CLI 与 UI 的常规行为改动判定，不碰任务推进／权限／业务核心规则）。若接口交付带来实际范围变化，再说明理由调整。

## 用户原话

- 「对，我的理解就是关键点谈就好了，而且我们已经有了attention入口」
- 「我个人觉得这个可以放到settings里面，由settings决定是系统弹还是saddle内部弹。」
- 「我一般就是使用一个saddle啊。为什么要使用多saddle？如果是saddle你打算怎么搞？」
- 「我觉得你的设计是合理的」
- 「可以，按照你的计划开干吧」

## 先读与范围

AGENTS.md、docs/DESIGN.md 第 48 节、`docs/任务/任务通知-接口与集成约定.md`，以及正式派发时指定的 Drover 固定 SHA 和公开契约。你是被委派实现者，不再派发。只改 saddle 中直接相关的 Settings、公开 CLI worker、提示状态／UI、去重存储、测试和文档；不改 Drover、corral、技能、用户配置或运行中服务。

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
