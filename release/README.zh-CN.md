# 构建与发布到 PyPI

**0.4.0 已发布**：增加双端点直线连接和允许省略几何的 `line()` 素材，Python、Rust 引擎和 VS Code 扩展统一版本号。五个平台 wheel 和正式版 VSIX 构建、Python 3.10/3.13/3.14 检查及完整契约测试均通过。[验证与发布记录](https://github.com/muxkin/LayMesh/actions/runs/37400436659)。

当前版本为 **[`0.4.0`](https://pypi.org/project/laymesh/0.4.0/)**，已发布五个平台 wheel。默认安装包含 NumPy、pandas、Matplotlib 和 IPython，使用 `pip install laymesh` 即可获得全部 Python 功能。[发布工作流](https://github.com/muxkin/LayMesh/actions/workflows/publish-pypi.yml)构建和验证五个平台 wheel，并执行包审计、校验和检查与 Trusted Publishing 上传。每个平台 wheel 包含 Python API、CLI 入口、一个 Rust 原生程序、依赖许可及构建清单。公式字体编译进 RaTeX，正文使用系统或用户字体。

[runtime.json](runtime.json) 定义五个目标：Linux x64 / arm64、macOS 14+ Intel / Apple Silicon、Windows x64。使用者需要 Python 3.10+；构建脚本需要 Python 3.11+ 和 Rust 1.93.1。本次只发布平台 wheel；`python/` 目录不包含完整的 Rust 引擎源码构建链，不要上传仅从该目录生成的 sdist。

发布前检查会拒绝 Python、Rust、runtime 和扩展版本不一致。工作流同时构建上述五个平台的 VSIX 并审计原生程序架构；扩展内置引擎与对应 wheel 的二进制哈希完全一致。PyPI 上五个文件的 SHA-256 已核对为该次审核包的原始校验和。

## 本地构建与审计

使用虚拟环境安装固定版本的构建工具，在仓库根目录运行：

```sh
python -m pip install -r release/build-requirements.txt
python scripts/check-release.py
python -m pip install -e "./python" pillow
python scripts/test-contracts.py
python scripts/collect-licenses.py
python scripts/build-python-wheel.py --output release/dist/pypi-0.4.0
python scripts/check-python-wheels.py release/dist/pypi-0.4.0 --checksums release/dist/pypi-0.4.0/SHA256SUMS
python -m twine check --strict release/dist/pypi-0.4.0/*.whl
```

构建器先检查版本一致性，再编译锁定依赖的 release 引擎，在源码树以外暂存并打包。独立输出目录避免混入旧包。Linux 标签依据二进制实际 GLIBC 符号要求生成，下限为 2.28；新系统构建不能宣称旧系统兼容。本地 Linux x64 wheel 要求 glibc 2.35+；CI runner 生成的包可能需要更新的 glibc，请查看实际文件名与清单。

清单记录 Python/Rust 版本、平台、原生程序 SHA-256 与 Cargo.lock SHA-256。包审计检查元数据、平台标签、执行权限、哈希、许可文本及本机目录泄漏；拒绝 Node/npm、JS 运行文件与正文字体。锁定依赖变化后必须重新收集许可，CI 每次构建均执行此步骤。

## 验证安装后的 wheel

另建一个**干净的虚拟环境**，安装生成的 wheel，不使用可编辑源码安装。下面的文件名应替换为本机实际生成的包：

```sh
python -m pip install "release/dist/pypi-0.4.0/laymesh-0.4.0-py3-none-manylinux_2_35_x86_64.whl" pillow
python -m pip check
python -m laymesh --version
python scripts/smoke-python-wheel.py --examples examples
python -m unittest discover -s python/tests -v
```

冒烟测试切换到临时目录并清空 PATH，通过内置引擎检查 SVG/PDF/PNG/JPEG/TIFF/WebP 等导出、CLI 与模块入口、数据保存、Matplotlib 导入、两种 Notebook Magic 及独立示例。本机检查仅证明实测平台通过。工作流在 Python 3.13 上构建五个平台，并在 Python 3.10 和 3.14 上安装同一批 wheel 验证兼容性。

## GitHub 与 PyPI 配置

现有[发布工作流](../.github/workflows/publish-pypi.yml)使用 [PyPA 的 Trusted Publishing 流程](https://packaging.python.org/en/latest/guides/publishing-package-distribution-releases-using-github-actions-ci-cd-workflows/)。首次上传前：

1. 在 PyPI 配置 pending trusted publisher：项目 `laymesh`、所有者 `muxkin`、仓库 `LayMesh`、工作流 `publish-pypi.yml`、环境 `pypi`。
2. 在 GitHub 仓库创建 `pypi` environment 并配置 required reviewers。工作流会检查人工审核门禁是否存在。
3. 将已验证的提交同步到 `main`，创建并推送稳定标签：`git tag v0.4.0`、`git push origin v0.4.0`。
4. 审核本次运行的 wheel、VSIX 和校验和，再批准 `pypi` 环境；上传成功后自动创建 GitHub Release。

发布任务下载审核过的同一批文件，不重新构建。PR 和普通构建运行不上传。发布后在仓库以外验证索引安装：`python -m pip install "laymesh==0.4.0"`，再运行 `python -m pip check` 与 `python -m laymesh --version`。已发布版本不能覆盖；需要替换时同步递增 Python、Rust、runtime 与 VS Code 扩展版本。

## 验证记录

`python scripts/test-contracts.py` 会生成 `release/verification/cargo-tests.log`、`python-tests.log` 与 `assertion-coverage.json`，检查当前源码、测试执行与历史断言映射的一致性。检查期间需安装 Poppler 和 MuPDF 命令行工具。发布候选的跨平台结果以 GitHub Actions 当前提交的运行记录为准；最终包与校验和保存在 `reviewed-python-release` artifact 中。

## VS Code 手动发布与 GitHub Release

稳定版本标签（例如 `v0.4.0`）触发五平台 wheel 和 VSIX 构建、审计与 Python 兼容性检查；通过现有 `pypi` 环境人工审批后自动上传 PyPI，再创建带中英文更新说明、十个发行包及 `SHA256SUMS` 的 GitHub Release。标签必须与全部版本元数据一致，且对应提交属于 `main`。PR 和手动启动的工作流只验证，不发布。

Marketplace 自动上传工作流已移除。下载 Release 中五个平台的 `.vsix`，通过 [Hyacine 发布者门户](https://marketplace.visualstudio.com/manage/publishers/Hyacine) 手动上传。也可在本地使用 `vsce publish --packagePath <已审核包.vsix>`；凭据只保存在本机，不需要 Actions Marketplace Secret。

`python scripts/marketplace-release.py audit` 保留为发行包审计工具。GitHub Release 重试使用同一批审核产物，核对已上传附件的 SHA-256；拒绝覆盖同版本不同内容。更新说明位于 `release/notes/<版本>.md`。重试时只重新运行原流程中失败的发布任务（`gh run rerun <run-id> --failed`），继续下载同一批审核产物；不要为已上传版本重新构建并替换包。

## 其他发行物

原生 VS Code 扩展使用 `python scripts/build-editors.py` 构建，`python scripts/package-editor.py --output release/dist/laymesh.vsix` 打包，再用 `python scripts/check-editor.py release/dist/laymesh.vsix` 审计。Rust/WASM 文档站使用 `python scripts/build-docs.py` 构建。发布工作流同时生成上述五个平台的 VSIX，扩展、Python 包和引擎必须使用同一版本号；VSIX 与 PyPI wheel 分开发放。

若修改 Rust 源码、Cargo 版本或锁文件，还需使用当前引擎运行 `python scripts/build-docs-media.py --binary target/release/laymesh`，随后运行 `python scripts/build-docs.py` 与 `python scripts/build-docs.py --check`。提交重新生成的 `site/media` 图片和清单；文档 CI 会拒绝与渲染器指纹不一致的旧缓存。
