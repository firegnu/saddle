# 会话交接

更新：2026-09-30。当前 main，T52 修复已合并、清理并编译 release；本次交接提交前 HEAD 为 f7aae7f。本交接随本次收尾提交并推送，最终推送结果见 Dispatch f4f71f175d9b454084ba2b3060b03777 的收尾 note。

## 当前状态

- T52 用户已明确授权“排查并委派修复，这是block级别的”，覆盖任务正文旧的“仅登记待办”。实现 3458e1e，合并 432cc12，主控审查 2490325，收尾 f7aae7f。
- 公开 drover done T52 已核对通过，退出码 8 表示等待放行。最新 list：current=null、awaiting=T52、loop=false、gate=true。不要重复 done；等用户验收与放行，不自动 go/next，不开启 Loop。
- release 已编译成功，~/.local/bin/saddle 仍指向共享 target/release/saddle。没有重启正在运行的 Saddle；用户需要重新打开 Saddle 使修复生效，再确认 Claude Code/Codex 的 Shift+Enter。
- 真实客户端输入框尚未验收，不能宣称已解决用户现场问题。实现、测试证据与边界见 docs/任务/T52-ShiftEnter换行.md，设计说明见 docs/DESIGN.md 第 5 节。
- t52-shift-enter worktree/分支已删除，其中自开的 saddle/dev-t52-shift-enter-1 已随工作目录删除一并关闭。仅保留 main 和历史 t38-dispatch-study（c15bc4d，未合入调研，不删除、不合并）。
- 最新 corral ls 只有 corral/main、dispatchlog/main、drover/main、saddle/main；它们都是用户正在使用的主控，不代操作或关闭。

## 验证与待确认

- T52 自动复现通过真实 Saddle 二进制、Crossterm、Viewer、内层 PTY 和假 CLI，外层使用终端协议模型。修复前收到 CR、修复后保留 Shift+Enter 字节，退出模式恢复与相关输入回归通过。
- 主控标准检查一次：cargo test --all-targets 294 passed、0 failed、3 ignored；cargo clippy --all-targets -- -D warnings 通过；diff 检查通过；main release 构建通过。
- 实现者标准测试为 293 passed、1 failed、3 ignored，picker 等待 SHELL READY 超时后单独复跑通过。保留原失败；主控通过不等于 T29 根因已修复。
- 下一步先让用户重开 Saddle 验证两个客户端。如果仍失败，继续 T52 定位用户所用外层终端的真实输入链路，不把协议模型通过当现场验收。

## Pending 与后续授权

最新顺序：T29 → T28 → T32 → T34 → T48 → T49 → T50 → T51 → T53，均不自动实施或派发。

- T29：偶发 picker／关闭确认测试失败，未修复。
- T28：评估 ctl 单实例 256 次修改上限。
- T32：本地与远程 agents 统一接入；T34：沙箱与定时任务；均待细化范围。
- T48：分离当前任务完成检查与自动派发，避免完全依赖主控登记完成。技术方案、上游边界未定，等用户放行。完成登记、用户放行、派发下一项是三个动作；Loop off 不是此前通知缺失的完整根因，上次是遗漏登记导致没有 Awaiting。详细历史调查保留于 07f7dd1 的 HANDOFF.md。
- T49：讨论跨会话交接与新 session 接续；T50：讨论跨项目总主控与通讯软件入口。只登记讨论，不接账号、不启动总主控、不发消息。
- T51：创建 agent 时无法选择在当前 pane 打开，截图未收到，只登记。
- T53：精调委派 agent 的模型选择，用户认为 effort 应该没问题；未调整模型策略。
- T47 已在历史中完成并放行，勿补调 done。Attention 真实等待输入曾由用户确认，无需因完成登记遗漏重做该功能。

## 接手约束

- 先读 AGENTS.md 和获授权任务，设计按 docs/DESIGN.md。仅通过公开 Corral/Drover CLI，不读内部文件、不改上游仓库、不干扰用户 agent。
- 主控按 corral-dispatch 与 dispatch-log/USAGE.md 路由、派发和记录；验证不超预算，返工只验增量。最终 note 在实际推送结果确定后保存，不补造历史、不读私人会话日志。
- 所有 Cargo 使用共享 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。
- 不把旧交接的“不调用 done/go/next”视为永久规则；本轮在 gate=true、loop=false 下只登记 T52 完成并停在 Awaiting，未放行或派发下一项。
