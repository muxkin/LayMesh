import {languageSupport} from './language.mjs';
import { Workspace } from './workspace.mjs';
import { EditorView } from '@codemirror/view';
import { editingSetup } from './setup.mjs';
import { EditorState, Compartment, ChangeSet } from '@codemirror/state';
import { keymap } from '@codemirror/view';
import { StreamLanguage } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { json } from '@codemirror/lang-json';
const en=document.documentElement.lang==='en', t=(zh,enText)=>en?enText:zh;
const base=new URL('./',import.meta.url);
const theme=EditorView.theme({
 '&':{fontSize:'14px',height:'100%',lineHeight:'1.8',backgroundColor:'var(--code)' ,color:'#dcece7'},
 '.cm-content':{fontFamily:'var(--mono, monospace)',padding:'14px 0',caretColor:'#ffffff'},
 '.cm-gutters':{backgroundColor:'var(--code)',color:'#91aaa2',border:'none'},
 '.cm-activeLine,.cm-activeLineGutter':{backgroundColor:'#ffffff08'},
 '&.cm-focused .cm-selectionBackground,.cm-selectionBackground':{backgroundColor:'#346355 !important'},
 '.cm-scroller':{overflow:'auto',minHeight:'0',height:'100%'},
 '.cm-cursor':{borderLeftColor:'#fff'},
 '.cm-tooltip':{backgroundColor:'#253d35',color:'#e7f3ef',border:'1px solid #536b63'},
},{dark:true});
const colors=HighlightStyle.define([{tag:tags.keyword,color:'#7ccde0'},{tag:tags.function(tags.variableName),color:'#91d6c6'},{tag:tags.propertyName,color:'#a7cdea'},{tag:tags.operator,color:'#aabfba'},{tag:tags.string,color:'#b9dd91'},{tag:tags.number,color:'#e5c48c'},{tag:tags.comment,color:'#8caaa0'}]);
class ExampleEditor {
 constructor(module) {
  this.module=module;this.entry=module.dataset.entry;this.original={};this.files={};this.views=new Map();this.maps=new Map();this.seq=0;this.busy=false;this.pending=false;this.loadedFonts=[];this.last=null;this.activated=false;
  for(const panel of module.querySelectorAll('[data-edit-file]')) { const field=panel.querySelector('.code-raw');const text=field.defaultValue;field.value=text;this.original[panel.dataset.editFile]=text;this.files[panel.dataset.editFile]=text;this.maps.set(panel.dataset.editFile,ChangeSet.empty(text.length).desc); }
  this.fontNote=module.querySelector('.font-note');
  this.status=module.querySelector('.live-status');this.metrics=module.querySelector('.live-metrics');this.diagnostics=module.querySelector('.live-diagnostics');this.static=module.querySelector('.original-preview');this.image=module.querySelector('.live-image');this.toolbar=module.querySelector('.live-toolbar');
  this.workspace=new Workspace(module,offset=>this.editSummary(offset));
  module.querySelector('.diagnostics-toggle').addEventListener('click',()=>{this.diagnosticsOpen=!this.diagnosticsOpen;this.sync();});
  module.querySelector('.live-run').addEventListener('click',()=>{if(module.dataset.liveState==='error'){this.worker?.terminate();this.worker=null;this.busy=false;}this.schedule(true);});
  module.querySelector('.live-reset').addEventListener('click',()=>this.reset());
  module.querySelector('.live-system-fonts').addEventListener('click',()=>this.localFonts());
  module.querySelector('.live-font-file').addEventListener('change',async e=>{for(const file of e.target.files)await this.font(file.name,await file.arrayBuffer());e.target.value='';});
  module.addEventListener('laymesh:tab',()=>this.sync());
  this.sync();
 }
 activate() { if(this.activated)return;this.activated=true;this.boot();this.schedule(true); }
 boot() {
  this.worker?.terminate();clearTimeout(this.timeout);clearTimeout(this.initTimeout);this.busy=false;this.ready=false;
  const start=performance.now();this.worker=new Worker(new URL('worker.js',base),{type:'module'});
  this.worker.postMessage({type:'init',base:base.href});
  this.initTimeout=setTimeout(()=>{this.worker?.terminate();this.worker=null;this.error({message:t('引擎或字体加载超时，可点击运行重试。','Engine or fonts timed out. Run again to retry.')});},45000);
  this.worker.onmessage=({data})=>{
   if(data.type==='init-error'){clearTimeout(this.initTimeout);this.error(data.error);this.worker?.terminate();this.worker=null;return;}
   if(data.type==='ready'){clearTimeout(this.initTimeout);this.ready=true;this.initialization=performance.now()-start;for(const font of this.loadedFonts)this.worker.postMessage({type:'font',...font});if(this.pending)this.drain();return;}
   if(data.type==='font-added'){this.fontNote.textContent=t('内置 DejaVu Sans、Noto Sans CJK SC；另已载入：','Bundled DejaVu Sans and Noto Sans CJK SC; also loaded: ')+(data.families||[]).join(' / ');this.schedule(true);return;}
   if(data.type==='resources'){if(data.id===this.active){if(data.waiting){clearTimeout(this.timeout);if(this.segmentStart!==undefined)this.remaining-=performance.now()-this.segmentStart;this.segmentStart=undefined;}else this.arm();}return;}
   if(data.type==='error'&&typeof data.id==='string'){this.diagnostics.textContent=data.error.message;this.diagnosticsOpen=true;this.sync();return;}
   if(data.id!==this.active)return;
   clearTimeout(this.timeout);this.busy=false;
   if(data.id===this.seq){if(data.type==='result')this.result(data);else this.error(data.error);}
   if(this.pending)this.drain();
  };
  this.worker.onerror=()=>{clearTimeout(this.initTimeout);clearTimeout(this.timeout);this.busy=false;this.error({message:t('预览引擎加载失败，可点击立即运行重试。','Preview engine failed to load. Run again to retry.')});this.worker.terminate();this.worker=null;};
 }
 arm(){clearTimeout(this.timeout);this.segmentStart=performance.now();this.timeout=setTimeout(()=>{this.worker?.terminate();this.worker=null;this.busy=false;this.error({message:t('运行超过 10 秒，已停止。请修改源码或恢复示例。','Execution exceeded 10 seconds and was stopped. Edit or restore the example.')});if(this.pending)this.drain();},Math.max(0,this.remaining));}
 drain(){clearTimeout(this.debounce);this.debounce=setTimeout(()=>{if(!this.busy&&!this.composing)this.run();},Math.max(0,(this.readyAt||0)-performance.now()));}
 schedule(immediate=false){this.readyAt=performance.now()+(immediate?0:250);this.seq++;this.pending=true;this.module.dataset.liveState='pending';this.statusText=t('等待更新…','Waiting to update…');this.sync();clearTimeout(this.debounce);this.changed();if(!this.activated)return;this.drain();}
 run(){if(!this.worker)this.boot();if(!this.ready){this.pending=true;this.statusText=t('正在加载预览引擎…','Loading preview engine…');this.sync();return;}this.pending=false;this.busy=true;this.active=this.seq;this.remaining=10000;this.statusText=t('正在运行…','Running…');this.sync();this.arm();this.worker.postMessage({type:'run',id:this.active,entry:this.entry,files:{...this.files}});}
 changed(){const dirty=Object.keys(this.files).some(f=>this.files[f]!==this.original[f]);this.module.dataset.modified=String(dirty);this.module.querySelector('.live-reset').disabled=!dirty;this.sync();}
 extensions(file,wrap){return [editingSetup,theme,syntaxHighlighting(colors),file.endsWith('.json')?json():/\.(lay|lcss)$/.test(file)?languageSupport(this,file):[],wrap.of(EditorView.lineWrapping),keymap.of([{key:'Mod-Enter',run:()=>{this.schedule(true);return true;}}]),EditorView.domEventHandlers({compositionstart:()=>{this.composing=true;this.seq++;this.pending=false;clearTimeout(this.debounce);},compositionend:()=>{this.composing=false;this.schedule();}}),EditorView.updateListener.of(update=>{if(!update.docChanged)return;this.maps.set(file,this.maps.get(file).composeDesc(update.changes.desc));this.files[file]=update.state.doc.toString();if(!this.composing)this.schedule();else this.changed();})];}
 editor(panel){const file=panel.dataset.editFile;if(this.views.has(file))return;
  const code=panel.querySelector('.code-block');code.getSource=()=>this.files[file];code.querySelector('pre').hidden=true;const host=document.createElement('div');host.className='live-editor';code.append(host);const wrap=new Compartment();
  const view=new EditorView({state:EditorState.create({doc:this.files[file],extensions:this.extensions(file,wrap)}),parent:host});view.contentDOM.setAttribute('aria-label',file);view.contentDOM.setAttribute('spellcheck','false');
  code.querySelector('.wrap-code').addEventListener('click',()=>view.dispatch({effects:wrap.reconfigure(code.classList.contains('wrap')?EditorView.lineWrapping:[])}));this.views.set(file,{view,wrap});
 }
 editSummary(offset){
  this.module.dispatchEvent(new CustomEvent('laymesh:select-source',{bubbles:true}));
  const view=this.views.get(this.entry)?.view;if(!view)return;
  const anchor=this.maps.get(this.entry).mapPos(offset,1);
  view.dispatch({selection:{anchor},effects:EditorView.scrollIntoView(anchor,{y:'center'})});view.focus();this.activate();
 }
 sync(){
  const summary=this.module.dataset.mode==='summary',panel=this.module.querySelector('.source-panel:not([hidden])'),editable=!summary&&!!panel?.dataset.editFile;
  this.toolbar.hidden=!editable;this.module.querySelector('.summary-edit').hidden=!summary;
  if(editable)this.editor(panel);
  this.static.hidden=editable&&!!this.last;this.image.hidden=!editable||!this.last;
  this.module.dataset.previewState=editable?this.module.dataset.liveState:'original';
  this.module.querySelector('.live-modified').textContent=this.module.dataset.modified==='true'?(summary?t('草稿已修改 · 仅本页','Draft modified · this page'):t('已修改 · 仅本页','Modified · this page')):t('修改仅保留在本页','Edits stay on this page');
  this.status.textContent=editable?(this.statusText||t('准备预览','Ready to preview')):t('原始示例','Original example');
  const stale=this.last&&this.last.id!==this.seq;
  this.module.querySelector('.live-total').textContent=editable&&this.last?`${stale?t('上次合计','Previous total'):t('合计','Total')} ${this.last.metrics.total.toFixed(1)} ms`:t('合计 —','Total —');
  this.module.querySelector('.timing-details').hidden=!editable||!this.last;
  const note=this.module.querySelector('.preview-state');
  const pendingDraft=this.module.dataset.liveState==='pending'&&this.module.dataset.modified==='true';
  note.hidden=!editable||(!stale&&this.module.dataset.liveState!=='error'&&!pendingDraft);
  note.textContent=this.module.dataset.liveState==='error'?(this.last?t('上次成功结果','Last successful result'):t('原始示例 · 编辑未通过','Original example · edits failed')):(this.last?t('上次结果 · 等待更新','Previous result · updating'):t('原始示例 · 等待更新','Original example · updating'));
  const diagnostic=editable&&!!this.diagnostics.textContent,button=this.module.querySelector('.diagnostics-toggle');
  button.hidden=!diagnostic;button.textContent=this.module.dataset.liveState==='error'?t('查看错误','View error'):t('诊断','Diagnostics');button.setAttribute('aria-expanded',String(!!this.diagnosticsOpen));
  this.diagnostics.hidden=!diagnostic||!this.diagnosticsOpen;
  this.workspace?.fit();
 }

 result(data){const old=this.last?.url;this.last={...data,url:URL.createObjectURL(new Blob([data.svg],{type:'image/svg+xml'}))};this.image.dataset.aspectRatio=String(data.inspection.page.width/data.inspection.page.height);this.image.src=this.last.url;this.image.dataset.full=this.last.url;this.image.onload=()=>{if(old)URL.revokeObjectURL(old);};this.statusText=t('预览已更新','Preview updated');const m=data.metrics,ms=n=>`${n.toFixed(1)} ms`;this.metrics.textContent=t(`编译 ${ms(m.compile)} · SVG ${ms(m.svg)} · 合计 ${ms(m.total)} · 初始化 ${ms(this.initialization||m.initialization)} · 资源 ${ms(m.resources)}`,`Compile ${ms(m.compile)} · SVG ${ms(m.svg)} · Total ${ms(m.total)} · Initialize ${ms(this.initialization||m.initialization)} · Resources ${ms(m.resources)}`);this.metrics.title=t('本设备本次浏览器实测；不包含输入防抖等待。不是 CLI 基准。','Measured in this browser on this device. Excludes typing debounce; not a CLI benchmark.');this.diagnosticsOpen=data.warnings.length>0;this.diagnostics.textContent=data.warnings.map(w=>`${w.code} ${w.file}:${w.loc.line}:${w.loc.column} — ${w.message}`).join('\n');this.module.dataset.liveState='success';this.sync();}
 error(error){this.statusText=this.last?t('上次成功结果 · 当前源码未通过','Last successful result · current source failed'):t('当前源码未通过','Current source failed');this.diagnosticsOpen=true;this.diagnostics.textContent=[error.code,error.file,error.loc?`${error.loc.line}:${error.loc.column}`:'',error.message].filter(Boolean).join(' ');this.module.dataset.liveState='error';this.sync();}
 reset(restart=true){this.seq++;clearTimeout(this.debounce);this.worker?.terminate();this.worker=null;clearTimeout(this.initTimeout);clearTimeout(this.timeout);this.busy=false;this.pending=false;this.files={...this.original};for(const [file,text]of Object.entries(this.original))this.maps.set(file,ChangeSet.empty(text.length).desc);for(const [file,{view,wrap}]of this.views){view.setState(EditorState.create({doc:this.files[file],extensions:this.extensions(file,wrap)}));this.module.querySelector(`[data-edit-file="${CSS.escape(file)}"] .code-raw`).value=this.files[file];}if(this.last)URL.revokeObjectURL(this.last.url);this.last=null;this.image.removeAttribute('src');this.diagnostics.textContent='';this.diagnosticsOpen=false;this.metrics.textContent='';this.statusText='';this.workspace.zoomTo(1);this.sync();this.changed();if(restart)this.schedule(true);}

 async font(name,bytes,alias){this.activate();const id=`font-${this.loadedFonts.length}`,font={id,name,bytes,alias};this.loadedFonts.push(font);this.worker.postMessage({type:'font',...font});}
 async localFonts(){this.diagnosticsOpen=true;if(!window.queryLocalFonts){this.diagnostics.textContent=t('此浏览器无法读取系统字体；请选择字体文件。未加载的字符显示方框。','This browser cannot read installed fonts. Select a font file; missing glyphs display as boxes.');this.sync();return;}
  try{const fonts=await window.queryLocalFonts();const dialog=document.createElement('dialog');dialog.className='font-dialog';const title=document.createElement('h2');title.textContent=t('选择本机字体系列','Choose an installed font family');const select=document.createElement('select');for(const family of [...new Set(fonts.map(f=>f.family))].sort()){const option=document.createElement('option');option.textContent=family;select.append(option);}const use=document.createElement('button');use.textContent=t('载入所选字体','Load selected family');const close=document.createElement('button');close.textContent=t('取消','Cancel');dialog.append(title,select,use,close);document.body.append(dialog);close.onclick=()=>dialog.close();dialog.onclose=()=>dialog.remove();use.onclick=async()=>{use.disabled=true;for(const f of fonts.filter(f=>f.family===select.value))await this.font(f.postscriptName,await(await f.blob()).arrayBuffer(),f.family);dialog.close();};dialog.showModal();}catch{this.diagnostics.textContent=t('未获得系统字体访问权限。可选择字体文件；缺字显示方框。','Installed-font access was not granted. Select a font file; missing glyphs display as boxes.');this.sync();}}
}
const editors=[...document.querySelectorAll('.example-module[data-entry]')].map(module=>new ExampleEditor(module));
const observer=new IntersectionObserver(entries=>{for(const entry of entries)if(entry.isIntersecting){editors.find(e=>e.module===entry.target)?.activate();observer.unobserve(entry.target);}},{rootMargin:'160px'});editors.forEach(e=>observer.observe(e.module));
addEventListener('pageshow',e=>{if(e.persisted)editors.forEach(editor=>editor.reset());});

addEventListener('pagehide',()=>editors.forEach(editor=>{editor.loadedFonts=[];editor.fontNote.textContent=t('预览内置 DejaVu Sans、Noto Sans CJK SC；也可载入自己的字体。','Preview bundles DejaVu Sans and Noto Sans CJK SC; you can also load your own fonts.');editor.reset(false);editor.workspace.resetView();}));
