# 原生绘图性能实测

> 历史 Node 基线记录（`codex/node-baseline`）。当前 Rust 安装、字体政策与命令见[安装说明](../topics/install.zh-CN.md)和[架构](../architecture.zh-CN.md)。

[原始 JSON](native-plots.json) · [可复跑脚本](../../scripts/benchmark-plots.py) · [绘图指南](../plotting.zh-CN.md)

2026-09-27 本机实测：**原生绘图减少了 Figure 转换与 CLI 桥接的耗时；这些样例并未证明其普遍快于 Matplotlib 单独绘图。** 原生输出 SVG 内嵌完整字体，十万点散点的矢量预览及栅格化仍有较高内存开销。

## 方法和边界

- 硬件：AMD Ryzen 7 8745H w/ Radeon 780M Graphics，16 个逻辑 CPU；Linux-6.12.73+deb13-amd64-x86_64-with-glibc2.41。Node v24.13.1，Python 3.13.11，Matplotlib 3.10.8，NumPy 2.4.4。
- 每个流程读取相同 JSON、相同数据；页面 120×90 mm、绘图区 74×46 mm、DejaVu Sans 8 pt、线宽 0.6 pt、散点直径 3 pt；固定范围及刻度。输出 SVG 预览和 254 DPI PNG（全部检查为 1200×900 px）。
- 折线/散点分别为 1,000、100,000 点；热图为 64×64、512×512。均不抽样；Matplotlib 关闭路径简化；热图使用最近邻显示与相同 viridis 配色范围。
- 冷启动：每次启动独立进程，包含导入与退出，3 次取中位数。重复生成：同进程先预热 1 次，再测 3 次完整生成的中位数；输入读取、布局、SVG 预览与 PNG 写入计时，原始数据准备不计时。
- RSS：每 10 ms 采样进程树 RSS 总和，取所有冷/热运行的最大值。它是近似峰值，可能重复计算共享内存；不是单纯 JS 堆大小，也不是无误差的峰值测量。
- Figure 桥接包含已有接口的转换、SVG 兼容检查、CLI 输出和预览。热图会按已有规则把整个 Figure 回退成 PNG；原生热图只栅格化数据层。PNG 最终尺寸相同，但文字排版、抗锯齿及预览字体编码不同，不能解释为逐像素相同的渲染器对决。
- 文件体积列分别统计最终 PNG 和内存中 SVG 预览。PNG 压缩体积不能代表数据精度。SVG 原生字体内嵌与 Matplotlib 字形路径策略不同，会影响体积。

## 结果

单位：时间 s、内存 MiB、文件 KiB。样本数较少，这是一组可复跑的工作流测量，不是稳定的延迟分布或跨机器性能承诺。

| 数据 | 流程 | 冷启动中位数 | 重复生成中位数 | 近似峰值 RSS | PNG | SVG 预览 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 折线 1000 | 原生 LayMesh | 0.425 | 0.081 | 259.0 | 22.9 | 1012.6 |
| 折线 1000 | Matplotlib | 0.347 | 0.040 | 75.8 | 44.8 | 34.7 |
| 折线 1000 | Figure 桥接 | 1.331 | 1.030 | 229.6 | 24.3 | 45.6 |
| 折线 100000 | 原生 LayMesh | 0.534 | 0.189 | 393.0 | 22.8 | 2892.3 |
| 折线 100000 | Matplotlib | 0.421 | 0.109 | 113.3 | 44.7 | 2377.6 |
| 折线 100000 | Figure 桥接 | 1.644 | 1.324 | 312.2 | 24.3 | 3169.4 |
| 散点 1000 | 原生 LayMesh | 0.437 | 0.087 | 237.1 | 31.3 | 1079.8 |
| 散点 1000 | Matplotlib | 0.367 | 0.049 | 76.8 | 35.0 | 98.6 |
| 散点 1000 | Figure 桥接 | 1.413 | 1.115 | 245.0 | 32.5 | 130.7 |
| 散点 100000 | 原生 LayMesh | 1.240 | 0.914 | 766.4 | 22.4 | 9630.4 |
| 散点 100000 | Matplotlib | 1.090 | 0.776 | 126.8 | 35.5 | 8725.6 |
| 散点 100000 | Figure 桥接 | 8.526 | 8.261 | 1185.3 | 23.9 | 11633.4 |
| 热图 64×64 | 原生 LayMesh | 0.419 | 0.084 | 229.0 | 36.3 | 1001.3 |
| 热图 64×64 | Matplotlib | 0.371 | 0.050 | 80.8 | 31.8 | 23.3 |
| 热图 64×64 | Figure 桥接 | 1.380 | 1.077 | 235.9 | 40.4 | 53.6 |
| 热图 512×512 | 原生 LayMesh | 0.505 | 0.142 | 301.8 | 116.5 | 1080.1 |
| 热图 512×512 | Matplotlib | 0.468 | 0.141 | 120.2 | 140.0 | 58.1 |
| 热图 512×512 | Figure 桥接 | 1.499 | 1.188 | 272.1 | 122.7 | 163.4 |

十万点折线重复生成，原生约 0.189 s，Matplotlib 约 0.109 s，Figure 桥接约 1.324 s。十万点散点原生约 0.914 s、766 MiB，Matplotlib 约 0.776 s、127 MiB，桥接约 8.261 s、1185 MiB。原生方案目前的主要收益是统一绘图与排版、保留可提取的轴文字、避免整个热图页面的栅格回退，以及减少桥接步骤；大规模散点的内存仍是后续优化方向。

## 复跑

```sh
npm ci
npm run build
python -m pip install -e './python' pillow psutil
python scripts/benchmark-plots.py --repeats 3 --output docs/benchmarks/native-plots.json
```

JSON 保存每次观测和警告。改变实现、依赖、字体或硬件后应重新运行；本 Markdown 是本次结果快照，后续复跑不会自动改写正文。

English: the benchmark measures complete SVG-preview-plus-PNG workflows, with equal data, physical geometry, fonts and output pixel dimensions. Native plotting reduced Figure-bridge overhead in these cases; it did not establish a general speed or memory advantage over standalone Matplotlib. Cold startup and repeated generation are reported separately. RSS is a sampled sum across the process tree. Heatmap bridge fallback and font-embedding differences limit direct renderer comparisons.
