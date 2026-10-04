# 条件与循环

<!-- walkthrough:start -->
## 用途与概念

for 按列表顺序或字典插入顺序遍历。支持键值和嵌套解包；enumerate 提供编号，zip 在最短输入结束时停止。循环开始时固定遍历内容。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/range.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

把图形尺寸、颜色或偏移保存到列表，用 range(len(items)) 遍历并按索引放置。while、break 和 continue 可以控制循环，但应让退出条件明确且可复现。

<!-- example:examples/gallery/scripting/control-flow.lay -->

## 常见错误与限制

循环最多 10,000 次，函数调用深度最多 64，语句总数有限制。新局部变量不会自动泄漏到外层；对已有外层变量的重新赋值遵循当前作用域规则。

## 逐项功能说明

### enumerate

生成索引和值的列表，支持循环解包；start 默认 0，最多 10,000 项。

返回：list

必需输入：`seq`.

[最小完整源码](../../examples/gallery/scripting/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#enumerate)

### zip

将位置参数中的可迭代集合逐项组合，在最短输入结束时停止；零参数返回空列表。

返回：list

[最小完整源码](../../examples/gallery/scripting/dictionaries.lay) · [组合源码](../../examples/gallery/scripting/dictionaries.lay) · [全部参数](interface-reference.zh-CN.md#zip)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `if / else` | 条件分支 | 受限 DSL |
| `for` | 遍历序列 | 受求值预算限制 |

### 常见用法

用条件选择布局分支，用有限循环重复放置素材。脚本有求值和循环限制；大量图形应优先使用原生批量图层。

[完整参数与规则](../language-reference.zh-CN.md)

### 限制与相关主题

[变量与表达式](values.zh-CN.md) · [函数与模块](modules.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## 键值解包与循环快照

`for key in d` 遍历键，`for key,value in d.items()` 遍历键值对。支持 `for i,(name,ys) in enumerate(d.items())` 等嵌套解包，变量属于循环作用域，`_` 丢弃对应值；解包数量须匹配，不允许重复变量名。

`enumerate(seq,start=0)` 返回编号和值；`zip(a,b,...)` 在最短输入结束时停止，零参数返回空列表。列表、字典和几何集合均可迭代。循环开始时固定遍历快照，修改原字典不会给本轮循环追加工作。break、continue、作用域和执行预算保留原有规则；字典键判断写作 `key in d` 或 `key not in d`。

[完整字典绘图](../../examples/plot/dictionary-series.lay) · [预设配色](cmaps.zh-CN.md)
