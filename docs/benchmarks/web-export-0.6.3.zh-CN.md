# 0.6.3 离线导出与工作区验证

本记录对应 2026-10-13 的 Linux 本地发行检查。跨平台安装包结果以同一提交的 GitHub Actions 为准；本地通过不代替远程发布完成。

## 静态文件导出

在 Chromium、Firefox、WebKit 中从 `file://` 打开离线作品，拦截所有 HTTP 请求，逐项下载并检查 15 种格式：PDF、SVG、PPTX、PNG、JPEG、TIFF、WebP、BMP、静态 GIF、ICO、PAM、PPM、PGM、PBM、TGA。三种浏览器全部通过，没有网络请求、页面异常或额外编译。

测试使用不同尺寸的两页作品，包含文字、透明背景、色块和阴影。通过 Pillow、ZIP/XML 解析及 Poppler 独立检查下载产物：

- 默认 1200 DPI；40×30mm 图片为 1890×1417 像素，PNG/JPEG/TIFF 密度信息和透明像素正确，TIFF 压缩标记为 LZW。
- PDF 合并两页，保留各页物理尺寸；SVG 文字为轮廓，不携带未授权的系统字体文件；PPTX 保留可编辑文字和形状，阴影使用局部图片。
- 当前页、全部页面 ZIP、ICO 尺寸错误及降低 DPI 后导出、取消、设置随 HTML 保存和重新打开均通过。
- 文件选择器写入成功与磁盘写入失败分支均验证；失败不报告成功。播放版同样可以导出静态文件。
- 检查 1440×1000 和 390×844 视口，导出弹窗位于可见内容范围内。

WebKit 的不透明离线 Worker 无法读取自己的 blob URL；CanvasKit 在此环境使用由字节直接生成的本地 data URL。其他宿主保留 blob URL，实际 VS Code 严格资源策略回归通过。

脚本：[浏览器操作](../../scripts/test-web-static-export.cjs)、[独立文件检查](../../scripts/check-web-static-export.py)。结果：[浏览器](web-export-0.6.3/browsers.json)、[产物](web-export-0.6.3/files.json)。

## 字体与交互

真实字体管理器与选择列表使用模拟的 1,005／5,000 项 FontData 目录；测量不代表操作系统枚举或磁盘延迟。

| 目录项数 | 连接及界面更新 | 搜索 P95 | 选择 P95 | 无关字体读取 | 必需字体读取 |
| --- | --- | --- | --- | --- | --- |
| 1,005 | 11.7ms | 0.4ms | 0.1ms | 0 | 1 |
| 5,000 | 12.3ms | 0.6ms | 0.1ms | 0 | 1 |

三种浏览器分别验证同款字体导入自动匹配、中文名称、替代与字重、关闭／重连、字体损坏，以及预览缩放和平移。文件管理和菜单测试覆盖目录事务、引用更新、未保存文本、撤销／重做、键盘菜单、弹窗、只读项、调色块尺寸与撤销。字体测量见 [JSON](web-export-0.6.3/fonts.json)。

## 引擎与宿主

- `scripts/test-contracts.py` 全量通过；891 个历史断言调用点均有映射，执行与源码指纹检查通过。
- GL Transitions 的 125 个效果通过 CanvasKit CPU 起止帧、多个中间帧、透明页面、宽高比、参数与纹理检查；详细边界见[转场验证](page-transitions.zh-CN.md)。
- `.laydoc` 同源页面分别应用文档默认、单页覆盖、关闭及重复引用；网页 Worker 与 VS Code 原生池都只编译 3 个不同配置，见 [JSON](web-export-0.6.3/laydoc.json)。
- 实际 VS Code 预览验证转场、原生导出、暗色／浅色、DPR 2、未保存缓冲区，以及禁止 Worker 请求扩展资源时仍可运行。
- 文档检查覆盖 API、样式、完整库目录、示例与媒体指纹。

浏览器 WebP 使用无损编码；MP4／动画 GIF 和完整原生 PDF／WebP 调节选项继续由 CLI／VS Code 提供。实体 Windows 跨电脑字体授权、真实显卡和 Office 应用打开后的人工视觉检查没有在本地模拟测试中验证。
