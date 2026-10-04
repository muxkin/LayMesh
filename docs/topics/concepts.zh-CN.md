# 基本概念

<!-- walkthrough:start -->
## 用途与概念

先建立页面，再定义素材，最后用 add 建立实例。素材保存可复用的内容和默认样式；实例保存本次尺寸、变换和位置。仅定义素材不会产生绘制，实例的锚点才具有容器坐标。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/canvas.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

同一素材可放置多次。保存 add 的返回值，后续实例就能通过它的 bounds 或 path 建立相对定位。改变素材的默认尺寸不会追溯修改已完成的放置。

<!-- example:examples/hello.lay -->

## 常见错误与限制

不要把 rect(...) 的结果当作页面实例查询 top_left；先使用 placed=page.add(material)。入口只允许一个 canvas，模块负责提供素材和函数。

## 逐项功能说明

### canvas

创建页面并设置页面物理尺寸、背景和默认几何单位。通过 add 放置素材；不带单位的几何长度默认使用 mm，字号和线宽默认使用 pt。

返回：用于接收素材的页面。

必需输入：`size`.

[最小完整源码](../../examples/manual/canvas.lay) · [组合源码](../../examples/hello.lay) · [全部参数](interface-reference.zh-CN.md#canvas)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `canvas` | 创建入口画布 | 单一入口画布 |
| `add` | 创建放置实例 | 按添加顺序绘制 |

### 常见用法

素材定义可以复用；add 创建放置实例。画布是页面，组是局部坐标容器，图表是可放置的素材。只有需要后续引用的实例才必须命名。

[完整参数与规则](../language-reference.zh-CN.md)

### 限制与相关主题

[安装与验证](install.zh-CN.md)
