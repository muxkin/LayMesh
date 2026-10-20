# 艺术字、静态效果与对象级三维

所有装饰默认关闭。`border_*` 控制布局外框；`text_stroke_*` 沿实际字形描边。字体继续由用户提供，并遵守原有嵌入许可检查。

<!-- example:examples/effects/art-text.lay -->

## Glyph paint

`text_fill` 支持颜色、线性和径向渐变等已有填充。渐变覆盖整段变形文字，不在每个字形上重新开始。`text_stroke_color` 与 `text_stroke_width` 设置居中的实线字形描边，裸线宽使用 pt。未设置 `text_fill` 时沿用 `color`；`span` 可分别覆盖字形填充和描边。

## text_path

`path=text_path(route,start=0mm,align="center",reverse=false)` 沿本地单条开放路径排字，支持 `path`、`line`、`polyline` 和 `arc` 素材。字形中心按弧长定位，并随切线旋转；保留字形塑形、字距与字体回退。负起始距离、路径长度不足、多子路径、闭合或零长度路径及多行文字均报错。行内公式支持变形和挤出，本版本不支持沿路径排公式。

## text_warp

`warp=text_warp("arc",angle=45deg)` 生成正向弧形字。`text_warp("wave",amplitude=2mm,wavelength=20mm,phase=0deg)` 按正弦曲线移动基线。`text_warp("perspective",corners=[(0mm,0mm),(60mm,2mm),(55mm,15mm),(5mm,12mm)])` 进行四角透视映射，角点顺序为左上、右上、右下、左下；退化、自交或非凸四边形报错。非线性变形生成矢量轮廓，源轮廓细分容差为 0.002 mm。

## text_extrude

`extrude=text_extrude(depth=1mm,angle=45deg,color="#555555")` 生成矢量背面和沿边扫掠的侧面，可控制深度、方向与侧面颜色。采用二维矢量挤出，不包含三维相机或光照。处理顺序固定为排版、路径定位、整体变形、挤出、描边和效果。

## shadow

`effects=[shadow(color="#000000",opacity=0.5,blur=1mm,spread=0mm,offset=(1mm,1mm),mode="outer",target="content")]` 根据可见内容的 Alpha 轮廓生成阴影，包括透明图片的实际轮廓。`blur` 是高斯标准差，`spread` 是非负扩散半径；偏移沿局部坐标，x 向右、y 向下。`mode="inner"` 把阴影限制在主体内部。颜色 Alpha 与效果透明度相乘。

## glow

`glow(color="#ffffff",opacity=1,blur=1mm,spread=0mm,mode="outer",target="content")` 与阴影共享参数，但没有偏移。外效果位于主体后方，内效果位于主体前方，各层内按列表顺序绘制；效果依据主体 Alpha 独立生成，不串联前一个效果。`target="object"` 包含底框与背景，默认 `content` 排除底框。组的内容包含子对象；组缩放和旋转同步作用于效果。

<!-- example:examples/effects/shadow-glow.lay -->

## LCSS decorations

LCSS 支持 `text-fill`、`text-stroke-color`、`text-stroke-width`、`effects`、`path`、`warp` 和 `extrude`。配置使用构造器字面表达式，枚举建议加引号；LCSS 中的路径需写成本地素材字面表达式，不能引用 `.lay` 变量。

```lcss
.title {
  text-fill: linear-gradient(to right, #ea366d, #365de6);
  text-stroke-color: #ffffff;
  text-stroke-width: 0.7pt;
  effects: [shadow(blur=0.8mm, offset=(1mm,2mm))];
  warp: text_warp("wave", amplitude=1mm, wavelength=25mm);
  extrude: text_extrude(depth=1mm, color="#213864");
}
```

## Geometry and export

布局尺寸和锚点保持原有含义，`subjectBounds` 记录变形后字形的占用，装饰或效果越出画布时给出 `W_EFFECT_OVERFLOW`，不会自动重排。原生和 WASM 预览共用效果参数与三维算法。艺术字以矢量路径绘制，并在 SVG 标签与 PDF ActualText 中保留原文；普通文字保持文本输出。

未启用三维或柔边时，PDF 保留主体、描边、变形和二维挤出为矢量，仅将效果层按有效导出 DPI 局部栅格化，默认 1200 DPI。分块渲染保留模糊与扩散的重叠区域后再裁剪，绕过依赖的 5000 像素滤镜上限。所有效果总预算为一亿像素，单块 RGBA 缓冲区限 64 MB；超限明确报错，不静默降质。特别大的模糊半径可能需要降低 DPI。外部 SVG 沿用原有安全白名单，内部效果渲染不开放任意外部 SVG 滤镜。

[艺术字源码](../../examples/effects/art-text.lay) · [阴影与发光源码](../../examples/effects/shadow-glow.lay) · [完整参数](interface-reference.zh-CN.md)

`laymesh inspect --json` 的 `decorations` 列表提供布局、主体和效果边界及到页面的变换。span 显式颜色覆盖继承的字形填充；span 自身的 text_fill 优先。

标准 CSS `box-shadow`、`text-shadow` 与单个 `filter: drop-shadow(...)` 构成独立的外观效果。原有 `effects` 构造器保留模糊标准差、作用对象和叠放语义，见[CSS 效果与局部覆盖](lcss.zh-CN.md#css-effects)。

## 样式与动画综合案例

[三库卡片与艺术字海报](style-animation-examples.zh-CN.md)展示主题、布局、关键帧和步骤如何一起使用，并提供已有复杂仪表板场景的完整源码。

## reflection

`reflection(distance=1mm,length=0.5,opacity=0.4,fade=1,blur=0mm,target="content")` 把投影后的主体沿其底边向下镜像。`distance` 是与底边的间隔；`length` 是保留的主体高度比例（0–1，不压缩倒影）；`opacity` 是上端透明度；下端透明度为 `opacity*(1-fade)`。`blur` 是高斯标准差。倒影不再复制阴影、发光或其他倒影。

<!-- example:examples/manual/reflection.lay -->

## soft_edge

`soft_edge(radius=1mm,target="content")` 在实际可见 Alpha 轮廓上向内柔化，包括字形孔洞、透明图片边缘和三维投影。它不模糊主体颜色，不增加可见范围；0 半径等同关闭。多个柔边取最大半径。柔边始终先作用于主体，阴影、发光和倒影随后独立生成；列表顺序仅决定同类前／后装饰层的叠放。

<!-- example:examples/manual/soft_edge.lay -->

## three_d

`three_d=three_d(...)` 适用于形状、文字、公式和图片，也可在 `add` 时覆盖。默认厚度 2 mm、XYZ 旋转为零、无斜面、正交投影、哑光材质及左上方白光；不设置该参数时完全沿用二维绘制。前后面和侧面保留孔洞，描边参与轮廓。图片使用裁剪和圆角后的平板轮廓，原图作为纹理；透明区域保留 Alpha。

| 参数 | 含义与默认值 |
| --- | --- |
| `depth` | 总厚度，默认 `2mm`，可为零 |
| `rotation` | `(X,Y,Z)`，默认 `(0deg,0deg,0deg)`；绕可见轮廓包围盒中心依次 X→Y→Z 旋转，再应用二维放置变换 |
| `bevel_top`、`bevel_bottom` | 前／后斜面的 `(宽度,高度)`，默认 `(0mm,0mm)`；两侧高度总和不得超过厚度 |
| `projection`、`distance` | `"orthographic"` 或 `"perspective"`；相机在 +Z 轴，透视距离默认 `500mm` |
| `material` | `"flat"` 平涂、`"matte"` 哑光、`"plastic"` 塑料、`"metal"` 金属；后三种使用表面法线和灯光明暗 |
| `light_direction` | 指向光源的非零向量，默认 `(-1,-1,2)`；X 向右、Y 向下、Z 朝观察者 |
| `light_color`、`intensity`、`ambient` | 默认 `"#ffffff"`、`1`、`0.25`；强度非负，环境光比例 0–1 |

<!-- example:examples/manual/three_d.lay -->

组可以包含多个独立三维对象；不支持把整个组、图表或视频变为统一三维场景。对象内部用深度缓冲解决遮挡，不同对象继续按页面图层顺序合成。没有跨对象投影、折射或全局光照；`text_extrude()` 的二维矢量语义保持兼容。退化轮廓、穿过物体的相机和会破坏孔洞／边缘的斜面明确报错；遇到窄笔画应减小斜面宽度。

Rust 共享轮廓、三角网格、投影与软件光栅化，Skia／CanvasKit 提供二维纹理和页面合成。投影不改变布局框与锚点；可见边界另算，用于裁剪、溢出和缓存。显示列表版本为 3，读取仍兼容 1、2；旧运行时需随新版应用一起更新。重复预览复用网格与当前分辨率纹理，放大重新生成，单对象修改保留无关对象缓存。

三维主体在 SVG／PDF／PPTX 中按有效导出 DPI 局部图片化，柔边主体也可能局部图片化；可分离的装饰层独立输出。其余对象继续走原有矢量／文本导出路径。PPTX 不写入 PowerPoint 原生三维参数，完整参数保存在 `.lay`。导出保留一亿像素限制和分块内存预算，三维工作块限 64 MiB、纹理缓存限 256 MiB，超限报错且不覆盖已有文件。技术性回退信息不进入画面。

网页编辑器缩放、平移和调整窗口时，先整体移动已有画面；停止后按当前视口与设备像素比统一替换高清结果。源码不因缩放重新编译，旧视口的延迟结果会被丢弃。三维曲线按投影误差细分，圆滑侧面插值法线，硬边保留；静止绘制使用 4×4 覆盖采样。材质透明度与几何覆盖度分开处理，用户设置的柔边、发光和倒影模糊仍然有效。v3 显示列表新增可选几何重建数据；缺少这些数据的旧列表仍能绘制原网格。已有离线 HTML 不会自动更新，需要重新导出或使用新文件。

## 效果预设

`canvas(stylesheet="@laymesh/effects.css")` 导入 30 个可组合预设，给对象设置 `class="fx-bevel-button"` 即可使用。LCSS 属性为 `three-d: three_d(...)`，`effects` 支持上述新构造器；对象显式参数优先于样式。预设覆盖阴影、发光、倒影、柔边、前后斜面、四种材质与多个三维视角；各项都有独立可编辑源码。

<!-- style-catalog:effects -->

[30 项对照源码](../../examples/effects/presets.lay) · [形状／文字／公式／图片组合](../../examples/effects/object-3d.lay) · [预设清单](../../examples/effects/presets.json)。运行 `python scripts/build-effects-gallery.py --binary target/debug/laymesh` 生成独立 HTML 画廊、PNG 和源码副本。

Python 接口直接使用相同源码：`render_source(source, output="effects.pdf", dpi=300)`。没有单独的 Python 三维语法；Notebook、CLI 和编辑器共享参数校验。

类别参考 [Microsoft 的形状效果说明](https://support.microsoft.com/en-us/office/graphics-visuals/add-a-fill-or-effect-to-a-shape-or-text-box)，不保证 PowerPoint 逐像素一致；LayMesh 允许在三维投影后继续应用柔边。

[本次可复现验证记录](../benchmarks/object-effects-2026-10-19/README.zh-CN.md)。
