# Matplotlib 导入

复用已有 Matplotlib Figure，通过 Python 桥接将它作为图像放入页面。

平台 wheel 内置一个 Rust 原生程序，无需额外安装运行时。首次 PyPI 发布前，可安装已审核的本地 wheel；正式发布后使用下列命令。原生数据不依赖 Matplotlib，导入 Figure 时增加 `plot` 额外依赖。

```sh
python -m pip install --pre "laymesh[plot,data]"
```

[安装与源码开发](install.zh-CN.md)

<!-- example:examples/gallery/notebook/vector.lay -->

## 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `--dpi` | 最终 PNG 分辨率 | 不控制 Figure 素材 |
| `--plot-dpi` | Figure 回退 PNG 分辨率 | 300 |

## 使用流程


先按前文安装 `plot` 可选依赖。这条路线复用已有 Matplotlib 图形，再放入 LayMesh 页面。

先在 Python 单元格中处理数据、生成 Figure：

```python
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

x = np.linspace(0, 2 * np.pi, 100)
df = pd.DataFrame({"x": x, "y": np.sin(x)})
fig, ax = plt.subplots(figsize=(4, 2.5))
ax.plot(df["x"], df["y"])
title = f"Peak: {df['y'].max():.2f}"
```

然后加载扩展，在下一个单元格直接写 `.lay`：

```text
%load_ext laymesh.ipython
```

```text
%%laymesh -o figure.pdf --save-source figure.lay
page = canvas(size=(160 mm, 100 mm), background="#ffffff")
chart = image(src={{fig}})
placed = page.add(chart,size=(130 mm, auto),
                  target=page.top_left, offset=(15 mm, 10 mm))
caption = text(content={{title}}, font_size=10 pt)
page.add(caption, target=placed.bottom_left, offset=(0 mm, 4 mm))
```



单元格会显示 SVG 预览，同时导出 `figure.pdf`。`--save-source figure.lay` 额外保存展开后的布局和 `figure.assets/` 中的绘图素材，可在 Notebook 之外重新运行 `laymesh render figure.lay -o another.pdf`。重复执行只覆盖带有 LayMesh 生成标记的 `.lay`；不会覆盖手写文件。生成素材使用内容哈希命名，旧版本素材可能留在目录中。无需保存独立源码时省略 `--save-source`，Notebook 本身就是可重复运行的源码。

已有布局文件可用行 Magic：

```text
%laymesh figures/layout.lay -o figures/output.png --dpi 300
```

此时相对图片、字体及组件路径从 `layout.lay` 所在目录解析。`%%laymesh` 中的相对路径则从 Notebook 进程的当前工作目录解析；可在 Python 中运行 `Path.cwd()` 确认。`-o` 支持 SVG、PDF、PNG；不写 `-o` 只生成临时预览。`--dpi` 控制最终 PNG 的像素尺寸，`--plot-dpi`（默认 300）控制 Matplotlib 图形必须栅格化时的分辨率，两者互不影响。

`{{name}}` 只引用 Notebook 命名空间中的一个变量，必须写在 `.lay` 字符串引号外。支持 Python 字符串、布尔值、有限的数字、NumPy 标量、`Path`、Matplotlib `Figure`，以及原生绘图用的一维/二维数值数组、Series 和 DataFrame。数组与表格保存为 JSON 数据资源；NaN/None/pandas NA 表示缺失值，无穷值会拒绝。未定义变量和 Python 表达式仍不接受。原生数据绑定、热图和保存后独立运行见[科研绘图指南](../plotting.zh-CN.md)。



Matplotlib 折线图等受支持的内容会保留为 SVG 矢量图。图中文字默认导出为矢量路径，视觉稳定，但不是可选中的文字。热图或其他超出现有 LayMesh SVG 安全子集的内容会自动改用 PNG，Notebook 会显示回退提示；LayMesh 自身的文字仍按原有方式导出。外部 SVG 文件不会经过 Matplotlib 适配器，仍执行 LayMesh 原有的安全检查。

如果出现 `W_PLOT_BOUNDS`，说明 Figure 中有可见内容超出了原始画布边界。例如坐标轴标题可能位于画布外：Jupyter 内联显示常使用紧边界而把它显示出来，但 LayMesh 按 Figure 原始边界导出时可能裁掉它。警告会指出超出的方向；LayMesh 不会自动改变图像尺寸或裁剪方式。需要保留这些内容时，可在 Python 绘图后调用 `fig.tight_layout()`，或创建 Figure 时使用 `layout="constrained"`，然后重新运行 Magic。

普通 Python 脚本也可以复用同一桥接层：

```python
from laymesh import render_source

result = render_source(
    r"""page=canvas(size=(100 mm,70 mm))
chart=image(src={{fig}})
page.add(chart,size=(80 mm, auto),target=page.center,anchor=center)
""",
    namespace={"fig": fig},
    output="figure.pdf",
)
print(result.output)
```

`render_file("layout.lay", namespace={...}, output="figure.pdf")` 对已有布局文件执行相同流程。两个函数都返回 SVG 预览文本、输出路径和可选的保存源码路径。

[完整示例 Notebook](../../examples/jupyter-integration.ipynb)包含折线图、热图与独立源码重渲染。

实际生成的素材类型、回退提示与两种 DPI 的区别见[Notebook 分类画廊](../gallery/notebook.zh-CN.md)。


## 限制与相关主题

[原生数组与 DataFrame](python-data.zh-CN.md) · [Notebook Magic](notebook.zh-CN.md) · [保存源码与独立重渲染](save-source.zh-CN.md)


## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
