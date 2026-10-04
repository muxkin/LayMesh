# Python API

## 操作流程

在 Notebook 所用的 Python 环境安装 `python -m pip install --pre laymesh`；NumPy/pandas 可选 `[data]`，Matplotlib Figure 导入可选 `[plot]`。首次 PyPI 上传前，按[安装说明](install.zh-CN.md)安装本地 wheel。仅使用已有 `.lay` 文件时可省略 `[plot]`。要求 Python 3.10+。桥接层调用现有 CLI，不重新实现 `.lay` 解析器。CLI 查找顺序：`LAYMESH_CLI`、wheel 内原生程序、仓库 `target/release/laymesh` 或 `target/debug/laymesh`，最后是 `PATH` 中的 `laymesh`。不支持 JavaScript CLI 回退。

```python
from laymesh import render_source, render_file, RenderResult, LayMeshBridgeError

result = render_source(
    r"""page=canvas(size=(20 mm,10 mm))
box=rect(size=(5 mm, 5 mm),fill="#087f8c")
page.add(box,target=page.top_left)
"""
)
print(result.preview_svg.startswith('<svg'), result.output, result.saved_source)
# True None None
```

| 公开函数或类 | 参数 | 行为 |
| --- | --- | --- |
| `render_source(source, *, namespace=None, base_dir=None, output=None, dpi=None, plot_dpi=300, save_source=None, show_warnings=None)` | `source` 是 `.lay` 文本；`namespace` 给 `{{name}}` 变量；`base_dir` 为相对素材与组件目录；`output` 是 `.svg/.pdf/.png` 路径；`dpi` 只用于 PNG；`plot_dpi` 只用于 Matplotlib 回退 PNG；`save_source` 是 `base_dir` 中生成的 `.lay` 文件 | 返回 SVG 预览，并可选导出文件与保存展开后的源码。临时 `.lay` 会清理；显式保存的素材放在同名 `.assets/` 目录。 |
| `render_file(file, *, namespace=None, output=None, dpi=None, plot_dpi=300, save_source=None, show_warnings=None)` | 读取现有 `.lay`；相对素材路径按该文件所在目录解析 | 复用同一展开和渲染流程，原文件不修改。 |
| `RenderResult` | `preview_svg: str`、`output: Path\|None`、`saved_source: Path\|None` | 不可变 dataclass；无 `output` 时仍有 SVG 预览。 |
| `LayMeshBridgeError` | 异常类 | 变量、素材、CLI 启动或 CLI 返回错误时抛出，适合在 Notebook 中展示。 |

`namespace` 支持字符串、`Path`、布尔、有限数字、NumPy 标量、Matplotlib `Figure`，以及数值 list/tuple、一维或二维 ndarray、Series 和 DataFrame；占位符必须是引号外的单个 `{{name}}`，不执行 Python 表达式。数组和表格可直接供[原生图表](../plotting.zh-CN.md)使用，并保存为可重复读取的 JSON 资源。Matplotlib 图优先尝试安全 SVG，超出支持子集时警告并回退 `plot_dpi` PNG。重复 `save_source` 只覆盖带 LayMesh 生成标记的文件，避免覆盖手写布局。详情与 Notebook 示例见[Python/Jupyter 指南](../python-jupyter.zh-CN.md)。

`%load_ext laymesh.ipython` 调用 `load_ipython_extension(ipython)` 注册 `LayMeshMagics`；随后使用：

```text
%laymesh figures/layout.lay -o figures/output.png --dpi 300
%%laymesh -o figure.pdf --save-source figure.lay
page = canvas(size=(100 mm, 70 mm))
```

两种 Magic 都将 SVG 预览显示在单元格中。行 Magic 读取文件，单元格 Magic 读取正文；均支持 `-o/--output`、`--dpi`、`--plot-dpi`、`--save-source`。`--plot-dpi` 默认 300，只影响 Figure 回退图像。`load_ipython_extension` 位于 [ipython.py](../../python/laymesh/ipython.py)，不在 `laymesh` 顶层导出。

`show_warnings=False` 隐藏显示的警告，`True` 显示，`None` 跟随 `LAYMESH_WARNINGS`（默认 `show`）。两种 Notebook Magic 均接受 `--warnings show|hide`。源码开发在编译 Rust CLI 后使用可编辑安装，见[安装说明](install.zh-CN.md)。

## 限制与相关主题

[dsl-reference](../language-reference.zh-CN.md) · [绘图对象与方法](plot-reference.zh-CN.md) · [Rust/WASM API](node-reference.zh-CN.md) · [CLI 参数](cli-reference.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
