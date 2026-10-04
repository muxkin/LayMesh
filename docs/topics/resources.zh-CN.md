# 资源路径

<!-- walkthrough:start -->
## 用途与概念

资源路径相对于定义资源的文件，而不是运行命令时的工作目录。素材定义、模块、独立 LCSS 各保留自己的资源来源；CLI 使用入口路径组织这些依赖。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/image.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

将图片、字体和组件放在作品旁，使用相对路径。Python save_source 把绑定数据和图像写进同名 .assets 目录；分享时保留源文件与资源的相对位置。

<!-- example:examples/gallery/containers/module-import.lay -->

## 常见错误与限制

WASM 的虚拟文件映射需要提供完整依赖；不能依靠浏览器任意读取本机文件。资源加载错误会带定义文件的位置，字体名称的解析依赖运行环境。

## 逐项功能说明

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `src` | 图片或数据路径 | 相对定义文件 |
| `font_family` | 字体名称或文件 | 可选；受系统字体影响 |

### 目录关系

```text
examples/
├── assets/
│   └── Custom.ttf
└── plot/
    ├── figure.lay
    └── results.csv
```

`font_family="../assets/Custom.ttf"` 从 `plot/figure.lay` 指向上一级的字体目录；`table(src="results.csv")` 指向同目录的数据。assets 可以改名，只要同步修改相对路径。组件内部的路径相对于组件文件，而不是入口文件。

若系统已安装 DejaVu Sans，可用 `font_family="DejaVu Sans"`；上述路径用于用户提供的自定义字体。程序不携带正文字体，找不到字体时警告并画方框。

### 限制与相关主题

[图像导入与裁剪](images.zh-CN.md) · [文字与字体](fonts.zh-CN.md) · [数学公式](formulas.zh-CN.md) · [形状与路径](shapes.zh-CN.md) · [填充渐变与边框](fills.zh-CN.md)
