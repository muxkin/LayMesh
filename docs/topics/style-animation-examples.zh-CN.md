# 样式与动画综合案例

从[样式与动画库](style-libraries.zh-CN.md)开始：先了解内置库、来源和导入方法，再逐项尝试、调参和组合；单文件 `.lay` 就能起步。

[样式手册](lcss.zh-CN.md) · [动画手册](animation.zh-CN.md)

这三组完整项目从样式库组合走到艺术字与复杂场景。每组都提供可编辑 `.lay`、主题、资源和真实 WASM 预览；静态图与动画从同一份源码生成。先看完整排版，再点“放映”：这些案例的第一步设置了 `after-previous`，进入后会自动开始。

| 案例 | 样式内容 | 动画内容 |
| --- | --- | --- |
| [三库卡片](#styled-cards) | Bootstrap 配色、2mm 模糊覆盖、Open Props 渐变、圆角与阴影、内阴影 | 标题入场、错开卡片入场、描线、自定义透视强调 |
| [艺术字海报](#art-direction) | 字形渐变、描边、矢量挤出、曲线路径、波浪变形、阴影、发光、透明图片 | 子对象独立动作、重复与交替、整个组的三维投影 |
| [复杂仪表板](#complex-dashboard) | 256 组矢量图案、12 张图表、24 个公式、透明面板和效果 | 并行循环、负延迟、透视翻转、12 条描线、有限窗口 |

## styled-cards

### 三个离线库和一个项目主题

布局文件只保存尺寸、放置、组件内容与步骤；`styled-theme.lcss` 同时管理外观和关键帧。主题显式导入 Bootstrap、Open Props 和 Animate.css。`metric()` 复用卡片结构，循环遍历三条内容记录，以不同 class 获得三种外观。第一张相对说明文字放置，后续卡片连接前一张的 `top_right`；组内文字也沿 `bottom_left` 接续。

第一张卡片组合 `bg-primary rounded shadow custom`，只把 Bootstrap 阴影模糊改为 2mm；第二张把 Open Props 变量用于渐变、圆角和多层阴影；第三张使用内阴影。`.metric .label` 等所属选择器统一组内文字，显式参数优先的规则仍然适用。

<!-- example:examples/animation/styled-cards.lay -->

`instances = append(instances,item)` 保存实际放置的实例；`append()` 返回新列表，需要重新赋值。第二个循环把这些实例交给 `animate()`，用 `delay=i*150ms` 实现错开入场。修改一个卡片尺寸或文字字号，邻接卡片及后续文字按锚点重新定位，无需逐个修改页面坐标。

| 全局时间 | 动作 |
| --- | --- |
| 0–0.6s | 标题与说明同时入场，说明延迟 0.2s |
| 0.6–0.8s | 停留 |
| 0.8–1.8s | 三张卡片相隔 0.15s 开始，分隔线同时描线 |
| 1.8–2.2s | 停留 |
| 2.2–3.1s | 第二张卡片执行主题中的自定义透视动作 |
| 3.1–3.5s | 停留，随后保留终态 |

在完整源码中编辑主题的 `box-shadow-blur:2mm`，静态阴影和动画中的阴影一起改变；改成 `box-shadow-spread:2mm` 则改变扩散，不是模糊。编辑 `focus-card` 改变运动。动画调用里的显式 `duration` 会覆盖 CSS 时长，见[步骤参数](animation-timing.zh-CN.md)。

[布局源码](../../examples/animation/styled-cards.lay) · [项目主题](../../examples/animation/styled-theme.lcss)

## art-direction

### 把已有艺术字与效果案例组合成动画

此项目沿用已有的[艺术字案例](../../examples/effects/art-text.lay)和[透明图片与组效果](../../examples/effects/images-groups.lay)，把重复外观移入 `art-direction.lcss`。图片来自仓库内的 `alpha-ring.png`，无需下载。

<!-- example:examples/animation/art-direction.lay -->

主题里的 `.hero` 组合字形渐变、白色描边、二维矢量挤出和原有 `effects` 阴影；`.curved` 使用 CSS 文字阴影，`.wave` 使用波浪变形与发光，`.stamp` 使用内容 Alpha 的 drop-shadow。底板的盒阴影是另一套轮廓。它们各自保留[阴影与效果](style-shadows.zh-CN.md)中说明的模糊语义。

海报底板相对页标题放置，页脚跟随海报的 `bottom_left`，印章以 `bottom_right` 贴住底板内侧。曲线路径仍由 `.lay` 定义；路径和沿路径排字的文字共用局部原点，并把它们的组对齐底板。路径控制点是几何数据，不是各对象的页面位置；描线动作逐步显示路径。三个步骤分别是：

1. 标题入场与路径描线并行；曲线文字在描线后延迟 0.2s 出现。
2. 波浪文字上下移动，两轮交替；透明图片同时执行 `flipInY`。
3. 放置后的整个海报组执行透视旋转，所有子对象及效果随组一起变化。

总时间为 7.6s，含每步停留。`float-wave` 的两轮动作占 2.4s，`inspect-poster` 占 2s。变形文字的轮廓是静态外观，`transform` 是播放时的整体变换；动画不重新排字。最后关键帧回到零旋转，退出放映或选择“完整排版”恢复静态设计。

[布局源码](../../examples/animation/art-direction.lay) · [项目主题](../../examples/animation/art-direction.lcss) · [透明图片](../../examples/effects/alpha-ring.png)

## complex-dashboard

### 用循环组织复杂动画场景

这个项目借用已有[压力场景配方](../../tests/skia/stress-scenes.mjs)中 `09-motion-3d` 的内容规模，改写成可维护的 Lay 程序：组件函数定义结构，循环创建对象，列表保存实例，锚点连接相邻对象。数值采样保存在独立 `dashboard-data.json` 中；它们是确定性合成信号，不代表实验结果。

<!-- example:examples/animation/complex-dashboard.lay -->

| 内容 | 组合方式 |
| --- | --- |
| 256 组矢量图案 | 渐变底板、椭圆和交叉线；每四组之一旋转 |
| 12 张图表 | 8 张三曲线＋散点图、4 张热图；卡片组带透明度、阴影与光晕 |
| 24 个公式 | 公式位于嵌套卡片中，带阴影并随卡片一起投影 |
| 12 条开放线 | 描线进度交替循环 |
| 标题、图表与公式 | 多个动作并行，使用不同负延迟错开循环相位 |

这些数量是输入内容数量，不是渲染后路径数或帧率指标。整个步骤显式限定 `duration=12s`，所以无限循环也能放映、定位和导出；窗口结束不修改 CSS 的重复次数。此页只有一个步骤，回退回到其起点；多步骤练习使用前两个案例。

源码按三层阅读：

1. `motif()`、`chart()`、`equation_card()` 分别定义图案、图表卡片和公式卡片；主题保存重复外观与关键帧。
2. `in_grid()` 只为第一项设置起点；同一行连接前一项的 `top_right`，换行连接上一行同列的 `bottom_left`。公式区接在最后一行图表下方，描线区接在最后一行公式下方。行列间距由 `gap` 控制。
3. 创建对象的循环收集实例；后面的动画循环遍历列表，用索引计算负延迟和循环相位。每第四组图案执行旋转，用 `range(0,len(motifs),4)` 表达。

`cycle_index()` 用循环实现调色板和公式的循环索引；当前 Lay 没有 `%` 运算符。图表的曲线数组与热图矩阵从 JSON 读取，编辑器中的数据标签可以直接修改。Node 维护脚本只重新计算合成采样数据，不生成或覆盖 `.lay`、`.lcss`。已有数据随项目提供，编译和播放不需要运行该脚本。

~~~sh
laymesh validate examples/animation/complex-dashboard.lay
# 只有更新合成数据配方时才需要：
node scripts/build-dashboard-data.mjs
~~~

这是进阶与压力案例，设备负载明显高于前两组。先用“适应”检查整体，再放大工作区看图表和公式；实际设备性能需测量，见[播放实现与验证](animation-internals.zh-CN.md)。

[完整布局源码](../../examples/animation/complex-dashboard.lay) · [项目主题](../../examples/animation/dashboard-theme.lcss) · [采样数据](../../examples/animation/dashboard-data.json) · [数据维护脚本](../../scripts/build-dashboard-data.mjs)

## 保存、导出与练习

保存项目时同时保留主题和相对资源。三套离线库仍由引擎提供，文件无需复制上游 CSS。静态导出显示完整设计；MP4/GIF 使用同一份源代码的连续时间线。

~~~sh
laymesh validate examples/animation/styled-cards.lay
laymesh render examples/animation/styled-cards.lay -o cards.pdf --dpi 150
laymesh animate examples/animation/styled-cards.lay -o cards.mp4 --width 960 --fps 20
laymesh render examples/animation/art-direction.lay -o poster.png --time 4 --width 960
laymesh animate examples/animation/complex-dashboard.lay -o dashboard.mp4 --width 960 --fps 10
~~~

练习时一次只改一个因素：先把模糊从 2mm 改成 1mm，再改卡片延迟，最后改变关键帧角度。对照静态图、时间快照与实际播放；CSS 导入警告、缺失动画、效果溢出分别按[兼容与排错](style-compatibility.zh-CN.md)和[动画排错](animation-export.zh-CN.md)处理。
