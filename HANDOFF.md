# Saddle 交接

更新：2026-10-03。**卡皮巴拉图片与 Blocks 两版已审查集成并清理，尚未部署。** 安装仍为 `7acceb3`，包含用户认可的 A 图片猫。

## 本轮完成：卡皮巴拉

- 实现 `ea665b3`，完成记录 `e6cf18a`，主控审查 `0776995`，合并 `33c00d7`，收尾 `eda201b`。任务及审查见 `docs/任务/卡皮巴拉宠物.md`；两版静态/动作预览见 `docs/调研/卡皮巴拉宠物.md`。
- Pet 增加 Capybara，配置 capybara，默认仍 Clawd；沿用 Auto/Blocks、Save/Cancel/恢复默认/即时生效。既有四套宠物素材与渲染器不变。图片 41 姿态、Blocks 35 姿态，两版七段动画节拍一致：走、转身、发呆、嚼叶、打盹、顶橘子、小鸟落背。接受方块版简化嘴和小道具细节。
- 主控标准首轮 496 passed、1 failed、9 ignored；既有 workflow 鼠标任务表单在 tests/workflow.rs:851 失败，限定复跑 1 passed；补完中断后插件/示例目标 128 passed。合计覆盖 625 项通过、9 项忽略，不是一次全套全绿；Clippy 通过。首次失败原因未证实，未扩修；日志 `/tmp/saddle-capybara-review-*.log`。
- `capybara-pet` 分支/worktree 已清理；自建实现者 `saddle/dev-capybara-1`（instance=`6c79ab3c803c`）确认 idle、attached=0 后关闭，stop ok、SIGHUP、exit_code=129。未选择遥测、未创建 Tasks run。
- 下一步等用户安排；本轮明确不自行部署，尚未在真实 Ghostty/Metalterm 目测。预览的 8×19 来自既有测试注释，不是本轮实时测量。

## 本轮完成：A 图片猫猫

- 用户在四个视觉候选中选择 A「描边大头橘猫」，确认仅替换图片猫。实施 `2378583`，主控审查 `d7d8785`，合并 `1690bb7`，收尾 `6dcbbf6`。
- 正式包仍为 55×23、23 姿态；八段动作清单与节拍原样保留，走路仅腿动。Blocks 猫、Blocks Clawd、图片 Clawd、渲染器和配置完全不变。
- 实现者记录 `tests/mascot.rs` 22 项通过；主控核对 diff、TOML 节拍和正式全姿态/深浅背景预览，通过。本轮是纯视觉调整，按预算未重复全套或 Clippy。8×19 是既有测试记录的尺寸，非本轮实时测量；未在实际 Ghostty/Metalterm 验收。
- 文件已进 main：`docs/任务/图片猫猫候选调研.md`、`docs/调研/图片猫猫候选.md` 和同名预览目录（正式结果 `final-*.png/gif`，旧概念图保留为历史）。
- `cat-image-candidates` worktree/分支已清理；自建实现者 `saddle/dev-cat-candidates-1`（instance=`8d0d47f7daf0`）确认 idle、attached=0 后关闭，stop ok、SIGHUP、exit_code=129。本任务未选择遥测、未创建 Tasks run。
- 用户随后要求部署，已切换两命令入口与 Drover/Diff 注册到 `7acceb3`；四程序哈希、插件回读与隔离 release 退出重开测试 1 项通过。备份 `~/.local/share/saddle/backups/cat-a-7acceb3-20261003-120318`；详见 `docs/任务/A图片猫猫-部署记录.md`。等用户重启后看效果，Pet=Cat、Display=Auto；Blocks 猫不变。
- 卡皮巴拉已由后续用户明确授权实施，当前分派见上文。

## 本轮完成：宠物显示方式

- Settings → General → Pet 后新增 Display：Auto 默认支持时图片、否则字符；Blocks 始终字符。不增加强制图片，沿用 Save/Cancel、恢复默认与保存后即时生效。
- 实现 `916e84e`，行为级 RED/GREEN 补充 `0bfbdc0`，主控审查 `ebe15d2`，合并 `1f5b6b9`，收尾 `71002a1`。任务及详细证据见 `docs/任务/宠物显示方式.md`。
- 主控标准首轮 495 passed、1 failed、9 ignored；既有 workflow 鼠标任务表单正文断言失败，原因未证实。限定复跑该项 1 passed；补完中断后未执行的插件和示例目标 128 passed；合计覆盖 624 项通过、9 项忽略，不是一次全套全绿。Clippy 通过，未改断言。日志 `/tmp/saddle-pet-display-review-*.log`。
- `pet-display-mode` 分支/worktree 已清理；实现者 `saddle/dev-pet-display-1`（instance=`25007afe83ef`）确认 idle、attached=0 后关闭，公开 stop 返回 ok、SIGHUP、exit_code=129。本任务未选择遥测、未创建 Tasks run。
- 用户随后明确要求「部署啊」，已切换 saddle/corral 入口及 Drover/Diff 注册到 `419d302`；四个程序哈希核对、候选/安装公开状态及隔离 release 退出重开测试 1 项通过。私有备份 `~/.local/share/saddle/backups/pet-display-419d302-20261003-113016`；详见 `docs/任务/宠物显示方式-部署记录.md`。
- 下一步等用户重启，Settings → General → Display 选择 Auto/Blocks 后 Ctrl-S 保存。尚未在真实 Ghostty/Metalterm 目测切换；不能把检查当作视觉验收，不自行重启用户界面或启动新任务。
- 上次宿主回读 instance=`3101f085078dc2e7`、PID=10055，加载 `f7c967a/bin/saddle`，Drover/Diff 同版；主控 instance=`faec2c2b00cb` 保持。用户的修复反馈不等于逐项验收所有终端与宠物动作。

## 当前状态

用户批准把最后一层 Corral infra 真正用 Rust 迁入 Saddle；已直接实施、验证、合并并切换安装。该迁移没有委派、Tasks 任务或遥测。`saddle/main` instance=`faec2c2b00cb`、agent_pid=99462，由成套 Rust Corral pen 托管；旧主控已不在公开列表。迁移后的首次核对为 `7755bb8`，后来宿主和插件更新为上文 `f7c967a`；存活主控的 pen 仍可引用保留的旧版本。迁移时 Drover 查询正常、Tasks 默认接收者 `saddle/main`、无 Running/Awaiting、8 项 Pending，这是历史快照。本轮未重查真实队列，也未复测新主控退出重开与外部 attach 全路径。

## 本轮计时与宠物修复

- Agents 状态计时：实现 `2df04b8`，主控审查 `6e44bfe`，合并 `bc5f44e`，收尾 `3df626b`。working 显示本轮时长，idle/waiting 从进入状态计时；公开状态缺可信起点时显示 `—`。主控标准复核 609 passed、0 failed、9 ignored，Clippy 通过。任务与审查见 `docs/任务/Agents状态计时.md`；现已随 `f7c967a` 部署，待用户重启宿主生效。
- 计时实现者 `saddle/dev-state-timer-1` 已确认 idle、attached=0 后清理 worktree/分支并关闭，stop 返回 ok、exit_code=0。没有操作真实 Tasks 或记录遥测。
- 宠物实现 `5bf6af5`（方块步态）、`ad5a643`（图片版），主控审查 `88605bf`，合并 `fdbe95c`，收尾 `79d315d`。用户要求的 Clawd 来回走时头和上身固定、只让腿动已落实；支持 Kitty 图形协议的终端默认图片，否则回退方块版。主控在合入计时修复的组合候选上复核：621 passed、0 failed、9 ignored，Clippy 通过；任务与完整取舍见 `docs/任务/图片版宠物与Clawd步态.md`。
- 宠物 worktree/分支已清理；实现者 `saddle/dev-pet-images-1`（instance=`1e54868368ec`）确认 idle、attached=0 后关闭，stop 返回 ok、SIGHUP、exit_code=129。该任务未选择遥测、未创建 Tasks run。当前只保留主控与下文三个历史 worktree。
- 待用户看效果：Ghostty/Metalterm 实际图片大小、背景/弹窗层次、闪烁和橘猫造型未验收。离线素材预览 `/tmp/saddle-pet-images-review-preview.png` 不等于真实终端截图；开发者 kitty 临时协议程序没有运行完整 Saddle。图片版筛选 20 段三行内动作，整体镜像及 2.4 格/秒步速的取舍已接受。启动探测期间普通键入会被丢弃、极慢响应可能进入按键流，作为非阻断后续建议记录，未扩修。
- 用户已授权部署并表示稍后自行重启。本轮在旧界面保持运行时准备不可变成套包并切换磁盘入口，未停宿主/agent；旧进程继续引用旧版本。当前入口为 `f7c967a`；包校验、隔离 release 生命周期 1 项、插件注册和旧主控身份回读通过。下一步等待用户重启后核验实际加载路径和视觉效果，不自行重启或再次切换。

用户要求使用感觉保持一致：关闭 Saddle 不停 agent，外部仍能 `corral attach`，重开恢复同一实例；原 Corral 仓库只读保留、独立可用，不产生对宿主/插件/遥测的反向依赖。用户现有主控都有 HANDOFF，部署后由用户自行重开，不热替换旧 Python 进程，不自动关闭它们。

此前文本宠物包已收尾，旧 Claude 已按授权关闭；迁移部署及新主控接续现已完成。图片版原暂缓要求已被本轮用户明确放行替代。后续部署仍先准备并通知用户退出界面，不直接停止现有主控。

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
- 新包 `~/.local/share/saddle/versions/7755bb8` 已用于两个命令入口及 Drover/Diff 注册路径；私有备份 `~/.local/share/saddle/backups/corral-rust-7755bb8-20261003-021136`。历史部署证据/哈希见 `docs/调研/Corral核心Rust部署记录-2026-10-03.md`。当前宿主 instance=`97bdbda2959d903f`，新主控身份见上文；该记录中的旧主控保留状态是迁移当时的快照。
- 部署必须先备份旧入口/配置/技能，再把成套包放持久版本目录、切链接。不要覆盖共享 `.target/release` 当成新版安装，不删除仍有 pen/hook 引用的版本。当前全局 corral 技能已核为实际目录/文件，不是软链接；部署时重新核所有权，不强制覆盖用户资源，不写原 Corral 仓库。
- 上轮遥测边界部署详情在 `docs/任务/遥测单任务边界-部署记录.md`；本轮已切换文件入口/插件路径，config.toml、资源所有权、真实 trace/队列未改。Corral 技能 dry-run 两处均 same，无需覆盖。原程序/插件目录保留。
- 详细操作见 `docs/Corral新旧主控交接清单-Saddle示例.md`，已用 Typora 打开。新旧主控可不同名并存，读取 HANDOFF 核对后再停旧；新建表单 Controller 短名锁定 main，自定义主控名需用公开 CLI 创建并带 role=controller。底层/角色显示/Tasks 不要求名字为 main，Tasks 接收者需显式调整；现有 run 不自动转移。本项目 AGENTS 的 saddle/main 是项目约定。

## 宠物包（由 saddle/claude-1 按用户直接布置完成，2026-10-03）

- 已合并 `0761aeb` 并推送，分支和 worktree `pet-packs` 已清理。吉祥物改为可换的宠物：Clawd 加原创橘猫，Settings → General → Pet 选择，配置 `mascot = "clawd" | "cat"`。素材是 `assets/pets/*.toml` 文本宠物包，Rust 直接读；Python 生成脚本和二进制帧已删除。设计见 `docs/DESIGN.md`「宠物包与第二只宠物」，格式见 `assets/pets/README.md`。
- 合并前分支上格式和 clippy 通过，全量 63 组通过；workflow 组 `pending_delete_button_confirms_names_the_task_and_can_be_cancelled`、`native_mouse_buttons_cover_forms_and_stop_confirmation` 两项失败，原因未查清（可能与共享 `.target/debug/saddle` 被并行构建覆盖有关，未证实）。用户决定先合并，由主控用隔离 target 复跑。
- 部署准备时基于 `7755bb8`、显式 native target 串行限定复核上述两项，2 passed；原失败原因仍未知，不声称一次全套绿。新 release 包隔离生命周期1项通过，日志 `/tmp/saddle-rust-deploy-7755bb8/`。
- 效果只看过离线渲染的预览图，未在真实终端核验；用户日常用 Ghostty 和 Metalterm。已随本次成套包切换安装，待用户重开体验。
- 图片版源码现已完成并合并，细节及未验收边界见上文「本轮计时与宠物修复」。新版已部署，待用户重启查看；无宠物遗留分支/worktree。

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
