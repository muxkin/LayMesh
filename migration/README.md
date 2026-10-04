# Rust + RaTeX 语义修复验收（0.3.0a2）

本轮已恢复图例、轴颜色、统计与极坐标数据规则、实例锚点、文字/图像适配及编辑交互的已识别迁移回归，并完成库接入和验收。之前的输入回放与导出成功检查覆盖不足，确实漏掉了截图中的上层语义；这不能笼统归结为换库后的外观差异。

工作分支为 `codex/rust-ratex`；完整 Node 基线保留在 `codex/node-baseline`（`78db22d`）。四种发行产物对应同一生产源码快照 `282638a`，随后只增加验证工具和报告。旧 Rust 失败产物、旧报告和最初性能记录分别保留在 `release/history/df2d5b1/`、`release/history/initial-process-measurement/`。本次仅本地构建，没有发布或推送。

## 对照与修复

本地 SVG 优先对照页（运行 `migration/build-comparison.py` 后查看） 的六例与基线使用逐字节相同的 `.lay` 和图像资源，每项展示 SHA-256，并保留修复前 Rust 产物。DejaVu 实际嵌入字体字节相同；Noto 集合拆分仅涉及签名、校验和及共享 CFF 家族名称，字形、映射和布局度量表相同。

- 多轴/断轴：恢复轴文字颜色继承、显式颜色优先规则和图例线、marker、误差方向、间距。
- 统计图：恢复图例 hatch、阶跃默认方向、数组 baseline、缺失行、分箱和 ECDF 的上层契约。
- 等高线：固定 `contour 0.13.1`，使用 f64、关闭 GeoJSON；保留网格中心偏移、非均匀映射、缺失单元、周期接缝。填色采用相邻超水平集合之差，保留孔洞及半透明不重复叠色。
- 几何：保留实例身份与锚点定义，正确重排嵌套组；最终变换下重新细分极坐标曲线、裁剪和纹理。继续使用 kurbo、i_overlay。
- 渲染：修复 RaTeX 横线中心坐标、最终 shaping 越框检查、字体嵌入限制、ICC 到 sRGB 及重复相乘的点透明度。公式 SVG/PDF 保持矢量，LaTeX 源码可提取。
- 编辑：补齐 LCSS/参数诊断、单位/class/返回类型补全、未保存依赖状态、网页动态资源、分项计时和 CLI 参数退出码。

公式保留 RaTeX/KaTeX 原生字形、笔画与度量，仍可能比 MathJax 看起来更小或更细；没有通过放大、加粗来强制相同。已证明正确的圆帽/融合圆弧也保留，旧 PathKit 近似外凸约 0.089258 mm 不作复刻。默认正文由系统或用户字体决定，实际对照另行固定字体。`typography.lay` 底部的 MathJax 字样是原始输入中的普通文字，为保持同一输入而保留；实际公式后端为 RaTeX。

## 包体积

单位为 MiB；解压体积是文件大小总和，不包含文件系统分配开销。

| 项目 | Node 基线 | Rust a1 | Rust a2 | a2 相对 a1 |
| --- | ---: | ---: | ---: | ---: |
| 原生程序 | — | 11.159 | 11.689 | +0.531 |
| Linux wheel，ZIP 压缩 | 96.953 | 5.346 | 5.625 | +0.279 |
| wheel 解压文件总和 | 236.146 | 12.641 | 13.256 | +0.614 |
| VSIX，ZIP 压缩 | — | 5.313 | 5.591 | +0.278 |
| WASM 包，ZIP 压缩 | — | 2.771 | 2.965 | +0.195 |
| WASM 原始模块 | — | 6.489 | 6.949 | +0.460 |

wheel 相对 Node 压缩体积减少 **94.20%**。a2 增量是库接入与全部修复的合计，不能将其当作 contour 单库的独立体积成本。VSIX 解压为 13.201 MiB，WASM 包解压为 8.460 MiB。

程序、WASM、wheel 和 VSIX 内部经 sfnt 字节审计，均只包含锁定 RaTeX 的 **20 个 KaTeX 公式字体，共 513664 字节**；没有正文测试字体或独立字体文件。字体许可证随包提供。正文使用系统或用户字体，缺字发出有位置的 `W_FONT`、直接画矢量方框并继续三格式导出，不自动下载字体。导出文档嵌入实际使用字体与发行包携带正文是不同事项。

## 实际验证

- Node/npm 不在 PATH 的环境中，`--offline --locked` 完成 **284 项 Rust 测试、38 项 Python 测试**及原生/WASM release 构建。唯一默认忽略项是最近边界算法微基准，已单独执行通过；没有忽略功能失败。
- 原 **183 个测试中的 891 个断言调用点全部映射**至 156 个明确测试身份，最终执行门禁通过，缺项为零。891 是静态调用点数，不是动态执行次数。源文件、夹具、映射和测试执行记录均绑定哈希，门禁通过 19 类故障注入。
- 461 次编译输入回放：297 成功、164 个错误及位置全部匹配。217 个图表、158 个输入的强化 Scene 比较为零未解释差异；3 个自动边距变化与 RaTeX 公式度量对应。139 个非图表成功输入中仍有 19 个布局变化，18 个涉及字体/公式度量，1 个为上述真实圆弧边界，单独记录。
- 8 个共享等高线夹具在原生和真实浏览器 WASM 中检查几何及像素，包括孔洞、缺失单元、阈值相等、非均匀网格、周期接缝和半透明色带。最终物理误差有 0.001 mm 的针对性测试，并非所有浮点极限输入的形式化证明。
- 7 个真实生产源码变异在隔离副本中编译后被原回归测试拒绝；另有 17 个绘图、5 个渲染、11 个语言输出变异及几何/透明度、曲线配方、字体包变异检查。输出变异与生产代码变异明确区分。
- 最终 wheel 在仓库外安装，空 PATH 下 38 项测试、77 个原生示例、SVG/PNG/PDF、DataFrame、Matplotlib 和两种真实 IPython magic 通过。
- 真实浏览器完成 74 个示例、7 项编辑状态、7 项语言 UI、7 项 WASM 协议及 8 项等高线场景；缺字体方框、手动字体载入、桌面/移动均检查。实际 VS Code 扩展宿主通过 10 项流程，包括应用快速修复后诊断消失。控制台错误为零，记录绑定最终程序/运行资源哈希。
- 356 个文档页、74 项媒体和 54 个画廊预览已更新并校验。发行打包也在隐藏 Node/npm 的 PATH 中完成。

细项见 [断言与渲染覆盖](render-test-coverage.md)、[修复说明](repair-notes.md)、[机器可读交付记录](delivery.json)。只实际验证 **Linux x86_64（本地 wheel 要求 GLIBC 2.35）与浏览器 WASM**。Linux arm64、macOS x64/arm64、Windows x64 尚未在本机验证；Python 包声明 3.10+，本次实际为 3.13.11。

## 启动、渲染与峰值内存

同一主机、同一输入、各运行 3 次。新旧两版均由轻量独立 `posix_spawnp/wait4` 测量程序运行，旧版来自保存的原始 wheel；修复了原 Python 报告进程历史 RSS 对子进程统计的污染，并用父进程额外分配 160 MiB 的故障注入确认隔离有效。CLI `--help` 启动中位数：**331.25 → 1.35 ms**。

渲染时间包含 CLI 启动、解析、布局和写文件；PNG 为 144 DPI。RSS 是被测 CLI 进程生命周期峰值，取三次最大值，不是渲染函数独占内存。测量器自身启动不计入用时；此为同机实验，不承诺跨机器或硬件隔离的性能。

| 示例 | 格式 | Node ms | Rust ms | Node 峰值 MiB | Rust 峰值 MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| basic.lay | svg | 416.9 | 38.6 | 151.9 | 15.5 |
| basic.lay | png | 448.3 | 51.2 | 171.3 | 19.9 |
| basic.lay | pdf | 451.8 | 36.3 | 163.2 | 16.0 |
| typography.lay | svg | 687.7 | 164.6 | 335.0 | 116.2 |
| typography.lay | png | 821.0 | 104.0 | 423.8 | 51.2 |
| typography.lay | pdf | 682.8 | 97.6 | 250.5 | 48.5 |
| outlines.lay | svg | 452.2 | 47.2 | 174.9 | 14.5 |
| outlines.lay | png | 475.4 | 52.7 | 189.7 | 19.1 |
| outlines.lay | pdf | 477.6 | 50.0 | 175.3 | 15.3 |
| plot/multi-axes-breaks.lay | svg | 448.0 | 51.1 | 167.9 | 15.2 |
| plot/multi-axes-breaks.lay | png | 558.8 | 148.8 | 201.2 | 35.1 |
| plot/multi-axes-breaks.lay | pdf | 478.3 | 52.6 | 172.9 | 16.2 |
| plot/statistics.lay | svg | 470.0 | 77.1 | 175.7 | 15.7 |
| plot/statistics.lay | png | 509.4 | 95.0 | 203.1 | 24.9 |
| plot/statistics.lay | pdf | 494.9 | 76.8 | 186.7 | 16.7 |
| plot/polar-field.lay | svg | 570.9 | 90.2 | 224.7 | 21.2 |
| plot/polar-field.lay | png | 609.3 | 108.2 | 241.4 | 28.1 |
| plot/polar-field.lay | pdf | 663.8 | 99.6 | 224.2 | 21.9 |

原始三次样本及来源/产物哈希见 `node-benchmark.json`、`rust-benchmark.json`。旧 wheel SHA-256 为 `8c73cdb44da0cfbfe31fe650df49cba666f68456f0f8354fdc88873d1866f3ee`；新产物校验和见 `release/dist/SHA256SUMS`。

## 复现

```sh
python scripts/check-native.py
cargo run --offline --locked --release -p laymesh-core --example replay
cargo run --offline --locked --release -p laymesh-core --example compare_geometry
cargo run --offline --locked --release -p laymesh-core --example compare_plots
python migration/test-core-mutants.py
python migration/audit-release-fonts.py
python migration/audit-render-assets.py --binary target/release/laymesh
python migration/benchmark.py rust --runs 3
python migration/benchmark.py node --legacy-wheel /path/to/saved-baseline.whl --runs 3
python migration/build-comparison.py
```

验收需要 Python 测试/Notebook 依赖、Poppler 与 MuPDF；性能测量辅助工具使用系统 C 编译器，不进入发行包。WASM 打包使用与锁定 wasm-bindgen 版本一致的 CLI。浏览器和编辑器脚本见 `scripts/smoke-browser.py`、`test-editor-transitions.py`、`test-browser-language.py`、`test-browser-contours.py`、`smoke-vscode.py`；预览服务应绑定本机的 Tailscale 地址。

Historical captures retain the retired arrow builtin. Current regression tests explicitly migrate their source through `laymesh_core::migration::migrate_arrows`; the runtime does not accept arrow. Fixed-size short heads and continuous cap junctions use the new endpoint contract.
