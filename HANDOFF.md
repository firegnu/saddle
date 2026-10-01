# 会话交接

更新：2026-10-01。当前 main。核心遥测阶段 01「存储与查询」已审查、合并、推送并清理；下一阶段是执行采集与总开关入口，尚未派发。本轮不发布、不安装、不读写真实遥测/队列。

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

- 阶段 01 无已知未关闭阻断。阶段 02–05 尚未实施：执行采集/设置入口 → 同进程 JEV 路由 → 现有 Drover 插件接入 → 消费者切换和退役外部 dlog。不要把存储完成说成整套遥测完成。
- 本轮未构建 release、安装、切换消费者或初始化真实遥测目录；日常程序仍为此前 Clawd Opus 版，链接 ~/.local/bin/saddle 指向共享 .target/release/saddle。后续 release 构建前必须按既有规则备份，当前不构建。
- 保留 ../saddle-worktrees/review-telemetry-design（detached 5ddd544，审查快照），t38-dispatch-study、t55-notification-flow；不要自动清理。
- corral ls 当前仅 corral/main、dispatchlog/main、saddle/main；前两个是用户会话，勿送话/关闭。
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

汇报阶段 01 已收尾；继续时按实施计划准备阶段 02 的限定任务书与派发，不提前启动阶段 03–05。本轮没有创建下一阶段 agent、分支或真实任务。

开发记录：实现 dispatch 4072770b1160441ca745b8ce78c7e490，独立审查 f514aac2b3de4f13a3cfa515df380147；设计审查 de1a9428bcfd448ea0c628b78f480fc5。收尾实际结果写入 dlog，各次审查原文已落主仓库，不依赖已关闭会话。
