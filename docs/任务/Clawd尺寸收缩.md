# Clawd 尺寸收缩

主控亲自做，不委派。Dispatch `5d674c0d0ede487281c442abd41e0b76`。

## 用户原话

> 按照file:///Users/firegnu/Downloads/clawd-reference-20261001-4pdhpd0_/preview.html里面的大小，我不觉得会占用到7行

> 继续吧，但是不要绑定terminal。

> 我觉得你是不是做复杂了。先把大小弄好。

> 好的，继续吧

## 范围

只改采样尺寸与匹配的画布常量，不改动作逻辑或布局，不引入图片协议。此为纯视觉变更，以同帧/同字号对照和既有回归作为验证，不伪造行为 RED。先交付尺寸预览，等待用户看过后再更新安装版。

## 尺寸结果与预览

- 最终候选画布16列×5行，站立身体在原3行区域内；动作逻辑、布局、agent输入实现未改。只重采样素材、同步3个尺寸常量及更新旧尺寸断言。
- 同一walking首帧，按截图估计每格17×38设备像素、2×显示换算：旧版72.5×57逻辑像素，候选55.5×38，原素材60×40。这里只是此字号下的栅格化对照，不是任意终端上的像素保证。
- 采样不透明覆盖阈值改为50%，防止缩小后低覆盖的边缘额外撑高一层；所有38种素材及源帧数不变。
- 对照图：`~/Downloads/clawd-reference-20261001-4pdhpd0_/saddle-size-comparison.png`。实际Rust Buffer预览：同目录`saddle-size-preview.png`与`.gif`。这些是栅格化预览，不是用户窗口截图。
- 字符采样仍有眼睛/轮廓细节差异；此候选仅处理用户本轮要求的大小，不能声称已达保真复刻。
- 未构建release、未安装、未合并/推送；安装版hash仍为`d2ebb2f549167413324afedff7aac41207e3be97302c7d3c289f0dd0e4fafb16`。保留独立分支等待用户看过尺寸。

## 验证

最终 `cargo test --all-targets`：372 passed / 0 failed / 5 ignored；Clippy、fmt、diff通过。日志 `/tmp/saddle-clawd-size-{all,clippy}-final.log`。原断言要求身体跨过边框，已按本次缩小目标改为站立在3行内，并将控件避让位置移到真实身体处；行为实现未改。

阶段结论：尺寸候选可供用户查看，非最终视觉验收；不合并、不安装，不记收尾。

## 眼睛修正（尺寸已获认可）

> 大小一致了，但是眼睛那块还有点问题

用户认可大小，继续修正采样产生的头顶缺口。保持画布与身体尺寸；修正抗锯齿肤色识别，并在放置眼睛前清除粗采样的重复黑块。公开Mascot绘制入口新增两眼同高、上方额头连续断言；旧候选实际RED为“eye must not cut the forehead”（`/tmp/saddle-clawd-eyes-red.log`）。

眼睛修正验证：目标绘制检查GREEN，完整标准检查373 passed / 0 failed / 5 ignored，Clippy/fmt/diff通过；日志`/tmp/saddle-clawd-eyes-{green,all,clippy}.log`。同帧对照修前/修后包围盒均111×76设备像素（本对照2×显示下55.5×38逻辑像素），尺寸未变。已检查38种动作的代表帧修前/修后联系图。

新对照`~/Downloads/clawd-reference-20261001-4pdhpd0_/saddle-eyes-comparison.png`已在系统预览打开；实际Buffer动画`saddle-eyes-preview.gif`同目录。修正头顶缺口与重复眼睛，字符网格仍不能精确复现原参考的眼睛像素形状。继续停在视觉候选，不构建release、不安装、不合并、不推送。
