# UI主题：预置与颜色覆盖（已批准，已实施待审查）

2026-10-02。Claude设计初稿2c42a0b经主控静态核对，用户已批准下方线框及“切换载入整套配色，再逐项覆盖”的推荐，并授权实施。初稿比较方案以本文为准；功能尚未实现。

## 用户需求与批准

> 写一个ui theme，预置几种theme，选择的时候，下面color随theme变化，但是用户可以覆盖这个颜色。
> 现在的theme可以设为dune
> 可以的。如果没有其他问题就实施吧

用户确认的提议：Dune（现有风格）/ Tide（冷色）/ Terminal（跟随终端）；保持英文；选择主题在草稿载入整套新配色，然后逐项修改，Save生效、Cancel撤回，升级不重置现有颜色。

## 界面

沿用Settings → Colors F2，在现有颜色字段上方加Theme选择；不重排其他Settings页面。

```text
Theme       ‹ Dune ▾ ›

Interface
Background    [default]
Focus         [yellow]   custom
…

Preview
                 Default   Cancel   Save
```

沿用键鼠选择方式（方向键/Space/Enter及点击），颜色字段保留原分组、色块、输入及滚动；值显示有效色，有覆盖标custom。当前有修改仍沿用草稿标记。不会把现有英文字段全部重新命名，上方线框仅展示层级。

## 预置及覆盖

- Dune：原Theme::default()的完整41色基线，名称dune。原有配置没有theme键时等价于Dune叠加原[colors]，无迁移和自动写回。
- Tide：冷色主题，采用协调的蓝灰中性色，既有语义色角色不变。需让宿主公共区域与Agents栏形成一致冷色观感，而不只更换Agents少量中性色；具体值由界面实现者在已有字段中选定，记录关键值和声明颜色对的核对。不要把静态颜色计算说成真实终端验收。
- Terminal：全部41字段只用default/ANSI/终端调色板值，包括agent类型色，不保留RGB再声称“完全跟随终端”。颜色实际观感依赖外层终端。
- 无主题商城、主题文件导入导出、多层继承或额外依赖。
- 有效颜色=预置+[colors]显式覆盖，覆盖须保留“有无”状态，不能仅凭有效值推断用户意图。现有Theme作为最终渲染值结构继续沿用。

## 交互规则（用户已批准）

- 明确选择另一个主题时，在草稿中载入该主题整套配色，清除当前草稿里的所有颜色覆盖。然后用户可逐项修改形成新覆盖。只选择当前主题不应意外清除覆盖。
- 切换后的提示应清楚说明颜色草稿已按主题替换；不增加模态确认。Save前可以Cancel撤回整次编辑。
- 颜色行Default Ctrl-D：移除该项覆盖，显示当前主题基准，保存时删除对应[colors]键。等于主题值的显式覆盖仍是覆盖，不用值相等推断“从没改过”。
- Theme行Default Ctrl-D：按切换规则回到Dune（已经Dune时不作为清空覆盖的隐含入口）。
- 编辑颜色保留原语法及错误处理，空字符串仍无效；恢复跟随主题使用Default操作。
- 主题及颜色草稿只影响现有色块与Preview，不即时改变整个宿主；Save成功走现有应用设置路径，立即生效；Cancel丢弃草稿。
- 文件冲突、Keep my edits / Discard my edits保留既有含义，并包含主题和颜色覆盖的增删。保存失败不丢草稿、不误报成功。

## 最小存储方案

```toml
theme = "tide"  # dune | tide | terminal; omitted = dune
[colors]        # optional per-color overrides
focus = "yellow"
```

- 新顶层theme键，未知值沿现有校验报错；旧配置无theme不变。保持[colors]现有语法；只在显式Save时持久化主题或覆盖增删，保留无关键/注释及冲突检查。
- 配置加载需区分显式覆盖与缺省；实现者选择最小内部表示，避免重复两套41字段业务定义。向渲染和插件传递的有效Theme不变。
- 保存了新theme键以后，旧版Saddle因deny_unknown_fields不能读取，这是降级限制；不增加迁移工具。
- 根config.toml示例颜色改成注释示例，避免新用户照抄成全覆盖。

## 当前配色兼容

用户现有[colors]有28项，24项与原默认相同，4项不同：agents_bg=default、agent_selected=#302a23、claude=#d97757、codex=#8ed9c1。升级只按Dune+覆盖读取，不改这4项或其他键，不把个人差异改成所有人的默认值。用户后来明确切换主题并Save时才应用上述整套替换规则。

Reset/default和ANSI取决于终端；Viewer里的agent输出颜色不属于Saddle主题，保持原样。

## 宿主及插件边界

静态已核：宿主Agents/边框/弹窗/按钮使用config.colors；现有插件通用协议仅传text/muted/background/accent/error五色，Drover和Diff这些部分跟随。Drover自有Tasks状态色和Diff固定语法高亮仍不跟随。此轮不扩插件协议、不改业务，不声称所有插件色都已统一。

## 主控核对与后续

初稿Terminal“全部ANSI”却保留RGB类型色的矛盾已在本稿纠正；初稿A/B切换策略已由用户批准的整套载入规则替代。后续交Claude实施，配置持久化属于功能变更，路由重新判定验证预算；不沿用设计的“只静态检查”预算。
