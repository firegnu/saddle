# Clawd忙碌与等候微动作

用户原话：还有就是working状态腿还是要动动，不然感觉都是静止的。但是不能像之前那种八爪鱼，要像很忙碌的那种动画。但是不能乱。这个尺度你来把握。waiting也最好有点动画。停着不太好。

主控亲自完成，不委派。Dispatch `04f2f51e9de448619b8ca7d3b991719e`。只增加Working有规律的原地小步、Waiting适度眨眼/示意；四细腿和已认可的外形、颜色、尺寸、布局、Idle及配置开关不变。不操作真实agent/任务/配置/插件/服务。

## 主控审查

- 生产代码仅增加Working半秒节拍内脚小步、Waiting四秒周期眨眼与微挥手。像素函数之前全部代码（尺寸、颜色、轮廓、绘制、移动）以及Idle分支与上版逐字一致；配置开关、状态来源和业务未改。
- 有效RED确认Working脚部不动；GREEN覆盖原地步伐、Waiting四秒内有动作、固定身体/外侧脚及四细腿。全部状态跨96帧检查眼睛、四腿分离和每cell至多两色，Working上身与Waiting仍至少四分之三时间静止。目标5项通过。日志 `/tmp/saddle-clawd-busy-red.log`、`/tmp/saddle-clawd-busy-green.log`。
- 隔离发布版开关/显示/取消/输入与布局检查1项通过，日志 `/tmp/saddle-clawd-busy-release.log`。发布候选SHA256 `71791e53b4fb94e39c067c2a59b756475f7afdf323a25b7b9032cfc493c9d2f9`。
- 三处Diff演示hash一致；真实配置/agent/插件/队列/服务未操作，未重启用户Saddle。

- 最终 `cargo test --all-targets -- --test-threads=4`：374 passed / 0 failed / 5 ignored；Clippy、fmt、diff检查通过。日志 `/tmp/saddle-clawd-busy-{all,clippy,green,release}.log`。主控审查通过，正常合并/推送/清理，备份更新日常宿主；用户重启观察。
