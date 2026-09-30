# 通用插件 Attention 来源

用户原话：“我理解了，那么按照步骤做吧”。按已讨论顺序，主控亲自完成第一步：通用 Attention 条目快照和点击打开目标，以独立演示插件验证；随后再接 Drover 插件并替换旧路径。不委派。

范围：本轮保留内建 Drover 与 Agent Attention，不操作真实队列/上游/服务。快照更新不自动发通知、不改变业务状态；演示只用合成数据。工作树 plugin-attention。

实施与检查：先检查新能力和快照协议的 RED，再实现协议/SDK/宿主/演示；验证来源隔离、替换/撤回、断连/停用、过期点击及目标打开，最后跑标准检查。本文记录的是主控方案，不扩写用户验收原话。

## 实施与主控审查

实现提交 6658cdc。attention.v1 提供有界完整快照、SDK 保存回执与 AttentionOpen 事件；宿主按运行会话/快照修订核对点击，不解释 opaque target。每来源独立，空快照撤回，非法快照保留旧值，断连替换为不可点击来源错误，停用撤下。旧插件/Counter 不需升级；Agent 与内建 Drover 路径保留。

审查确认：不增加系统通知或任务写操作，不读取 Corral/Drover 内部数据；点击只路由到已运行的同一插件，插件重新核实业务状态。插件条目不能 Mark seen。Enter 的过期选择拒绝，显示新快照时按插件+条目 ID 保留选择，重排行不会转到别的条目。新增协议能力及清单 action 引用一致；请求预算/目标深度受限；SDK 最多一个快照请求在途，覆盖未发快照不产生幽灵回执。注意 accepted 不代表用户已经看见。

## 检查记录

- 新能力 RED：宿主原不支持 attention.v1，目标检查确实失败；实现后通过。日志 `/tmp/saddle-attention-red.log`、`/tmp/saddle-attention-green.log`。
- 选择重排 RED：旧选择逻辑在 revision 改变且重排后选错项；修复后回归通过，`/tmp/saddle-attention-selection-red.log`。
- 插件与 Attention 模块最终 22 passed / 1 ignored（该 ignored 是既有 SDK stdio 外部构建检查）。覆盖非法字段/重复ID/容量/深度/能力、原子替换/空快照、两来源隔离、重启会话与旧点击、进程退出与停用/移除。日志 `/tmp/saddle-attention-module-tests.log`。协议预算检查另在标准套件通过。
- 真实SDK演示PTY：准确选择 sample-1/2、更新/撤回、关闭重开和重新发布通过。测试夹具先出现异步绘制片段定位、缓存画面尚未交互、连续 Esc+a 被识别为组合键的问题，已改为等界面完成并在 Esc 后等 Agents；没有因此改动宿主原输入策略。`/tmp/saddle-attention-pty.log`。
- 仓库外完整复制、固定 Git SDK `6658cdc2d4f0534fba52b0360a109de46f60d895`，独立 package.sh 成功；其产物真实PTY通过1条。日志 `/tmp/saddle-attention-external-build.log`、`/tmp/saddle-attention-external-pty.log`。本地首次解析新提交仅以命令级 Git URL 替换验证，不改用户 Git 配置；合并推送后公开提交可达。
- 宿主标准 Clippy 和独立示例 Clippy 均通过。`/tmp/saddle-attention-clippy.log`、`/tmp/saddle-attention-demo-clippy.log`。
- 首轮全套一条 T20 完成请求 attach 的 pty 状态断言失败，单独复跑通过；最终全套一条 T20 当前pane/picker检查失败，不能宣称全绿。详见后续最终统计，不顺带修 picker/T29。

本轮无委派、无真实队列/服务/上游改动。最终提交仅做通用接口和独立示例，不能将其记为 Drover 插件迁移完成。

最终完整标准套件：323 passed / 1 failed / 8 ignored，`/tmp/saddle-attention-final-tests.log`。失败 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases` 在3239行等待 Create agent，画面仍停在 picker。候选单跑仍失败；未改 main（33f4efe）第一次单跑通过、第二次在同一行同一画面复现失败，证明确为基线也存在的问题，根因未查清。基线日志 `/tmp/saddle-attention-t20-baseline.log`、`/tmp/saddle-attention-t20-baseline-repeat.log`。随后的 change-repeat 因共享 target 重用基线测试程序（93 filtered），不作为候选新证据；候选以最终全套和基线之前的 targeted 检查为准。没有为了全绿改动无关 picker 测试。

主控审查结论：本步可合并；已知标准套件失败如实保留。固定 SDK 的独立示例 Clippy 再通过，日志 `/tmp/saddle-attention-demo-pinned-clippy.log`。日常 release 将另行构建并以真实插件PTY核验，不使用上述共享debug程序推断发布有效。

## 合并与日常发布

05efd12 固定独立SDK并记录审查，8888bb2 合并，62453ee 空提交收尾；已清理本轮分支/worktree，没有创建或关闭新agent。日常旧程序备份 `/Users/firegnu/Library/Application Support/saddle-release-backups/plugin-attention-20260930-191209`，旧SHA-256 631d2fed76b93b0d3f225b482d709daccaeb7c847b07d717ca61126eb8610324。

main release 构建与正式 demo package.sh 成功。随后用实际release执行全部9条插件PTY（包含旧Counter和新独立Attention demo）全部通过：`/tmp/saddle-attention-release-pty.log`。测试结束后的日常release SHA-256 `d92ff67177a1d6143264ae57f50e1661694a87a20d6d4db6c228eda3ea9dfc8a`，入口仍为 ~/.local/bin/saddle → 共享target/release/saddle。未强制重启窗口。

正式演示产物 `/Users/firegnu/Developer/personal_projs/saddle/examples/attention-plugin/dist/attention-plugin` 已生成，尚未登记到真实插件配置。用户重启新版后可以自行添加/启用，从Attention点两条合成数据；u更新/恢复，w撤回，Esc关闭。此次交付只完成迁移第一步，下一步是Drover插件，最后才删除宿主旧专用路径。
