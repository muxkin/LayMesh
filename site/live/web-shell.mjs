import { iconLabel } from "./presentation-icons.mjs";
export function buildShell(root, editable) {
  root.innerHTML = `
<header class="app-header"><button id="workspace-tree" title="文件工作区" aria-label="文件工作区"></button><div class="wordmark"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3v18h18M7 3v14h14M11 3v10h10"/></svg><b>LayMesh</b></div><span class="header-divider"></span><strong class="project-name"></strong><div class="header-actions"><button id="fonts">字体</button><button id="export" class="primary">导出</button></div></header>
<div id="font-guide" hidden><span id="font-status"></span><span>编辑文字前可连接本机字体，或导入字体文件。</span><button id="font-guide-open">管理字体</button><button id="dismiss-fonts" aria-label="关闭字体提示">×</button></div>
<main><aside class="workspace-tree" ${editable ? "" : "hidden"}><div class="tree-heading"><span>资源管理器</span><button id="import-button" title="导入文件" aria-label="导入文件"></button><input id="import" type="file" multiple hidden></div><div id="tree" role="tree" aria-label="项目文件"></div><div class="tree-hint">拖放文件以导入资源</div></aside><div class="splitter tree-split" role="separator" tabindex="0" aria-label="调整文件树宽度" aria-orientation="vertical" ${editable ? "" : "hidden"}></div>
<section class="edit-pane" ${editable ? "" : "hidden"}><div class="file-tabs source-files" role="tablist" aria-label="打开的文件"></div><div id="panels"><div id="empty-editor">从文件树打开文件</div></div></section><div class="splitter editor-split" role="separator" tabindex="0" aria-label="调整编辑器宽度" aria-orientation="vertical" ${editable ? "" : "hidden"}></div>
<section class="preview-pane"><div class="preview-toolbar"><button id="fit" title="适应页面 (0)" aria-label="适应页面"></button><select id="zoom" aria-label="预览缩放"><option value="fit">适应</option><option value=".25">25%</option><option value=".5">50%</option><option value="1">100%</option><option value="2">200%</option><option value="4">400%</option></select><span class="tool-separator"></span><select id="page" aria-label="页面"></select><div class="toolbar-end"><button id="full" class="present-button">放映</button><details id="preview-more"><summary title="更多预览选项" aria-label="更多预览选项">•••</summary><div class="popover"><button id="windowed">窗口放映</button><button id="rulers" aria-pressed="true">显示标尺</button><button id="static">完整排版</button><button id="replay">重播当前页</button><label>播放模式<select id="mode" aria-label="播放模式"><option value="manual">手动</option><option value="auto">自动</option></select></label><label>动画时间<input id="seek" type="range" min="0" step="0.001" aria-label="动画时间"></label><span id="time"></span><p>Ctrl / ⌘ + 滚轮缩放<br>空格 + 拖动或中键平移</p></div></details></div></div><section class="stage" tabindex="0" aria-label="作品预览"><canvas aria-label="LayMesh 画面"></canvas><div class="page-outline"></div><canvas class="ruler horizontal" aria-hidden="true"></canvas><canvas class="ruler vertical" aria-hidden="true"></canvas><span class="ruler-corner">mm</span></section><div class="preview-footer"><button id="previous" title="上一步" aria-label="上一步"></button><button id="play" title="播放" aria-label="播放"></button><button id="next" title="下一步" aria-label="下一步"></button><span id="preview-state">完整排版</span><span id="coordinates">—</span></div></section></main>
<footer class="app-status"><span id="status" role="status">正在准备显示资源…</span><span class="status-format">${editable ? "可编辑工作区" : "作品预览"}</span></footer>
<dialog id="export-panel"><div class="dialog-heading"><h2>导出作品</h2><button class="dialog-close" aria-label="关闭导出">×</button></div><h3 class="export-package-heading">离线作品</h3><p class="export-note">保留页面、动画与资源，下载后可离线打开。</p><label id="embed-label"><input id="embed-fonts" type="checkbox"> 包含使用的系统字体</label><div class="export-actions"><button id="save" class="primary" ${editable ? "" : "hidden"}>下载可编辑文件</button><button id="download">下载播放版</button><button id="zip">下载标准 ZIP</button></div><span class="export-note">默认文件名 .zip.html · 本地播放，无需联网</span></dialog>
<dialog id="font-panel"><div class="dialog-heading"><h2>作品字体</h2><button class="dialog-close" aria-label="关闭字体面板">×</button></div><div class="font-body"><p class="font-notice" role="status" tabindex="-1">连接本机字体需浏览器授权。未授权仍可播放预编译画面。</p><div class="font-actions"><button id="connect-fonts" class="primary">连接本机字体</button><button id="font-import">导入字体文件</button><input id="font-file" type="file" multiple accept=".ttf,.otf,.ttc,.otc" hidden></div><div class="font-replacement-heading"><span>原字体</span><span>替换为</span></div><p class="font-replacement-note">在右侧选择替代字体，点击“确认并应用”后生效；留空则保留原字体。</p><div class="font-list"></div></div><div class="font-bottom"><span class="font-summary"></span><button id="font-later">暂不处理</button><button id="font-apply" class="primary">确认并应用</button></div></dialog>`;
  const el = (id) => root.querySelector("#" + id);
  for (const [id, icon, label] of [
    ["workspace-tree", "folder"],
    ["import-button", "add"],
    ["fit", "fit"],
    ["full", "window-play", "放映"],
    ["fonts", "font", "字体"],
    ["export", "export", "导出"],
    ["previous", "previous"],
    ["play", "play"],
    ["next", "next"],
  ])
    iconLabel(el(id), icon, label);
  el("fonts").hidden = !editable;
  el("workspace-tree").hidden = !editable;
  el("embed-label").hidden = !editable;
  el("export").onclick = () => el("export-panel").showModal();
  el("export-panel").querySelector(".dialog-close").onclick = () =>
    el("export-panel").close();
  el("font-guide-open").onclick = () => el("fonts").click();
  el("workspace-tree").onclick = () => root.classList.toggle("tree-open");
  for (const [selector, pane] of [
    [".tree-split", ".workspace-tree"],
    [".editor-split", ".edit-pane"],
  ]) {
    const handle = root.querySelector(selector),
      target = root.querySelector(pane);
    let start;
    handle.onpointerdown = (e) => {
      start = {
        vertical:
          matchMedia("(max-width:700px)").matches && pane === ".edit-pane",
        x: e.clientX,
        y: e.clientY,
        width: target.getBoundingClientRect().width,
        height: target.getBoundingClientRect().height,
      };
      handle.setPointerCapture(e.pointerId);
      root.classList.add("resizing");
    };
    handle.onpointermove = (e) => {
      if (start) {
        target.style.flexBasis =
          (start.vertical
            ? Math.max(
                120,
                Math.min(
                  root.querySelector("main").clientHeight * 0.75,
                  start.height + e.clientY - start.y,
                ),
              )
            : Math.max(
                160,
                Math.min(
                  root.clientWidth * 0.7,
                  start.width + e.clientX - start.x,
                ),
              )) + "px";
      }
    };
    handle.onpointerup = () => {
      start = null;
      root.classList.remove("resizing");
    };
    handle.onpointercancel = handle.onpointerup;
    handle.onkeydown = (e) => {
      const vertical =
          matchMedia("(max-width:700px)").matches && pane === ".edit-pane",
        keys = vertical
          ? ["ArrowUp", "ArrowDown"]
          : ["ArrowLeft", "ArrowRight"];
      if (keys.includes(e.key)) {
        e.preventDefault();
        target.style.flexBasis =
          Math.max(
            120,
            target.getBoundingClientRect()[vertical ? "height" : "width"] +
              (e.key === keys[0] ? -20 : 20),
          ) + "px";
      }
    };
  }
  return el;
}
