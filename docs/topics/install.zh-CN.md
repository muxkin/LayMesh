# 安装与验证

## 安装平台 wheel

要求 Python 3.10+。当前发布候选版本为 alpha 预发布 `0.3.0a2`。首次 PyPI 上传尚未完成；上传后使用：

```sh
python -m pip install --pre laymesh
python -m laymesh --version
```

发布前可按[发布流程](../../release/README.zh-CN.md)构建 wheel，再使用 `python -m pip install /path/to/laymesh-<version>-<tags>.whl` 安装生成的具体文件。请选择匹配操作系统、架构及 Linux glibc 版本的包。[画廊](../sections/examples.zh-CN.md)中的仓库示例需要另行获取，pip 不安装示例目录。

| 用途 | 发布后的安装命令 |
| --- | --- |
| CLI、原生图表、Python API 与 IPython Magic | `python -m pip install --pre laymesh` |
| NumPy / pandas 数据绑定 | `python -m pip install --pre "laymesh[data]"` |
| 导入 Matplotlib Figure | `python -m pip install --pre "laymesh[plot]"` |
| 两者都需要 | `python -m pip install --pre "laymesh[data,plot]"` |

请安装到 Notebook 内核使用的 Python 环境。在 Notebook 中可用 `%pip install --pre "laymesh[data,plot]"` 指定当前内核环境。JupyterLab 等 Notebook 应用需另行安装；原生绘图无需 Matplotlib。

## 平台与字体

| 构建目标 | 要求 |
| --- | --- |
| Windows x64 | 64 位 Windows 与 x64 Python |
| macOS Intel / Apple Silicon | macOS 14+，Python 架构匹配 |
| Linux x64 / arm64 | glibc 不低于 wheel 的 `manylinux_2_XX` 标签 |

本地实测覆盖 Linux x64，wheel 标记为 `manylinux_2_35_x86_64`，要求 glibc 2.35+。CI 配置构建五个平台，并检查 Python 3.10、3.13、3.14；工作流已配置不代表远端检查已通过。pip 能安装的平台以实际上传并验证的 wheel 为准。当前没有 Alpine/musl、32 位系统和 Windows ARM64 构建目标。

wheel 包含一个 Rust 原生程序、公式字体及依赖许可。绘图时不下载引擎或字体，无需额外安装 Rust、Node.js 或 TeX。正文使用系统或用户字体；缺字会警告并显示方框。共享可复现图件时请提供明确的字体文件。文档网页预览单独提供自己的字体。

## 首次运行

将下列源码保存为 `figure.lay`：

```lay
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
p = plot(size=(110 mm, 80 mm),
         x=axis(label="Time (s)"), y=axis(label="Signal"))
p.line(x=[0, 1, 2, 3], y=[1, 3, 2, 4], label="Experiment")
p.legend(position="top_left")
page.add(p, offset=(5 mm, 5 mm))
```

```sh
python -m laymesh validate figure.lay
python -m laymesh inspect figure.lay --json
python -m laymesh render figure.lay -o figure.pdf
python -m laymesh render figure.lay -o figure.png --dpi 300
```

`laymesh` 与 `python -m laymesh` 调用同一引擎。`validate` 检查源码和素材；`render` 检查后生成文件。错误返回非零状态，并说明文件、行、列及原因。`inspect --json` 输出尺寸、坐标映射、变换和诊断。使用 `--warnings show|hide` 或 `LAYMESH_WARNINGS` 控制警告；隐藏警告仍保留错误和检查 JSON 中的诊断。

## 源码开发

需要 Rust 1.93.1；构建脚本需要 Python 3.11+。在仓库根目录运行：

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python[data,plot]'
python -m laymesh render examples/basic.lay -o basic.pdf
```

直接运行原生程序时使用 `target/release/laymesh`（Windows 为 `target/release/laymesh.exe`），或 `cargo run --release --locked -p laymesh-cli --`。可编辑安装使用仓库中的引擎，不内置它。开发时可用 `LAYMESH_CLI` 指定其他原生程序。

[基本概念](concepts.zh-CN.md) · [第一张布局](first-layout.zh-CN.md) · [Python API](python-reference.zh-CN.md) · [Notebook](notebook.zh-CN.md)
