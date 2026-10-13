# Page transitions

LayMesh reuses all 125 effects from pinned **GL Transitions 1.74.0**, using upstream SkSL output in its existing CanvasKit renderer. The same implementation serves preview, window/fullscreen presentation and offline HTML. Static exports retain their original behavior.

```css
@import "@laymesh/gl-transitions.css";
canvas {
  -laymesh-transition: InvertedPageCurl;
  -laymesh-transition-duration: 700ms;
  -laymesh-transition-timing-function: ease-in-out;
}
```

A page defines its entry effect. Backward navigation reverses the effect on the boundary being crossed. Jumps use the destination effect when advancing and the departing page effect when going back. Missing configuration or `none` switches immediately. An effect without duration/easing defaults to `700ms` / `ease-in-out`. Initial display, same-page recompilation and resize do not animate. Reduced-motion preference skips transitions.

Preset classes have the form `gl-transition-InvertedPageCurl`; effect and class names are case sensitive. Set `class` and `stylesheet` on the canvas. Classes set only the effect, leaving duration and easing overridable.

Use `-laymesh-transition-param-<kebab-name>` for upstream numeric, boolean and vector parameters. Vectors accept space- or comma-separated components, such as `-laymesh-transition-param-size: 6 4` for `GridFlip`. Defaults below come from upstream.

Two effects require textures: `displacement` needs `-laymesh-transition-texture-displacement-map`, and `luma` needs `-laymesh-transition-texture-luma`. Supply `url("mask.png")`, relative to the declaring stylesheet. Raster images including PNG, JPEG, WebP and TIFF are supported; SVG textures receive a diagnostic. Textures are packaged offline. Missing textures and unknown/invalid effect parameters are diagnosed.

The destination is prepared before transition playback; its element timeline starts afterward. Rapid navigation keeps the last target. Exit, recompile, zoom/resize and context loss cancel playback. Runtime failures show the destination immediately and record `transitionError`. The renderer reuses its context and two snapshots, without per-frame layout or GPU readback.

Run [demo.laydoc](../../examples/animation/transitions/demo.laydoc) or export it with `laymesh render examples/animation/transitions/demo.laydoc -o demo.html`.

Sources: [GL Transitions](https://github.com/gl-transitions/gl-transitions) and [upstream converter](https://github.com/gl-transitions/gl-transitions/blob/e5056320c2b899c10e67076123a9f3d23e159094/scripts/targets/build-sksl.mjs). Version, SHA-256 hashes and effect licenses are recorded in `vendor/gl-transitions/manifest.json` and `release/licenses/GL-Transitions.txt`; normal builds need no CDN.

The CPU fallback downsamples only intermediate frames; endpoints remain full resolution. It can be substantially slower than GPU playback; see the measured limits below.

## transition

Set document defaults with `document(transition=...)` and override individual pages with `page(transition=...)`. Precedence is page override, document default, then page LCSS. Overrides replace the entire effect configuration. `transition="none"` disables transitions; omission inherits.

```python
doc = document(transition=transition(name="InvertedPageCurl", duration=700ms, easing="ease-in-out"))
doc.add(page(src="pages/first.lay"))
doc.add(page(src="pages/second.lay", transition=transition(name="cube", params={"persp":0.6})))
doc.add(page(src="pages/last.lay", transition="none"))
```

`transition()` accepts `name`, `duration`, `easing`, `params`, and `textures`. A string such as `transition="fade"` is shorthand. Parameter and texture keys use the kebab-case names below. Texture paths, e.g. `textures={"luma":"assets/mask.png"}`, resolve relative to the declaration file, including imported helpers. Texture loading remains lazy. Defaults remain 700ms / ease-in-out. Repeated references to one source can use different effects.

## Complete catalog

Parameter names below use CSS kebab-case. See the [validation record](../benchmarks/page-transitions.zh-CN.md) for measured compatibility and environment limits.

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
