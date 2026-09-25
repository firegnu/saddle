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

### PTY 与三窗格界面

- PTY 读写各自独立线程，输入有界队列；切换先 SIGINT 旧 attach，主循环轮询退出后再启动新的，3 秒后只强制结束本程序创建的 attach PID。
- 原生 Agents 显示全部字段（窄屏身份信息折行）、分组、滚动条/隐藏项计数、回复分页、x/y 确认停止；后台 status/reply/stop 不阻塞输入。
- Queue 执行配置 command/cwd；Viewer/Queue 支持颜色、中文、光标、应用光标键、鼠标坐标换算、粘贴和 PTY resize。
- `cargo test --test pty --test terminal --test input` 通过，RED 证据包括 PTY 未收到输入、网格未绘制、鼠标编码为空。
- 整程序外层 PTY 测试需模拟光标查询响应，并按终端网格断言（绘制流会插入定位序列）；启动/退出测试已接入。尚在补充切换和操作回归。
