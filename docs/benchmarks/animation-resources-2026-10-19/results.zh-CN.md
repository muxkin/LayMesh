# Open Props 动画与实际资源占用对照

自然播放优化版已记录 8 / 8 个环境组合，其中 7 个完成全部两轮且无任务错误。报告保留未通过及未测项目，不以界面没有卡死代替验收。

作品为原附件的 26 页；源码、字体、图片和显示列表共 64 项逐项 SHA-256 相同，改变 0 项。没有缩短动画或降低输出分辨率。

测试环境：AMD Ryzen 7 8745H w/ Radeon 780M Graphics，16 个逻辑 CPU，Linux 6.12.73+deb13-amd64。浏览器依次测试，版本及实际后端见下表。

测量期间主机重启，启动时间约为 2026-10-20T00:07:38.819534+00:00（UTC）。已完成记录保留原时间戳；中断的软件 4K 记录不参与汇总，恢复后从第一页重跑。硬件 1920 配对结果来自重启前；其余配对的时间与运行环境边界见 environment-resume.json，不将跨启动批次当成同一次连续实验。

## 自然首遍与重复遍历

每个完整记录应完成 52 次访问（26 页 × 2 轮）。失败记录保留；未完成记录的内存峰值不能与完整遍历直接计算降幅。测试遵循作品原有停留时间，没有提前等待全篇准备、跳到终态或在计时期间截图。

| 记录 | 完成访问 | 状态 | 实际后端 | PSS 采样峰值 / MiB | DRM VRAM 采样峰值 / MiB | 已准备切页 P95 / ms | 保持阶段重复绘制 |
|---|---|---|---|---|---|---|---|
| baseline-chromium-1920-default | 52 | failed | Skia WebGL2 (software driver) | 2,085.5 | 未测 | 103.9 | 36 |
| baseline-chromium-1920-hardware | 52 | passed | Skia WebGL2 | 1,624.2 | 454.1 | 33.0 | 36 |
| baseline-chromium-3840-default | 7 | failed | Skia WebGL2 (software driver) | 3,736.8 | 未测 | 543.7 | 5 |
| baseline-chromium-3840-hardware | 10 | failed | Skia WebGL2 | 2,372.0 | 1,361.3 | 586.3 | 6 |
| baseline-firefox-1920-default | 13 | failed | Skia CPU | 1,696.1 | 未测 | 3,814.0 | 8 |
| baseline-firefox-3840-default | 0 | failed | 未测 | 1,360.1 | 未测 | 未测 | 0 |
| baseline-webkit-1920-default | 52 | passed | Skia CPU | 2,187.1 | 未测 | 714.0 | 35 |
| baseline-webkit-3840-default | 1 | failed | Skia CPU | 3,005.4 | 未测 | 未测 | 0 |
| optimized-chromium-1920-default | 52 | passed | Skia WebGL2 (software driver) | 1,441.7 | 未测 | 164.8 | 0 |
| optimized-chromium-1920-hardware | 52 | passed | Skia WebGL2 | 1,028.2 | 375.1 | 34.4 | 0 |
| optimized-chromium-3840-default | 52 | passed | Skia WebGL2 (software driver) | 2,119.9 | 未测 | 458.6 | 0 |
| optimized-chromium-3840-hardware | 52 | passed | Skia WebGL2 | 1,032.4 | 1,324.3 | 46.2 | 0 |
| optimized-firefox-1920-default | 52 | passed | Skia CPU | 1,679.8 | 未测 | 389.0 | 0 |
| optimized-firefox-3840-default | 52 | passed | Skia CPU | 2,632.9 | 未测 | 359.0 | 0 |
| optimized-webkit-1920-default | 47 | failed | Skia CPU | 2,026.9 | 未测 | 未测 | 0 |
| optimized-webkit-3840-default | 52 | passed | Skia CPU | 3,255.2 | 未测 | 351.0 | 0 |
| original-chromium-1920-default | 52 | failed | Skia WebGL2 (software driver) | 2,148.5 | 未测 | 92.5 | 36 |
| original-chromium-3840-default | 7 | failed | Skia WebGL2 (software driver) | 3,836.8 | 未测 | 1,147.8 | 5 |
| original-firefox-1920-default | 13 | failed | Skia CPU | 1,683.1 | 未测 | 3,625.0 | 8 |
| original-firefox-3840-default | 0 | failed | 未测 | 1,379.3 | 未测 | 未测 | 0 |
| original-webkit-1920-default | 52 | passed | Skia CPU | 2,279.2 | 未测 | 702.0 | 36 |
| original-webkit-3840-default | 1 | failed | Skia CPU | 3,003.9 | 未测 | 未测 | 0 |

PSS 按 /proc 中完整 PPid 进程树采集，包含非主线程创建的子进程。早期仅遍历主线程 children 的记录已作废，不用于资源降幅；DRM 为 Linux 驱动客户端驻留数据，按设备与客户端 ID 去重，每 2 秒采样。驻留量包含共享缓冲，不等于显卡全局独占物理用量。共享缓冲字段另存 JSON，并保留原始客户端记录；不从客户端求和推断显卡全局独占物理用量。集成 GPU 的 VRAM 是驱动内存域，不等于独立显卡板载显存；软件驱动或缺少 DRM 数据时记为未测。

## 两轮结束时的驻留量

以下为相同操作位置的实际采样，属于本次有限遍历的证据，并不代表任意长时间运行的泄漏证明。缓存未因为轮次而清空。

| 记录 | 轮次 | PSS / MiB | DRM VRAM / MiB |
|---|---|---|---|
| baseline-chromium-1920-default | 1 | 1,961.6 | 未测 |
| baseline-chromium-1920-default | 2 | 1,993.6 | 未测 |
| baseline-chromium-1920-hardware | 1 | 1,561.0 | 345.2 |
| baseline-chromium-1920-hardware | 2 | 1,612.2 | 345.2 |
| baseline-webkit-1920-default | 1 | 2,151.7 | 未测 |
| baseline-webkit-1920-default | 2 | 2,168.5 | 未测 |
| optimized-chromium-1920-default | 1 | 1,150.4 | 未测 |
| optimized-chromium-1920-default | 2 | 1,123.4 | 未测 |
| optimized-chromium-1920-hardware | 1 | 727.2 | 345.5 |
| optimized-chromium-1920-hardware | 2 | 732.1 | 352.6 |
| optimized-chromium-3840-default | 1 | 2,033.4 | 未测 |
| optimized-chromium-3840-default | 2 | 1,977.1 | 未测 |
| optimized-chromium-3840-hardware | 1 | 756.9 | 1,126.3 |
| optimized-chromium-3840-hardware | 2 | 761.3 | 1,129.6 |
| optimized-firefox-1920-default | 1 | 1,583.3 | 未测 |
| optimized-firefox-1920-default | 2 | 1,674.5 | 未测 |
| optimized-firefox-3840-default | 1 | 2,491.6 | 未测 |
| optimized-firefox-3840-default | 2 | 2,587.9 | 未测 |
| optimized-webkit-1920-default | 1 | 1,934.8 | 未测 |
| optimized-webkit-3840-default | 1 | 2,939.5 | 未测 |
| optimized-webkit-3840-default | 2 | 3,083.4 | 未测 |
| original-chromium-1920-default | 1 | 1,994.8 | 未测 |
| original-chromium-1920-default | 2 | 2,031.9 | 未测 |
| original-webkit-1920-default | 1 | 2,236.6 | 未测 |
| original-webkit-1920-default | 2 | 2,276.4 | 未测 |

## 运行时、分配与重建

本表为放映 Worker 的最大单实例数据；各渲染器末次观测分项另存 JSON，整个浏览器占用以 PSS 为准。Rust 活跃分配是分配器请求字节，不含静态区、栈及分配器元数据。WASM 堆容量另外记录；CanvasKit 未公开活跃分配器数据，因此只列其容量。逻辑 RGBA 缓存字节与驱动显存分开保存在 summary.json。

| 记录 | 单实例 Rust 活跃分配峰值 / MiB | 单实例 Rust 堆容量 / MiB | 单实例 CanvasKit 堆容量 / MiB | 动画期间层重建 | 动画期间效果重绘 |
|---|---|---|---|---|---|
| baseline-chromium-1920-default | 未测 | 9.0 | 458.9 | 6 | 未测 |
| baseline-chromium-1920-hardware | 未测 | 9.0 | 458.9 | 0 | 未测 |
| baseline-chromium-3840-default | 未测 | 4.5 | 660.9 | 0 | 未测 |
| baseline-chromium-3840-hardware | 未测 | 4.5 | 774.8 | 0 | 未测 |
| baseline-firefox-1920-default | 未测 | 9.0 | 265.5 | 0 | 未测 |
| baseline-firefox-3840-default | 未测 | 未测 | 未测 | 未测 | 未测 |
| baseline-webkit-1920-default | 未测 | 9.0 | 382.4 | 0 | 未测 |
| baseline-webkit-3840-default | 未测 | 2.2 | 221.2 | 0 | 未测 |
| optimized-chromium-1920-default | 6.0 | 9.0 | 128.0 | 0 | 0 |
| optimized-chromium-1920-hardware | 7.2 | 18.0 | 128.0 | 0 | 0 |
| optimized-chromium-3840-default | 4.5 | 9.0 | 128.0 | 0 | 0 |
| optimized-chromium-3840-hardware | 7.2 | 18.0 | 128.0 | 0 | 0 |
| optimized-firefox-1920-default | 4.5 | 9.0 | 153.6 | 0 | 0 |
| optimized-firefox-3840-default | 4.2 | 9.0 | 458.9 | 0 | 0 |
| optimized-webkit-1920-default | 未测 | 未测 | 未测 | 未测 | 未测 |
| optimized-webkit-3840-default | 4.8 | 9.0 | 382.4 | 0 | 0 |
| original-chromium-1920-default | 未测 | 9.0 | 458.9 | 6 | 未测 |
| original-chromium-3840-default | 未测 | 4.5 | 550.7 | 0 | 未测 |
| original-firefox-1920-default | 未测 | 9.0 | 265.5 | 0 | 未测 |
| original-firefox-3840-default | 未测 | 未测 | 未测 | 未测 | 未测 |
| original-webkit-1920-default | 未测 | 9.0 | 382.4 | 0 | 未测 |
| original-webkit-3840-default | 未测 | 2.2 | 221.2 | 0 | 未测 |

前台与后台使用独立 Worker 和渲染上下文。后台分块准备、等待 GPU fence 并接受取消；前台耗时反馈控制后台提交间隔。任务重叠说明软件队列不再相互等待，不代表已测得 GPU 硬件同时执行。

失败记录：

| 记录 | 观测到的失败 |
|---|---|
| baseline-chromium-1920-default | Presentation renderer timed out |
| baseline-chromium-3840-default | Error: page.waitForFunction: Error: Presentation renderer timed out |
| baseline-chromium-3840-hardware | Error: page.waitForFunction: Error: Presentation renderer timed out |
| baseline-firefox-1920-default | Error: page.waitForFunction: Presentation renderer timed out |
| baseline-firefox-3840-default | Error: page.waitForFunction: Presentation renderer timed out |
| baseline-webkit-3840-default | Error: page.waitForFunction: Error: Presentation renderer timed out |
| optimized-webkit-1920-default | Error: page.waitForFunction: Target page, context or browser has been closed |
| original-chromium-1920-default | Presentation renderer timed out; Presentation renderer timed out |
| original-chromium-3840-default | Error: page.waitForFunction: Error: Presentation renderer timed out |
| original-firefox-1920-default | Error: page.waitForFunction: Presentation renderer timed out |
| original-firefox-3840-default | Error: page.waitForFunction: Presentation renderer timed out |
| original-webkit-3840-default | Error: page.waitForFunction: Error: Presentation renderer timed out |

## 稳定缓存后的 60 Hz 帧预算

这些页面的动画属性为 transform / opacity。下面仅统计没有静态或对象缓存重建、也没有 CPU 像素表示转换的活动帧；16.67 ms 是单帧预算。提交耗时、异步 GPU 时间和逐页回复间隔分别保存。表中最慢页面的活动回复间隔可揭示驱动或调度造成的低帧率；仅提交耗时低于预算不代表达到 60 Hz。这些指标都不能直接替代物理显示器帧率。

| 记录 | 稳定帧数 | 光栅提交 P95 / ms | GPU P95 / ms | 最慢页面活动回复间隔 P95 / ms | 提交超过 16.67 ms 的帧数 | 活动期间跳过请求数 |
|---|---|---|---|---|---|---|
| optimized-chromium-1920-default | 1473 | 1.9 | 未测 | 68.2 | 0 | 0 |
| optimized-chromium-1920-hardware | 2100 | 2.2 | 0.8 | 18.2 | 0 | 0 |
| optimized-chromium-3840-default | 404 | 4.2 | 未测 | 365.8 | 0 | 0 |
| optimized-chromium-3840-hardware | 2100 | 3.4 | 2.8 | 17.9 | 0 | 0 |
| optimized-firefox-1920-default | 658 | 59.0 | 未测 | 286.0 | 658 | 0 |
| optimized-firefox-3840-default | 191 | 200.0 | 未测 | 1,074.0 | 191 | 0 |
| optimized-webkit-1920-default | 0 | 未测 | 未测 | 未测 | 0 | 0 |
| optimized-webkit-3840-default | 298 | 199.0 | 未测 | 217.0 | 298 | 0 |

## 第二轮实际重建次数

单独检查第二轮全部实际绘制请求，不预先剔除发生重建的帧。首次像素表示转换单列；缺少旧版计数时记为未测。

| 记录 | 实际绘制请求 | 对象缓存重建 | 静态缓存重建 | 直接效果绘制 | 效果内容重放 | CPU 像素表示转换 |
|---|---|---|---|---|---|---|
| optimized-chromium-1920-default | 857 | 0.0 | 0.0 | 0.0 | 0.0 | 未测 |
| optimized-chromium-1920-hardware | 1107 | 0.0 | 0.0 | 0.0 | 0.0 | 未测 |
| optimized-chromium-3840-default | 267 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| optimized-chromium-3840-hardware | 1107 | 0.0 | 0.0 | 0.0 | 0.0 | 未测 |
| optimized-firefox-1920-default | 411 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| optimized-firefox-3840-default | 178 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| optimized-webkit-1920-default | 0 | 未测 | 未测 | 未测 | 未测 | 未测 |
| optimized-webkit-3840-default | 232 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |

## 前台与后台任务重叠

| 记录 | 后台准备期间前台完成活动帧数 | 这些前台帧的光栅提交 P95 / ms |
|---|---|---|
| optimized-chromium-1920-default | 65 | 3.2 |
| optimized-chromium-1920-hardware | 69 | 3.1 |
| optimized-chromium-3840-default | 29 | 7.2 |
| optimized-chromium-3840-hardware | 69 | 4.8 |
| optimized-firefox-1920-default | 51 | 276.0 |
| optimized-firefox-3840-default | 54 | 181.0 |
| optimized-webkit-1920-default | 0 | 未测 |
| optimized-webkit-3840-default | 21 | 209.0 |

此处只统计视觉属性正在变化的动画区间，并检查不同 Worker 的请求时间区间重叠，验证前台无需等待整页后台准备；不把它称为 GPU 物理并行度测量。

## 冷切到阴影页

独立压力测试：每次新开浏览器与原文件，打开 P15，观察后台 P16 请求实际提交，150 ms 后使用正常的 16 + Enter 快捷键。该测试刻意提前切页；自然播放另行测量。失败按失败记录，不作为零延迟计入 P95。

| 记录 | 可见 / 尝试 | 状态 | 首次变化 P95 / ms |
|---|---|---|---|
| baseline-1920 | 20 / 20 | passed | 25,962.7 |
| baseline-3840 | 0 / 20 | failed | 未测 |
| optimized-1920 | 20 / 20 | passed | 148.1 |
| optimized-3840 | 20 / 20 | passed | 875.7 |
| original-1920 | 20 / 20 | passed | 26,095.2 |
| original-3840 | 0 / 20 | failed | 未测 |

1920 像素宽冷切页 P95 降幅：99.4%；50% 目标达到。

## WebKit 崩溃复核

首次 WebKit 1920 × 920 测试在第二轮第 22 页前发生 WebProcess 崩溃，完成 47 次访问。该失败保留在主矩阵，没有用复跑覆盖。首次失败未保存完整调用轨迹；已修正测试收集器，使页面崩溃后仍保留外部已收到的调用和导航记录。

| 复跑记录 | 完成访问 | 状态 | 已准备首次变化 P95 / ms | 活动缓存帧绘制 P95 / ms | PSS 采样峰值 / MiB |
|---|---|---|---|---|---|
| webkit-1920-1 | 52 | passed | 110.0 | 50.0 | 2,103.5 |

一次相同候选、尺寸和操作序列的完整复跑未再次崩溃。原因尚未确认，不能将未复现写成故障已消除。初次崩溃和复跑的原始结果，以及可用日志边界，见 webkit-crash-review.json 和验证压缩包。

## 目标检查

| 项目 | 本次结果 |
|---|---|
| 优化版完整遍历且无任务错误（不代表性能达标） | 7 / 8 通过 |
| 已准备页首次变化 P95 ≤ 200 ms | 3 / 8 个已记录组合满足 |
| 保持阶段不持续重绘 | 7 / 8 个已记录组合满足 |
| 菜单及退出 P95 ≤ 100 ms | 通过（每浏览器 5 次） |
| 其他页面不得出现明显回退 | 未通过：本机硬件 P01 首次进入观察到延迟增加，见下节 |
| 其他 GPU / 操作系统 | 未测 |

## 首次进入第 1 页的回退

本机硬件 Chromium、1920 × 920 的完整两轮对照中，P01 初次进入的首次变化由 104.3 ms 增至 414.4 ms。这里各版本只有一次冷进入观察，不称为多次冷启动 P95，也不与下方 20 次阴影页压力测试混用。该回退尚未修复，因此不能判定其他页面均无性能回退。

新路径在首屏可见前准备了 53 块内容，入口准备约 374 ms，其中 GPU 完成等待约 224 ms。等待包含驱动／浏览器确认延迟，不等于 GPU 实际执行时长。逐次访问、阶段数据和文件身份见 first-entry-regression.json。

## 尚未解决的 CPU 工作块延迟

Firefox CPU 的 P02 激活请求，与同一 Worker 中后台 P16 的同步阴影块重叠。该块耗时 1246 ms，目标请求排队 787 ms。取消在工作块边界生效；此例仍超过 200 ms 切页目标。独立前台能够继续绘制，不等于后台每个 CPU 工作块都已满足响应预算。原始时间区间与文件身份保存在 remaining-bottlenecks.json。

## CPU 缓存像素合成校验

三个浏览器使用相同缓存网格对比原 Skia 合成与新路径，覆盖真实 P12/P17 的 4K 中间态和终态、透明画布上的半透明叠加、旋转与嵌套裁剪回退，并断言每块只长期保留一种像素表示。

| 浏览器 | 状态 | 比较数 | 最大通道差 / 255 |
|---|---|---|---|
| chromium | passed | 24 | 1 |
| firefox | passed | 24 | 1 |
| webkit | passed | 24 | 1 |

不透明页面比较 RGBA，透明叠加比较预乘 RGBA。首次转换计入首帧时间和实际资源分配；稳定缓存帧预算单独排除这些一次性转换，并在逐页 CSV 中列出转换次数。

## 可直接配对的资源变化

仅比较同环境、同分辨率、同采样协议且完成 52 次访问的记录。负值为下降，正值为上升；内存与显存分别判断，不以其中一项下降代表两项都下降。

| 环境 | 之前 PSS / MiB | 之后 PSS / MiB | PSS 变化 | 之前 VRAM / MiB | 之后 VRAM / MiB | VRAM 变化 |
|---|---|---|---|---|---|---|
| optimized-chromium-1920-hardware | 1,624.2 | 1,028.2 | -36.7% | 454.1 | 375.1 | -17.4% |

| 记录 | 完整进程树采样数 | 进程数量范围 | 采样 P95 / ms | 包含已退出／不可读进程的样本数 |
|---|---|---|---|---|
| baseline-chromium-1920-default | 303 | [6, 6] | 42.0 | 0 |
| baseline-chromium-1920-hardware | 292 | [6, 6] | 30.0 | 0 |
| baseline-chromium-3840-default | 61 | [6, 6] | 59.0 | 0 |
| baseline-chromium-3840-hardware | 65 | [6, 6] | 47.0 | 0 |
| baseline-firefox-1920-default | 126 | [7, 7] | 33.0 | 0 |
| baseline-firefox-3840-default | 27 | [7, 7] | 29.0 | 0 |
| baseline-webkit-1920-default | 369 | [4, 4] | 35.0 | 0 |
| baseline-webkit-3840-default | 34 | [4, 4] | 39.0 | 0 |
| optimized-chromium-1920-default | 282 | [6, 6] | 36.0 | 0 |
| optimized-chromium-1920-hardware | 281 | [6, 6] | 26.0 | 0 |
| optimized-chromium-3840-default | 293 | [6, 6] | 51.0 | 0 |
| optimized-chromium-3840-hardware | 281 | [6, 6] | 29.0 | 0 |
| optimized-firefox-1920-default | 290 | [7, 7] | 31.0 | 0 |
| optimized-firefox-3840-default | 309 | [7, 7] | 38.0 | 0 |
| optimized-webkit-1920-default | 249 | [3, 4] | 37.0 | 1 |
| optimized-webkit-3840-default | 289 | [4, 4] | 50.0 | 0 |
| original-chromium-1920-default | 297 | [6, 6] | 42.0 | 0 |
| original-chromium-3840-default | 61 | [6, 6] | 62.0 | 0 |
| original-firefox-1920-default | 126 | [7, 7] | 32.0 | 0 |
| original-firefox-3840-default | 27 | [7, 7] | 34.0 | 0 |
| original-webkit-1920-default | 369 | [4, 4] | 31.0 | 0 |
| original-webkit-3840-default | 34 | [4, 4] | 31.0 | 0 |

## CPU 缓存表示与工作缓冲

CPU 像素数组替换对应的原生图像，是逻辑缓存字节的子集；不把它漏算，也不与逻辑缓存再次相加。转换时临时双持按资源分配计数，表中工作字节是按像素缓冲请求大小记录的上界；整体驻留仍以浏览器 PSS 为准，不能用 CanvasKit 堆容量下降代替实际内存下降。

| 记录 | 单渲染器 CPU 缓存像素峰值 / MiB | 转换工作缓冲上界 / MiB |
|---|---|---|
| optimized-chromium-1920-default | 0.0 | 0.0 |
| optimized-chromium-1920-hardware | 0.0 | 0.0 |
| optimized-chromium-3840-default | 0.0 | 0.0 |
| optimized-chromium-3840-hardware | 0.0 | 0.0 |
| optimized-firefox-1920-default | 24.6 | 1.5 |
| optimized-firefox-3840-default | 77.7 | 5.9 |
| optimized-webkit-1920-default | 未测 | 未测 |
| optimized-webkit-3840-default | 109.3 | 8.1 |

## 连续退出与重开

WebKit 连续 50 次退出／重开：passed。首次退出后 PSS 1655.4 MiB；最后十次 1623.3–1720.0 MiB。后续准备可能保留新增有效内容，首末差值不能独立证明泄漏或无泄漏；资源与缓存复用分项在 functional-summary.json。

## 操作、离线编辑与画质

| 浏览器 | 版本 | 菜单 P95 / ms | 退出 P95 / ms | 样本数 | 离线编辑、保存、重新打开 |
|---|---|---|---|---|---|
| chromium | 151.0.7922.34 | 21 | 10 | 5 | 通过 |
| firefox | 153.0 | 41 | 15 | 5 | 通过 |
| webkit | 26.5 | 24 | 23 | 5 | 通过 |

CPU 回退共检查 26 页、2348 个对象区域；最差对象平均通道差 0.4971 / 255。真实 VS Code 回归：passed。原生渲染、增量更新、透明叠加、中间态、终态、转场及缓存生命周期的记录随验证文件提供。

## 解释边界

- 帧回复间隔不等于物理显示器扫描输出时间；GPU timer、fence 等待和主线程请求排队分别记录。CanvasKit 内部上传尚未独立暴露，不能把缺失值写成零。

- 硬件测试以设备字符串为准；本机 AMD 的结果不能代替其他显卡或驱动。所有未测场景不标记通过。

- 完整逐页数据和准备阶段记录见 summary.json 与 per-page.csv；原始测试记录保留失败、异常及后端信息。

- 不发布版本。旧离线 HTML 不会自动获得运行时修复。

优化包 SHA-256：`d32e652bd3a154bf54f5d7b2f424fcf8126cc1698e5d6f6d1e3aff87b9bc593c`。
