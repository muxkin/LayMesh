const $ = (id) => document.getElementById(id);
const STATUS = {rendered:"已渲染",font_glyph_missing:"字体缺字",parse_rejected:"解析不支持",policy_rejected:"禁止命令",invalid_geometry:"尺寸异常"};
let data, filtered = [], detailCase;
const state = {suite:"all",query:"",status:"all",style:"display",size:24,limit:20,page:1,fonts:[],baselines:false,textFallback:true};
const element = (tag, cls, text) => {const node=document.createElement(tag);if(cls)node.className=cls;if(text!==undefined)node.textContent=text;return node;};
const format = (value) => value.toLocaleString("zh-CN");
const variants = (c) => data.renders[c.render_index][state.textFallback?"text_variants":"variants"];
const suiteLabel = (id) => data.meta.suites.find(s=>s.id===id)?.label || id;
const positionLabel = (c) => c.position_kind==="text" ? `第 ${c.position} 行` : `第 ${c.position} 条`;
const sourceURL = (c) => c.upstream_path ? `${data.meta.upstream.repository}/blob/${data.meta.upstream.commit}/${c.upstream_path}${c.position_kind==="text"?`#L${c.position}`:""}` : `sources/${c.fixture}`;

function loadState(){
  const q=new URLSearchParams(location.search);
  if(data.meta.suites.some(s=>s.id===q.get("suite")))state.suite=q.get("suite");
  state.query=q.get("q") || "";
  if(["failed","fallback"].includes(q.get("status")) || STATUS[q.get("status")])state.status=q.get("status");
  if(q.get("style")==="inline")state.style="inline";
  const numeric=(key,min,max,fallback)=>{const n=Number(q.get(key));return q.has(key)&&Number.isFinite(n)?Math.min(max,Math.max(min,Math.round(n))):fallback;};
  state.size=numeric("size",16,72,24);state.page=numeric("page",1,10000,1);
  if([20,40,100].includes(Number(q.get("limit"))))state.limit=Number(q.get("limit"));
  const chosen=(q.get("fonts") || "").split(",");state.fonts=data.meta.fonts.filter(f=>chosen.includes(f.id)).map(f=>f.id);
  if(!state.fonts.length)state.fonts=data.meta.fonts.map(f=>f.id);
  state.baselines=q.get("baselines")==="1";
  state.textFallback=q.get("text")!=="0";
}
function saveState(){
  const q=new URLSearchParams();
  if(state.suite!=="all")q.set("suite",state.suite);
  if(state.query)q.set("q",state.query);
  if(state.status!=="all")q.set("status",state.status);
  if(state.style!=="display")q.set("style",state.style);
  if(state.size!==24)q.set("size",state.size);
  if(state.page!==1)q.set("page",state.page);
  if(state.limit!==20)q.set("limit",state.limit);
  if(state.fonts.length!==data.meta.fonts.length)q.set("fonts",state.fonts.join(","));
  if(state.baselines)q.set("baselines","1");
  if(!state.textFallback)q.set("text","0");
  history.replaceState(null,"",location.pathname+(q.size?"?"+q:""));
}
function selectedFonts(){return data.meta.fonts.filter(f=>state.fonts.includes(f.id));}
function filterCases(){
  const query=state.query.trim().toLowerCase();
  filtered=data.cases.filter(c=>{
    if(state.suite!=="all"&&c.suite!==state.suite)return false;
    if(query && !`${c.source} ${c.id} ${c.label} ${c.fixture} ${suiteLabel(c.suite)} ${c.position}`.toLowerCase().includes(query))return false;
    const results=state.fonts.map(f=>variants(c)[f][state.style]);
    return state.status==="all" || (state.status==="rendered"?results.every(r=>r.status==="rendered"&&!r.placeholder_glyphs&&!r.preview_error):
      state.status==="failed"?results.some(r=>r.status!=="rendered"||r.placeholder_glyphs||r.preview_error):
      state.status==="invalid_geometry"?results.some(r=>r.preview_error):
      state.status==="fallback"?results.some(r=>r.text_fallback||r.placeholder_glyphs):
      state.status==="font_glyph_missing"?results.some(r=>r.status==="font_glyph_missing"||r.placeholder_glyphs):results.some(r=>r.status===state.status));
  });
}
function errorContent(result){
  const node=element("div","error-cell");node.append(element("div","error-title",result.preview_error?"尺寸异常":STATUS[result.status] || "渲染失败"),element("p","error-message",result.preview_error || result.error),element("div","error-code",`${result.preview_error_code || result.code} · 已记录的限制`));return node;
}
function imageFor(result,size,alt){
  const img=element("img");img.src=result.svg;img.alt=alt;img.loading="lazy";img.decoding="async";
  img.style.width=`${result.width*size/5}px`;img.style.height=`${result.height*size/5}px`;
  return img;
}
function renderRows(){
  filterCases();
  const pages=Math.max(1,Math.ceil(filtered.length/state.limit));state.page=Math.min(pages,state.page);
  const start=(state.page-1)*state.limit, entries=filtered.slice(start,start+state.limit), fonts=selectedFonts();
  const head=element("tr");head.append(element("th",null,"公式源码"));
  for(const font of fonts)head.append(element("th",null,font.label));
  $("table-head").replaceChildren(head);
  $("comparison").style.minWidth=`${Math.max(600,270+fonts.length*180)}px`;
  const fragment=document.createDocumentFragment();
  for(const c of entries){
    const row=element("tr");row.dataset.caseId=c.id;
    const sourceCell=element("td"), title=element("div","case-title",`${suiteLabel(c.suite)} · ${String(c.ordinal).padStart(3,"0")}`);
    const link=element("a",null,positionLabel(c));link.href=sourceURL(c);link.target="_blank";link.rel="noreferrer";title.append(link);sourceCell.append(title);
    if(c.label)sourceCell.append(element("p","case-label",c.label));
    sourceCell.append(element("pre","source-code",c.source));
    const detail=element("button","detail-button","查看详情 / 放大");detail.addEventListener("click",()=>openDetail(c));sourceCell.append(detail);row.append(sourceCell);
    const results=fonts.map(font=>variants(c)[font.id][state.style]);
    const successful=results.filter(r=>r.status==="rendered"&&!r.preview_error);
    const ascent=Math.max(0,...successful.map(r=>r.ascent))*state.size/5;
    const depth=Math.max(0,...successful.map(r=>r.height-r.ascent))*state.size/5;
    for(let i=0;i<fonts.length;i++){
      const result=results[i], cell=element("td");
      if(result.status!=="rendered"||result.preview_error)cell.append(errorContent(result));
      else {
        const stage=element("div",`formula-stage${state.baselines?" show-baseline":""}`);
        stage.style.height=`${Math.max(72,ascent+depth+24)}px`;stage.style.setProperty("--baseline",`${12+ascent}px`);
        const img=imageFor(result,state.size,`${fonts[i].label}：${c.source}`);img.style.top=`${12+ascent-result.ascent*state.size/5}px`;stage.append(img);
        stage.title="长公式可横向滚动，也可点击详情放大";
        if(result.invisible)stage.append(element("span","invisible-note","不可见构造 / 仅占位"));
        const status=element("div",`render-status${result.placeholder_glyphs?" placeholder":""}`,result.placeholder_glyphs?"缺字占位 · 已导出":result.text_fallback?"已渲染 · 文本回退":result.bounds_errors.length?"已渲染 · 边界记录见详情":"已渲染");
        const svgLink=element("a",null,"SVG ↗");svgLink.href=result.svg;svgLink.target="_blank";svgLink.rel="noreferrer";svgLink.setAttribute("aria-label",`打开 ${fonts[i].label} 原始 SVG`);status.append(svgLink);cell.append(stage,status);
      }
      row.append(cell);
    }
    fragment.append(row);
  }
  if(!entries.length){const row=element("tr"),cell=element("td","empty","没有匹配用例。试试其他关键词或筛选条件。");cell.colSpan=fonts.length+1;row.append(cell);fragment.append(row);}
  $("rows").replaceChildren(fragment);
  for(const stage of $("rows").querySelectorAll(".formula-stage"))if(stage.scrollWidth>stage.clientWidth+1)stage.after(element("p","scroll-note","↔ 横向滚动 / 详情放大"));
  $("result-count").textContent=`${format(filtered.length)} 条匹配 · 显示 ${filtered.length?start+1:0}–${Math.min(start+state.limit,filtered.length)}`;
  $("page-info").textContent=`第 ${state.page} 页 / ${pages} 页`;
  $("page-jump").value=state.page;$("page-jump").max=pages;$("previous").disabled=state.page===1;$("next").disabled=state.page===pages;
  $("inventory").textContent=`${format(data.meta.case_entries)} 条公式 · ${fonts.length} 套字体 · 2 种模式`;
  $("size-output").value=`${state.size}px`;
  $("size").style.setProperty("--progress",`${(state.size-16)/56*100}%`);
  for(const button of document.querySelectorAll("[data-style]"))button.setAttribute("aria-pressed",String(button.dataset.style===state.style));
  for(const button of document.querySelectorAll("[data-text]"))button.setAttribute("aria-pressed",String((button.dataset.text==="1")===state.textFallback));
  for(const button of document.querySelectorAll("[data-suite]")){if(button.dataset.suite===state.suite)button.setAttribute("aria-current","page");else button.removeAttribute("aria-current");}
  saveState();document.body.dataset.ready="true";
}
function openDetail(c){
  detailCase=c;$("detail-title").textContent=`${suiteLabel(c.suite)} · ${c.id}`;
  $("detail-label").textContent=`${c.fixture} / ${positionLabel(c)}${c.label?" · "+c.label:""} · ${state.style==="display"?"独立公式":"行内公式"}`;
  $("detail-source").textContent=c.source;$("copy-source").textContent="复制源码";
  $("original-source").href=sourceURL(c);
  $("detail-size").value=Math.max(48,state.size);renderDetail();$("detail-dialog").showModal();
}
function renderDetail(){
  const size=Number($("detail-size").value);$("detail-size-output").value=`${size}px`;
  $("detail-size").style.setProperty("--progress",`${(size-24)/104*100}%`);
  const fragment=document.createDocumentFragment();
  for(const font of selectedFonts()){
    const result=variants(detailCase)[font.id][state.style],section=element("section","detail-font"),heading=element("h3",null,font.label);section.append(heading);
    if(result.status!=="rendered"||result.preview_error)section.append(errorContent(result));
    else {
      const link=element("a",null,"下载 SVG");link.href=result.svg;link.download=`${detailCase.id}-${font.id}-${state.style}.svg`;heading.append(link);
      const viewport=element("div","detail-image");viewport.append(imageFor(result,size,`${font.label}：${detailCase.source}`));section.append(viewport);
      if(result.invisible)section.append(element("p","error-message",`此构造没有可见轮廓。原布局宽度 ${result.layout_width.toFixed(4)} mm，高度 ${result.layout_height.toFixed(4)} mm；负宽度可表示负间距。`));
      section.append(element("div","render-status","已渲染"));
      if(result.text_fallback)section.append(element("p",null,`文本字体：${result.fallback_families.join("、")}。使用所选正文列表排版，数学字形保持当前数学字体。`));
      for(const face of result.text_fonts || [])section.append(element("p","font-metadata",`${face.family} · ${face.face} · 字重 ${face.weight}${face.italic?" · 斜体":""} · ${face.glyph_count} 个字形 / ${face.character_count} 个字符`));
      if(result.placeholder_glyphs)section.append(element("p","error-message",`${result.placeholder_glyphs} 个字符以缺字方框占位，并非正确字形。`));
      for(const warning of result.warnings)section.append(element("p","error-message",`${warning.code}：${warning.message}`));
      if(result.bounds_errors.length)section.append(element("pre","source-code",`原布局边界记录（预览包含完整轮廓）：\n${result.bounds_errors.join("\n")}`));
    }
    fragment.append(section);
  }
  $("detail-renders").replaceChildren(fragment);
}
async function copySource(){
  try{
    if(navigator.clipboard&&window.isSecureContext)await navigator.clipboard.writeText(detailCase.source);
    else{const field=element("textarea");field.value=detailCase.source;field.style.position="fixed";field.style.left="-9999px";$("detail-dialog").append(field);field.select();const copied=document.execCommand("copy");field.remove();if(!copied)throw Error("copy unavailable");}
    $("copy-source").textContent="已复制";
  }catch{$("copy-source").textContent="请选中源码复制";}
}
function provenance(){
  const meta=data.meta,content=$("provenance-content");content.replaceChildren();
  for(const text of [
    `RaTeX ${meta.upstream.tag} · 提交 ${meta.upstream.commit}`,
    `保留 ${meta.upstream.files.length} 个原始公式语料文件的 ${format(meta.upstream_entries)} 条非空、非注释用例，包括各套文件中的重复条目。加上 ${meta.case_entries-meta.upstream_entries} 条领域与文本补充，共 ${format(meta.case_entries)} 条。原始文件字节与 SHA-256 均已核验。`,
    `${format(meta.distinct_formulas)} 条不同公式，四套数学字体、两种公式模式、两种文本策略。${format(meta.layout_checks)} 个条目结果，检查 ${format(meta.distinct_layout_checks)} 个不同组合，生成 ${format(meta.svg_files)} 个 SVG；其余结果保留失败原因。重复公式共用相同 SVG。`,
    `禁用文本回退时，其中 ${format(meta.audit_checks)} 个不同组合与固定审计逐项一致；另 ${format(meta.supplemental_checks)} 个组合涵盖新增用例及启用文本回退的完整语料，并检查几何。解析不支持和禁止命令保留错误；字体缺字统一方框占位并标出警告，不当作正确字形。`,
    `使用当前 LayMesh 公式排版及 SVG 导出生成矢量轮廓。浏览器调整显示尺寸；字体字形及数学排版来自生成时的渲染器。`,
    `“启用文本回退”使用固定的 Noto 派生测试字体列表（Latin、CJK、Arabic、Indic），在数学排版前完成文本塑形和尺寸测量；“仅数学字体”关闭正文回退，用于核对数学字体的字形覆盖。正文测试字体不会随产品运行库打包。`,
    `快照生成于 ${new Date(meta.generated_at).toLocaleString("zh-CN")} · LayMesh 基于 ${meta.git_revision.slice(0,7)}`,
  ])content.append(element("p",null,text));
  content.append(element("h3",null,"原始用例与校验值"));const list=element("ul","source-list");
  for(const file of meta.upstream.files){const item=element("li"),link=element("a",null,file.path);link.href=`sources/${file.file}`;link.target="_blank";link.rel="noreferrer";item.append(link,element("br"),element("code",null,file.sha256));list.append(item);}content.append(list);
  const license=element("a",null,"RaTeX MIT 许可证");license.href="sources/LICENSE";license.target="_blank";content.append(license);
  content.append(element("h3",null,"正文测试字体与校验值"));
  for(const font of meta.text_font_fixtures)content.append(element("p",null,`${font.family} ${font.style} · ${font.sha256}`));
  const fontManifest=element("a",null,"正文测试字体来源与派生记录");fontManifest.href="sources/text-fonts-manifest.json";fontManifest.target="_blank";content.append(fontManifest);
  $("provenance-dialog").showModal();
}
function initialize(){
  loadState();
  for(const suite of [{id:"all",label:"全部用例",count:data.meta.case_entries},...data.meta.suites]){
    const button=element("button","category");button.dataset.suite=suite.id;button.append(element("span",null,suite.label),element("span",null,String(suite.count)));
    button.addEventListener("click",()=>{state.suite=suite.id;state.page=1;renderRows();});$("categories").append(button);
  }
  for(const font of data.meta.fonts){
    const label=element("label"),checkbox=element("input");checkbox.type="checkbox";checkbox.checked=state.fonts.includes(font.id);checkbox.value=font.id;
    checkbox.addEventListener("change",()=>{const chosen=[...$("font-options").querySelectorAll("input:checked")].map(i=>i.value);if(!chosen.length){checkbox.checked=true;return;}state.fonts=chosen;state.page=1;renderRows();});label.append(checkbox,document.createTextNode(font.label));$("font-options").append(label);
  }
  $("search").value=state.query;$("status").value=state.status;$("size").value=state.size;$("page-size").value=state.limit;$("baselines").checked=state.baselines;
  $("filters").addEventListener("submit",e=>e.preventDefault());
  let searchTimer;$("search").addEventListener("input",()=>{clearTimeout(searchTimer);searchTimer=setTimeout(()=>{state.query=$("search").value;state.page=1;renderRows();},120);});
  $("status").addEventListener("change",()=>{state.status=$("status").value;state.page=1;renderRows();});
  for(const button of document.querySelectorAll("[data-style]"))button.addEventListener("click",()=>{state.style=button.dataset.style;state.page=1;renderRows();});
  for(const button of document.querySelectorAll("[data-text]"))button.addEventListener("click",()=>{state.textFallback=button.dataset.text==="1";state.page=1;renderRows();});
  $("size").addEventListener("input",()=>{state.size=Number($("size").value);renderRows();});
  $("page-size").addEventListener("change",()=>{state.limit=Number($("page-size").value);state.page=1;renderRows();});
  $("baselines").addEventListener("change",()=>{state.baselines=$("baselines").checked;renderRows();});
  function go(page){state.page=Math.max(1,Math.min(Number($("page-jump").max),page));renderRows();$("table-scroll").scrollIntoView({block:"start",behavior:"smooth"});}
  $("previous").addEventListener("click",()=>go(state.page-1));$("next").addEventListener("click",()=>go(state.page+1));
  $("page-jump").addEventListener("change",()=>{const value=Number($("page-jump").value);if(Number.isFinite(value))go(Math.round(value));});
  $("detail-size").addEventListener("input",renderDetail);$("copy-source").addEventListener("click",copySource);
  $("provenance-button").addEventListener("click",provenance);
  for(const dialog of document.querySelectorAll("dialog")){dialog.querySelector("[data-close]").addEventListener("click",()=>dialog.close());dialog.addEventListener("click",event=>{if(event.target===dialog){const r=dialog.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)dialog.close();}});}
  $("snapshot-note").textContent=`完整条目保留重复用例。快照 ${new Date(data.meta.generated_at).toLocaleString("zh-CN")} · 可按原始文件位置逐项核对。`;
  renderRows();
}
try{
  const response=await fetch("data.json");if(!response.ok)throw Error(`HTTP ${response.status}`);data=await response.json();initialize();
}catch(error){$("inventory").textContent="测试集加载失败";$("result-count").textContent="请刷新页面重试";$("rows").replaceChildren();const row=element("tr"),cell=element("td","empty",String(error));row.append(cell);$("rows").append(row);console.error(error);}
