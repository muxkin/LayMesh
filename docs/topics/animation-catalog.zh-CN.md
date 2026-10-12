# 97 个 Animate.css 效果

[动画手册](animation.zh-CN.md) · [第一段动画](animation-start.zh-CN.md)

## 选择效果并实际播放

目录来自随引擎分发的 Animate.css 4.1.1 原始文件。下面只有一个共享预览区，选择效果会更新可编辑 Lay 源码，再由 LayMesh WASM 编译、采样并绘制，不使用网页 CSS 模拟效果。

<!-- animation-catalog -->

## 使用任何一项

导入 @laymesh/animate.css，并把表中类名与 animate__animated 一起传给动作。完整源码在预览的源码标签中，可复制、修改时长或保存到项目。需要看退场效果时，点击重播，从可见对象开始；“完整排版”恢复静态画面。

基础工具类 animate__fast/faster/slow/slower 调整时长，animate__delay-1s 至 delay-5s 调整延迟，animate__repeat-1/2/3 和 animate__infinite 控制重复次数。库通过 --animate-duration、--animate-delay、--animate-repeat 变量计算这些值。显式 duration 或 iterations 仍优先；目录为便于比较显式指定 1s，测试速度工具类时先删掉该覆盖。

下面效果说明描述动作类型；具体位移、原点、缓动与时长以固定上游定义为准。二维与三维效果均作用于 LayMesh 实例，具体对象内容仍受[兼容边界](animation-export.zh-CN.md)约束。

<!-- effects:start -->
## 强调

| 类名 | 效果 |
| --- | --- |
| `animate__bounce` | 上下弹跳 |
| `animate__flash` | 反复闪烁 |
| `animate__pulse` | 轻微缩放脉冲 |
| `animate__rubberBand` | 橡皮筋拉伸回弹 |
| `animate__shakeX` | 水平摇动 |
| `animate__shakeY` | 竖直摇动 |
| `animate__headShake` | 水平位移与三维摇头 |
| `animate__swing` | 绕上边缘摆动 |
| `animate__tada` | 缩放与旋转强调 |
| `animate__wobble` | 左右摇摆 |
| `animate__jello` | 斜切变形回弹 |
| `animate__heartBeat` | 两次心跳式缩放 |

## 缩放回弹入场

| 类名 | 效果 |
| --- | --- |
| `animate__backInDown` | 缩放回弹入场，向下 |
| `animate__backInLeft` | 缩放回弹入场，向左 |
| `animate__backInRight` | 缩放回弹入场，向右 |
| `animate__backInUp` | 缩放回弹入场，向上 |

## 缩放回弹退场

| 类名 | 效果 |
| --- | --- |
| `animate__backOutDown` | 缩放回弹退场，向下 |
| `animate__backOutLeft` | 缩放回弹退场，向左 |
| `animate__backOutRight` | 缩放回弹退场，向右 |
| `animate__backOutUp` | 缩放回弹退场，向上 |

## 弹跳入场

| 类名 | 效果 |
| --- | --- |
| `animate__bounceIn` | 弹跳入场 |
| `animate__bounceInDown` | 弹跳入场，向下 |
| `animate__bounceInLeft` | 弹跳入场，向左 |
| `animate__bounceInRight` | 弹跳入场，向右 |
| `animate__bounceInUp` | 弹跳入场，向上 |

## 弹跳退场

| 类名 | 效果 |
| --- | --- |
| `animate__bounceOut` | 弹跳退场 |
| `animate__bounceOutDown` | 弹跳退场，向下 |
| `animate__bounceOutLeft` | 弹跳退场，向左 |
| `animate__bounceOutRight` | 弹跳退场，向右 |
| `animate__bounceOutUp` | 弹跳退场，向上 |

## 淡入

| 类名 | 效果 |
| --- | --- |
| `animate__fadeIn` | 透明度渐变入场 |
| `animate__fadeInDown` | 透明度渐变入场，向下 |
| `animate__fadeInDownBig` | 透明度渐变入场，向下，大幅位移 |
| `animate__fadeInLeft` | 透明度渐变入场，向左 |
| `animate__fadeInLeftBig` | 透明度渐变入场，向左，大幅位移 |
| `animate__fadeInRight` | 透明度渐变入场，向右 |
| `animate__fadeInRightBig` | 透明度渐变入场，向右，大幅位移 |
| `animate__fadeInUp` | 透明度渐变入场，向上 |
| `animate__fadeInUpBig` | 透明度渐变入场，向上，大幅位移 |
| `animate__fadeInTopLeft` | 透明度渐变入场，左上角 |
| `animate__fadeInTopRight` | 透明度渐变入场，右上角 |
| `animate__fadeInBottomLeft` | 透明度渐变入场，左下角 |
| `animate__fadeInBottomRight` | 透明度渐变入场，右下角 |

## 淡出

| 类名 | 效果 |
| --- | --- |
| `animate__fadeOut` | 透明度渐变退场 |
| `animate__fadeOutDown` | 透明度渐变退场，向下 |
| `animate__fadeOutDownBig` | 透明度渐变退场，向下，大幅位移 |
| `animate__fadeOutLeft` | 透明度渐变退场，向左 |
| `animate__fadeOutLeftBig` | 透明度渐变退场，向左，大幅位移 |
| `animate__fadeOutRight` | 透明度渐变退场，向右 |
| `animate__fadeOutRightBig` | 透明度渐变退场，向右，大幅位移 |
| `animate__fadeOutUp` | 透明度渐变退场，向上 |
| `animate__fadeOutUpBig` | 透明度渐变退场，向上，大幅位移 |
| `animate__fadeOutTopLeft` | 透明度渐变退场，左上角 |
| `animate__fadeOutTopRight` | 透明度渐变退场，右上角 |
| `animate__fadeOutBottomRight` | 透明度渐变退场，右下角 |
| `animate__fadeOutBottomLeft` | 透明度渐变退场，左下角 |

## 三维翻转

| 类名 | 效果 |
| --- | --- |
| `animate__flip` | 透视三维翻转 |
| `animate__flipInX` | 三维翻转入场，绕 x 轴 |
| `animate__flipInY` | 三维翻转入场，绕 y 轴 |
| `animate__flipOutX` | 三维翻转退场，绕 x 轴 |
| `animate__flipOutY` | 三维翻转退场，绕 y 轴 |

## 快速滑动

| 类名 | 效果 |
| --- | --- |
| `animate__lightSpeedInRight` | 快速倾斜滑动入场，向右 |
| `animate__lightSpeedInLeft` | 快速倾斜滑动入场，向左 |
| `animate__lightSpeedOutRight` | 快速倾斜滑动退场，向右 |
| `animate__lightSpeedOutLeft` | 快速倾斜滑动退场，向左 |

## 旋转入场

| 类名 | 效果 |
| --- | --- |
| `animate__rotateIn` | 旋转入场 |
| `animate__rotateInDownLeft` | 旋转入场 |
| `animate__rotateInDownRight` | 旋转入场 |
| `animate__rotateInUpLeft` | 旋转入场 |
| `animate__rotateInUpRight` | 旋转入场 |

## 旋转退场

| 类名 | 效果 |
| --- | --- |
| `animate__rotateOut` | 旋转退场 |
| `animate__rotateOutDownLeft` | 旋转退场 |
| `animate__rotateOutDownRight` | 旋转退场 |
| `animate__rotateOutUpLeft` | 旋转退场 |
| `animate__rotateOutUpRight` | 旋转退场 |

## 特殊效果

| 类名 | 效果 |
| --- | --- |
| `animate__hinge` | 从铰链处摆动后掉落 |
| `animate__jackInTheBox` | 缩放旋转弹出 |
| `animate__rollIn` | 边滚动边入场 |
| `animate__rollOut` | 边滚动边退场 |

## 缩放入场

| 类名 | 效果 |
| --- | --- |
| `animate__zoomIn` | 缩放入场 |
| `animate__zoomInDown` | 缩放入场，向下 |
| `animate__zoomInLeft` | 缩放入场，向左 |
| `animate__zoomInRight` | 缩放入场，向右 |
| `animate__zoomInUp` | 缩放入场，向上 |

## 缩放退场

| 类名 | 效果 |
| --- | --- |
| `animate__zoomOut` | 缩放退场 |
| `animate__zoomOutDown` | 缩放退场，向下 |
| `animate__zoomOutLeft` | 缩放退场，向左 |
| `animate__zoomOutRight` | 缩放退场，向右 |
| `animate__zoomOutUp` | 缩放退场，向上 |

## 滑入

| 类名 | 效果 |
| --- | --- |
| `animate__slideInDown` | 滑动入场，向下 |
| `animate__slideInLeft` | 滑动入场，向左 |
| `animate__slideInRight` | 滑动入场，向右 |
| `animate__slideInUp` | 滑动入场，向上 |

## 滑出

| 类名 | 效果 |
| --- | --- |
| `animate__slideOutDown` | 滑动退场，向下 |
| `animate__slideOutLeft` | 滑动退场，向左 |
| `animate__slideOutRight` | 滑动退场，向右 |
| `animate__slideOutUp` | 滑动退场，向上 |

<!-- effects:end -->
