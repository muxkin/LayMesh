# 第一段动画

从[样式与动画库](style-libraries.zh-CN.md)开始：先了解内置库、来源和导入方法，再逐项尝试、调参和组合；单文件 `.lay` 就能起步。

[动画手册](animation.zh-CN.md) · 下一章：[步骤与动作参数](animation-timing.zh-CN.md)

## 从静态页面开始

动画附着在已完成的排版上，默认预览与普通导出仍显示完整静态版面。先定义素材，再用 add() 放置，保存返回的实例，最后把实例交给 animate()。不能把未放置的 text(...) 或 rect(...) 当作动画目标。

<!-- example:examples/animation/first.lay -->

完整源码依次做四件事：导入 Animate.css；放置标题；创建名为 Title entrance 的步骤；给标题实例安排 fadeInUp 动作。类名写在动画调用中，作为这次动作的临时 class，不会永久修改素材外观。

## 基础类与效果类

Animate.css 需要同时使用 animate__animated 和效果类。基础类定义默认时长和 fill-mode，效果类选择关键帧；仅有 animate__fadeInUp 可能仍采用 CSS 初始时长 0s。

Bootstrap、Open Props 不提供这些 Animate.css 类；Open Props 另有自己的动画变量和关键帧，见[库手册](style-open-props.zh-CN.md)。引擎内有离线文件不等于已导入；必须使用 stylesheet 或主题中的 @import。

~~~lcss
/* motion.lcss */
@import "@laymesh/animate.css";
.title-motion { --animate-duration:800ms; }
~~~

页面使用 stylesheet="motion.lcss"，对象可以带 title-motion 类；动作使用 class="animate__animated animate__fadeInUp"。

## 点击放映后何时开始

step 默认 start="click"。进入放映后，第一次点击画面、空格或右方向键才执行第一步；仅点击“放映”不是默认的动画开始操作。动画中再次推进会完成当前步并停留。

想让第一步随进入页面启动，使用：

~~~lay
s = page.step(name="入场",start="after-previous",hold=1s)
s.animate(title,class="animate__animated animate__fadeInUp")
~~~

此片段接在上面的页面与 title 定义之后。网页预览菜单中的“播放/重播”可以直接播放时间线，“放映”采用演示步骤规则。

## 自己修改时长

主题中 .custom { animation-duration:2s; } 可以覆盖库的普通 1s 默认值。动作使用 class="animate__animated animate__fadeInUp custom"。单次 duration=3s 则优先于 CSS，包括重要声明。

统一样式级联见[选择器与覆盖](style-cascade.zh-CN.md)。使用[97 个效果目录](animation-catalog.zh-CN.md)选择其他效果；用[步骤与动作参数](animation-timing.zh-CN.md)调整先后顺序。

## 没动时先检查

检查导入、类名、基础类、有效时长、动画目标以及是否仍在等待点击。W_ANIMATION 会指出缺失定义或时长。系统减少动态效果偏好可能使动画很短；显式 none、0s 和媒体禁用都是合法设置。
