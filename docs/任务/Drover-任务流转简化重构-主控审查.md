# Drover 任务流转简化重构：主控审查

2026-09-30；审查提交 `f7e1b5fe0ac4b8c48f5f7a10332477fa0861100c`。

## 当前结论

返工提交 `cb67fe8881d6dce88a1ff4e33dbc8a87542da111` 复审通过，可进入 Saddle schema 2 接入和隔离联调。以下保留首轮问题及证据。尚未合并、发布或推进真实任务；插件化不在本轮范围。

## 必须修复：合法的旧 gate=false + go 历史被当作损坏

- 位置：`bin/drover_core.py` 的 `task_fold`，旧 `done` 和 `go` 解码路径（约 134–145 行）。
- 旧版 `cmd_go` 对 Running 调用 `record_done`，按当时配置写入 `done gate=false`，随后仍追加真实的 `go`。这是旧 CLI 能正常生成的记录，不是人工构造的非法状态。
- 新版先把 `done gate=false` 解为 `done`，随后只允许从 `awaiting_release` 处理 `go`，因此抛出 `invalid acceptance`，公开 list 返回 `state_invalid`。
- 影响：只要历史里出现这种旧版合法用法，整个项目的列表及后续操作均不可用，与保留旧事实的要求冲突。
- 修复边界：在旧事件兼容解码中保留真实 go 及其接受时间；新 `accepted` 事件继续严格要求 Awaiting，不能因此放松新流程规则。不能伪造没有 go 的旧自动完成的接受记录。

## 隔离复现证据

证据目录：`/var/folders/vs/3tm61ygs569g764_td0zxtym0000gn/T/drover-review-legacy-go-_md1rmcp`。

临时状态依次为 `start T1` → `done T1 gate=false` → `go T1`，T2 待办；临时 HOME、配置和 Git 仓库，无真实 agent。

- 基线 `9955469` CLI：`list --json` 退出 0，T1 为 done 且 t2=3。
- 待审新版 CLI：同一数据退出 2，`error.code=state_invalid`。
- 复现结果保存为该目录的 `result.json`，合成记录及旧 CLI 副本一并保留。

## 已执行检查及证据边界

- 主控复跑 `bash tests/drover.sh`：24 项流转、list JSON、1 项文本、10 项通知通过。
- 主控复跑 `bash tests/drover-board.sh`：历史13、正文10、滚动条9及布局、帮助、待办详情通过。
- 主控复跑 `sh tests/install.sh`：11 项隔离安装检查通过；未真实安装或调用 launchctl。
- 用新版公开 `list --json` 只读核对真实 Saddle：current=T57、awaiting=null、pending=11、history=45；真实 Drover 项目亦可读取。未直接读真实内部状态文件，未执行写操作。
- 已审共享状态核心、CLI 转换、通知观察与接口契约。已通过的测试不能覆盖上述新发现，不代表本轮审查通过。
- 尚未做 Saddle schema 2 接入、跨项目联调或发布验证；Corral 和技能未改。

## 下一步

由现有 drover/main 本人先补目标失败检查、修复旧记录解码并验证相关历史/新流转回归。保持原任务边界，不再委派，不发布、不操作真实 T55/T57。提交后复审该缺陷及回归，再推进 Saddle 接入。

## 返工复审（2026-09-30，cb67fe8）

- 公开 status 为 idle，dlog reply 对应本轮兼容修复，分支提交与回复一致。改动仅为旧事件兼容、目标测试及相关记录。
- 兼容限定为旧 `go`、当前旧 done、尚无接受时间且 submission 为 `done gate=false`；保留真实 go 的 t2。新 `accepted`、重复 go/accepted 不走该例外。无 go 的旧完成不生成 t2。
- 主控复跑 `bash tests/drover.sh`：26 项流转、list JSON、1 项文本、10 项通知通过；`python3 tests/board-history.py`：13 项历史检查通过。新回归包含非法新接受与重复接受的拒绝及状态字节不变。
- 主控以原隔离反例再次调用新版公开 list：退出 0，T1 Done、t1=2、真实接受 t2=3，T2 仍 Pending；结果为原证据目录 `rereview-cb67fe8.json`。没有修改真实任务或数据。
- 结论：首轮必须修复项已关闭。保留 Drover 分支/worktree 等待 Saddle 接入与隔离联调，不单独发布，不推进 T57。安装等未改路径不重复验证。
