# 脚本与计算

限定在 .lay 语言中的变量、单位、函数和流程控制。

[返回画廊索引](README.zh-CN.md) · [执行结果](../examples-and-results.zh-CN.md)

## 表达式

<a id="units-builtins"></a>

### 单位与内置函数

append、len、min、max、abs、str 的结果直接写到图片中。

```lay
values = append([2, 4], 6)
count = len(values)
low = min(2 mm, 5 mm, 3 mm)
high = max(2 mm, 5 mm, 3 mm)
distance = abs(-4 mm)
label = text(content="len=" + str(count) + "  min=" + str(low) +
                     "  max=" + str(high) + "  abs=" + str(distance),
             font_family=font, font_size=10 pt, color="#087f8c")
page.add(label, target=page.top_left, offset=(8 mm, 34 mm))
```

![单位与内置函数的实际渲染结果](../../site/media/gallery-scripting-units-builtins-1920.webp)

源码：[units-builtins.lay](../../examples/gallery/scripting/units-builtins.lay)。

复现命令：`laymesh validate examples/gallery/scripting/units-builtins.lay`；`laymesh render examples/gallery/scripting/units-builtins.lay -o units-builtins.png --dpi 150`。

实测：`有效：examples/gallery/scripting/units-builtins.lay（120 × 80 mm，2 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

## 控制流

<a id="control-flow"></a>

### 函数与循环

自定义函数、for、while 和 if 共同计算可见的文字结果。

```lay
function doubled(value, factor=2) { return value * factor }
count = 0
for index in range(3) { count = count + index }
while count < 5 { count = count + 1 }
status = "pending"
if count == 5 { status = "ready" } else { status = "error" }
label = text(content="count=" + str(count) + "  gap=" +
                     str(doubled(3 mm)) + "  " + status,
             font_family=font, font_size=11 pt, color="#087f8c")
page.add(label, target=page.top_left, offset=(10 mm, 34 mm))
```

![函数与循环的实际渲染结果](../../site/media/gallery-scripting-control-flow-1920.webp)

源码：[control-flow.lay](../../examples/gallery/scripting/control-flow.lay)。

复现命令：`laymesh validate examples/gallery/scripting/control-flow.lay`；`laymesh render examples/gallery/scripting/control-flow.lay -o control-flow.png --dpi 150`。

实测：`有效：examples/gallery/scripting/control-flow.lay（120 × 80 mm，2 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。


<a id="dictionaries"></a>

### 字典与解包循环

按插入顺序生成卡片，演示独立副本、更新、键值列表及嵌套解包。

```lay
# Insertion order controls the layout; assigning a dictionary copies its values.
page=canvas(size=(150mm,90mm),background="#ffffff")
labels=dict()
labels["B"]="Baseline"
labels["A"]="Annealed"
copy=labels
copy["B"]="Changed copy"
labels=labels.update({"C":"Cold worked"})
keys=labels.keys()
values=labels.values()
items=labels.items()
default=labels.get("missing","No data")
colors=["#0072b2","#d55e00","#009e73"]
for i,(key,value) in enumerate(items) {
 card=group()
 card.add(rect(size=(42mm,55mm),fill=colors[i]))
 card.add(text(content=key,font_family="DejaVu Sans",font_size=20pt,color="#ffffff"),offset=(5mm,7mm))
 card.add(text(content=value,font_family="DejaVu Sans",font_size=8pt,color="#ffffff"),offset=(5mm,35mm))
 page.add(card,offset=(5mm+i*48mm,10mm))
}
for i,(key,value) in enumerate(zip(keys,values)) {
 page.add(text(content=f"{i+1}. {key} = {value}",font_family="DejaVu Sans",font_size=8pt),offset=(5mm+i*48mm,72mm))
}
```

![字典与解包循环](../../examples/gallery/scripting/dictionaries.png)

源码： [dictionaries.lay](../../examples/gallery/scripting/dictionaries.lay).

复现命令： `laymesh validate examples/gallery/scripting/dictionaries.lay`; `laymesh render examples/gallery/scripting/dictionaries.lay -o dictionaries.png --dpi 150`.

实测： `有效：examples/gallery/scripting/dictionaries.lay（150 × 90 mm，6 个顶层实例）`; PNG **886 × 531 px**, 150 DPI.
