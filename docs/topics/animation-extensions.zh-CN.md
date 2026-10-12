# 描线与揭示

[动画手册](animation.zh-CN.md) · [步骤与动作参数](animation-timing.zh-CN.md)

## LayMesh 自带关键帧

laymesh-draw/draw 和 laymesh-reveal/reveal 是 LayMesh 自带的关键帧，不属于 Animate.css，不需要为了引用它们导入该库。需要给出时长和适当的填充模式。

| 名称 | 作用 |
| --- | --- |
| laymesh-draw / draw | 逐步显示具有描边的路径 |
| laymesh-reveal / reveal | 逐步扩大揭示区域 |

~~~lcss
.draw-motion { animation:laymesh-draw 1200ms ease both; }
.reveal-motion { animation:laymesh-reveal 1s ease both; }
~~~

也可直接使用 `s.animate(line_instance,name="laymesh-draw",duration=1s,fill="both")`。目标必须是已放置实例；填充图形不会因此变成描边路径。

## 图表与普通对象

图表揭示缩小数据区裁剪框，保留坐标轴和图例；普通对象按布局框揭示。它不是数值插值，也不改变数据、坐标轴范围或静态几何。用入口主题为图表外观设置统一样式，再给放置实例安排揭示动作。

<!-- example:examples/animation/timing.lay -->

## 自定义扩展进度

-laymesh-draw 和 -laymesh-reveal 接受 0–1 的非标准进度，可以放进自定义关键帧：

~~~lcss
@keyframes reveal-part {
  from { -laymesh-reveal:0; }
  to { -laymesh-reveal:1; }
}
~~~

使用常规 animation 或 name 引用此定义。进度、时长、delay、fill 与其他动画一样参与采样和调度。
