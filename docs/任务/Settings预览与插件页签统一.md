# Settings预览与插件页签统一

用户原话：2个问题，1: settings下这个页面下面也太乱了 2: 图2点击plugins后，和其他的tab页签点击不一样。plugns没有其他页签的入口了

主控亲自完成，不委派。Dispatch `9125dd85794742b383fbb2978a33658d`。

范围：Colors底部预览分为状态与文字两行，去除模拟按钮和混杂的任务状态；Plugins管理页复用Settings顶部五个页签，点击或F1–F5可直接切页，保留设置草稿、即时插件操作及Add local子对话框。保留真实配置、插件包、队列、服务与三处Diff演示改动。

检查：切页路径与草稿保留先复现，再修复；Colors预览和窄窗口渲染检查；标准检查及隔离release流程通过后更新日常宿主，不重启用户窗口。

## 主控审查

| 程度 | 位置 | 原表现 | 修改后 | 原因 |
| --- | --- | --- | --- | --- |
| MEDIUM | src/settings.rs:923 | 预览混排agent、任务状态、模拟按钮与输入提示 | 两行Status/Text示例，与真实操作隔开 | 清楚区分示例与可点击操作 |
| MEDIUM | src/settings.rs:647、src/plugins/ui.rs:243 | Plugins独立弹层缺少其他页签 | 复用配置路径与五页签绘制，宽度/内边距一致，直接切页 | 保持导航入口和对齐一致 |

主控核对：插件页没有底层输入光标；目录添加流程保持自己的输入框。页签复用现有Pointer的按下/松开和悬停处理；F5不重置当前插件选择。跨页继续使用同一个Settings草稿；插件操作仍即时生效，未将其混入配置Save。底部状态栏在重画前清空，避免残留背后的快捷键文字。

有效RED：隔离PTY打开Plugins后缺少General F1，日志 `/tmp/saddle-settings-pages-red.log`。修复后实际点击Colors/Advanced/Diagnostics/General、键盘F2/F5切页并保存此前的60列草稿均通过，日志 `/tmp/saddle-settings-pages-green-final.log`。扩展测试中曾误写大小写不同的Advanced标签，已按实际文案修正；不计作产品缺陷证据。

布局验证：Colors在76/140列的分组文字、预览与保存控件留白通过；现有极小/正常窗口Settings和插件管理/Add local测试通过。最终release上切页与预览2项通过，日志 `/tmp/saddle-settings-pages-release-check.log`。未检查用户当前窗口的肉眼效果，也没有重启该窗口。

最终标准检查：`cargo test --all-targets -- --test-threads=4` **360 passed / 0 failed / 5 ignored**；`cargo clippy --all-targets -- -D warnings`、fmt和diff检查通过。日志 `/tmp/saddle-settings-pages-all.log`、`/tmp/saddle-settings-pages-clippy.log`。release候选SHA-256 `ed44209ed84a2fa5b81d06e7418944fa230659ec85e3edcaa59ed66d6cca10a5`，已完成前述2项release检查。

Approve。按既定流程合并、推送并原子更新日常宿主；用户重启后观察，配置、真实任务及插件包不变。
