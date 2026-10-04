# 原生数组与 DataFrame

把 NumPy 或 pandas 数据传给原生图层；这条路线无需 Matplotlib。

平台 wheel 内置一个 Rust 原生程序，无需额外安装运行时。首次 PyPI 发布前，可安装已审核的本地 wheel；正式发布后使用下列命令。原生数据不依赖 Matplotlib，导入 Figure 时增加 `plot` 额外依赖。

```sh
python -m pip install --pre "laymesh[data]"
```

[安装与源码开发](install.zh-CN.md)

## 操作流程

先在 Python 单元格中计算数据：

```python
import numpy as np
import pandas as pd

x = np.linspace(0, 2 * np.pi, 33)
df = pd.DataFrame({"x": x, "y": np.sin(x)})
```

加载一次扩展：

```text
%load_ext laymesh.ipython
```

在另一个单元格中直接将数据交给原生图表：

```lay
%%laymesh -o native.pdf --save-source native.lay
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
d = {{df}}
p = plot(size=(110 mm, 80 mm), plot_area=box(offset=(20 mm, 10 mm), size=(80 mm, 55 mm)),
         x=axis(label="x", range=(0, 7)), y=axis(label="sin(x)", range=(-1.2, 1.2)),
         style=plot_style(font_family="DejaVu Sans", font_size=8 pt))
p.line(x=d["x"], y=d["y"], color="#0072B2")
page.add(p, offset=(5 mm, 5 mm))
```

坐标轴、标签和折线均由 LayMesh 生成，不创建 Figure。`DejaVu Sans` 是系统字体名，跨机器复现时建议提供字体文件。支持数值 list/tuple、一维／二维 ndarray、Series 和 DataFrame；DataFrame 列名必须为唯一非空字符串，索引不会自动导出。表格通过 `d["列名"]` 访问，数组绑定可直接传给图层的 `y`、`z` 等参数。

`None`、NaN 和 pandas NA 会转成 JSON `null`：折线在缺失处断开，散点跳过，热图单元格透明，并保留诊断。无穷值和不支持的类型报错。绑定只接受未加引号的单个变量名，不接受表达式；计算留在 Python 中。

## 保存与独立运行

使用 `--save-source` 保存展开后的布局和数据资源。完整 Python 脚本与独立 CLI 重渲染步骤见[保存源码与独立重渲染](save-source.zh-CN.md)。

## 限制与相关主题

[Notebook Magic](notebook.zh-CN.md) · [保存源码与独立重渲染](save-source.zh-CN.md) · [Matplotlib 导入](matplotlib.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## Python 字典绑定

使用 `series={{series}}` 绑定 Mapping，然后在 `.lay` 中写 `for name,ys in series.items()`。字符串键顺序和空字典、嵌套字典均保留；值支持 JSON 容器、字符串、布尔、数字、null 及一维或二维 NumPy 数组。NaN 转为 null，无穷数、超大整数、非字符串键、循环引用和任意对象在渲染前报错，并指出字段路径。`--save-source` 保存字典 JSON 资源后，可脱离 Python 命名空间重放。

[可运行的 Python 字典绑定示例](../../examples/plot/dictionary-series.py)。
