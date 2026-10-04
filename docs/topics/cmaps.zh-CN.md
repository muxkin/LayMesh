# 颜色映射与预设

<!-- walkthrough:start -->
## 用途与概念

cmap 是不可变命名配色对象，palette 返回可直接用于循环或 plot_style 的颜色列表。预设使用 Matplotlib 3.11.2 原生颜色表，普通运行无需 Matplotlib。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/cmaps.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

按 cmap_names() 遍历全部预设；分类配色按原始顺序取色，连续配色均匀采样，循环配色避开重复端点。共享 color_scale 保持数据图层和色标一致。

<!-- example:examples/manual/cmaps.lay -->

## 常见错误与限制

名称区分大小写，支持原生别名与 _r；rdbu 是 RdBu 的兼容别名。n 为 0–10,000 的整数，位置 t 必须有限，范围外钳制。原有配色已统一为 Matplotlib 颜色，部分旧输出会变化。

## 逐项功能说明

### cmap

获取 Matplotlib 3.11.2 命名配色对象；不可变，可取色、获取颜色列表和反向。

返回：cmap

[最小完整源码](../../examples/manual/cmaps.lay) · [组合源码](../../examples/plot/cmap-presets.lay) · [全部参数](interface-reference.zh-CN.md#cmap)

### cmap_names

查询独立配色预设，可按类别过滤并包含反向名称；不重复列出别名。

返回：list

[最小完整源码](../../examples/manual/cmaps.lay) · [组合源码](../../examples/plot/cmap-presets.lay) · [全部参数](interface-reference.zh-CN.md#cmap_names)

### palette

生成配色列表；分类颜色按顺序轮换，连续均匀采样，循环配色不重复端点。

返回：list

[最小完整源码](../../examples/manual/cmaps.lay) · [组合源码](../../examples/plot/cmap-presets.lay) · [全部参数](interface-reference.zh-CN.md#palette)

### cmap-sample

按有限归一化位置取色；范围外钳制到 0–1。

返回：color

必需输入：`t`.

[最小完整源码](../../examples/manual/cmaps.lay) · [组合源码](../../examples/plot/cmap-presets.lay) · [全部参数](interface-reference.zh-CN.md#cmap-sample)

### cmap-colors

生成颜色列表；n 默认原生数量，分类轮换、连续均匀采样、循环不重复端点。

返回：list

[最小完整源码](../../examples/manual/cmaps.lay) · [组合源码](../../examples/plot/cmap-presets.lay) · [全部参数](interface-reference.zh-CN.md#cmap-colors)

### cmap-reversed

返回反向配色对象，使用 Matplotlib 原生反向颜色表。

返回：cmap

[最小完整源码](../../examples/manual/cmaps.lay) · [组合源码](../../examples/plot/cmap-presets.lay) · [全部参数](interface-reference.zh-CN.md#cmap-reversed)

<!-- walkthrough:end -->

## 详细行为与补充示例

名称区分大小写，接受 Matplotlib 原生别名和 `_r` 名称；`rdbu` 对应 `RdBu`。`cmap_names()` 返回 87 种独立预设，`reversed=true` 返回 174 个独立及反向名称；全部上游注册别名仍可直接使用。

`cmap("viridis")` 返回不可变配色对象，`.sample(t)` 将有限位置钳制到 0–1，`.colors(n)` 返回保留颜色类型的列表，`.reversed()` 返回独立反向对象。`palette("tab10")` 等价于 `cmap("tab10").colors()`。

连续与发散配色均匀采样，单色取中点；循环配色采样 `[0, 1)`，避免重复端点。分类配色按原始颜色顺序取色，数量超过原始长度后轮换。省略或传入 null 的 n 使用原生颜色表长度，n=0 返回空列表，其他 n 须为 0–10,000 的整数。

```lay
cm = cmap("coolwarm")
scale = color_scale(norm="centered", vmin=-2, vmax=2, center=0, cmap=cm)
style = plot_style(colors=palette("tab10"))
```

同一个 color_scale 可供热图、填充等高线、数值散点和色标共享。原有自定义颜色列表仍按 RGB 和透明度插值；图层显式颜色优先于样式颜色轮换。

[字典循环绘图](../../examples/plot/dictionary-series.lay) · [共享色标](../../examples/plot/cmap-scales.lay) · [完整配色总览](../../examples/plot/cmap-presets.lay)

### 预设目录

| 类别 | 预设 |
| --- | --- |
| `sequential` | `magma`, `inferno`, `plasma`, `viridis`, `cividis`, `Blues`, `BuGn`, `BuPu`, `GnBu`, `Greens`, `Greys`, `OrRd`, `Oranges`, `PuBu`, `PuBuGn`, `PuRd`, `Purples`, `RdPu`, `Reds`, `Wistia`, `YlGn`, `YlGnBu`, `YlOrBr`, `YlOrRd`, `afmhot`, `autumn`, `bone`, `cool`, `copper`, `gist_heat`, `gray`, `hot`, `pink`, `spring`, `summer`, `winter` |
| `diverging` | `berlin`, `managua`, `vanimo`, `BrBG`, `PRGn`, `PiYG`, `PuOr`, `RdBu`, `RdGy`, `RdYlBu`, `RdYlGn`, `Spectral`, `bwr`, `coolwarm`, `seismic` |
| `cyclic` | `twilight`, `twilight_shifted`, `hsv` |
| `qualitative` | `Accent`, `okabe_ito`, `Dark2`, `Paired`, `Pastel1`, `Pastel2`, `Set1`, `Set2`, `Set3`, `tab10`, `tab20`, `tab20b`, `tab20c` |
| `misc` | `turbo`, `CMRmap`, `binary`, `brg`, `cubehelix`, `flag`, `gist_earth`, `gist_gray`, `gist_ncar`, `gist_rainbow`, `gist_stern`, `gist_yarg`, `gnuplot`, `gnuplot2`, `jet`, `nipy_spectral`, `ocean`, `prism`, `rainbow`, `terrain` |

### 兼容性与复现

颜色统一为 Matplotlib 3.11.2 的原生颜色表及反向表，并量化为 8 位 RGB；旧 cividis/RdBu 等输出可能变化。数据与上游源码哈希已保存，普通原生/WASM 构建无需 Python 或 Matplotlib。重新生成或校验时，安装可选的 `release/cmap-requirements.txt`，运行 `python scripts/build-cmaps.py`，校验时追加 `--check`。发行说明包含来源及 Matplotlib 许可证。
