# 交接

更新：2026-09-28。T30 Settings 已按用户确认派发，见下文；当前发布仍为 T26。T26 设计与理由见 `docs/DESIGN.md` 第 46 节；实施、审查及发布证据见 `docs/任务/T26-任务与交付结果跳转.md`、`T26-主控审查.md`、`T26-独立审查.md`。

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

- T26 已完成并由用户放行，用户已手动下放 T30；最新公开状态 current=T30(doing)、awaiting=null，loop=false、gate=true。本轮未调用 go／next。
- 最新公开 Pending 顺序：T31 评估并行派发多个不同任务 → T29 偶发测试失败 → T28 ctl 上限 → T32 统一接入本地与远程 corral agents。后续任务仍待讨论细化，不自动派发。
- 队列 T26 正文保留派发时旧审查稿，历史不改；后续已获用户「按照你的建议来」确认，最新定稿与完成记录以任务书和 DESIGN 为准。
- T26 实现与独立审查 worktree、实现分支已删除。两个自建 agent 在 idle、attached=0、工作区干净、提交已合并后，随工作目录删除一并关闭：`saddle/dev-t26-task-links-1`（96c48b700af7）、`saddle/dev-t26-review-1`（7e6f0a94ce23）。迟到提醒查到 not_found 即忽略。
- saddle 主控 `saddle/main` 在主仓库；T30 实现者见下文。原有 corral/main、drover/main、globalmesh/main、owlet/main 保留在各自工作目录。

## 仍需注意与下一步

T30 已完成共同设计并正式开始实施。任务书 `docs/任务/T30-Settings配置入口.md`，设计 `docs/DESIGN.md` 第 47 节，定稿提交／分支基线 `99c7f34`。用户先要求只改任务书，主控当时未改队列；手动下放带出旧占位稿后，用户再次确认以最新任务书正式委派，故以定稿为准，不修改上游历史。

- 实现者 `saddle/dev-t30-settings-1`（instance `acef3e8b5939`），Claude Code 常规 `opus[1m]` / `high`，role=implementer。
- 分支 `t30-settings`，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/t30-settings`，实现提交 `857f154`，回复 DONE、工作区干净。此前 corral 报告 working 而输出已回输入提示符，主控按用户指示在固定提交的独立目录完成审查；迟到提醒到达后已核实实现者 idle、attached=0，最新回复原地待命，提交不变。状态不同步的具体原因未诊断。
- route.py 三项 verdict 均为 null；主控判断常规／交叉审查要／碰要害：用户配置持久化和外部修改冲突需要独立审查。实现结束先主控审查，再安排独立 Codex 审查，通过后合并发布收尾。
- 主控固定 `857f154` 审查通过：242 passed／0 failed／2 ignored，Clippy、fmt、diff 通过，详见 `docs/任务/T30-主控审查.md`。已建 detached `../saddle-worktrees/review-t30-settings`，固定 `857f154`；独立审查者 `saddle/dev-t30-review-1`（instance `3bbf22741dd6`，Codex `gpt-6-astra` / `xhigh`，role=reviewer）按 `T30-独立审查.md` 工作，待结论后裁决；未合并发布。实现者迟到提醒先核对状态／SHA，不重复派发或审查。

- T29 的 picker／close confirmation 偶发 workflow 问题未定位；一次通过不代表修复。
- 共用 target 跨 checkout 曾复用旧二进制；后续检查需核实构建对应当前源码。
- T28 的 ctl 单实例 256 次修改上限仍在。
- T24 用户已看到历史功能，但系统剪贴板真实 Copy 写入仍未获得明确现场验证反馈。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。
