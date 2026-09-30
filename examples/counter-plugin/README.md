# Counter demo

独立进程插件。只依赖公开 `saddle-plugin-sdk`，不导入 Saddle 私有模块。

在 Saddle 仓库根目录运行一条命令（需 Rust stable 1.96 或以上）：

```sh
./examples/counter-plugin/package.sh
```

产物在 `examples/counter-plugin/dist/counter-plugin/`，包含 `plugin.toml` 和 `bin/saddle-counter`。也可传一个输出目录：`./examples/counter-plugin/package.sh /path/to/counter-plugin`。输出是本机平台的 release 程序，不能直接跨操作系统或 CPU 架构使用。脚本不会安装到全局 PATH，也不会启动 Saddle 或登记插件。可通过 `CARGO_TARGET_DIR` 共用已有构建缓存；本项目开发检查使用 `$HOME/Developer/personal_projs/saddle-worktrees/.target`。

在支持直接入口能力的 Saddle 中打开 Settings → Plugins F5 → Add local，输入输出目录，Read manifest → Add disabled → Enable；关闭设置后直接点击左侧 Counter（或在 Agents 中 F6、Enter）。使用者拿到整个产物目录即可，不需要 Rust 或开发环境变量。登记只记目录，不复制文件或执行安装脚本，因此登记后保留该目录；可先移到长期保留的位置再添加。启用的插件以当前用户权限运行。

再次执行脚本会覆盖输出目录中的示例程序和清单。更新已登记的目录前先 Disable，打包成功后再 Enable；本脚本不是后台更新器。开发可从 [插件开发入门](https://github.com/firegnu/saddle/blob/main/docs/插件开发入门.md) 开始。

Enter 或点击 Increment：计数增加并请求内部通知。Shift+Enter 不激活按钮。关闭面板保留计数，停用或重启进程归零；Remove 只删登记。限流时计数继续增加并显示提示。此 demo 不写业务文件，不执行命令，不访问网络或操作 agent/任务。

SDK 作者入口为 `run(factory)`；把初始化代码放进 factory，确保在 stdout/stdin 重定向之后执行。通过 Context::redraw 请求绘制，Context::notify 请求通知。Ratatui buffer 由 SDK 转成完整字符帧；普通 println 输出进入日志。Event::Tick 可消费自己后台线程的有界结果队列，长工作不能阻塞 event/render。插件负责在 Drop 中取消自己启动的工作；SDK 不替业务代码管理任意线程/孙进程。

复制此独立项目到仓库外后，在副本目录运行 `./package.sh` 即可交付；固定 Git revision 的 SDK 依赖仍可构建，无需 Saddle 源码。首次构建需要下载依赖。首版是已验证 demo 的开发接口，尚未发布 crates.io 或承诺稳定 1.0。

新版清单默认居中弹窗：Esc 关闭并恢复来源焦点，Ctrl-] 关闭并返回 Agents。点击左侧 Counter 重开时计数保留。需要工作区 tab 时，在停用后把 plugin.toml 的 view.placement 改为 workspace，再启用；同一插件只保留一个逻辑视图，已有工作区面板优先聚焦。旧宿主会明确拒绝新能力；使用同版 Saddle 与打包产物。
