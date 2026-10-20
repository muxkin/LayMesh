# Open Props 动画与资源占用验证

本文保留 v24 的实现和测量口径。后续 CPU 合成、后台调度与首次进入修复见 [后续验证](followup.zh-CN.md)；后续测试按用户要求暂缓 WebKit，不能从旧记录推断新版本已通过 WebKit。

本轮对象是附件中的 26 页 Open Props 可编辑作品。三维效果工作已停止，本轮不发布版本。作品源码、字体、图片、动画时长和显示分辨率保持不变。

## 实现

- 前台和后台各自持有 Worker、CanvasKit 渲染器及已准备资源。后台使用实际可用的 GPU；没有 GPU 时使用相同缓存规则的 CPU 回退。后台分块提交并等待 GPU fence，在任务边界接受取消；前台帧耗时反馈控制下一块的提交间隔。不同 Worker 可以并发准备和提交，GPU 是否同时执行由驱动决定。
- 每层阴影从主体边界独立计算，最终取并集。缓存昂贵静态对象时使用对象自身的可见范围，避免把背景及远距离对象之间的空白保存成整页纹理。
- 动画对象按内容缓存，位移、旋转和透明度仅改变合成。提前计算时间轴所需的投影采样密度；暂停和终帧继续使用相同内容缓存。入口和终态保存合成状态，不再重复保存整页像素。
- 缓存绘制保留原有多重采样抗锯齿，但只保留 resolve 后的单采样像素，避免每块缓存同时保留 MSAA 附件。同一渲染器的不可变图片按资源身份共享解码结果和编码字节；不同页面的相同静态绘制内容，在资源身份、放置和视口一致时共享像素。共享资源由最后一个使用者释放。
- 隐藏的后台渲染器只需要上下文和分块工作面，释放未显示的整页画布；内容缓存和采样密度仍按最终视口计算，激活时直接合成。切页转场需要的位图先完成所有权转移，再释放旧画布。
- 静止保持阶段继续推进作品时钟，只在可见状态、媒体帧或视口变化时绘制。菜单及退出不等待页面准备。后台让出时间可被新目标或暂停消息唤醒，避免旧帧产生的长延时阻挡已准备页面；进入静止保持后解除旧的提交间隔。
- CPU 回退对显式阴影轮廓复用覆盖度遮罩，以线性时间的两次滑动最小／最大值计算替代重复的逐层收缩／扩张。遮罩保留原采样密度、透明边缘、像素相位及 Skia 的半径取整；旋转等不适用情况继续使用原有路径。模糊外扩较大时，输出块随输入外扩范围调整，避免大量小块反复处理同一模糊区域。
- 退出后暂停同一文档的两套运行时，释放显示缓冲，保留仍有效的内容；再次打开复用 WASM 堆、上下文及缓存。文档内容失效或整体关闭时销毁运行时。最终销毁先释放绘图资源、关闭图形上下文，再终止 Worker。
- CPU 放映缓存生成时，用复用的 Alpha8 临时缓冲检查静态块内部是否完全不透明。仅这些像素对齐的块使用 Src 直接复制；含透明边缘的块继续使用原混合方式。该检查不读回 GPU，不在每帧执行，准备和绘制结束时释放临时缓冲。
- CPU 放映使用预乘像素缓冲和可复用的显示缓冲。颜色转换由当前 CanvasKit 生成的 64 KiB 查表保持一致；静态缓存块直接在屏幕像素位置绘制。跨 CPU Worker 直接转移已有 RGBA 缓冲，避免经过两个 Canvas2D 和 ImageBitmap 的转换。显示缓冲与传输字节单独计入诊断。
- 淡入淡出仅在能够从页面背景和当前状态证明完全不透明的交集内部使用直接图片合成。透明区域、抗锯齿边缘和不同尺寸快照继续使用原始着色器；其他转场保留原算法。该优化不改变时长、分辨率或半透明对象的页面合成方式。CPU 目标页先合成到内部缓冲，第一次转场绘制时才呈现，避免多余的输出转换和旧画面复制。不透明区域的首次拷贝及已清空画布上的终态使用 Src 替换；半透明混合保留 SrcOver。正常结束时不重复绘制已经呈现的终帧。

CPU 放映对顶层、接近一比一采样的平移图层直接合成预乘像素，保留真实亚像素位置、双线性取样和透明度。像素数组替换原生图像，长期只保留一种表示；旋转、缩放或嵌套裁剪继续使用 Skia，恢复图像后不反复转换。首次转换及其临时字节单独记录，稳定帧统计排除转换帧，但首次可见时间和逐页帧间隔保留这些开销。该路径不改变 GPU 绘制，也不降低缓存分辨率。

没有新增页数、RAM 或 VRAM 配额；没有降低分辨率、删除效果、缩短时间轴或周期性清空缓存。

## 测量口径

自然播放测试从打开作品立即开始，按作品原有时长播放，随后用真实方向键进入下一页；第二轮重复相同页面顺序。性能计时不使用提前准备完全部页面、跳至终态或截图读回。

首次与冷切页指新浏览器文档及尚未准备的内容缓存；没有强制清除操作系统文件缓存或驱动磁盘着色器缓存。三个版本采用相同条件，这些结果不代表一台全新系统上第一次启动驱动的耗时。

分别记录：解码、安装、采样、光栅化提交、GPU timer、资源 fence 等待、请求排队、呈现确认、缓存分配和释放、Rust WASM 活跃分配及堆容量、CanvasKit 堆容量、浏览器进程树 PSS，以及可用的 Linux DRM 客户端数据。请求返回间隔和 fence 完成不等于物理显示器扫描输出时间。

CPU 像素数组属于逻辑内容缓存，`cpuTileBytes` 是 `liveTextureBytes` 的子集，不能再次相加。迁移时计入短暂的新旧表示和工作缓冲，原表示随后释放；原生图像与 JavaScript 像素数组不会长期重复保留。CanvasKit 堆容量变小本身不作为内存下降证据，整体比较仍使用完整浏览器进程树 PSS。

新运行时将跨 Worker 交接时间与请求排队时间分开记录；排队时间从主线程实际发送请求开始计算。早期诊断中的排队字段包含了发送前的快照交接时间，不能直接当作相同口径比较。原始交互的首次变化时间仍从用户操作开始计算，没有移动起点。

RGBA 纹理字节只是逻辑估算；本机 AMD 的 DRM VRAM/GTT 是驱动实测驻留，两者分开报告。集成显卡的 VRAM 指驱动内存域，不等于独立显卡板载显存。GPU 进程 PSS 也不等于显存。缺失数据记为未测，不能记作零占用。

进程树按 `/proc/*/status` 的 PPid 图枚举，包含工作线程创建的子进程；每次样本记录发现、读取成功和已退出／无法读取的进程数量，以及采样耗时。早期只读取主线程 `children` 的采样器漏掉 Firefox 等浏览器的子进程，这些历史 PSS／DRM 数据作废，不用于资源降幅。最终自然播放全部使用 `proc-ppid-v2` 重测，恢复测试也以该协议为条件。

DRM 按设备和客户端 ID 去重。本机 Linux 6.12 AMD 实现按实际后备存储所在内存域累计 `drm-memory-*`，并另列其中共享缓冲的 `drm-shared-*`。原始字段分别保留，不从客户端求和推断全局独占物理显存。客户端驻留总量可能包含共享缓冲，不能直接称为显卡全局独占物理用量。本次硬件记录中主要分配集中于一个客户端，另外两个客户端各占 12 KiB VRAM。

性能测试依次执行，避免多个浏览器或构建任务互相争用资源。正确性测试另外执行，可通过直接绘制与缓存绘制的像素读回比较透明叠加、阴影、中间态和终态。

性能诊断每 250 ms 传出到浏览器之外，由测试进程保存；浏览器不长期保留逐帧日志，避免旧运行时在静止阶段多画的帧数污染内存对比。进程与驱动数据每 2 秒采样，报告的是采样峰值。更早保留完整浏览器日志的诊断轮次只用于定位，不作为最终资源验收。

## 复现入口

```sh
python scripts/build-web-export.py --skip-wasm
node scripts/refresh-web-runtime.mjs BEFORE.zip.html OPTIMIZED.zip.html

ENGINES=chromium HARDWARE_GPU=1 ROUNDS=2 \
  node scripts/test-openprops-performance.cjs OPTIMIZED.zip.html OUTPUT
node scripts/test-openprops-interaction.cjs OPTIMIZED.zip.html INTERACTION_OUTPUT
node scripts/test-animation-cache-quality.cjs QA_BASE_URL QUALITY_OUTPUT
python scripts/summarize-openprops.py OPTIMIZED.zip.html RUNS SUMMARY.json
```

`scripts/benchmark-openprops.py` 提供顺序执行的分辨率、浏览器与运行时对照矩阵。`PLAYWRIGHT_MODULE` 可指定已安装的 Playwright；`WIDTH`、`HEIGHT` 指定视口像素，默认 1920 × 920。硬件测试使用显式 Vulkan ANGLE 启动参数，并以返回的设备字符串确认实际后端。

当前源码中的 `test-animation-cache-quality.cjs` 默认比较当前渲染器的直接绘制与缓存绘制，适合无需历史夹具的 CI。需要另外比较历史直接绘制时，设置 `BASELINE_RENDERER_MODULE=baseline/skia-renderer.mjs`，并使用验证包中包含该模块的 QA 服务。历史验证包保留当时实际执行的脚本。

冷切页使用 `SAMPLES=20 ENGINE=chromium HARDWARE_GPU=1 node scripts/test-openprops-cold-navigation.cjs FILE.zip.html NEW_OUTPUT`。每个样本新开浏览器，从 P15 开始，观察后台 P16 请求实际提交，150 ms 后用 `16` + Enter 提前切页；它与自然播放测试分开。超时样本保留为失败，不纳入成功延迟后假装整体通过。

验证压缩包包含实际使用的 `qa/` 夹具、基线渲染器、WASM、CanvasKit、逐对象数据和测试脚本。在解压目录运行 `python source/scripts/serve-preview.py --directory qa --bind YOUR_TAILSCALE_IP --port 18791`，再以该地址运行画质与裁剪脚本即可复现。CPU 逐对象检查使用 `ENGINE=webkit SOFTWARE=1 node source/scripts/test-openprops-quality.cjs http://YOUR_TAILSCALE_IP:18791 NEW_OUTPUT`。重跑时新建输出目录，保留历史记录。

诊断中的 `openprops-phase-probe.cjs` 只用于逐阶段定位，会在测试进程中插入计时，不改写交付 HTML。自然播放验收不启用该探针。它区分合成、显示缓冲转换、蒙版生成和单个阴影绘制；诊断样本不混入最终性能分布。

设计参考：[Chromium 合成与分块栅格化](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/how_cc_works.md)、[Chromium GPU command buffer](https://www.chromium.org/developers/design-documents/gpu-command-buffer/)、[Flutter 昂贵离屏层建议](https://docs.flutter.dev/perf/best-practices#use-savelayer-thoughtfully)。这些资料提供设计依据，性能结论以本次测量为准。

同尺寸图像绘制的 API 语义另核对 [Skia CanvasKit 绑定源码](https://github.com/google/skia/blob/main/modules/canvaskit/canvaskit_bindings.cpp)。具体优化是否有效仍以当前打包版本的像素和耗时对照为准。

驱动字段按 [Linux DRM 客户端统计定义](https://docs.kernel.org/6.12/gpu/drm-usage-stats.html) 解释；资源数据的采样范围、去重方式与缺失字段都保留在记录中。

本机驱动的驻留与共享统计还核对了 [Linux 6.12 AMD 缓冲统计源码](https://github.com/torvalds/linux/blob/v6.12/drivers/gpu/drm/amd/amdgpu/amdgpu_object.c#L1103)。

验证包中的 `records/runtime-changes.patch` 比较优化前离线包与本次候选的运行时源码模块，便于把本轮动画改动与工作区此前的三维改动分开审查。Rust 和构建代码另有冻结源码及哈希；压缩包不是独立仓库或发布包。

软件驱动的额外后端诊断可以用 `ENGINES=chromium WIDTH=3840 HEIGHT=1840 PAGES=2 ROUNDS=2 FORCE_CPU=1 node scripts/test-openprops-performance.cjs FILE.zip.html NEW_OUTPUT` 复现，再以 `FORCE_CPU=0` 比较。该标志仅在测试注入时令放映 Worker 使用 CPU，不改写离线包，也不用于自然播放矩阵。首两页的局部收益不能代表完整作品或所有后端。

新增 CPU 合成路径可用 `node source/scripts/test-cpu-layer-blit.cjs QA_BASE_URL NEW_OUTPUT` 复现。该测试在三个浏览器中比较冻结的 v23 Skia 合成与新路径，检查真实 P12/P17、透明背景、旋转和嵌套裁剪，并验证每块内容只有一种长期表示。透明画布用预乘 RGBA 比较，以避免接近零的 alpha 放大直通颜色的舍入差。
