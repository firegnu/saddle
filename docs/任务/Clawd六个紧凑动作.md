# Clawd 六个紧凑动作

主控亲自实现，不委派。Dispatch `690266cb13e64e2ab5e772b8f44501fb`。

## 用户原话

> 先做你建议的，我还是那句话，一次成型。不要返工

已接受的建议：看手机、冥想、笔记本电脑、玫瑰、爱心、呼啦圈。保持现有身体大小和3行高度。

## 实现与观察

沿用已确认的公开 Mascot.draw、边框/控件保护和状态独立检查。此次仅扩充视觉素材；不为像素布局编造失败测试，以全关键姿态、动态总览及公开绘制巡游观察外观，并核对原14段帧数据不变。道具在侧面，脸和基线保持稳定，各段从静止进入、返回静止。

## 观察与回归证据

- 共20段813帧，新手机49帧、电脑46帧、玫瑰44帧、爱心47帧、冥想63帧、呼啦圈47帧。原14段与基线6e17259逐字节不变，原调色板保留；生成器重复生成输出相同。
- 六段所有独特关键姿态逐张观察，收细手机/电脑比例，玫瑰保留花茎与叶片，爱心用普通文本心形避免粗网格失真，呼啦圈细弧在腰后变化。保持基线、三行画布与静止体型，不加入终端品牌或字体配置。
- 实际公开 Mascot.draw 导出3600帧（五分钟），逐帧匹配确认手机4、冥想3、电脑4、玫瑰3、爱心3、呼啦圈3种非静止独特姿态全部进入巡游。临时导出example已删除，不进入提交。
- 新增预览 `saddle-six-actions.mp4`/`.gif`、`saddle-six-actions-keyframes.png`，全部20段总览 `saddle-curated-20.mp4`/`.gif`/`-poses.png`，公开绘制巡游 `saddle-twenty-patrol.mp4`，均在原Downloads预览目录。预览为字符栅格化，不是用户窗口截图；已用既有授权打开目录。
- 首次全量检查在现有插件背压测试等待Running的3秒超时处失败；单独重跑通过（0.49秒），未修改插件代码或测试。日志 `/tmp/saddle-clawd-six-all.log`、`/tmp/saddle-clawd-six-plugin-retry.log`。
- 第二轮全量 `cargo test --all-targets`：377 passed / 0 failed / 5 ignored；Clippy、fmt、diff检查通过。最终日志 `/tmp/saddle-clawd-six-all-final.log`、`/tmp/saddle-clawd-six-clippy.log`。
