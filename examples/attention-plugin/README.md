# Attention demo

独立进程插件，演示 Saddle 的持续待处理条目和带目标打开。不读取任务或操作真实队列。

运行 `./package.sh`，在 Saddle Settings → Plugins 添加输出的 `dist/attention-plugin` 目录并 Enable。关闭设置，点击 Attention，选择 Demo item 1/2；插件画面显示选中目标。`u` 更新/恢复两条记录，`w` 撤回全部，Esc 关闭。关闭视图保留进程和来源，停用撤下来源。

需要支持 attention.v1 的 Saddle。SDK 固定到公开 Git 提交；可把整个示例目录复制出仓库独立打包。细节见 Saddle 的插件开发入门和协议 §10。
