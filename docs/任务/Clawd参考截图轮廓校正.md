# Clawd参考截图轮廓校正

用户原话：[Image #1] 这是你现在画出来的。[Image #2] 这个是我从Claude code启动的时候截图出来的。差别有点大，哥们

参考 `/Users/firegnu/Desktop/SCR-20261001-crgc.png`。主控亲自完成，不委派。Dispatch `09d4a2217bfe4098bcac316c4868256d`。

只修绘制细节：采用四分字符块保留半字符宽眼睛和四条细腿、矩形身体及两侧低位短臂，保留参考颜色；轻微步态不得消失成两条粗腿。tab/pane几何、agent状态及业务不变。纯视觉修正，验证实际渲染与截图对照、颜色/四腿/输入隔离和标准检查，不制造RED。

## 主控审查

- 只改 `src/mascot.rs` 的像素与绘制，16种四分块映射；状态来源、速度、tab/pane几何、业务与输入不变。未引入库或终端图像协议。
- 每帧检查四条腿细且分离，眼睛cell不混入第三种颜色；保留状态/小窗口/鼠标不输入检查。实际Ratatui cell导出 `/tmp/saddle-clawd-reference-layout.png` 和 `/tmp/saddle-clawd-reference-sprite.png`，已与用户启动截图对照，轮廓的细眼/低短臂/四细腿对应。该预览并非用户窗口截图。
- 发布版动画/鼠标不传入agent检查1项通过，日志 `/tmp/saddle-clawd-reference-release.log`。发布候选SHA256 `e4ef277dd085c8d89953e0dadc116b650db72e7cb5f2de6dd91232fd3ac34494`。
- 三处Diff演示hash一致；真实agent/配置/插件/队列/服务未操作，未重启用户Saddle。

- 最终 `cargo test --all-targets -- --test-threads=4`：368 passed / 0 failed / 5 ignored；Clippy、fmt、diff检查通过。日志 `/tmp/saddle-clawd-reference-{all,clippy,target,release}.log`。主控审查通过，正常合并/推送/清理，备份更新日常宿主；用户重启观察。
