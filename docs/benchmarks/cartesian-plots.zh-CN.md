# 二维科研绘图验收与性能实测

> 历史 Node 基线记录（`codex/node-baseline`）。当前 Rust 安装、字体政策与命令见[安装说明](../topics/install.zh-CN.md)和[架构](../architecture.zh-CN.md)。

[English](cartesian-plots.en.md) · [完整参考](../cartesian-plots.zh-CN.md) · [原始测量](cartesian-plots.json)

测量时间：`2026-10-02T04:36:38.114Z`。本地 Linux，Node `v24.13.1`，CPU `AMD Ryzen 7 8745H w/ Radeon 780M Graphics`。以下只描述本轮运行，不据旧基准推断，也不外推到远端 CI 或其他绘图工具。

## 复现与口径

```sh
npm run build
node scripts/benchmark-cartesian-plots.mjs
node scripts/check-plot-baseline.mjs
```

`性能脚本`（`codex/node-baseline`）对每种场景启动三个全新的 Node 进程。编译时间包括解析、统计、轴映射和布局；SVG、180 DPI PNG、PDF 按顺序分别计时。PNG 时间包含重新生成 SVG。总进程耗时含启动与模块加载，峰值 RSS 为该子进程全程最大值，包含编译和三种导出，不能当成单个后端的独立峰值。未删点或下采样。表中时间为三次中位数，RSS 为三次最大值，文件体积为第一次导出的字节数换算 MiB。

| 场景 | 编译 ms | SVG ms | PNG ms | PDF ms | 总进程 ms | 峰值 RSS MiB | SVG / PNG / PDF MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| scatter-50000 | 103.7 | 87.1 | 371.8 | 665.3 | 1559.6 | 555.2 | 8.550 / 0.248 / 3.913 |
| three-axes-independent-breaks | 460.9 | 5.2 | 2935.9 | 808.0 | 4539.8 | 457.1 | 10.725 / 0.060 / 4.173 |
| statistics-20000 | 194.2 | 3.9 | 75.8 | 80.6 | 674.9 | 248.0 | 1.747 / 0.024 / 0.166 |

散点包含 50000 个颜色值、0.8 mm 标记和逐点绘制透明度；多轴场景为三条各 50000 点折线，横轴断开，两条纵轴使用不同断口，第三条纵轴连续；统计场景用 20000 个确定性样本生成 40 箱密度、箱线、小提琴和 ECDF。分段图层会在各可见窗口中保留裁剪几何，密集路径仍可能增加输出体积和 PNG 时间。

## 几何、数值与输出验收

- Node：114/114；Python：14/14。完整命令为 `npm test` 和 `PYTHONPATH=python python -m unittest discover -s python/tests -v`。
- 旧六份绘图：scientific、annotations、publication、markers、scientific-labels、colorbars。SVG/180 DPI PNG 文件相同；PDF 由同一 Poppler 以 180 DPI 渲染后逐像素相同。原示例文件作为冻结基线，另存[基线 SHA-256](default-plot-baseline.json)。PDF 不比较带时间戳的文件字节。
- 新增三轴独立断口、正反向和 symlog、四侧命名轴、跨断口线/柱/填充/热图/误差棒、物理段长及间距、数据/轴锚点、重复放置、外框覆盖、旋转与组缩放检查。
- 可手算的直方图端点/权重/密度积分、R-7 分位数及离群点、Gaussian KDE、重复值 ECDF、五种颜色归一化、D3 半网格坐标修正和缺失网格覆盖测试。
- 四份手工示例与 Python 数据示例均导出 SVG/PDF/PNG；目视检查 PNG 和 PDF 渲染页，SVG 经同一 PNG 后端检查。另有像素断口/透明度/标记描边测试，以及 PDF 普通文字与公式源码提取测试。
- 缺失网格的裁剪矩形合并保持逐单元格覆盖、不产生重叠；避免将等高线路径重复写入每个有效单元格。
- 画廊 48 例及 Notebook 检查；中英文文档构建、站内链接及图片检查。常量小提琴和 Python 样例的范围外/缺失数据警告为预期结果。

此次实现保持手动面板定位。大数据文件体积和内存仍需按具体图层与格式评估；测量不构成对全部场景的性能承诺。
