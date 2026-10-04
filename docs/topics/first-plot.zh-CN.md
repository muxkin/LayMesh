# 第一张数据图

<!-- walkthrough:start -->
## 用途与概念

图表是原生矢量素材，而不是从其他绘图库导入的图片。先创建 plot，添加图层与图例，再将它放到页面；轴把数据数值映射到绘图区。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

从内联 x/y 列表改成 table 的列即可使用实验数据。保持显式轴范围可以在多个图表之间比较；使用 plot_area 保持相同物理数据区。

<!-- example:examples/plot/first-plot.lay -->

## 常见错误与限制

图表第一次被放置后封存，不能再添加图层。x/y 数量必须匹配；不要把 mm 写在数据数值上，物理样式和数据单位各有用途。

## 逐项功能说明

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `plot_area` | 固定数据区域 | 示例：(20,10,80,55) mm |
| `src` | CSV 路径 | 相对定义文件 |

### 数据与固定绘图区

示例读取同目录中的 [first-plot.csv](../../examples/plot/first-plot.csv)。在源码区切换 CSV 标签即可查看全部数据；保留字体和 CSV 的相对位置。

画布为 120 × 90 mm。图表放在页面 (5,5) mm，内部绘图区从 (20,10) mm 开始，因此页面中的绘图区左上角为 **(25,15) mm**，宽高为 **80 × 55 mm**。修改标签或移动图例不会改变它；空间不足时会警告。先完成图层与装饰，再执行 `page.add`。

检查 JSON 中的 `plot_area` 与 `page_transform` 应与这些尺寸对应。300 DPI 的 PNG 应为 **1417 × 1063 px**。

### 导出与检查

```sh
laymesh validate examples/plot/first-plot.lay
laymesh render examples/plot/first-plot.lay -o first-plot.svg
laymesh render examples/plot/first-plot.lay -o first-plot.pdf
laymesh render examples/plot/first-plot.lay -o first-plot.png --dpi 300
laymesh inspect examples/plot/first-plot.lay --json
```

### 限制与相关主题

[第一张版面](first-layout.zh-CN.md)
