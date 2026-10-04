# 保存源码与独立重渲染

保存展开后的 .lay 和它的资源目录，便可脱离 Notebook 与 Python 重渲染。

## 准备环境

```sh
python -m pip install laymesh
```

## 保存布局和数据

```python
from laymesh import render_source
import numpy as np
import pandas as pd

x = np.linspace(0, 2 * np.pi, 33)
df = pd.DataFrame({"x": x, "y": np.sin(x)})
source = r"""
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
d = {{df}}
p = plot(size=(110 mm, 80 mm), plot_area=box(offset=(20 mm, 10 mm), size=(80 mm, 55 mm)),
         x=axis(label="x", range=(0, 7)), y=axis(label="sin(x)", range=(-1.2, 1.2)),
         style=plot_style(font_family="DejaVu Sans", font_size=8 pt))
p.line(x=d["x"], y=d["y"], color="#0072B2")
page.add(p, offset=(5 mm, 5 mm))
"""
result = render_source(source, namespace={"df": df},
                       output="native.pdf", save_source="native.lay")
print(result.output)
```

系统字体名适合快速开始；跨机器复现建议改用随项目保存的字体文件。`native.assets/` 中的 JSON 使用内容哈希命名。

## 独立重渲染

```sh
laymesh render native.lay -o native.svg
laymesh render native.lay -o native.png --dpi 300
laymesh inspect native.lay --json
```

保留 `.lay`、`.assets/` 和字体的相对位置。以上命令只调用 Rust CLI，不运行 Python。不要只分享 .lay 而遗漏其资源。

[Python/Jupyter](../sections/integration.zh-CN.md) · [Matplotlib](matplotlib.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
