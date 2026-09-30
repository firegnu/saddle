# 插件宿主、Rust SDK 与首个 Demo

主控亲自实施，分支 plugin-demo，worktree ../saddle-worktrees/plugin-demo；没有创建/操作实现 agent，也没有加入或推进 Drover 队列。

## 授权和范围

用户原话：“接下来的工作你自己做，不走任务委派了”；确认在 Saddle 内绘制插件界面后：“那继续吧”。

本轮按已确认的进程插件设计和管理页/面板交互实施首个独立 demo，Drover 为第二阶段。验收原话不扩写；以下为主控工程检查与实际证据。

## 实现

- 公开 protocol crate：版本信封、完整样式片段帧、固定 Unicode profile、有界 JSON 编解码。
- 公开 Rust SDK：私有 CLOEXEC 协议句柄、业务 stdout 转日志/stdin 转空输入、事件循环与通知超时、Ratatui buffer 转换。
- 宿主：子进程独立 session/无 PTY/清理父身份，握手/超时/重启/回收，后台画面验证与有界管道，内部通知。
- 插件登记：独立 plugins.toml、默认停用、清单校验、短写锁/基线比较/原子保存、即时管理页，设置草稿保留。
- 工作区：plugin 内容类型、单插件单面板、结构化输入、禁用/故障占位，layout v2 兼容及旧文件备份，ctl inspect 如实识别。
- 独立 Counter demo：按钮/Enter、通知、关闭保留计数、停用/重启归零。SDK 消费方式见其 README。

## 检查记录

RED→GREEN：

1. 协议初始骨架接受所有画面；目标检查因“中”越过一列边界未被拒绝而失败；实现宽度及行校验后通过。
2. 生命周期初始骨架停留 Disabled，真实假插件握手检查失败；补上进程通路后通过。中间补齐夹具的 plugin.toml 缺失，该夹具错误单独修复，不作为额外 RED 证据。
3. 现有布局解析不认识 plugin，恢复测试失败；增加独立内容及 v2 后通过。
4. 关闭最后一个插件会把格式降回 v1，新增恢复保护测试失败；保留格式版本后通过。

隔离界面：单独构建 SDK demo，在临时 HOME/config/state/runtime 和假 Corral/Drover 中，经过实际 Saddle PTY 验证登记、启用、面板、Enter/鼠标、内部通知、Shift+Enter、关闭后重开保留计数、重启归零、停用占位。没有触碰真实工作区。

主控发现并修正：通知成功回执无变化重绘导致过期点击；新增 F5 页签折行挤掉原通知设置说明；关闭最后一个插件反复降级布局；终端 inspect 不能给插件伪造项目 cwd；插件提示与既有 Drover 提示分别占位，避免点击目标重叠。

其他定向检查：中英文/组合字符/emoji/样式边界、非法控制码及超长行、未知登记版本保留、并发登记拒绝覆盖、身份不符停止回收、顽固进程先回收再重启、旧布局备份；真实 SDK stdio probe 验证 println/子命令日志不污染协议、子命令 stdin 已关闭。

最终标准检查：`cargo test --offline --all-targets --no-fail-fast` 为 **305 passed / 1 failed / 6 ignored**。失败 `t20_r1_replacing_pane_keeps_displayed_cwd_in_both_pending_phases` 位于 workflow.rs:3239，picker 点击后仍留在原弹窗。同一未修改基线 2bfe1c4 的该例首次单独通过，再跑在同一步失败；基线全量 workflow 亦有 `terminal_picker_binds_new_form_and_shell_exit_and_close_are_modal` 失败。保留既有 T29 范围，不为插件任务扩大修复 picker。

较早全量运行另有 `closing_a_start_target_keeps_the_created_agent_available_without_attaching` 一次失败，单独复跑通过，最终全量也通过；未声称其偶发根因已查清。F5 引入的两个 Settings 可见性回归已修正，15 项 Settings 检查全部通过。

`cargo clippy --offline --all-targets -- -D warnings` 通过。标准套件跳过项中，本轮两项需外部构建产物的检查已另行显式运行通过：真实 SDK stdio probe，以及独立 demo 的实际 Saddle PTY 流程。其余既有跳过项未扩大运行。标准检查后新增的管理页/添加弹窗极小窗口检查单独通过；没有为凑全绿重复整个套件。

独立交付验证：SDK/protocol 固定 Git revision `5191a782bb7671390c124485258d83e434104560`；demo 复制到 `/tmp/saddle-plugin-external.h9LoWT/counter`，使用 Git 依赖构建及随后 `--offline --locked` 构建通过，未引用宿主私有模块。提交尚未推送时，仅本次 git-fetch 的环境将远程 URL 改写为本地 Git 仓库传输，不写全局配置；依赖与锁文件仍为公开 Git URL/固定 revision。用这份仓库外产物再跑实际 Saddle PTY 检查通过，合并后该 SDK revision 随 main 一起推送。

补充 RED→GREEN：公开信封把 result:null 错认成字段缺失，roundtrip 检查失败；区分字段存在与 null 后通过。文档围栏、JSON/TOML、链接及 diff 空白检查通过。

主控审查结论：首个插件面板通路可交付源码；没有发现尚待处理的插件范围阻断项。运行预算是首版保守上限，不代表已经做多插件长时间压力测试；T29 的既有不稳定用例保留。未安装/发布到用户实际工作区，未迁移 Drover，不把源码合入当作用户实机验收。
