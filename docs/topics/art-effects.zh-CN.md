# 艺术字、阴影与发光

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

布局尺寸和锚点保持原有含义，`subjectBounds` 记录变形后字形的占用，装饰或效果越出画布时给出 `W_EFFECT_OVERFLOW`，不会自动重排。SVG、WASM、VS Code 预览和原生位图导出共用滤镜定义。艺术字以矢量路径绘制，并在 SVG 标签与 PDF ActualText 中保留原文；普通文字保持文本输出。

PDF 保留主体、描边、变形和挤出为矢量，仅将效果层按有效导出 DPI 局部栅格化，默认 1200 DPI。分块渲染保留模糊与扩散的重叠区域后再裁剪，绕过依赖的 5000 像素滤镜上限。所有效果总预算为一亿像素，单块 RGBA 缓冲区限 64 MB；超限明确报错，不静默降质。特别大的模糊半径可能需要降低 DPI。外部 SVG 沿用原有安全白名单，内部效果渲染不开放任意外部 SVG 滤镜。

[艺术字源码](../../examples/effects/art-text.lay) · [阴影与发光源码](../../examples/effects/shadow-glow.lay) · [完整参数](interface-reference.zh-CN.md)

`laymesh inspect --json` 的 `decorations` 列表提供布局、主体和效果边界及到页面的变换。span 显式颜色覆盖继承的字形填充；span 自身的 text_fill 优先。
