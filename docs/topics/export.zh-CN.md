# 导出格式与分辨率

## 操作流程

```text
laymesh validate <file.lay>
laymesh inspect <file.lay> --json
laymesh render <file.lay> -o <output.svg|pdf|png> [--dpi <positive-number>]
```

仓库中用 `cargo run --release --locked -p laymesh-cli --` 作命令前缀。`validate` 完成语法、名称、单位、导入、图片/字体、字形覆盖和放置约束检查；`render` 先做相同检查再写文件。`--dpi` **只用于 PNG**，默认 96。错误以 `文件:行:列: E_代码: 原因` 报告并返回非零状态；`W_FONT`、`W_MATH_FONT`、`W_PLOT_LAYOUT` 等警告写到 stderr，但验证仍可成功。CLI 参数错误退出码为 2，布局或导出错误为 1。

输出区别：SVG 保留可编辑路径与普通 `<text>`；PDF 保留矢量图形并嵌入普通文字，公式额外存放原 LaTeX 源码；PNG 从同一 Scene 栅格化。SVG 输入经过白名单检查，脚本、外链、DTD、滤镜及未支持的特效会以 `E_SVG` 拒绝。PNG/JPEG/TIFF 输入规范化为 sRGB PNG 后进入 Scene；TIFF 限单页 8 位灰度或 RGB。[实测输出和错误示例](../examples-and-results.zh-CN.md)给出了完整复现命令。

逐项源码、命令和结果见[分类画廊](../gallery/README.zh-CN.md)。

`inspect --json` 提供绘图区几何、映射、变换和全部诊断。每个命令都支持 `--warnings show|hide`，优先于环境变量 `LAYMESH_WARNINGS`（默认 `show`）。隐藏警告不影响错误和检查 JSON 中的诊断。

## 限制与相关主题

[CLI 使用](cli.zh-CN.md) · [检查与警告](diagnostics.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
