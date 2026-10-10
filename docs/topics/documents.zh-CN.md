# 多页文档

`.lay` 描述独立页面，`.laydoc` 使用同一种 Lay 语法编排页面。每个页面保留自己的 `canvas`，仍可单独预览和导出。

## document

```lay
doc = document(name="研究汇报")
```

一个编排入口创建一个文档。`name` 默认为 `Untitled`；`doc.pages` 返回按加入顺序排列的页面引用列表。

## page

```lay
cover = page(src="./pages/cover.lay", id="cover", name="封面")
```

`src` 是相对于调用所在文件的本地 `.lay` 路径，包括导入模块或函数所在文件。`id` 是文档内唯一的稳定标识，默认为文件名去掉后缀；`name` 默认为 `id`。可读取 `cover.id`、`cover.name`、`cover.src`。

页面引用按需加载。各页的变量、单位、布局 DPI 和样式独立；页面内的资源仍相对于原 `.lay` 文件。完整文档编译时，同一源文件的重复引用共用编译结果，各引用使用不同的 `id`。

## document.add

```lay
pages = [cover, page(src="./pages/results.lay", id="results")]
for p in pages {
    doc.add(p)
}
```

执行 `doc.add` 的顺序决定页序。可以使用普通 Lay 函数、条件、循环及 `import` 管理编排。移动列表元素即可调序，增删元素即可增删页。文档至少有一页，最多 10,000 页；重复 `id`、缺失文件、页面编译错误均明确报错。切换动画尚未实现，传入 `transition` 会报错。

## 导出与预览

```sh
laymesh validate examples/document/report.laydoc
laymesh inspect examples/document/report.laydoc --json
laymesh render examples/document/report.laydoc -o report.pdf
laymesh render examples/document/report.laydoc -o results.png --page 2
laymesh render examples/document/report.laydoc -o page.png --all-pages
```

页码从 1 开始。PDF 默认输出所有页，各页保留自己的物理尺寸和矢量内容。其它格式必须指定 `--page N` 或 `--all-pages`；批量输出为 `page-001.png`、`page-002.png` 等。整本 PPTX 尚未实现；单页和编号批量 PPTX 保留现有可编辑对象及回退诊断。

VS Code 打开 `.laydoc` 后可选择页面、打开对应页面源码、导出当前页或整本 PDF。网页使用相同 Rust/WASM 编排接口，按选中页绘制。预览中的选中页使用稳定 `id`，调整页序后继续显示该页；当前页被删除时明确报错。

```python
from laymesh import render_document

result = render_document("examples/document/report.laydoc", output="report.pdf")
result = render_document("examples/document/report.laydoc", output="results.png", page=2)
```

Python 返回包含 `output`、`page` 和 `preview_jpeg` 的 `DocumentResult`。Notebook 可直接显示结果，或运行 `%laymesh report.laydoc --page 2`；预览继续使用 Rust CPU 的 192 DPI、质量 95、4:4:4 JPEG。`preview_page=2` 只选择预览页，保留整本 PDF 导出。

完整示例：[report.laydoc](../../examples/document/report.laydoc)。
