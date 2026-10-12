# Open Props：变量、参数与完整目录

[样式与动画库](style-libraries.zh-CN.md)

## 是什么、从哪里来

Open Props 是[开源设计变量库](https://open-props.style/)，固定使用 1.7.23 原始 CSS，MIT 许可。它提供已经配好的颜色、渐变、圆角、阴影、字体尺度和运动参数；不是给对象套上的一组外观类。

## 第一组变量

<!-- source:examples/libraries/open-props.lay -->

定义 `.card` 把变量应用到属性，再在对象上使用 `class="card"`。只导入文件不会自动给所有对象加渐变和阴影。

## 变量分别用在哪里

| 变量类型 | 用到的属性 | 参数含义 |
| --- | --- | --- |
| `--blue-*`、`--gray-*` 等 | color、background-color、border-color | 同色系不同深浅 |
| `--gradient-*` | background | 完整渐变配方 |
| `--radius-*` | border-radius | 圆角尺度 |
| `--shadow-*`、`--inner-shadow-*` | box-shadow | 多层外阴影或内阴影 |
| `--size-*` | padding 等受支持的长度属性 | 留白尺度；尺寸定位仍由 Lay 设置 |
| 字号、字重、行高变量 | 对应 font/line-height 属性 | 按实际属性选择变量，不把字体族当字号 |
| `--ease-*`、`--duration-*` | animation-timing-function、animation-duration | 缓动或时长 |
| `--animation-*` | animation | 库自己的动画配方和关键帧 |

相对长度按根字号、画布尺寸及 `layout_dpi` 计算。颜色、字体、动画和辅助变量不能互换使用；每项的适用属性见目录。

## Open Props 自己也有动画

<!-- source:examples/libraries/open-props-motion.lay -->

此例只导入 Open Props，用 `--animation-fade-in`；它不使用 Animate.css 的 `animate__fadeIn` 类。Open Props 运动变量与 Animate.css 效果目录是两组不同定义。

## 完整可检索目录

目录列出全部声明的变量与关键帧，包括辅助变量和当前不能用于渲染的内容。可用项都有完整案例；同类共用预览。字体族不附带字体文件；不支持的 CSS 消费属性或求值失败会说明原因。

目录案例用低具体程度的 `rect` 底色与边框，让圆角、透明度和动画可见；类声明与变量属性照常级联。

<!-- style-catalog:open-props -->
