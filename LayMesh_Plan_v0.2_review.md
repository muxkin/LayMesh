# LayMesh v0.2 方案评审与修订建议

**评审日期：** 2026-09-24

**评审对象：** `LayMesh_Plan_v0.2.md`（用户提供的附件）
**结论：** 适合作为产品方向和技术预研提纲；在进入实现前，建议先修订单位、图像变换与标尺的契约，再压缩首版范围。当前仓库只有 `LICENSE`，以下是方案评审，不是原型或性能验收。

## 1. 判断依据

方案最有价值的组合是：**代码式单页布局、对象间锚点依赖、科研图像标尺校准、可复现的多格式导出**。单独的物理页面、图像 `contain/cover` 和定位已有成熟实现；例如 Typst 提供页面定位和图像适配，ImageJ 提供图像空间标定和标尺。因此应通过真实工作流证明这一组合减少了人工改图、标尺错误和重复排版，而不应仅以“支持九点锚点”作为差异点。[Typst Place](https://typst.app/docs/reference/layout/place/) · [Typst Image](https://typst.app/docs/reference/visualize/image/) · [ImageJ 图像标注](https://imagej.net/imaging/annotating-images)

报告中的分层结构、TypeScript 核心与 Python 薄桥接、Scene 统一几何、PDF 后端先验证再选择、对未校准标尺发出诊断等方向合理。主要问题是部分语义尚不够精确，而首版同时承诺 DSL、编辑器、TIFF、复杂文字、三种导出和 Notebook，研发风险偏集中。

## 2. 优先修订项

| 优先级 | v0.2 中的问题 | 建议写入 v0.3 的确定规则 |
|---|---|---|
| P0 | 第 4.1 节把布局 `px` 交给画板 `dpi` 换算，易使导出 DPI 改变布局。SVG/CSS 中 `1in = 96px`，与 PNG 导出 DPI 不是同一概念。 | v0.1 布局只接受 `mm/cm/in/pt`；沿用方案中的 `image_px` 专指源图像像素；输出像素仅由页面物理长度和导出 DPI 计算。若以后开放布局 `px`，明确采用 CSS 的 96 px/in，绝不复用导出 DPI。见 [CSS Values 4](https://www.w3.org/TR/css-values-4/)。 |
| P0 | `fit=contain`、裁剪、旋转后，图片框锚点、可见图像边界及源像素到页面的映射未完全定义，标尺可能标错。 | 定义 `frame_bounds`（布局框）、`content_bounds`（实际图像）、`visible_bounds`（裁剪后可见部分）；默认锚点取框。明确图像方向校正 → 源裁剪 → `contain/cover` → 对象/组变换 → 页面坐标的次序。Scene 保存源像素到页面坐标的变换及裁剪区域。SVG 的 `viewBox` 与 `preserveAspectRatio` 也区分视口和内容适配，见 [SVG 2 坐标规范](https://www.w3.org/TR/SVG/coords.html)。 |
| P0 | 标尺样例给出 `2.5 nm/image_px`，但尚未规定非等比缩放、重采样、各向异性像素及标尺超出可见区域的结果。 | 标定属于**当前显示图像对应的源像素坐标系**，允许 x/y 独立标定。标尺长度经同一图像变换计算；默认拒绝会改变物理含义的非等比缩放，或要求显式采用方向性校准。缺标定、单位不匹配、数据处理后映射不明、标尺放不下均报错；手动长度必须明确标为未校验。ImageJ 也区分空间标定与显示标尺，且提醒导出或处理可能丢失标定，见 [ImageJ 图像标注](https://imagej.net/imaging/annotating-images)。 |
| P0 | “Scene 唯一”并不足以保证三端文字一致；不同后端可对字体替换、字形整形和换行作不同决定。 | 在布局阶段固定字体文件、替换结果、换行和字形位置；Scene 至少记录已解析字体及文本布局结果。分别验收**视觉位置**和**文本可编辑性**；路径化字形不能再声称 PDF/SVG 文字可编辑。HarfBuzz 文档说明整形会产生已定位的字形，见 [HarfBuzz](https://harfbuzz.github.io/glyphs-and-rendering.html)。 |
| P0 | 第 2、8、10 节的首版功能和阶段退出条件互相挤压；DataFrame → 图表数据的验收也早于原生图表能力。 | 将 Python/Jupyter、通用表格/图表、通用 `preset`/循环、复杂 SVG 效果、多页 TIFF 移出 v0.1 交付承诺。Python 首次接入只要求标量、图片和 Matplotlib SVG；DataFrame 直连等表格/图表语义确定后再做。 |
| P1 | `.lay` 示例混用构造、`.place(...)` 与 `.on(...)`；把位置存在图片定义上还会妨碍同一素材多次使用。 | 采用 Python 风格的 `名称 = 类型(...)` 定义画布与素材，再用 `画布实例.add(素材, ...)` 创建放置实例。`add` 返回可引用的实例；尺寸、裁剪、锚点和偏移属于该实例，标尺绑定具体的图片实例。三份真实图页验证后再冻结语法。 |
| P1 | 可复现性只写到资源哈希，字体版本、渲染器/解码器、颜色映射和导出参数仍可能漂移。 | 输出 manifest 记录引擎及 Scene 版本、字体文件标识/哈希、源素材哈希、EXIF 方向、裁剪/显示映射、色彩处理、导出选项。目标先定义为可复现的几何与视觉结果，不承诺 PDF 二进制逐字节相同。 |
| P1 | SVG 素材及 Webview 的访问边界仍偏概括。 | 先规定可导入 SVG 的安全子集和外部资源策略；预览使用受限资源根与 CSP；Python 任务仅在受信任工作区显式执行。见 [VS Code Webview](https://code.visualstudio.com/api/extension-guides/webview) 与 [Workspace Trust](https://code.visualstudio.com/api/extension-guides/workspace-trust)。 |

### 统一 DSL 语法草案：定义与放置分离

以下按用户提出的 Python 风格组织。`page` 是 `canvas(...)` 创建的画布实例；`photo` 和 `bar` 是可复用定义，`left`、`right`、`bar_left` 和 `bar_right` 是各次 `add` 返回的放置实例：

```text
page = canvas(
    name = "TEM composite",
    size = (180 mm, 120 mm),
    export_dpi = 300,
)

photo = image(
    src = "assets/tem-a.tif",
    calibration = (2.5 nm/image_px, 2.5 nm/image_px),
)

left = page.add(
    photo,
    width = 80 mm,
    anchor = top_left,
    target = page.top_left,
    offset = (8 mm, 12 mm),
)

right = page.add(
    photo,
    width = 40 mm,
    anchor = top_left,
    target = left.top_right,
    offset = (4 mm, 0 mm),
)

bar = scalebar(length = 200 nm, color = "#FFFFFF")
bar_left = page.add(
    bar,
    on = left,
    anchor = bottom_right,
    target = left.bottom_right,
    offset = (-3 mm, -3 mm),
)
bar_right = page.add(
    bar,
    on = right,
    anchor = bottom_right,
    target = right.bottom_right,
    offset = (-3 mm, -3 mm),
)
```

所有对象都由 `名称 = 类型(...)` 定义，包括画布；所有放置都由 `page.add(...)` 完成。`add` 的返回值才具有页面锚点，`photo` 本身没有 `top_right`；因此相对定位必须引用 `left` 等实例。图片源文件和标定属于不可变的 `photo` 定义，显示宽度、裁剪及位置属于每次 `add`；同一图片可用不同尺寸放置多次，修改一次放置不影响其他放置。标尺定义可复用，但 `on` 必须指向某个已放置的图片实例，标尺长度按该实例的裁剪和缩放计算。`anchor` 表示本次放置对象自身的锚点；`target` 是已有实例或画布的目标锚点。调用顺序确定绘制顺序，实例引用必须先定义，循环引用因而在 v0.1 中直接成为未定义引用错误。

此示例是 **Python 风格的 `.lay` 草案，不是可直接执行的 Python**：`80 mm` 和 `2.5 nm/image_px` 是自定义单位语法。若决定让 `.lay` 完全采用 Python 语法，应写成 `80 * mm` 和 `2.5 * nm / image_px`，并只静态接受 `canvas`、`image`、`scalebar` 与 `page.add` 等许可调用，避免布局文件执行任意代码。两种写法应在语言定稿时选定一种，不能混用。

### 标尺的一个可执行检查

以宽 1024 `image_px` 的原图为例：水平标定 2.5 nm/`image_px`，完整图像显示宽 80 mm，200 nm 标尺对应 80 个源像素，因此页面长度为 **6.25 mm**。同一原图再次显示为 40 mm 宽时，另一个标尺应为 **3.125 mm**。如果先把源图像裁成中间 512 像素，再将该区域铺满 80 mm，标尺应变为 **12.5 mm**。只用原图宽度和目标框宽度计算，会在裁剪情况下给出错误标尺。这三个情况应进入黄金测试；重采样后的源像素映射若无法追溯，应要求用户重新标定。

## 3. 建议的首版交付边界

**v0.1 要解决的工作：** 用 `.lay` 将一张双图显微组合图从源码稳定导出。包含单页物理画板、PNG 与常见灰度/RGB TIFF、图像裁剪/等比适配、文本标签、矩形背景、线/箭头、九点锚点、经过标定的水平标尺、CLI 校验与 SVG/PDF/PNG 导出。16 位灰度输入若在目标样本中出现，必须显式设定显示窗口并保留该参数；不能静默转成 8 位。

**v0.1 暂缓：** 通用分组约束、自动等距/网格、复杂 SVG 过滤器与外链、多页 TIFF、原生图表/表格、Python SDK、Notebook MIME/`%%lay`、网页入口和 PPTX。内部可以用组实现标尺，但不急于开放完整分组语法。VS Code 预览进入紧接 CLI 核心通过之后的下一验收阶段，不作为验证单位和导出语义的前置条件。

**阶段 0 先做选择实验：** 固定一套包含中英文字体、裁剪、半透明背景、箭头和两张 TIFF 的基准页；比较候选 PDF 路径对页面尺寸、文本、裁剪、透明度、嵌入图像及峰值内存的表现。先形成决策记录，再冻结渲染依赖和语言细节。

## 4. 可量化的退出条件

1. **单位：** 180 × 120 mm 页面在 300 DPI 时 PNG 输出为 2126 × 1417 px；明确使用统一的四舍五入规则。调整 DPI 不改变任何对象的物理坐标。SVG/PDF 页面尺寸另由结构检查确认。
2. **标尺：** 上述 6.25/3.125/12.5 mm 三个用例通过；修改图像宽度、裁剪和旋转后仍依据各实例的变换重算。校准模式缺失或失效标定时 CLI 非零退出，不导出带“已校准”标尺的交付文件；显式手动模式给出未校验诊断。
3. **布局：** 文本尺寸变化后依赖对象重排；依赖环报告对象名和源码位置；锚点相对 Scene 的数值结果由结构断言验证，截图仅用于发现渲染差异。
4. **输出：** 预览、SVG、PDF 和 PNG 使用同一基准页；分别检查页面尺寸、图像裁剪、透明度、字体替换诊断、文字是否保留文本和细线的矢量属性。可接受偏差应由原型样本和投稿需求确定并记录。
5. **性能：** 大 TIFF 样本记录硬件、冷/热缓存、输入大小、P50/P95 延时和峰值内存；在测到这些数值之前，不把“100 ms 预览”写成发布承诺。

## 5. 推荐的下一步

先收集三张真实但可公开测试的组合图：双 TEM 图、含中英文字和矢量图表的论文 Figure、重复模板的批量图。记录原素材尺寸/位深/标定来源及目标导出格式，并用 Typst/ImageJ 或现有工具完成同样任务，比较修改一处图片尺寸后的操作量与标尺风险。随后用第一张图实现**最小 Scene + CLI 导出**，再根据字体与 TIFF 实测结果选择 PDF/图像后端。现阶段不宜先投入完整 DSL、Notebook 和 VS Code 扩展。

**评审限制：** 未实现原型，也未测渲染器、性能、跨平台行为或实际显微素材；表中的数值是语义测试例，不是现有产品能力证明。
