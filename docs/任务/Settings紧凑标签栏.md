# Settings 紧凑标签栏

用户原话：「这个界面改一下吧。plugins都这到第二行了」，附 Settings 五页按钮因空间不足换行的截图。

主控亲自完成，不委派。Dispatch `1d77432fe0ce40978e99c4c5fc5a20fa`。

范围：仅将 Settings 顶部分页从三行高的描边按钮改为现有紧凑标签样式，使正常窗口五项同排；窄窗口按内容紧凑换行，保留全部名称、F1–F5、选中提示和点击区域。保留 main 中用户观察 Diff 的三处临时演示改动。不改变设置内容、保存流程、插件管理或其他对话框按钮。

验证：直接检查76/100列窗口五项同排、24/40列各页完整可点击且区域无重叠；沿用小尺寸及保存/诊断测试、完整标准检查；构建独立发布候选后更新日常宿主，用户重启观察，不强制重启其窗口。

## 主控审查

| Severity | Location | Before | After | Why |
| --- | --- | --- | --- | --- |
| MEDIUM | src/settings.rs:649 | 三行高描边按钮；正常宽度Plugins换排，极窄时最后一页甚至没有点击区域 | 复用单行紧凑按钮，正常五项同排，窄时紧凑换行 | 保持分页分组，避免装饰挤占内容与入口 |

实现仅调整Settings分页绘制器的选择，未改共享按钮实现、页名、快捷键或保存逻辑。无需新增插件接口，也未改其他弹窗。

验证：76/100列五项同排；24/40列五项完整、点击区域不重叠，渲染文字和点击范围一致；既有极小尺寸和设置/诊断回归通过。真实release程序的设置保存/侧栏即时调整、release标签几何检查各1项通过。RED为76列旧标签高度3而预期紧凑1，日志 `/tmp/saddle-settings-tabs-red.log`；相关GREEN见 `/tmp/saddle-settings-tabs-target.log` 和 `/tmp/saddle-settings-tabs-release-check.log`。

Not verified：用户当前窗口重启后的实际画面，留给用户观察；未强制重启或操作其agent。

Approve：限定Settings顶部布局范围内通过。

最终标准检查：`cargo test --all-targets -- --test-threads=4` **354 passed / 0 failed / 5 ignored**；Clippy、fmt、diff检查通过。日志 `/tmp/saddle-settings-tabs-all.log`、`/tmp/saddle-settings-tabs-clippy.log`。发布候选SHA-256：`1459a6b6d3631305056ae4ee8f3723d7effa1ed9fa9e2ab3b71a49e2e9996c16`，原日常宿主 `eebb3117…`；候选独立构建在共享target的aarch64子目录，未在验证前覆盖日常程序。
