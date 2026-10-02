# Plugins 弹窗布局

2026-10-02。用户批准已展示的三列线框，并明确：“继续刚才的ui计划吧。不委派，不遥测”。主控直接实现。

## 范围与结果

- 搜索框增加边界；列表使用名称 / Status / Action 固定列，Built-in 移至名称旁。
- 选中整行使用既有暖色底色，名称加粗；停用项弱化，操作统一为 ‹Manage› / ‹Open› 等按钮形式。
- 说明只占一行，优先显示错误原因，完整管理信息仍到管理页；条目范围移到标题右侧，去掉原三行说明留白。
- 窄屏隐藏行内动作但保留 Enter；极矮屏省略搜索框装饰。键盘、鼠标命中区随显示一致，生命周期/启停/打开行为不变。
- 只改宿主 Plugins 入口弹窗，不改管理页、Drover、遥测或 Corral。未派发 agent，未记录本任务遥测。

## 验证

纯视觉改动，不制造功能 RED。既有底色/高度断言随批准的布局调整；新增常规/窄宽度的三列对齐、选中底色、停用样式和按钮边缘点击检查，沿用搜索、状态变化取消点击、小尺寸、滚动和 built-in 管理入口检查。

80×24 的 TestBackend 文本预览已核，临时预览 example 已删除；产物 /tmp/saddle-palette-preview.txt。标准首次 415 通过、1 失败、5 忽略：workflow 仍匹配旧版连写的 Built-in · Disabled。改为分别核名称旁 Built-in 和状态 Disabled 后，该场景复测通过；补齐中断后其余工作区 122 项和宿主 examples 均通过。总计 538 个不同用例通过、5 忽略；不是一次全套绿。全目标 Clippy 通过，更新 workflow 断言后对应目标 Clippy 再通过；diff --check 通过。

日志：/tmp/saddle-palette-standard.log、workflow-recheck.log、remaining.log、examples.log、clippy.log、clippy-workflow.log（后五者同 saddle-palette- 前缀）。纯视觉无行为 RED 宣称；初次目标检查中的旧底色断言失败同样保留为旧预期，不当作功能缺陷。

宿主 release 在共享 target 的 aarch64-apple-darwin 子目录构建，未直接覆盖日常路径。候选 /tmp/saddle-palette-stage-path 指向的目录，manifest.json 保存旧/新 SHA256；候选 --help 通过。安装结果见 HANDOFF。
