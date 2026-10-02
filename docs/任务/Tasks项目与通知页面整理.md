# Tasks 项目与通知页面整理

用户截图反馈：Projects、Add project / Project settings、Task notifications信息挤在左上、操作远离内容。延续本轮约定：主控自己改，不委派不遥测，只改UI、保持英文，不扩业务。

## 完成

- Projects独立居中面板，去掉无关队列工具栏；列出项目名和状态/接收者，完整选中路径及原操作保留。
- 添加项目、项目设置和字段选择器使用有边框的紧凑面板；字段边界、焦点和只读状态清晰，Browse/Check、Choose agent/No receiver紧邻对应字段，Save/Cancel放底部。
- 通知页面标题、单选项、保存反馈、适用范围及Save/Cancel分组；选中项圆点/底色明确，不改保存语义。
- 宿主外框、英文、任务/配置/通知业务均保持，未操作真实队列或agent。

## 验证

纯视觉任务，无伪造RED。相关project_setup 2项、原UI 22项、通知保存失败/重试1项及新增表单布局1项通过，共26个不同目标。表单与Projects检查48×24、80×24、160×50；核字段/按钮可见、点击区域不覆盖字段及名称字段点击。表单文本渲染已人工读取，通知走隔离HOME的真实插件协议。最终目标日志/tmp/saddle-project-dialogs-{layout-final,projects-final,notifications,test}.log，必要插件release构建日志/tmp/saddle-project-dialogs-build.log；不跑全仓测试/Clippy。真实桌面复看留给用户。

Projects扩展检查第一次编译引用了私有buttons字段，未形成运行证据；已改为核公开项目行区域，最终通过。该夹具编译错误不计为功能RED。
