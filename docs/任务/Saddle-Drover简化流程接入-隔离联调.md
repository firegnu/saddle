# Saddle 与 Drover 简化流程：隔离联调

2026-09-30，saddle/main 给既有 saddle/dev-drover-schema2-1 的同任务后续。继续使用原分支/worktree和模型，不再委派。本轮只补一个隔离主路径联调，不扩产品功能或插件设计。

## 已审事实

主控已审 5fbe04f/5e74eac 的接口、命令目标、确认页、返回结果和通知。主控标准检查 292 passed / 1 failed / 3 ignored，Clippy 与 diff check 通过。失败为既有 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases，单独复跑仍在 picker 点击处失败；不改该用例/终端逻辑，不顺带修 T29。对照结果见主控审查文件。

## 联调范围

- 用你当前 Saddle 测试二进制 + /Users/firegnu/Developer/personal_projs/drover-worktrees/task-flow-simplification/bin/drover（cb67fe8）。不要使用已安装旧 CLI。只走公开 CLI，不改 Drover 仓库。
- 复用现有 workflow Harness，必要时增加一个显式 opt-in 联调测试，放 tests/workflow.rs；不要另建框架。HOME/XDG/运行目录/项目登记/状态/配置/临时 Git 仓库完全隔离，Corral 为假脚本或空 MAIN_AGENT 手动派发；不得联系真实 agent。不操作真实 T55/T57、注册表、服务或用户正在运行的 Saddle。
- 单一主路径：临时任务 A 开始后退回 Pending，保留 A 的未合并分支；明确派发任务 B，经过 Saddle 的 Submit for review 确认进入 Awaiting，观察渲染屏幕上的内部提示与 Attention；再经 Saddle Accept 确认进入 Done。A 仍 Pending、分支保留，没有自动派发。测试自身的 B 接受只限合成任务。
- 此路径使用实际 Drover list/show/action 响应；转换由公开命令产生，不直接把预设 Awaiting JSON 塞给 Saddle。保留通知启动基线；不扩渠道切换、完整边界矩阵或桌面录屏。派发可使用手动模式，不需真实实现 agent。
- 若发现当前契约/实现缺陷，保留失败证据、报告具体调用点后停；不要擅自修改功能代码或上游。

## 预算与交付

只运行该隔离联调主路径，新增/调整测试后运行其相关编译检查；不重跑全套、不再复跑 T29、不发布或重启真实服务。只提交必要测试和完成记录到当前分支，不合并、不推送。报告实际两端提交、命令/退出结果、渲染提示及状态证据、限制与取舍。记录需区分测试二进制中的提示和用户桌面现场提示。命令前台等完，最后一行 DONE。

## 本轮补充请求（2026-09-30）

> 主控已核对本轮交付，现继续同任务隔离联调。先读 /Users/firegnu/Developer/personal_projs/saddle/docs/任务/Saddle-Drover简化流程接入-隔离联调.md 和同目录 Saddle-Drover简化流程接入-主控审查.md，按请求只补并运行一个隔离主路径。真实开发版 Drover CLI + Saddle 测试二进制；不真实发布，不推进 T57，不再委派。标准检查已有 picker 失败，主控另在独立 target 核对 main 基线，你不要重复跑全套或该用例，不修 T29。把本请求及结果记录到你的分支。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 两端及改动

- Saddle 产品源码：`5e74eacf21be3714120289aee6690d75e61fa4a5`（包含实现 `5fbe04f`），本轮产品源码零改动。当前分支 `drover-schema2` 的本次提交只包含 `tests/workflow.rs` 和本记录。
- Drover：`cb67fe8881d6dce88a1ff4e33dbc8a87542da111`；实际调用上述开发 worktree 的 `bin/drover`，结束时该 worktree 干净，未改上游。
- 复用 workflow Harness，增加显式忽略用例 `real_drover_schema2_return_submit_notify_accept_path`。通过 `SADDLE_DROVER_FLOW_BIN` 指定真实 CLI，通过 `SADDLE_FLOW_EVIDENCE_DIR` 保存证据；默认测试不启动该联调。
- Harness 补齐临时 XDG/config/data/cache/runtime 与 TMPDIR；原有 HOME、XDG_STATE_HOME、SADDLE_RUNTIME_DIR 继续隔离。合成项目登记、`.drover.conf`、HANDOFF_DIR 和 Git 仓库均在 TempDir 内。CLI 包装器清空继承环境，原样转发真实 CLI 输出/退出码；只写合成配置，任务与运行状态全部由公开 CLI 生成。
- Drover 的 `MAIN_AGENT` 为空，派发为手动模式；PATH 首项和 `DROVER_CORRAL_BIN` 指向拒绝调用的假 Corral，断言未被调用。Saddle 使用空 agent 列表的假 Corral，断言只有只读 `ls`。没有接触真实 agent、任务、注册表、全局配置或服务。
- 实际 Saddle 测试二进制：`/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/debug/saddle`，SHA-256 `b1fbd73c18bc7ef8f616ef41afd35edd56467fe60a1727a35b9851c202686b7b`。两端路径、提交与二进制摘要保存在最终证据的 `endpoints.json`。

### 命令与退出结果

全部命令前台等待退出；以下为本轮范围内的编译、联调和静态检查，没有运行全量或 picker 用例。

```sh
CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo test --test workflow --no-run

SADDLE_DROVER_FLOW_BIN=/Users/firegnu/Developer/personal_projs/drover-worktrees/task-flow-simplification/bin/drover \
SADDLE_FLOW_EVIDENCE_DIR=/tmp/saddle-schema2-flow-20260930-GBrP3r \
CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target \
cargo test --test workflow real_drover_schema2_return_submit_notify_accept_path -- --ignored --exact --nocapture

CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target cargo clippy --test workflow -- -D warnings
rustfmt --check --edition 2024 --config skip_children=true tests/workflow.rs
git diff --check
```

- 编译检查退出 0；最后一次联调退出 0，**1 passed / 0 failed / 85 filtered out**，耗时 4.51 秒；限定 workflow 的 Clippy、rustfmt 和 diff check 均退出 0。最终 `run.log`、`clippy.log` 在 `/tmp/saddle-schema2-flow-20260930-GBrP3r/`。
- 保留同一主路径前三次试跑的退出 101；均为新增测试的输入同步/定位错误，未改产品代码，不计作产品缺陷的 RED 证据：
  - `/tmp/saddle-schema2-flow-20260930-zF1k8u/run.log`：旧详情读取/旧页面满足了等待条件，过早点击尚在加载的 Submit 确认；调用日志没有 `done`。改为先观察本次确认页，再等待读取结束。
  - `/tmp/saddle-schema2-flow-20260930-Xuxi7S/run.log`：两次 Esc 连续写入只退出一层；已成功进入 Awaiting 并捕获内部提示。改为逐次发送，并等待列表按钮出现。
  - `/tmp/saddle-schema2-flow-20260930-z6MVqU/run.log`：`Accept` 子串命中了详情中的 `Accepted`，未进入确认页；已观察内部提示和 Attention。改为精确定位 `Accept›` 按钮。
- 每次试跑都是全新临时项目/状态，失败日志及真实 CLI 响应都保留；未重跑全套、既有 picker/T29 用例或其他 opt-in 联调。

### 实际主路径证据

最终证据目录：`/tmp/saddle-schema2-flow-20260930-GBrP3r/`。`drover-calls.jsonl` 保存每次公开调用的参数、工作目录、退出码及原始 stdout/stderr，所有响应退出码为 0。每个检查点同时保存真实 `list --json` 和 vt100 解析后的渲染文本；确认页也有独立快照。

合成 A 为 `T1`，运行 `cfc1da06f4a648758d53789097039a0d`；合成 B 为 `T2`，运行 `341c9a629e754fde8c2234284fbbf935`。这些编号仅属于临时项目。

| 检查点 | 公开状态与画面事实 | 证据文件前缀 |
| --- | --- | --- |
| 启动基线 | A Running、B Pending；通知设置预先为 In saddle，观察至少两次设置读取；Attention 0，无 ready for review 提示 | `01-startup-baseline` |
| A 退回 | Saddle 填写原因并勾选工作停止后确认；current/awaiting 均 null；Pending 顺序 A、B；paused 仍 false | `02a-return-confirmation`、`02-a-returned` |
| 明确派发 B | Saddle 对第二项 Dispatch selected；B Running、A Pending，delivery=not_sent；未派发 A | `03-b-running` |
| 提交 B | Saddle Submit for review 确认后实际 `done T2 --target-token … --json` 成功；B Awaiting，同一 run_id，公开 notification_key 存在 | `03a-submit-confirmation`、`04-b-awaiting-toast` |
| 内部提示与 Attention | 实际渲染 `flow-project · T2 ready for review`；Attention 1，打开后含 B 的 Awaiting release 条目 | `04-b-awaiting-toast-screen.txt`、`05-attention-screen.txt` |
| 接受 B | Saddle Accept 确认后实际 `go T2 --target-token … --json` 成功；history 中 B 同一 run_id、status=done、t2=1790743882.563464 | `05a-accept-confirmation`、`06-b-accepted` |
| 结束 | current/awaiting 均 null；Pending 仅 A；Attention 0；`git branch --no-merged main` 仍有 synthetic-a | `07-no-auto-dispatch`、`unmerged-branches.txt` |

实际渲染文本节选：

```text
Attention · 1
→ flow-project · T2 ready for review ×
→ flow-project · T2  Awaiting release · Synthetic B
```

状态变更调用顺序严格断言为：A `dispatch-pending --pos 1` → A `return-to-pending`（原因及 `--work-stopped`）→ B `dispatch-pending --pos 2` → B `done` → B `go`。没有 `next`、`loop`、人工覆盖完成或接受后的自动派发。

A 分支包含独立的调查提交，退回后及 B 接受后都仍未合并；B 没有实现提交或合并操作，仍可完成提交与接受。临时项目在用例结束后统一清理，外部证据保留。

### 限制与交付边界

这是 **Saddle 测试二进制的 PTY 渲染提示**，不是用户桌面正在运行的 Saddle 的现场提示；没有桌面录屏或实际发布结论。只验证指定的一条主路径，不扩渠道切换、真实 agent 送达或边界矩阵。

没有发现本路径上的产品/公开契约缺陷；本轮没有功能修复。主控记录的标准检查 picker 失败及 main 独立 target 对照仍由主控处理，本轮未复跑、未修 T29，不把本用例通过等同于标准检查全绿。

未委派、合并、推送、安装、发布、重启真实服务或推进 T57/T55/T38；仅将测试和本请求/结果记录提交在 `drover-schema2`，等待主控后续处理。
