# 变量与表达式

<!-- walkthrough:start -->
## 用途与概念

变量可以保存数值、字符串、列表、素材和实例。固定选项使用预定义字符串变量，无需声明：round 与 "round" 完全等价。它们不是宏，没有参数展开或特殊赋值规则。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/str.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

把 [triangle,open,dot] 放入列表并循环生成头部，或把 round 传入函数默认参数。用户作用域优先，因此 round="square" 会改变该作用域中 round 的值。ratex_katex 对应字符串 "ratex-katex"。

<!-- example:examples/gallery/scripting/units-builtins.lay -->

## 常见错误与限制

字符串写法继续合法且没有弃用警告。未知名称仍报 E_NAME。内置选项变量在悬停中只显示类型和值；允许哪些值以及它们的含义由对应参数说明负责。

## 逐项功能说明

### range

生成不包含 stop 的有限整数列表；一个参数为 stop，两个为 start/stop，三个增加 step。step 非零，最多 10,000 项。

返回：integer[]

[最小完整源码](../../examples/manual/range.lay) · [组合源码](../../examples/functions.lay) · [全部参数](interface-reference.zh-CN.md#range)

### len

返回列表、字典、几何集合的项数或字符串长度。

返回：integer

[最小完整源码](../../examples/manual/len.lay) · [组合源码](../../examples/gallery/scripting/units-builtins.lay) · [全部参数](interface-reference.zh-CN.md#len)

### append

返回加入一个新项的列表副本，不原地修改输入列表。需要累积结果时将返回值重新赋给变量。

返回：list

[最小完整源码](../../examples/manual/append.lay) · [组合源码](../../examples/gallery/scripting/units-builtins.lay) · [全部参数](interface-reference.zh-CN.md#append)

### str

将支持的数值、布尔值或字符串转换为文字，长度包含单位。可拼接为标签，不执行任意格式表达式。

返回：string

[最小完整源码](../../examples/manual/str.lay) · [组合源码](../../examples/gallery/scripting/units-builtins.lay) · [全部参数](interface-reference.zh-CN.md#str)

### abs

返回绝对值并保留数值的单位类型。适用于标量或物理长度，不接受素材与列表。

返回：number | length

[最小完整源码](../../examples/manual/abs.lay) · [组合源码](../../examples/gallery/scripting/units-builtins.lay) · [全部参数](interface-reference.zh-CN.md#abs)

### min

返回一个或多个兼容单位数值中的最小值。所有参与值须使用兼容类型，返回值保留单位。

返回：number | length

[最小完整源码](../../examples/manual/min.lay) · [组合源码](../../examples/gallery/scripting/units-builtins.lay) · [全部参数](interface-reference.zh-CN.md#min)

### max

返回一个或多个兼容单位数值中的最大值。所有参与值须使用兼容类型，返回值保留单位。

返回：number | length

[最小完整源码](../../examples/manual/max.lay) · [组合源码](../../examples/gallery/scripting/units-builtins.lay) · [全部参数](interface-reference.zh-CN.md#max)

### dict

创建空字典，或从 JSON 文件加载有序嵌套字典；不放宽 table 的列校验。

返回：dict

[最小完整源码](../../examples/manual/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#dict)

### dict-keys

返回按插入顺序排列的键列表。

返回：list

[最小完整源码](../../examples/manual/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#dict-keys)

### dict-values

返回按插入顺序排列的值列表；保留单位和对象类型。

返回：list

[最小完整源码](../../examples/manual/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#dict-values)

### dict-items

返回按插入顺序排列的键值对列表，可用于循环解包。

返回：list

[最小完整源码](../../examples/manual/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#dict-items)

### dict-get

按字符串键取值；缺失时返回 default，默认 null。

返回：value

必需输入：`key`.

[最小完整源码](../../examples/manual/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#dict-get)

### dict-update

返回合并后的新字典；覆盖键保留位置，新键追加到末尾；原字典不变。

返回：dict

必需输入：`other`.

[最小完整源码](../../examples/manual/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#dict-update)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `mm / cm / in / pt / px` | 长度单位 | 长度参数需要单位 |
| `range(...)` | 生成循环序列 | 有限求值 |

### 常见用法

变量保存值或素材定义。长度必须带单位；列表、元组、索引和内置运算在受限 DSL 中执行，不能执行任意 Python 或 JavaScript。

[完整参数与规则](../language-reference.zh-CN.md)

### 限制与相关主题

[条件与循环](control-flow.zh-CN.md) · [函数与模块](modules.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## 字符串等价与局部覆盖

[可运行对照](../../examples/manual/variables-strings.lay) 分别用 triangle 与 "triangle" 绘制相同头部，再用用户绑定 round="square" 改变端帽。列表、比较与作用域均采用普通字符串规则。

## 有序字典与遍历

使用字符串键，例如 `{ "B": value, "A": other }`；键可由表达式生成，值可保留长度单位、颜色、素材或几何查询。字典按插入顺序排列，重复键覆盖值且保留原位置。`d["key"]` 缺失时报 E_INDEX，`d.get("key", default=null)` 提供默认值。`len(d)`、`.keys()`、`.values()`、`.items()` 同样适用于表格列。

字典采用值语义：执行 `b=a` 后修改 b，不改变 a。`d["panel"]["color"]="#0072b2"` 等嵌套赋值重建并重新绑定根字典，中间键须存在，导入绑定仍不可修改。`.update(other)` 返回新字典，使用 `d=d.update(other)`。字典内的素材对象保留原有句柄行为。点号用于调用方法，名为 items 等的键通过索引访问。

`dict()` 创建空字典；`dict(src="config.json")` 加载嵌套 JSON 对象并保持对象顺序，数字须有限且能精确表示。table 和 array 继续执行原有的严格数据校验。
