# 第一张版面

<!-- walkthrough:start -->
## 用途与概念

从一个页面、两个卡片和标题开始，观察绝对偏移与相对定位的差异。页面大小采用物理长度，坐标原点在左上，x 向右、y 向下。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/add.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

使用 target=first.bottom_right 和 offset=(3mm,3mm) 放置第二个实例。标题用 anchor=center 与卡片 center 对齐，因此字体变化时仍保持中心关系。

<!-- example:examples/hello.lay -->

## 常见错误与限制

add 的默认 anchor 是 top_left，offset 沿容器方向。页面不会自动增大，超出页面的对象可能在导出时被裁切。

## 逐项功能说明

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `size` | 画布物理尺寸 | 示例：160 × 100 mm |
| `anchor` | 实例定位点 | top_left |

### 素材与实例

`card` 定义可复用的图形，两个 `page.add(card, ...)` 创建两个实例。`left.center` 用于把标题放在第一个实例中心。完整源码可直接在源码标签中查看和复制。

### 导出与检查

```sh
laymesh validate examples/hello.lay
laymesh render examples/hello.lay -o hello.svg
laymesh render examples/hello.lay -o hello.pdf
laymesh render examples/hello.lay -o hello.png --dpi 300
```

### 限制与相关主题

[第一张数据图](first-plot.zh-CN.md)
