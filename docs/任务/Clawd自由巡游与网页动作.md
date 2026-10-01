# Clawd 自由巡游与网页动作

主控亲自实现；不委派。分支 `clawd-free-roam`。

## 用户要求与验收原话

> 你能复刻这个大小和这种动作吗？我觉得这些动作+来回walking基本就满足我的要求了。状态什么的就不用对了。

> 就是这个意思

> 按这些边界验证

> 我在想能否保持现在的位置，但是可以横跨agent区域，然后agent上部分的border适当减弱色彩。这样不会给人感觉把身体分割了。如何？

> 我可以接受

## 执行记录

Dispatch：`2fc3de7c1ae74a84878685a1db8de8b4`。参考提取目录在仓库外 `~/Downloads/clawd-reference-20261001-4pdhpd0_`。不触碰真实 agent/队列。

测试边界已确认：现有公开绘制入口，检查 walking/动作切换、状态独立性和不覆盖 tab。

用户后续决定覆盖增高方案：保持原布局，采用越过上边框的浮层。未合并、安装增高方案。

## 实现与主控审查

- 保留现有 tab 与 pane/PTY 几何。38 个网页素材片段转换为内置四分块帧；自主往返、随机动作，不读取 agent 状态。动画向下越过原上边框，线条降为 dim，保护 tab/pane 控件与浮层点击。
- 参考身体约59×45 CSS像素；终端近似8列×3行，画布18列×7行，随字体改变物理尺寸。小于网格的细节无法逐像素复现；全部素材均已纳入，未删减高动作。
- 原公开入口的状态独立测试先失败后通过（`/tmp/saddle-clawd-roam-{red,green}.log`）；道具动作检查先失败后通过（`/tmp/saddle-clawd-actions-{red,green}.log`）；最终浮层几何检查先失败后通过（`/tmp/saddle-clawd-overlay-{red,green}.log`）。中间增高方案已撤回，不作为最终验收证据。
- 三项变异检查分别禁用移动、控件避让、鼠标保护，均由目标断言检出，随后恢复实现。日志 `/tmp/saddle-clawd-mutation-{motion,protection,mouse}.log`。
- 最终标准检查 `cargo test --all-targets`：**372 passed / 0 failed / 5 ignored**；`cargo clippy --all-targets -- -D warnings`、fmt、diff 检查通过。日志 `/tmp/saddle-clawd-{all,clippy}-final.log`。流程测试默认关闭装饰，避免已获接受的覆盖影响文本断言；专用动画/开关流程显式打开。动画流程等待列表加载并明确选择假 agent，避免启动竞态。
- 发布版通过跨边框/鼠标不穿透和设置保存/取消/布局检查（2项）；全程临时 HOME、假 corral，无真实 agent/队列操作。日志 `/tmp/saddle-clawd-release-{overlay,settings}.log`。
- 已检查全部38种素材的联系图，以及实际 Rust Buffer 导出的30秒动画。预览 `~/Downloads/clawd-reference-20261001-4pdhpd0_/saddle-overlay-preview.{png,gif}` 是 Buffer 栅格化预览，不是用户窗口截图；没有声称实际用户窗口已加载新版。
- 主控审查通过：没有新增高度、改变终端尺寸或残留临时预览程序，没有新增运行时依赖；本轮无委派、无现有 agent 操作。

## 安装记录

旧宿主备份：`~/Library/Application Support/saddle-release-backups/clawd-roam-20261001-112416/`，SHA256 `71791e53b4fb94e39c067c2a59b756475f7afdf323a25b7b9032cfc493c9d2f9`。

日常链接仍为 `~/.local/bin/saddle` → 共享 target 的 `release/saddle`。发布版 SHA256 `d2ebb2f549167413324afedff7aac41207e3be97302c7d3c289f0dd0e4fafb16`。未重启用户窗口；下次启动加载新版本。
