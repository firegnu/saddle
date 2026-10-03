# Corral 通用升级部署记录

2026-10-03。用户在确认“构建新包、隔离检查、更新入口，然后由用户退出旧主控并重开”的顺序后要求继续，需要用户时告知。本轮部署已执行；没有对旧主控执行升级、停止、resume 或重建。

## 安装与验证

- 干净源码 `711ab18e4684f283de28216aab49cb3c9cb75a2a`，已包含恢复身份修正 `9648ff0`。使用共享 CARGO_TARGET_DIR、原生 `aarch64-apple-darwin` 和 `scripts/package.sh` 构建新不可变包：`/Users/firegnu/.local/share/saddle/versions/711ab18`。
- BUILD.txt 标记 clean，saddle/corral/Drover/Diff 四程序 SHA256 校验通过。没有复用不含返工的 `816358a` 临时包，没有覆盖旧包。
- 从该固定 release 包运行已有隔离 product 检查 `tui_exit_and_reopen_preserve_agent_identity_and_runtime -- --ignored --exact`：1 passed。使用临时 HOME/CORRAL_HOME 和合成 cat，不是实际用户界面/真实 coding agent 验收。
- 另在临时 HOME/CORRAL_HOME 用新包公开 start/status/stop 核对合成实例：exe 为 `711ab18/bin/corral`，custody=owner，capabilities 的 upgrade/recover/snapshot 均为 1；测试实例已公开 stop 清理。未调用真实 `upgrade --all`。

## 备份与实际切换

- 私有备份目录（0700）：`/Users/firegnu/.local/share/saddle/backups/corral-upgrade-711ab18-20261003-191430`。保存 config.toml、plugins.toml、两处 Corral SKILL.md、旧链接目标、文件哈希与新包 BUILD.txt；不复制对话、任务或 Corral 私有状态。
- 临时配置中预览新插件路径，公开 plugin status 显示 Drover/Diff enabled、manifest_readable，其他插件及开关保留。
- 持 `plugins.lock`，逐项比较当前文件/链接与备份基线后，仅原子更新注册中的 Drover/Diff directory；分别原子替换 `~/.local/bin/saddle`、`~/.local/bin/corral` 到新包。config.toml 字节不变。两处链接的替换不宣称跨文件事务。
- 新入口公开 plugin status 再次确认两配套插件路径和状态，原有 demo 插件、Dispatch 设置与资源状态保留。
- 新版 Corral 操作说明通过 `corral install-skills --yes` 同步两处自有技能；变更仅增加持久提醒 request_id/unknown 规则及升级授权/结果边界。随后 dry-run 两处 same、written=false。旧技能已备份，Dispatch 技能未变。
- 旧版本目录保留。回滚可使用备份的旧链接目标、注册和 Corral 技能；需要先确认没有新实例/用户改动，不自动回滚或删除被引用版本。

## 当前主控与待用户操作

新 CLI 只读查询旧 `saddle/main` 成功，仍为 instance `19185812ef0e`、agent PID `15740`、pen PID `15167`；没有升级 capabilities，不能声称旧核心已换新。运行中的 Saddle 未由主控重启；新插件注册可读不代表旧宿主/插件进程已换映像。

Claude 设计评估会话已按用户上一轮明确指令关闭。现在需要用户退出当前旧主控，再关闭并重开 Saddle，创建新的主控；接续者先读 HANDOFF.md，核对新宿主/插件实际映像及新 agent status 的 exe/capabilities。不要恢复旧会话并把它视为已升级，不自动迁移旧 Tasks runs。

未执行真实 Claude/Codex/pi/omp 全路径兼容冒烟、现存旧 agent 无缝首次迁移、真实任务/遥测操作。退出旧主控后新建是用户选择的首次替换，不是无缝迁移。
