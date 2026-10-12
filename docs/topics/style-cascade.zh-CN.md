# 选择器、变量与级联

[样式手册](lcss.zh-CN.md) · 下一章：[外观属性与单位](style-properties.zh-CN.md)

## 选择哪些对象

| 选择器 | 含义 |
| --- | --- |
| rect | 矩形素材 |
| .card | 带 card 类的对象 |
| rect.card、.card.emphasis | 类型和类、多个类同时匹配 |
| text, formula | 分组应用 |
| .panel text | panel 内的文字后代 |
| .panel > text | 直接所属的文字 |
| [class~="card"] | class 列表含独立 card 项 |
| :root、html、:where(html) | 根画布 |

class 属性选择器支持等于、词项、前缀、后缀和子串等比较；比较实际类字符串。#id、交互伪类及网页 DOM 选择器不能默认视为受支持。所属关系指 LayMesh 容器和对象关系。

图表部件包括 plot::area、plot::axis(x)、plot::tick-label、plot::axis-label、plot::legend、plot::colorbar。类型或类后选择部件，可以统一轴线和字号。

~~~lcss
plot::area { background:#f8fafc; }
plot::axis(x) { line-color:#334155; line-width:0.6pt; }
plot::tick-label { font-size:8pt; color:#334155; }
.annotation { font-size:9pt; background:#ffffff; padding:1mm; }
~~~

## 同时定义 radius，谁生效

显式 .lay 参数始终优先，包括样式中的重要声明。选择器和 !important 的比较发生在样式声明之间。

~~~lay
page = canvas(size=(80mm,50mm))
style { .card { border-radius:2mm !important; background:#dbeafe; } }
page.add(rect(size=(30mm,20mm),class="card",border_radius=5mm),anchor=center,target=page.center)
~~~

最终使用 5mm 圆角。希望圆角由主题管理时，不要重复传 border_radius 或对应别名 radius；必要的单次例外保留显式参数。

## 样式之间的覆盖顺序

模块样式只作用于模块创建的对象，提供组件默认值；入口主题可以覆盖。在主题声明之间依次比较：

1. !important 优于普通声明。
2. 选择器具体程度：类、属性选择器比单纯类型更具体，多个条件增加具体程度；根 :where 选择器不增加具体程度。
3. 前两项相同时，源码中后声明优先。

先导入库再声明主题，不保证普通声明能覆盖库的重要声明。Bootstrap 阴影类使用 !important：

~~~lcss
@import "@laymesh/bootstrap.css";
.shadow.custom { box-shadow-blur:2mm !important; }
~~~

`class="shadow custom"` 比单独 .shadow 更具体。静态与动画采用同一套规则，stylesheet 与标准 @import 不改变优先级。

## 变量、fallback 与空值

~~~lcss
:root { --accent:#0d6efd; --gap:1mm; }
.card { --edge:var(--accent); border:0.5pt solid var(--edge); padding:var(--gap); }
.caption { color:var(--caption,var(--accent,#334155)); }
~~~

自定义属性以 -- 开头，沿所属关系继承。var(--name, fallback) 在变量不存在或不能解析时使用 fallback；fallback 可以继续引用变量。

`--optional:;` 是合法空变量，与未定义不同：var(--optional,red) 取得空值，不会因空值选择 red；结果不适用于颜色时，当前声明失效。循环变量也可能使声明失效或走可用 fallback，不会停止后续声明。

变量在样式上下文求值，不读取 .lay 变量。inherit 显式取得父对象值，initial 取初始值，unset 对继承属性采用继承、对其他属性采用初始值。

## 哪些属性继承

文字族、数学字体、数学文本回退、字号、字重、字体样式、颜色、行高及文字阴影继承。背景、边框、圆角和盒阴影不继承。透明度自身不继承，但父级和子级透明度相乘。

同一素材可多次放置，各实例通过自己的 class 设置外观。优先把共同外观放入主题。
