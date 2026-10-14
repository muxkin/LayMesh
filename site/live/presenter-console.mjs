import {PageThumbnails} from './page-thumbnails.mjs';
import {iconLabel} from './presentation-icons.mjs';
export const consoleModuleURL=import.meta.url;
const element=(tag,parent,cls,text)=>{const e=document.createElement(tag);if(cls)e.className=cls;if(text)e.textContent=text;parent.append(e);return e;};
const clockText=seconds=>`${Math.floor(seconds/60).toString().padStart(2,'0')}:${Math.floor(seconds%60).toString().padStart(2,'0')}`;
class RemotePreview {
 constructor(owner){this.owner=owner;this.canvas=document.createElement('canvas');}
 async draw(index,width){const image=await this.owner.requestPreview(index,width);if(this.closed){image.close();return;}this.canvas.width=image.width;this.canvas.height=image.height;this.canvas.getContext('2d').drawImage(image,0,0);image.close();return this.canvas;}
 dispose(){this.closed=true;this.canvas.width=this.canvas.height=1;}
}
export async function bootPresenterConsole({port,config}){
 document.documentElement.lang='zh-CN';document.title=config.title+' · 演讲者视图';document.body.replaceChildren();document.body.style.margin='0';const style=document.createElement('style');style.textContent=config.css;document.head.append(style);
 window.laymeshPresenterConsole=new PresenterConsole(port,config);
}
class PresenterConsole {
 constructor(port,config){Object.assign(this,{port,config,opened:true,connected:false});this.pages=config.pages;this.noteScroll=new Map();this.noteSize=20;this.pending=new Map();this.requestId=0;this.build();
  this.currentContext=this.currentCanvas.getContext('2d');this.nextRenderer=new RemotePreview(this);
  this.thumbs=new PageThumbnails({parent:this.filmstrip,pages:this.pages,renderer:new RemotePreview(this),horizontal:true,onSelect:i=>this.command('go',i),paused:()=>!!this.state?.running||document.hidden});
  this.resize=new ResizeObserver(()=>{this.sendVisibility();this.paintNext();});this.resize.observe(this.currentCanvas.parentElement);this.resize.observe(this.nextFrame);
  this.visibility=()=>{this.sendVisibility();if(!document.hidden)this.paintNext();};document.addEventListener('visibilitychange',this.visibility);
  port.onmessage=({data})=>this.receive(data);port.start();this.sendVisibility();port.postMessage({type:'ready'});
  this.watch=setInterval(()=>{if(!window.opener||window.opener.closed){this.connected=false;this.root.dataset.connected='false';this.connection.textContent='原作品窗口已关闭，请重新打开作品';}},1000);
 }
 build(){
  this.root=element('section',document.body,'presenter');this.root.tabIndex=-1;this.root.setAttribute('role','dialog');this.root.setAttribute('aria-modal','true');this.root.setAttribute('aria-label','演讲者视图');
  this.inert=[...document.body.children].filter(n=>n!==this.root&&!n.inert&&n instanceof HTMLElement);this.inert.forEach(n=>n.inert=true);
  const header=element('header',this.root,'presenter-header');const brand=element('div',header,'presenter-brand');brand.innerHTML='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3v18h18M7 3v14h14M11 3v10h10"/></svg><b>LayMesh</b>';element('span',header,'presenter-heading','演讲者视图');element('span',header,'presenter-title',this.config.title);this.connection=element('output',header,'presenter-connection','正在准备观众窗口');this.connection.setAttribute('aria-live','polite');
  this.button(header,'定位观众窗口',()=>this.command('focus-audience'),'focus-audience');this.reconnect=this.button(header,'前往观众窗口全屏',()=>this.command('fullscreen'),'reconnect');this.reconnect.hidden=false;this.button(header,'结束放映',()=>this.command('end'),'end','danger');
  this.displayControls=element('div',this.root,'presenter-display-controls');this.displayControls.append(this.connection);element('span',this.displayControls,'muted','手动拖动窗口到所需屏幕，再在观众窗口全屏。');
  const meta=element('div',this.root,'presenter-meta');element('span',meta,'muted','当前页');this.pageLabel=element('strong',meta,'presenter-page','正在准备…');element('span',meta,'muted timer-label','已用时间');this.elapsedLabel=element('output',meta,'presenter-time','00:00');element('span',meta,'muted','当前时间');this.wall=element('time',meta,'presenter-time');this.timerButton=this.button(meta,'暂停计时',()=>this.command('timer-pause'),'timer-pause');iconLabel(this.timerButton,'pause');const reset=this.button(meta,'计时归零',()=>this.command('timer-reset'),'timer-reset');iconLabel(reset,'replay');
  const grid=element('div',this.root,'presenter-grid'),left=element('div',grid,'presenter-left'),frame=element('div',left,'presenter-current');this.currentCanvas=element('canvas',frame);this.currentCanvas.setAttribute('aria-label','当前幻灯片');this.previewMask=element('div',frame,'presenter-preview-mask');this.previewMask.hidden=true;
  const timeline=element('div',left,'presenter-timeline');element('span',timeline,'muted','页内动画');this.step=element('span',timeline,'','等待画面');this.progress=element('progress',timeline);this.progress.max=1;
  const controls=element('nav',left,'presenter-controls');controls.setAttribute('aria-label','放映控制');for(const [action,label]of [['previous','上一步'],['next','下一步'],['pause','暂停动画'],['replay','重播本页'],['black','黑屏'],['white','白屏']])this.button(controls,label,()=>this.command(action),action,action==='next'?'primary':'');
  const side=element('aside',grid,'presenter-side'),next=element('section',side,'presenter-next');this.nextLabel=element('div',next,'muted','下一页');this.nextFrame=element('div',next,'presenter-next-frame');this.nextCanvas=element('canvas',this.nextFrame);this.lastPage=element('p',this.nextFrame,'muted','已是最后一页');this.lastPage.hidden=true;
  const notes=element('section',side,'presenter-notes'),notesHead=element('header',notes);element('span',notesHead,'muted','演讲备注');for(const [delta,label]of [[-2,'A−'],[2,'A+']]){const b=this.button(notesHead,label,()=>{this.noteSize=Math.max(14,Math.min(32,this.noteSize+delta));this.notes.style.fontSize=this.noteSize+'px';},delta<0?'notes-smaller':'notes-larger');b.setAttribute('aria-label',delta<0?'缩小备注字号':'放大备注字号');}
  this.notes=element('div',notes,'presenter-note-text','本页没有演讲备注');this.notes.style.fontSize=this.noteSize+'px';this.notes.tabIndex=0;this.notes.onscroll=()=>this.noteScroll.set(this.state?.pageId,this.notes.scrollTop);
  this.filmstrip=element('div',this.root,'presenter-filmstrip');element('footer',this.root,'presenter-help','← → 上一步 / 下一步　 S 暂停　 B 黑屏　 W 白屏');
  this.root.onkeydown=e=>{if(e.isComposing||e.ctrlKey||e.metaKey||e.altKey||e.repeat)return;if(e.key==='Escape'){e.preventDefault();this.command('end');return;}if(e.target.closest('button,input,select,.page-thumbnails,.presenter-note-text'))return;const action=({'ArrowLeft':'previous','ArrowRight':'next',' ':'next','Enter':'next','s':'pause','b':'black','w':'white'})[e.key];if(action){e.preventDefault();this.command(action);}};
  this.elapsed=0;this.timerPaused=false;this.tickAt=performance.now();this.timer=setInterval(()=>this.updateTimer(),250);this.updateTimer();this.root.focus();this.pagehide=()=>this.dispose();window.addEventListener('pagehide',this.pagehide,{once:true});
 }
 button(parent,label,action,id,cls=''){const b=element('button',parent,cls,label);b.type='button';b.title=label;b.setAttribute('aria-label',label);b.dataset.presenterAction=id;b.onclick=action;return b;}
 updateTimer(){const now=performance.now();if(this.connected&&this.state?.ready&&!this.timerPaused)this.elapsed+=(now-this.tickAt)/1000;this.tickAt=now;this.elapsedLabel.textContent=clockText(this.elapsed);this.wall.textContent=new Date().toLocaleTimeString([], {hour:'2-digit',minute:'2-digit'});}

 sendVisibility(){if(this.opened)this.port.postMessage({type:'mirror-options',visible:!document.hidden,width:Math.ceil(this.currentCanvas.parentElement.clientWidth*Math.min(devicePixelRatio||1,2))});}
 command(action,index){this.port.postMessage({type:'command',action,index});}
 requestPreview(index,width){return new Promise((resolve,reject)=>{const id=++this.requestId,timer=setTimeout(()=>{this.pending.delete(id);reject(Error('静态预览准备超时'));},120000);this.pending.set(id,{resolve,reject,timer});this.port.postMessage({type:'preview',id,index,width});});}
 receive(data){if(data.session!==this.config.session){data.bitmap?.close();return;}if(!this.opened){data.bitmap?.close();return;}
  if(data.type==='frame'){
   try{if(data.serial>=Math.max(this.frameSerial||0,this.state?.serial||0)&&data.sequence>(this.frameSequence||0)){this.currentCanvas.width=data.bitmap.width;this.currentCanvas.height=data.bitmap.height;this.currentContext.drawImage(data.bitmap,0,0);this.frameSerial=data.serial;this.frameSequence=data.sequence;this.root.dataset.framePage=data.pageId;this.root.dataset.frameSequence=data.sequence;}}
   finally{data.bitmap.close();this.port.postMessage({type:'frame-ack',sequence:data.sequence});}return;
  }
  if(data.type==='preview-result'){const item=this.pending.get(data.id);if(!item){data.bitmap?.close();return;}clearTimeout(item.timer);this.pending.delete(data.id);if(data.error)item.reject(Error(data.error));else item.resolve(data.bitmap);return;}
  if(data.type==='notice'){this.connection.textContent=data.message;return;}
  if(data.type==='ended'){this.dispose();window.close();return;}
  if(data.type!=='state'||data.sequence<=(this.state?.sequence||0))return;
  const previous=this.state;this.state=data;this.elapsed=data.elapsed;this.timerPaused=data.timerPaused;this.tickAt=performance.now();iconLabel(this.timerButton,this.timerPaused?'play':'pause');this.timerButton.setAttribute('aria-label',this.timerPaused?'继续计时':'暂停计时');this.updateTimer();this.connected=true;this.root.dataset.connected='true';this.root.dataset.page=data.pageId;
  this.connection.textContent=data.error||data.notice|| (data.fullscreen?'观众画面已全屏':'观众画面未全屏');this.reconnect.hidden=data.fullscreen&&!data.error&&!data.notice;this.reconnect.textContent=data.error?'重试页面':'前往观众窗口全屏';this.reconnect.onclick=()=>this.command(data.error?'retry':'fullscreen');
  const p=this.pages[data.index];if(!p)return;if(data.inspection)p.inspection=data.inspection;
  this.pageLabel.textContent=`${data.index+1} / ${this.pages.length} · ${p.name||p.id}`;
  const status={playing:'播放中',paused:'已暂停',waiting:'等待点击',complete:'已完成',static:'静态页面',holding:'等待切页'}[data.animationStatus];
  this.step.textContent=!data.ready?'正在准备…':data.stepCount?`${Math.min(data.stepCount,data.active>=0?data.active+1:data.completed)} / ${data.stepCount} · ${data.stepName||status}${data.animationStatus==='holding'?` · 等待切页 ${data.holdRemaining.toFixed(1)}s`:data.animationStatus==='complete'?' · 已完成':''}`:'静态页面';
  this.progress.value=data.ready?data.animationProgress:0;this.progress.setAttribute('aria-valuetext',status);
  const pause=this.root.querySelector('[data-presenter-action="pause"]');for(const b of this.root.querySelectorAll('.presenter-controls button'))b.disabled=!data.ready;pause.disabled=!data.ready||['complete','static'].includes(data.animationStatus);pause.textContent=data.animationStatus==='complete'?'动画已完成':data.running?'暂停动画':data.animationStatus==='waiting'?'播放下一步':'继续动画';
  this.previewMask.hidden=!data.blank;this.previewMask.style.background=data.blank||'transparent';
  const notes=p.inspection?.speakerNotes||'本页没有演讲备注';if(this.notes.textContent!==notes)this.notes.textContent=notes;
  if(previous?.pageId!==data.pageId){this.notes.scrollTop=this.noteScroll.get(data.pageId)||0;this.thumbs.select(data.index);const next=this.pages[data.index+1];this.nextLabel.textContent=next?`下一页　${data.index+2} / ${this.pages.length}`:'下一页';this.lastPage.hidden=!!next;this.nextCanvas.hidden=!next;this.paintNext();}
 }
 async paintNext(){if(!this.state||!this.pages[this.state.index+1]||document.hidden)return;const index=this.state.index+1,width=Math.min(1024,Math.max(240,Math.round(this.nextFrame.clientWidth*Math.min(devicePixelRatio||1,2)))),key=index+':'+width;if(this.nextKey===key)return;this.nextKey=key;try{const canvas=await this.nextRenderer.draw(index,width);if(!this.opened||key!==this.nextKey||!canvas)return;this.nextCanvas.width=canvas.width;this.nextCanvas.height=canvas.height;this.nextCanvas.getContext('2d').drawImage(canvas,0,0);}catch(error){if(this.opened&&key===this.nextKey){this.nextKey=null;this.nextLabel.textContent='下一页暂不可用：'+error.message;}}}
 dispose(){if(!this.opened)return;this.opened=false;clearInterval(this.watch);clearInterval(this.timer);this.resize.disconnect();document.removeEventListener('visibilitychange',this.visibility);this.thumbs.dispose();this.nextRenderer.dispose();for(const p of this.pending.values()){clearTimeout(p.timer);p.reject(Error('演讲者视图已关闭'));}this.pending.clear();this.port.postMessage({type:'console-closed'});this.port.close();}
}
