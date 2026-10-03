# A 图片猫猫部署记录

2026-10-03。用户明确要求「部署啊」。本轮切换安装入口，不停止宿主或真实 agent，等待用户自行重启。

- 源码 `7acceb321c418a11ee23fab63ebbdbe130068066`，构建时 main 干净；新包 `~/.local/share/saddle/versions/7acceb3`。包含 A 图片猫猫、既有 Auto/Blocks 与之前修复，另外三套宠物素材不变。
- 按 scripts/package.sh 使用共享 CARGO_TARGET_DIR 与 native target 构建完整 release 包。saddle/corral/Drover/Diff 四个程序 SHA256 与 BUILD.txt 一致。
- 0700 私有备份 `~/.local/share/saddle/backups/cat-a-7acceb3-20261003-120318` 保存旧入口、config.toml、插件注册、旧包 BUILD/清单及技能快照。旧版本目录保留。
- 先验证候选 plugin status；持 plugins.lock 核对备份基线后原子更新注册文件中的 Drover/Diff 路径，再分别原子切换 saddle/corral 链接。用户 config.toml 字节未变，其他插件和启用状态保留。
- 安装后公开 plugin status 回读两插件新路径、enabled、manifest_readable；两个命令链接均解析到 7acceb3。Corral 技能 dry-run 两处 same、written=false，未覆盖技能。
- 固定新包运行隔离 product 测试 1 passed：合成 TUI 退出重开后 agent 身份保持。未重复素材审查已通过的检查，未进行真实终端视觉验收。
- 新 Corral CLI 读取真实主控成功，instance=`faec2c2b00cb` 保持；部署前宿主公开 instance=`91ef8d20be4231cd`，本轮未重启宿主。没有操作真实 Tasks 或记录遥测。
- 日志 `/tmp/saddle-deploy-7acceb3-build.log`、`/tmp/saddle-deploy-7acceb3-product.log`、`/tmp/saddle-deploy-7acceb3-plugin-preview.json`、`/tmp/saddle-deploy-7acceb3-installed-plugins.json`；备份内有 snapshot.json 和 deployment.json。

待用户重启 Saddle，Settings → General 选择 Pet=Cat、Display=Auto 并保存；终端支持图片时显示 A 猫猫。Blocks 模式的猫猫不变。本记录不宣称运行中的旧宿主已加载新素材。
