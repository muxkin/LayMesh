# v006：默认普通放映，手动安排演讲窗口

2026-10-14。按用户最新选择，移除自动多屏策略，保留手动演讲者视图。旧 v003、v004、v005 与原附件未改写；不发布或重打版本标签。

## 当前行为

- 主“放映”从当前页进入普通全屏，不因显示器数量、连接方式或屏幕权限状态打开控制台。
- 菜单“演讲者视图”显式打开控制台，原窗口保留观众画面；默认窗口模式。用户拖动窗口到目标屏幕后，在观众窗口点击“进入全屏”或按 F。
- 不读取 `screen.isExtended`、不调用 `getScreenDetails()`、不查询或申请 `window-management` 权限、不调用窗口 `moveTo()` / `resizeTo()`，也不在弹窗参数中指定屏幕坐标。移除屏幕选择、交换和断屏后的自动处置。
- 控制台的全屏命令只提示用户前往观众窗口点击，不经消息自动申请全屏。全屏失败与弹窗失败分别恢复，连接成功不会清除未处理的全屏错误。
- 保留当前页启动、单次动画渲染与位图镜像、备注、计时、100% 完成进度及退出恢复。兼容旧 `mode: 'auto'` 调用，将其视作普通放映。

## 验证

Linux，Playwright Chromium 151.0.7922.34、Firefox 153.0、WebKit 26.5，另测系统 Edge 146.0.3856.84。弹窗拦截保持开启。Browser 插件未提供可调用的 Node REPL 执行接口，沿用项目 Playwright 测试，未安装新依赖。

测试路径：当前预览页 → 默认放映 → 单窗口全屏；退出 → 菜单演讲者视图 → 两个窗口 → 观众窗口手动全屏 → 退出恢复。

| 检查 | 结果 |
|---|---|
| 默认入口：单屏、扩展虚拟屏、复制标志、权限拒绝、接口缺失 | Chromium、Linux Edge 均通过；屏幕读取／权限查询／窗口移动调用全部为零，默认弹窗次数为零 |
| 页面身份、非空页面、无错误覆盖层、控制台异常 | 离线三引擎通过；无应用异常，无 HTTP 依赖 |
| 手动控制台、弹窗拦截与重开、全屏拒绝与重试 | 三引擎通过；手动入口初始不申请全屏；后续全屏调用均来自观众窗口点击 |
| 关闭控制台、退出全屏、结束放映恢复 | 三引擎通过；观众页码和退出后预览页码正确 |
| 镜像、备注、计时、黑白屏、静态页、点击步骤 | 三引擎通过；控制台不加载第二套 CanvasKit；镜像在途帧至多 1 帧 |
| 桌面与窄窗口 | 1672×941、1366×768、600×900 检查通过；手动摆放提示可见，无横向溢出 |
| 附件实际六页自然播放、减少动态效果 | Chromium 通过；最后一页在 6 秒结束，原总时长仍为 7 秒，完成进度 100%；镜像像素平均差约 0.46/255 |
| Tailscale HTTP 入口 | 默认全屏、手动演讲入口及零屏幕管理调用通过 |
| VS Code 原生外部浏览器服务 | 当前页默认全屏、显式演讲者入口、页面准备／缓存和退出回归通过 |

未验证 Windows 物理双屏拖动、操作系统复制／扩展切换、真实 GPU 和 Windows VS Code 桌面。这次不主张自动落屏能力，也不将虚拟显示器测试当作物理双屏测试。

截图由测试保存在 `/tmp/laymesh-presenter-v006/`：`natural/presenter.png` 显示实际六页作品及完成进度；`engines/chromium-600.png` 为窄窗口；`manual-console.png` 为手动窗口模式。截图未写入离线作品。

## 体积与交付

v006 为 **4,179,561 字节**，比 v005 减少 **3,616 字节（3.53 KiB）**。按现有保守口径，播放版导出、控制台、页面预览、界面与许可证的累计功能预算为 **260,979 字节（254.86 KiB）**，低于 300 KiB。具体作品相对 v002 增量为 **217.06 KiB**；作品差值与模块预算分别记录，不混用。

离线作品：`video/04-LCSS与动画/exports/v006/04_LCSS_离线播放_v006.zip.html`。

Tailscale 预览：<http://100.64.0.6:8774/preview/04-lcss-v006.zip.html>。

记录：[启动](presenter-playback/v006/startup.json)、[Linux Edge](presenter-playback/v006/edge-startup.json)、[三引擎](presenter-playback/v006/presenter-results.json)、[六页自然播放](presenter-playback/v006/natural.json)、[Tailscale 网页](presenter-playback/v006/http-startup.json)、[原生外部服务](presenter-playback/v006/external.json)、[体积和校验值](presenter-playback/v006/size.json)、[功能预算](presenter-playback/v006/export-budget.json)。

## 重现

```sh
python scripts/build-web-export.py --skip-wasm
python scripts/build-docs-live.py --skip-wasm
python scripts/build-editors.py --binary target/release/laymesh
node scripts/test-presentation-clock.mjs
node scripts/test-presentation-session.mjs
node scripts/test-presenter-mirror.mjs
node scripts/test-presenter-startup.cjs PATH_TO_V006.zip.html /tmp/startup.json
node scripts/test-presenter.cjs /tmp/presenter-qa
LAYMESH_BROWSERS=chromium node scripts/test-presenter-natural.cjs PATH_TO_V006.zip.html /tmp/natural
node scripts/test-presentation-host.cjs /tmp/served
python scripts/check-web-export-budget.py
python scripts/build-docs-media.py --check
```

浏览器测试使用现有 `PLAYWRIGHT_MODULE` 路径；Edge 入口测试通过 `LAYMESH_BROWSER_EXECUTABLE=/usr/bin/microsoft-edge` 选择浏览器。`--skip-wasm` 适用于对应 Rust/WASM 已构建且源码未变化；本次只修改浏览器模块、构建清理和测试说明。
