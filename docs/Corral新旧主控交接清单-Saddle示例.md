# Corral 新旧主控交接清单：以 Saddle 为例

日期：2026-10-03。

**本文是待执行清单。写下这些步骤不代表已经部署，也不代表已经停止或新建任何主控。**

当前代码已合并推送；日常命令尚未切换。新核心已做隔离与新旧互操作验证，真实编程 agent 的部署验收留在下面的流程中完成。

## 1. 先弄清楚要切换的两件事

| 对象 | 怎么切换 | 切换后是什么情况 |
| --- | --- | --- |
| 安装和命令入口 | 安装成套新版，切换 `saddle` / `corral` 入口和插件路径 | 之后启动的新会话使用 Rust 核心 |
| 已经运行的主控 | 写 HANDOFF，新建会话读取交接，确认后关闭旧会话 | 新主控接续项目；旧进程不会原地变成 Rust |

**重开 Saddle 界面不等于重开主控。** 关闭界面只断开显示，agent 继续运行。

下面约定：

- 旧主控：`saddle/main`，由旧 Python Corral 托管。
- 新主控：`saddle/main-rust`，由新版 Rust Corral 创建。
- 项目目录：`/Users/firegnu/Developer/personal_projs/saddle`。
- 交接文件：项目根目录 `HANDOFF.md`。
- 新旧共用原有 Corral 状态目录；不搬家、不删除、不手改内部状态文件。

`saddle/main-rust` 只是为了避免同名冲突，不是运行时识别标记。真正用哪个核心，要看创建它的程序路径。开始前确认该名字未被占用；若已占用，换一个明确的新名字，并在后续步骤中保持一致。

过渡过程：

```text
旧 saddle/main：完成交接 → 停止接新工作 → 保留供核对 ───→ 用户确认后关闭
新版程序：                  安装并切换入口
新 saddle/main-rust：                         创建 → 只读接续 → 正式接管
```

## 2. 旧主控先准备交接

这一步在当前旧主控对话里做，还不用退出 Saddle。

- [ ] 等正在执行的工作达到可交接位置；列清未完成的构建、实现者、审查者和提醒。
- [ ] 不让两位主控同时修改项目、派发工作或处理同一个完成提醒。
- [ ] 不借迁移之机合并别人的改动、关闭其他 agent 或推进 Tasks。

给旧主控的提示词：

```text
准备把 Saddle 主控从旧 Corral 会话交接到新的 Rust Corral 会话。
请更新 HANDOFF.md，写清当前分支/提交、未提交改动、已完成工作、待办、
仍在运行的实现者/审查者、待处理提醒、保留的 worktree，以及不能碰的用户会话。
如果有进行中的 Tasks run 或尚未结束的遥测链路，也写明身份和当前情况；
不要自动提交/接受/退回 Tasks，不要为了迁移强行结束业务或遥测。
不要关闭任何 agent，不要部署，不要开新任务。
写完后只报告交接位置和未完成事项，等待我确认。
```

- [ ] 检查 HANDOFF 反映真实状态。已有提交的记下 SHA；未提交的也明确列出，不必为了交接强行提交所有文件。
- [ ] 从这一刻起，旧主控只负责本次部署/交接和回答核对问题，不再接新的开发工作。

## 3. 成套部署新版程序

这部分由负责部署的 agent 执行，你负责退出和重新打开界面。**不需要你自己逐条改软链接。**

部署前可给当前主控：

```text
按《Corral新旧主控交接清单-Saddle示例》准备成套部署。
先备份旧命令入口、Saddle 配置、插件注册和相关技能，准备持久版本目录。
不要停止现有主控，不删除原 Corral 仓库或 ~/.corral 数据。
在需要退出 Saddle 时通知我；确认界面退出后再切换，完成后通知我重新打开。
暂时不要创建新主控，也不要操作 Tasks 队列。
```

### 部署者的核对表

- [ ] 记录旧 `saddle` / `corral` 命令实际解析路径、软链接目标和显式路径覆盖。
- [ ] 备份配置、插件注册及 Corral 技能，记录恢复路径。不要把 API key 等秘密写进交接文档或输出。
- [ ] 将已核验成套包放进新的持久版本目录，例如 `~/.local/share/saddle/versions/<本次版本>/`；不长期从 `/tmp` 运行，不覆盖共享编译目录当作部署。
- [ ] 成套包至少包含 `bin/saddle`、`bin/corral`、Drover/Diff 插件包和匹配的 Corral 资源，核对 `BUILD.txt` 与文件校验和。
- [ ] 先准备好完整目录，再让用户退出 Saddle 界面。用户仍可通过外部终端 attach 旧主控沟通。
- [ ] 确认界面退出后，切换两个命令入口，并将官方插件注册路径指向新包；保留启用状态和其他插件。
- [ ] 检查 Saddle 的显式 Corral 路径、Drover 的 `--corral` 等覆盖，不能全局命令切了而某个消费者仍指旧程序；已有自定义配置要逐项处理，不能全盘覆盖。
- [ ] 备份后更新配套 Corral 技能，遵守资源所有权保护；遇到用户自定义或外来资源先保留并说明，不强制覆盖。
- [ ] 通知用户重新打开 Saddle，核验插件启动和旧会话显示。

本次已观察到的旧入口：

```text
~/.local/bin/corral → /Users/firegnu/Developer/personal_projs/corral/bin/corral
~/.local/bin/saddle → /Users/firegnu/Developer/personal_projs/saddle-worktrees/.target/release/saddle
```

执行当天应重新核对。全局 Corral 技能目前是实际目录/文件，不应按“都是软链接”的旧假设处理。

**不要删除旧 Python 启动脚本、源码、环境或旧会话所需 hook。** 切换的是命令入口，旧进程仍需要原有文件。原 Corral 仓库按约定独立保留。

整个界面已退出时，无需再把 Drover 单独 Disable / Enable 一遍；重新打开后检查它是否按原启用状态启动。不要为核验点击 Submit / Accept / Return。

## 4. 用 Rust 核心新建 Saddle 主控

### 4.1 先确认新入口

部署者在外部普通终端核对：

```sh
command -v saddle
command -v corral
ls -l ~/.local/bin/saddle ~/.local/bin/corral
corral --version
```

- [ ] 两个入口都指向本次成套版本目录，`bin/saddle` 与 `bin/corral` 相邻。
- [ ] 没有 shell alias/function 或旧 PATH 优先项遮住新版命令；旧终端有命令缓存时重新开普通终端后核对。
- [ ] 不仅看版本号：版本号或会话名称本身不能证明是 Rust 程序，须核对目标路径及包的构建记录。

### 4.2 创建新会话

**当前界面限制：** Saddle 的新建表单选择 Controller 时，短名固定为 `main`，不能直接在该表单创建 `saddle/main-rust`。这只是创建表单的限制；Corral 核心、宿主角色显示和 Tasks 接收者没有要求主控必须叫 `main`。不要为了绕过名字限制改选 Regular，再误认为它带有 Controller 标签。

本次自定义名字的主控使用下面的命令创建，创建后在 Saddle 中选择它显示即可。不要重复创建。

在已核对入口的外部普通终端执行以下命令。模型使用当前 Codex 配置，不借迁移改变模型选择。

```sh
corral start saddle/main-rust \
  --cwd /Users/firegnu/Developer/personal_projs/saddle \
  --label role=controller \
  --prompt '你是 Saddle 项目的新主控，本次是旧 Python Corral 到 Rust Corral 的会话交接。先读取 AGENTS.md、HANDOFF.md 和其中要求优先阅读的文档，再只读核对 git 状态及交接中的未完成事项。本会话 saddle/main-rust 在本次交接中承担原 saddle/main 的主控职责；不要因为名字不同擅自改项目规则。不要修改代码，不要委派，不要部署，不要操作 Tasks 队列，不要关闭任何 agent，不要自行开启或续用旧遥测。读完报告当前状态、未完成事项、需要保留的会话/worktree，以及你是否具备接管所需上下文，等待我正式放行。' \
  -- codex --yolo
```

在 Saddle 左侧 Agents 中选择 `saddle/main-rust` 查看。也可以从普通终端：

```sh
corral attach saddle/main-rust
```

按 `Ctrl-]` 是脱离 attach，不是停止 agent。

**若创建命令超时或结果不明确，先 `corral status saddle/main-rust`，不要直接重发创建。** 若名字已被占用，先确认占用者，不要为了腾名字停止未知会话。

## 5. 确认新主控确实接上了

- [ ] 新主控说出的分支、提交、未提交改动与旧主控交接一致；有差异先查明。
- [ ] 新主控知道当前不做什么：不动用户 agent、不推进真实队列、不复用历史遥测授权。
- [ ] 未完成的实现者/审查者、异步提醒、worktree 都已交代。旧提醒不会因为换了主控名称自动改投新主控，要逐项处理，不能重复派任务。
- [ ] 需要 JEV 时只核对 `TYPESAFE_API_KEY` 是否设置，不输出其值；旧进程独有的环境变量不会因读取 HANDOFF 自动复制到新进程。
- [ ] 当前项目规则可由新主控继续遵守；不要求改写整个 AGENTS.md。

HANDOFF 传递的是工作状态和待办，不是把旧聊天逐字搬进新会话。缺材料时可以回旧会话补充，此时旧主控仍在。

### 5.1 验证关闭 Saddle 不影响新会话

先在外部普通终端记下结果：

```sh
corral where saddle/main-rust
corral status saddle/main-rust
```

- [ ] 记下新会话的 `instance` 和 `agent_pid`，不只记名字。
- [ ] 退出 Saddle 界面，不执行 `corral stop`。
- [ ] 外部终端再次执行上面两条命令：实例和进程身份不变，仍能 `corral attach saddle/main-rust`。
- [ ] `Ctrl-]` 脱离，然后重新打开 Saddle，仍能显示同一个实例。

这里核验的是实际新会话，不代替所有 agent 类型的完整验收。

## 6. 若使用 Tasks，调整以后派发的接收者

不走 Tasks 的口头工作可以跳过这一步；但应记住以后启用 Tasks 前检查接收者。

如果后续主控名字保持 `saddle/main-rust`：

1. 打开 **Tasks**。
2. 进入 **Projects**，选中 Saddle 项目。
3. 打开 **Settings**（`s`）。
4. 将 **Default receiver** 设为 `saddle/main-rust`，可用 **Choose agent** 选择。
5. Save，回看确认。

- [ ] 只改以后派发的默认接收者，不为了交接创建或派发测试任务。
- [ ] 已经运行的任务不会自动改归新会话；旧 run 的会话身份、记录和遥测保持原样。
- [ ] 如果旧主控还有实际未完成的 Tasks run，先完成或明确单独交接方案，不靠修改 Default receiver 冒充已迁移。

## 7. 正式接管，再关闭旧主控

所有核对通过后，给新主控：

```text
交接核对通过，从现在起你是 Saddle 项目的主控。
后续工作只由你接收，继续遵守项目规则。
本消息只确认交接，不授权新开发任务，不操作 Tasks 队列，也不要求遥测。
请等待我下一条任务。
```

- [ ] 旧主控没有尚未处理的工作或收尾动作；需要保存的材料已经落盘。
- [ ] 新主控已完成接续核对，不是只打开了一个空窗口。
- [ ] 从普通终端再核对 `corral status saddle/main`，确认名字和 `instance` 对应计划关闭的旧主控。
- [ ] **由用户明确确认关闭后**，在普通终端执行：

```sh
corral stop saddle/main
```

- [ ] 再检查旧会话已停止，新 `saddle/main-rust` 仍正常。不要把关闭显示 Pane 当成停止旧 agent。

不要让旧主控执行一个会结束自身、却还承诺继续完成后续核验的流程。关闭动作由用户普通终端或已获授权的新主控执行。

## 8. 我希望最后还叫 saddle/main，怎么办？

**本清单默认保留 `saddle/main-rust` 作为新主控，不假设存在会话改名功能。**

若希望最终恢复 `saddle/main`，在旧主控关闭后、正式开始新工作前：

1. 将 `saddle/main-rust` 保留为只读交接会话，确认 HANDOFF 最新。
2. 用第 4 节同样的方法，通过 Rust 入口新建 `saddle/main`，第一条提示词中的名字也相应替换。
3. 新的 `saddle/main` 读取 HANDOFF 并完成第 5 节核对，取得新的 `instance`；不要把它当作旧同名实例。
4. Tasks 的默认接收者若已改成 `saddle/main-rust`，按第 6 节改回 `saddle/main`。已有历史 run 不重写。
5. 用户确认后停止过渡用的 `saddle/main-rust`，之后只由新的 `saddle/main` 接工作。

这会多建一次会话，但能保留原主控名字；不能通过改文件或改软链接把活着的会话重命名。

## 9. 出问题时怎么退回

### 新主控还没接好，旧主控仍在

- 暂停交接，不关闭旧主控，不给两边同时派任务。
- 返回旧主控查看缺失材料，补好后再让新主控读取。
- 如果是新版程序问题，由部署者根据备份成套恢复旧宿主入口、插件注册和必要技能，再核验；不只切回一个 `corral` 链接就声称完全回退。
- 保留新版版本目录：已经创建的 Rust 会话仍可能引用其中的 pen/hook 程序。

### 旧主控已经关闭

- 已关闭的旧进程不能复活。需要时用旧入口另建会话，再通过 HANDOFF 接续。
- 恢复旧入口不会把已有 Rust 会话转换回 Python。
- 新旧互操作已做隔离验证，但不能据此承诺任意现场状态都能无损回退；先检查实际会话与数据，不手改内部状态。

## 10. 最终验收勾选

- [ ] 日常 `saddle` / `corral` 来自同一持久版本目录。
- [ ] 官方插件使用配套版本，显式路径覆盖已逐项核对。
- [ ] 新主控读取交接并报告正确状态。
- [ ] 关闭、重开 Saddle 后，新主控 `instance` / `agent_pid` 保持不变。
- [ ] 外部 `corral attach` 可用。
- [ ] Tasks 默认接收者符合最终主控名称；没有顺手操作 Submit / Accept / Return。
- [ ] 用户批准后才关闭旧主控；其他 agent 保留。
- [ ] 原 Corral 仓库、历史数据、回退备份保留，活跃会话引用的版本目录没有删除。
- [ ] 记录部署版本、新旧会话身份、验收结果和剩余事项；随后才开始下一项工作。

完成记录：

| 项目 | 实际结果 |
| --- | --- |
| 部署版本 / 持久目录 | 待执行 |
| 备份目录 | 待执行 |
| 旧主控名字 / instance | 待核对 |
| 新主控最终名字 / instance | 待创建 |
| 退出、重开与 attach 核验 | 待执行 |
| Tasks 默认接收者 | 待核对 |
| 旧主控关闭时间与用户确认 | 待执行 |
| 未完成事项 | 待核对 |

参考：`docs/Corral核心Rust集成设计.md`、`docs/调研/Corral核心Rust集成-实施核验.md`、`crates/corral-core/README.md`、`plugins/drover/README.md`。
