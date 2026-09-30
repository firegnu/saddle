# Counter demo

独立进程插件。只依赖公开 `saddle-plugin-sdk`，不导入 Saddle 私有模块。

构建（与宿主使用同一个 target，但不是把 demo 编进宿主）：

```sh
CARGO_TARGET_DIR="$HOME/Developer/personal_projs/saddle-worktrees/.target" cargo build --locked --manifest-path examples/counter-plugin/Cargo.toml
```

把本目录的 `plugin.toml` 和构建出的 `saddle-counter` 放进一个长期保留的目录，后者放在 `bin/saddle-counter`。也可以创建该相对路径的符号链接，指向自己的构建目录；移动/清理构建产物会使插件不可用。

在开发版 Saddle 中打开 Settings → Plugins F5 → Add local，输入那个目录，Read manifest → Add disabled → Enable → Open panel。登记只记目录，不复制文件或执行安装脚本。启用的插件以当前用户权限运行。

Enter 或点击 Increment：计数增加并请求内部通知。Shift+Enter 不激活按钮。关闭面板保留计数，停用或重启进程归零；Remove 只删登记。限流时计数继续增加并显示提示。此 demo 不写业务文件，不执行命令，不访问网络或操作 agent/任务。

SDK 作者入口为 `run(factory)`；把初始化代码放进 factory，确保在 stdout/stdin 重定向之后执行。通过 Context::redraw 请求绘制，Context::notify 请求通知。Ratatui buffer 由 SDK 转成完整字符帧；普通 println 输出进入日志。Event::Tick 可消费自己后台线程的有界结果队列，长工作不能阻塞 event/render。插件负责在 Drop 中取消自己启动的工作；SDK 不替业务代码管理任意线程/孙进程。

复制此独立项目到仓库外后，可在副本目录运行 `cargo build --locked`（同样设置 CARGO_TARGET_DIR）；固定 Git revision 的 SDK 依赖仍可构建。首版是已验证 demo 的开发接口，尚未发布 crates.io 或承诺稳定 1.0；本机需 Rust stable 1.96 或以上。
