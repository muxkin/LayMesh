# 函数与模块

<!-- walkthrough:start -->
## 用途与概念

function 把计算或素材组合封装为可复用逻辑；组件 .lay 文件用 export 暴露变量和函数，入口文件用 import 选择名称并可重命名。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/group.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

函数可接受尺寸、标题或端帽变量并返回素材或组。预定义变量在模块中同样可用，用户或模块自己的同名变量优先；模块中的资源路径相对于模块文件。

<!-- example:examples/gallery/containers/module-import.lay -->

## 常见错误与限制

导入名只读，循环导入与缺失导出报错。模块不创建或直接绘制入口 canvas；主文件应先建立 canvas 再使用导入组件。

## 逐项功能说明

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `import` | 导入本地组件 | 路径相对导入文件 |
| `function` | 定义可复用函数 | 组件模块不创建入口画布 |

### 常见用法

模块路径只允许本地相对 `.lay` 文件。模块用 `export` 导出常量、素材、组或函数；可继续导入其他模块，导入名只读。入口画布须在使用导入值前建立，组件模块不能创建或直接绘制入口画布。缺失导出、循环导入均报错。完整例子见[scripted.lay](../../examples/scripted.lay)与[card.lay](../../examples/components/card.lay)。

### 限制与相关主题

[变量与表达式](values.zh-CN.md) · [条件与循环](control-flow.zh-CN.md)


使用[库作者文档注释](library-docs.zh-CN.md)为可复用组件提供函数、参数和导出变量说明。

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
