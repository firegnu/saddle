# 任务：03C 可选dispatch插件、路由与技能资源接入

2026-10-02，saddle/main交给新的Codex实现者（重档gpt-6-astra / xhigh）。
路由：重 / 交叉审查要 / 影响面：碰要害（JEV三项均如此，主控采纳）。
类型：功能变更
依据：03A与03B已审查通过，用户要求继续；按已批准设计完成03C，不切换真实消费者。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的实现者：照本文件做，不再开agent。

## 先读

- AGENTS.md；docs/DESIGN.md末尾dispatch职责和用户确认。
- docs/dispatch插件接口设计.md第2节依赖与现有接口、第4.4/4.5、第5节全文、第6.1/6.7、第7/8节03C、第9节用户决定。
- docs/调研/03C-ureq接口核对-2026-10-02.md；docs/任务遥测接口契约.md第5.1节。
- crates/core-plugin/src/lib.rs；src/plugins/{core,capture,cli}.rs的实际接口，src/main.rs组装根及资源/管理测试；不要重做03A/B。
- docs/任务/遥测03B-独立交叉审查.md末尾最终裁定（M1–M3关闭，S1非阻断暂不改功能，目录清理非原子边界保留）。
- 只读旧来源 `/Users/firegnu/Developer/personal_projs/corral/corral-dispatch-skill/{route.py,SKILL.md,项目AGENTS模板.md,README.md}`；HEAD/哈希见研究记录。

## 在哪里干活

- 分支telemetry-dispatch-plugin，worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/telemetry-dispatch-plugin`。从03B审查候选597f48a叠放后合入主控文档；以主控派发时告知的基线为准。
- 允许新增plugins/dispatch及直接测试；修改根Cargo.toml/Cargo.lock、src/main.rs目录组装与必要直接测试；同步README、docs/遥测使用.md、docs/插件系统设计.md及本任务完成记录。插件仅依赖下层core-plugin及成熟通用库，不依赖宿主、Drover或Corral。
- 如通用框架现有接口确有阻断，先报告具体最小缺口，不直接修改03A/B代码；SDK/外部协议/遥测schema与公开行为不变。主控保留03A/B原实施与审查会话处理必要集成返工。

## 要做的

1. `plugins/dispatch`实现CorePlugin，ID dispatch，命令route且capture=Some(Route)，默认停用。仅在二进制组装根登记，宿主库不import插件业务。使用现有headless入口、启停/资源管理，不增加项目采用名单、项目开关、自动AGENTS修改或新界面。
2. 按接口设计5.1逐项移植旧route.py的请求常量、Python strip字符集、顺序/重复键/舍入、置信度/阈值/null兜底、model/usage透传、业务0/1及错误文本边界。完整解析JSON表示保留原键顺序及重复键后值首次位置；不启用serde_json全局preserve_order。只接受5.2已列边缘差异，不自行增加语义差异。
3. 用已核ureq3.4.2/rustls传输，固定生产URL/model，安全TLS、无重定向、分阶段10秒预算、最多2次尝试，重试规则保持5.1。局部传输适配可按研究记录隔离；测试端点/合成key只可通过内部注入，不提供生产环境或CLI换URL入口。读取上限16MiB，无key或非UTF8不发请求；不记录授权头/key，不把库错误Debug整包直接输出。
4. 使用现有Recorder和Completion，按4.4提供begin摘要/实际请求字节、版本/规则指纹、end完整解析response与stdout原字节suggestion；紧挨首次外部调用begin至多一次，重试仍同一operation。不向JEV发送任务书正文；任务书快照由宿主既有机制处理。shape失败仍保存已解析response；非2xx/前次429/截断/无效JSON/超大响应按既定gap分类。未请求采集不建库，遥测失败/关闭不改变业务，不能由插件直接构造遥测事件或访问Store。
5. 复制三份资源到plugins/dispatch/resources/corral-dispatch；模板逐字节不改。SKILL只改5.4指定的问路由命令和兜底，保留技能名、其余编排规则与旧dlog说明（05再处理）；README按5.4更新。停用/未知结果不自行启用、不重试、不回退route.py、不追加询问，依原项目规则/本次明确委派要求继续主控自行定档；完整配对回执判断按设计，不单看末行。资源include_bytes+revision/指纹，插件提供setup_note/setup_files；不要在宿主编码业务说明。
6. 在使用文档说明新入口、采集上下文与业务独立、资源启用/旧链接冲突，以及源码接入不等于真实安装/消费者已切换。将03B候选文档中过宽的“所有路径注明directory kept”收窄，保留S1事实和非原子限制，不顺手修S1功能。产品默认值维持原话注明来源、遥测初始关闭、允许标记晚交。

## 用户要求原话

- “你打算怎么怎么把这个corral-dispatch从corral中拆出来作为sdaale的plugin并且还能支持saddle接下来的遥测？”
- “不但要能支持接到saddle的遥测还要支持用户不要太折腾。”
- “第一版这个插件就在模块安装或者启动的地方写一个说明，让用户知道怎么接入想走主控项目的做法。”
- “如果涉及到要改corral或者发现依赖关系反转了，一定要告知我”。

## 验证预算

- 行为按轻量TDD先有效RED再GREEN，保存实际目标日志与RED测试补丁；编译/fixture失败不算行为RED。纯资源文案不用造RED。
- 限设计第8节03C关键验证：请求常量等价、排序/重复键/阈值和舍入定值、已批准重试/响应gap分类（假传输及回环TcpListener）、无key无begin无请求、摘要/任务书分离、合成key不进入记录回执、资源revision指纹与模板字节一致。完成与03A/B主路径集成一次即可，用临时资源/库，不做额外覆盖矩阵、真实录屏或产品JEV请求。
- 前台标准各一次：cargo test --all-targets；cargo clippy --all-targets -- -D warnings。先设置隔离HOME及XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_STATE_HOME/XDG_CACHE_HOME/XDG_RUNTIME_DIR，保留真实CARGO_HOME/RUSTUP_HOME，固定共享CARGO_TARGET_DIR=/Users/firegnu/Developer/personal_projs/saddle-worktrees/.target，stdin=/dev/null（目标需正文时仅合成输入）。确保新workspace成员纳入标准测试。
- 已有无关失败可单项复跑一次，原失败保留，不声称根因修好；历史registry busy/工作流超时原因未明，不扩旁支。通过后无新改动不重复；返工仅目标/直接回归不全套clippy。git diff --check。

## 不要做

- **需要改Corral或反向依赖先停相关部分报告，不先改后报。** 不编辑Corral/dispatch-log仓库，不改变core Corral技能或实际两处全局链接，不安装技能，不改任一项目AGENTS。
- 不调用真实JEV/Corral进行产品验证，不读取真实密钥值或把它打印到日志；环境中即使有key也不可使用，测试必须合成值或显式移除。依赖下载允许，生产业务请求不允许。
- 不改Drover/Diff/外部SDK/协议/遥测schema，不迁移旧记录、不切消费者，不做04/05。不改原保留worktree，不关闭任何agent。
- 不release/build --release/install，不启动用户正在用的Saddle，不改~/.local/bin；现有安装指向共享target/release，不能覆盖。测试只用隔离HOME/数据，禁止真实队列操作。
- 不引入通用网络框架、后台服务、项目启用管理或额外恢复机制；若已定语义无法在现有接口实现，先报最小缺口。
- 不批量杀进程，只停自己记录PID的进程。不合并main、不推送、不清分支worktree；整阶段集成审查后由主控共同处理。

## 做完

本任务末尾追加完成记录：做了什么、实际RED/GREEN及标准检查、取舍/限制/未做项；在本分支提交。回复SHA与简短结果，命令都在前台跑完，全部完成后最后一行DONE。
