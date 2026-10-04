# Notebook Magic

平台 wheel 内置 Rust 引擎。默认安装包含 NumPy、pandas、Matplotlib 和 IPython，数据绑定、Figure 导入及 Notebook Magic 均可直接使用。

```sh
python -m pip install laymesh
```

[安装与源码开发](install.zh-CN.md)

## 操作流程

`%load_ext laymesh.ipython` 调用 `load_ipython_extension(ipython)` 注册 `LayMeshMagics`；随后使用：

```text
%laymesh figures/layout.lay -o figures/output.png --dpi 300
%%laymesh -o figure.pdf --save-source figure.lay
page = canvas(size=(100 mm, 70 mm))
```

两种 Magic 都将 SVG 预览显示在单元格中。行 Magic 读取文件，单元格 Magic 读取正文；均支持 `-o/--output`、`--dpi`、`--plot-dpi`、`--save-source`。`--plot-dpi` 默认 300，只影响 Figure 回退图像。`load_ipython_extension` 位于 [ipython.py](../../python/laymesh/ipython.py)，不在 `laymesh` 顶层导出。

## 限制与相关主题

[原生数组与 DataFrame](python-data.zh-CN.md) · [保存源码与独立重渲染](save-source.zh-CN.md) · [Matplotlib 导入](matplotlib.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## 完整的双单元格示例

```python
%load_ext laymesh.ipython
values = [1, 2, 4, 3]
```

```text
%%laymesh -o figure.svg --save-source figure.lay
page=canvas(size=(100mm,75mm))
p=plot(size=(86mm,62mm))
p.line(x=[0,1,2,3],y={{values}},marker=circle)
page.add(p,offset=(7mm,6mm))
```

先运行注册单元格。单元格 magic 读取 Notebook 命名空间，展开 {{values}}，生成预览并保存可独立运行的源码和资源。占位符是不加引号的名称，不执行任意 Python 表达式。再次执行会更新自动生成文件；普通手工源码受覆盖保护。

`render_source` 和 `render_file` 还接受 `quality=None`、`compression=None`、`background=None`、`webp_lossless=None`、`webp_method=None`、`webp_alpha_quality=None`、`webp_near_lossless=None`。Magics 接受相应的 `--quality`、`--compression`、`--background` 与 `--webp-*` 参数。[导出格式与编码参数](export.zh-CN.md)
