import {PlaybackStats} from './playback-stats.mjs';
import {PreviewPlayer} from './preview-player.mjs';
import {PreviewControls} from './preview-controls.mjs';
import {presentationEntry} from './presentation-entry.mjs';
import {Presentation} from './presentation.mjs';
import {uiIcon,iconLabel} from './presentation-icons.mjs';
import {webPresentationProvider} from './presentation-web.mjs';
import { Workspace } from './workspace.mjs';
import {setupLibraryCatalog} from './library-catalog.mjs';
const en=document.documentElement.lang==='en', t=(zh,enText)=>en?enText:zh;
const base=new URL('./',import.meta.url);
class ExampleEditor {
 constructor(module) {
  this.module=module;this.entry=module.dataset.entry;this.original={};this.files={};this.views=new Map();this.maps=new Map();this.seq=0;this.busy=false;this.pending=false;this.loadedFonts=[];this.last=null;this.activated=false;
  for(const panel of module.querySelectorAll('[data-edit-file]')) { const field=panel.querySelector('.code-raw');const text=field.defaultValue;field.value=text;this.original[panel.dataset.editFile]=text;this.files[panel.dataset.editFile]=text;}
  this.fontNote=module.querySelector('.font-note');this.startup=module.querySelector('.preview-startup');
  this.status=module.querySelector('.live-status');this.metrics=module.querySelector('.live-metrics');this.diagnostics=module.querySelector('.live-diagnostics');this.static=module.querySelector('.original-preview');this.image=module.querySelector('.live-image');this.toolbar=module.querySelector('.live-toolbar');
  this.diagnosticPanel=document.createElement('div');this.diagnosticPanel.className='live-diagnostics-panel';this.diagnosticPanel.hidden=true;this.diagnostics.before(this.diagnosticPanel);this.diagnosticPanel.append(this.diagnostics);const heading=document.createElement('div');heading.className='diagnostic-heading';this.diagnosticTitle=document.createElement('strong');const close=document.createElement('button');close.type='button';close.setAttribute('aria-label',t('关闭诊断','Close diagnostics'));close.append(uiIcon('close'));close.onclick=()=>{this.diagnosticsOpen=false;this.sync();};heading.append(this.diagnosticTitle,close);this.diagnosticPanel.prepend(heading);this.diagnosticNote=document.createElement('p');this.diagnosticNote.className='diagnostic-note';heading.after(this.diagnosticNote);
  for(const [label,action]of [[t('复制','Copy'),()=>{const field=document.createElement('textarea');field.value=this.diagnostics.textContent;this.diagnosticPanel.append(field);field.select();document.execCommand('copy');field.remove();}],[t('跳转源码','Go to source'),async()=>{const error=this.currentError;if(!error)return;const file=error.file?.replace(/^\//,'')||this.entry;module.dispatchEvent(new CustomEvent('laymesh:select-source',{bubbles:true}));const tab=[...module.querySelectorAll('.source-files [role=tab]')].find(e=>e.title===file);tab?.click();const panel=module.querySelector('.source-panel:not([hidden])');if(!panel)return;await this.editor(panel);const view=this.views.get(file)?.view;if(view){const line=view.state.doc.line(Math.min(error.loc?.line||1,view.state.doc.lines));view.dispatch({selection:{anchor:Math.min(line.to,line.from+(error.loc?.column||1)-1)}});view.focus();}}],[t('重试','Retry'),()=>this.retry()]]){const button=document.createElement('button');button.type='button';button.textContent=label;button.onclick=action;this.diagnosticPanel.append(button);}
  this.playbackStats=new PlaybackStats();
  this.player=new PreviewPlayer(module.querySelector('.preview-toolbar').parentElement,()=>{if(this.player?.running)this.playbackStats.refresh(performance.now());this.wasRunning=!!this.player?.running;this.previewControls?.sync();this.requestPaint(this.player?.running);},{statusInterval:100,page:direction=>{const n=(this.pageSelect?.selectedIndex??-1)+direction;if(!this.pageSelect||n<0||n>=this.pageSelect.options.length)return false;this.pageSelect.selectedIndex=n;this.pageSelect.onchange();return true;}});const motion=document.createElement('details');motion.className='preview-motion-menu';const trigger=document.createElement('summary');trigger.textContent=t('播放控制','Playback controls');motion.append(trigger,this.player.bar);module.querySelector('.preview-toolbar').append(motion);
  const canvas=document.createElement('canvas');canvas.className=this.image.className;canvas.hidden=true;canvas.setAttribute('role','img');canvas.setAttribute('aria-label','LayMesh');this.image.replaceWith(canvas);this.image=canvas;
  this.overview=document.createElement('canvas');this.overview.className='live-overview';this.overview.hidden=true;this.overview.setAttribute('aria-hidden','true');this.overview.style.pointerEvents='none';canvas.before(this.overview);
  this.cancelTransition=()=>{this.worker?.postMessage({type:'cancel-transition'});this.mainSkia?.transitions.cancel();};
  this.player.motion.addEventListener('change',this.cancelTransition);
  new ResizeObserver(this.cancelTransition).observe(this.module.querySelector('.preview-viewport')||this.image.parentElement);
  this.module.addEventListener('laymesh:draw',()=>this.requestPaint());
  this.image.parentElement.addEventListener('wheel',this.cancelTransition,{passive:true});
  this.image.parentElement.addEventListener('pointerdown',this.cancelTransition);
  this.workspace=new Workspace(module);this.previewControls=new PreviewControls(this);
  const catalog=module.closest('.animation-catalog');
  if(catalog){
   this.effectSelect=catalog.querySelector('.effect-select');
   const effects=JSON.parse(catalog.querySelector('.effect-data').textContent);
   const describe=()=>{const effect=effects.find(e=>e.name===this.effectSelect.value);catalog.querySelector('.effect-description').textContent=effect?t(effect.description[0],effect.description[1]):'';};
   describe();
   this.effectSelect.addEventListener('change',async()=>{
    const selection=this.effectSelect.value;
    if(!effects.some(e=>e.name===selection))return;
    this.effectChange=(this.effectChange||0)+1;const revision=this.effectChange;this.effectPreparing=true;
    this.player.action('static');this.playAfterPaint=false;this.module.dataset.playRequested='false';describe();
    const panel=module.querySelector('[data-edit-file="'+this.entry+'"]');
    module.dispatchEvent(new CustomEvent('laymesh:select-source',{bubbles:true}));
    await this.editor(panel);
    if(revision!==this.effectChange)return;
    const view=this.views.get(this.entry)?.view;
    if(!view){this.effectPreparing=false;return;}
    const current=view.state.doc.toString(),effectClass=new RegExp('animate__(?:'+effects.map(e=>e.name).join('|')+')\\b');
    const source=(effectClass.test(current)?current:this.original[this.entry]).replace(effectClass,'animate__'+selection);
    view.dispatch({changes:{from:0,to:view.state.doc.length,insert:source}});
    this.activate();this.schedule(true);this.effectPreparing=false;
   });
   catalog.querySelector('.effect-play').addEventListener('click',()=>{
    this.module.dataset.playRequested='true';this.activate();
    if(!this.effectPreparing&&this.module.dataset.liveState==='success'&&this.presented&&this.player.metadata){
     this.module.dataset.playRequested='false';this.player.action('replay');
    }
   });
   catalog.querySelector('.effect-static').addEventListener('click',()=>{this.playAfterPaint=false;this.module.dataset.playRequested='false';this.player.action('static');});
   module.querySelector('.live-reset').addEventListener('click',()=>{this.effectChange=(this.effectChange||0)+1;this.effectPreparing=false;this.effectSelect.value='fadeInUp';describe();});
  }
  module.querySelector('.diagnostics-toggle').addEventListener('click',()=>{this.diagnosticsOpen=!this.diagnosticsOpen;this.sync();});
  module.querySelector('.live-run').addEventListener('click',()=>this.retry());
  module.querySelector('.startup-retry').addEventListener('click',()=>this.retry());
  module.addEventListener('laymesh:play',()=>{this.activate();if(this.presented&&this.player.metadata){this.module.dataset.playRequested='false';this.player.action('replay');}});
  module.querySelector('.live-reset').addEventListener('click',()=>this.reset());
  module.querySelector('.live-system-fonts').addEventListener('click',()=>this.localFonts());
  module.querySelector('.live-font-file').addEventListener('change',async e=>{for(const file of e.target.files)await this.font(file.name,await file.arrayBuffer());e.target.value='';});
  module.addEventListener('laymesh:tab',()=>this.sync());
  this.presentation=new Presentation({base,getStartPageId:()=>this.pageId,provider:webPresentationProvider(this,base),onEnter:()=>{this.presentation.parent=this.workspace.dialog||document.body;this.previewResume=this.player.running;if(this.previewResume)this.player.action('play');this.previewView={zoom:this.workspace.zoom,pan:{...this.workspace.pan}};},onExit:()=>{if(this.previewView){Object.assign(this.workspace,this.previewView);this.workspace.fit();}if(this.previewResume)this.player.start();}});window.addEventListener('pagehide',()=>this.presentation.dispose(),{once:true});const previewToolbar=module.querySelector('.preview-toolbar');previewToolbar.append(presentationEntry(this.presentation));const options=document.createElement('details');options.className='preview-options-menu';const optionsTrigger=document.createElement('summary');optionsTrigger.setAttribute('aria-label',t('更多','More'));optionsTrigger.append(uiIcon('more'));const popover=document.createElement('div');popover.className='preview-options-popover';const zoomControls=module.querySelector('.preview-zoom');previewToolbar.querySelector('h2').after(module.querySelector('.preview-fit'));popover.append(module.querySelector('.preview-backgrounds'),module.querySelector('.preview-enlarge'),zoomControls,motion);options.append(optionsTrigger,popover);previewToolbar.append(options);options.addEventListener('toggle',()=>{if(options.open)module.querySelectorAll('details[open]:not(.dependencies)').forEach(other=>{if(other!==options&&!options.contains(other))other.open=false;});});module.querySelector('.workspace-status').prepend(module.querySelector('.diagnostics-toggle'));this.sync();
  setupLibraryCatalog(this,module);
 }
 retry(){if(this.sourceLoadError){location.reload();return;}if(!this.ready||!this.worker){this.worker?.terminate();this.worker=null;this.busy=false;}if(this.fullFontState==='error')this.fullFontState='idle';this.activate();this.schedule(true);}
 ensureFullFonts(){if(!this.ready||this.fullFontState!=='idle')return;this.fullFontState='loading';this.module.dataset.fullFonts='loading';this.worker.postMessage({type:'full-fonts'});}
 activate() { if(this.activated)return;this.activated=true;this.boot();this.schedule(true); }
 boot() {
  this.mainSkia?.dispose();this.mainSkia=null;this.worker?.terminate();clearTimeout(this.timeout);clearTimeout(this.initTimeout);this.busy=false;this.ready=false;this.fullFontState='idle';this.module.dataset.fullFonts='idle';
  this.paintInFlight=null;this.pendingPaint=null;
  clearTimeout(this.paintTimer);this.paintTimer=0;
  const start=performance.now();this.worker=new Worker(new URL('worker.js',base),{type:'module'});
  // A transferred canvas cannot be transferred again after a worker restart.
  const canvas=this.image.cloneNode(false);this.image.replaceWith(canvas);this.image=canvas;
  const offscreen=canvas.transferControlToOffscreen?.();this.directCanvas=!!offscreen;
  this.worker.postMessage({type:'init',base:base.href,canvas:offscreen},offscreen?[offscreen]:[]);
  this.initTimeout=setTimeout(()=>{this.worker?.terminate();this.worker=null;this.error({message:t('引擎或字体加载超时，可点击运行重试。','Engine or fonts timed out. Run again to retry.')});},45000);
  this.worker.onmessage=async({data})=>{
   if(data.type==='context'){this.module.dataset.renderBackend=data.ready?'Skia WebGL2':'Skia restoring';if(data.ready){this.paintKey='';this.presented=null;this.requestPaint();}return;}
   if(data.type==='skia-scene'){this.mainSkiaReady=(async()=>{const {initializeSkia,SkiaRenderer}=await import('./skia-renderer.mjs');if(!this.mainSkia){const meta=await(await fetch(new URL('skia/manifest.json',base))).json();this.mainSkia=new SkiaRenderer(await initializeSkia(new URL('skia/'+meta.wasm,base).href),this.image,{onContextChange:ready=>{if(ready){this.paintKey='';this.presented=null;this.requestPaint();}}});}await this.mainSkia.install(data.list,id=>data.resources[id],data.generation,data.navigation);})();return;}
   if(['paint-error','painted','paint-skipped'].includes(data.type)){
    const request=this.paintInFlight&&data.id===this.paintInFlight.id&&data.generation===this.paintInFlight.generation?this.paintInFlight:null;
    if(request)this.paintInFlight=null;
    if(data.generation===this.last?.generation&&request){
     if(data.type==='paint-error'&&request.key===this.paintKey){this.paintKey='';this.error(data.error);}
     else if(data.type==='painted'&&(request.key===this.paintKey||this.pendingPaint)){if(!this.directCanvas){await this.mainSkiaReady;const metrics=this.mainSkia.render(request.rect,data.width,data.height,data.states,data.generation,!!request.running);if(metrics.skipped)return;Object.assign(metrics,await this.mainSkia.transitions.play(this.player.reduced));Object.assign(data,metrics);}this.module.dataset.renderBackend=data.backend;this.module.dataset.transitionName=data.transitionName||'';this.module.dataset.transitionFrames=String(data.transitionFrames||0);this.module.dataset.transitionError=data.transitionError||'';this.module.dataset.renderWidth=String(data.width);this.module.dataset.renderHeight=String(data.height);this.module.dataset.sampleMs=String(data.sampleMs);this.module.dataset.gpuMs=data.gpuMs==null?'':String(data.gpuMs);
      const p=this.last.inspection.page;
      if(request.time===null&&request.rect[0]===0&&request.rect[1]===0&&request.rect[2]===p.width&&request.rect[3]===p.height){const s=Math.min(768/p.width,2048/p.height,Math.sqrt(500000/(p.width*p.height)));this.overview.width=Math.max(1,Math.floor(p.width*s));this.overview.height=Math.max(1,Math.floor(p.height*s));this.overview.getContext('2d').drawImage(this.image,0,0,this.overview.width,this.overview.height);this.overviewGeneration=data.generation;}
      if(request.running&&this.player.running&&!this.statsFrame)this.statsFrame=requestAnimationFrame(now=>{this.statsFrame=0;const frame=this.presented;if(frame?.running){this.playbackStats.record(`${frame.generation}:${frame.time}`,now);this.module.dataset.playbackFps=this.playbackStats.fps.toFixed(1);}});
      const first=!this.presented;this.presented=request;
      if(this.firstFrameMs===undefined){this.firstFrameMs=performance.now();this.module.dataset.firstFrameMs=this.firstFrameMs.toFixed(1);}
      if(first)this.sync();if(this.deckGeneration!==data.generation){this.deckGeneration=data.generation;if(!this.presentation.opened)this.presentation.prepare().catch(()=>{});}
      if(this.playAfterPaint&&request.time===this.player.time){this.playAfterPaint=false;this.module.dataset.playRequested='false';this.player.start();}
      this.positionPixels();this.last.metrics.raster=data.rasterMs;this.last.metrics.total=this.last.metrics.compile+this.last.metrics.lower+data.rasterMs;if(data.id===this.paintId)this.module.dataset.painted='true';this.updateMetrics();this.requestPaint();}
    }
    if(!this.paintTimer)this.flushPaint();return;
   }
   if(data.type==='init-error'){clearTimeout(this.initTimeout);this.error(data.error);this.worker?.terminate();this.worker=null;return;}
   if(data.type==='ready'){clearTimeout(this.initTimeout);this.ready=true;this.initialization=performance.now()-start;for(const font of this.loadedFonts)this.worker.postMessage({type:'font',...font});if(this.pending)this.drain();return;}
   if(data.type==='full-fonts-ready'){this.fullFontState='ready';this.module.dataset.fullFonts='ready';if(this.pending)this.drain();return;}
   if(data.type==='full-fonts-error'){this.fullFontState='error';this.module.dataset.fullFonts='error';this.error({message:t('编辑字体准备失败，请点击重试。','Editing fonts could not load. Please retry.')+' '+data.error.message});return;}
   if(data.type==='font-added'){this.fontNote.textContent=t('内置 DejaVu Sans、Noto Sans CJK SC；另已载入：','Bundled DejaVu Sans and Noto Sans CJK SC; also loaded: ')+(data.families||[]).join(' / ');this.schedule(true);return;}
   if(data.type==='resources'){if(data.id===this.active){if(data.waiting){clearTimeout(this.timeout);if(this.segmentStart!==undefined)this.remaining-=performance.now()-this.segmentStart;this.segmentStart=undefined;}else this.arm();}return;}
   if(data.type==='error'&&typeof data.id==='string'){this.diagnostics.textContent=data.error.message;this.sync();return;}
   if(data.id!==this.active)return;
   clearTimeout(this.timeout);this.busy=false;
   if(data.id===this.seq){if(data.type==='result')this.result(data);else this.error(data.error);}
   if(this.pending)this.drain();
  };
  this.worker.onerror=()=>{clearTimeout(this.initTimeout);clearTimeout(this.timeout);this.busy=false;this.error({message:t('预览引擎加载失败，可点击立即运行重试。','Preview engine failed to load. Run again to retry.')});this.worker.terminate();this.worker=null;};
 }
 arm(){clearTimeout(this.timeout);this.segmentStart=performance.now();this.timeout=setTimeout(()=>{this.worker?.terminate();this.worker=null;this.busy=false;this.error({message:t('运行超过 10 秒，已停止。请修改源码或恢复示例。','Execution exceeded 10 seconds and was stopped. Edit or restore the example.')});if(this.pending)this.drain();},Math.max(0,this.remaining));}
 drain(){clearTimeout(this.debounce);this.debounce=setTimeout(()=>{if(!this.busy&&!this.composing)this.run();},Math.max(0,(this.readyAt||0)-performance.now()));}
 schedule(immediate=false){this.presentation?.invalidate();this.readyAt=performance.now()+(immediate?0:250);this.seq++;this.pending=true;this.module.dataset.liveState='pending';this.statusText=t('等待更新…','Waiting to update…');this.sync();clearTimeout(this.debounce);this.changed();if(!this.activated)return;this.drain();}
 run(){if(!this.worker)this.boot();if(!this.ready){this.pending=true;this.statusText=t('正在加载预览引擎…','Loading preview engine…');this.sync();return;}if(this.module.dataset.modified==='true'&&this.fullFontState!=='ready'){this.pending=true;if(this.fullFontState==='error'){this.error({message:t('编辑字体准备失败，请点击重试。','Editing fonts could not load. Please retry.')});return;}this.ensureFullFonts();this.statusText=t('正在准备编辑字体…','Preparing fonts for editing…');this.sync();return;}this.pending=false;this.busy=true;this.active=this.seq;this.remaining=10000;this.statusText=t('正在运行…','Running…');this.sync();this.arm();this.worker.postMessage({type:'run',reduced:this.player.reduced,id:this.active,entry:this.entry,pageId:this.pageId,files:{...this.files}});}
 changed(){const dirty=Object.keys(this.files).some(f=>this.files[f]!==this.original[f]);this.module.dataset.modified=String(dirty);this.module.querySelector('.live-reset').disabled=!dirty;this.sync();}
 editor(panel){
  if(this.sourceEditor){this.sourceEditor.editor.call(this,panel);return Promise.resolve();}
  if(this.sourceLoadError)return Promise.resolve();
  this.sourceLoading??=import('./source-editor.mjs');
  return this.sourceLoading.then(editor=>{this.sourceEditor=editor;if(!panel.hidden&&this.module.dataset.mode==='source')editor.editor.call(this,panel);},error=>{this.sourceLoadError=true;this.error({message:error.message});});
 }
 sync(){
  const panel=this.module.querySelector('.source-panel:not([hidden])'),editable=!!panel?.dataset.editFile;
  if(this.pageSelect)this.pageSelect.hidden=false;
  this.toolbar.hidden=!editable;
  if(editable&&this.activated)this.editor(panel);
  // Playback and the current page are independent of the source editor tab.
  const live=!!this.last&&!!this.presented;
  this.static.hidden=live;this.image.hidden=!live;
  this.player.bar.hidden=!live;
  const failed=this.module.dataset.liveState==='error';
  this.startup.hidden=live;
  this.startup.querySelector('span').textContent=failed?t('准备失败，请重试','Could not prepare preview. Please retry'):this.module.dataset.hasAnimation==='true'?t('正在准备动画…','Preparing animation…'):t('正在准备预览…','Preparing preview…');
  this.startup.querySelector('.startup-retry').hidden=!failed;
  this.overview.hidden=this.player.time!==null||!live||this.overviewGeneration!==this.last.generation;
  this.module.dataset.previewState=this.module.dataset.liveState||'pending';
  this.module.querySelector('.live-modified').textContent=this.module.dataset.modified==='true'?t('已修改 · 仅本页','Modified · this page'):t('修改仅保留在本页','Edits stay on this page');
  this.status.textContent=this.statusText||t('正在准备预览…','Preparing preview…');
  const stale=this.last&&this.last.id!==this.seq;
  this.module.querySelector('.live-total').textContent=this.last?`${stale?t('上次合计','Previous total'):t('合计','Total')} ${this.last.metrics.total.toFixed(1)} ms`:t('合计 —','Total —');
  this.module.querySelector('.timing-details').hidden=!this.last;
  const note=this.module.querySelector('.preview-state');
  const pendingDraft=this.module.dataset.liveState==='pending'&&this.module.dataset.modified==='true';
  note.hidden=(!stale&&this.module.dataset.liveState!=='error'&&!pendingDraft);
  note.textContent=this.module.dataset.liveState==='error'?(this.last?t('上次成功结果','Last successful result'):t('原始示例 · 编辑未通过','Original example · edits failed')):(this.last?t('上次结果 · 等待更新','Previous result · updating'):t('原始示例 · 等待更新','Original example · updating'));
  const diagnostic=!!this.diagnostics.textContent,button=this.module.querySelector('.diagnostics-toggle');
  button.hidden=!diagnostic;const diagnosticLabel=this.module.dataset.liveState==='error'?t('1 个错误','1 error'):t(`${this.warningCount||0} 个警告`,`${this.warningCount||0} warnings`);iconLabel(button,'warning',diagnosticLabel);this.diagnosticTitle.textContent=diagnosticLabel;this.diagnosticNote.textContent=this.currentError?t('源码有错误，显示上次结果','Source error; showing the last result'):'';button.setAttribute('aria-expanded',String(!!this.diagnosticsOpen));
  this.diagnostics.hidden=!diagnostic||!this.diagnosticsOpen;this.diagnosticPanel.hidden=this.diagnostics.hidden;
  this.workspace?.fit();
 }

 updateDocument(doc){
   if(doc.pageId)this.pageId=doc.pageId;
   if(!this.pageSelect){this.pageSelect=document.createElement('select');this.pageSelect.className='document-pages';this.pageSelect.setAttribute('aria-label',t('文档页面','Document page'));this.module.querySelector('.preview-toolbar').querySelector('h2').after(this.pageSelect);this.pageSelect.onchange=()=>{this.pageId=this.pageSelect.value;this.workspace.zoom=1;this.workspace.pan={x:0,y:0};this.schedule(true);};}
   this.pageSelect.replaceChildren();for(const [i,p] of doc.pages.entries()){const option=document.createElement('option');option.value=p.id;option.textContent=`${i+1}. ${p.name||p.id}`;this.pageSelect.append(option);}this.pageSelect.value=doc.pageId||'';
 }
 result(data){if(data.protocol!==3||(!data.drawingInfo&&!data.displayList)){this.error({code:'E_PREVIEW_PROTOCOL',message:'Expected drawing protocol 3'});return;}
  if(data.document)this.updateDocument(data.document);
  this.last=data;
  if(this.readyMs===undefined){this.readyMs=performance.now();this.module.dataset.readyMs=this.readyMs.toFixed(1);}
  const resume=this.player.resume;this.player.update(data.inspection.animation);
  const resumeStart=resume?this.player.restorePage(resume):false;
  this.playAfterPaint=!!data.inspection.animation&&(resumeStart||this.module.dataset.playRequested==='true');
  if(this.playAfterPaint&&!resume)this.player.seek(0);this.previewControls.update();this.module.dataset.generation=String(data.generation);this.module.dataset.painted='false';this.paintKey='';this.image.dataset.aspectRatio=String(data.inspection.page.width/data.inspection.page.height);this.statusText=data.inspection.animation?t('可以播放','Ready to play'):t('预览已更新','Preview updated');
  this.currentError=null;this.warningCount=data.warnings.length;if(!this.warningCount)this.diagnosticsOpen=false;this.diagnostics.textContent=data.warnings.map(w=>`${w.code} ${w.file}:${w.loc.line}:${w.loc.column} — ${w.message}`).join('\n');this.module.dataset.liveState='success';this.sync();this.updateMetrics();this.requestPaint(true);
 }
 updateMetrics(){const now=performance.now();if(this.player.running&&now<this.nextMetrics)return;this.nextMetrics=now+250;const m=this.last.metrics,ms=n=>`${n.toFixed(1)} ms`;this.metrics.textContent=t(`编译 ${ms(m.compile)} · 绘制指令 ${ms(m.lower)} · 绘制 ${ms(m.raster)} · 合计 ${ms(m.total)} · 初始化 ${ms(this.initialization||m.initialization)} · 资源 ${ms(m.resources)} · 准备 ${ms(this.readyMs||0)} · 首帧 ${ms(this.firstFrameMs||0)}`,`Compile ${ms(m.compile)} · Commands ${ms(m.lower)} · Raster ${ms(m.raster)} · Total ${ms(m.total)} · Initialize ${ms(this.initialization||m.initialization)} · Resources ${ms(m.resources)} · Ready ${ms(this.readyMs||0)} · First frame ${ms(this.firstFrameMs||0)}`);if(this.playbackStats.fps)this.metrics.textContent+=t(` · 呈现估计 ${this.playbackStats.fps.toFixed(1)} fps · 提交 ${this.playbackStats.submissionFps.toFixed(1)} fps · 错过时隙估计 ${this.playbackStats.missedPercent.toFixed(1)}%`,` · Presentation estimate ${this.playbackStats.fps.toFixed(1)} fps · Submissions ${this.playbackStats.submissionFps.toFixed(1)} fps · Missed slots estimate ${this.playbackStats.missedPercent.toFixed(1)}%`);this.metrics.textContent+=` · ${this.module.dataset.renderBackend||'Skia'} · ${this.module.dataset.renderWidth||0}×${this.module.dataset.renderHeight||0} · ${t('采样','Sample')} ${ms(Number(this.module.dataset.sampleMs||0))}`;if(this.module.dataset.gpuMs)this.metrics.textContent+=` · GPU ${ms(Number(this.module.dataset.gpuMs))}`;this.metrics.title=t('帧率由浏览器刷新回调估计，不代表显示器实际扫描输出。绘制耗时为 CPU 提交；GPU 查询可用时另列。 本设备本次浏览器实测；不包含输入防抖等待。','Presentation is estimated from refresh callbacks, not physical scan-out. Raster time is CPU submission; GPU time is separate when available. Excludes typing debounce.');this.module.querySelector('.live-total').textContent=`${t('合计','Total')} ${ms(m.total)}`;}
 positionPixels(){if(!this.presented||!this.last)return;const box=this.workspace.canvas.getBoundingClientRect(),page=this.last.inspection.page,c=this.presented.rect,sx=box.width/page.width,sy=box.height/page.height;Object.assign(this.image.style,{position:'absolute',left:c[0]*sx+'px',top:c[1]*sy+'px',width:(c[2]-c[0])*sx+'px',height:(c[3]-c[1])*sy+'px',maxWidth:'none',maxHeight:'none'});
  this.overview.hidden=this.player.time!==null||this.image.hidden||this.overviewGeneration!==this.last.generation;
  if(!this.overview.hidden){const [x0,y0,x1,y1]=c.map((v,i)=>100*v/(i%2?page.height:page.width));Object.assign(this.overview.style,{position:'absolute',left:'0',top:'0',width:box.width+'px',height:box.height+'px',clipPath:`polygon(evenodd,0% 0%,100% 0%,100% 100%,0% 100%,0% 0%,${x0}% ${y0}%,${x1}% ${y0}%,${x1}% ${y1}%,${x0}% ${y1}%,${x0}% ${y0}%)`});}
 }
 requestPaint(immediate=false){this.paintImmediate||=immediate;if(immediate&&this.player.running){cancelAnimationFrame(this.paintSchedule);this.paintNow();return;}if(this.paintSchedule)return;this.paintSchedule=requestAnimationFrame(()=>this.paintNow());}
 frameTime(){
  const time=this.player.time;if(time===null)return null;
  const step=this.player.timeline?.steps.findLast(step=>step.start<=time);if(!step)return time;
  const end=step.start+step.duration;
  // Sample inside the hold: (start + duration) - start can round down and
  // accidentally keep a CSS animation active instead of applying its fill.
  return time>end?Math.min(time,end+Number.EPSILON*Math.max(1,Math.abs(end))):time;
 }
 paintNow(){this.paintSchedule=0;const force=this.paintImmediate;this.paintImmediate=false;if(!this.last||!this.worker)return;this.positionPixels();
  const box=this.workspace.canvas.getBoundingClientRect(),view=this.workspace.viewport.getBoundingClientRect();if(!box.width||!box.height)return;
  const x=Math.max(0,view.left-box.left),y=Math.max(0,view.top-box.top),w=Math.min(box.width,view.right-box.left)-x,h=Math.min(box.height,view.bottom-box.top)-y;if(w<=0||h<=0)return;
  const time=this.frameTime(),page=this.last.inspection.page,visible=[x/box.width*page.width,y/box.height*page.height,(x+w)/box.width*page.width,(y+h)/box.height*page.height],scale=[box.width/page.width,box.height/page.height],dpr=devicePixelRatio||1,old=this.presented;
  if(old?.time===time&&old?.reduced===this.player.reduced&&old?.generation===this.last.generation&&old.scale[0]===scale[0]&&old.scale[1]===scale[1]&&old.dpr===dpr&&visible[0]>=old.rect[0]-1e-9&&visible[1]>=old.rect[1]-1e-9&&visible[2]<=old.rect[2]+1e-9&&visible[3]<=old.rect[3]+1e-9){this.pendingPaint=null;clearTimeout(this.paintTimer);this.paintTimer=0;this.paintKey=old.key;this.module.dataset.painted='true';return;}
  const rect=[Math.max(0,visible[0]-96/scale[0]),Math.max(0,visible[1]-96/scale[1]),Math.min(page.width,visible[2]+96/scale[0]),Math.min(page.height,visible[3]+96/scale[1])],width=Math.ceil((rect[2]-rect[0])*scale[0]*dpr),height=Math.ceil((rect[3]-rect[1])*scale[1]*dpr);
  const key=JSON.stringify([this.last.generation,rect,width,height,time,this.player.reduced]);if(key===this.paintKey)return;this.paintKey=key;this.paintId=(this.paintId||0)+1;this.module.dataset.painted='false';
  // Rendering is synchronous inside the worker. Keep one request in flight
  // and replace its pending successor as the viewport moves, instead of
  // queueing a bitmap for every animation frame that will later be discarded.
  this.pendingPaint={running:this.player.running,time,reduced:this.player.reduced,type:'paint',id:this.paintId,generation:this.last.generation,rect,width,height,scale,dpr,key};clearTimeout(this.paintTimer);this.paintTimer=0;
  if(force||(old?.scale[0]===scale[0]&&old?.scale[1]===scale[1]))this.flushPaint();else this.paintTimer=setTimeout(()=>{this.paintTimer=0;this.flushPaint();},80);
 }
 flushPaint(){if(this.paintInFlight||!this.pendingPaint||!this.worker)return;
  const next=this.pendingPaint;this.pendingPaint=null;if(next.generation!==this.last?.generation)return;
  // A slow renderer may finish after several clock ticks. Sample the latest
  // time instead of delivering a backlog of obsolete frames.
  if(this.player.running&&next.time!==null){next.time=this.frameTime();next.key=JSON.stringify([next.generation,next.rect,next.width,next.height,next.time,next.reduced]);this.paintKey=next.key;}
  next.sentAt=performance.now();if(next.running)this.playbackStats.submit(next.sentAt);this.paintInFlight=next;this.worker.postMessage(next);
 }

 error(error){this.currentError=error;if(error.document)this.updateDocument(error.document);this.statusText=this.last?t('源码有错误，显示上次结果','Source error; showing last successful result'):t('当前源码未通过','Current source failed');this.diagnostics.textContent=[error.code,error.file,error.loc?`${error.loc.line}:${error.loc.column}`:'',error.message].filter(Boolean).join(' ');this.module.dataset.liveState='error';this.sync();}
 reset(restart=true){this.previewControls?.close();if(this.previewControls)this.previewControls.ready=false;this.presentation?.invalidate();this.playAfterPaint=false;this.module.dataset.playRequested='false';const play=this.startup.querySelector('.startup-play');if(play){play.disabled=false;play.textContent=t('播放','Play');}this.player.update(null);this.seq++;this.pageId=undefined;if(this.pageSelect){this.pageSelect.remove();this.pageSelect=null;}clearTimeout(this.debounce);this.worker?.terminate();this.worker=null;clearTimeout(this.initTimeout);clearTimeout(this.timeout);this.busy=false;this.pending=false;this.files={...this.original};this.sourceEditor?.reset.call(this);this.last=null;this.presented=null;clearTimeout(this.paintTimer);this.paintTimer=0;this.paintKey='';this.mainSkia?.dispose();this.mainSkia=null;const blank=this.image.cloneNode(false);this.image.replaceWith(blank);this.image=blank;this.diagnostics.textContent='';this.diagnosticsOpen=false;this.metrics.textContent='';this.statusText='';this.workspace.zoomTo(1);this.sync();this.changed();if(restart)this.schedule(true);}

 async font(name,bytes,alias){this.activate();const id=`font-${this.loadedFonts.length}`,font={id,name,bytes,alias};this.loadedFonts.push(font);this.worker.postMessage({type:'font',...font});}
 async localFonts(){this.diagnosticsOpen=true;if(!window.queryLocalFonts){this.diagnostics.textContent=t('此浏览器无法读取系统字体；请选择字体文件。未加载的字符显示方框。','This browser cannot read installed fonts. Select a font file; missing glyphs display as boxes.');this.sync();return;}
  try{const fonts=await window.queryLocalFonts();const dialog=document.createElement('dialog');dialog.className='font-dialog';const title=document.createElement('h2');title.textContent=t('选择本机字体系列','Choose an installed font family');const select=document.createElement('select');for(const family of [...new Set(fonts.map(f=>f.family))].sort()){const option=document.createElement('option');option.textContent=family;select.append(option);}const use=document.createElement('button');use.textContent=t('载入所选字体','Load selected family');const close=document.createElement('button');close.textContent=t('取消','Cancel');dialog.append(title,select,use,close);document.body.append(dialog);close.onclick=()=>dialog.close();dialog.onclose=()=>dialog.remove();use.onclick=async()=>{use.disabled=true;for(const f of fonts.filter(f=>f.family===select.value))await this.font(f.postscriptName,await(await f.blob()).arrayBuffer(),f.family);dialog.close();};dialog.showModal();}catch{this.diagnostics.textContent=t('未获得系统字体访问权限。可选择字体文件；缺字显示方框。','Installed-font access was not granted. Select a font file; missing glyphs display as boxes.');this.sync();}}
}
const editors=[...document.querySelectorAll('.example-module[data-entry]')].map(module=>new ExampleEditor(module));
const observer=new IntersectionObserver(entries=>{for(const entry of entries)if(entry.isIntersecting){editors.find(e=>e.module===entry.target)?.activate();observer.unobserve(entry.target);}},{rootMargin:'160px'});editors.forEach(e=>observer.observe(e.module));
addEventListener('pageshow',e=>{if(e.persisted)editors.forEach(editor=>editor.reset());});

addEventListener('pagehide',()=>editors.forEach(editor=>{editor.loadedFonts=[];editor.fontNote.textContent=t('预览内置 DejaVu Sans、Noto Sans CJK SC；也可载入自己的字体。','Preview bundles DejaVu Sans and Noto Sans CJK SC; you can also load your own fonts.');editor.reset(false);editor.workspace.resetView();}));
