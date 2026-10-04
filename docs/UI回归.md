# UI 回归与分层验证

适用于现有 Ratatui 界面。优先复用固定合成数据、`TestBackend` 和插件 Buffer；断言检查布局、文字、样式角色与命中区，视觉观感仍需人工查看。这里是现有检查的索引，不是新的截图测试框架。

## 按改动选择范围

| 改动 | 运行范围 |
|---|---|
| 单个页面、字段、文案或局部修复 | 直接相关测试；行为修复先确认目标检查因该缺陷失败，再做最小修复及相关回归 |
| 公共样式、按钮、布局或公共 UI 组件 | 下方受影响的 UI 组；宿主及插件共同使用的样式/组件改变时运行所有受影响组 |
| 跨模块行为/接口改动、阶段集成、发布前 | 全量标准检查；说明触发原因 |
| 文档、运行入口、样例整理 | 校验命令、链接和受影响样例，不为文档制造 RED，不因文档同时涉及多个模块就机械触发全量 |

判断以依赖和影响面为准，不以文件数判断。UI 组不能代替被修改业务的测试：例如任务状态流转应加 Drover 对应业务检查，插件协议改变需要协议/进程集成检查。任务书和完成记录写明选择理由与实际结果。通过后相关代码未再改，不重复全量；返工只验证该次修正和直接相关回归。既有失败如实保留，不借本任务修复无关问题。

保留有价值的测试，分别管理测试总量和单次执行范围；不以删测试、放宽断言、增大超时或一律串行掩盖失败。重命名/迁移测试时同步本索引。

## 运行入口

从仓库或任一 worktree 根目录运行，所有命令共用编译缓存：

```sh
export CARGO_TARGET_DIR="$HOME/Developer/personal_projs/saddle-worktrees/.target"
```

使用 `-p` 限定包，避免工作区 default-members 扩大范围。下面每一块是一组，可单独执行；没有自建调度器。

宿主 / Agents / 弹窗 / Telemetry 阅读层（含公共按钮）：

```sh
cargo test -p saddle --test buttons --test ui --test ui_dialogs_polish \
  --test ui_reading_polish --test ui_second_host --test ui_final_host
```

Settings / Diagnostics / Updates / 插件管理：

```sh
cargo test -p saddle --test settings --test diagnostics --test updates --test ui_second_settings
```

这一组沿用现有测试文件，含配置保存和假命令检查，不拆出重复的绘制测试。局部改动可使用下方单例命令。

Drover：

```sh
cargo test -p saddle-drover-plugin --test ui --test ui_drover_polish
```

Diff（绘制单元测试及包含空状态的现有进程集成样例）：

```sh
cargo test -p saddle-diff-plugin --lib --test process
```

单例示例：只改 Agents 窄栏时，测试名须完整，输出应实际运行 1 项，不能将 0 项视为通过：

```sh
cargo test -p saddle --test ui_final_host narrow_agents_rows_keep_the_name_before_the_time -- --exact
```

全量标准检查（阶段集成、跨模块行为变更或发布前）：

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

这些命令保留现有默认并发及 ignored 规则；不会自动运行需额外环境的 ignored 集成测试。Rust 局部改动仍检查格式；涉及类型/实现时按受影响包、目标选择 Clippy，单纯文档不要求编译整个工作区。

## 已有场景与固定输入

表中测试名可在对应文件搜索。尺寸单位为终端列×行；窄窗断言只承诺检查到的内容，不承诺所有内容同时可见。夹具中的路径、agent 名称和任务数据均为合成值，临时目录的随机前缀不作为截图基线。

| 界面 / 场景 | 现有样例与检查重点 |
|---|---|
| 宿主正常、弹窗 | [ui_dialogs_polish.rs](../tests/ui_dialogs_polish.rs)：`split_sides_share_one_width_and_line_up`（60×20），`open_content_rules_off_fixed_actions_from_existing_agents`（80×24）；分割选项对齐、固定操作与 agent 列表分隔 |
| 宿主空白 / 无匹配 | [ui_second_host.rs](../tests/ui_second_host.rs)：`no_agents_and_a_search_without_matches_read_differently`（100×20），区分没有 agent 与搜索没有结果 |
| 宿主报错 | [ui_dialogs_polish.rs](../tests/ui_dialogs_polish.rs)：`attention_failed_source_reads_as_not_openable_even_when_selected`（80×24），失败来源不可打开；[ui_second_host.rs](../tests/ui_second_host.rs) 保留停止/关闭后果提示和状态栏通知检查 |
| 宿主长文本 / 窄矮窗 | [ui_second_host.rs](../tests/ui_second_host.rs)：`long_shell_directories_keep_their_last_levels_in_the_pane_title`（120/60/40×10）；`saved_pane_actions_sit_right_below_their_explanation`（80×24/9），长路径保留末级、恢复操作贴近说明 |
| Agents 正常 / 长文本 | [ui.rs](../tests/ui.rs)：`multi_agent_layout_gives_names_room_and_keeps_every_field_with_its_agent`、`paths_show_in_full_when_they_fit_and_lose_leading_levels_only_when_too_wide`、`git_summary_line_follows_each_agents_directory_and_wraps_when_narrow`，固定 agent、状态和时间，字段不串行、路径和 Git 摘要退让 |
| Agents 窄窗 | [ui_final_host.rs](../tests/ui_final_host.rs)：`narrow_agents_rows_keep_the_name_before_the_time`（窗口 120/52/60×24；52 列窗的 Agents 栏只有 26 列），先保名称，保留状态符号与选择命中区；空白/错误沿用上述宿主样例 |
| Settings 正常 / 窄窗 | [settings.rs](../tests/settings.rs)：`settings_pages_keep_the_same_frame_and_tab_positions`（120×40、80×24、40×24、60×14）；`page_tabs_fit_one_row_at_normal_width_and_wrap_compactly_when_narrow`（76/100/40/24×24）；`only_the_active_field_shows_its_hint_and_narrow_values_remain_readable`（40×24），临时配置、固定页签和字段 |
| Settings 未检查 / 报错 | [diagnostics.rs](../tests/diagnostics.rs)：`diagnostics_in_settings_show_the_host_groups_and_never_call_the_unrecorded_a_success`、`recorded_results_show_their_errors_and_the_summary_hides_the_home_folder`（100×40），未记录不能显示成功，错误如实展示 |
| Settings 长文本 / 管理失败 | [ui_second_settings.rs](../tests/ui_second_settings.rs)：`diagnostics_and_updates_share_one_label_column_and_keep_every_word`（120×50），固定长报告字段；管理页（110×40）、Add local（100×30）、launcher（80×24）检查主操作、焦点、禁用与失败分离；[updates.rs](../tests/updates.rs) 的 `the_updates_page_names_source_installed_running_and_agents_without_a_new_dot`（120×50、60×40）覆盖实际版本分组 |
| Telemetry 阅读层 | [ui_reading_polish.rs](../tests/ui_reading_polish.rs)：`columns_align_and_selection_reads_by_weight_and_background`（100×40），临时合成 store、需求正文与消息，列表/详情层级和选中样式；本例未打开正文阅读页，不声称覆盖所有遥测状态 |
| Drover 正常 / 长标题 / 窄窗 | [ui.rs](../plugins/drover/tests/ui.rs)：`management_layouts_keep_cjk_status_and_input_target_visible`（160×48、120×36、160×24、80×48、80×24），中文长标题省略、状态和输入目标保留；`content_views_and_edit_keep_reading_positions` 检查长内容滚动；`secondary_pages_are_bounded_and_keep_their_actions_without_queue_tools`（48×24、80×24、180×50） |
| Drover 空白 / 加载 / 报错 | 同文件：`queue_reports_no_active_tasks_even_when_history_exists`、`all_pending_overlay_names_projects_reports_each_state_and_scrolls_to_the_last_task`、`detail_loading_and_failures_never_fake_data_and_refreshes_keep_the_scroll`；固定队列/项目 JSON，空队列与历史、各项目错误、过期详情分开 |
| Drover 操作与矮窗 | [ui_drover_polish.rs](../plugins/drover/tests/ui_drover_polish.rs)：`grouped_task_actions_keep_their_clicks_at_each_size`（150×40、100×30、80×24）；`delete_path_and_empty_wording_is_current`（120×36）；`short_task_editor_keeps_a_body_line_and_normal_size_keeps_the_hint`（80×10、120×36） |
| Diff 正常 / 报错 / 窄窗 | [app.rs](../plugins/diff/src/app.rs)：`header_body_and_footer_use_theme_roles_and_keep_states_distinct`（120×12、90×10、34×10、70×12），固定两文件补丁，模式、错误、正文和底栏角色；34 列 Staged+error 不能互相挤掉 |
| Diff 空白 / 加载 | [process.rs](../plugins/diff/tests/process.rs)：`continuous_view_updates_unfocused_switches_modes_and_reopens_fresh` 在临时仓库切换到空 Staged，检查 `No changes in this mode.`；`dump_synthetic_frames` 的 loading（60×6）供查看，不把仅打印当断言 |
| Diff 长路径 / 长代码 | [app.rs](../plugins/diff/src/app.rs)：`long_paths_and_code_keep_chrome_visible_while_scrolling`（70/120×12），沿用两文件 fixture，合成长路径和 100 个中文宽字符，横向滚动 200 显示列后尾标记可见、顶部和底栏不变；同一夹具进入文本预览 |

## 重复查看合成画面

已有 SVG / 文本导出入口：

```sh
cargo run -p saddle --example ui_preview -- /tmp/saddle-ui-preview-t66
```

生成 `wide`（160×48）、`many-agents`（160×100）、`narrow-agents`（80×24）、`stop` 和 `reply`（120×36）各一份 `.svg` 和 `.txt`。数据与 `now=150.0` 固定；在浏览器打开 SVG 查看排版，文字文件便于比较。它只导出这五个宿主场景，不涵盖所有 Settings/插件页面。

已有测试打印的固定帧可直接查看（单例不会交错输出）：

```sh
cargo test -p saddle --test ui_second_host no_agents_and_a_search_without_matches_read_differently -- --exact --nocapture
cargo test -p saddle --test ui_second_settings diagnostics_and_updates_share_one_label_column_and_keep_every_word -- --exact --nocapture
cargo test -p saddle-diff-plugin --lib app::tests::dump_synthetic_frames -- --exact --nocapture
```

这些入口不读取真实 agent/队列；Diff 进程测试使用临时 Git 仓库。不要为了样例接入正在使用的 agent。完整组加 `--nocapture` 时，如需连续阅读可仅为输出排序加 `--test-threads=1`，这不是改变日常检查并发的约定。

## 覆盖缺口与边界

T66 盘点确认：主要正常、空白、错误、长文本、窄窗口场景已有覆盖；只补 Diff 长路径/长代码绘制样例，复用已有空状态进程检查，不复制相同行为。没有展开所有页面×主题×尺寸组合。

极小尺寸遍历（例如 1 列/1 行）主要保证不崩溃和帧合法，不能当作可读性保证。Diff 明确 RGB 背景的可读性由 `light_theme_keeps_unchanged_code_readable` 检查；终端 Reset 的真实背景未知。SVG 使用预览字体/颜色映射，无法证明真实终端字体、SSH、设备或所有主题的观感；这些仍由人工查看实际界面。

后续 T76 可复用合成 Agent、队列 JSON、报告字段和补丁内容。Ratatui 的 cell 坐标、颜色属性和命中区断言依赖当前渲染器，不能直接算作 GPUI 测试；需要在 GPUI 原型中重新定义界面断言。补全所有 TUI 场景不是 GPUI 原型的硬门槛。
