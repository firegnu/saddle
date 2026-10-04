# T74：实现已确认的 Tasks 直接入口

2026-10-04，saddle/main 交给原实现者 saddle/dev-t74-design-1（Claude Code，opus[1m] / high）。沿用原 worktree/分支；这是同一 T74 的实施阶段，不另开 Tasks 任务。
路由：常规 / 交叉审查要 / 影响面碰要害（tier.verdict=null，主控沿用已熟悉设计的常规档；路由提示异步定位风险，完成后由独立 Codex 审查）。
类型：功能变更
依据：用户已确认方案并说“继续吧，实现依然在这个任务中实现”。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户要求与批准

用户原话：现在的每一个repo的tasks入口有点麻烦，需要先打开plugins，之后定位到tasks插件，然后才能打开任务。这个有点深。要重新设计一下。

目标：重新设计各 repo 的任务入口，让访问对应项目 Tasks 的路径更直接，减少打开 Plugins 后再定位 Tasks 插件的操作层级。具体入口形式留待设计时确定。

设计已完成并经主控复核（候选 ee9b570、复核 ec33dec）。用户随后询问未安装、未启用、后来停用和卸载的情况，主控按设计说明：未安装无入口；停用保留固定、点击解释原因且不自动启用；再次启用无需重固定；移除清除固定、重装需重新固定；未曾固定则启停不新增入口。用户认可后正式要求“继续吧，实现依然在这个任务中实现”。

## 先读与工作位置

- AGENTS.md、docs/UI回归.md。
- docs/Tasks入口设计.md 全文及 docs/任务/T74-Tasks入口设计复核.md 的最终结论。
- docs/DESIGN.md §62 的插件边界及已批准规则；实现前先同步本轮已获批的规则变化，把提案状态改为已批准待实施，不重开设计选择。
- worktree：/Users/firegnu/Developer/personal_projs/saddle-worktrees/t74-tasks-entry-design，分支 t74-tasks-entry-design。当前 ec33dec，干净，保留设计提交；不要切到 main。
- 范围：宿主 src/plugins/、src/ui.rs、src/app.rs、src/app_plugins.rs 及直接需要的输入/底栏/来源目录代码；plugins/drover/ 的定位与来源展示；相应测试、设计和运行说明。不得搭车改无关模块；不改主控维护的 HANDOFF.md。

## 实施约定

按已确认设计实施，不再停在写方案：

- 宿主管理一个固定入口。管理页 Pin/Unpin，仅可打开视图的插件可固定，无默认固定；顶部显示 action 标题，保留 Plugins/Settings/Telemetry。按设计窄栏退让，绘制与点击一致。Agents 焦点 p 打开固定入口并提示使用 selected；Viewer 的键盘输入不拦截。
- 固定与启用状态分离。停用或失败保留入口，点击解释状态并到已有管理入口，不自动启停/重启；注册移除时清除固定。复用 registry 现有锁、并发修改保护和持久化通路，正常启停/增加其他插件等写操作不得丢失固定。
- 来源按设计：Agents 的选中 agent、Viewer 的当前 agent；普通终端用启动目录，不跟随 cd，不把它说成实时 cwd。宿主只传 cwd，不读取 Drover 业务数据。Attention/通知的显式目标不被普通入口来源覆盖。
- Drover 定位区分匹配、确认无匹配、无法判断；已登记空项目仍可到达，读取失败由正常错误显示。保留现有主目录/worktree 优先规则。用户输入、关闭/再次打开等过期结果不能覆盖新现场，草稿/确认/忙碌/通知偏好等按设计保留；来源行清楚标出实际项目和未切换状态。不要通过新增后台重试或延后静默切换补偿。
- 设计里的三态为 Drover 内部结果，不增加公共协议/SDK/插件清单能力。沿用英文界面、现有任务状态流转及生命周期；不吸收 T63/T76。
- 开发中的具体实现选择按现有机制做最小改动；如发现必须改变已批准产品行为或公开接口的实质缺口，报告主控，不自行扩大。

## 验收原话

“重新设计各 repo 的任务入口，让访问对应项目 Tasks 的路径更直接，减少打开 Plugins 后再定位 Tasks 插件的操作层级。”

“继续吧，实现依然在这个任务中实现”

## 验证预算

- 行为变化先定位/新增目标检查，在实现前记录因目标行为缺失而失败的 RED，再最小实现达到 GREEN；不把编译错误当 RED。
- 围绕本轮实际改动验证固定读写/启停移除、相关宿主 UI/输入、Drover 来源定位与现场保留。使用现有 UI 组及直接相关测试，覆盖已确认边界，避免重复造样例或扩展所有尺寸/主题矩阵。仅合成数据、临时目录和假公开命令，不调用真实 agent/项目操作。
- 跑相应目标回归、受影响包/目标 Clippy、cargo fmt --check、git diff --check。所有 Cargo 命令共用 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。
- 本次跨宿主与 Drover 行为变更，全量标准测试与全量 Clippy 由主控在阶段集成时集中跑一次；实现阶段不要再跑全工作区，每次小修也不重复全量。保留失败原始结果，不删测试、放宽断言或超时；怀疑无关失败可单项复核一次并如实记录，不搭车修复。

## 不要做

不合并 main、不推送、不部署或重启、不修改真实配置/任务队列、不自动 Submit/Accept/Return；不改 Corral 仓库或用户 agent，不批量杀进程，不读私人 Corral 状态目录，不自行委派。遥测由主控记录，不自行调用记录命令。

## 完成时

仅在本分支提交实现与完成记录，说明用户能看到的结果、目标 RED/GREEN、回归命令/结果、取舍和未做事项。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

记录身份：trace 0a8a12d6-8ea4-477a-b00c-1e1fe470919c，run 5e378db26877a85fefe4684199d17910，implementation dispatch b4447de7-1097-4b0a-9a04-4155cabe24ff。任务正文中旧“仅加入待办”与设计阶段“不实施”已由当前授权覆盖；保持同一任务，不另建或重派队列项。

## 完成记录（实现者，2026-10-04）

### 用户能看到的结果

- Settings → Plugins 管理页选中 Drover 点 `Pin`，Agents 头部出现 `Tasks`（Telemetry 左侧）。之后点一下 `Tasks`，或在 Agents 焦点按 `p`，直接打开 Tasks；底栏写 `p Tasks (selected)`，表明键盘用的是列表选中项。没有默认固定，首次需 Pin 一次；`Unpin` 取消。
- 只能固定有视图的插件；内置 Dispatch 不出现 Pin，无视图插件 Pin 禁用并在详情写 `No view to open; cannot be pinned.`。标题最多 12 列；窄栏与 Telemetry 同组换行，放不下时先隐藏固定入口，Telemetry/Plugins/Settings 位置不变。
- 停用、失败等状态下入口保留并弱化；点击或 `p` 打开 Plugins 面板并预选该插件，显示原有原因说明，不启停或重启。启停、增加其他插件保留固定；移除该插件清除固定。固定存于宿主 `plugins.toml` 的 `pinned`，沿用登记锁和“已被修改，先刷新”保护。
- 来源：Agents 选中 agent、Viewer 当前 agent 窗格；普通终端现在传启动目录（不跟随 `cd`）。宿主只传 cwd，不读 Drover 数据。
- Drover：已登记的空项目也会切过去；Tasks 内来源行写明 `Showing <实际项目> · <结果> · from <来源>`（第一轮返工后的格式，实际项目与状态优先占宽），结果为 locating…、no added project matches（附 `c Projects → a Add project`）、couldn't match: <原因>、kept your current page、not applied。已登记项目即使队列读不出也切到其列表并显示 Read failed（第一轮返工补齐）。草稿/确认/通知偏好/项目接入/执行中操作不切换；定位中按键、粘贴、点击、拖动、滚动作废结果，悬停不作废；关闭、Attention/通知打开清除来源，再次打开重新定位。协议、SDK、清单格式未改。

### RED / GREEN

- Drover RED（实现前，进程测试，均启动成功后卡在目标行为）：`opening_from_an_added_project_without_tasks_shows_that_project`（空项目不切换，等不到 `other ▾ c`）、`opening_from_an_unmatched_or_unreadable_source_keeps_the_shown_project_and_says_why`（无提示）、`opening_over_a_draft_or_preferences_keeps_the_page_and_names_the_shown_project`（无保留说明）、`input_while_locating_discards_the_result_but_hovering_does_not`（无定位状态）。实现后 4 项 GREEN。
- 宿主 RED（实现前，workflow）：`pinned_tasks_entry_is_set_once_opens_with_p_and_stays_while_disabled` 卡在 `p` 无反馈；`a_terminal_gives_its_launch_directory_as_the_tasks_source` 卡在终端打开 Tasks 无来源行。实现后 GREEN。
- 实现后补的直接检查：`tests/plugins.rs::a_pin_outlives_other_registry_writes_and_leaves_with_its_registration`（固定读写、启停/新增保留、并发保护、移除清除、残留 pin 忽略、无视图不可固定）；`tests/ui_final_host.rs::a_pinned_entry_joins_telemetry_wraps_with_it_and_gives_way_first`（宽/窄/极窄/超长/不可用，已登记到 UI回归.md）。

### 回归命令与结果（均加共享 CARGO_TARGET_DIR）

- `cargo test -p saddle --test buttons --test ui --test ui_dialogs_polish --test ui_reading_polish --test ui_second_host --test ui_final_host`：全过。
- `cargo test -p saddle --test settings --test diagnostics --test updates --test ui_second_settings`：全过。
- `cargo test -p saddle-drover-plugin`（整包）：全过。
- `cargo test -p saddle --lib --test plugins --test plugin_resources --test app --test input --test core_plugins --test mascot`：全过（plugins 1 项原有 ignored）。
- `cargo test -p saddle --test workflow`（因改了 Agents 按键/头部/底栏而跑整个目标）：多次运行均为 102 过、1 败、4 忽略，失败是 `native_mouse_buttons_cover_forms_and_stop_confirmation:851`。该项在基线 ec33dec 的临时 detached worktree 同负载下 2/2 同处失败，文档中已多次记为负载相关既有问题；单项复跑通过；未修复。`closing_a_start_target_keeps_the_created_agent_available_without_attaching` 一次偶发失败（已有记录），其后未复现。
- `cargo clippy -p saddle -p saddle-drover-plugin --all-targets -- -D warnings`：无告警。`cargo fmt --check`、`git diff --check`：通过。
- 未跑全工作区 `cargo test --all-targets` / 全量 Clippy（留给主控集成）。

### 测试改动说明

- `ui_second_settings.rs::add_local_shows_its_main_step_and_errors` 与 `plugins.rs::plugin_management_separates_details_and_keeps_full_setup_scrollable`：管理页多了 Pin 一站，按 Tab 次数定位的步数各加 1，断言不变。
- `workflow.rs::tasks_open_on_the_focused_agents_repository_unless_it_has_no_tasks` 改名为 `…_even_without_tasks`：原测试断言的“无任务不切换”正是本次批准改掉的行为，改为断言切到空项目并显示来源行。
- 新 workflow 测试曾在满负载下因 Esc 与随后按键被合并读成 Alt 而失败；改为等底栏变为 `Input ▸ Drover`、关闭后等覆盖内容消失再继续（与 `open_tasks` 助手一致）。

### 取舍与未做

- 管理页按钮名用 `Pin`/`Unpin`：`Pin to header` 在 48 列下多占一行、挤掉详情（被现有测试发现）。悬停不作废定位：否则鼠标点入口后移入 Tasks 即失效。均已写入 `docs/Tasks入口设计.md` 实施说明与 DESIGN。
- 未改协议/SDK/清单，未合并 main、未推送、未部署或重启，未动真实配置、队列或 agent；HANDOFF.md 未改。基线对照用的临时 worktree 已删除。

## 第一轮定向返工记录（2026-10-04，依据主仓库 `docs/任务/T74-Tasks入口实施返工.md` 与独立审查 R1/R2）

起点 e6edaca，只修 R1/R2；Pin/Unpin 文案、悬停不作废定位及其余通过项未动。

### R1：保留现场时看清实际项目与未切换状态

- 来源行改为 `Showing <实际项目>[ · <结果>] · from <来源>`，按实际可用宽度排版：先保实际项目和结果，其次原因，最后来源路径（剩余不足 8 列时省略）。`Source` 移到 `plugins/drover/src/queue.rs`，由列表分隔线、对话框顶行、Projects 页按各自宽度绘制。
- 项目接入与通知偏好：覆盖区第一行留给来源行，对话框在其下的区域居中绘制，不再被盖住；表单命中区来自绘制结果，随之下移。草稿与按钮命中保持。
- 实测（插件内区，对应宿主窗口）：62×17（80×24）的 Add task 草稿重开显示 `Showing project-alpha · kept your current page`，来源因宽度省略，草稿保留；94×26（120×36）的接入表单第一行为 `Showing project-alpha · kept your current page · from …work/case-…/project-beta`，表单完整；62×17 的通知偏好第一行同样保留。

### R2：已登记但队列读不出时切到目标列表

- `Request::Project`：已登记且有 `.drover.conf` 的项目直接建立列表与 worker，队列读取错误进入原有 `read_error`（显示 `Read failed`）；未登记、缺配置的目录仍走原接入检查。不写入、不修复数据。
- 定位结果只有在列表确实显示目标项目（无 setup、项目一致）后才记为已切换，否则为 not applied。
- 实测（94×26）：`project-beta ▾ c`、`Read failed`、`Showing project-beta · from …`，没有 Add project；`queue.md` 仍为目录，未生成 tasks.state。

### RED / GREEN

- 新增 `plugins/drover/tests/process.rs`：`a_kept_draft_names_the_shown_project_in_an_80_by_24_window`、`reopened_setup_and_preferences_keep_a_source_row_the_form_does_not_cover`、`a_matched_project_whose_queue_cannot_be_read_opens_on_its_read_error`。
- RED（修复前，`/tmp/saddle-t74-rework/red.log`）：草稿页断言 `Showing .tmp… missing`（帧中只有 `From …/other · kept your current`）；接入表单第一行是对话框边框；R2 停在 `other ▾ c`，帧为 Add project 表单。首轮运行另有两项因 resize 后用旧帧号发键被插件拒收而卡住，属测试自身问题，补“等新帧”后重跑得到上述 RED。
- GREEN（`/tmp/saddle-t74-rework/green.log`）：3 项通过。来源行格式变更后，原先断言旧格式的 3 处同步为新格式：process 的空项目/worktree 用例（`Showing other · from `、`Showing <root> · from ` 且含 `/wt `）、workflow 的 `Showing project-three · from `、终端来源的 ` · from `，断言强度不降。
- 审查探针复制到 `/tmp/saddle-t74-rework/probe.py`（仅改输出目录，并把复现断言改为修复后预期），`visual()`、`unreadable()` 在新构建上通过，画面存于同目录 `draft-host80x24.txt`、`setup-host120x36.txt`、`matched-unreadable-queue-host120x36.txt` 等。

### 本轮运行的检查（均加共享 CARGO_TARGET_DIR，日志 `/tmp/saddle-t74-rework/regression.log`）

- `cargo test -p saddle-drover-plugin --test process --test ui --test ui_drover_polish --test project_setup`：19 / 23 / 3 / 2 通过；格式化后 process 再跑 19 通过。
- `cargo test -p saddle --test workflow -- --exact tasks_open_on_the_focused_agents_repository_even_without_tasks a_terminal_gives_its_launch_directory_as_the_tasks_source pinned_tasks_entry_is_set_once_opens_with_p_and_stays_while_disabled`：3 通过。
- `cargo clippy -p saddle-drover-plugin -p saddle --all-targets -- -D warnings`：无告警；`cargo fmt --check`、`git diff --check`：通过。
- 未跑全工作区、整个 workflow 或所有 UI 组（留主控阶段集成）；未处理既有 `native_mouse…:851` 时序失败。

### 未决

- 列表页的 NoMatch/Unknown 在 94 列下仍会省略来源路径或截断原因（实际项目与结果始终在前）；这是按宽度退让的结果。
- 从 Projects 页手动选择“已登记但队列读不出”的项目，现在同样进入其列表并显示 Read failed，而不是接入表单（与 R2 同一判断）。

## 主控审查与合并收尾

2026-10-04：独立审查发现 R1/R2 后由原实现者修复于 `439c5e3`；原 Codex 定向复核通过、阻塞0。主控阶段集成首次覆盖 697 passed / 2 failed / 9 ignored，两项失败各一次限定复核通过；全量Clippy、fmt及差异检查通过。原始失败和完整命令保存在 `docs/任务/T74-Tasks入口独立审查.md` 及 `/tmp/saddle-t74-integration/`，不能称默认并行全套稳定全绿。

已合并 `a62eecf` 并推送，两个 T74 worktree 与分支已清理，对应实现/审查 agent 随工作目录删除一并关闭。没有部署、Submit/Accept 或关闭 task trace；等待用户后续触发。

### 用户追加授权后的部署

用户随后明确“编译+部署”。从干净 main `29c94d4` 运行 `cargo build -p saddle -p saddle-drover-plugin --bins --release --locked --target aarch64-apple-darwin`（共享 target），退出0；没有重复全量测试。安装包 `~/.local/share/saddle/versions/29c94d4` 更新宿主和 Drover，其余产物沿用原包并在 BUILD.txt 声明；旧包保留。Saddle 入口原子切换，Drover登记目录在原锁及内容比对保护下仅替换路径，配置/其他登记/启用状态和Corral链接不变。入口help和两份release二进制SHA256回读通过。

备份与日志 `~/.local/share/saddle/backups/t74-deploy-20261004-201807/`。公开实例和进程映像核对：宿主PID68447仍运行d7da5b4、Drover PID68450仍运行5f39f88；用户正常重开Saddle才加载新版。主控agent instance不变。没有 Submit/Accept、部署后任务派发或关闭task trace。
