# Clawd 原版动作精修审查（Opus）

你是用户明确指定的独立 Opus 审查者，不是主控，不委派子agent。这一轮只审查并给建议，不实现。

## 用户原话

> 要不要让opus参照 file:///Users/firegnu/Downloads/clawd-reference-20261001-4pdhpd0_/preview.html 再看看是不是有可以细化的地方？我觉得是静止态已经比较完美了，就是在这个静止态的基础上的动作请他参考原版看看能不能再优化一下？如何

## 固定边界

- 用户已认可静止态。保留静止身体轮廓、大小、眼睛位置/形状、颜色和地面基线；不要重开静止态设计。
- 保持16列×3行画布、现有tab/pane/PTY布局，动作不增加高度、不越过agent上方边框，不遮挡控件。
- 不绑定终端品牌、不调查/安装字体、不引入图片协议或重写绘制架构。不增加agent状态绑定。
- 现有31段已安装，其中30段参考原网页，pirate是原创。只审查已有31段；不加入8个已明确排除的跳跃/开心跳跃/杂耍/撒彩纸/开心撒彩纸/彩虹/风筝/霹雳舞。
- 用户重视交付前完成打磨，尽量一次成型；不要把可检出的视觉问题留给用户逐个指出。但不要承诺绝对零审美调整。

## 材料

审查基线：f7e57dd，当前detached worktree `/Users/firegnu/Developer/personal_projs/saddle-worktrees/review-clawd-opus`。

原版参考目录 `/Users/firegnu/Downloads/clawd-reference-20261001-4pdhpd0_/`：
- `preview.html` 是用户明确指定的完整原版动作预览，需实际观察原动作。这个HTML内嵌大量Lottie数据，非常大，不要直接cat/整行打印；用浏览器渲染或读取对应 `animations/Clawd-*.json` 并渲染关键帧来观察。
- `animations/`、`manifest.json`、`动作清单.csv` 为原资源及目录信息。原版38段，当前31=原版30+原创海盗。
- `saddle-curated-31.mp4`、`saddle-curated-31-poses.png` 是当前全31段。
- `saddle-eleven-actions.mp4`、`saddle-eleven-keyframes-{1,2,3}.png` 和 `saddle-eleven-patrol-excerpts.mp4` 是最新11段；后者为实际公开绘制巡游节选，带源模拟秒数，有跳切。
- `saddle-six-actions.mp4`、`saddle-six-actions-keyframes.png`，`saddle-excited-pirate.mp4` 和相关keyframes图补充前两批。

实现入口：`scripts/prepare-clawd.py`（离线手工关键姿态、停顿时长）、`assets/clawd/{frames.bin,sources.json,README.md}`、`src/mascot.rs`、`tests/mascot.rs`、`docs/DESIGN.md`。运行时12fps，普通字符/四分块，原身体32×6子像素映射至16×3字符；调度walking2.2–4.6秒、边缘转身、随机其余动作并排除最近5段。这轮重点是动作本身的精细度，调度建议另列，不自行更改规则。

已有临时工具可只读参考（不要覆盖它们）：`/tmp/clawd-eleven-preview.py`、`/tmp/clawd-31-preview.py`、`/tmp/check-clawd-eleven-patrol.py`；Pillow环境 `/tmp/saddle-clawd-render-venv/bin/python`，ffmpeg在PATH。不要默认视频或GIF能被模型直接观看，必要时导出帧并实际看图。若无法观察原版或当前某段，明确标为未验证，不能仅从文件名猜动作后宣称对比过。

## 工作与输出

对照原版和当前效果，区分当前字符分辨率的客观限制与可改善的问题。围绕手脚节奏、身体稳定性、道具与手的接触、视线配合、可读停顿、进入/退出衔接提出具体建议。不要为了像原版而破坏用户已认可的静止态。

报告先给最值得改的少量重点，再给31段逐项简短判断（保留/微调/需明显改善/证据不足）。每项重要建议应有：观察到的具体差异，参考动作/帧或时间依据，现有代码位置，三行内可执行的最小改法，预期收益及风险。明确哪些动作已足够好、不值得动。给一个可在交付前完成的视觉检查办法，避免把用户当逐帧验收者。

代码和测试只读，不运行完整测试、不构建release、不安装、不提交/合并/推送、不操作用户agent或任务队列。只允许写报告 `/Users/firegnu/Developer/personal_projs/saddle/docs/调研/Clawd动作精修-Opus审查-2026-10-01.md`，以及你自己创建的 `/tmp/clawd-opus-review-*` 临时观察产物。不要修改现有预览/源文件。用户这次明确授权Opus审查，覆盖HANDOFF中以往不委派的偏好，仅限本轮审查。

命令都在前台跑完，全部做完后，回复最后一行写 DONE。
