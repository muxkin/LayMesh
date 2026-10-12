# CLI 使用

## 操作流程

按[安装说明](install.zh-CN.md)安装平台 wheel。运行 `python -m pip install laymesh` 即可安装全部功能，也可安装本地 wheel。将[安装页](install.zh-CN.md)的完整示例保存为 `figure.lay` 后运行：

```sh
laymesh validate figure.lay
laymesh render figure.lay -o figure.pdf
```

CLI 提供验证、检查、静态导出、动画导出及语言服务：

```text
laymesh validate <file.lay|file.laydoc>
laymesh animate <file.lay|file.laydoc> -o <output.mp4|gif>
laymesh inspect <file.lay|file.laydoc> --json
laymesh render <file.lay|file.laydoc> -o <output.svg|pdf|pptx|png|jpg|tif|webp|bmp|gif|ico|pnm|pbm|pgm|ppm|pam|tga> [--dpi <number>]
```

`python -m laymesh` 与 `laymesh` 等价。源码开发时先运行 `cargo build --release --locked -p laymesh-cli`，再调用 `target/release/laymesh`（Windows 为 `laymesh.exe`），或用 `cargo run --release --locked -p laymesh-cli --` 作为命令开头。`validate` 只检查源文件和素材；`render` 在检查通过后生成文件。出错时命令返回非零状态，并给出文件、行、列及原因。

`inspect --json` 输出绘图区、坐标轴映射、变换和警告。validate、inspect 和静态 render 支持 `--warnings show|hide`；显式选项优先于环境变量 `LAYMESH_WARNINGS`，默认 `show`。隐藏警告不会隐藏错误，也不会删除检查 JSON 中的诊断。

## 限制与相关主题

[导出格式与分辨率](export.zh-CN.md) · [检查与警告](diagnostics.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## 动画与多页

.laydoc 支持整本 PDF、单页及编号批量导出；非 PDF 需 --page 或 --all-pages。animate 使用连续时间线导出 MP4/GIF，需 FFmpeg。--time/--step 当前只输出 PNG 快照。详见[文档](documents.zh-CN.md)与[动画导出](animation-export.zh-CN.md)。
