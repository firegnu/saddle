# Clawd小号与无占位布局

用户原话：就是有一个问题是他把tab给撑上去了。而且有点大，不可爱

主控亲自完成，不委派。Dispatch `502c463cff7340afb34d55c5bc322bd8`。

范围：恢复终端原高度；小号Clawd放在tab右侧空闲区域，空间不足隐藏。保留参考配色和轮廓，以及已有状态绑定/输入/任务/插件行为。只改UI/UX，不操作真实agent/队列/配置；保留三处Diff演示。

验证：不再占用终端高度、与tab控件无重叠、tab拥挤隐藏，既有状态绑定和鼠标不传入终端检查；标准检查与隔离发布检查后更新日常宿主，不重启用户窗口。

## 主控审查

- 仅改三个生产文件：mascot帧/尺寸/姿态速度，terminals绘制区域与恢复原始pane几何，ui传入已经绘制的控件边界；未改App、状态判断、输入、Corral、Viewer/PTY实现或插件业务。
- 有效RED：期望pane仍从第3行开始，原版从第7行开始，明确多占4行；改后通过。新增tab拥挤时隐藏且保留tab控件的检查。标签绘制及点击算法未变，装饰只取其右侧余量，不预留宽度。
- 图形由11×8缩为9×6（终端9列×3行），保留原橘/黑眼/四腿，姿态由8改为6帧/秒。查看实际Ratatui渲染导出的 `/tmp/saddle-clawd-compact-layout.png`，确认顶部tab与小号Clawd同排，紧靠下方pane边框，没有额外高度；这是隔离渲染，不是用户窗口截图。
- 目标检查通过；已有黑眼检测改为同时检查半格字符的fg/bg，原尺寸与原带坐标断言更新到无占位布局。隔离PTY真实开发版和release上动画/等待问号/鼠标不发给agent通过，日志 `/tmp/saddle-clawd-compact-{red,target,flow,release}.log`。
- 真实配置、插件包、agent、队列和服务未操作；三处Diff演示hash不变。

- 首轮完整检查有一项既有移动workflow失败：移动回上下分屏后，测试只看到pane标题就断言原输出；失败画面仍残留旧picker和旧输入目标，属于尚未读完PTY重画。补齐“Input ▸ p/a”与原输出等待，保持原输出/布局/未重复attach等断言，目标复查通过。未改会话移动实现。失败证据 `/tmp/saddle-clawd-compact-all.log`，复查 `/tmp/saddle-clawd-compact-move.log`。

- 最终标准检查通过：`cargo test --all-targets -- --test-threads=4` 共368 passed / 0 failed / 5 ignored；`cargo clippy --all-targets -- -D warnings` 通过。日志 `/tmp/saddle-clawd-compact-all-final.log`、`/tmp/saddle-clawd-compact-clippy-final.log`。隔离release主路径额外1项通过。
- 发布候选SHA256：`8ad60e7a9eb6b27a0bd75a35712ccebacef6a882f10ec5c723686bb76edb7b14`。主控审查通过，按现有交付方式合并推送、备份后更新日常宿主；不代用户重启。
