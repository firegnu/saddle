# Saddle 交接

更新：2026-10-03。部署准备源码 main=`7755bb8`，包含 Corral Rust 核心与宠物包。本轮两项定向复核通过、新成套 release 包构建及隔离退出/重开检查通过、旧安装已备份；**待用户退出 Saddle 后才切换入口，当前仍是旧安装。**

## 当前状态

用户批准把最后一层 Corral infra 真正用 Rust 迁入 Saddle；主控已直接实施、验证、合并。没有委派，没有创建 Tasks 任务，没有记录本次遥测。**源码完成，真实部署尚未进行；日常使用仍是原安装和原 Corral。**

用户要求使用感觉保持一致：关闭 Saddle 不停 agent，外部仍能 `corral attach`，重开恢复同一实例；原 Corral 仓库只读保留、独立可用，不产生对宿主/插件/遥测的反向依赖。用户现有主控都有 HANDOFF，部署后由用户自行重开，不热替换旧 Python 进程，不自动关闭它们。

用户此前要求等 Claude 开发收尾后再迁移；文本宠物包已收尾、Claude 已按授权关闭，图片版暂缓。用户最新同意推进部署准备和后续退出/切换/接续流程；先准备，通知用户退出界面后再切换，不因批准流程直接停止现有主控。

## 已完成

- 独立 `crates/corral-core`：Rust CLI、pen/PTY/socket、attach、终端模式、状态/事件/cursor、输入确认、after、环境重建、Rust hook、技能安装；pi/omp 使用随包 TypeScript 扩展。新运行核心和新测试无 Python 源码，无旧 Python fallback。
- 保留公开 `corral start/status/send/reply/attach/stop/…` 和子进程边界。默认 `corral` 指同包程序，显式路径覆盖保留；宿主、采集入口、Drover 通过通用 `SADDLE_AGENT_BIN` 使用一致程序路径。任务/遥测业务未改。
- 修正 macOS current_exe 保留软链接的问题：内部 worker/hook、宿主同包定位、传给插件的宿主路径均固定到真实版本路径。升级只切入口，存活会话引用的版本目录必须保留。
- `scripts/package.sh NEW_DIRECTORY` 生成不可变成套目录，包含 saddle/corral、Drover/Diff 包和操作资源；拒绝覆盖目标，不安装、不注册、不改链接。
- 本次分支/worktree `corral-rust-core` 已清理；没有为本任务创建 agent，所以没有关闭任何 agent。

## 验证结果与限制

- 核心最终22项通过；旧/新 CLI 与旧/新 pen 的生命周期、事件/回复/cursor、四种 attach/resize/detach 组合通过。最终 release 包退出/重开 TUI 后 agent PID/instance 不变，未产生遥测 trace。
- 首次标准69过11败3忽略，补根目标379过2败5忽略、补其他workspace128过0败；保留原失败，不能说一次全套绿。
- 用户 Claude 的 pet-packs 构建曾替换共享 `.target/debug/saddle`，界面失败截图出现本分支没有的 Pet 设置。全部11个调用宿主二进制的根目标改用 `.target/aarch64-apple-darwin/debug` 重验并补2个examples：193过2败4忽略，其中workflow101过0败4忽略。
- 剩余两项 agent_capture ready屏障超时限定复核2过，最终软链接路径1过；初次超时根因未证实，未放宽断言/预算、未改遥测逻辑。最终核心22、插件路径2、Clippy均通过；完整核对见下方实施核验，不再无条件重复全套。
- 只验证 macOS arm64 与合成 agent，未跑真实 Claude/Codex/pi/omp 产品请求。不要把假进程验证说成四种真实agent完整验收。
- 证据集中为 `/tmp/saddle-corral-*.log`；明细/原失败/裁定在 `docs/调研/Corral核心Rust集成-实施核验.md`。

## 构建产物与下一步

- Corral 核验时生产源码 `820fda28d52028ef66c1a0d2953410e5b4a94900` 的临时包：`/tmp/saddle-corral-820fda2.xaHm6L/product`，指针 `/tmp/saddle-corral-package-final-path`。**该包不包含后来合并的宠物功能，不能直接作为最新整包安装。** 迁移前按最新 main 重新成套构建并核对 BUILD.txt；保留原核验证据，不把它当新整包已验收。
- 新成套包已准备在 `~/.local/share/saddle/versions/7755bb8`，包含宠物，BUILD.txt 校验通过；私有备份 `~/.local/share/saddle/backups/corral-rust-7755bb8-20261003-021136`。具体证据、哈希和待办见 `docs/调研/Corral核心Rust部署记录-2026-10-03.md`。尚未更换真实命令链接/配置/技能/插件注册，未动真实 trace/队列；下一步等用户退出界面并核实后切换。
- 部署必须先备份旧入口/配置/技能，再把成套包放持久版本目录、切链接。不要覆盖共享 `.target/release` 当成新版安装，不删除仍有 pen/hook 引用的版本。当前全局 corral 技能已核为实际目录/文件，不是软链接；部署时重新核所有权，不强制覆盖用户资源，不写原 Corral 仓库。
- 上轮遥测边界日常部署详情在 `docs/任务/遥测单任务边界-部署记录.md`；本轮仅备份/准备，原安装尚未改变。Corral 安装技能 dry-run 两处均 same，无需覆盖。
- 详细操作见 `docs/Corral新旧主控交接清单-Saddle示例.md`，已用 Typora 打开。新旧主控可不同名并存，读取 HANDOFF 核对后再停旧；新建表单 Controller 短名锁定 main，自定义主控名需用公开 CLI 创建并带 role=controller。底层/角色显示/Tasks 不要求名字为 main，Tasks 接收者需显式调整；现有 run 不自动转移。本项目 AGENTS 的 saddle/main 是项目约定。

## 宠物包（由 saddle/claude-1 按用户直接布置完成，2026-10-03）

- 已合并 `0761aeb` 并推送，分支和 worktree `pet-packs` 已清理。吉祥物改为可换的宠物：Clawd 加原创橘猫，Settings → General → Pet 选择，配置 `mascot = "clawd" | "cat"`。素材是 `assets/pets/*.toml` 文本宠物包，Rust 直接读；Python 生成脚本和二进制帧已删除。设计见 `docs/DESIGN.md`「宠物包与第二只宠物」，格式见 `assets/pets/README.md`。
- 合并前分支上格式和 clippy 通过，全量 63 组通过；workflow 组 `pending_delete_button_confirms_names_the_task_and_can_be_cancelled`、`native_mouse_buttons_cover_forms_and_stop_confirmation` 两项失败，原因未查清（可能与共享 `.target/debug/saddle` 被并行构建覆盖有关，未证实）。用户决定先合并，由主控用隔离 target 复跑。
- 部署准备时基于 `7755bb8`、显式 native target 串行限定复核上述两项，2 passed；原失败原因仍未知，不声称一次全套绿。新 release 包隔离生命周期1项通过，日志 `/tmp/saddle-rust-deploy-7755bb8/`。
- 效果只看过离线渲染的预览图，未在真实终端核验；用户日常用 Ghostty 和 Metalterm。未替换安装版。
- 下一步：图片版，Kitty 图形协议，支持的终端默认用图片、其余自动退回方块版。先做猫；Clawd 用用户提供的 claude.dev 参考动画，只做 3 行高度放得下的。尚未开始：用户先切换新内核，之后重新开会话再做；宠物相关没有遗留分支或 worktree。

## 保留项

- `saddle/claude-1`（instance `cb7fea1096b7`）已按用户本轮授权关闭：交接提交 `803e8b3` 推送后，再核同实例 idle、attached=0，公开 stop 返回 ok、stopped_by=SIGHUP、exit_code=129；随后 status 返回 not_found。最后回复“等你切完再开我”，图片版留待用户换内核后新开会话，不恢复旧会话。没有宠物遗留分支/worktree，没有删除主仓库目录或停止其他 agent。
- 保留用户所有既有 agent。保留 `review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow`；没有修改原 Corral 仓库。
- 单任务遥测规则仍是一次明确授权对应一条新 trace，完整交付后正式 close；下一任务不自动续记。不要复用旧Theme上下文。本任务没有记录选择。
- 既有资源失败提示/Tasks降级文案等历史非阻断项没有扩修；详见先前审查记录。

## 优先阅读

1. `AGENTS.md`、`docs/DESIGN.md`。
2. `docs/Corral核心Rust集成设计.md`、`docs/调研/Corral核心Rust集成-实施核验.md`。
3. `crates/corral-core/README.md`、`scripts/package.sh`、`src/agent_program.rs`。
4. `docs/插件协议.md` 第19节、`plugins/drover/README.md`。
5. `docs/Corral新旧主控交接清单-Saddle示例.md`；宠物后续见本文件「宠物包」与 `assets/pets/README.md`。

后续 cargo 仍共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。并行项目构建会覆盖同名顶层binary；需要启动固定候选时用不可变暂存包或明确隔离的target路径，不能把被覆盖的测试结果当本候选证据。
