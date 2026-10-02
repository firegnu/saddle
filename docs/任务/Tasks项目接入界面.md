# Tasks 项目接入界面

用户要求：
> 我觉得你考虑的基本差不多了。开始实现吧。你直接实现吧。不委派了。
> agents.md先暂时让人工维护。
> 好的。开始吧

主控直接实施；范围为 Drover 项目接入业务和 Tasks 界面。设计见 DESIGN 末节。采用隔离项目、临时 HOME 和假 Corral 验证；不操作真实队列，不改宿主或 Corral，不派 agent。

完成记录待追加。

## 完成记录

- 实施全部在 Drover 插件：Projects 显示已接入/异常状态和接收方；Add project 支持目录输入/浏览、自动建议短名、选择已有 agent 或留空；旧配置需明确 Reuse & add，已接入项目 Open；Settings 修改/清空接收方，Cancel 不保存。
- 新建配置与数据目录、登记路径、补 .gitignore；旧配置原样复用。配置 token 防止过期表单覆盖，设置修改沿用任务锁，锁忙不写。数据目录重名拒绝覆盖，损坏配置/断链不当新项目初始化，.gitignore 无法更新时明确报告部分成功。
- 不写 AGENTS.md、不操作真实任务、不创建/发送/关闭 agent、不改 Saddle 生产代码或 Corral。选择已有 agent 只用公开 Corral ls。插件停用/重启语义保持。
- RED：/tmp/saddle-project-setup-red.log 两项因缺 project-info、Projects 缺 Add project/Settings 失败；此前一次测试构造 private field 编译失败不算 RED，修正夹具后两项失败才作证据。
- GREEN：隔离 HOME 的 project_setup 覆盖只读检查、初始化、注册、保留旧配置/任务、外部改动、任务锁、重名、断链与部分成功；真实插件 SDK process 测试覆盖 80×24 目录浏览、选择接收方、保存、取消、清空接收方，假 Corral 仅允许 ls。
- 标准 cargo test --all-targets 首轮 72 过1败，止于旧 tests/app.rs 的 Settings 行号（上次顶部两行布局后该测试仍找第1行）；单项原样复现。仅改第2行定位，定向1过。日志 /tmp/saddle-project-setup-standard.log、app-recheck.log、app-fixed.log（后两者同 saddle-project-setup- 前缀）。
- 补齐未执行 targets：宿主 337过1败5忽略，插件/协议/SDK等120过0败；宿主唯一失败为旧 workflow 测试期待未登记旧配置直接打开。按本次批准流程补 Reuse & add，并检查配置/队列字节不变，定向1过。日志 /tmp/saddle-project-setup-remaining-host.log、remaining-plugins.log、workflow-fixed.log（同前缀）。全部标准目标覆盖后不同用例531过、5忽略；不是把最初失败运行改写为一次全绿。
- cargo clippy --all-targets -- -D warnings 通过；git diff --check 通过。未重复全套。所有测试使用隔离目录/HOME/假服务。
- Release 仅构建 Drover：cargo build --release --locked --target aarch64-apple-darwin -p saddle-drover-plugin --bin saddle-drover。隔离暂存 /tmp/saddle-project-setup-package-3755yq76，二进制 sha256 896077aefbc151e01db6c681cc6be01d1e435b16e0d5dce890996d621dbe3c05；保留日常清单（无能力变更），sha256 6b059a067bfe4407da5170222c53aa3ed160e6a9910e4bef8f3ea5fd2c29a9f2。
- 运行中的日常 Drover 尚未替换；已请用户先停用，收到答复后才备份/替换。Saddle 宿主无需更新。人工验收文档 A2 已在主仓库改为界面流程；真实人工端到端仍未执行。

- 合并 8a317e6 已推送，开发工作区/分支清理，无 agent；release 隔离 initialize/shutdown 通过，日志 /tmp/saddle-project-setup-release-smoke.log。日常更新仍等待用户停用插件，未把构建等同安装。

## 实际安装

用户确认停用 Drover 后，公开状态 enabled=false，进程列表无 saddle-drover。旧包完整备份 `/Users/firegnu/.local/share/saddle/backups/tasks-project-onboarding-20261002-125905`。原子替换日常二进制，旧 SHA256 `159135cfdc86eb849dbd1ea8db1d27c73a0c414b120e8e374e05b0bea5836820`，新 SHA256 `896077aefbc151e01db6c681cc6be01d1e435b16e0d5dce890996d621dbe3c05`；manifest 保持 `6b059a067bfe4407da5170222c53aa3ed160e6a9910e4bef8f3ea5fd2c29a9f2`，未改其他配置或真实任务数据。替换后公开状态仍 disabled，通知用户 Enable。宿主和 Corral 未重启；安装证据在备份 update.json 和 /tmp/saddle-project-setup-install.json。真实人工业务验收未执行。
