# Saddle 简化流程接入：主控审查

2026-09-30，审查 5fbe04f / 5e74eac，Drover 对端 cb67fe8。Dispatch 05458465051541c4a9c2cc908dc9fe3e。

## 本阶段结论

已核对公开 idle 状态和本轮 DONE 回复，分支工作树干净。代码审查及隔离主路径联调均通过，可以准备联合发布；用户随后授权联合发布，执行结果见《Saddle-Drover联合发布.md》。不推进真实 T57，插件设计延后。

## 接口与实现

- schema 2 读取失败/版本不符不会变成空队列。提交、接受、退回使用确认页读取的 task.actions 令牌，绑定项目/任务/run_id；成功同时核对退出码、ok、任务、运行、状态及 record。失败需刷新并重新确认，不自动重试或串联。
- 退役 Next/Loop、旧连做和人工覆盖入口；Running/Awaiting 都可退回，保留原因及工作停止确认，暂停设置不变。
- 通知使用公开 notification_key，保留既有基线及去重。详情按公开记录显示，不补 t2 或通过结论；Git/检查仅为参考。
- 接受取舍：复用确认子页；历史无公开 Git 结束端点时不伪造 Links 区间；按 run_id 丢弃旧详情；派发送达与记录分别报告。没有扩展 Corral、Drover 或插件功能。

## 主控检查

- cargo test --all-targets：292 passed / 1 failed / 3 ignored，日志 /tmp/saddle-schema2-review-tests.log。失败为 t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases，在 picker 的 New agent 点击步骤未进入 Create agent，未到目录断言；该用例及相关 pane 实现本轮未改。
- 按预算单独复跑一次仍失败，日志 /tmp/saddle-schema2-review-pane-rerun.log。不能写作全套全绿或 T29 已解决。
- cargo clippy --all-targets -- -D warnings 与 git diff --check 通过，日志 /tmp/saddle-schema2-review-clippy.log。
- 为区分既有偶发与本轮回归，另做 main 基线同一用例对照；初次共享 target 的结果不能可靠区分构建来源，不作基线证据。改用独立 /tmp/saddle-schema2-main-baseline-target，结果见下。只核对来源，不修复 T29，不扩全套。

## 后续

同一实现者按《Saddle-Drover简化流程接入-隔离联调.md》只补并运行一个隔离主路径。使用真实开发分支 CLI、合成项目/状态和假 Corral，覆盖 A 退回保留分支后 B 提交、内部提示、接受且无自动派发。测试中接受的仅是临时 B，真实 T57 保持不动。

## main 基线对照结果

- 在 main `2899286` 使用独立 target 实际重新编译后，仅运行同一个 pane 用例：仍失败，停在相同的 picker 点击／等待 Create agent 位置（旧源码行 3232，新源码行 3228）。日志 /tmp/saddle-schema2-review-pane-baseline-isolated.log。
- 因此该失败在本次接入前的 main 已可复现，不作为本次流转接入返工项；它仍是未修复的既有测试问题。主控整套结果保持 292/1/3，不改写成通过。
- 已通过 dlog followup 向原实现者确认送达隔离联调请求，仍保留两个开发分支，待联合结果审查。

## 隔离联调主控复核（2026-09-30，fb302d6 / cb67fe8）

- 已核对本轮公开 idle/DONE 回复、测试 diff、真实 CLI 调用日志及渲染文本。Saddle 产品源码未再改；新增 opt-in 主路径及测试环境隔离，工作树干净。开发者最后一次联调 1 passed；前三次新增测试输入同步错误已如实保留，不当作产品缺陷的 RED。
- 主控独立复跑同一用例：1 passed / 0 failed / 85 filtered out，退出 0（4.84 秒）。证据目录：`/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-schema2-flow-review-62gl1qjb`；调用真实 Drover 开发 CLI，运行 Saddle 测试二进制，临时 HOME/XDG/状态/项目，假 Corral。未使用真实任务。
- 真实调用顺序为 A dispatch → A return → B dispatch → B done → B go，各退出 0。A 的未合并 synthetic-a 分支始终保留；B 没有实现提交或合并也能进入 Awaiting，再经明确接受进入 Done。
- 测试 PTY 渲染出现 `flow-project · T2 ready for review` 与 Attention 1，接受后 Attention 0；current/awaiting 均 null，Pending 仅 A，无 next/loop/人工覆盖或自动派发调用。该证据不是用户当前桌面的现场通知。
- 本次 diff check 通过；不重复全套。既有标准检查 292/1/3 和 main 基线可复现的 picker 失败继续保留，本轮未修 T29。
- 结论：批准这两个开发版本的代码与隔离主路径结果，无新增必须返工项。下一步为获用户授权后的联合切换：先备份并退出旧自动推进服务，再协调两个版本和通知观察器，最后只读核对状态；真实 T57 的提交/接受仍须另按用户指令执行。现在不合并、推送、安装、重启或清理分支。插件设计延后。
