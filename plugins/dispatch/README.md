# Dispatch 内置插件

可选的同版本 CorePlugin，ID `dispatch`，命令 `route`，默认停用。只在 Saddle 二进制组装根登记；插件依赖下层 `saddle-core-plugin` 和通用库，不依赖宿主、Drover 或 Corral。资源与接入说明都由插件提供，使用现有通用启停、安装和采集机制。

在 Settings → Plugins 启用 Dispatch，查看各技能目标状态与项目模板路径；只读状态入口是 `saddle plugin status dispatch`。资源随启用安装，已有软链接和用户改动只报告冲突。需要项目默认主控分派时，由用户合入项目模板；没有项目采用名单或自动 AGENTS 修改。停用保留技能和项目规则，停用后可 Remove resources。详见 [资源说明](resources/corral-dispatch/README.md) 与 [遥测使用](../../docs/遥测使用.md)。

```sh
echo "<三五句任务摘要>" | saddle plugin run dispatch route
# 选择采集已有链路时：
echo "<三五句任务摘要>" | saddle plugin run dispatch route \
  --record-context /绝对路径/context.json --brief-file /绝对路径/任务书.md
```

环境中的 `TYPESAFE_API_KEY` 只用于 Authorization。没有 key 或摘要不是 UTF-8 时不请求、不 begin；任务书不发给 JEV。采集上下文由宿主读取，遥测初始关闭，关闭或存储失败不改变业务；无上下文不建库。stdout 是业务 JSON，退出码 0/1；宿主回执在 stderr，通过本次完整首尾边界配对判断，未知不重跑。插件只给建议，主控按项目规则/当次授权做最终决定。

路由语义来源为只读 Corral `6923da1ee0c766468c31e1f577fb8370b8b6da77` 的 route.py（SHA-256 `d4ab1cc82fe5c1b10987800eaf1c44d1e1fe026e7b1227b0eeb508cce548167a`），规则原文在 `src/rules.rs`。请求使用有字段顺序的类型，响应使用局部有序 JSON，未启用 serde_json 的全局 preserve_order。规则指纹是 model、tiers、questions、thresholds 组成的递归按键排序紧凑 UTF-8 JSON 的 SHA-256。`router_version=route-v1`；改变整理/重试语义需递增该常量。响应表示保留 i64/u64 范围整数，其余数值为 f64；舍入对二进制值取三位小数、平局取偶。

HTTP 使用精确固定的 ureq 3.4.2（仅 rustls feature，ring/WebPki），固定 URL/model、安全 TLS、无重定向。分阶段各 10 秒、最多两次尝试；仅首次 429/529 延迟 1 秒，发送完成前错误及超时立即重试一次。`transport.rs` 中的局部适配只在底层发送成功后累计 HTTP 头结束和 Content-Length 请求体字节，不依赖 Reader 已耗尽或 timeout.reason 推断发送完成。响应读取 16 MiB 上限，多读一个字节判超限；诊断不格式化库错误/请求头。旧脚本的已批准边缘差异见 [设计 §5.2](../../docs/dispatch插件接口设计.md)。

`corral-dispatch` 资源 revision=1；固定指纹测试要求改资源时更新 revision/期望对。项目模板逐字节保留；SKILL 只改问路由命令和兜底，其他编排规则及 dlog 说明留到阶段 05。

验证只使用合成输入、内部假传输/回环服务器和隔离 HOME。宿主集成测试编译同一份业务源码来注入假传输，无生产测试开关或换 URL 入口；HTTP 适配单独通过回环测试。未验证真实 JEV/TLS 或代理兼容性。源码接入不等于发布、实际安装或消费者切换；旧技能副本和链接仍保留，未迁移旧数据或实施 04/05。
