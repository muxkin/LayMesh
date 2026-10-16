# 放映动画与后台并行准备，2026-10-16

## 结论与实现

旧运行时把当前页动画与其他页安装、入口画面及图层准备放在同一个 Worker 的串行请求队列中。每次绘制结束立即恢复预加载，其他页的同步安装可能占住下一帧。初步诊断测到当前页绘制约 1～2 ms、请求却等待约 177 ms；暂停后台的对照能减轻停顿，但不是并行方案。

本次将窗口及全屏放映分成独立的显示与准备 Worker。每页的绘图对象留在构建它的渲染器中，切页时交换前后台角色，不重新安装已有页面。跨渲染器转场只交接上一页的一张 ImageBitmap。渲染器保留已经显示的画布缓冲区；交接直接复制它，不重新采样和绘制上一页。当前动画不再等待另一渲染器的 CPU 工作，预加载在动画期间继续进行。已归当前渲染器所有的页面不会在动画期间向它插入后台任务；这类尚未完成的准备留待角色交换或前台请求时处理。首页面不等待后台运行时启动。

后台入口画面、末帧及动画图层用实际显示密度的 CPU 表面准备，避免两个渲染器同时向软件 GPU 提交整页预热。保留原始路径、字体、样式、动画时长和像素尺寸。CPU/GPU 抗锯齿并非逐像素相同，差异见下文。仅拆 Worker、仍做 GPU 后台预热的诊断中，软件 GPU 上封面 P95 约 71 ms；CPU 准备加二进制传输约 36 ms。这些是诊断试验，正式结果以下表为准。

初始离线归档和原生压缩页面直接传二进制，由 Worker 解包、解析，避免 UI 重复解压、构造和复制大对象。WebKit 的 `file://` Worker 无法读取跨环境 Blob，因此传递 ArrayBuffer/Uint8Array；不依赖网络或 blob URL 取页面内容。离线编辑后尚无归档字节的页面仍有一次 JSON 编码；后续可继续优化这条路径。源码编译的并行安排保持原实现。

另一个原有热点是普通揭示动画每帧克隆并序列化完整曲线；旧 v004 也有该成本，不能全归因于上一轮。新增 `presentation_state()` 输出裁剪与变换，渲染器复用普通揭示内容。完成后的描线复用原几何；图表揭示和真正变化的描线保持原有语义。跳过的 GPU 提交不再重置上一成功帧时间。

并行调度已通过验证，但性能目标没有全部达到。已准备页面的切换在 Chromium 达到中位数 ≤100 ms、P95 ≤200 ms；VS Code 为 117.2 / 197.5 ms，中位数仍超出目标。第二页动画明显改善；复杂页面、4K 和 CPU 后端仍有较高光栅化成本。也并非所有页面都变快：原生 motion 页首次中位帧间隔从 31.5 ms 增至 34.9 ms，WebKit 封面重复遍历从 233 ms 增至 240 ms。这些结果保留在下表，不把本轮改动等同于全面流畅或 60 fps。

## 测量方法与边界

- 基线为 v005 资源复用附件；结果为 v008 独立附件及对应 Linux x64 VS Code staging。正式批次串行运行，不与构建或其他浏览器测试并发。基线取自同机同日上一批次的完整记录，未重新运行相同基线；起止时间与来源保留在 measure-status.json。机器信息见 [environment.json](animation-playback-2026-10-16/environment.json)。
- 三浏览器在 1920×920 下真实按键自然播放六页两遍；Chromium 另在录屏的 3840×1840 尺寸播放前两页两遍。真实 VS Code 使用隔离配置、原生编译与 Webview 资源传输链路，六页两遍。
- 没有 seek、预演、等待全篇就绪或改变作品时长。首次与重复遍历分开。表中样本为正持续时间动画内的成功绘制回复；跳过的提交计入相邻成功帧间隔。
- **这些是绘制回复的间隔，不是显示器扫描输出或实机 60 fps 认证。** Chromium / VS Code 是软件 WebGL；Firefox / WebKit 是 CPU 后端。复杂页的光栅化仍可能慢，少量帧的 P95 只描述该次运行。
- 初始化列从渲染器初始化请求起，到首次激活回复结束；不包含打开 HTML、编译源码或全屏授权。重叠次数要求后台请求与另一渲染器的成功动画帧时间区间交叠，独立测试还验证阻塞后台任务时前台仍能完成一帧。
- 全部逐帧、页面和首次/重复结果见 [summary.json](animation-playback-2026-10-16/summary.json)，命令及退出状态见 [measure-status.json](animation-playback-2026-10-16/measure-status.json)。

## 录屏重点页面，1920×920

| 环境 / 页面 / 遍历 | 有效帧数 前→后 | 帧间隔中位数 ms 前→后 | 帧间隔 P95 ms 前→后 | 采样 P95 ms 前→后 |
|---|---:|---:|---:|---:|
| chromium / cover / 1 | 71→93 | 23.5→19.7 | 59.5→33.7 | 0.7→0.2 |
| chromium / seed / 1 | 81→233 | 71.0→25.4 | 81.8→30.8 | 39.1→0.2 |
| chromium / cover / 2 | 95→116 | 22.1→17.1 | 25.8→20.6 | 0.8→0.1 |
| chromium / seed / 2 | 92→250 | 67.2→24.0 | 79.5→27.7 | 40.7→0.3 |
| firefox / cover / 1 | 2→2 | 1782.5→1655.0 | 1940.0→1669.0 | 1.0→0.0 |
| firefox / seed / 1 | 21→39 | 284.0→168.0 | 316.0→178.0 | 116.0→1.0 |
| firefox / cover / 2 | 2→2 | 1621.0→1589.0 | 1642.0→1610.0 | 2.0→0.0 |
| firefox / seed / 2 | 23→40 | 280.0→165.0 | 290.0→175.0 | 109.0→1.0 |
| webkit / cover / 1 | 8→9 | 242.5→244.0 | 401.0→278.0 | 1.0→1.0 |
| webkit / seed / 1 | 94→209 | 64.0→30.0 | 70.0→33.0 | 39.0→1.0 |
| webkit / cover / 2 | 9→9 | 233.0→240.0 | 262.0→283.0 | 1.0→1.0 |
| webkit / seed / 2 | 89→212 | 61.0→30.0 | 70.0→33.0 | 37.0→1.0 |

## 录屏像素尺寸，3840×1840

| 环境 / 页面 / 遍历 | 有效帧数 前→后 | 帧间隔中位数 ms 前→后 | 帧间隔 P95 ms 前→后 | 采样 P95 ms 前→后 |
|---|---:|---:|---:|---:|
| chromium / cover / 1 | 15→26 | 99.9→78.1 | 539.2→127.7 | 1.4→0.2 |
| chromium / seed / 1 | 44→56 | 136.8→110.3 | 167.6→119.6 | 41.6→0.3 |
| chromium / cover / 2 | 26→31 | 86.3→71.7 | 105.8→101.6 | 1.1→0.2 |
| chromium / seed / 2 | 48→60 | 133.8→100.4 | 146.5→118.5 | 59.0→0.2 |

## 真实 VS Code 六页播放

| 页面 / 遍历标识 | 有效帧数 前→后 | 帧间隔中位数 ms 前→后 | 帧间隔 P95 ms 前→后 |
|---|---:|---:|---:|
| cover / serial 1 | 40→52 | 34.0→34.2 | 125.0→108.7 |
| seed / serial 2 | 88→177 | 71.7→33.5 | 84.0→38.2 |
| anchors / serial 3 | 29→29 | 88.5→87.2 | 105.6→97.4 |
| motion / serial 4 | 147→139 | 31.5→34.9 | 35.8→38.7 |
| offline / serial 5 | 65→73 | 32.0→30.3 | 67.9→67.5 |
| editable / serial 6 | 20→41 | 157.8→73.6 | 168.8→82.0 |
| cover / serial 7 | 76→83 | 26.7→24.7 | 32.1→28.5 |
| seed / serial 8 | 86→182 | 71.5→33.7 | 85.2→38.0 |
| anchors / serial 9 | 30→30 | 88.8→90.3 | 94.4→96.0 |
| motion / serial 10 | 154→137 | 31.5→35.4 | 34.9→40.0 |
| offline / serial 11 | 75→77 | 30.2→30.1 | 63.5→65.4 |
| editable / serial 12 | 20→42 | 153.5→71.8 | 174.3→80.9 |

## 首帧与并行证据

| 环境 | 渲染器初始化至首帧完成 ms 前→后 | 后台请求与前台帧实际重叠次数 |
|---|---:|---:|
| playback-after/chromium.json | 1061.3→913.6 | 21 |
| playback-after/firefox.json | 2490.0→2423.0 | 17 |
| playback-after/webkit.json | 721.0→687.0 | 22 |
| 4k-after/chromium.json | 1662.4→1628.7 | 20 |
| native-animation-after.json | 1917.1→2012.4 | 20 |

## 连续切页

保留原有切页按键保护。真实 ↓/→ 每 80 ms 输入一次，每个环境记录 50 次被接受的页面变更；末页后使用 Home。`firstTraversal` 是第一次遍历；`repeat` 是后续遍历；`prepared` 是按键时已有入口画面的页面。首次变化通知后再经过浏览器 RAF/task 计时；完整完成时间包含作品自身转场，不混作后台等待。原始 `queueWaitMs` 含浏览器调度且可能来自重叠请求，不能把相加值当作关键路径。

| 环境 / 范围 | 样本数 | 按键至开始变化 中位 / P95 ms | 包含作品转场的完成 中位 / P95 ms |
|---|---:|---:|---:|
| Chromium / all | 50 | 91.1 / 240.2 | 738.6 / 1015.9 |
| Chromium / firstTraversal | 5 | 240.2 / 673.5 | 857.2 / 1295.2 |
| Chromium / repeat | 45 | 82.6 / 166.3 | 733.1 / 965.0 |
| Chromium / prepared | 47 | 82.7 / 199.3 | 733.8 / 965.0 |
| VS Code / all | 50 | 123.1 / 325.4 | 752.0 / 1038.1 |
| VS Code / firstTraversal | 5 | 325.4 / 701.4 | 959.7 / 1301.8 |
| VS Code / repeat | 45 | 116.2 / 197.5 | 749.7 / 1009.4 |
| VS Code / prepared | 47 | 117.2 / 197.5 | 750.0 / 1009.4 |

## 正确性、像素差异与交付边界

- 三浏览器六页场景 JSON，以及每页 0 / 3 / 6.5 秒的完整绘制结果与 v005 逐像素一致。保存时包含待完成修改，随后重开、导出播放版并离线打开均通过，无外网请求和页面异常。见 [roundtrip.json](animation-playback-2026-10-16/roundtrip.json)。
- 实际缓存另做三浏览器、六页各五个采样点的 CPU/GPU 对照，并检查转场位图非空、关闭重开。Firefox/WebKit 都使用 CPU，差异为零；Chromium 的跨后端缓存最大平均通道差为 **0.541 / 255**，大于 8 的通道占比最高 **2.07%**，主要是密集曲线边缘。原始最大差异完整保留，不能称跨后端像素完全一致。见 [preparation-oracle.json](animation-playback-2026-10-16/preparation-oracle.json)；本地截图为数据目录中的 `preparation-chromium.cpu.png` 与 `preparation-chromium.gpu.png`。
- 校验曾发现等待 GPU 完成后再读屏可能取得已清空缓冲区，相关旧对照作废。最终测试要求非空；同步像素读回在重新绘制后立即执行，转场位图直接复制保留的显示缓冲区；尺寸变化或该缓冲区不可用时才恢复绘制。
- Rust 动画与变换测试共九项通过，包括 97 个效果、描线、图表坐标轴、父子动画及 1500 条线的紧凑揭示。前后台独立调度、资源保留、取消、帧交接、别名、关闭和单线程回退测试通过。
- 无 OffscreenCanvas 的回退及演讲者镜像模式仍使用主线程渲染，连续动画期间暂停同一渲染器的后台任务；本次并行验收覆盖窗口和全屏幻灯片模式。
- 本轮针对放映动画和预加载，没有重跑 `.lay` / LCSS / `.laydoc` 编辑的 50 次计量。上一轮编辑计量单独见 [资源生命周期报告](resource-lifecycle-2026-10-16.zh-CN.md)，不以本轮动画数据代替编辑延迟。
- 原附件及旧版本保留。最终 HTML 的 32 个非运行时条目逐字节保留。VSIX 与实际测试 staging 一致，包审计通过；未发布，也未安装到日常 VS Code 配置。文件大小与 SHA-256 见 [artifacts.json](animation-playback-2026-10-16/artifacts.json)。

## 复跑

```sh
export PLAYWRIGHT_MODULE=/home/keqing/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright
node scripts/test-presentation-animation.cjs INPUT_HTML OUTPUT_DIRECTORY
WIDTH=3840 HEIGHT=1840 PAGES=2 ENGINES=chromium node scripts/test-presentation-animation.cjs INPUT_HTML OUTPUT_DIRECTORY
.venv/bin/python scripts/test-vscode-keyboard.py --animation-rounds 2 --extension-dir EXTENSION_STAGE --binary NATIVE_BINARY --source target/property-preview-fixture/storyboard.laydoc --output OUTPUT.json
ENGINES=chromium SAMPLES=50 node scripts/test-presentation-keyboard.cjs INPUT_HTML OUTPUT_DIRECTORY
.venv/bin/python scripts/test-vscode-keyboard.py --samples 50 --extension-dir EXTENSION_STAGE --binary NATIVE_BINARY --source target/property-preview-fixture/storyboard.laydoc --output OUTPUT.json
node scripts/test-presentation-preparation.cjs INPUT_HTML OUTPUT.json
node scripts/test-web-semantic-roundtrip.cjs BEFORE_HTML AFTER_HTML OUTPUT_DIRECTORY
python3 scripts/summarize-presentation-animation.py RAW_DIRECTORY SUMMARY.json
node scripts/test-presentation-workers.mjs
node scripts/test-presentation-lifetime.mjs
node scripts/test-retained-presentation-store.mjs
node scripts/test-presentation-frame-delivery.mjs
cargo test --offline -p laymesh-render --test animation --test transform_planning
```
