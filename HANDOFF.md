# 交接

更新：2026-09-28。设计与理由见 `docs/DESIGN.md` 第 44 节；本次证据见 `docs/任务/T24-终端历史查看搜索与复制.md` 和 `docs/任务/T24-主控审查.md`。

## 当前状态

T24 历史查看、搜索与复制已实现、返工复核通过、本地合并并构建发布。功能 `e864cf7`，底栏修复 `d3f14f7`，合并 `8ba2c05`，复核记录 `f126f53`，收尾空提交 `821fc56`。本交接随最终文档提交推送 origin/main；后续以实时 Git 状态为准。

- 当前窗格底栏 History 进入历史模式；滚轮、↑↓、PgUp／PgDn 回看，已上翻时新输出不拉回底部。
- `/` 输入关键词，Enter 查找，n 向旧／N 向新循环匹配；匹配作为选区。鼠标拖选后 Copy 写系统剪贴板。
- 搜索中 Esc 返回历史，历史中 Esc／Live Esc 返回实时终端；历史输入不透传到 agent。容量仍为 10,000 行。
- 正常窄窗与分屏保留 History／Copy，缩短其他底栏控件并留出历史状态空间。具体取舍见 DESIGN。
- T22／T27 已放行；T24 用户已看到功能，公开队列已无待放行。用户已下放 TASK T25，指定 Codex 实现；方案见 DESIGN 第 45 节与 `docs/任务/T25-重开后恢复工作布局.md`。路由为重／交叉审查要／碰要害，使用 gpt-6-astra / xhigh。

## 验证与发布

- 首轮主控标准检查：200 passed、1 failed、2 ignored。失败为旧 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases`，workflow.rs:2973；不宣称 T29 已修复。实现者首轮报告全套通过，两份记录均保留。
- 返工主控定向复核：原失败探针 2 passed、terminals 13 passed、历史输入隔离 workflow 1 passed；Clippy 全 targets、fmt、diff 通过。按预算未重复全套。
- 系统剪贴板真实写入尚未现场验证；自动测试未覆盖用户剪贴板。程序已覆盖的内容或备用屏幕自身未保留的历史不补造。
- main release 构建通过；共享 target/release/saddle、仓库 target/release/saddle 和默认 ~/.local/bin/saddle 三入口 SHA-256：`ee4b8e96a019819c216fdd7ae331e2df9a2ee731045959344b09fe2cb6d46cf1`。默认入口仍链接共享 release，`--help` 已含 History。未重启用户现场；下次启动使用新版本。

## 队列与开发环境

- T24 已完成并放行；最新公开队列 current=T25（doing）、awaiting=null，loop=false、gate=true。未调用 go/next。
- Pending 顺序：T26 任务产物跳转 → T29 偶发测试失败 → T28 ctl 上限。逐项讨论后再实施，不自动派发下一件。
- T24 worktree／分支／实现者及主控临时探针已清理；用户 agent 未动，迟到提醒查到 not_found 即忽略。
- T25 实现者 `saddle/dev-t25-layout-restore-1`（instance `2410a2971081`，Codex gpt-6-astra / xhigh，role=implementer），分支 `t25-layout-restore`，worktree `../saddle-worktrees/t25-layout-restore`，基线 `8ee564d`。上一轮中断时尚未创建 agent，本次无重复派发。
- T25 实现已提交 `5972ae0`，实现者 idle、工作区干净；主控首轮审查未发现阻塞项，标准测试 218 passed／0 failed／2 ignored，Clippy、fmt、diff 通过，详见 `docs/任务/T25-主控审查.md`。尚未合并发布。
- 独立审查者 `saddle/dev-t25-review-1`（instance `78197c46589e`，Codex gpt-6-astra / xhigh，role=reviewer），detached worktree `../saddle-worktrees/review-t25-layout-restore` 固定 `5972ae0`，按主仓库 `docs/任务/T25-独立审查.md` 只读审查并在该文件追加意见。

## 仍需注意

- T29 的 picker／close confirmation 偶发 workflow 问题未定位；一次通过不代表修复。
- 共用 target 在跨 checkout／临时副本复用时曾读到旧二进制；运行检查时核实构建对应当前源码。主控本次探针使用独立包名。
- ctl 单实例 256 次修改上限仍在，已单列 T28。本轮未改 corral／drover／corral-dispatch 或全局 skill。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。

## 下一步

等待 T25 独立审查提醒，读取状态、回复和审查文件；逐条判断意见，有阻塞交回原实现者，否则合并、发布和收尾。其余待办继续逐项讨论，不自动推进队列；T24 实际系统剪贴板写入仍未获得明确现场验证反馈。
