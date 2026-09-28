# 交接

更新：2026-09-28。当前 main；T36 两边已按用户最新决定完成合并、推送、发布及清理，部署交接完成。

## 当前状态

- T36 saddle 固定实现 `3d21879`，合并 `2beb321`，收尾 `b463f8d`。主控标准检查各一次：262 passed／0 failed／3 ignored，clippy 通过；没有追加测试。
- 用户明确「不用再安排审查了……收尾吧」，取消 saddle 独立审查并结束追加验证。隔离联调没有执行，不能记作通过。取舍、验证和裁决见 `docs/任务/T36-主控审查.md`。
- release 构建已成功，默认 `~/.local/bin/saddle` 仍链接共享 release 文件。用户需自行重启 saddle 使用新版；未替用户重启。
- saddle 实现 worktree／分支已删除，idle、attached=0 的 `saddle/dev-t36-manual-complete-1` 随工作目录一并关闭；没有创建审查 agent。saddle 仅剩主仓库 worktree。
- Drover 已审交付 `0e4d031` 合并为 `2b6e657`，收尾 `d3605ef`；最终 main/origin 已核对为 `be3bdbd7e0786ee72012dfade2bda5dde385a23d`。CLI 软链生效，原引擎已重载为 PID 37103；其主控报告无遗留部署项，真实状态未变，开发／审查 worktree、分支和自开 agent 已清理。正式 CLI `../drover/bin/drover`，契约 `../drover/docs/人工完成JSON接口.md`；不要使用已删除的 m36 worktree 路径。
- **不关闭 drover/main**，它是用户主控。不得干扰其他用户 agent。

## 下一步

等待用户使用反馈或新指示。T36 部署与清理已闭合，不重跑套件、另派审查或恢复取消的联调。若重复完成提醒到达，只核对本记录，勿再次合并、发布或清理。

真实队列未操作：最后公开状态为 T36 doing、loop=false、gate=true。不要调用 done/go/next 或人工完成替用户推进，后续任务由用户处理。待办仍是 T31、T29、T28、T32、T34。

## 保留事项

- 现有通知已联合发布，用户亲自验证过 T35 完成提示并放行；通知偏好 In saddle（用户设置），不代改。
- T29 偶发测试问题未修复；本次全绿不代表已解决。此前通知功能会话去重的已接受限制保持不变，见 DESIGN §48。
- Hammerspoon 提醒保留；CCNotify 已授权移除，备份 `~/.local/state/ccnotify-removal/20260928-171636/`，不再操作。
- T36 设计在 DESIGN §49。人工完成为新增入口，成功后等待放行，旧自动检查和普通 go 语义保留。记录只在 Run details 显示，入口无新键盘快捷键。
