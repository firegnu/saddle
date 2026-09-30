# Clawd再缩小与短腿步态

用户原话：个人觉得啊。再小一号，另外左右跑的时候动作调整一下，像八爪鱼似的。

主控亲自完成，不委派。Dispatch `c2775ff6e5624f068ec6dfc106a37d65`。

仅调整尺寸与动画姿态：7列×2行，底部对齐tab空白区；四短腿交替抬脚，不左右甩腿，停顿落地。保留配色和现有状态/输入/业务逻辑，不操作真实agent/队列/插件/服务。纯视觉变更，以帧渲染、既有状态和输入隔离检查验证，不制造RED。

## 主控审查

- 生产变更只在 `src/mascot.rs`：7×4像素、底部对齐、短脚交替抬起，适配眼睛/手臂坐标。移动速度与范围、状态来源、tab/pane几何、键鼠输入、业务代码未改。
- 帧检查确认走路时上身不变、脚只留在原四个位置、停顿四脚落地；既有颜色/状态/拥挤隐藏检查通过。纯视觉变更未制造RED。
- 实际Ratatui帧导出 `/tmp/saddle-clawd-tiny-layout.png`，紧靠pane边框，不增加tab行高；黑眼RGB在输出cell中核实。隔离release真实CLI动画与点击不传入agent检查1项通过，日志 `/tmp/saddle-clawd-tiny-release.log`。非用户窗口截图，没有重启其Saddle。
- 发布候选SHA256 `7bdb229ad9799814c6ec37a6c4d95173e0b7d176051aa9cbad4ff1aa68896642`。三处Diff演示文件hash一致，真实agent/任务/配置/插件/服务未操作。

- 最终标准检查 `cargo test --all-targets -- --test-threads=4`：368 passed / 0 failed / 5 ignored；Clippy、fmt、diff检查通过。日志 `/tmp/saddle-clawd-tiny-{all,clippy,target,release}.log`。主控审查通过，合并推送并备份更新日常宿主，用户重启后观察。
