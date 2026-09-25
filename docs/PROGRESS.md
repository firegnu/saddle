# 进度

## 第 1 步（进行中）

- 已读 AGENTS、DESIGN 和 corral/tools/board；只读调用公开 ls/status/reply 核对 JSON。
- 已完成 Rust stable 项目骨架、配置解析/校验、可缩放三格布局。
- RED→GREEN：`cargo test --test layout_config`，布局从整窗错误分配到预期三格；无效比例从错误接受到拒绝。2 项通过。
- 终端原型：`cargo run --example compare_parsers` 通过。选 alacritty_terminal；理由和设计变更见 DESIGN 第 10 节。
- 用户明确要求不启动真实 agent；所有后续自动测试使用合成流和假的 corral/queue。
- 待做：Agents 数据/交互、PTY 生命周期与渲染、主循环和假命令集成测试、完整 clippy。
- 最后由用户在真实终端目视确认，自动测试不作为真实 Claude Code/drover 体验验收。

### Agents 数据与输入路由

- 公开命令客户端与后台刷新；失败/超时显式报错，启动中/协议不兼容项不请求 status。
- 保持选中名字、项目内排序、状态完成标记；Ctrl-] / Tab / Shift-Tab 路由和终端按键编码。
- RED→GREEN：假 corral 空列表、刷新跳选、终端键被面板截走、UTF-8 编码为空均先复现后修复。
- 合成网格测试覆盖分片中文、宽字符占位、真彩/256 色、鼠标/粘贴模式及光标查询回复。
