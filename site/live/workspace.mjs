// Presentation controls only. Source, worker results and undo remain owned by ExampleEditor.
const en=document.documentElement.lang==='en',t=(zh,english)=>en?english:zh;
export class Workspace {
 constructor(module,editSummary) {
  this.module=module;this.box=module.querySelector('.example-workspace');this.body=module.querySelector('.workspace-body');
  this.viewport=module.querySelector('.preview-viewport');this.canvas=module.querySelector('.preview-canvas');
  this.splitter=module.querySelector('.workspace-splitter');this.ratio=.5;this.zoom=1;this.pan={x:0,y:0};
  this.bindSplit();this.bindPreview();this.bindSummary(editSummary);
  this.resize=new ResizeObserver(()=>{this.layout();this.fit();});this.resize.observe(this.box);this.resize.observe(this.viewport);
  module.querySelectorAll('.preview-canvas img').forEach(img=>img.addEventListener('load',()=>this.fit()));
  module.querySelector('.workspace-expand').addEventListener('click',()=>this.expand());
  module.querySelectorAll('details:not(.dependencies)').forEach(details=>details.addEventListener('toggle',()=>{if(details.open)module.querySelectorAll('details:not(.dependencies)').forEach(other=>{if(other!==details)other.open=false;});}));
  module.addEventListener('keydown',e=>{if(e.key==='Escape'){const open=module.querySelector('details[open]:not(.dependencies)');if(open){open.open=false;open.querySelector('summary').focus();e.preventDefault();e.stopPropagation();}}});
  document.addEventListener('pointerdown',e=>{for(const menu of module.querySelectorAll('details[open]:not(.dependencies)'))if(!menu.contains(e.target))menu.open=false;});
 }
 layout(){
  const width=this.body.clientWidth,wide=width>=900;this.splitter.tabIndex=wide?0:-1;
  if(!wide)return;
  const available=width-8,min=Math.min(.5,320/available);
  this.ratio=Math.max(min,Math.min(1-min,this.ratio));
  this.body.style.setProperty('--code-width',`${this.ratio*available}px`);
  this.splitter.setAttribute('aria-valuemin',String(Math.ceil(min*100)));this.splitter.setAttribute('aria-valuemax',String(Math.floor((1-min)*100)));this.splitter.setAttribute('aria-valuenow',String(Math.round(this.ratio*100)));
  this.splitter.setAttribute('aria-valuetext',t(`代码 ${Math.round(this.ratio*100)}%，图形 ${100-Math.round(this.ratio*100)}%`,`Code ${Math.round(this.ratio*100)}%, preview ${100-Math.round(this.ratio*100)}%`));
 }
 bindSplit(){
  let dragging=false;
  this.splitter.addEventListener('pointerdown',e=>{if(e.button!==0)return;dragging=true;this.splitter.setPointerCapture(e.pointerId);e.preventDefault();});
  this.splitter.addEventListener('pointermove',e=>{if(!dragging)return;this.ratio=(e.clientX-this.body.getBoundingClientRect().left)/(this.body.clientWidth-8);this.layout();});
  for(const type of ['pointerup','pointercancel','lostpointercapture'])this.splitter.addEventListener(type,()=>dragging=false);
  this.splitter.addEventListener('dblclick',()=>{this.ratio=.5;this.layout();});
  this.splitter.addEventListener('keydown',e=>{if(!['ArrowLeft','ArrowRight','Home','End','Enter'].includes(e.key))return;e.preventDefault();this.ratio=e.key==='Home'?0:e.key==='End'?1:e.key==='Enter'?.5:this.ratio+(e.key==='ArrowLeft'?-1:1)*(e.shiftKey?.1:.02);this.layout();});
 }
 bindSummary(edit){
  const pre=this.module.querySelector('.summary-panel pre');pre.tabIndex=0;pre.setAttribute('aria-label',t('摘要代码；按 Enter 编辑完整源码','Summary code; press Enter to edit full source'));pre.title=t('点击编辑完整源码','Click to edit full source');
  let start;
  pre.addEventListener('pointerdown',e=>{start={x:e.clientX,y:e.clientY,scroll:pre.scrollTop};});
  pre.addEventListener('click',e=>{
   if(e.button!==0||!start||Math.hypot(e.clientX-start.x,e.clientY-start.y)>6||pre.scrollTop!==start.scroll||!getSelection()?.isCollapsed)return;
   const line=e.target.closest('.code-line');if(!line)return;
   const lines=this.module.querySelector('.summary-panel .code-raw').defaultValue.split('\n'),index=[...pre.querySelectorAll('.code-line')].indexOf(line);
   const offset=lines.slice(0,index).reduce((n,line)=>n+line.length+1,0);
   edit(Number(this.module.querySelector('.summary-panel').dataset.sourceFrom)+offset);
  });
  pre.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();edit(Number(this.module.querySelector('.summary-panel').dataset.sourceFrom));}});
  this.module.querySelector('.summary-edit').addEventListener('click',()=>edit(Number(this.module.querySelector('.summary-panel').dataset.sourceFrom)));
 }
 activeImage(){return this.module.querySelector('.original-preview').hidden?this.module.querySelector('.live-image'):this.module.querySelector('.original-preview img');}
 fit(){
  const img=this.activeImage(),ratio=Number(img.dataset.aspectRatio)||(Number(img.getAttribute('width'))||img.naturalWidth)/(Number(img.getAttribute('height'))||img.naturalHeight)||1.5;
  this.viewport.style.setProperty('--figure-ratio',String(ratio));
  const width=this.viewport.clientWidth,height=this.viewport.clientHeight;if(!width||!height)return;
  const pad=width<480?16:24,fitWidth=Math.min(width-pad*2,(height-pad*2)*ratio),w=fitWidth*this.zoom,h=w/ratio;
  const limitX=Math.max(0,(w-width)/2+pad),limitY=Math.max(0,(h-height)/2+pad);
  this.pan.x=Math.max(-limitX,Math.min(limitX,this.pan.x));this.pan.y=Math.max(-limitY,Math.min(limitY,this.pan.y));
  this.canvas.style.width=`${w}px`;this.canvas.style.height=`${h}px`;this.canvas.style.transform=`translate(-50%, -50%) translate(${this.pan.x}px, ${this.pan.y}px)`;
  // Assigning sizes again can reload an image even when its URL is unchanged.
  // The load listener calls fit(), so an unconditional assignment loops forever.
  const sizes=`${Math.ceil(w)}px`;if(img.sizes!==sizes)img.sizes=sizes;
  this.viewport.style.touchAction=this.zoom>1?'none':'pan-y';
  this.module.querySelector('.preview-fit').textContent=this.zoom===1?t('适应','Fit'):`${Math.round(this.zoom*100)}%`;
 }
 zoomTo(value){this.zoom=Math.max(.25,Math.min(8,value));if(this.zoom===1)this.pan={x:0,y:0};this.fit();}
 bindPreview(){
  this.module.querySelector('.preview-zoom-in').addEventListener('click',()=>this.zoomTo(this.zoom*1.5));
  this.module.querySelector('.preview-zoom-out').addEventListener('click',()=>this.zoomTo(this.zoom/1.5));
  this.module.querySelector('.preview-fit').addEventListener('click',()=>this.zoomTo(1));
  this.module.querySelector('.preview-enlarge').addEventListener('click',()=>this.enlargeFigure());
  this.module.querySelectorAll('[data-preview-background]').forEach(button=>button.addEventListener('click',()=>{this.viewport.dataset.background=button.dataset.previewBackground;this.module.querySelectorAll('[data-preview-background]').forEach(b=>b.setAttribute('aria-pressed',String(b===button)));}));
  let drag;
  this.viewport.addEventListener('pointerdown',e=>{if(e.button!==0||(e.pointerType==='touch'&&this.zoom<=1))return;drag={x:e.clientX,y:e.clientY,pan:{...this.pan}};this.viewport.setPointerCapture(e.pointerId);this.viewport.focus({preventScroll:true});e.preventDefault();});
  this.viewport.addEventListener('pointermove',e=>{if(drag){this.pan={x:drag.pan.x+e.clientX-drag.x,y:drag.pan.y+e.clientY-drag.y};this.fit();}});
  for(const type of ['pointerup','pointercancel','lostpointercapture'])this.viewport.addEventListener(type,()=>drag=null);
  this.viewport.addEventListener('wheel',e=>{if(!e.ctrlKey&&!e.metaKey)return;e.preventDefault();this.zoomTo(this.zoom*(e.deltaY<0?1.1:1/1.1));},{passive:false});
  this.viewport.addEventListener('keydown',e=>{if(['+','=','-','0'].includes(e.key)){e.preventDefault();this.zoomTo(e.key==='0'?1:this.zoom*(e.key==='-'?1/1.5:1.5));}else if(e.key.startsWith('Arrow')){e.preventDefault();this.pan.x+=e.key==='ArrowLeft'?30:e.key==='ArrowRight'?-30:0;this.pan.y+=e.key==='ArrowUp'?30:e.key==='ArrowDown'?-30:0;this.fit();}});
 }
 enlargeFigure(){
  const img=this.activeImage();if(!img.src)return;
  const dialog=document.querySelector('.image-dialog'),image=dialog.querySelector('img'),stage=dialog.querySelector('.image-stage');
  image.src=img.dataset.full||img.currentSrc||img.src;image.alt=img.alt;dialog.querySelector('p').textContent=img.alt;
  const style=getComputedStyle(this.viewport);stage.style.backgroundColor=style.backgroundColor;stage.style.backgroundImage=style.backgroundImage;stage.style.backgroundSize=style.backgroundSize;
  dialog.showModal();stage.scrollTo(0,0);document.dispatchEvent(new CustomEvent('laymesh:lightbox',{detail:{image:img}}));
 }
 expand(){
  if(this.dialog){this.dialog.close();return;}
  const button=this.module.querySelector('.workspace-expand'),placeholder=document.createComment('workspace position'),dialog=document.createElement('dialog');
  dialog.className='workspace-dialog';dialog.setAttribute('aria-label',t('代码与图形工作区','Code and figure workspace'));
  this.box.before(placeholder);this.module.append(dialog);dialog.append(this.box);this.dialog=dialog;
  button.setAttribute('aria-label',t('退出放大工作区','Exit expanded workspace'));button.title=button.getAttribute('aria-label');
  dialog.addEventListener('close',()=>{placeholder.replaceWith(this.box);dialog.remove();this.dialog=null;button.setAttribute('aria-label',t('放大工作区','Expand workspace'));button.title=button.getAttribute('aria-label');button.focus({preventScroll:true});this.layout();this.fit();},{once:true});
  dialog.showModal();button.focus();this.layout();this.fit();
 }
 resetView(){this.dialog?.close();this.ratio=.5;this.zoom=1;this.pan={x:0,y:0};this.module.querySelector('[data-preview-background="dark"]').click();this.layout();this.fit();}
}
