# Skia playback workloads

`benchmark.html` is served at `/skia-benchmark.html` by the documentation build.
It uses the production Rust/WASM compiler, sampler and shared CanvasKit renderer,
with a fixed 3840 × 2160 drawing buffer. `stress-scenes.mjs` generates deterministic
LAY and LCSS inputs; all chart values are synthetic analytic signals.

The ten levels introduce 64 and 256 vector motifs, 6 and 12 charts, 24 mathematical
formulas, translucent nested cards, shadows/glows, parallel 2D motion, then
perspective and stroke animation. The final level has 512 motifs, 24 charts and
48 formulas. Charts combine three sampled line series with scatter points or
16 × 16 / 24 × 24 heatmaps. Formula cases include integrals, derivatives, sums,
matrices, fractions, limits and Fourier transforms.

The early levels animate only a title so that additional scene complexity can be
measured separately from the many-object animations introduced in levels 8–10.
Each level compiles once before its first real playback. The measurement loop receives only frame
properties and metrics; it does not capture RGBA or rebuild the scene.

Choose a starting level and optionally **Only this level**. Quick mode measures
one 10-second round; full mode measures five 30-second rounds. There is no
fixed-duration warm-up or excluded rehearsal round. First-play cache construction
is included in that first round. The default 60 fps cap preserves 4K resolution.
Display-refresh mode exposes additional load on high-refresh displays. Stop
retains completed rounds; results can be downloaded as JSON.

The **Static-region GPU cache** switch supports an A/B comparison with the same
workload, buffer and frame cap. Static roots between moving objects are cached
as device-aligned GPU tiles; changing the viewport rebuilds them. The **Moving-object GPU cache** switch independently controls reusable animated
group content. It is enabled by default; disable it to compare with `static-tiles-v1`.
Groups with changing geometry or animated descendants retain direct drawing. `rendererVersion` and
`cacheStatic` / `cacheLayers` identify the compared pipeline; `staticTexturePeakBytes`,
`staticCacheHits` and `staticCacheBuilds` measure the explicit tile cache.
`cachePeakBytes` is Skia's own reported resource-cache value, not total GPU memory.

Reports contain input parameters, actual lowered node/path/glyph/effect counts,
animated targets, preparation phases, per-round sampling and submission P95,
unique completed GPU timer queries, GPU cache high-water marks, refresh intervals
and estimated delivered frames. `requestFps` counts paint requests (including busy-GPU
skips); `submissionFps` counts completed worker draw submissions. The presentation
estimate counts worker replies seen on animation-frame callbacks, not direct
GPU-completion or compositor scan-out observations. The table compares GPU P95 to the preceding
selected stage. GPU timer duty estimates exclude browser composition, Copy
engines and other processes; they are **not total-device utilization**. Compare
with Task Manager manually. Software drivers and refresh callbacks alone do not
establish hardware presentation acceptance.

The existing three Animate.css scenes remain selectable as a baseline. The
[user-supplied discrete-GPU measurements](user-hardware-summary.json) apply only
to that earlier baseline. The [progressive measurement before static tile caching](user-stress-baseline.json)
records one 10-second round per level on an RX 6750 GRE in display-refresh mode:
158.0, 157.2, 142.6, 101.5, 76.4, 69.6, 51.8, 47.9, 37.8 and 17.2 estimated fps.
The first sub-60 level adds shadows/glows. The last level has 63.9 ms submission
P95 and 48.3 ms GPU P95, versus 1.2 ms sampling P95. Submission includes driver
waiting; these timings overlap and must not be added. This is a windowed 4K
buffer test, not fullscreen scan-out or integrated-GPU acceptance.

The [same-device static-cache A/B](user-static-cache-ab.json) measures level 7 on
that RX 6750 GRE with matching scene counts, 4K buffer and display-refresh mode.
Cache off/on gives 54.24/159.90 estimated fps, 16.70/0.90 ms submission P95 and
23.21/1.24 ms GPU P95. Cached tiles occupy 59.10 MiB; the measured round records
3200 hits and no rebuilds. Each setting has one 10-second measured round after
warmup. This validates reuse of the static scene around one moving title;
it does not establish performance for the many animated targets in levels 8–10,
integrated GPUs or physical scan-out. Preparation remains separate (1.88/2.14 s
for off/on); this comparison does not establish a compilation improvement.

## 递增压力场景

测试页默认按十档逐步增加复杂度，也可选择起始档并勾选“仅当前档”。最后一档包含
512 组矢量图案、24 张多序列或热力图、48 个复杂公式，叠加嵌套布局、透明卡片、
阴影、光晕、透视翻转和描线。数据为确定性的合成数据，可由同一生成器复现。

各档准备与编译单独计时，播放期间不重新编译、不发送整幅像素。表格显示对象数量、
动画对象、采样与绘制耗时、GPU 耗时相对前档的倍率、呈现估计及缓存峰值。GPU 查询
不包含浏览器合成和其他程序，请结合任务管理器观察独显的 3D、Copy 和显存变化。
核显实测后续补充，不用当前独显成绩替代。

新增“静态区域 GPU 缓存”开关用于同场景 A/B 对比。缓存使用当前绘制缓冲区的像素
精度，不降低分辨率，不向主线程传输 RGBA。窗口/缩放改变时重建，内容编辑按连续
静态区域失效，暂停与销毁时释放。纹理峰值与命中次数单独记录；原来的 Skia 缓存
计数不能作为整卡显存占用。移动和透视对象仍按当前帧绘制。

[优化前独显实测](user-stress-baseline.json)为 RX 6750 GRE、160 Hz 跟随屏幕模式、
每档一轮 10 秒：第 7 档加入阴影/光晕后约 51.8 fps，最后一档约 17.2 fps。
最后一档采样 P95 为 1.2 ms、提交 63.9 ms、GPU 48.3 ms，绘制是主要问题。
提交时间可能包括驱动等待，不能与 GPU 时间简单相加，也不能据此推算核显帧率。

[同机缓存开关对照](user-static-cache-ab.json)确认第 7 档的优化：关闭/开启缓存时，
帧率估计为 54.24/159.90 fps，提交 P95 为 16.70/0.90 ms，GPU P95 为
23.21/1.24 ms。静态纹理约 59.10 MiB，测量期间命中 3200 次、重建 0 次。
场景数量、4K 缓冲区及跟随屏幕设置一致，各测一轮 10 秒。这一档只有标题运动，
首次编译耗时未因此消除；第 8–10 档后续实测见下文，核显验收仍待补充。


## Moving-content reuse / 运动内容复用

[第 8–10 档同机静态缓存对照](user-motion-static-cache-ab.json)已归档。缓存关闭/开启的
帧率估计分别为 46.22/74.00、37.41/51.14、16.18/22.06；开启后 GPU P95 分别为
16.18、20.72、50.09 ms，而采样 P95 为 0.4、0.8、1.4 ms。各测一轮 10 秒，
不是稳定 60 Hz 呈现验收。后两档说明还需要减少运动内容的重复绘制。

`object-layers-v1` 缓存内容不变的运动群组，复用图表、公式和已有阴影/光晕的 GPU
结果，更新变换和透明度。使用当前投影密度上界，必要时提高密度并重建；预算不足
不降低密度。图层按 2048 像素分块，保留采样边缘。动静纹理共用有界预算，保留前一帧
的活动资源，避免逐帧互相淘汰。暂停、视口变化、上下文丢失和销毁均清理纹理。
描线和区域揭示的变化几何不复用旧像素，子对象独立运动时不缓存整个父群组。

新增的运动缓存默认开启，可单独关闭作 A/B。结果包含 `layerTexturePeakBytes`、
`layerCacheHits`、`layerCacheBuilds`、`layerCacheBudgetMisses` 和
`layerCacheAllocationFailures`，用来区分实际复用、精度提高及容量压力。
[缓存生命周期测试](../../scripts/test-skia-layer-cache.cjs)覆盖第 8–10 档、暖缓存、
改变姿态、局部编辑、缩放、预算边界、上下文恢复及暂停释放。软件 GPU 的这些检查
只证明代码路径和资源管理，不用于报告硬件加速倍率。

之前认可的画面是基准。允许用户已确认可接受的轻微抗锯齿/采样差异，不以逐像素
相等为性能优化门槛；不降低输出分辨率、简化内容、删除效果或复用过期状态。
当前增加的是复用工作，新的复杂场景硬件帧率仍需实测。

The animated-content cache preserves the requested output resolution and effects.
It reuses GPU tiles while updating transforms/opacity, raises density when needed,
and avoids evicting the previous frame's active working set. Cache hits, builds,
admission failures and bytes are reported separately. Local software-driver tests
establish functionality, not hardware throughput.

## Implementation references

- [Chromium property trees and tile management](https://raw.githubusercontent.com/chromium/chromium/main/docs/how_cc_works.md): separate transform/effect updates from content and manage reusable raster resources.
- [WebRender rendering overview](https://firefox-source-docs.mozilla.org/gfx/RenderingOverview.html): retained pictures, separate effect tasks, final-scale text rendering and local tile invalidation.
- [Flutter raster cache](https://raw.githubusercontent.com/flutter/flutter/master/engine/src/flutter/flow/raster_cache.cc) and [saveLayer guidance](https://docs.flutter.dev/perf/best-practices): reuse offscreen work and avoid unnecessary target switches while retaining group compositing semantics.
- [Skia GPU text](https://raw.githubusercontent.com/google/skia/main/src/text/gpu/SubRunContainer.cpp): a future candidate is using positioned glyph APIs instead of replaying each glyph outline. The protocol already carries Rust-shaped glyph IDs/positions; this optimization has not been implemented here. Current upstream source explains mechanisms, not a guarantee about every path in pinned CanvasKit 0.42.0.

## Compilation delay / 页面编译等待

用户于 2026-10-11 要求停止继续优化绘制，改查第 10 档开始前的等待。
[两份原始测量](user-compile-delay-baseline.json)显示该档准备约 13.5 秒，其中编译
约 12.8 秒，生成绘制指令约 20 毫秒。两份报告虽标记运动缓存开启，但相应计数
均为零，因此不将它们作为该缓存已生效的硬件性能证据。本轮未修改绘制策略。

根因是分词时计算 `Loc`：每个 token 都重新扫描源码前缀中的换行，以及所在行的
UTF-16 列数。第 10 档有 648,275 字节源码，大量数值导致重复扫描成本接近平方增长。
解析器现在顺序推进位置，保留字节偏移、从 1 开始的行号与 UTF-16 列号。

[三次本机对照测量](compile-profile.json)的中位数：

| 环境 / 阶段 | 修复前 | 修复后 |
| --- | --- | --- |
| 原生，第 10 档分词 | 16,585.72 ms | 5.66 ms |
| 原生，第 10 档完整编译 | 17,419.07 ms | 449.78 ms |
| 浏览器 WASM，第 10 档完整编译 | 18,648.80 ms | 1,130.50 ms |

原生与浏览器不能互相比作提速倍率，也不能用本机数字替代用户电脑。分词、解析
包含于编译计时，不能相加。第 3、9、10 档修复前后的原生场景、浏览器绘制指令及
inspection 校验值完全相同。真实 Worker 流程的一次补充运行中，第 10 档编译约
722 ms、整体准备约 1,821 ms；不同运行的 JIT/负载会影响时间，不能混用最优值。

复现入口为 [原生分项测量](../../crates/laymesh-core/examples/profile_compile.rs) 和
[浏览器测量](../../scripts/profile-stress-compile.cjs)，使用本目录的同一压力场景生成器。
位置回归覆盖中文、补充平面字符、多行/格式化字符串、跳过的注释和样式块、错误及 EOF。

上述词法分析修复当时仍按需编译所选页，未包含整份演示准备。
源码编辑可继续按需编译；若正式播放要求翻页立即完成，还需要在进入演示前准备页面，
并缓存编译结果及所需资源。仅把准备时间从报表中扣除不会消除实际等待。

The lexer now advances UTF-16 source positions incrementally. The same generated
inputs produce identical scene/display-list/inspection hashes. These local compiler
measurements are independent of raster performance. Document-wide presentation
preparation is a separate mechanism, now implemented below; editing page selection remains lazy.


## Whole-document presentation, no rehearsal

`presentation.html` is served at `/presentation-test.html`. By default it opens
the [32-page academic deck](../presentation/academic/README.md), with 141 plots,
119 formulas, 97 Animate.css effects and the draw/reveal extensions. This is
analytical/computed teaching material with reproducible sources and values.
Use `?deck=stress` for the older workload, which combines the real
levels 8–10 into one `.laydoc`, prepares all three pages, and opens manual
presentation at time zero. It uses the shared production controller, compilation
worker, artifact store, sampler and CanvasKit renderer. The five-round button
uses a fixed 3840×2160 buffer and navigates 1→2→3→2→1 for the stress deck or
1→2→3→18→25→32→31→30→1 for the academic deck each round. It never plays
animations merely to prepare caches. Close with Esc and download the JSON.

`prepareMs` covers the complete provider preparation (including initialization,
compilation and artifact storage). `transitions[].firstFrameMs` measures navigation
input through completed draw reply and the following animation-frame callback.
The clock starts only afterwards. `resident` records whether the page was already
in the rendering cache when the input arrived; nonresident restoration is reported
separately. The first transition is retained. Background neighboring-resource work
that delays an input is included. These observations are not compositor scan-out
measurements. Compare resident P95 to 100 ms only on the target real hardware;
software-driver timings are functional evidence, not hardware acceptance.

Production checks:

```sh
node scripts/test-presentation-clock.mjs
node scripts/test-presentation-wheel.mjs
node scripts/test-presentation-browser.cjs http://100.64.0.6:8769/ /tmp/presentation-browser
node scripts/test-presentation-ui.cjs http://100.64.0.6:8769/ /tmp/presentation-ui
node scripts/test-academic-presentation.cjs 'http://100.64.0.6:8769/presentation-test.html?deck=academic' /tmp/presentation-academic
node scripts/test-presentation-host.cjs /tmp/presentation-native
python scripts/test-vscode-preview.py --output /tmp/presentation-vscode
# Include the same academic deck in the real VS Code webview tests:
LAYMESH_ACADEMIC_DOC="$PWD/tests/presentation/academic/report.laydoc" python scripts/test-vscode-preview.py --output /tmp/presentation-academic-vscode
```

The browser suite checks whole-document preparation, manual/automatic controls,
rapid navigation, storage overflow, cancel/retry, fixed content versions and
rendering without compilation or RGBA frame transfers. The native suite checks
unsaved snapshots, duplicate page sources, the scoped Tailscale service and browser
playback of native artifacts. VS Code checks real embedded playback, narrow/light
and wide/dark layouts, source recovery and unchanged canvas bounds when diagnostics
open. Existing 97-effect CSS reference and export tests remain the animation
correctness baseline. No new hardware acceptance is inferred from these tests.
