# 动画与 Animate.css

## step

动画演示附着在静态画布上。`page.step(name="标题", hold=1s)` 按创建顺序添加步骤；同一步内的动作默认并行，`delay` 用于错开。`hold` 默认为 `1s`。自动窗口取所有有限动画的最晚结束时间，计入负延迟和小数重复次数。无限循环或暂停的效果必须给 step 指定有限的 `duration`。

预览默认显示完整静态排版，进入播放后才启用演示状态。控制栏提供播放/暂停、上一步/下一步、跳过当前动画、重播、时间定位、倍速，以及手动/自动推进。手动模式在动画窗口末尾暂停；自动模式经过 hold 后推进，支持文档跨页。回退会按目标时间重新采样。第一个使用 backwards/both 的效果可以预置入场前状态，后续尚未激活的效果不会抢占同一属性。停留期间使用步骤窗口的边界状态。这些是 **LAY 演示调度规则**。

## 正式放映

网页示例点击 **放映** 进入浏览器全屏；VS Code 的 **放映** 菜单提供内嵌放映和在浏览器全屏放映。浏览器入口复用原生已编译产物，通过绑定 Tailscale 地址的临时会话服务访问，关闭浏览器页面或预览会话后释放服务。仅需本机预览时可选择内嵌放映。

默认打开完整静态排版。首帧显示后在独立编译队列中准备整份文档，显示真实完成页数；首页及其显示资源就绪后即可放映，后续页面继续后台准备。跳到未就绪页时优先准备目标页，保留当前画面并暂停时钟；目标真实首帧完成后才切换。后台失败只影响对应页，跳到该页时可重试。准备不播放动画，也不推进时钟。失败显示页码和位置，可重试或取消。正式放映固定当前文档版本，编辑更新在退出后应用。

放映默认手动推进：第一次点击执行第一步，动画中再次推进会完成当前步并停留，再次推进执行下一步；全部步骤完成后才翻页。回退按步骤边界重采样，页首回退显示上一页完成态。无动画页面直接按页推进，自动模式默认停留 1 秒。自动播放、倍速、时间定位、页面列表和快捷键帮助在浮动工具条的菜单内。控制栏默认隐藏，按 Tab 呼出并切换按钮，点击画面收起。右键 →“控制栏显示”可选择“一直显示”“鼠标到位置显示”或“不显示”；鼠标模式只在底部控制栏位置触发，不会因画面中的鼠标移动或翻页弹出。三种模式下均可使用 Tab，选择在本次预览会话内保留。右键菜单暂停计时，关闭后恢复先前状态。页面进入后台后暂停，返回时需手动继续。

| 输入 | 行为 |
| --- | --- |
| 点击、空格、Enter、右/下方向键、PageDown、N | 推进 / 完成正在播放的步骤 |
| 左/上方向键、PageUp、Backspace、P | 回退至步骤边界；页首回到上一页 |
| 滚轮向下 / 向上 | 推进 / 回退；连续滚动与触控板惯性合并为一次操作 |
| 数字后 Enter | 跳到对应页 |
| Home / End | 首页初态 / 末页完成态 |
| S | 暂停或继续 |
| B / W | 黑屏 / 白屏，再按恢复 |
| 右键 | 打开放映菜单 |
| Esc | 关闭菜单；菜单关闭时退出放映 |

放映键盘操作只在放映区域内接管，忽略按键自动重复。退出恢复编辑预览的页面、缩放、滚动和焦点。工具条浮在画面内，不为控件预留黑边；画面按原始比例适应屏幕，不裁切、不拉伸。VS Code 内嵌放映占满预览区域，浏览器入口才使用浏览器全屏。

滚轮只在放映画布内导航。页面列表等菜单保留正常滚动，横向滚动和带 Ctrl/Command/Shift/Alt 的操作不会推进。相邻滚轮事件间隔超过 180 ms 后才视为新手势，避免一次触控板滑动跳过多步；编辑预览原有的滚轮缩放行为保持不变。

编译产物与资源按内容去重，超出内存预算的产物写入浏览器会话 IndexedDB 或原生临时目录。GPU 优先保留当前页和相邻页；静态区域和对象通过资源构建准备，不通过试播建立缓存。切到非驻留页会保留上一帧，目标首帧完成后才切换并启动时钟。暂停与菜单保留绘制缓存；退出放映释放绘制资源。

编辑预览保持固定单行工具栏和状态栏。错误只更新同一个数量入口；源码编译失败保留上次成功画面。诊断与性能信息按需展开为可滚动浮层，支持复制和跳转源码，不改变画布尺寸、不自动抢焦点。

网页首页先行，其后使用最多 4 个独立编译 Worker；低内存设备最多 2 个，每个 Worker 只允许一个待保存页面。文档只解析一次，共享资源按 ID 去重。VS Code 使用最多 4 个低优先级原生编译进程，按可用核心与内存限制并发；当前页预览繁忙时不派发后台编译，进行中的后台编译会中断并在预览完成后重试，已完成的页面保留；编辑失效的准备队列取消，待新预览首帧完成后再启动。编译完成后退出后台进程，已编译产物保留至会话结束。显示资源准备会分析变换、直接构建对象纹理并等待 GPU 完成，不播放动画。投影密度在真实播放时仍逐帧核对；超出预估或缓存预算时保留完整绘制质量，不能承诺任意自定义动画都没有首次构建。测试结果分别记录编译准备、资源准备和首次/重复播放中的纹理重建。

### 退出与移动终态

退出动画支持真正保持消失。例如 `s.animate(item, class="animate__animated animate__fadeOut", fill="both")` 在结束后保持透明；`fill="none"` 则恢复原值。Animate.css 的 `animate__animated` 已定义 `both`，通常不需要重复指定。若后续动作再次设置同一属性，会按动画覆盖规则更新它。

移动的最终位置由最后一个关键帧决定。自定义 `@keyframes move { from { transform: translateX(0px); } to { transform: translateX(320px); } }` 配合 `s.animate(item, name="move", duration=1s, fill="forwards")` 会在演示中停留于右移 84.67 mm 的位置；`pulse` 等首尾相同的效果自然回到原处。变换不会修改静态排版或其他对象的锚点，退出放映后恢复完整原始布局。回退和定位重新计算对应时间状态。

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

## “点击”“跟随前一动画”与自动放映

`step` 默认是点击边界；`start` 是 LAY 调度参数，不是 CSS 动画属性。

| 意图 | 定义方式 |
| --- | --- |
| 单击时开始 | `page.step(start="click")`，这是默认值 |
| 与步骤一起开始 | `s.animate(..., start="step")`，这是默认值 |
| 与前一动作同时开始 | `s.animate(..., start="with-previous")` |
| 前一动作结束后开始 | `s.animate(..., start="after-previous")` |
| 前一步停留结束后自动开始 | `page.step(start="after-previous")`；用于第一步时随页面进入开始 |

```python
first = page.step(name="点击：标题与图表", hold=500ms)
first.animate(title, class="animate__animated animate__fadeInUp", duration=600ms)
first.animate(chart, name="laymesh-reveal", duration=800ms, fill="both",
              start="with-previous")
# 自动衔接图表的结束时间，再延迟 200ms；修改图表时长后不用重算 delay。
first.animate(note, class="animate__animated animate__fadeIn", duration=400ms,
              start="after-previous", delay=200ms)

second = page.step(name="自动强调", start="after-previous", hold=1s)
second.animate(title, class="animate__animated animate__pulse", duration=800ms)
third = page.step(name="等待下一次点击")
```

`after-previous` 引用前一次 `animate()` 调用。如果该调用包含 CSS 动画列表，以其中最晚结束时间为准；计算包含延迟、重复次数、媒体条件和单次覆盖。`delay` 相对所选起点计算，可以为负数。`with-previous` 以该调用最早实际开始时间为准；第一项没有前项时以步骤起点为准。空动画不增加窗口。无限循环或永久暂停的动作没有结束时间，不能被同一步的 `after-previous` 引用；可把它放进有限 `duration` 的独立 step，再接自动 step。

手动模式下，普通 step 完成后等待点击；紧接 `start="after-previous"` 的 step 则在前一步的 `hold` 结束后自动开始。点击跳过进行中的步骤仍会停留，回退和定位也保持暂停。所有步骤设为 `after-previous` 可让页面进入后自动执行，末页动作完成后保留画面；跨页自动推进由放映菜单的 **自动播放** 控制。全局自动播放与 MP4/GIF 导出使用全部窗口及 `hold` 的连续时间线，不等待点击。

可运行示例：[自动衔接与保持终态](../../tests/presentation/linked.lay)。

高信息密度测试：[32 页学术演示源码](../../tests/presentation/academic/report.laydoc)、[复现说明与数据边界](../../tests/presentation/academic/README.md)。24 页正文包含解析曲线、有限差分结果、热图、网格与公式，8 页附录将全部 97 种 Animate.css 效果应用到混合学术对象；数据是可复算教学数据，不是实验结果。

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
| 三维 | headShake、flip、flipInX、flipInY、flipOutX、flipOutY；Skia 局部图层与 GPU 投影、透视裁剪、背面隐藏；不实现网页三维场景排版 |

Rust 与 WASM 共用时间模型、插值和属性采样，只有内容变化时重新排版。CanvasKit 保留矢量绘制记录与图片；原生导出使用 Skia CPU、SkPDF。显示缓冲区始终按视口和设备像素比绘制，播放不降低画质。“详情”显示画布尺寸、后端、采样时间、CPU 绘制提交耗时、可用时的 GPU 查询耗时、提交 fps 与呈现估计。浏览器刷新回调不能证明显示器实际扫描输出；SwiftShader、llvmpipe 等软件 GPU 驱动不计入 4K60 硬件验收。

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
