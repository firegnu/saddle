# 交接

更新：2026-09-28。T30 设计与理由见 `docs/DESIGN.md` 第 47 节；实施、审查和发布证据见 `docs/任务/T30-Settings配置入口.md`、`T30-主控审查.md`、`T30-独立审查.md`。

## 当前状态

T30 Settings 配置入口已通过主控及独立复核，合并并本机构建发布。实现 `85ead03`，合并 `5b88932`，收尾空提交 `27e18d2`。本交接随最终文档提交推送 origin/main；后续以实时 Git 状态为准。

- Settings 固定在 Agents 顶部第二行右侧，与 Attention 同行，窄栏另起一行；点击或在 Agents 按逗号打开，关闭回原焦点。
- General／Colors／Advanced 编辑现有 config.toml；提供草稿、单项恢复默认、颜色局部预览、Save／Cancel。颜色及侧栏宽度保存后立即生效，其余项标 Restart required，下次启动生效。
- 保留注释和未编辑内容，外部修改在 Save 时检测，冲突提供 Keep／Discard／Back；失败保留草稿。Keep 重载失败后重试仍保留草稿，无法解析目标的符号链接拒绝保存而不覆盖链接。
- 局部快捷键 F1–F3 切页、Tab／↑↓ 选字段、Ctrl-U 清空、Ctrl-D 默认、Ctrl-S 保存、Esc 取消。设置打开时新 ctl 修改请求返回 busy，Inspect／Request 等查询正常。
- 只改 saddle；未改 corral／drover／corral-dispatch 或全局技能。未重启用户当前 saddle，下次启动使用新版本。

## 验证与发布

- 主控首轮标准测试 242 passed／0 failed／2 ignored，Clippy、fmt、diff 通过。返工主控 Settings 12 项和 app 1 项通过，受影响 Clippy、fmt、diff 通过；独立复核 Settings 12 项通过。未重复无关全套，不宣称 T29 偶发问题已修复。
- 首轮独立审查 R1 草稿丢失、R2 悬空链接被覆盖均修复并关闭；最终必须改 0，原非阻塞建议 S1 1，新增相关问题 0。首轮取舍全部认可。
- 合并后 src、tests、Cargo.toml、Cargo.lock 与审查提交完全一致。
- main 共享 target `cargo build --release` 通过。共享 `../saddle-worktrees/.target/release/saddle`、仓库 `target/release/saddle`、默认 `~/.local/bin/saddle` 三入口 SHA-256 一致：`61a169b9fb6db2faab8e35d999777c9053d6c2e2f8822bb8151d6edb7851bd7c`。默认入口仍链接共享 release，`--help` 已核对 Settings。
- 测试只用临时配置、合成文件／链接、假 CLI 及隔离状态和 runtime；未访问用户真实配置、布局、agent 或 saddle socket。未验证断电、ACL／owner／xattr、网络文件系统及最终替换时同步竞写，不作额外保证。

## 队列与开发环境

- `drover done T30` 核对通过，退出 8 等用户放行。公开状态 current=null、awaiting=T30(done)，loop=false、gate=true；没有调用 go／next。
- Pending 顺序：T31 评估并行派发多个不同任务 → T29 偶发测试失败 → T28 ctl 上限 → T32 统一接入本地与远程 corral agents。后续需求待逐项讨论，不自动设计或派发。
- T30 队列正文保留派发时旧占位稿；手动下放后用户已明确确认按最新任务书正式实施，设计和完成记录以任务书与 DESIGN 为准，不改上游历史。
- T30 实现与独立审查 worktree、实现分支均已清理。删除前确认两 agent idle、attached=0、工作区干净且提交已合入 main，随工作目录删除一并关闭：`saddle/dev-t30-settings-1`（acef3e8b5939）、`saddle/dev-t30-review-1`（3bbf22741dd6）。迟到提醒查到 not_found 即忽略。
- saddle 仅保留主控 `saddle/main`，cwd 主仓库；原有 corral/main、drover/main、globalmesh/main、owlet/main 保留在各自目录。

## 仍需注意与下一步

等用户体验 Settings 并放行，再讨论下一项。

- T30 S1：长配置路径可能把单行保存错误的实际原因挤掉，保存失败本身仍可见且保留草稿。本轮记录为非阻塞建议，已告知用户，尚未另立任务，不自动扩大修复。
- T30 实现者曾回复 DONE、输出回到输入提示符而 corral 仍显示 working。主控按用户指示对固定提交继续审查；之后公开状态恢复 idle，清理前再次确认。原因未诊断，不据此宣称上游已修复。
- T29 picker／close confirmation 偶发 workflow 问题未定位；一次通过不代表修复。
- 共用 target 跨 checkout 曾复用旧二进制，检查须核实构建对应当前源码。
- T28 ctl 单实例 256 次修改上限仍在；T24 系统剪贴板真实 Copy 写入仍无明确现场验证反馈。
- effort 图标仅表示创建标签，不表示运行时实际推理强度。
