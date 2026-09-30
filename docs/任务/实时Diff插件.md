# 实时 Diff 插件

用户原话：「只做diff吧」「要能够实时的（延迟一点没关系）当前的diff」「不同文件类型diff显示其实大家也都形成共识了」「好的，那就开始写这个插件吧。」「我理解了。继续吧」。

澄清后的展示：打开即展示当前 worktree 全部改动，按文件连续阅读；文件列表只是定位入口，不要求预先选择文件。自动更新并保持阅读位置。沿用 Git 常见文件类型展示，支持现有居中/tab/split。

主控亲自实施；Dispatch `ec99fea90a2248bfa0ff7ac31bae5ad3`。只读比较，不增加暂存、丢弃或提交动作；不改真实任务队列。

实施检查：临时仓库验证 HEAD/index/worktree 基线和各文件类型；隔离插件进程验证自动刷新、关闭暂停、迟到结果隔离与单实例；验证文本排版与尺寸，运行标准测试和 Clippy。上述为实施检查，不扩写为用户新增需求。

## 实施与主控审查（2026-10-01）

主控亲自完成 `plugins/diff`：Git 只读快照、后台串行轮询、按文件连续阅读、行内改动与语法色、宽屏左右对照/窄屏单列、文件/差异块跳转、横向滚动和阅读锚点。仅增加独立插件及 workspace 成员；宿主运行代码、SDK、协议、Drover 和 Corral 均未修改。没有派发 agent 或修改真实队列。

打开即显示所有文件；文件列表点击只是定位，未加入“选择文件才加载”的流程。来源 cwd 只在明确打开时更新；无来源目录的已有视图切换保持原仓库。关闭停止 Git 检查，失焦仍继续；隐藏 tab 尚保持打开，所以仍检查，这沿用现有宿主生命周期。

主控核对 Git raw NUL 路径、HEAD/index/worktree 基线、未出生分支、未跟踪文件、实际 worktree、重命名/权限/链接、属性二进制、编码与预览预算、冲突和 gitlink。Git 子进程只读且有输出/时间/取消预算，禁用外部助手；语法高亮与行内差异在后台计算，数据未变不重建。未修改进程插件公共接口，不增加新的安装方式。

RED 证据：

- `/tmp/saddle-diff-red.log`：临时仓库应有2个文件，空快照实现实际0个。
- `/tmp/saddle-diff-metadata-red.log`：未跟踪可执行权限缺少100755；gitlink错误地向主仓库读取子模块对象。
- `/tmp/saddle-diff-context-red.log`：无新cwd切回已有视图后，“second”内容消失。

上述已修正并纳入自动回归。插件自身13项检查包含有效协议帧、小尺寸/Unicode、连续多文件、行内强调、锚点保持、外部助手不执行、真实SDK进程自动更新、失焦更新、关闭后实际Git调用次数不再增加、重开新内容以及扫描中切换目录不出现旧结果。

发布包联调：`plugin_real_diff_continuous_live_overlay_split_and_tab` 通过；真实打包插件与隔离Saddle、临时Git仓库/配置和假Corral验证居中打开全部改动、文件点击跳转、实时更新、关闭不动布局、split、新tab移动原pane、模式切换。日志 `/tmp/saddle-diff-host-final.log`，隔离画面 `/tmp/saddle-diff-overlay.txt`。没有使用或重启用户窗口。

插件包 SHA-256：`21db815d69db57be88b12fe2c16187aeb69e6b307681586d88abbd1737326f9f`。打包脚本只输出插件目录，不登记启用。最终目录为主仓库 `plugins/diff/dist/diff-plugin`；不替换日常Saddle二进制，用户可直接添加，无需重启。

最终标准检查：`cargo test --all-targets -- --test-threads=4` **353 passed / 0 failed / 5 ignored**；Clippy `--all-targets -- -D warnings`、fmt、diff检查通过。日志 `/tmp/saddle-diff-all-complete.log` 与 `/tmp/saddle-diff-clippy-complete.log`。5项忽略检查包括既有4项及本次需要独立发布包的1项，后者已按上述方式单独运行通过。

期间最终回归曾暴露测试等待条件不正确：SDK省略相同帧，“无新cwd保留原内容”后不会再发送相同画面。已将检查改为切换显示布局后核实内容仍存在，完整重跑通过；没有修改SDK去制造多余刷新。旧失败日志保留 `/tmp/saddle-diff-all-final.log`。
