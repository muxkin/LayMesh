# LayMesh 0.6.5 发布兼容性验证

本轮将已完成的增量编译、逐页放映与资源复用整理为正式版本，重点检查 Python、Jupyter、VS Code 和离线文件兼容性。性能数据沿用[首次编辑测量](first-edit-2026-10-16.zh-CN.md)，本轮并行执行的正确性检查不作为延迟测量。

## 本机验证

- Rust 工作区通过 674 项测试；另以优化构建通过超过 16 MiB 的排版缓存生命周期测试。一个原有手动几何基准未执行。
- Python 源码与独立安装的 wheel 各通过 67 项测试；32 项迁移检查通过，891 个旧断言位置全部映射且执行验证通过。
- 使用真实 Jupyter 内核分别执行源码和安装包，每本 Notebook 连续执行两遍。验证共享 LCSS 改色、`.laydoc` 重排/删除/恢复、资源错误恢复、相对路径、数据及 Matplotlib 更新、JPEG 与原生像素一致、PDF/PNG 导出和旧配置兼容。
- 安装包在无系统 Node、空 PATH 条件下完成 255 个原生示例入口检查；10 个 PPTX 示例及 VS Code 导出桥接通过。
- Chromium、Firefox、WebKit 均验证最终导出的依赖索引恢复、首次局部修改只编译相关页、阻塞无关页时当前页仍可开播，以及保存和导出等待最新全稿。
- VS Code 真实窗口验证中文输入法、连续编辑、旧响应丢弃、错误恢复、页面重排、动画、导出和资源传输。特意阻断 Worker 对扩展资源的直接请求，预览仍正常。19 组 JavaScript 契约检查通过。
- 最终独立 HTML 三浏览器冷启动连续改色均只编译封面；六页场景及每页三个动画采样与 v009 一致，保存重开与播放版导出通过。[冷启动](release-0.6.5/final-html-cold.json)、[场景与往返](release-0.6.5/final-html-roundtrip.json)。
- 百页缩略图回归保留 67,781,280 字节解码图像，超过原 32 MiB 配额后返回第一页仍复用原 URL，关闭文档后全部释放。旧测试中“必须淘汰至 32 MiB 以下”的断言已改为当前生命周期契约。[缓存证据](release-0.6.5/thumbnail-cache.json)。
- 发布 CI 的演讲者双窗口回归发现适配层把新增 `navigation-visible` 通知误判为上下文丢失，清空首帧镜像并暂停播放。已按事件类型分别处理，并保留控制台连接前完成的首帧；三浏览器双窗口回归重新通过。[演讲者证据](release-0.6.5/presenter-regression.json)。

本机平台为 Linux x64、Python 3.13；其他平台和 Python 版本以正式 CI 结果为准。证据：[本机汇总](release-0.6.5/local-verification.json)、[源码 Jupyter](release-0.6.5/jupyter-source.json)、[安装包 Jupyter](release-0.6.5/jupyter-installed.json)、[VS Code](release-0.6.5/vscode-regression.json)、[三浏览器](release-0.6.5/incremental-preview.json)、[断言覆盖](release-0.6.5/assertion-coverage.json)。

## 发布门槛

合并与正式标签发布继续使用仓库现有工作流：三系统验收、五平台 wheel 与同引擎 VSIX、真实 Jupyter、Python 3.10/3.14 兼容性、许可证和包审计。PyPI 使用审核过的构建产物；发布完成后核对公开文件摘要。Marketplace 保持手动上传。

本地原始日志、执行后的 Notebook 和截图保存在 `release/verification/0.6.5/`；大型离线附件及历史运行时保持本地，不作为文档网站下载资源。
