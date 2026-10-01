# 会话交接

更新：2026-10-01。当前 main。核心遥测阶段 01「存储与查询」已审查、合并、推送并清理；用户已授权继续阶段 02；02A 执行采集候选已通过独立复审，02B 设置界面正在串行集成。本轮不发布、不安装、不读写真实遥测/队列。

## 1. 会话摘要

dispatch-log 的需求已收敛为 Saddle 核心遥测，完成三轮 Claude 独立设计审查后实施首阶段。当前只交付存储、纯记录、查询及存储侧开关；整条执行链路尚未接入。

## 2. 完成的工作

- 设计基线 5360933；首阶段实现 9fb8981，返工 61014a0；合并 465c4fb 已推送，空提交收尾 eb82212。本交接及计划状态随后提交推送。
- 新增 src/telemetry/、SQLite 八表与完整正文文件、headless telemetry CLI、不可变证据与来源标记、两级开关及 generation。使用方法见 docs/遥测使用.md。
- 独立审查发现“已有 end 错清未知结果”一项阻断，原实现者修复后原审查者明确可以合并，无新增问题。详情和取舍保存在任务及独立审查记录。
- 初版主控标准检查：399 passed / 0 failed / 5 ignored，clippy 通过。返工后实现者相关遥测 26 项通过，主控定向 4 项、独立复审 6 项通过；没有重复全量标准套件，不宣称真实链路或掉电验证。
- telemetry-storage-query 分支、实施 worktree 与 review-telemetry-storage-query 已安全删除；对应 saddle/dev-telemetry-storage-1、saddle/dev-telemetry-storage-review-1 已在 idle/attached=0、目录清理后关闭。
- Claude 设计审查会话 saddle/dev-telemetry-design-review-1 已按用户后续要求关闭；设计审查 worktree 保留。

## 3. 待完成与现场状态

- 阶段 01 无已知未关闭阻断。阶段 02A 候选已通过审查、尚未合并；02B正在实施，03–05尚未实施：执行采集/设置入口 → 同进程 JEV 路由 → 现有 Drover 插件接入 → 消费者切换和退役外部 dlog。不要把存储完成说成整套遥测完成。
- 本轮未构建 release、安装、切换消费者或初始化真实遥测目录；日常程序仍为此前 Clawd Opus 版，链接 ~/.local/bin/saddle 指向共享 .target/release/saddle。后续 release 构建前必须按既有规则备份，当前不构建。
- 保留 ../saddle-worktrees/review-telemetry-design（detached 5ddd544，审查快照），t38-dispatch-study、t55-notification-flow；不要自动清理。
- 新实施者 saddle/dev-telemetry-agent-1（instance 0f297a0ff859，Codex gpt-6-astra/xhigh），分支 telemetry-agent-capture，worktree ../saddle-worktrees/telemetry-agent-capture，基线 951b13e。另有 corral/main、dispatchlog/main、saddle/main；前两个是用户会话，勿送话/关闭。
- 旧独立 Drover 已退役，现用 plugins/drover；历史数据保留，不恢复服务。此前安装包仍带外部 --dispatch-log 配置，本阶段未切换；旧 dlog 仍用于开发派发记录。

## 4. 约束与决定入口

设计和理由以 docs/DESIGN.md 最新遥测章节及 docs/任务遥测接口契约.md 为准，不重开已批准架构。Corral 无遥测依赖，宿主不反向依赖插件；用户批准的默认值已写入设计。本阶段不做旧日志导入或真实业务操作。

主控不写功能代码；后续按 AGENTS.md、corral-dispatch 技能及 ../dispatch-log/USAGE.md 分派。所有新轮次重新挂提醒。测试隔离 HOME/状态目录且共享 CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target；任务登记、放行、下一项分开，不自动推进真实队列。

## 5. 优先阅读

1. docs/任务遥测实施计划.md、docs/任务遥测接口契约.md、docs/DESIGN.md 最新章节。
2. docs/任务/遥测01-存储与查询.md、docs/任务/遥测01-独立交叉审查.md、docs/遥测使用.md。
3. docs/调研/Saddle遥测设计-Claude审查-2026-10-01.md、docs/调研/Saddle任务遥测数据模型与SQLite存储-2026-10-01.md。
4. AGENTS.md、../dispatch-log/USAGE.md。

## 6. 下一步

02A候选ae77967经主控和原独立审查者确认M1关闭：stderr字节0宿主起始与末行final按本次call_id匹配，缺失/截断/不匹配保持未知。主控首版标准423通过5忽略/clippy通过；修正后主控4项、独立23项通过，没有重复全套。S1发送参数换序建议未纳入，历史两项未改动测试失败未查明且主控未复现，记录保留。

当前02B实施者 saddle/dev-telemetry-settings-1（instance f8977ca3444b，Claude opus[1m]/high），dispatch 52441b99bb8e4d96955de374d94f65b5；任务 docs/任务/遥测02B-设置总开关.md。分支telemetry-settings、worktree ../saddle-worktrees/telemetry-settings，基线8378fe9（ae77967加主控文档main）。只在现有General加Store总开关，使用草稿Save/Cancel，不另存config开关；需核对保存失败/外部修改和假Corral集成。JEV三项拿不准，主控判常规/交叉审查要/碰要害。

完成提醒后先status，再经dlog reply读完整DONE；主控核对及标准测试后另开重档Codex审查02B/集成。02A实施者saddle/dev-telemetry-agent-1及审查者saddle/dev-telemetry-agent-review-1、对应两个worktree和分支保持，整个02集成通过才合并清理，不提前清02A。每轮重挂提醒，主控不写功能代码；没有真实队列操作。

用户最新硬边界：如需改 Corral 或发现依赖反转，先停止相关工作告知用户，不得先改后报。独立遥测查询界面明确归 Saddle，在阶段04实现，Drover仅提供带关联条件的快捷入口；旧视图/旧dlog依赖到阶段05再切换。主控发现Corral公开at为数值、现有Saddle存储按字符串校验，已让实施者在Saddle内按原样保存契约最小纠正，不改Corral。

开发记录：实现 dispatch 4072770b1160441ca745b8ce78c7e490，独立审查 f514aac2b3de4f13a3cfa515df380147；设计审查 de1a9428bcfd448ea0c628b78f480fc5。收尾实际结果写入 dlog，各次审查原文已落主仓库，不依赖已关闭会话。
