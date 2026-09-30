# Saddle 与 Drover 简化流程联合发布

2026-09-30。用户在核对发布步骤后明确要求“你能帮我执行这堆东西吗？”，授权执行本次联合切换、推送及正常收尾；真实 T57 的提交/接受、插件设计仍不在范围。

## 已审交付

Saddle 实现 5fbe04f，隔离联调 fb302d6；Drover 兼容修复 cb67fe8。两端代码审查及主控独立联合主路径检查已通过；既有 picker 用例失败在 main 基线可复现，标准检查结果 292/1/3 如实保留，Clippy 通过。详细证据见相应主控审查及隔离联调记录。

## 本次执行

- 正常退出旧 Saddle，仅断开显示，不停止 Corral 主控；退出前只有 agent pane，没有普通 shell。公开 ctl instances 确认旧实例退出。
- bootout `gui/$(id -u)/dev.drover.loop`，确认未加载且旧 PID 46666 已消失；KeepAlive 不再拉起旧推进逻辑。
- 备份目录：`/Users/firegnu/Library/Application Support/saddle-release-backups/20260930-135645`。包括 ~/.drover 全量副本、旧 Saddle release、公开状态与主控文档。目录权限 0700；Drover 主控另核对项目配置及数据备份。
- Saddle main 合并 2a6862c，主控审查记录 c97270a，worktree/分支正常删除后空提交收尾 c939fd1；住在其中的自开 saddle/dev-drover-schema2-1 已关闭。T55/T38 worktree 保留，Drover 主控保留。
- main 执行 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo build --release --locked` 退出 0，6.31 秒。实际入口 ~/.local/bin/saddle 指向共享 target 的 release；SHA-256 e63d8e5ab76012e1b67e34b9232636a5ee3067972ea2382771f147a1d6d9dd76。日志 /tmp/saddle-schema2-release-build.log。
- 已安装 Drover list 返回 schema 2；与备份前公开结果比对 current/awaiting、Pending 的 ID/正文/顺序一致。T57 Running，awaiting=null，T55 Pending。未调用真实 done/go/dispatch/return。

## 切换结果

- Drover 主控本人完成：合并 b02f142、空提交收尾 16ef806、发布 e0d8118，已推送且远端一致。三个登记项目任务事实一致；32 个校验路径仅服务 plist 改变，其他31个不变，备份与核验结果在备份目录 drover-phase1/ 下。任务 worktree/分支已清理，无关 T27 文档保留。
- 用户明确说“Saddle我已经重启了”。公开 ctl 显示新实例 5d8f7242dcc4636b；PID 63344，14:00:43 启动，lsof 确认实际执行共享 target 的新版 release。主控没有声称取得新的桌面截图；旧界面退出后计算机工具连接失败，用户完成了重启。
- 主控 bootstrap 原 LaunchAgent 链接；新 PID 83246，launchctl 为 running，实际进程参数 `~/.local/bin/drover notifications watch`，未发生退出。旧 PID 46666 已不存在。服务标签保留 dev.drover.loop，仅是部署名称，不再自动推进任务。
- 重启后公开状态：T57 Running、awaiting=null、T55 Pending；待办 ID/正文/顺序及45条历史任务既有状态/时间字段与切换前一致。没有执行真实任务写操作。
- 既有测试失败仍未修，插件设计未启动。新事件不能直接交给旧二进制继续操作；回退须保留新日志并协调两侧版本。

## 最后收尾

本发布记录与 HANDOFF 随本次 Saddle 推送保存；最终提交、推送结果及两端状态见 Dispatch 05458465051541c4a9c2cc908dc9fe3e 的收尾记录。
