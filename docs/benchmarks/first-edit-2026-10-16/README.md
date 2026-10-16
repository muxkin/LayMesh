# 复跑与诊断

测试对象是交付目录 `video/04-LCSS与动画/exports/first-edit-preview-v009/` 中的独立 HTML 和 Linux x64 VSIX。原 v008 附件作为视觉与动画对照，保留不动。正式结果的命令、时间、退出状态和文件哈希见 `validation-status.json`、`environment.json`、`artifacts.json`。

在此 LayMesh 工作树中运行；需要 Python、Node、Playwright 的 Chromium/Firefox/WebKit、本机 VS Code，以及仓库的 Python 虚拟环境。没有 HTTP/CDN 依赖；网页测试直接 `file://` 打开文件。若另开人工预览服务器，按项目要求绑定 Tailscale 地址。

```sh
export PLAYWRIGHT_MODULE=/home/keqing/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright
python3 scripts/validate-first-edit.py \
  --html 'video/04-LCSS与动画/exports/first-edit-preview-v009/04_LCSS_离线可编辑_v005_首次编辑修复_v009.zip.html' \
  --baseline 'video/04-LCSS与动画/exports/animation-preview-v008/04_LCSS_离线可编辑_v005_并行动画优化_v008.zip.html' \
  --extension-dir target/first-edit-v009-extension \
  --output /tmp/laymesh-first-edit-rerun
python3 scripts/summarize-first-edit.py \
  /tmp/laymesh-first-edit-rerun/first-edit/chromium-raw.json \
  /tmp/laymesh-first-edit-rerun/edit-summary.json
python3 scripts/summarize-presentation-animation.py \
  /tmp/laymesh-first-edit-rerun /tmp/laymesh-first-edit-rerun/animation-summary.json
```

如果验证 staging 或源文件 fixture 不存在，可从本轮构建重新准备（不要覆盖已有手工修改的 fixture）：

```sh
# 仅在目标目录不存在时解出作品源文件；原附件不变。
node scripts/prepare-semantic-preview-fixture.mjs INPUT_HTML target/property-preview-fixture
python3 scripts/build-editors.py --binary target/release/laymesh --extension-dir target/first-edit-v009-extension
```

`--steps` 可以只跑指定阶段，逗号分隔：`progressive,cold,roundtrip,first-edit,animation,animation-edit,native-animation,native-keyboard,ordinary-export`。`animation-edit` 在共享 LCSS 首次修改后立即放映，用于检查编译与动画重叠。正式 `first-edit` 默认每类 50 个冷样本及紧接的 50 个暖样本；不要同时运行构建或其他测量。原生验证使用独立 VS Code 配置和扩展目录，不安装到日常配置。

单项契约测试：

```sh
node scripts/test-web-preview-state.mjs
node scripts/test-web-preview-compiler.mjs
node scripts/test-web-progressive-compile.mjs
node scripts/test-web-compiler-pool.mjs
node scripts/test-web-compile-slices.mjs
node scripts/test-wasm-allocation.mjs
cargo test --offline -p laymesh-core --lib session
cargo test --offline -p laymesh-core --test compile_session
```

分配器测试需先完成正常 WASM 构建，使用单独的测试模块模拟引擎最大内存，用于验证较大扩展失败后的最小需求重试；生产运行时没有该测试上限。`diagnostics/` 保存原因定位的单次对照，不能混入 50 次正式样本。

在用户 Chrome/Edge 上复现后，复制运行 `scripts/collect-first-edit-diagnostics.js` 到开发者工具控制台，下载首个及当前编辑阶段记录。无需在编辑前打开开发者工具。尚未取得原录屏中接近一分钟等待的对应记录。

原始编辑、动画及按键 JSON 以 `.json.gz` 无损保存；例如 `gzip -dc first-edit/chromium-raw.json.gz > /tmp/chromium-raw.json` 后可交给汇总脚本。`edit-summary.json` 包含 300 行逐样本阶段数据及分组统计；`animation-summary.json` 包含全部六页、两轮的帧间隔统计。`animation-v008-reference.json` 是同日上一轮保存的对照，不是本轮重新配对采集。测试启动顺序修正及失败记录说明见 `checks/harness-notes.md`。
