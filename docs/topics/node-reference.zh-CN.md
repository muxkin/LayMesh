# Rust 与浏览器 WASM 接口

核心已完整迁入 Cargo workspace，不需要 Node.js 或 npm。原 Node/TypeScript API 由 Rust 库和浏览器 WASM 入口接替。

```rust
use laymesh_core::engine::compile_file;
let scene = compile_file("figure.lay")?;
let svg = laymesh_render::render_svg(&scene)?;
```

内存源码使用 `compile_source(source, filename, Host)`，资源通过 `Host.files` 提供。原生 CLI 使用系统字体；WASM 仅使用用户加载的字体。浏览器绑定提供 `render(source, filename, files_json)` 和 `language_query(files_json, filename, method, offset, locale)`，使用同一解析、布局、字体与 SVG 实现；偏移量为 UTF-16。

Python/Jupyter 公共接口保持不变，调用本地原生程序；`LAYMESH_CLI` 可指定另一原生构建。

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## 从 Node 调用原生 CLI

旧 TypeScript 包已经移除。Node 可通过标准库调用同一个原生程序，此示例不依赖 npm 包。命令参数单独传递，支持含空格的路径；资源仍相对 .lay 文件解析。

```js
// Node uses the existing native CLI; there is no separate Node layout engine.
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const binary=process.env.LAYMESH_CLI||fileURLToPath(new URL('../../target/release/laymesh',import.meta.url));
const source=process.argv[2]||fileURLToPath(new URL('../manual/line.lay',import.meta.url));
const output=process.argv[3]||'figure.svg';
execFileSync(binary,['validate',source],{stdio:'inherit'});
execFileSync(binary,['render',source,'-o',output],{stdio:'inherit'});
```

```sh
node examples/integration/render.mjs examples/manual/line.lay /tmp/figure.svg
```

应用应处理子进程非零退出状态；LAYMESH_CLI 可选择其他原生构建。浏览器代码使用生成的浏览器 WASM 绑定，Node 程序不能依赖浏览器的字体加载流程。

## 浏览器 WASM 调用流程

```js
import init, {render, language_query} from "./wasm/laymesh_wasm.js";
await init();
const source = 'page=canvas(size=(60mm,40mm))\npage.add(rect(size=(30mm,20mm),fill="#087f8c"))';
const filename = "/figure.lay";
const result = JSON.parse(render(source, filename, "{}"));
console.log(result.svg, result.inspection, result.warnings);
const help = JSON.parse(language_query(
    JSON.stringify({[filename]: source}), filename, "hover", 7, "en"
));
console.log(help);
```

render 返回 JSON，并非单独 SVG 字符串；资源参数是资源名到文本或字节数组的 JSON 映射。渲染文字前需注册实际字体字节；需要分开计时可用 prepare_render 返回的 job.svg()/inspection()。language_query 使用 UTF-16 偏移，静态分析不执行源码；失败时抛出包含诊断的 JS 异常。
