# Settings 插件管理页整理

用户截图反馈Settings → Plugins不够清楚、长说明与技术路径堆叠。本轮仅UI与英文说明文案，主控直接做、不委派不遥测。

- 统一窗口尺寸；宽屏列表/详情左右分栏，窄屏上下分区，列表列对齐并突出当前选择。
- 详情按状态、资源、接入说明、路径分组；PgUp/PgDn及详情区滚轮可读全部内容，切换选择回顶部。
- 原按钮、焦点顺序、可用条件、启停/同步/删除/注册操作不改；添加插件页路径字段加框。
- Dispatch的setup_note及模板标签改英文；不修改SKILL/模板文件、资源revision、路由或遥测。

验证：两项直接管理页检查通过，覆盖1×1至100×40既有极小尺寸，以及48×24、80×24、140×44的内置/外部详情、完整说明可滚动、选择重置、Back焦点动作。测试只用临时registry/资源目录，外部假插件只注册disabled，不启动。读取合成终端文本预览确认宽屏分栏；真实桌面复看待用户。必要宿主release及diff check通过，不跑全仓/Clippy。日志/tmp/saddle-plugin-settings-{ui-test,layout,final,verified,build}.log；本轮无功能RED要求。
