# 播放实现与验证

本页面向维护者；正常播放操作见[放映手册](animation-presentation.zh-CN.md)。

编译产物与资源按内容去重，并随有效文档和页面保留。缓存不按固定容量或页面数量淘汰；被替换且无引用的版本、删除页面和关闭文档释放相应资源。静态区域和对象通过资源构建准备，不通过试播建立缓存。请求尚未就绪的页面时保留当前画面并提高目标页优先级，目标首帧完成后切换并启动时钟。暂停与菜单保留绘制缓存；退出放映释放绘制资源。

离线编辑器使用常驻编译 Worker 池，当前页先提交，随后按任务与浏览器可用并行度准备其他页；页面保持 Worker 归属以复用语义会话、字体和绘图基线。独立网站编辑器同样优先当前页，并行准备后续页面。VS Code 按机器可用并行度派发原生编译，保留有效页面成果；编辑使过期任务失效，当前预览优先。绘制当前页的 Worker 与准备其他页的 Worker 分开，后台准备可与动画推进。起始页对应最新源码后即可放映；保存和导出要求全稿成功。对象纹理按实际投影密度准备，播放时继续检查；自定义动画仍可能需要首次构建，保持实际显示精度。测试分别记录编译、资源准备及首次和重复播放中的纹理重建。

| 项目 | 支持范围 |
| --- | --- |
| 时间 | `@keyframes`、`animation` 和八个长属性；标准默认值 0s/ease/none；负延迟、零/小数/无限次数、四种方向和填充模式、running/paused |
| 关键帧 | 省略端点使用基础属性，同一位置重复帧按属性合并，分段缓动；动画列表后面的效果覆盖前面相同属性；关键帧内 !important 忽略 |
| 缓动 | linear、ease 系列、cubic-bezier()、steps() 及 jump 位置；CSS steps() 是阶梯缓动，与 LAY step 无关 |
| 样式 | 本地 .css/.lcss 导入、:root、类型/类选择器、后代/子代及 class 属性选择器、继承 var()、类型化 calc()、!important、标准与 -webkit- 对应声明去重、@charset |
| 媒体 | 固定库使用的 screen/all/print 与 prefers-reduced-motion 条件；预览遵循系统减少动态效果偏好，视频默认 screen/no-preference |
| 属性 | opacity 替换目标自身透明度，父级透明度另行相乘；标准变换函数、transform-origin、visibility、backface-visibility |
| 单位 | 静态与动画 CSS 共用 canvas.layout_dpi：layout_dpi 个 px=1in，默认 96；导出 DPI 不改变物理长度。百分比平移和原点基于未变换的布局框 |
| 三维 | headShake、flip、flipInX、flipInY、flipOutX、flipOutY；Skia 局部图层与 GPU 投影、透视裁剪、背面隐藏；不实现网页三维场景排版 |

Rust 与 WASM 共用时间模型、插值和属性采样，只有内容变化时重新排版。CanvasKit 保留矢量绘制记录与图片；原生导出使用 Skia CPU、SkPDF。显示缓冲区始终按视口和设备像素比绘制，播放不降低画质。“详情”显示画布尺寸、后端、采样时间、CPU 绘制提交耗时、可用时的 GPU 查询耗时、提交 fps 与呈现估计。浏览器刷新回调不能证明显示器实际扫描输出；SwiftShader、llvmpipe 等软件 GPU 驱动不计入 4K60 硬件验收。

## Verification

`cargo test -p laymesh-core --test animation` 检查固定库与时间语义；`cargo test -p laymesh-render --test animation` 检查全部效果渲染、回退随机定位和静态恢复。编译 `animation_reference` 示例及 web WASM 后，用已有 Playwright 运行 `node scripts/test-animation-browser.cjs /tmp/animation-qa`，对照原生/WASM/CSS 的采样属性，并查看并排代表帧。`PLAYWRIGHT_MODULE`、`CHROMIUM_PATH`、`LAYMESH_TEST_HOST` 指定现有测试工具与服务绑定地址。

浏览器脚本验证 97 个效果的 873 个时间采样点，并自动比较六个非平面效果的代表帧，容许抗锯齿导致的平均 RGB 通道误差不超过 3/255。`python scripts/test-animation-export.py --binary target/release/laymesh` 检查全部 97 个效果的 MP4/GIF 帧数、尺寸、时长，以及取消和失败清理；需要 FFmpeg 和 ffprobe。`python scripts/test-vscode-preview.py --output /tmp/vscode-animation-qa` 覆盖实际扩展的播放、定位、静态恢复和动画导出。
