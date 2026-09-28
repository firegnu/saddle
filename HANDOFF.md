# 交接

更新：2026-09-28。本文件记录当前状态；设计与理由以 `docs/DESIGN.md` 为准，详细实现/验证历史在 `docs/任务/` 和 Git 记录中。

## 当前任务：T23 实现分派

用户已从队列派发 T23。范围为 Agents 搜索切换与 split 窗格 Zoom／Restore，只改 saddle。方案见 DESIGN 第 41 节，实施任务见 `docs/任务/T23-快速切换与临时放大.md`。分支 `t23-search-zoom`，worktree `../saddle-worktrees/t23-search-zoom`，按常规 Claude Code opus[1m] / high 实现、主控审查。

当前只推进 T23；其余 T22/T27/T29/T28/T24/T25/T26 保持待办。loop=false、gate=true。实现完成后按任务记录审查、合并、发布及清理，不自动重启用户现场。

## 上一轮状态：日常使用观察（以下为 2026-09-27 快照）

本轮完成终端工作区能力、Agents 3a 布局和 effort 视觉修正，随后用两个小任务验证分派与显示。用户已确认「我看到了，已经好了」及「现在agents区域显示的工整多了」。

当前工作约定是先稳定使用，暂停主动增加功能或继续调整 UI。新想法先记录，真正影响使用的 bug 再处理；反复出现的体验问题再讨论是否调整。不要因交接、旧提醒或历史待办自动开启新任务。

## 已完成的工作

- **T20 终端工作区**：新 tab、四方向 split、普通终端/已有或新建 corral agent、公开 `saddle ctl` 和配套 saddle skill 已落地。合并 `7c753ff`、收尾 `7eb65e9`。skill 已安装到 Codex 与 Claude Code。行为边界见 DESIGN 第39节。
- **Agents 3a**：分组、固定列、状态排序、折叠与窄窗适配已发布；路径完整显示/超宽左截、256色近似及 Attached 正常色已修正。合并 `2c884e7`、收尾 `5a7012b`。
- **effort 视觉修正**：去掉 working 文字前的紫色盲文动画，保留左侧蓝色圆点旋转。effort 的位置、两格字形、颜色、折叠显示及标签语义不变。合并 `dda65e7`、收尾 `94e3e85`；用户现场确认正常。没有新增模型名显示。
- **颜色测试隔离**：tests/app.rs 子进程清空 NO_COLOR，避免宿主禁色导致颜色检查假失败。合并 `93d044a`、收尾 `d1d611f`，仅改测试。
- **T21 README 对照表**：中英文 effort 三档说明已整理为表格。实现 `cddbce4`、合并 `90fda11`、收尾 `2b012cf`；公开 `drover done T21` 核对通过。最新公开查询已为 `done / history`，不再待放行。

以上均已合并推送、清理开发环境并更新记录。本次更新前 main 与 origin/main 同为 `a6536d0`，工作区干净。

## 运行与队列状态

- 当前分支 main；只剩主仓库 worktree，没有遗留开发/审查分支或自有任务 agent。
- 公开 corral ls 仅有 corral/main、drover/main、saddle/main，分别位于各自主仓库。不要关闭、送话或输入操作这些用户会话。
- 公开 drover list：current、awaiting 均为空，pending 为空，loop=false、gate=true、paused=false。本次只读核对，没有执行 go/next 或切换循环/暂停状态。
- 最新产品二进制来自 working 动画修正：共享 target/release/saddle 与仓库 target/release/saddle 已同步，默认 ~/.local/bin/saddle 链接共享 release；发布时三入口 SHA-256 均为 `70068fdb01bee765b7e58b0501590b1e6c824db9d623b870b38a0db1d6851dc3`。之后仅修改测试/文档，未再次发布；不自动重启用户现场。
- 迟到任务提醒查到 not_found 即忽略，不重新创建 agent 或重复分派。

## 尚未解决与验证限制

- 暂无正在实施或等待审查的任务，无待提交代码。仍有 **picker/close confirmation 的偶发 workflow 测试失败**，基线也曾复现，根因未定位。
- 最近一次主控标准检查在颜色测试隔离任务：NO_COLOR=1 下 app 两项通过，workflow 57 passed、1 failed、2 ignored；唯一失败为 `t20_r1_pending_new_pane_keeps_known_source_cwd_for_shell` 取得 close confirmation 时 unwrap(None)。Clippy/fmt/diff 通过。不能因其他轮次全套通过便宣称此问题已修复，不重复跑整套碰运气。
- working 动画修正按纯视觉预算审查，开发 UI 39 项通过；T21 只核对文档与实现及 diff 检查，没有跑套件、交叉审查或构建发布。
- effort 图标只读取创建时公开 labels.effort，不表示或控制运行中的实际推理强度；不要把用户视觉确认写成运行时档位检测验证。
- T20 的单实例 ctl 修改记录上限仍为256次，满后拒绝新修改；属已记录边界，当前不扩展。

## 接手优先读

- `AGENTS.md`：主控分派、公开CLI边界、用户agent保护及合并收尾规则。
- `docs/DESIGN.md` 第23/39/40节：effort、终端工作区、Agents 3a及动画修正。
- `docs/任务/Agents侧栏3a主控审查.md`：规格修正与已知偶发问题。
- `docs/任务/颜色测试环境隔离.md`、`docs/任务/Working动画与effort区分.md`、`docs/任务/T21-effort图标对照说明.md`：最近三项的完成、审查和清理记录。
- T20需要追查时读 `docs/任务/T20-终端工作区与命令控制.md` 和 `docs/任务/T20-主控与交叉审查.md`，不重开已完成任务。

## 下一步

等待 T23 实现结果，主控审查后完成合并与收尾。其余任务等用户逐项讨论后再派发。
