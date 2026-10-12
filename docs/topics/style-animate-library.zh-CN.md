# Animate.css：基础类、工具类与效果

[样式与动画库](style-libraries.zh-CN.md)

## 是什么、从哪里来

Animate.css 是[开源 CSS 动画库](https://animate.style/)，固定版本 4.1.1，MIT 许可。它提供进入、退出、强调等关键帧与类，不负责对象尺寸和排版。

## 第一段完整动画

<!-- source:examples/libraries/animate.lay -->

先导入、放置实例、创建步骤，再执行动作。`animate__animated` 提供基础时长和填充规则，效果类选择关键帧。进入放映是否等待点击由步骤 `start` 决定。

## 由效果到参数

1. 到[97 个效果目录](animation-catalog.zh-CN.md)选择入场、退场或强调动作。
2. 使用 `duration=800ms`、`delay=200ms` 等[动作参数](animation-timing.zh-CN.md)控制单次播放。显式参数优先于 CSS。
3. 工具类 `animate__fast/faster/slow/slower` 调速，`animate__delay-1s` 至 `delay-5s` 延迟，`animate__repeat-1/2/3` 和 `animate__infinite` 控制重复。
4. `--animate-duration`、`--animate-delay`、`--animate-repeat` 支持统一主题和动作变量覆盖。比较工具类时先移除对应显式覆盖。
5. 无限动作需要有限步骤窗口；打印与减少动态效果另有媒体规则，见[导出与排错](animation-export.zh-CN.md)。

## 类、变量与关键帧完整目录

97 个效果继续使用专用效果说明与共享播放区。下面补齐基础类、工具类、变量和关键帧的逐项案例。

目录案例用低具体程度的 `rect` 底色与边框，让圆角、透明度和动画可见；类声明与变量属性照常级联。

<!-- style-catalog:animate -->
