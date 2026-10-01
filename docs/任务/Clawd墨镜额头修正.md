# Clawd 墨镜额头修正

主控亲自实现，不委派。Dispatch `cdebdd65b29646f38d2a0703aecb9003`。

## 用户原话

> 其他都没问题，就是sunglasses有点小问题。这是墨镜的截图

## 修正

截图中的黑色镜片从额头顶缘开始，和深色背景连成两个缺口。只把墨镜佩戴帧中的半格黑块换成肤色背景上的方形镜片`■`，保留鼻梁`─`。其余11个片段逐字节不变，墨镜帧数、节奏、身体轮廓和3行占用不变。没有字体调查或安装。

沿用已确认的公开Mascot.draw入口，先检查实际巡游到墨镜时镜片必须在完整肤色背景内。旧版RED为`▀`而非内嵌方形镜片，日志`/tmp/saddle-clawd-sunglasses-red.log`；实现后全部7项Mascot检查GREEN，日志`/tmp/saddle-clawd-sunglasses-green.log`。

修前/修后同帧对照位于原提取目录：`saddle-sunglasses-fix.png`、`.gif`、`.mp4`。同时更新`saddle-curated-12.mp4`/`.gif`/`-poses.png`总览；预览为指定字号栅格化，不是用户运行窗口截图。

最终标准全量375 passed / 0 failed / 5 ignored；Clippy/fmt/diff通过。日志`/tmp/saddle-clawd-sunglasses-{all,clippy}.log`。90秒实际Buffer巡游预览也已同步更新。
