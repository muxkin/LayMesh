# 页间转场

LayMesh 复用固定版本 **GL Transitions 1.74.0** 的 125 个效果，着色器来自上游 SkSL 产物。预览、窗口／全屏放映和离线 HTML 使用同一实现；PDF、图片等静态导出保持静态画面。

```css
@import "@laymesh/gl-transitions.css";
canvas {
  -laymesh-transition: InvertedPageCurl;
  -laymesh-transition-duration: 700ms;
  -laymesh-transition-timing-function: ease-in-out;
}
```

样式定义“进入此页”的效果；向后跨越同一页边界时反向播放该效果。跳页使用目标页（向前）或离开页（向后）的效果。未设置或设为 `none` 时直接切换；选择效果后省略时长／缓动，默认 `700ms`／`ease-in-out`。首次显示、同页编译及尺寸重绘不触发转场。系统开启“减少动态效果”时直接切换。

也可以用预设类：`canvas(..., stylesheet="theme.lcss", class="gl-transition-InvertedPageCurl")`。类名及效果名区分大小写。每个类仅设置效果名，可以在自己的样式中覆盖时长等属性。

参数写作 `-laymesh-transition-param-<名称>`，名称转为 kebab-case；数值、布尔和向量使用上游类型及默认值。向量写作空格或逗号分隔的分量，例如：

```css
canvas {
  -laymesh-transition: GridFlip;
  -laymesh-transition-param-size: 6 4;
  -laymesh-transition-param-bgcolor: 0 0 0 1;
  -laymesh-transition-param-randomness: 0.2;
}
```

两个效果需要额外纹理：`displacement` 使用 `-laymesh-transition-texture-displacement-map`，`luma` 使用 `-laymesh-transition-texture-luma`。使用 `url("mask.png")`，相对路径以声明所在样式文件为基准；支持 PNG、JPEG、WebP、TIFF 等位图，SVG 纹理会给出明确诊断。纹理随离线文件携带，缺失、未知效果、未知参数及无效参数会报错。

目标页准备好后开始转场，完成后启动其元素动画。快速连续导航保留最后一个目标；退出、重编译、缩放／尺寸变化和上下文丢失会取消转场。运行时着色器失败则直接显示目标页，并在渲染诊断中记录 `transitionError`。转场只更新进度，复用当前 CanvasKit 上下文及两页快照，不逐帧重新排版或读取 GPU 像素。

可运行示例：[demo.laydoc](../../examples/animation/transitions/demo.laydoc)，包括翻页、立方体和纹理揭示。离线导出：`laymesh render examples/animation/transitions/demo.laydoc -o demo.html`。

上游来源：[GL Transitions](https://github.com/gl-transitions/gl-transitions)、[SkSL 转换器](https://github.com/gl-transitions/gl-transitions/blob/e5056320c2b899c10e67076123a9f3d23e159094/scripts/targets/build-sksl.mjs)。本地版本、SHA-256 和逐效果授权见 `vendor/gl-transitions/manifest.json` 与 `release/licenses/GL-Transitions.txt`。正常构建无需 CDN。

纯 CPU 回退只对中间帧降采样；起止页保持完整分辨率。该路径可能明显慢于 GPU，实测限制见验证记录。

## transition

`document(transition=...)` 设置默认转场，`page(transition=...)` 覆盖单页。优先级为页面显式配置、文档默认配置、页面 LCSS；整份配置覆盖，不混合不同效果的参数。`transition="none"` 明确关闭，省略则继承。

```python
doc = document(transition=transition(name="InvertedPageCurl", duration=700ms, easing="ease-in-out"))
doc.add(page(src="pages/first.lay"))
doc.add(page(src="pages/second.lay", transition=transition(name="cube", params={"persp":0.6})))
doc.add(page(src="pages/last.lay", transition="none"))
```

`transition()` 的参数为 `name`、`duration`、`easing`、`params`、`textures`；仅设置名称也可写成 `transition="fade"`。参数／纹理名称使用下表的 kebab-case 拼写。纹理字典示例：`textures={"luma":"assets/mask.png"}`。路径相对于 **声明转场的文件**（包括导入的辅助文件），仅在编译使用它的页面时读取。省略时长和缓动仍为 700ms / ease-in-out。同一页面源文件可以在不同页面引用中使用不同效果。

## 完整效果目录

下表参数名已经转成 CSS kebab-case；未列出的配置沿用上游默认值。兼容性测量见 [验证记录](../benchmarks/page-transitions.zh-CN.md)。

<!-- generated-catalog -->

| Effect / preset suffix | CSS parameter defaults | Extra texture | License |
| --- | --- | --- | --- |
| `AdvancedMosaic` | `pixel-size`: `50` | — | MIT |
| `BlockDissolve` | `blocksize`: `0.02` | — | MIT |
| `BookFlip` | — | — | MIT |
| `Bounce` | `shadow-colour`: `[0,0,0,0.6]`; `shadow-height`: `0.075`; `bounces`: `3` | — | MIT |
| `BowTieHorizontal` | — | — | MIT |
| `BowTieVertical` | — | — | MIT |
| `BowTieWithParameter` | `adjust`: `0.5`; `reverse`: `false` | — | MIT |
| `Box` | `rect-in`: `1`; `location`: `0` | — | MIT |
| `ButterflyWaveScrawler` | `amplitude`: `1`; `waves`: `30`; `color-separation`: `0.3` | — | MIT |
| `CircleCrop` | `bgcolor`: `[0,0,0,1]` | — | MIT |
| `ColourDistance` | `power`: `5` | — | MIT |
| `CrazyParametricFun` | `a`: `4`; `b`: `1`; `amplitude`: `120`; `smoothness`: `0.1` | — | MIT |
| `CrossZoom` | `strength`: `0.4` | — | MIT |
| `DefocusBlur` | `blur-size`: `0.02` | — | MIT |
| `Directional` | `direction`: `[0,1]` | — | MIT |
| `DirectionalScaled` | `direction`: `[0,1]`; `scale`: `0.7` | — | MIT |
| `DoomScreenTransition` | `bars`: `30`; `amplitude`: `2`; `noise`: `0.1`; `frequency`: `0.5`; `drip-scale`: `0.5` | — | MIT |
| `Dreamy` | — | — | MIT |
| `DreamyZoom` | `rotation`: `6`; `scale`: `1.2` | — | MIT |
| `Drop_Zone_Flicker` | `frame-rate`: `24`; `rgb-offset`: `0.014`; `block-amount`: `0.72`; `ghost-amount`: `0.62`; `red-cyan`: `0.58`; `scanline`: `0.075` | — | MIT |
| `EdgeTransition` | `edge-thickness`: `0.001`; `edge-brightness`: `8` | — | MIT |
| `FilmBurn` | `seed`: `2.31` | — | MIT |
| `Fold` | — | — | MIT |
| `GlitchDisplace` | — | — | MIT |
| `GlitchMemories` | — | — | MIT |
| `GridFlip` | `size`: `[4,4]`; `pause`: `0.1`; `divider-width`: `0.05`; `bgcolor`: `[0,0,0,1]`; `randomness`: `0.1` | — | MIT |
| `HSVfade` | — | — | MIT |
| `HorizontalClose` | — | — | MIT |
| `HorizontalOpen` | — | — | MIT |
| `InvertedPageCurl` | — | — | BSD 3 Clause |
| `LeftRight` | — | — | MIT |
| `LinearBlur` | `intensity`: `0.1` | — | MIT |
| `Mosaic` | `endx`: `2`; `endy`: `-1` | — | MIT |
| `Overexposure` | `strength`: `0.6` | — | MIT |
| `PolkaDotsCurtain` | `dots`: `20`; `center`: `[0,0]` | — | MIT |
| `PuzzleRight` | `size`: `[4,4]`; `pause`: `0.1`; `divider-width`: `0.005` | — | MIT |
| `Radial` | `smoothness`: `1` | — | MIT |
| `Rectangle` | `bgcolor`: `[0,0,0,1]` | — | MIT |
| `RectangleCrop` | `bgcolor`: `[0,0,0,1]` | — | MIT |
| `Revolve_Left` | `center`: `[0.46,0.52]`; `direction`: `-1`; `max-rotation`: `1.95`; `peak-zoom`: `2.22`; `swirl`: `2.85`; `barrel`: `0.38`; `motion-blur`: `1`; `switch-start`: `0.3`; `switch-end`: `0.5`; `shadow`: `0.16` | — | MIT |
| `Rolls` | `type`: `0`; `rot-down`: `false` | — | MIT |
| `RotateScaleVanish` | `fade-in-second`: `true`; `reverse-effect`: `false`; `reverse-rotation`: `false` | — | MIT |
| `SimpleFlip` | — | — | MIT |
| `SimpleZoom` | `zoom-quickness`: `0.8` | — | MIT |
| `SimpleZoomOut` | `zoom-quickness`: `0.8`; `fade`: `true` | — | MIT |
| `Slides` | `type`: `0`; `in`: `false` | — | MIT |
| `StarWipe` | `border-thickness`: `0.01`; `star-rotation`: `0.75`; `border-color`: `[1,1,1,1]`; `star-center`: `[0.5,0.5]` | — | MIT |
| `StaticFade` | `n-noise-pixels`: `200`; `static-luminosity`: `0.8` | — | MIT |
| `StereoViewer` | `zoom`: `0.88`; `corner-radius`: `0.22` | — | BSD 2 Clause |
| `StripDatamoshGlitch` | `strength`: `1`; `horizontal-bars`: `42`; `vertical-slits`: `18`; `tear`: `0.18`; `chroma`: `0.032`; `residue`: `0.62`; `noise-amount`: `0.16`; `scan-amount`: `0.13`; `flash-amount`: `0.2` | — | MIT |
| `Swirl` | — | — | MIT |
| `TVStatic` | `offset`: `0.05` | — | MIT |
| `TilesWave` | `tile-count`: `[8,8]`; `flip-x`: `true`; `flip-y`: `false` | — | MIT |
| `TopBottom` | — | — | MIT |
| `VerticalClose` | — | — | MIT |
| `VerticalOpen` | — | — | MIT |
| `WaterDrop` | `amplitude`: `30`; `speed`: `30` | — | MIT |
| `ZoomInCircles` | — | — | MIT |
| `ZoomLeftWipe` | `zoom-quickness`: `0.8` | — | MIT |
| `ZoomRigthWipe` | `zoom-quickness`: `0.8` | — | MIT |
| `angular` | `starting-angle`: `90` | — | MIT |
| `burn` | `color`: `[0.9,0.4,0.2]` | — | MIT |
| `burn0` | `burn-color`: `[1,0.5,0]` | — | MIT |
| `cannabisleaf` | — | — | MIT |
| `chessboard` | `grid-num`: `10` | — | MIT |
| `circle` | `center`: `[0.5,0.5]`; `back-color`: `[0.1,0.1,0.1]` | — | MIT |
| `circleopen` | `smoothness`: `0.3`; `opening`: `true` | — | MIT |
| `colorphase` | `from-step`: `[0,0.2,0.4,0]`; `to-step`: `[0.6,0.8,1,1]` | — | MIT |
| `coord-from-in` | — | — | MIT |
| `crosshatch` | `center`: `[0.5,0.5]`; `threshold`: `3`; `fade-edge`: `0.1` | — | MIT |
| `crosswarp` | — | — | MIT |
| `cube` | `persp`: `0.7`; `unzoom`: `0.3`; `reflection`: `0.4`; `floating`: `3` | — | MIT |
| `directional-easing` | `direction`: `[0,1]` | — | MIT |
| `directionalwarp` | `smoothness`: `0.1`; `direction`: `[-1,1]` | — | MIT |
| `directionalwipe` | `direction`: `[1,-1]`; `smoothness`: `0.5` | — | MIT |
| `displacement` | `strength`: `0.5` | displacement-map | MIT |
| `dissolve` | `u-line-width`: `0.1`; `u-spread-clr`: `[1,0,0]`; `u-hot-clr`: `[0.9,0.9,0.2]`; `u-pow`: `5`; `u-intensity`: `1` | — | MIT |
| `doorway` | `reflection`: `0.4`; `perspective`: `0.4`; `depth`: `3` | — | MIT |
| `fade` | — | — | MIT |
| `fadecolor` | `color`: `[0,0,0]`; `color-phase`: `0.4` | — | MIT |
| `fadegrayscale` | `intensity`: `0.3` | — | MIT |
| `flyeye` | `size`: `0.04`; `zoom`: `50`; `color-separation`: `0.3` | — | MIT |
| `fragment` | — | — | MIT |
| `heart` | — | — | MIT |
| `hexagonalize` | `steps`: `50`; `horizontal-hexagons`: `20` | — | MIT |
| `kaleidoscope` | `speed`: `1`; `angle`: `1`; `power`: `1.5` | — | MIT |
| `luma` | — | luma | MIT |
| `luminance_melt` | `direction`: `true`; `l-threshold`: `0.8`; `above`: `false` | — | MIT |
| `morph` | `strength`: `0.1` | — | MIT |
| `mosaic_transition` | `mosaic-num`: `10` | — | MIT |
| `multiply_blend` | — | — | MIT |
| `old_tv_lost_signal` | — | — | MIT |
| `parametric_glitch` | `ampx`: `1`; `ampy`: `1` | — | MIT |
| `perlin` | `scale`: `4`; `smoothness`: `0.01`; `seed`: `12.9898` | — | MIT |
| `pinwheel` | `speed`: `2` | — | MIT |
| `pixelize` | `squares-min`: `[20,20]`; `steps`: `50` | — | MIT |
| `polar_function` | `segments`: `5` | — | MIT |
| `powerKaleido` | `scale`: `2`; `z`: `1.5`; `speed`: `5` | — | MIT |
| `randomNoisex` | — | — | MIT |
| `randomsquares` | `size`: `[10,10]`; `smoothness`: `0.5` | — | MIT |
| `ripple` | `amplitude`: `100`; `speed`: `50` | — | MIT |
| `rotateTransition` | — | — | MIT |
| `rotate_scale_fade` | `center`: `[0.5,0.5]`; `rotations`: `1`; `scale`: `8`; `back-color`: `[0.15,0.15,0.15,1]` | — | MIT |
| `scale-in` | — | — | MIT |
| `splitSlideInHorizontal` | `reverse`: `false` | — | MIT |
| `splitSlideInOutHorizontal` | `reverse`: `false` | — | MIT |
| `splitSlideInOutVertical` | `reverse`: `false` | — | MIT |
| `splitSlideInVertical` | `reverse`: `false` | — | MIT |
| `splitSlideOutHorizontal` | `reverse`: `false` | — | MIT |
| `splitSlideOutVertical` | `reverse`: `false` | — | MIT |
| `squareswire` | `squares`: `[10,10]`; `direction`: `[1,-0.5]`; `smoothness`: `1.6` | — | MIT |
| `squeeze` | `color-separation`: `0.04` | — | MIT |
| `static_wipe` | `u-transition-up-to-down`: `true`; `u-max-static-span`: `0.5` | — | MIT |
| `swap` | `reflection`: `0.4`; `perspective`: `0.2`; `depth`: `3` | — | MIT |
| `tangentMotionBlur` | — | — | MIT |
| `undulatingBurnOut` | `smoothness`: `0.03`; `center`: `[0.5,0.5]`; `color`: `[0,0,0]` | — | MIT |
| `wind` | `size`: `0.2`; `reversed`: `false` | — | MIT |
| `windowblinds` | — | — | MIT |
| `windowslice` | `count`: `10`; `smoothness`: `0.5` | — | MIT |
| `wipeDown` | — | — | MIT |
| `wipeLeft` | — | — | MIT |
| `wipeRight` | — | — | MIT |
| `wipeUp` | — | — | MIT |
| `x_axis_translation` | — | — | MIT |
| `zoomInOut` | — | — | MIT |
