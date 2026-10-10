# 动画与 Animate.css

## step

动画演示附着在静态画布上。`page.step(name="标题", hold=1s)` 按创建顺序添加步骤；同一步内的动作默认并行，`delay` 用于错开。`hold` 默认为 `1s`。自动窗口取所有有限动画的最晚结束时间，计入负延迟和小数重复次数。无限循环或暂停的效果必须给 step 指定有限的 `duration`。

预览默认显示完整静态排版，进入播放后才启用演示状态。控制栏提供播放/暂停、上一步/下一步、跳过当前动画、重播、时间定位、倍速，以及手动/自动推进。手动模式在动画窗口末尾暂停；自动模式经过 hold 后推进，支持文档跨页。回退会按目标时间重新采样。第一个使用 backwards/both 的效果可以预置入场前状态，后续尚未激活的效果不会抢占同一属性。停留期间使用步骤窗口的边界状态。这些是 **LAY 演示调度规则**。

## animate

原样导入第三方文件，不需要改名或预处理：

```css
/* motion.lcss */
@import "./vendor/animate.css";
.title-motion { --animate-duration: 800ms; }
```

```python
page = canvas(size=(160mm, 90mm), stylesheet="motion.lcss")
title = page.add(text("研究结果", class="title-motion"), offset=(10mm, 10mm))
s1 = page.step(name="标题入场", hold=1s)
s1.animate(title, class="animate__animated animate__fadeInUp")
s2 = page.step(name="强调标题", hold=2s)
s2.animate(title, class="animate__animated animate__pulse", duration=600ms)
```

目标必须是 `add()` 返回的已放置实例。动作的临时 class 提供动画声明、变量、变换原点和背面可见性等，不修改静态排版。也可用 `name` 直接引用关键帧。单次覆盖支持 `duration/delay/easing/iterations/direction/fill/vars/play_state`；未提供的参数取 CSS 计算结果，显式 LAY 参数优先于 CSS，包括 `!important`。例如 `vars={"--animate-duration": 800ms}`、`iterations=2.5` 或 `iterations="infinite"`。

完整示例：[presentation.lay](../../examples/animation/presentation.lay)、[motion.lcss](../../examples/animation/motion.lcss)、[双页 presentation.laydoc](../../examples/animation/presentation.laydoc)。

## Compatibility

兼容目标限定为 **Animate.css v4.1.1 的 97 个效果**，不表示兼容任意网页样式表。[原始文件及 SHA-256](../../tests/animation/vendor/README.md) 固定具体版本，保留原始 MIT 许可头。`transition-*` 可解析，但不会自动为排版编辑生成过渡动画。

| 项目 | 支持范围 |
| --- | --- |
| 时间 | `@keyframes`、`animation` 和八个长属性；标准默认值 0s/ease/none；负延迟、零/小数/无限次数、四种方向和填充模式、running/paused |
| 关键帧 | 省略端点使用基础属性，同一位置重复帧按属性合并，分段缓动；动画列表后面的效果覆盖前面相同属性；关键帧内 !important 忽略 |
| 缓动 | linear、ease 系列、cubic-bezier()、steps() 及 jump 位置；CSS steps() 是阶梯缓动，与 LAY step 无关 |
| 样式 | 本地 .css/.lcss 导入、:root、类型/类选择器、后代/子代及 class 属性选择器、继承 var()、类型化 calc()、!important、标准与 -webkit- 对应声明去重、@charset |
| 媒体 | 固定库使用的 screen/all/print 与 prefers-reduced-motion 条件；预览遵循系统减少动态效果偏好，视频默认 screen/no-preference |
| 属性 | opacity 替换目标自身透明度，父级透明度另行相乘；标准变换函数、transform-origin、visibility、backface-visibility |
| 单位 | 动画 CSS 固定 96px=1in；百分比平移和原点基于未变换的布局框；LAY 原有几何单位规则不变 |
| 三维 | headShake、flip、flipInX、flipInY、flipOutX、flipOutY；对象局部 RGBA 合成、透视裁剪、背面隐藏；不实现网页三维场景排版 |

Rust 与 WASM 共用时间、插值和渲染实现。内容变化时编译布局和字体，播放逐帧更新动画属性并复用缓存。二维效果沿用矢量绘制列表，透视对象只合成局部图层。过大的投影图层会给出像素上限诊断。

## Extensions

`-laymesh-draw` 和 `-laymesh-reveal` 是取值 0–1 的非标准扩展。内置 `laymesh-draw`/`draw`、`laymesh-reveal`/`reveal` 关键帧，可直接引用或放进自定义预设类。需提供时长和填充方式，例如 `s.animate(line_instance, name="laymesh-draw", duration=1s, fill="both")`。描线用于有描边的路径；图表揭示缩小数据区裁剪框，保留坐标轴和图例。普通对象按布局框揭示。

## Export

FFmpeg 需已安装并位于 PATH：

```sh
laymesh animate examples/animation/presentation.laydoc -o presentation.mp4
laymesh animate examples/animation/presentation.lay -o presentation.gif
laymesh animate examples/animation/presentation.lay -o reduced.mp4 --reduced-motion reduce
laymesh render examples/animation/presentation.lay -o frame.png --time 2.4 --width 1920
laymesh render examples/animation/presentation.lay -o step.png --step 2
```

MP4 默认宽 1920、30fps，GIF 默认宽 960、20fps。保持比例，MP4 尺寸向偶数对齐；多页按首个选中页面比例居中留边。每页只编译一次，逐帧送入 FFmpeg，不在内存堆积全部帧。失败或取消会清理临时文件，保留原有目标文件。CLI 用 Ctrl+C 取消，VS Code 的进度通知提供取消操作。第一版不含音频。

VS Code 仍然只选择格式和路径；选 MP4 动画或 GIF 动画，普通 GIF 保留静态图片含义。编码参数在 `laymesh.export.animation.*` 设置中调整。CLI 可覆盖 `--width/--fps/--crf/--preset/--background/--reduced-motion/--media/--page`。项目 `.laymesh.json` 可以分别配置两种格式：

```json
{"export":{"animation":{"crf":18,"reduced_motion":false},"animation_formats":{"mp4":{"width":1920,"fps":30},"gif":{"width":960,"fps":20}}}}
```

`inspect --json` 列出步骤名、开始时间、窗口、停留时长及动画目标，同时提供减少动态效果时的时长。普通 SVG/PDF/PNG 导出使用完整静态排版，不应用 print 动画规则。指定时间/步骤的快照仅支持 PNG；不包含 PPTX 原生动画、形状变形或数据插值。

## Verification

`cargo test -p laymesh-core --test animation` 检查固定库与时间语义；`cargo test -p laymesh-render --test animation` 检查全部效果渲染、回退随机定位和静态恢复。编译 `animation_reference` 示例及 web WASM 后，用已有 Playwright 运行 `node scripts/test-animation-browser.cjs /tmp/animation-qa`，对照原生/WASM/CSS 的采样属性，并查看并排代表帧。`PLAYWRIGHT_MODULE`、`CHROMIUM_PATH`、`LAYMESH_TEST_HOST` 指定现有测试工具与服务绑定地址。

浏览器脚本验证 97 个效果的 873 个时间采样点，并自动比较六个非平面效果的代表帧，容许抗锯齿导致的平均 RGB 通道误差不超过 3/255。`python scripts/test-animation-export.py --binary target/release/laymesh` 检查全部 97 个效果的 MP4/GIF 帧数、尺寸、时长，以及取消和失败清理；需要 FFmpeg 和 ffprobe。`python scripts/test-vscode-preview.py --output /tmp/vscode-animation-qa` 覆盖实际扩展的播放、定位、静态恢复和动画导出。

规范：[CSS Animations](https://www.w3.org/TR/css-animations-1/)、[CSS Transforms](https://www.w3.org/TR/css-transforms-1/)、[CSS Easing](https://www.w3.org/TR/css-easing-1/)。
