# CLI 参数

## 操作流程

[main.rs](../../crates/laymesh-cli/src/main.rs) 中的命令行入口提供以下子命令：

```text
laymesh validate <file.lay|file.laydoc> [--warnings show|hide]
laymesh animate <file.lay|file.laydoc> -o <output.mp4|gif>
laymesh inspect <file.lay|file.laydoc> --json [--warnings show|hide]
laymesh render <file.lay|file.laydoc> -o <output.svg|pdf|pptx|png|jpg|tif|webp|bmp|gif|ico|pnm|pbm|pgm|ppm|pam|tga> [--dpi <positive-number>] [--warnings show|hide]
```

在当前 workspace 中以 `cargo run --release --locked -p laymesh-cli --` 为前缀。`-o` 与 `--output` 等价；扩展名决定格式；`--dpi` 用于所有位图输出、PDF 图片上限与 PPTX 局部图片回退，默认 1200；`--config PATH` 指定项目配置，`--pdf-*` 调节 PDF 压缩、精度和透明度保留。[完整参数与配置](export.zh-CN.md)。成功的 `validate` 打印尺寸与**顶层**实例数；成功的 `render` 打印输出路径。警告写 stderr，成功仍退出 0；源文件/渲染错误退出 1，参数用法错误退出 2。CLI 先渲染再经临时文件改名到目标路径，防止正常错误留下半成品。具体 stdout 见[执行记录](../examples-and-results.zh-CN.md)。

## 限制与相关主题

[dsl-reference](../language-reference.zh-CN.md) · [绘图对象与方法](plot-reference.zh-CN.md) · [Rust/WASM API](node-reference.zh-CN.md) · [Python API](python-reference.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## LSP / version / help

```sh
laymesh lsp --stdio
laymesh --version
laymesh --help
```

lsp 在标准输入/输出上运行持续的 JSON-RPC 服务，标准输出专供协议。--version/-V 与 --help/-h 是独立命令。--warnings show|hide 覆盖 LAYMESH_WARNINGS；inspect 即使隐藏显示也保留诊断数据。

位图格式包括 PNG、JPEG、TIFF、WebP、BMP、GIF、ICO、PNM 和 TGA；可设置质量、压缩、底色及 WebP 模式。完整选项、默认值及限制见[导出格式与编码参数](export.zh-CN.md)。

## 动画与多页

.laydoc 支持整本 PDF、单页及编号批量导出；非 PDF 需 --page 或 --all-pages。animate 使用连续时间线导出 MP4/GIF，需 FFmpeg。--time/--step 当前只输出 PNG 快照。详见[文档](documents.zh-CN.md)与[动画导出](animation-export.zh-CN.md)。
