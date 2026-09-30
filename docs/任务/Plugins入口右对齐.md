# Plugins入口右对齐

用户原话：“Plugins是不是可以放到右边了，左边三个太挤了”。

主控亲自完成，不委派。Dispatch：`8b9c4cebd9da493eae61b3018c0509f1`。

顶部整理为左侧Agents、Attention，右侧Plugins、Settings。Plugins与Agents标题同行，不再额外占据左侧一行；极窄窗口放不下时右对齐换行，点击区域随文字移动。只改变宿主入口排布，不改插件、任务、agent或配置。

纯视觉变更，使用现有渲染与实际入口点击/弹层流程验证，不构造RED。Settings的旧测试仍定位第3行，已按两行新布局同步，并核对Plugins位于首行右侧；release设置/保存/侧栏缩放流程1项通过，日志 `/tmp/saddle-plugins-right-release-app.log`。

标准 `cargo test --all-targets -- --test-threads=4`：335 passed / 0 failed / 4 ignored；Clippy（`--all-targets -- -D warnings`）、fmt、diff检查通过。日志 `/tmp/saddle-plugins-right-{all-final,clippy-final}.log`。日常宿主更新后由用户重启观察；插件包不变，部署hash和备份位置见HANDOFF.md。
