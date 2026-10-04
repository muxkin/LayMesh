# 数据输入与缺失值

<!-- walkthrough:start -->
## 用途与概念

数据数值与物理尺寸不同：x/y 是轴范围中的无单位数，线宽和 marker_size 是物理长度。内联列表适合小样本，table 读取 CSV/列式 JSON，array 读取 JSON 向量或矩阵。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/table.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

同一个 table 的列可用于折线、误差棒与填充带。Python 绑定 DataFrame 会生成数据资源，保存源码后可由 CLI 独立读取。null 保留缺失信息，不会自动插值。

<!-- example:examples/plot/first-plot.lay -->

## 常见错误与限制

table 的列名非空且唯一，列长相同；array 不接受 values 参数，内联数据直接写列表。缺失点可能分断曲线或跳过标记，并产生 W_PLOT_MISSING。

## 逐项功能说明

### table

读取 CSV 或列式 JSON 表格，列名须非空且唯一，列长相同；d["列名"] 选择一列，null 或空 CSV 单元格保留缺失。

返回：table

必需输入：`src`.

[最小完整源码](../../examples/manual/table.lay) · [组合源码](../../examples/plot/first-plot.lay) · [全部参数](interface-reference.zh-CN.md#table)

### array

从 JSON 读取非空数值向量或矩阵。矩阵须为矩形，null 表示缺失；内联数组直接使用列表而非 array(values=...)。

返回：number[] | number[][]

必需输入：`src`.

[最小完整源码](../../examples/manual/array.lay) · [组合源码](../../examples/plot/complete-data.lay) · [全部参数](interface-reference.zh-CN.md#array)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `table(src=...)` | CSV/JSON 表格 | 列名访问 |
| `null` | 缺失值 | 保留诊断 |

### 常见用法



`table` 接受带唯一非空表头的 CSV，或按列组织的 JSON：`{"x":[0,1],"y":[2,3],"sample":["A","B"]}`。各列必须等长且非空；允许保留字符串列，但传给绘图方法的列必须为数值。CSV 支持引号、列名中的逗号和 UTF-8 BOM；空单元格视为缺失，文字 `NaN` 不会自动当作缺失。

`array` 接受一维或二维数值 JSON。外部 JSON 使用 `null` 表示缺失；不接受 JSON `NaN`、`Infinity`、复数或布尔数组。数据采用 JavaScript number；超出安全整数范围的整数会拒绝。相对路径依据**定义该数据的 `.lay` 文件**解析，可用于组件模块。文件按单次编译缓存，下次编译会重新读取修改后的内容。

缺失数据触发带源码位置及数量的 `W_PLOT_MISSING`：折线和填充带在缺失行断开，散点和误差棒跳过缺失行，热图缺失单元格透明。长度不匹配、无有效点、负误差、非法对数值、不规则矩阵和非有限范围报错。普通数据图层不排序或补点；统计方法按明确规则计算描述统计，不修改输入数组。

### 限制与相关主题

[绘图区与物理尺寸](plot-area.zh-CN.md) · [坐标轴与刻度](axes.zh-CN.md) · [标签与科学计数法](labels.zh-CN.md) · [图例与共享色标](legends.zh-CN.md) · [数据锚点与标注](annotations.zh-CN.md)
