# Saddle 交接

更新：2026-10-03。当前 main；Corral Rust 核心实现 `820fda2`，测试/产物记录 `ff58742`，合并 `2527c8f`（已推送），收尾空提交 `9020fec`。本交接提交后需确认 main 与 origin/main 同步、工作区干净。

## 当前状态

用户批准把最后一层 Corral infra 真正用 Rust 迁入 Saddle；主控已直接实施、验证、合并。没有委派，没有创建 Tasks 任务，没有记录本次遥测。**源码完成，真实部署尚未进行；日常使用仍是原安装和原 Corral。**

用户要求使用感觉保持一致：关闭 Saddle 不停 agent，外部仍能 `corral attach`，重开恢复同一实例；原 Corral 仓库只读保留、独立可用，不产生对宿主/插件/遥测的反向依赖。用户现有主控都有 HANDOFF，部署后由用户自行重开，不热替换旧 Python 进程，不自动关闭它们。

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

- 最终干净生产源码 `820fda28d52028ef66c1a0d2953410e5b4a94900` 的临时包：`/tmp/saddle-corral-820fda2.xaHm6L/product`，指针 `/tmp/saddle-corral-package-final-path`。BUILD.txt 含各二进制 SHA256；临时目录若被清理可重建。ff58742之后只改测试/文档，不改该包生产代码。
- 尚未更换真实 saddle/corral 命令链接、配置、技能或插件注册，未迁移/关闭真实 trace、未动队列、未发布远端 release。下一步是用户安排真实部署切换，再做自建新会话的实际体验核验。
- 部署必须先备份旧入口/配置/技能，再把成套包放持久版本目录、切链接。不要覆盖共享 `.target/release` 当成新版安装，不删除仍有 pen/hook 引用的版本。现有全局 corral 技能链接受所有权保护，`install-skills` 会保守报 foreign；需在部署时单独处理链接，不写原 Corral 仓库。
- 上轮遥测边界日常部署与备份详情仍在 `docs/任务/遥测单任务边界-部署记录.md`；本次没有改动那套安装。此前“不能自动迁移Corral”的旧交接已被本次源码实施授权替代，真实安装切换仍未授权执行。

## 保留项

- 用户要求只打开的 `saddle/claude-1`（此前instance cb7fea1096b7）由用户自己布置 pet-packs；其 `../saddle-worktrees/pet-packs` 继续保留，不送话、不停止、不替它合并或安装。会话身份如需操作必须先公开核对。
- 保留用户所有既有 agent。保留 `review-telemetry-design`、`t38-dispatch-study`、`t55-notification-flow`；没有修改原 Corral 仓库。
- 单任务遥测规则仍是一次明确授权对应一条新 trace，完整交付后正式 close；下一任务不自动续记。不要复用旧Theme上下文。本任务没有记录选择。
- 既有资源失败提示/Tasks降级文案等历史非阻断项没有扩修；详见先前审查记录。

## 优先阅读

1. `AGENTS.md`、`docs/DESIGN.md`。
2. `docs/Corral核心Rust集成设计.md`、`docs/调研/Corral核心Rust集成-实施核验.md`。
3. `crates/corral-core/README.md`、`scripts/package.sh`、`src/agent_program.rs`。
4. `docs/插件协议.md` 第19节、`plugins/drover/README.md`。

后续 cargo 仍共用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`。并行项目构建会覆盖同名顶层binary；需要启动固定候选时用不可变暂存包或明确隔离的target路径，不能把被覆盖的测试结果当本候选证据。
