# Rust 与浏览器 WASM 接口

核心已完整迁入 Cargo workspace，不需要 Node.js 或 npm。原 Node/TypeScript API 由 Rust 库和浏览器 WASM 入口接替。

```rust
use laymesh_core::engine::compile_file;
let scene = compile_file("figure.lay")?;
let svg = laymesh_render::render_svg(&scene)?;
```

内存源码使用 `compile_source(source, filename, Host)`，资源通过 `Host.files` 提供。原生 CLI 使用系统字体；WASM 使用宿主显式注册的字体，文档网站会注册随网页提供的正文字体。浏览器绑定提供 `render(source, filename, files_json)` 和 `language_query(files_json, filename, method, offset, locale)`，使用同一解析、布局、字体与 SVG 实现；偏移量为 UTF-16。

Python/Jupyter 公共接口保持不变，调用本地原生程序；`LAYMESH_CLI` 可指定另一原生构建。

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## 最小 Rust 源码

```rust
use laymesh_core::{engine::compile_source, model::Host};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = "page=canvas(size=(60mm,40mm))\npage.add(rect(size=(30mm,20mm),fill=\"#087f8c\"))";
    let scene = compile_source(source, "/figure.lay", Host::default())?;
    std::fs::write("figure.svg", laymesh_render::render_svg(&scene)?)?;
    std::fs::write("figure.pdf", laymesh_render::render_pdf(&scene)?)?;
    std::fs::write("figure.png", laymesh_render::render_png(&scene, 300.)?)?;
    Ok(())
}
```

把此代码放入依赖本工作区 laymesh-core 和 laymesh-render 的 Cargo 二进制程序。Host.files 将规范化绝对资源名映射到字节；Host::default() 开启原生资源及字体加载。WASM 使用 native=false，并显式提供资源。

## 交互绘制与动画

浏览器实时预览使用 prepare_render 创建可复用任务，通过 draw_job 得到绘制任务。绘制任务提供 display_list 与 resource_bytes；时间采样得到对应动画帧，CanvasKit 绘制这些指令。render 的 SVG 返回值仍可用于静态集成；不能把它误认为全部浏览器绘制能力。宿主提供资源和字体，缺失资源通过实际编译请求补齐。语言服务偏移使用 UTF-16。

参见 [worker 源码](../../site/live/worker.mjs)及[动画实现](animation-internals.zh-CN.md)。

## Object surfaces

显示列表版本 3 携带 `hint.surface3d`、倒影与柔边参数；读取端兼容版本 1–3。共享浏览器渲染器调用 WASM 的 `surface_texture_region(mesh_json, region, density)` 选择纹理区域，再调用 `raster_surface3d(mesh_json, rgba, texture_width, texture_height, region, width, height)` 统一处理几何、遮挡和光照。内联 worker 宿主在 WASM 初始化后通过 `setSurfaceRasterizer` 注入这两个函数。升级时应同步编译器、播放、编辑器和导出运行时。同步 `RenderJob.svg()` 接口也会嵌入局部三维 PNG；交互预览使用 CanvasKit 提供二维纹理与页面合成。
