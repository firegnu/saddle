# T2 New 表单任务的同轮补充：Open 语义与弹框一致性

2026-09-26，saddle/main 发给正在进行 `t2-new-agent-form` 的 saddle/dev-t2-new-form-1。这是用户对同一轮交互改进的追加要求，继续原 worktree/分支，不新开 agent，不合并、不推送。原任务与本文件一起完成后再交主控；不要因原任务完成就把本轮视为结束。

## 用户原话

> 你这个太晦涩的，我都没看懂你那个功能怎么用，而且设计语言也没对齐啊。其他弹出对话框的esc/cancel可不是那样的。

这里指刚询问过的 Open agent 弹框。原先 New 输入框轮廓、点击光标及编辑体验要求继续有效。

## 补充范围

- 只增加 Open 的文案、动作分组、取消/返回和控件风格调整，仍限 src/launch.rs、src/app.rs/src/ui.rs 必要接线、对应测试与说明。覆盖原任务「不重做其他弹框」中对 Open 的限制；其他弹框、tab/split 和 PTY 状态机不改。
- 主仓库 `/Users/firegnu/Developer/personal_projs/saddle/docs/DESIGN.md` 第 27 节末尾已新增「同轮补充：Open 的用途和弹框一致性」。该节不在你的旧 checkout 中，读这个绝对路径即可；不要复制或重写主仓库文件，避免与本轮并行文档更新冲突。
- 先对照 `src/queue.rs::draw_overlay/controls` 现有 Cancel Esc、`src/ui.rs` 的停止确认及 `buttons::draw_compact`。Open 现在使用 `buttons::draw` 的大块三行按钮，不符合现有紧凑底部取消工具栏；要复用既有样式，不另造一套。
- 用明确文字说明「将已有 agent 显示到哪里」，显示所选名称。可按设计示意将入口改为 Show in…，区分 Replace current pane / Open in new tab / Split current pane 下四个方向，并清楚说明分屏相对于当前活动窗格。六项语义和去重行为不变，不增加确认步骤。保留原有数字快捷键作为次要提示，不能以数字说明代替用途说明。
- New 与 Open 的取消/返回都要对齐现有弹框的按钮样式、快捷键提示、底部位置及 Esc 行为；原 New 表单草稿保留等既有行为不改。只需复用局部控件，不全局重构其他弹框。

## 验证与交付

- 沿用原任务改行为预算；若原 New 的标准套件尚未运行，将本补充一起完成后跑一次。若已跑完，只做 Open 直接相关增量及 clippy/diff 检查，不重复无关全套。
- 必须观察 New、Open 与一个现有 Queue 弹框的合成渲染，确认取消按钮的外观/位置一致；相关行为检查保持六种位置映射、Esc/Cancel 不执行接入。可重用已有假 CLI 与渲染检查，不启动真实 agent、队列，不录屏或做覆盖矩阵。
- 保留全部已有行为断言；文字/布局/点击入口变动仅相应调整断言或操作步骤，不删除旧接入安全回归。
- 在你的 `docs/任务/T2-New表单体验改进.md` 完成记录中明确记录本补充也已完成（引用本文件路径即可）。不要修改主仓库补充任务文件或 HANDOFF。
- 命令都在前台跑完，全部做完后，回复最后一行写 DONE。
