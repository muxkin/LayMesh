# 0.6.4 PDF 导出修复验证

本轮针对用户提供的六页《LayMesh 04 / 一个小伏笔》播放文件，保持 320 × 180 mm 页面和默认 **1200 DPI**。可编辑版继续使用完整 Rust/WASM，VSIX／CLI／Python／Jupyter 使用原生完整引擎；未删格式、修改默认精度或替换原生 PDF 文本语义。

## 实际播放文件

同一 Linux x64 主机、Chromium 151.0.7922.34、file://、CanvasKit 软件导出。内存取浏览器全部进程 RSS 之和，包含预览、GPU 进程和 Worker；“新增峰值”扣除点击导出前基线，不等同于 JS 堆。

| 指标 | v007 修复前 | 修复后 |
|---|---:|---:|
| 六页导出 | 111.014 s | 44.184／44.031／44.274 s |
| 浏览器总峰值 | 3449.3 MiB | 首次 1708.9 MiB；后两次 1633.8／1632.6 MiB |
| 新增峰值 | 2166.2 MiB | 389.5／428.1／402.5 MiB |
| 导出后 RSS | — | 1205.2／1229.0／1212.6 MiB |
| Poppler 首屏，1600 px | 4.68 s | 约 1.1 s |
| PDF 大小 | 2,322,875 B | 2,490,246 B |

导出中位耗时降低 **60.2%**，按最坏一次计算的新增峰值降低 **80.2%**，均超过减半目标。取消到恢复可操作约 **37 ms**。三轮 Worker 都回到导出前数量，导出后 RSS 有波动但没有持续增长；成功下载的短期 Blob URL 会在原有超时后释放。PDF 压缩体积略增，换来更小的解码面和更快的阅读器打开速度。

证据：[修复前](release-0.6.4/pdf-export/baseline.json)、[三次导出及分阶段计时](release-0.6.4/pdf-export/chromium.json)。渲染约 38.6–38.8 s，透明度读取约 0.43 s，压缩约 4.3 s，各页主线程 PDF 转换合计约 0.06–0.08 s；总时间另含 Worker 初始化、消息和最终写入。旧流程约 52 s 用于效果／SVG，约 59 s 用于 PDF 图片转换。

另在特效完成后注入损坏的 SVG 页面：界面显示解析错误、释放 Worker 和临时页面，随后在同一界面重新导出成功，恢复文件仍包含 54 条矢量绘制记录。[失败恢复](release-0.6.4/pdf-export/failure-cleanup.json)。

## 画面与结构

- 每个阴影独立计算边界，按变换后的物理密度分块。64 MiB RGBA 预算包含滤镜输入重叠区；Skia 自行扩展滤镜输入，不重复渲染输出周围的一圈模糊。
- 单色阴影只读取透明度。压缩灰度分块直接进入 PDF，先以不透明灰度拼成蒙版再统一着色，分块保留一像素重叠，避免接缝处重复叠加阴影透明度。复杂彩色回退同样分开拼接颜色和透明度。
- Worker 逐页请求已有绘制数据，压缩分块和页面均等待消费确认。主线程不执行阴影 PNG 编码、Base64／DOM 图片解码和重新压缩；一页转换后即移除 DOM。相同压缩资源按字节复核复用。
- 六页 PDF 有 **58 个灰度图片资源、19 个软蒙版层**；各页保留 **99／54／117／81／95／120** 条矢量绘制记录。普通单图形透明度不回退为图片。复杂透明组合、蒙版或渐变回退包含页码、对象及原因。
- 六页在 1600 px 下与修复前 PDF 比较，RGB 平均误差最大约 **0.106/255**；另以原生 PNG 为参考，覆盖阴影、内阴影、渐变、透明组、蒙版、旋转和非等比缩放，编辑／播放两条路径在三浏览器均通过。

证据：[PDF 结构](release-0.6.4/pdf-export/pdf-structure.json)、[逐页比较与 MuPDF 阅读时间](release-0.6.4/pdf-export/six-page-parity.json)、[复杂效果对比](release-0.6.4/pdf-export/vector-parity.json)。

## 兼容性和边界

- 实际六页文件在 Firefox 153、WebKit 26.5 离线导出成功，取消后可继续导出。WebKit 约 **45.2 s**。Firefox 软件滤镜明显更慢，约 **320.1 s**，其中约 311.8 s 在 CanvasKit 渲染；不将 Chromium 的性能结论套用到 Firefox。初次 Firefox 测试的 240 s 上限超时后，将验证等待上限改为 600 s，未修改作品或 DPI。[Firefox](release-0.6.4/pdf-export/firefox.json)、[WebKit](release-0.6.4/pdf-export/webkit.json)。
- 完整 Rust/WASM Worker 的同一六页作业约 **61 s**，浏览器总峰值约 **2253 MiB**，新增约 **1015 MiB**。逐页 Rust PDF 转换与原 PNG 编码器保留；本轮并未给完整后端带来播放版相同的耗时降幅。[完整后端](release-0.6.4/pdf-export/full-worker.json)。完整后端修复前后六页画面比较最大误差为 0.034/255。[逐页比较](release-0.6.4/pdf-export/full-six-page-parity.json)。
- 原生 PDF 回归覆盖页面尺寸、结构树、公式文本语义和 16 位颜色／透明度；Python 67 项契约通过。真实 Jupyter 内核重复执行示例，检查默认 1200 DPI、TIFF LZW、文档选页、JPEG 展示、NumPy／pandas／Matplotlib 更新和错误恢复。[内核记录](release-0.6.4/pdf-export/jupyter-source.json)。本地 wheel 安装后独立内核复测也通过；五平台流水线继续复核。[安装后的内核记录](release-0.6.4/pdf-export/jupyter-installed.json)。
- 尝试过 GPU 导出：软件 GPU 下约 23.6 s，但新增峰值约 1315 MiB，未达到内存减半目标，因此未作为此次默认实现。
- 本轮实机是 Linux；Windows Edge 物理设备、真实 GPU 的导出耗时与峰值未实测。跨平台构建和原生测试不替代这项实机验证。
- 最终构建在三浏览器均通过可编辑版 15 格式、播放版 5 格式的实际文件解码、尺寸、透明度、DPI 和重新打开检查；v008 六页自然放映最终进度为 100%，控制台镜像回归通过。[导出格式](release-0.6.4/pdf-export/static-formats.json)、[放映](release-0.6.4/pdf-export/presentation.json)。

## 体积、复现与发行

保守预算进一步计入完整 ZIP／压缩共享模块与许可，目前播放版累计功能代码**303,153 B（296.05 KiB）**，低于 300 KiB；作品本身的图片、字体和绘制数据另计。[预算记录](release-0.6.4/pdf-export/budget.json)。原 v007、附件和已构建发行包保留，新文件通过播放版界面重新下载，六页绘制数据与 v007 相同，运行时模块与构建产物一致。[文件校验](release-0.6.4/pdf-export/v008.json)。新文件名为 `04_LCSS_离线播放_v008.zip.html`。旧文件不会在打开时联网更新，需改用重新导出的版本。

```sh
node scripts/test-web-pdf-resources.mjs
cargo test -p laymesh-render --test portable_svg --test skia_pdf --locked
# 设置 PLAYWRIGHT_MODULE；性能测试独立运行，避免构建或其他浏览器干扰。
node scripts/test-web-pdf-performance.cjs /path/to/04_LCSS_离线播放_v008.zip.html /tmp/pdf-results
node scripts/test-web-vector-export.cjs /tmp/vector-results
python scripts/check-web-vector-export.py /tmp/vector-results
python scripts/check-web-export-budget.py
```

0.6.4 修复后的正式包以本轮重新构建、审计的产物为准。之前五平台审计记录属于 PDF 修复前候选包；发布继续使用既有 PyPI 审批门禁，Marketplace 手动上传。

首轮 CI 在样式库目录的旧引擎指纹检查处失败，未进入浏览器测试；随后重新生成 3,316 个目录条目并验证 1,168 个可用示例。目录内容不变，仅更新引擎来源指纹，完整文档站重建／检查通过。修订后的 CI Linux wheel 与 v008 比较了 49 个运行时模块：执行代码一致，两处 WASM 差异仅为编译诊断中的工作目录路径。[运行时核对](release-0.6.4/pdf-export/v008-ci-runtime.json)。

修复后的[五平台构建与审计](https://github.com/muxkin/LayMesh/actions/runs/38559302420) 已通过，构建源码 `f37f735e14dd3e06478f9aa7ef1538fe3482e968`。下载后十个包校验值均匹配，wheel／VSIX／真实 Jupyter 的引擎哈希逐平台一致，Python 3.10／3.14 十组兼容任务全部通过。[候选包审计](release-0.6.4/pdf-export/candidate-packages.json)。核心检查为 630 项 Rust 测试通过、1 项手动基准忽略，891 项契约断言均已映射。

随后浏览器 CI 暴露既有演讲者测试的时序问题：两秒动画结束后，测试仍尝试点击已禁用的“继续动画”。测试夹具改为八秒，并明确断言暂停状态和暂停期间时间不变，再验证自然播放完成；没有修改作品或产品的动画时长。修订后三浏览器本地通过。[控制测试](release-0.6.4/pdf-export/presenter-controls.json)。
