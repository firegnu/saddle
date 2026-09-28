# 交接

更新：2026-09-28。T26 设计与理由见 `docs/DESIGN.md` 第 46 节；实施、审查及发布证据见 `docs/任务/T26-任务与交付结果跳转.md`、`T26-主控审查.md`、`T26-独立审查.md`。

## 当前状态

T26 任务与交付结果跳转已通过主控和独立审查，合并并本机构建发布。实现 `7e2df89`，合并 `8eb3b66`，收尾空提交 `c48af1d`。本交接随最终文档提交推送 origin/main；后续以实时 Git 状态为准。

- Tasks 选中任务后的详情区新增 Links，按 Files／Commits／Agents 查看明确关联，显示来源、缺失和失败。
- 读取任务正文及明确任务书的一层引用，项目内普通 UTF-8 文本和 Git 提交／记录区间在 Tasks 内只读查看；Back／Esc 返回原 Links。agent 仅按明确名称和原实例接入或定位，不猜关联、不自动创建。
- 首轮独立审查发现 shell 在替换确认期间退出会使 Links 卡在 Checking。R1 已修复，确认失效后释放当前请求并提示重试；同一独立审查者复核可以合并，剩余必须改 0、建议改 0，三项取舍均同意。
- 只改 saddle；未改 corral／drover／corral-dispatch 或全局技能。未重启当前用户 saddle，下次启动使用新版本。

## 验证与发布

- 主控首轮标准测试 229 passed／0 failed／2 ignored，Clippy、fmt、diff 通过；R1 主控 Links 5 项和 shell 替换 1 项通过，受影响 Clippy、fmt、diff 通过。独立复核 2 项通过。未重复无关全套，不宣称 T29 偶发问题已修复。
- 合并后 src、tests、Cargo.toml、Cargo.lock 与审查提交完全一致。
- main 的共享 target `cargo build --release` 通过。共享 `../saddle-worktrees/.target/release/saddle`、仓库 `target/release/saddle`、默认 `~/.local/bin/saddle` 三入口 SHA-256 一致：`20c0d53803dd131d245c993673ecf5e6f740db844072e1d18ce096feb742c6ea`。默认入口仍链接共享 release，`--help` 成功。
- 测试只用假 CLI、合成 shell／文件／Git 和临时 HOME／状态／runtime，未访问用户真实布局、agent 或 saddle socket。独立审查的未验证边界保留在审查文件，不宣称覆盖全部并发时序或真实现场。

## 队列与开发环境

- `drover done T26` 核对通过，返回 8 等用户放行。公开状态 current=null、awaiting=T26(done)，loop=false、gate=true；没有调用 go／next。
- Pending 顺序：T30 Settings 配置入口 → T29 偶发测试失败 → T28 ctl 上限 → T31 评估并行派发多个不同任务。T30／T31 仅记录需求，待讨论细化；不自动设计或派发下一件。
- 队列 T26 正文保留派发时旧审查稿，历史不改；后续已获用户「按照你的建议来」确认，最新定稿与完成记录以任务书和 DESIGN 为准。
- T26 实现与独立审查 worktree、实现分支已删除。两个自建 agent 在 idle、attached=0、工作区干净、提交已合并后，随工作目录删除一并关闭：`saddle/dev-t26-task-links-1`（96c48b700af7）、`saddle/dev-t26-review-1`（7e6f0a94ce23）。迟到提醒查到 not_found 即忽略。
- saddle 仅保留主控 `saddle/main`，cwd 为主仓库。原有 corral/main、drover/main、globalmesh/main、owlet/main 保留在各自工作目录。

## 仍需注意与下一步

等用户体验 T26 并放行，再逐项讨论后续任务。入口为 Tasks → 选任务 → Links。

- T29 的 picker／close confirmation 偶发 workflow 问题未定位；一次通过不代表修复。
- 共用 target 跨 checkout 曾复用旧二进制；后续检查需核实构建对应当前源码。
- T28 的 ctl 单实例 256 次修改上限仍在。
- T24 用户已看到历史功能，但系统剪贴板真实 Copy 写入仍未获得明确现场验证反馈。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。
