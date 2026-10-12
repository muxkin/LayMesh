# 自定义关键帧与 CSS 动画

[动画手册](animation.zh-CN.md) · [效果目录](animation-catalog.zh-CN.md)

## 定义动画与调用动画

@keyframes 定义随时间变化的属性，动画声明选择定义及播放参数，animate() 把它安排到实例。仅定义关键帧不会自动播放。

<!-- example:examples/animation/keyframes.lay -->

在完整源码的 keyframes.lcss 标签中编辑关键帧。from 等于 0%，to 等于 100%；中间位置可用百分比。上例透明度从 0 到 1，并向右移动 35mm。中间关键帧只修改 opacity，不会突然丢失 transform。

## 简写和长属性

~~~lcss
.motion { animation:move 2s ease-in-out 200ms 2 alternate both running; }
~~~

简写包含名称、时长、缓动、延迟、次数、方向、填充、状态；第一个时间是 duration，第二个是 delay。逗号定义多个动画。重新声明 animation 会重置对应长属性；长属性列表按 CSS 的列表规则匹配动画名称。

| 属性 | 初始值与意义 |
| --- | --- |
| animation-name | none；关键帧名称列表 |
| animation-duration | 0s；每轮非负时长 |
| animation-delay | 0s；可负，负值从中途开始 |
| animation-timing-function | ease；关键帧间进度函数 |
| animation-iteration-count | 1；支持零、小数和 infinite |
| animation-direction | normal；播放方向 |
| animation-fill-mode | none；活动区间之外的取值 |
| animation-play-state | running；paused 冻结动作 |

关键帧内 !important 忽略。缺省端点从对象基础属性取得；同一位置重复声明按属性合并；动画列表后面的效果覆盖前面相同属性。不同属性可同时动画。

## 支持的动画属性和变换

动画改变 opacity、transform、transform-origin、visibility、backface-visibility，以及[LayMesh 扩展](animation-extensions.zh-CN.md)。不提供任意颜色动画、形状变形或数据插值。

| 变换 | 用途 |
| --- | --- |
| translate、translateX/Y/Z、translate3d | 移动 |
| scale、scaleX/Y/Z、scale3d | 缩放 |
| rotate、rotateX/Y/Z、rotate3d | 旋转 |
| skew、skewX/Y | 斜切 |
| matrix、matrix3d | 直接矩阵 |
| perspective | 透视 |

变换顺序影响结果。百分比平移与原点以未变换布局框为基准；px 使用 layout_dpi，角度用 deg/rad 等。transform-origin 默认中心；backface-visibility:hidden 可隐藏翻转后的背面。三维效果使用局部投影，不引入网页三维场景排版。

动画透明度替换对象自身 opacity，父级透明度仍相乘。动画变换不修改静态锚点，也不会让其他对象重新定位。

## 缓动

linear 匀速；ease、ease-in、ease-out、ease-in-out 改变加减速；cubic-bezier(x1,y1,x2,y2) 自定义进度曲线，x 控制点在 0–1。steps(n, jump-start|jump-end|jump-none|jump-both) 分段跳变，start/end 也可作为常用位置名称。CSS steps() 是缓动函数，不是 Lay 的演示步骤。

关键帧中的 animation-timing-function 控制从该帧到下一帧的区间。上例在 50% 之后使用 ease-out。

## 方向、重复和终态

normal 正向，reverse 反向，alternate 每轮交替，alternate-reverse 从反向开始交替。小数次数停在最后一轮的部分进度；无限动画需要有限步骤窗口。

| fill | 活动区间外 |
| --- | --- |
| none | 不保留动画值 |
| backwards | 延迟期间应用起始值 |
| forwards | 结束后保留最终值 |
| both | 同时采用 backwards 和 forwards |

Animate.css 基础类已设置 both，退出效果因此通常保持消失。fill=none 结束后恢复基础值。移动是否保持取决于最后关键帧和 fill；pulse 首尾相同，自然回到原处。

第一步的 backwards/both 可预置入场初态；后续尚未激活动作不会抢占同一属性。回退和定位重新采样，退出放映恢复完整原始静态布局。

## 样式与动画综合案例

[三库卡片与艺术字海报](style-animation-examples.zh-CN.md)展示主题、布局、关键帧和步骤如何一起使用，并提供已有复杂仪表板场景的完整源码。
