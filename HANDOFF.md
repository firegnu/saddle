# 交接

更新：2026-09-28。当前分支 `main`；T35 收尾为 `a4cfa1a`，上一交接提交为 `ea267c2`。以下状态已用公开 CLI 和 Git 核对；开始新工作前仍需刷新实时状态。

## 会话摘要

完成 Drover 系统通知与 saddle 内提示的渠道设置、联合审查、隔离联调和两边发布。用户已选择 In saddle，并亲自看到 T35 完成提示、完成手动放行；通知主流程验证闭合。随后开展 T36 评估，用户已确认“人工确认完成→等待放行”方案；用户随后明确 T36 已正式下发，现已送达 Drover 主控开展第一阶段，消费方等待固定交付。

## 已完成

- saddle 任务通知实现 `409e34c`，合并 `fadce6f`，收尾 `94f50be`；主控审查、联合发布记录在 `docs/任务/任务通知-主控审查.md`。release 已更新，默认 `~/.local/bin/saddle` 仍链接共享构建目录，发布二进制 SHA-256 为 `f19b4d78848a7c1a119e2d4de044a99bbab130babc337d725b4ef9971bc1287a`。
- Drover main/origin 固定交付落地 `46d769915901fb9f80c2f73cd26f0477c1a94353`，合并 `2c52bc6`、收尾 `6c52bda`。其主控确认全部登记项目 loop off 后重载旧引擎，PID 44037→82350，新版已加载；既有命令软链生效，未重装或改 PATH／plist。
- T33 调研报告归档合并 `ef3d67a`，收尾 `2b5ccc4`，交接 `9f5bc08`。研究者完成记录、主控审查与 CCNotify 后续清理记录均保留；公开历史确认 T33 已由用户放行。
- T35 只在 README.zh-CN.md 补三步使用示例：实现 `c2cfb11`、合并 `bd056df`、收尾 `a4cfa1a`，已推送。仅人工核对文案及 diff 检查；`drover done T35` 核对通过并停在待放行。用户明确反馈「我看到了消息了，而且也放行了」，公开历史确认已放行。
- 本次交接前唯一未跟踪文件是本会话起草的 `docs/任务/T33-Drover通知最小改动说明.md`，不是用户另行修改。现补历史说明并随交接归档提交；以正式设计／接口文档为准。

## 当前队列与环境

- 最新公开队列：current=T36（doing）、awaiting=null、loop=false、gate=true。T36 已进入串行实施，没有调用 done/go/next；T33、T35 均已完成并放行。
- 公开通知偏好：system_enabled=false、revision=1，即 In saddle；是用户自行设置，不要替用户切回。
- 待办顺序：T31 评估并行派发多个不同任务 → T29 排查偶发 workflow 测试失败 → T28 评估 ctl 单实例 256 次修改上限 → T32 统一接入本地与远程 corral agents → T34 saddle 沙箱与定时任务。
- saddle 仅剩主仓库 worktree，相关实现／研究分支及自开 agent 均已清理。公开 corral 列表只剩 saddle/main、drover/main、corral/main、globalmesh/main、owlet/main 这些用户主控。
- **不得主动关闭 drover/main**：用户明确说它不是 saddle 委派开的。不要干扰其他用户主控。

## 未解决事项与验证边界

- T29 仍未解决：任务通知主控标准测试一次为 255 passed／1 failed／3 ignored，旧 T20 窗格替换用例超时，单项核对一次通过；Clippy、通知相关检查及真实 Drover 隔离主流程通过。不能称整套全绿，也不能从旧基线失败推断新增轮询无负载回归。
- 用户已确认收紧范围：撤回 saddle 持久去重阻塞，接受会话去重／启动基线；限制和理由见 DESIGN §48。不得恢复旧的返工要求或继续统计式重跑。Drover 发送端持久去重不变。
- Hammerspoon 限流／上下文／用量系统提醒保留。CCNotify 已按单独授权移除，数据和配置备份保留在 `~/.local/state/ccnotify-removal/20260928-171636/`；细节见 T33 任务记录，不再操作。
- T36 已正式开工。委托文件 `docs/任务/T36-Drover人工完成实施委托.md`，提交 `11c106b`；corral 已向原 drover/main（a781ee31bc67）送达 confirmed。它先固定最小公开契约并派 Codex 实现，模型／强度／审查按其路由决定，实际派发信息待回报。上一任务 worktree 已全部收尾。

## 接手先读

- `AGENTS.md`
- `docs/DESIGN.md` 第 48 节：现行通知设计及已接受取舍。
- `docs/任务/任务通知-主控审查.md`：固定提交、验证局限、两边发布闭合。
- `docs/任务/任务通知-接口与集成约定.md`：共享接口边界。
- Drover 正式公开契约：`../drover/docs/通知JSON接口.md`；正式命令 `~/.local/bin/drover`。原 m35 worktree 已删除，不再引用它执行命令。
- `docs/任务/T33-系统通知收敛调研.md` 与 `T33-系统通知调研报告.md`：历史调研与归档记录，不将旧状态当作当前状态。

## 下一步

等待 Drover 主控 T36 回报，先分清“已派发”和“实现及审查完成”。若只是派发进展，记录并等后续交付；只有固定 SHA、公开契约及审查通过后才安排 saddle 消费方实现。产品流程已确认，见 DESIGN §49，不再让用户重复下发。两边隔离联调通过前不分别合并发布或清开发／审查 worktree，不操作真实队列，不关闭 drover/main。实现预算保持简短，不扩成任务类型、分支归属或历史迁移系统。
