# Saddle 交接

更新：2026-10-05。主控 `saddle/main`，分支 `main`。本次按用户「你自己改，不要委派了」直接实施、审查和收尾；没有启动其他 agent。实现提交 ff3c441 已合入 main，本次 `ranch-dispatch-extraction` worktree/分支已清理，空收尾提交 86909c1。

## 当前状态

- 被删功能（Dispatch 插件、遥测、Drover 及插件系统）的代码可从带说明标签 `before-cut`（`c21674a`，已推送 origin）取回，数据留在磁盘上。
- 用户最终决定：Saddle 只保留 Agents 面板、终端、设置、布局恢复、Diagnostics、Updates 和非插件 ctl。遥测、Drover、Dispatch 插件及整个插件宿主/SDK/协议已删除，包内只剩 `bin/saddle` 与 `BUILD.txt`。依据是 DESIGN 末尾“只保留终端前端”，覆盖同日早先“不动遥测/Drover/SDK”的安排。
- Corral 与 Dispatch 由 ranch（`../ranch`，paddock 主控兼管）维护。Saddle 默认从 PATH 使用 Corral；显式配置路径仍有效。派发走 `ranch dispatch route`，不记遥测。Saddle 不安装 Corral 或 corral-dispatch 技能。
- 全量首轮 289 passed / 1 failed；缩窗测试增加完整新帧同步后，工作区 66 项复测全部通过；Attention/宿主单元回归 29 项、Clippy 和格式检查通过。保留功能的回归涵盖 Agents、终端输入、设置、布局恢复和 ctl open/inspect/close；只用临时目录与假 Corral。详细记录见 `docs/任务/Saddle终端前端精简-2026-10-05.md`。
- 发布从干净 main 构建；当前安装路径以 `~/.local/bin/saddle` 链接及包内 `BUILD.txt` 为准。部署回执与归属记录备份在 `~/.local/share/saddle/backups/ranch-frontend-*/`。部署只切换 Saddle 入口并解除两处 corral-dispatch 归属；保留技能文件与其他条目，不更改 Corral 链接，不操作现有 agent。
- 旧 `~/.local/state/saddle/telemetry/`、`~/.drover`、插件注册与资源记录留在磁盘上，新程序不再读取。除已授权的 corral-dispatch 归属释放外，不读或删除这些数据。Saddle 的旧版本目录全部保留，现有会话仍可能使用其中的 Corral。
- Settings 现在为 F1–F5，Updates 是 F5；旧插件窗格恢复为空位，其余布局及原 agent 身份保留。旧 `[queue]` 配置忽略，保存设置保留其原文。Updates 的 agent 提示比较 Corral 运行时，不能通过反复重启 Saddle 来升级旧 agent。

## 下一步

用户正常退出并重开 Saddle，加载新版；本次不强制重启。之后 paddock 主控在用户在场时用 ranch 安装技能并核对 Saddle 与 ranch 配合。不自动升级现有 agent，也不自动发送消息给 paddock。

Saddle 定位为保底前端，不增加新功能，只保证与 ranch 运行时对得上。旧 Tasks 队列已退出操作链，不继续处理历史 Pending。

## 独立项目与保留现场

- cairn、paddock、ranch 由各自主控负责，不在 Saddle 恢复其实现，不干扰其 agent；当前状态以各自仓库文档为准。
- T49/T76 的旧研究与 GPUI 原型不在本次清理范围，保留 `t49-handoff-study`、`t76-gpui-research`、`t76-gpui-prototype`，不合并进 Saddle。cairn 引用的原始提交不改写。
- 其他历史 worktree（包括 `corral-live-upgrade-research`、`review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow`）不擅自清理。本次只清理自己的分支/worktree。
- Cargo 继续共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`；旧遥测/插件/任务设计文档只是历史，不作为当前操作入口。
