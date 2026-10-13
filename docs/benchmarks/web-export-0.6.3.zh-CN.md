# 0.6.3 离线导出与工作区验证

本记录对应 2026-10-13 的 Linux 本地发行检查。跨平台安装包结果以同一提交的 GitHub Actions 为准；本地通过不代替远程发布完成。

## 静态文件导出

在 Chromium、Firefox、WebKit 中从 `file://` 打开离线作品，拦截所有 HTTP 请求，可编辑版逐项下载并检查 15 种格式：PDF、SVG、PPTX、PNG、JPEG、TIFF、WebP、BMP、静态 GIF、ICO、PAM、PPM、PGM、PBM、TGA。放映版裁剪为 PDF、SVG、PNG、JPEG、TIFF，并验证从可编辑版另存、再次保存后都不携带完整编码器。三种浏览器全部通过，没有网络请求、页面异常或额外编译。

测试使用不同尺寸的两页作品，包含文字、透明背景、色块和阴影。通过 Pillow、ZIP/XML 解析及 Poppler 独立检查下载产物：

- 默认 1200 DPI；40×30mm 图片为 1890×1417 像素，PNG/JPEG/TIFF 密度信息和透明像素正确，TIFF 压缩标记为 LZW。
- PDF 合并两页，保留各页物理尺寸；SVG 文字为轮廓，不携带未授权的系统字体文件；PPTX 保留可编辑文字和形状，阴影使用局部图片。
- 当前页、全部页面 ZIP、ICO 尺寸错误及降低 DPI 后导出、取消、设置随 HTML 保存和重新打开均通过。
- 文件选择器写入成功与磁盘写入失败分支均验证；失败不报告成功。播放版同样可以导出静态文件。
- 检查 1440×1000 和 390×844 视口，导出弹窗位于可见内容范围内。

WebKit 的不透明离线 Worker 无法读取自己的 blob URL；CanvasKit 在此环境使用由字节直接生成的本地 data URL。其他宿主保留 blob URL，实际 VS Code 严格资源策略回归通过。

脚本：[浏览器操作](../../scripts/test-web-static-export.cjs)、[独立文件检查](../../scripts/check-web-static-export.py)。结果：[浏览器](web-export-0.6.3/browsers.json)、[产物](web-export-0.6.3/files.json)。

## 体积与许可清单

导出能力按版本分包：可编辑版保留 15 种格式，放映版保留 5 种。放映版导出增量的保守预算为 **240,048 字节（234.4 KiB）**，低于 300 KiB。此数包括导出模块、ZIP 目录开销和新增许可文本，还完整计入了大部分原已存在的播放器界面代码与样式。它是固定功能增量，不是含用户图片／字体的整个作品大小；[预算脚本](../../scripts/check-web-export-budget.py) 在 CI 中强制检查，[明细](web-export-0.6.3/export-budget.json)。

同一份 8 页、共享一张图片的作品，以 `original` 图片策略对比；未嵌入系统字体：

| 文件 | 优化前 | 优化后 | 格式 |
| --- | ---: | ---: | --- |
| 可编辑 HTML | 10.18 MiB | 8.81 MiB | 全部 15 种 |
| 放映 HTML | 6.10 MiB | 3.78 MiB | PDF、SVG、PNG、JPEG、TIFF |

页面数据压缩后从 1,180,480 字节降至 60,367 字节，移除了与已有绘制指令重复的 SVG 快照。图片资源在作品中共用，导出 SVG／PDF 时才组装。体积结果随作品内容变化，不能把这个样例的节省比例套用于所有文件。[原始字节数](web-export-0.6.3/size-comparison.json)。

许可文本按原始字节 SHA-256 去重，保留完整署名与条款；放映版还剔除完整编码器和编辑器独有依赖。没有截断许可正文，也没有通过网络链接替代必须随包携带的文本。Linux VSIX 中的许可文件由 682 份减为 264 份，压缩体积由 1.31 MiB 减至 0.39 MiB；[清单统计](web-export-0.6.3/licenses.json)。

## 播放运行时裁剪

放映文件现在只携带作品实际用到的 GL Transitions 着色器，可编辑版保留完整 125 项目录。无转场样例的目录压缩体积由 42,102 字节降至 31 字节；有转场作品仍保留前进、后退和纹理效果需要的全部项目。Chromium、Firefox、WebKit 验证了裁剪目录后的翻页、连续导航、缩放取消、减少动态效果、放映和退出；可编辑版另存与播放版再次保存也保持裁剪结果。[导航记录](web-export-0.6.3/pruned-transitions.json)。

CanvasKit 本体仍为上游 0.42.0，压缩后 2,940,749 字节。WASM 已无调试自定义区段，不能通过删除调试信息进一步显著缩小。LayMesh 使用自身文字轮廓与排版，未调用 CanvasKit 字体排版、SkParagraph、Skottie 接口；[上游构建脚本](https://github.com/google/skia/blob/e2def78fa232884d4aedcf3ad92d4a0dd673b898/modules/canvaskit/compile.sh) 提供相应裁剪开关。定制构建需要固定与当前包对应的 Skia 源码和 Emscripten 工具链，并重新验证 GPU／CPU、滤镜、透明组、图案和全部转场。当前环境没有该工具链，本版本尚未替换 CanvasKit 二进制，也没有把推测节省量计入结果。WebGL、SkSL 运行时效果、图片解码、路径绘制和图像滤镜仍为必需能力。

从当前 WASM 的初始化数据段重建线性内存后，识别到 ICU `CmnD` 数据包的 20 项资源，起始地址 947296，最后一个 `zone/tzdbNames.res` 的 bundle-top 对应结束地址 1812168；数据共 864,872 字节（0.825 MiB），单独 Deflate 压缩为 461,237 字节（0.440 MiB）。这是可裁剪数据的测量，不是最终 HTML 节省量；仍需重新链接并比较整个文件。固定版本 `no_font` 会关闭 ICU／HarfBuzz 及字体接口，但 GN 参数仍启用 FreeType，应同时核对未引用代码是否被移除。

轻量放映导出已经复用 CanvasKit 的 PNG 编码：PNG 下载、SVG／PDF 中非 PNG/JPEG 资源转换，以及复杂效果的局部图片都在调用 `encodeToBytes(PNG)`。所以当前方案不能直接裁掉 PNG 编码器；如果以后改用浏览器编码，还需重测 DPI、透明度、WebKit 离线 Blob 和局部图片导出。

本次缓存修复回归通过 4K／120 次保留绘制、同 ID 图片替换、失败安装回滚、上下文恢复和主线程回退；97 种元素动画的 873 次比较无失败。驱动为 SwiftShader，另有 CPU 回退验证，不能据此声称完成真实显卡验证。[渲染回归](web-export-0.6.3/renderer.json)。

## 复杂矢量导出

新增 80×60mm 测试页，覆盖旋转透明组、内阴影／发光、渐变透明度、亮度蒙版、裁剪、图案和轮廓文字。在三种浏览器中分别导出可编辑版与放映版的 PNG、SVG、PDF，使用原生 300 DPI PNG 作为参照、Poppler 独立渲染 PDF，检查 PDF 仍包含路径指令且图片均为局部图片。

PNG 平均 RGB 差约 0.15/255；PDF 约 2.91–2.94/255。SVG 由浏览器光栅化后的差异约 0.18–2.46/255，WebKit 对重叠亮度蒙版的合成与 Chromium／Skia 存在可见差异；该差异在两种导出版本中均出现。放映版 PDF 对不支持的透明组、蒙版和渐变混合使用局部高分辨率图片，主体外的路径继续保留矢量。[逐项记录](web-export-0.6.3/vector-exports.json)。

JPEG 使用浏览器主窗口异步编码，规避 WebKit 在 `file://` Worker 中读取自己生成的 Blob 时出现 `NotReadableError`；渲染和背景合成仍在 Worker。PNG／JPEG 均验证物理 DPI，TIFF 保留 LZW 与非预乘透明度。

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
