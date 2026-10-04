# 检查与警告

先验证输入，再检查布局与映射。隐藏警告只改变显示，不删除诊断或屏蔽错误。

## 操作流程

## 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `--warnings` | 显示或隐藏警告 | show |
| `LAYMESH_WARNINGS` | 环境默认值 | show / hide |
| `show_warnings` | Python 覆盖参数 | None |

```sh
export LAYMESH_WARNINGS=hide
laymesh render figure.lay -o figure.pdf --warnings show
laymesh validate figure.lay --warnings hide
laymesh inspect figure.lay --json --warnings hide
```

```python
from laymesh import render_file
result = render_file("figure.lay", output="figure.pdf", show_warnings=False)
# Notebook: %laymesh figure.lay --warnings hide
```

优先级：显式参数/CLI 选项 → 环境变量 → 默认显示。环境变量仅接受 show/hide；Python 参数接受 None/True/False。开关覆盖 LayMesh 的字体回退、缺失数据、负半径和排版警告；不修改 Python 的全局警告过滤器。一次 Python 导出及预览的相同警告只显示一次。

隐藏不丢弃诊断，不改变图像和数据映射；`inspect --json` 仍返回全部警告。语法、参数、范围和导出错误仍报错并失败。检查结果含绘图区、投影圆心、内外半径、角范围、方向、径向/雷达范围、裁剪路径、装饰边界以及页面变换。

Python 数组与缺失数据示例：[polar-data.py](../../examples/plot/polar-data.py)；运行 `PYTHONPATH=python python examples/plot/polar-data.py`，保存可独立编辑的 .lay 和哈希 JSON。验收与性能见[本轮实测](../benchmarks/polar-plots.zh-CN.md)。

## 限制与相关主题

[CLI 使用](cli.zh-CN.md) · [导出格式与分辨率](export.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
