# 交接

更新：2026-09-28。T25 设计与理由见 `docs/DESIGN.md` 第 45 节；证据见 `docs/任务/T25-重开后恢复工作布局.md`、`T25-主控审查.md` 和 `T25-独立审查.md`。

## 当前状态

T25 重开后恢复工作布局已实现，主控与独立审查通过，合并并本机构建发布。实现 `5972ae0`，合并 `9245209`，收尾空提交 `4ca2bc4`。本交接随最终文档提交推送 origin/main；后续以实时 Git 状态为准。

- 布局变化及正常退出时保存 tab、分屏方向与比例、活动位置和窗格内容身份。agent 退出保留名称、目录和占位。
- 下次启动接回仍在运行且可接入的原实例；其他 agent 保留占位，可 Create new agent 或 Choose existing agent。创建只预填原名称／目录，当前默认命令，确认后执行。
- 普通终端恢复占位，Open terminal 手动在原启动目录打开新 shell。历史、旧命令和已结束对话不恢复。
- 默认文件 `~/.local/state/saddle/layout.json`，绝对 XDG_STATE_HOME 可覆盖。缺失文件正常打开默认布局；损坏、不支持版本或不可读时提示并保护原文件，本次禁止覆盖；保存失败提示但不阻止使用。
- 本次未重启正在运行的用户 saddle。下次启动使用新版本；旧版本当前内存中的布局不会因二进制更新而获得保存功能。

## 验证与发布

- 主控一次全套：218 passed／0 failed／2 ignored；Clippy、fmt、diff 检查通过。独立审查定向 15 passed，结论可以合并，必须改 0、建议改 0，三项取舍全部同意。
- 实现者首轮 216 passed／2 failed／2 ignored 及两项定向复跑通过记录保留；本次通过不代表 T29 偶发问题已修复。
- 合并后核对 src、tests、Cargo.toml、Cargo.lock 与审查提交完全一致；没有重复无关全套测试。
- main 的 `cargo build --release` 通过，使用共享 CARGO_TARGET_DIR。共享 `../saddle-worktrees/.target/release/saddle`、仓库 `target/release/saddle`、默认 `~/.local/bin/saddle` 三入口 SHA-256 一致：`06c67410c10ab91bbdf2e5f3c6455e085326ef2a826a75c405b089ec3485b315`。默认入口仍链接共享 release，`--help` 已含 Layout 说明。
- 只用假 CLI、合成数据和临时状态目录验证，未读取或覆盖用户真实布局，未操作用户真实 agent／saddle socket。未验证断电和多个 saddle 同时写同一布局文件。

## 队列与开发环境

- T25 已完成并由用户放行；用户已手动下放 T26，最新公开队列 current=T26（doing）、awaiting=null，loop=false、gate=true。本轮未调用 go/next。
- Pending 顺序：T29 偶发测试失败 → T28 ctl 上限。逐项讨论后由用户手动下放，不自动派发下一件。
- T25 实现与独立审查 worktree、实现分支已清理；两个 agent 均在 idle、attached=0、工作区干净且提交已合并后，随工作目录删除一并关闭：`saddle/dev-t25-layout-restore-1`（2410a2971081）、`saddle/dev-t25-review-1`（78197c46589e）。记录保留在 main，迟到提醒查到 not_found 即忽略。
- T25 收尾时 saddle 仅保留主控 `saddle/main`，cwd 为主仓库；本轮新建的 T26 实现者见下文。corral/main、drover/main、globalmesh/main、owlet/main 原有用户 agent 保留在各自工作目录。

## 仍需注意与下一步

T26 用户手动下放后回复「按照你的建议来」，主控已定稿 `docs/任务/T26-任务与交付结果跳转.md`、DESIGN 第 46 节：只改 saddle，Tasks 增加 Links，明确引用加一层任务书，项目内文本／Git 在弹窗内只读查看，agent 校验原实例。路由重／交叉审查要／碰要害，交给 Codex gpt-6-astra / xhigh。队列当前正文保留派发时的旧审查稿；它已是 current，公开 edit 只支持 pending，因此不改上游历史，实施以最新任务书、设计和用户本轮确认为准。

实现者 `saddle/dev-t26-task-links-1`（instance `96c48b700af7`，role=implementer），分支 `t26-task-links`，worktree `../saddle-worktrees/t26-task-links`，基线 `a3bb249`，已提交 `2cd9a97`，idle、工作区干净。主控首轮审查未发现阻塞项，标准测试 229 passed／0 failed／2 ignored，Clippy、fmt、diff 通过，详见 `docs/任务/T26-主控审查.md`。尚未合并发布。

独立审查者 `saddle/dev-t26-review-1`（instance `7e6f0a94ce23`，Codex gpt-6-astra / xhigh，role=reviewer），detached worktree `../saddle-worktrees/review-t26-task-links` 固定 `2cd9a97`。首轮结论改完再合并：必须改 1、建议改 0、可以不改 3；定向 11 项通过，另一个仓库外探针复现了阻塞问题。主控认可：shell 在 Links 替换确认期间自行退出后，确认被拒绝却未清除 checking_agent，导致所有 Links 无法继续打开。

已在 `docs/任务/T26-主控审查.md` 写明本轮唯一返工项，交回原实现者；审查者保持 idle。收到返工完成后核对定向修复，先把 review worktree 更新到新 SHA，再请同一审查者复核。当前尚未合并发布，不自动推进下一任务；独立复核轮次尚为 0。

- T29 的 picker／close confirmation 偶发 workflow 问题未定位；一次通过不代表修复。
- 共用 target 跨 checkout 曾复用旧二进制；后续检查需核实构建对应当前源码。
- T28 的 ctl 单实例 256 次修改上限仍在。本轮未改 corral／drover／corral-dispatch 或全局技能。
- T24 用户已看到历史功能，但系统剪贴板真实 Copy 写入仍未获得明确现场验证反馈。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。
