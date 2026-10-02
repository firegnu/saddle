# Corral Rust 核心集成：实施与核验

日期：2026-10-03。基线：Saddle `2495cd5`（含已批准设计）、原 Corral `6923da1ee0c766468c31e1f577fb8370b8b6da77`。主控直接实施，没有创建 Tasks 任务、没有委派或记录本次遥测。

## 交付范围

- `crates/corral-core/`：独立底层库及 `corral` CLI；命令族、登记与唯一名称、pen/PTY/socket、独立 attach、终端模式、环境重建、Claude/Codex Rust hook、pi/omp 同版本 TypeScript hook、事件/游标、输入确认、after、技能安装。
- `src/agent_program.rs`：默认同包路径；宿主和 headless agent 包装共享；显式覆盖保留，无 PATH 兜底。
- 通用进程插件环境 `SADDLE_AGENT_BIN`；Drover 只选择程序路径，任务引擎、派发/验收/遥测业务不改。
- `scripts/package.sh`：新建不可变产品目录，内含宿主、Corral、Drover/Diff、技能说明和二进制哈希；不安装或切换入口。
- 新核心及配套新测试没有 Python 源码；原 Corral 仓库仅作只读基线，外部可选比较不属于产品运行依赖。没有新增底层对宿主/插件/遥测的依赖。

## 行为证据

以下均用隔离 HOME/CORRAL_HOME、合成事件及 `/bin/cat`/shell 假 agent，不读写真实任务或会话数据。

| 边界 | 自动检查 |
|---|---|
| CLI 独立存活 | start 返回后 pen 存活、重复名退出码5、显式 stop 后退出码2、同名重开 |
| 输入与状态 | recognized hook、完整 reply、confirmed 与 merged、子会话过滤、未知结果不重发、人类活动保护 |
| attach | 真实测试 PTY、Ctrl-] 分离、多个连接写者交接、resize、退出末段输出、wait 取消 |
| 并发与身份 | unique 并发登记、并发 ls/status 不清活锁、after 不跟随复用名称的新 instance、stop 客户端超时后 pen 继续收尾 |
| 环境与文件 | 登录环境重建/父身份清除/显式覆盖、已有父目录权限不变、新目录0700、helper 固定绝对路径、切公共链接后旧 helper 仍可执行 |
| 上层一致性 | 默认同包优先于 PATH、缺失不回退、配置和显式覆盖、插件收到宿主选择的绝对程序路径、无记录上下文不建遥测数据 |
| 新旧互操作 | 两种 CLI 分别操作两种 pen 的状态、read、keys、where、ls、stop；交替事件/cursor/完整回复/合并输入；四种 attach 组合的输入、resize 与分离 |
| 产品生命周期 | 未开 TUI 先 start；退出 TUI 后同一 agent PID/instance 存活，重开重接；遥测列表为空。已用暂存 release 宿主和核心验证 |

对应源码：`crates/corral-core/tests/{lifecycle,protocol,compat,product}.rs`、核心单元测试、`tests/agent_program.rs`、`tests/plugins.rs`。兼容比较显式设置 `CORRAL_COMPAT_BIN`；产品测试显式设置 `SADDLE_TEST_HOST`（可再用 `SADDLE_TEST_CORE` 指定同包 release 核心）。默认底层测试无需旧 Corral 或宿主。

## RED/GREEN 与静态修正

有效的目标失败和修正日志保留在 `/tmp/saddle-corral-*-red.log` / `*-green.log`：生命周期、read/send、attach、hook/reply、after、技能同意/所有权、attach 缺失检查/wait、环境捕获截止、默认程序路径、目录权限、子命令帮助、启动准备截止。其中初始未实现命令返回 `usage`/未实现错误后由相应实现变绿；权限用例明确复现已有目录0775被改0700；准备截止用例明确复现半行管道等待超过截止。

不能视为有效 RED：compat 初期借用检查编译错误、错误外部基线路径、PTY 夹具未排空终端输出导致 TCSADRAIN 等待、产品夹具未响应光标位置查询；均只修夹具。`drain-red` 名称的那次运行本身通过，最终输出 flush 是对原实现的静态契约对齐，不能宣称该日志证明 RED。

升级测试额外发现 macOS `current_exe()` 会保留公共软链接：真实 RED 分别表现为 hook 仍引用 `current-corral`、宿主错误选择链接旁的 Corral。现先 canonicalize 再派生 helper/同包路径，插件收到的宿主路径也固定到真实版本。目标均变绿：`/tmp/saddle-corral-version-link-{red,green}.log`、`/tmp/saddle-corral-bundle-link-{red,green}.log`；没有改真实链接。

静态对齐还包括 pen 最终输出有限冲刷、ls 单栏位5秒/普通请求30秒和 IO 失联的 not_found 分类。Clippy 只机械整理本次新代码，没有改无关生产代码。

## 标准检查的原始结果与污染处理

- 首次 `cargo test --all-targets`：69通过、11失败、3忽略，在 agent_capture 中止。日志 `/tmp/saddle-corral-standard.log`。该目标单独串行复跑24通过，原失败的就绪/截止问题没有证据可宣称同因或已修复；没有改遥测生产逻辑或放宽断言。
- 补首次未执行的根目标：379通过、2失败、5忽略，在 workflow 结束后停止，两个 examples 尚未运行。日志 `/tmp/saddle-corral-omitted-root.log`。
- 补其他 workspace 成员：128通过、0失败。日志 `/tmp/saddle-corral-omitted-workspace.log`。
- 根目标补跑的 mascot 失败截图明确出现本分支没有的 `Pet`/`Mascot` 设置；当前源仍是 `Clawd mascot`。另一个用户正在同一共享 `.target/debug/saddle` 构建 pet-packs，证明该运行中至少有测试启动了其他候选的二进制。`closing_a_tab...` 失败在启动阶段 agents=0 且无假Corral日志；不能仅凭前者断言后者同因。
- 因此，没有把上述共享二进制运行称为候选全量绿；所有启动 `CARGO_BIN_EXE_saddle` 的根测试目标转到同一共享 `.target` 下的显式原生 target 子目录重验，并补 examples。未改旧UI断言、未杀或干预用户的其他开发会话。命令清单 `/tmp/saddle-corral-pinned-args.json`，构建日志 `/tmp/saddle-corral-pinned-build.log`，结果 `/tmp/saddle-corral-pinned-tests.log`。
- 独立路径重验11个宿主二进制消费者目标和2个examples：193通过、2失败、4忽略；workflow 101通过/4忽略，原两个界面失败未复现。agent_capture仍有2个夹具ready屏障超时，不能把全部原始失败归因于共享二进制。最后只复核这两项和新增软链接路径用例，不重复全套。
- 标准 Clippy 原先报告新代码嵌套if/冗余write选项，限定整理后 `cargo clippy --all-targets -- -D warnings` 通过：`/tmp/saddle-corral-clippy-final.log`；最终软链接修正后再次通过 `/tmp/saddle-corral-clippy-verified.log`。

最终限定复核 `/tmp/saddle-corral-pinned-targeted.log`：原两项 ready 超时目标2通过，最终软链接宿主选择1通过；没有增超时、放宽断言或修改遥测生产路径。最终核心 `/tmp/saddle-corral-core-verified.log`：单元4、生命周期8、协议/身份10，合计22通过。插件路径直接回归2通过，最终 Clippy 通过。原次时间敏感失败根因未确定，不声称修好；显式假Corral路径逻辑未变且相关复核通过，主控将此作为已记录验证限制，不扩大本项为旧测试调度修复。

三项独立互操作检查分别通过：`/tmp/saddle-corral-compat.log`、`/tmp/saddle-corral-events-compat.log`、`/tmp/saddle-corral-attach-compat-green.log`。release 产品生命周期检查通过 `/tmp/saddle-corral-release-product.log`；最终源码 `820fda2` 的干净 release 包同项确认通过 `/tmp/saddle-corral-final-product-ready.log`。确认中曾发现测试在恢复尚为 attaching 时即读取 instance；只把夹具就绪条件收紧到该 pane 为 running 且有 corral_instance，沿用原8秒上限，没有改生产逻辑或降低身份断言。原失败保留 `/tmp/saddle-corral-final-product.log`。依赖图 `/tmp/saddle-corral-dependency-tree.log` 不含上层包。`git diff --check` 和打包脚本语法检查通过。原始失败保留，不改写为一次标准全绿。

主控核对结论：本项源码功能通过，允许按仓库规则合并；真实安装和真实agent验收仍另行进行。没有另外派发审查者。


## 交付限制与部署边界

- 本轮只验证 macOS 原生架构。未运行真实 Claude/Codex/pi/omp 产品请求；各适配按固定原基线移植并用合成事件/假进程验证，不能冒称四个真实 agent 全部端到端验收。
- 未修改原 Corral 仓库、用户现有 pen、真实 CORRAL_HOME、配置、技能链接、任务队列或 SQLite，也未更换日常安装/发布远端 release。
- 包内版本目录不得原地覆盖；存活 pen/hook 所引用版本保留。旧 Python 会话保持原状，用户部署后自行重开主控、读取已有 HANDOFF。
- 现有独立 Corral 未来版本不在本次无条件兼容承诺内。协议/事件版本不认识时拒绝，不调用旧 Python 回退。

## 最终构建产物（未安装）

构建源码 `820fda28d52028ef66c1a0d2953410e5b4a94900`，working-tree clean，macOS arm64。目录 `/tmp/saddle-corral-820fda2.xaHm6L/product`，指针 `/tmp/saddle-corral-package-final-path`；构建日志 `/tmp/saddle-corral-package-final.log`。这是临时验证产物，若被系统清理可按打包脚本重建；不要把临时目录当日常安装路径。

| 文件 | SHA256 |
|---|---|
| bin/saddle | b90f9e1b45e9678bfa6609be32b91ffaa1c8e66938edd2ea34c7adf5bf0b7083 |
| bin/corral | 6c52fe4cc4e0f521e3921ff822582bbf82bc5e56e195c294716cad00741d6dfa |
| plugins/drover/bin/saddle-drover | 9a3b138b2d60abe6d032cf6f1d16e26a2088be72fe49e70e73c84798cda38256 |
| plugins/diff/bin/saddle-diff | d661e34cf2b7f7523216d67ba7070bfe48d0ed076450875440b03b44ba2a82ba |

已核两入口为 Mach-O arm64；`corral --version` 返回 contract 1；包内无 Python 文件。重复指定已有产品目录退出1且拒绝覆盖，未触发再次构建或安装。后续仅测试/核验文档提交不改变以上生产源码。
