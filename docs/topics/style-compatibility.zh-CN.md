# 样式兼容与排错

[样式手册](lcss.zh-CN.md) · [外观属性参考](style-properties.zh-CN.md)

## CSS 支持边界

完整上游 CSS 可加载，但只将已实现的外观映射到固定画布。背景、字体、边框、圆角、透明度、padding、阴影与动画可生效；flex/grid、width/height/margin/position、交互伪类不参与排版。CSS 不能代替 .lay 的定位参数。

未知属性或无效值按声明恢复，不支持的选择器按规则处理，后续有效声明继续解析。每个文件用一条 W_CSS 汇总忽略项并保留位置。完整 Bootstrap 出现大量忽略项是预期兼容结果。

## 媒体条件与 supports

静态样式默认 screen。尺寸条件基于固定画布，不是浏览器窗口：

~~~lcss
@media screen and (min-width:100mm) {
  .caption { font-size:4mm; }
}
@supports (background:linear-gradient(90deg,red,blue)) {
  .card { background:linear-gradient(90deg,red,blue); }
}
~~~

支持 min/max-width、min/max-height，及 and/or/not 和逗号组合。@supports 只在引擎实际支持属性及其值时成立；不支持的色彩插值分支不会因语法看似 CSS 就激活。

静态减少动态效果条件采用 no-preference，动画预览遵循系统偏好，视频默认 screen/no-preference，见[动画导出与排错](animation-export.zh-CN.md)。

## 按现象排查

| 现象 | 检查与处理 |
| --- | --- |
| 类没有效果 | 是否导入库，class 是否拼对，属性是否适用于对象 |
| 圆角或字号改不动 | .lay 是否已有显式参数，是否有更具体或重要声明 |
| Open Props 变量无效 | 是否导入库，变量是否存在，求值后的属性值是否受支持 |
| fallback 没有启用 | 变量可能是合法空值，而非未定义 |
| 只改 blur 后仍无阴影 | 先声明 box-shadow 或使用阴影类 |
| 阴影被页面切掉 | 调整位置、范围或页面大小 |
| CSS 尺寸没有改变对象 | 使用 .lay 的 size、offset、anchor |
| 导出 DPI 改变后阴影尺寸不变 | 物理长度应保持一致，layout_dpi 才决定 px 换算 |

## 警告和错误

W_CSS 指向样式文件，表示已忽略部分声明或规则。W_FONT 表示字体或字形问题，不代表 CSS 加载失败。W_ANIMATION 说明动作缺少匹配定义或有效时长。

文件缺失、导入循环和 .lay 参数错误仍明确报错。单位错误保持 E_UNIT；160*100/90mm 是倒数长度，可改为 160mm*100/90 或 (160*100/90)*1mm，不自动修改源码。

~~~sh
laymesh validate examples/manual/style-project.lay
laymesh inspect examples/manual/style-project.lay --json
laymesh render examples/manual/style-project.lay -o style.pdf
~~~

源码错误时，网页保留标明的上次成功画面，不能把旧画面当作本次编译结果。
