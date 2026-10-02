# Corral Rust 核心部署记录

## 准备完成，尚未切换

用户同意按新旧主控交接清单推进，先准备包和备份；待用户退出 Saddle 界面后再切换。现有主控继续运行，本轮未记录遥测、未操作 Tasks。

- 源码：`7755bb87336de0cca1429a25eea7ca716b445e40`，打包时工作区干净，已包含宠物合并 `0761aeb`。
- 固定版本目录：`/Users/firegnu/.local/share/saddle/versions/7755bb8`。
- 私有备份（0700）：`/Users/firegnu/.local/share/saddle/backups/corral-rust-7755bb8-20261003-021136`。包括旧入口链接及宿主文件、Python 启动脚本副本、完整 Drover/Diff 包、Saddle 配置/插件注册、两种 agent 的 Corral/Dispatch 技能和资源所有权记录。20 个文件校验通过；没有复制或改写真实遥测数据库/队列，原 Corral 仓库保持不动。
- 日志：`/tmp/saddle-rust-deploy-7755bb8/`。部署前再次核对备份与当前安装一致，避免覆盖准备期间用户修改。

## 本轮验证

- 宠物遗留两项 workflow 测试在显式 `aarch64-apple-darwin` target 下串行复核，2 passed：`native_mouse_buttons_cover_forms_and_stop_confirmation`、`pending_delete_button_confirms_names_the_task_and_can_be_cancelled`。日志 `workflow-two.log`。不改写原失败，不认定其原因为并行构建覆盖。
- `scripts/package.sh` 构建最新成套 release 包成功，未改全局链接/注册。四个程序 SHA256 与 BUILD.txt 一致，宿主/Corral 为 macOS arm64 Mach-O。
- 使用该包宿主和核心运行隔离 product 检查，1 passed：未开界面先创建合成 agent、退出并重开界面仍为同 instance/PID、遥测列表为空。日志 `product.log`。这不是实际 Claude/Codex 产品验收。
- Corral `install-skills --dry-run`：Claude/Codex 两处均 `same`，written=false，无需更新技能；没有写用户技能。
- 本轮不重复全套/clippy，不修改功能代码。

| 程序 | SHA256 |
| --- | --- |
| bin/saddle | 265aa7d5f3c887f25d0dee5a42c9efef50aca2381788b05be00c3c9038b0aee8 |
| bin/corral | 6c52fe4cc4e0f521e3921ff822582bbf82bc5e56e195c294716cad00741d6dfa |
| plugins/drover/bin/saddle-drover | 9a3b138b2d60abe6d032cf6f1d16e26a2088be72fe49e70e73c84798cda38256 |
| plugins/diff/bin/saddle-diff | d661e34cf2b7f7523216d67ba7070bfe48d0ed076450875440b03b44ba2a82ba |

## 待切换

1. 用户退出 Saddle 界面后，公开 ctl 和精确进程检查确认已退出；不要停 Corral 主控。
2. 复核配置/注册/技能与备份未变，切换 `~/.local/bin/saddle` 与 `corral` 至新版本的相邻程序；只更新 Drover/Diff 注册路径，保留其他插件及启用状态。配置 corral 当前为默认值 `corral`，两份旧插件 manifest 无显式 `--corral` 覆盖（实际切换前再核）。
3. 两处 Corral 技能同版，不做无意义覆盖；Dispatch 技能保持原有资源所有权。
4. 通知用户重开，核宿主、插件和原会话可用。然后另按交接清单创建新主控接续，未经用户确认不关闭旧 `saddle/main`。

准备时公开实例为 `693d73ccc278e385`，显示旧 `saddle/main` instance `4ebbecf235f8`；执行时重新查询，不依赖旧实例号定位。真实切换/新主控接续均未执行。
