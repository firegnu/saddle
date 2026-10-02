# Theme第一次集中返工

交给原saddle/dev-theme-1（instance e641723be7ef），继续theme-presets，基线候选5c06809。
类型：Bug修复。依据：主控核对一个配置注释保存缺陷，以及标准验证暴露的旧UI测试假设。不重新路由，原Claude opus[1m]/high，限定原任务直接修正。
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的实现者，不再委派，无需采集遥测。

## 先读
主仓库 /Users/firegnu/Developer/personal_projs/saddle/docs/任务/UI主题-主控核对.md 末两节，以及原任务已批准的设计。
主控结果 /var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/saddle-theme-controller-hek2ddkh/test.log 与results.json。

## 修正范围
1. M1：删除[colors]最后一个覆盖键，不能丢独立注释。切主题清覆盖/Default是正常操作；保留表内独立说明即使表空，删除键本行备注可随键删除。最小修remove及直接目标，不扩TOML编辑框架。先目标有效RED再GREEN。
2. 主控标准542过12败5忽略，3项plugin_resources、9项workflow逐项旧断言已核。仅在tests/plugin_resources.rs和tests/workflow.rs这12个失败用例及其必要的局部辅助中适配已批准UI：旧标题边框/旧管理页文案、名称截断、选中符、列表详情分栏/滚动、等待条件。保留原业务断言与交互覆盖；不能删除/跳过检查或只把断言放宽成空泛成功，不增超时。不得改src/plugins、Drover或其他生产逻辑来凑测试通过。若跟到真实生产问题，停该项报告，不自动扩大。
3. 修完成记录/DESIGN的注释丢失限制；Tide保持default背景/正文取舍主控接受，不再改配色。用户未要求每主题独立保存覆盖，不新增。
4. 补充原RED/GREEN及标准日志的已有路径和关键结果；若没保存原始日志如实说明，不重跑旧候选伪造历史证据。

## 验证预算
- M1一个直接目标RED/GREEN与settings相关回归一次。
- 只重跑上面12个原失败用例（精确名称）；修订中失败只再跑修到的项，不重复全套、Clippy、构建或无关检查。
- git diff --check。把命令与结果写入新增完成记录，不声称原全套绿；不能再擅自重复全套。
- 所有cargo命令CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target。

## 交付
不改真实配置/队列/数据，不安装release、不改Corral/反向依赖，不合并不推送，不关用户agent。只在原worktree提交；在分支的原实施任务末追加本次完成记录（不要写主仓库本文件）。保留原失败和预算超限事实。命令前台完成，最后一行DONE。
