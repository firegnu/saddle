# 第一个插件 Demo 设计

2026-09-30。状态：已按用户确认实施首个独立 demo，尚未安装到真实工作区。依据：[插件系统设计](插件系统设计.md)。用户确认“先最小 demo，Drover 第二个”，下面的计数器是主控建议的具体内容，不是新增业务需求。

## 目标

证明独立 Rust 插件仅按公开规范就能安装、启用、显示、交互、发通知和停用；不改 Saddle 才能识别这个插件，也不要求先完成 Drover 迁移。

## 可见内容

插件 ID `demo.counter`，名称 `Counter demo`。一个面板：

```text
Counter demo

Clicks: 0

[ Increment ]

Enter or click to increment.
```

- 点击 Increment 或面板聚焦时按 Enter，计数加一并重绘。
- 每次增加请求一条内部通知，例如 `Counter demo: 1`；遇到宿主限流时在面板显示反馈，不重复请求。
- 计数在插件进程内保存。关闭面板再打开保留；停用或重启插件归零。正文明确这一行为，不伪装持久化。
- 窗口变小时只裁剪/重新布局，不添加第二种视图、设置页或滚动需求。
- 没有文件保存、网络、本地命令、agent 操作或任务状态。

## 用户操作路径

1. 在独立 demo 项目构建本机插件产物，目录包含清单和可执行文件。
2. Saddle 的 Settings → Plugins → Add local，选择该目录；此时插件仍停用。
3. Enable，看到 Running；Open panel 在新 tab 打开计数器。
4. 点击按钮或按 Enter，看到计数更新与内部通知。
5. 关闭面板再打开，计数保留；Disable 后面板显示停用占位。
6. 再 Enable，计数归零；Remove 只移除登记，原插件目录还在。

用户已确认在 Saddle 内绘制界面并授权继续；该路径已在临时 HOME 的实际 Saddle PTY 中验证。线框保留作为交互说明，具体检查见 [实施记录](任务/插件首个Demo实现.md)。

## 界面线框（已确认）

沿用 Saddle 现有英文界面文案、主题与弹窗布局。以下是文字线框，不是运行截图；宽度不足时按钮换行、详情折行，列表保留 Name/Runtime，启用状态在详情中可见。不新增侧边栏或常驻工具条。

Settings 原有 General/Colors/Advanced/Diagnostics 保留，新增 `Plugins F5`。进入后显示独立即时操作页；按 Back/Esc 回到原设置草稿，保存/取消原设置不撤销插件操作。

```text
┌─ Settings · Plugins ───────────────────────────────────────┐
│ Changes here apply immediately.                            │
│                                                           │
│   Name                 Enabled    Runtime                  │
│ > Counter demo         Yes        Running                  │
│                                                           │
│ ID         demo.counter                                    │
│ Directory  /Users/you/Plugins/counter                       │
│ Program    /Users/you/build/saddle-counter                  │
│ Version    0.1.0                                           │
│                                                           │
│ [Open panel] [Disable] [Restart]                            │
│ [Add local…] [Remove (disabled)] [Refresh] [Back]            │
│ ↑↓ Select   Tab Move focus   Enter Activate   Esc Back      │
└───────────────────────────────────────────────────────────┘
```

这里括号里的 disabled 表示按钮置灰，实际 UI 不把它作为按钮文案。列表可点击选择；操作按钮支持点击、Tab/Shift+Tab 和 Enter。不让 Enter 在选中插件行时隐式启用/重启，只有明确聚焦按钮才执行。

Add local 使用路径输入弹窗，不开发文件浏览器。输入现有目录后 Read manifest 只读取检查并显示预览；只有成功预览才可 Add disabled。修改路径使预览失效；登记前重读清单核对没有变化。显示解析后的可执行路径，让用户能看见符号链接目标。

```text
┌─ Add local plugin ─────────────────────────────────────────┐
│ Directory  [/Users/you/Plugins/counter                 ]   │
│ [Read manifest]                                            │
│                                                           │
│ Counter demo  0.1.0  ·  demo.counter                        │
│ Program: /Users/you/build/saddle-counter                    │
│ Runs with your user permissions when enabled.              │
│ Adding does not start the plugin.                          │
│                                                           │
│ [Add disabled] [Cancel]                                    │
└───────────────────────────────────────────────────────────┘
```

不要求用户每次 Enable 再确认权限。第一次登记默认 Disabled，选 Enable 后立即尝试启动；Running 后 Open panel。Open panel 关闭 Settings 并切到右侧工作区的新 tab，已有面板则定位过去；现有设置草稿保留，下次打开继续。宿主不替换正在使用的 agent/shell。

```text
Agents                   Workspace: [agent] [Counter demo ×]
                         ┌─ Counter demo ───────────────────┐
                         │                                  │
                         │  Clicks: 0                       │
                         │                                  │
                         │  [ Increment ]                   │
                         │                                  │
                         │  Enter or click to increment.    │
                         │  Close keeps count.              │
                         │  Disable or restart resets it.   │
                         └──────────────────────────────────┘
                         Ctrl-] Agents

                     ┌─ Counter demo ──────────────┐
                     │ Count: 1                [×] │
                     └─────────────────────────────┘
```

只在该面板有焦点时响应 Enter，第一次打开按钮默认可操作。提示使用宿主内部浮层，不抢焦点；高频操作被通知限流时计数仍正常增加，面板短文本显示 `Notification rate limited`。不把没有收到通知结果误写成限流：请求断开/超时显示 `Notification result unknown`。

管理按钮按实际状态决定可用性：

| Runtime | 主要可用操作 |
|---|---|
| Disabled（enabled=No，已回收） | Enable、Remove |
| Starting | Disable；其余运行操作不可用 |
| Running | Open panel、Disable、Restart |
| Unresponsive（enabled=Yes） | Disable、Restart；原面板标无响应，停止业务输入 |
| Failed（enabled=Yes，已回收） | Restart、Disable；详情显示失败原因 |
| Stopping | 等待并显示错误/进度；不允许再启动或移除 |
| Unavailable（入口不可用，已回收） | enabled=Yes 时可 Disable；enabled=No 时可 Remove，修好目录后 Refresh |

登记冲突等错误直接显示在管理页，Refresh 重新读取，不覆盖外部修改。目录突然不可用但旧进程还活着时仍以实际 Runtime 为准，不能绕过停止/回收。Remove 仅在 disabled 且进程已回收后生效，不再弹第二层确认，因为只删登记、保留目录和业务文件；该影响在按钮附近说明。Refresh 不重启进程；跨实例登记意图与本实例已加载意图不同则在详情提示，本实例启停仍须明确操作。

停用后已打开面板显示 `Plugin disabled · Open Plugins / Close`；移除后显示 `Plugin unavailable · Close`；崩溃显示 `Plugin failed · Restart / Open Plugins / Close`。这些是宿主占位操作，不来自失效插件。关闭 tab 不弹“停止插件”确认，不等于 Disable。

## 开发交付建议

示例使用独立 Cargo 项目和自己的锁文件，依赖固定版本/revision 的公开 Rust SDK；不导入 Saddle `src/`，不链接宿主私有 crate，不把示例编进 Saddle。可先作为仓库内独立 examples 项目维护，但单独复制到仓库外仍应能构建；是否新建远程仓库不在本轮范围。

宿主在 demo 构建/安装后无需重新编译。宿主及 SDK 不能为 `demo.counter` 设置专门逻辑；第二个无关插件能按同一接口接入才有扩展意义，但首期不为此另造第三个产品插件。

## 与工程检查的区别

计数器只展示一条正常交互路径，不承担文件、任务流程或通用组件库功能。故障隔离、输入背压、Unicode/宽字符和过期尺寸帧等实现问题，使用合成帧或假插件做定向检查；不能把“计数器能点击”写成整个协议已经全面验证。

工程检查在独立测试环境执行，不触碰真实任务或服务。验收不把本页设计建议冒充用户原话；具体通过项和标准检查限制见实施记录。

## 后续

Demo 的协议和体验验证通过后，再整理 Drover 迁移清单。Drover 的状态机、CLI、数据与 notifications watch 不因 demo 而改动；尚未接入的 Attention、任务编辑、宿主跳转等能力单独按真实需要设计。
