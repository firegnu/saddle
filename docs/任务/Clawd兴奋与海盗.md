# Clawd 兴奋与海盗

主控亲自实现，不委派。Dispatch `37d50d80c3ec43df8229d86b252b66b5`。

## 用户原话

> 现在有兴奋这个动作吗？没有的话就加上。另外加一个额外的海盗的那种形象动画（就是那种经典的海盗形象，有一个从左向右的眼罩的那种，动作你自己设计一下），最好这两个不要返工。

## 实现范围

当前精选12段没有兴奋。新增兴奋和海盗，使总数为14。兴奋采用笑眼、双手欢呼、脚步节拍与短暂闪光；海盗采用红头巾、单眼眼罩、从左到右与眼罩相连的细带，扶眼罩后抬手致意、眨眼。沿用已有普通字符绘制，不调查/安装字体。

保持已认可的静止轮廓、尺寸、基线、3行边界和既有12段帧数据。新动作包含进入、保持、退出姿态，避免突然切换和额头缺口。先检查每个关键姿态，再检查实际巡游衔接及回归。

沿用已确认的公开Mascot.draw绘制入口检查新动作参与自主巡游、眼罩/眼睛和边框约束；纯视觉表现通过关键姿态与动画预览观察。

## 验证与审查

- 两个新增公开绘制检查先因缺少动作而失败，再通过；日志 `/tmp/saddle-clawd-excited-red.log`、`/tmp/saddle-clawd-pirate-red.log`、`/tmp/saddle-clawd-excited-pirate-green.log`。最终海盗检查验证细带与眼罩直接相连、另一只眼可见、额头完整。
- 全量 `cargo test --all-targets`：377 passed / 0 failed / 5 ignored；Clippy、fmt、diff 检查通过。日志 `/tmp/saddle-clawd-excited-pirate-{all,clippy}.log`。
- 对比基线 86fc1ef，原12段帧数据和原调色板逐字节不变；新增兴奋39帧、海盗56帧，总14段517帧。运行时只增加两个普通字符映射。
- 已逐张观察欢呼4个、海盗8个关键姿态；初版斜线容易像划痕，最终采用与眼罩相连的横向细带。新动作维持16列×3行，未改布局和状态输入。
- 预览目录 `~/Downloads/clawd-reference-20261001-4pdhpd0_/`：`saddle-excited-pirate.mp4`、两份 keyframes PNG、`saddle-curated-14.mp4`。`saddle-curated-patrol.mp4` 为公开 Mascot.draw 导出的120秒巡游，确认包含两种新动作。预览是字符栅格化结果，不是用户窗口截图。
