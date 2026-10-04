# CLI 参数

## 操作流程

[main.ts](../../crates/laymesh-cli/src/main.rs) 中的命令行入口提供三个子命令：

```text
laymesh validate <file.lay> [--warnings show|hide]
laymesh inspect <file.lay> --json [--warnings show|hide]
laymesh render <file.lay> -o <output.svg|pdf|png> [--dpi <positive-number>] [--warnings show|hide]
```

在当前 workspace 中以 `cargo run --release --locked -p laymesh-cli --` 为前缀。`-o` 与 `--output` 等价；扩展名决定格式；`--dpi` 仅能搭配 PNG。成功的 `validate` 打印尺寸与**顶层**实例数；成功的 `render` 打印输出路径。警告写 stderr，成功仍退出 0；源文件/渲染错误退出 1，参数用法错误退出 2。CLI 先渲染再经临时文件改名到目标路径，防止正常错误留下半成品。具体 stdout 见[执行记录](../examples-and-results.zh-CN.md)。

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
