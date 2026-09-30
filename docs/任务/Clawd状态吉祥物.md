# Clawd 状态吉祥物

用户原话：在右侧agent最上面那根border的上方，放一个claude code的吉祥物，可以是像素的；根据当前agent状态切换动画，idle左右走来走去。

用户确认：可以可以，先做出来看看。这玩意还有宠物引擎？

补充：这个吉祥物一定要像哦。还有颜色。

主控亲自完成，不委派。Dispatch `de272393c79b4e83abae5f4d46276225`。

范围：宿主装饰动画，单只跟随当前焦点pane的agent；复用现有状态判断，不操作agent、任务、输入或插件生命周期。原图参考 https://www.stickermule.com/claudecode/item/19156131 ，保留陶土橘、黑色双眼、两侧短臂和四条腿。半格像素保留11×8比例，占4行；tab下方单独留带，小窗口隐藏，普通终端/插件pane不显示。不引入宠物引擎或配置系统。

检查：先证明agent上方缺少独立动画带；动画帧/状态绑定/分屏与终端几何、完整标准检查和隔离发布流程。三处Diff演示保留。仅更新日常宿主，不重启用户窗口。

## 主控审查

- 形象：对照Clawd原图读取像素RGB `#D97757`，沿用11×8轮廓、黑色双眼、两侧短臂与四腿，不采用贴纸白色裁切边。sprite用半格方块绘制，身体不因agent状态变色；仅低色终端映射邻近256色。实际Ratatui缓冲导出的像素预览 `/tmp/saddle-clawd-preview.png`，已查看核对；这不是用户窗口截图。
- 状态：由 `Panel::status` 读取，不另造blocked/stalled等业务判据；以正在显示的pane为目标，不用左侧选中项；实例变化重置动画。绘制状态只在App内存中，无线程、命令、数据或配置读写。`src/agents.rs`、input、corral、viewer、pty和所有插件代码未改。
- 布局：共享 `Terminals::rects` 扣除动画带，PTY尺寸/渲染/鼠标定位共同使用；tab区域不变。小窗口、空pane及插件不预留动画带；鼠标不添加吉祥物target。
- 有效RED：已显示agent的pane仍从第3行开始，没有4行装饰区。修复后该检查及既有terminals/UI回归通过；另检查慢走反弹/原地状态切换/隐藏恢复/实例切换、原图特征、颜色、焦点绑定与小窗口。
- 真实开发版隔离PTY验证：idle位置变化、blocked问号与原色、点击动画不发入PTY、正常键盘输入和退出不stop agent均通过。全部假Corral/临时配置，没有操作真实agent。日志 `/tmp/saddle-clawd-flow.log`。
- 首次标准检查中两项workflow失败已核对：旧固定鼠标坐标和resize期望需计入4行，改为定位内容/期望106×34；移动检查在部分重画时提前判断，画面证据仍残留picker，改为等输入目标及原输出到齐，不改会话移动实现。目标复查均通过，日志 `/tmp/saddle-clawd-{all,move-detail,move-final,input-final}.log`。补充两项Clippy样式修正，临时预览程序的类型/借用问题已修正且临时example移出仓库，不计作产品缺陷。

最终检查：`cargo test --all-targets -- --test-threads=4` **367 passed / 0 failed / 5 ignored**；Clippy/fmt/diff通过。日志 `/tmp/saddle-clawd-all-final.log`、`/tmp/saddle-clawd-clippy-final.log`。发布版动画/装饰区不传输入与modified Enter/粘贴回归2项通过，日志 `/tmp/saddle-clawd-release-check.log`。release候选SHA-256 `7983c23351ed092f64f3bd60af2b9b119167ba75f5f1b789d489df476ab8fd64`。

Approve。按既定流程合并推送、备份后原子更新日常宿主。不重启用户窗口，真实配置/插件包/agent/队列/服务和三处Diff演示均不变。用户重启后观察动画和颜色。
