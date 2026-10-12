# 阴影与效果

[样式手册](lcss.zh-CN.md) · 下一章：[兼容与排错](style-compatibility.zh-CN.md)

## 三种 CSS 阴影

| 效果 | 轮廓与适用对象 |
| --- | --- |
| box-shadow | 布局边框盒，透明对象、椭圆内容也使用盒轮廓 |
| text-shadow | 文字或公式内容的 alpha 轮廓 |
| filter:drop-shadow(...) | 渲染内容的 alpha 轮廓，目前支持一个 drop-shadow |

~~~lcss
.card { box-shadow:1mm 2mm 3mm 0mm #0004; }
.title { text-shadow:0.5mm 0.5mm 1mm #0006; }
.cutout { filter:drop-shadow(1mm 1mm 1mm #0005); }
~~~

盒阴影依次为 x 偏移、y 偏移、模糊半径、扩散距离和颜色。前两个长度必需，模糊和扩散可省略，省略颜色使用 currentColor。文字阴影不接受 spread 或 inset；drop-shadow 不接受 spread 或 inset。

## 多层、内阴影与负扩散

~~~lcss
.card {
  box-shadow:1mm 2mm 3mm #192c4e40,
             3mm 4mm 6mm -1mm #2e2a6820;
}
.inset { box-shadow:inset 1mm 1mm 3mm #33415580; }
~~~

逗号定义多个阴影，首层在上。正 spread 外扩、负 spread 收缩。外阴影裁掉盒内部分；内阴影在背景上、内容和边框下。效果随组变换并遵循裁剪，超出画布部分可能被裁掉。

<!-- example:examples/gallery/styles/layers.lay -->

## partial-overrides

引用样式库后只把模糊改为 2mm：

~~~lcss
@import "@laymesh/bootstrap.css";
.shadow.custom { box-shadow-blur:2mm !important; }
~~~

class="bg-primary rounded shadow custom" 保留库的偏移、扩散、颜色和已有层数。Bootstrap 阴影带重要声明，局部覆盖也需要 !important 和更具体的选择器。

| 调整意图 | 声明 |
| --- | --- |
| 模糊半径变为 2mm | box-shadow-blur:2mm |
| 向外扩散 2mm | box-shadow-spread:2mm |
| 向右、向下偏移 2mm | box-shadow-offset:2mm 2mm |
| 改颜色 | box-shadow-color:#0d6efd66 |
| 改为内阴影 | box-shadow-position:inset |

下面从左到右是库的原始阴影、仅把模糊改为 2mm、仅把扩散改为 2mm、仅把 x/y 偏移改为 2mm。每项只改一个分项，颜色及其余字段保持原值；完整源码可以直接编辑比较。

<!-- example:examples/gallery/styles/shadow-parameters.lay -->

分项名称采用 [CSS Borders Level 4 工作草案](https://www.w3.org/TR/css-borders-4/#box-shadow)，仍处于草案阶段。以下规则说明 LayMesh 当前实现。单值应用到全部已有层，逗号列表逐层设置，较短列表循环使用。仅设置 blur 不会创建阴影。offset 的单值也保留层数，显式 offset 列表可设置层数；单层一个 offset 长度同时设置 x/y，none 关闭对应层。

重新声明 box-shadow 会重置分项；box-shadow:none 清除阴影。重要分项仍参与级联，普通简写不能越过重要声明。

<!-- example:examples/manual/css-shadow-override.lay -->

## blur 与原有 effects 的区别

CSS 盒阴影和文字阴影的模糊半径转换为高斯标准差时除以 2：blur=2mm 对应 sigma=1mm。drop-shadow 的第三个长度直接表示 sigma。

effects=[shadow(...),glow(...)] 是独立的 LayMesh 效果栈，按列表顺序叠加，保留原有 target、顺序和模糊语义。shadow(blur=2mm) 使用 sigma=2mm，spread 非负。盒阴影分项不会修改此栈，见[艺术字与效果](art-effects.zh-CN.md)。

## 导出与预算

SVG、PNG 和 PDF 使用同一效果定义。PDF 主体保留矢量，模糊效果局部栅格化。输出 DPI 影响栅格细节，不改变毫米长度。沿用现有像素与内存预算；预算错误需缩小影响范围或降低相关栅格密度，不能通过隐藏警告忽略错误。
