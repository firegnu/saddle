# Drover 插件

Saddle 的可选任务界面和后台观察器。源码暂放本仓库，通过公开 Rust SDK 接入；任务读写只调用 Drover CLI，Dispatch 详情只调用 dlog。Saddle 宿主无需安装 Drover 也能管理终端和 agent。

在仓库根目录打包：

```sh
CARGO_TARGET_DIR="$HOME/Developer/personal_projs/saddle-worktrees/.target" ./plugins/drover/package.sh
```

在 Saddle → Plugins → Manage plugins 添加生成的完整目录 `plugins/drover/dist/drover-plugin`，启用后关闭管理页，再从 Plugins 选择 Drover。目录中同时需要 `plugin.toml` 和 `bin/saddle-drover`。已登记目录更新前先停用插件，更新后重新启用。

关闭任务面板只关闭视图，进程继续观察已登记项目；停用才停止后台并撤下该来源的 Attention。宿主不会再保留另一份内建 Tasks 或通知来源。插件不启动通知 watch，不自动派发、提交或接受任务。

默认调用 PATH 中的 `drover` 和 `dlog`，读取 `~/.drover/projects` 项目目录。初始项目选启动目录（如已登记），否则第一项；打开时可跟随当前 agent 所在仓库。没有登记项则尝试启动目录，项目选择页可以输入路径。

需要覆盖命令或初始目录时，在打包目录的 `plugin.toml` 顶层（`[view]` 之前）填写：

```toml
args = ["--drover", "~/bin/drover", "--dispatch-log", "dlog", "--cwd", "~/projects/example", "--refresh-ms", "2000"]
```

旧 Saddle `[queue]` 配置保留可读取，但已不生效。自定义值需转到上述插件参数；不自动启用或登记插件。普通用户使用默认值无需填写参数。

任务页面保留添加/编辑/排序/删除、项目与 All pending、详情、Links/Dispatch，以及显式派发和带确认的提交/接受/退回。`N` 打开通知偏好，选择 System/In Saddle 后 `Ctrl-S` 保存；这仍是 Drover 的用户偏好。首次观察和偏好变化建立基线，不为已经等待的任务补弹通知。失败历史的 `Mark seen m` 在插件任务操作中，本次进程内隐藏该条 Attention，不改变任务历史。

Esc 返回当前子页面；列表中 Esc/q 关闭视图。Ctrl-] 总是回到 Agents。关联 agent 通过宿主验证原始 instance 后打开或定位；不会启动新 agent 或覆盖现有终端。
