# 构建与发布到 PyPI

**0.3.9 已发布**：增加双端点直线连接和允许省略几何的 `line()` 素材，Python、Rust 引擎和 VS Code 扩展统一版本号。五个平台 wheel 和正式版 VSIX 构建、Python 3.10/3.13/3.14 检查及完整契约测试均通过。[验证与发布记录](https://github.com/muxkin/LayMesh/actions/runs/37400436659)。

当前版本为 **[`0.3.9`](https://pypi.org/project/laymesh/0.3.9/)**，已发布五个平台 wheel。默认安装包含 NumPy、pandas、Matplotlib 和 IPython，使用 `pip install laymesh` 即可获得全部 Python 功能。[发布工作流](https://github.com/muxkin/LayMesh/actions/workflows/publish-pypi.yml)构建和验证五个平台 wheel，并执行包审计、校验和检查与 Trusted Publishing 上传。每个平台 wheel 包含 Python API、CLI 入口、一个 Rust 原生程序、依赖许可及构建清单。公式字体编译进 RaTeX，正文使用系统或用户字体。

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
python scripts/build-python-wheel.py --output release/dist/pypi-0.3.9
python scripts/check-python-wheels.py release/dist/pypi-0.3.9 --checksums release/dist/pypi-0.3.9/SHA256SUMS
python -m twine check --strict release/dist/pypi-0.3.9/*.whl
```

构建器先检查版本一致性，再编译锁定依赖的 release 引擎，在源码树以外暂存并打包。独立输出目录避免混入旧包。Linux 标签依据二进制实际 GLIBC 符号要求生成，下限为 2.28；新系统构建不能宣称旧系统兼容。本地 Linux x64 wheel 要求 glibc 2.35+；CI runner 生成的包可能需要更新的 glibc，请查看实际文件名与清单。

清单记录 Python/Rust 版本、平台、原生程序 SHA-256 与 Cargo.lock SHA-256。包审计检查元数据、平台标签、执行权限、哈希、许可文本及本机目录泄漏；拒绝 Node/npm、JS 运行文件与正文字体。锁定依赖变化后必须重新收集许可，CI 每次构建均执行此步骤。

## 验证安装后的 wheel

另建一个**干净的虚拟环境**，安装生成的 wheel，不使用可编辑源码安装。下面的文件名应替换为本机实际生成的包：

```sh
python -m pip install "release/dist/pypi-0.3.9/laymesh-0.3.9-py3-none-manylinux_2_35_x86_64.whl" pillow
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
3. 将最终改动同步到 `main`，手动运行 **Python wheels and PyPI**，设置 `publish=false` 验证候选包。检查 `reviewed-python-release` 中的五个平台 wheel 与校验和。
4. 真正上传时，在 `main` 手动运行同一工作流，设置 `publish=true`、`version=0.3.9`。它会重新构建并测试、检查指定版本与校验和，再等待 environment 审核；请审核该次运行生成的包。

发布任务下载审核过的同一批文件，不重新构建。PR 和普通构建运行不上传。发布后在仓库以外验证索引安装：`python -m pip install "laymesh==0.3.9"`，再运行 `python -m pip check` 与 `python -m laymesh --version`。已发布版本不能覆盖；需要替换时同步递增 Python、Rust、runtime 与 VS Code 扩展版本。

## 验证记录

`python scripts/test-contracts.py` 会生成 `release/verification/cargo-tests.log`、`python-tests.log` 与 `assertion-coverage.json`，检查当前源码、测试执行与历史断言映射的一致性。检查期间需安装 Poppler 和 MuPDF 命令行工具。发布候选的跨平台结果以 GitHub Actions 当前提交的运行记录为准；最终包与校验和保存在 `reviewed-python-release` artifact 中。

## VS Code 自动发布

[Publish VS Code Marketplace](../.github/workflows/publish-marketplace.yml) 在 `main` 的 PyPI 发布成功后自动运行。它下载原运行的五个平台 VSIX、审核 wheel 和校验和，检查来源提交、版本、哈希及内置引擎一致性，再使用固定版本的 `@vscode/vsce` 4.0.0 发布正式版。商店校验完成后，会核对五个平台和 VSIX 哈希；重试会跳过已发布的平台，同一版本的不同包会报错。

在仓库的 [Actions Secrets](https://github.com/muxkin/LayMesh/settings/secrets/actions) 或 `marketplace` 环境中配置一次 `VSCE_PAT` Secret。令牌需具有 Marketplace Manage 权限，其账户必须能访问现有 [Hyacine 发布者](https://marketplace.visualstudio.com/manage/publishers/Hyacine)。流程默认使用 `pat`，配置后无需逐次手动上传 VSIX。[微软发布指南](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)说明了令牌创建方法及全局 PAT 于 2026 年 12 月 1 日退役的安排。通过 GitHub 界面或 `gh secret set VSCE_PAT` 保存凭据，不要写进源码或发布日志。

补发当前 0.3.9，或在产物保留期内恢复部分成功的发布：

```sh
gh workflow run publish-marketplace.yml --ref main \
  -f run_id=37400436659 -f version=0.3.9 -f publish=true -f auth=pat
```

设置 `publish=false` 可执行仅审计运行。失败、fork、PR 以及跳过 PyPI 上传的运行不能作为发布来源；流程复用原包，不重新构建或递增版本。

[可信发布](https://github.com/microsoft/vscode-vsce#trusted-publishing)保留为后续无需存储 PAT 的可选方式。[2026 年 10 月 6 日实发尝试](https://github.com/muxkin/LayMesh/actions/runs/37404048392)已通过来源、产物及 OIDC 请求协议检查，但商店返回 `Trusted Publishing is not supported`。待商店支持该发布者后，可授权所有者 `muxkin`、仓库 `LayMesh`、`main` 上的 `publish-marketplace.yml` 和环境 `marketplace`；手动运行选择 `auth=oidc`，自动运行在该环境设置变量 `MARKETPLACE_AUTH=oidc`。OIDC 模式固定使用包含[微软协议修复](https://github.com/microsoft/vscode-vsce/commit/c960f2e97da3360899f2bfe93390fa4890c69327)的 4.0.1-3，并在使用前验证请求协议；认证失败不会自动改用 PAT。

## 其他发行物

原生 VS Code 扩展使用 `python scripts/build-editors.py` 构建，`python scripts/package-editor.py --output release/dist/laymesh.vsix` 打包，再用 `python scripts/check-editor.py release/dist/laymesh.vsix` 审计。Rust/WASM 文档站使用 `python scripts/build-docs.py` 构建。发布工作流同时生成上述五个平台的 VSIX，扩展、Python 包和引擎必须使用同一版本号；VSIX 与 PyPI wheel 分开发放。

若修改 Rust 源码、Cargo 版本或锁文件，还需使用当前引擎运行 `python scripts/build-docs-media.py --binary target/release/laymesh`，随后运行 `python scripts/build-docs.py` 与 `python scripts/build-docs.py --check`。提交重新生成的 `site/media` 图片和清单；文档 CI 会拒绝与渲染器指纹不一致的旧缓存。
