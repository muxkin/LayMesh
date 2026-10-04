# 容器与组件

使用局部坐标组织多个对象，并通过组和本地模块复用布局。

[返回画廊索引](README.zh-CN.md) · [执行结果](../examples-and-results.zh-CN.md)

## 组坐标

<a id="group-local"></a>

### 组内坐标

子元素围绕组内已有实例的中心定位，再整体放到页面。

```lay
badge = group()
back = badge.add(rect(size=(44 mm, 32 mm), border_radius=3 mm, fill="#d8f0ec"),
                 target=badge.top_left)
badge.add(ellipse(size=(14 mm, 14 mm), fill="#eaa94a"),
          anchor=center, target=back.center)
page.add(badge, target=page.top_left, offset=(29 mm, 29 mm))
```

![组内坐标的实际渲染结果](../../site/media/gallery-containers-group-local-1920.webp)

源码：[group-local.lay](../../examples/gallery/containers/group-local.lay)。

复现命令：`laymesh validate examples/gallery/containers/group-local.lay`；`laymesh render examples/gallery/containers/group-local.lay -o group-local.png --dpi 150`。

实测：`有效：examples/gallery/containers/group-local.lay（120 × 80 mm，2 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="group-nested"></a>

### 嵌套与缩放

内层组被外层组包含，外层实例统一缩放并旋转。

```lay
inner = group()
inner.add(ellipse(size=(15 mm, 15 mm), fill="#f2ab47"), target=inner.top_left)
outer = group()
plate = outer.add(rect(size=(52 mm, 35 mm), fill="#d8f0ec"),
                  target=outer.top_left)
outer.add(inner, anchor=center, target=plate.center)
page.add(outer,size=(73 mm, 49 mm), target=page.top_left,
         offset=(21 mm, 25 mm), rotation=-7 deg)
```

![嵌套与缩放的实际渲染结果](../../site/media/gallery-containers-group-nested-1920.webp)

源码：[group-nested.lay](../../examples/gallery/containers/group-nested.lay)。

复现命令：`laymesh validate examples/gallery/containers/group-nested.lay`；`laymesh render examples/gallery/containers/group-nested.lay -o group-nested.png --dpi 150`。

实测：`有效：examples/gallery/containers/group-nested.lay（120 × 80 mm，2 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

## 复用

<a id="group-reuse"></a>

### 同一组放置两次

同一组以不同尺寸放置两次，不回写组内原始定义。

```lay
card = group()
base = card.add(rect(size=(38 mm, 28 mm), border_radius=3 mm, fill="#d8f0ec",
                     border_color="#087f8c", border_width=0.5 mm), target=card.top_left)
card.add(ellipse(size=(10 mm, 10 mm), fill="#f2ab47"),
         anchor=center, target=base.center)
first = page.add(card, target=page.top_left, offset=(8 mm, 30 mm))
page.add(card,size=(52 mm, 38 mm), target=first.top_right,
         offset=(11 mm, -3 mm))
```

![同一组放置两次的实际渲染结果](../../site/media/gallery-containers-group-reuse-1920.webp)

源码：[group-reuse.lay](../../examples/gallery/containers/group-reuse.lay)。

复现命令：`laymesh validate examples/gallery/containers/group-reuse.lay`；`laymesh render examples/gallery/containers/group-reuse.lay -o group-reuse.png --dpi 150`。

实测：`有效：examples/gallery/containers/group-reuse.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="module-import"></a>

### 本地模块组件

从同目录 .lay 模块导入卡片函数，产生两个独立实例。

```lay
import { card } from "./card-component.lay"
first = page.add(card("A"), target=page.top_left, offset=(8 mm, 29 mm))
page.add(card("B"), target=first.top_right, offset=(13 mm, 0 mm))
```

![本地模块组件的实际渲染结果](../../site/media/gallery-containers-module-import-1920.webp)

源码：[module-import.lay](../../examples/gallery/containers/module-import.lay)。

复现命令：`laymesh validate examples/gallery/containers/module-import.lay`；`laymesh render examples/gallery/containers/module-import.lay -o module-import.png --dpi 150`。

实测：`有效：examples/gallery/containers/module-import.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。
