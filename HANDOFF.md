# Saddle 交接

更新：2026-10-05。当前分支 `main`；本次交接前 HEAD 与 origin/main 均为 `dedf26a`，工作区干净。用户要求本次工作由主控直接完成，不委派。

## 1. 会话摘要

Saddle 已精简为 ranch 的终端前端，删除遥测、Drover 和整个插件系统，保留 Agents、终端、设置、布局、Diagnostics、Updates 与非插件 ctl。随后将顶部单项 More 菜单改为直接显示 Settings，并把设置框最大宽度从 108 列收至 88 列；已编译部署。

## 2. 完成的工作

- 插件系统精简：`ff3c441`；Settings 直接入口：`ba2144e`；对话框收窄：`f1173e7`。均已合入并推送 main，自己的实现分支/worktree 已清理，收尾提交已完成。
- 带说明标签 `before-cut` 指向 `c21674a`，已推送 origin。被删功能代码可从该标签取回，数据留在磁盘上。
- 已安装 `~/.local/share/saddle/versions/dedf26a/bin/saddle`，入口为 `~/.local/bin/saddle`。本次现场核对 PID `19435` 的程序映像也是该版本，用户已加载新版；PID 只是核对时快照。
- PATH Corral 入口仍指向 `~/.local/share/ranch/versions/df46247/bin/corral`。两处 corral-dispatch 归属已释放，技能文件保留；后续 Settings 部署只切换 Saddle 入口。20 个版本目录均保留。
- Settings 入口相关 100 项测试、Clippy 和格式检查通过。宽度调整覆盖设置/Diagnostics/Updates 48 项：首轮一项未滚动读取长回执，测试保留全部内容断言并增加 PageDown 后，Updates 13 项复测通过。正式 release 的隔离冒烟通过，涵盖直接点击 Settings、终端、布局保存和 ctl open/inspect/close，不使用真实 agent。
- 精简插件时的首次全量为 289 passed / 1 failed，修正缩窗测试同步后工作区 66 项复测通过；不要改写为首次全量全绿。Settings 小改期间误启动的全量已按用户意见停止，不计通过，不重跑。
- 最新部署与检查回执：`~/.local/share/saddle/backups/settings-direct-dedf26a-kdla7upu/`。归属释放备份：`~/.local/share/saddle/backups/ranch-frontend-a4b45c2-53kdb8bk/`。

## 3. 待完成的工作

暂无已知待完成实现工作。本次仅更新 HANDOFF 并提交、推送，不再编译部署；之后 Updates 的 Source 若领先 Installed 一个交接文档提交，无须为此重装。

paddock 主控在用户在场时安装 ranch 技能及核对配合的后续事项，尚未在本会话确认完成；不自动代做、发消息或升级现有 agent。

## 4. 关键决策与约束

- Saddle 是保底前端，不加新功能，只保持 ranch 运行时兼容。Corral 与 Dispatch 由 ranch（`../ranch`，paddock 主控兼管）维护；默认使用 PATH Corral，显式配置路径保留；派发走 `ranch dispatch route`，不记遥测。
- 不读取或删除旧遥测目录、`~/.drover`、插件登记及资源记录；用户自行处理。旧版本仍可能被会话使用，不清理。旧插件布局恢复为空位，其余布局和 agent 身份保留。
- 小 UI 改动按影响面检查，发布本身不触发全量；已通过且相关代码未变不重复检查。此规则已写入 AGENTS、README 和 UI 回归指引。
- 不操作用户现有 agent，不强制重启 Saddle。Settings 为 F1–F5，Updates 为 F5；agent 升级提示比较的是 Corral 运行时。
- 主目录留有旧插件的本地 dist/target 构建产物；已用 `.git/info/exclude` 延续原有忽略规则，保留原路径，别当作待删源码。
- cairn、paddock、ranch 独立管理；历史研究/GPUI worktree 不在清理范围。现场仍保留 `corral-live-upgrade-research`、`review-telemetry-design`、`t38-dispatch-study`、`t49-handoff-study`、`t55-notification-flow`、`t76-gpui-research`、`t76-gpui-prototype`。不恢复旧 Tasks Pending，不改写 cairn 引用的历史提交。
- Cargo 共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。

## 5. 重要文件

- `AGENTS.md`、`docs/UI回归.md`：当前操作边界和分层验证规则。
- `docs/DESIGN.md` 末尾：精简范围及 Settings 的最终决定；早期插件/遥测/任务章节只是历史。
- `docs/任务/Saddle终端前端精简-2026-10-05.md`、`docs/任务/Settings直接入口-2026-10-05.md`：实现、失败记录和复测证据。
- `README.md`、`README.zh-CN.md`：现有功能及入口；`scripts/package.sh` 只打包 Saddle 和 BUILD 记录。

## 6. 下一步建议

等待用户新指令或由用户安排 paddock/ranch 配合核对。若继续开发，先核对分支、工作区、入口链接及运行版本；不要重复部署、测试或清理历史现场。
