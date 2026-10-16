import {consoleModuleURL} from './presenter-console.mjs';
import {PagePreviewRenderer} from './page-previews.mjs';
import {initializeSkia} from './skia-renderer.mjs';
import {createPresentationRuntime} from './presentation-worker.mjs';
import {PresenterMirror} from './presenter-mirror.mjs';
import {loadRendererRuntime} from './renderer-resources.mjs';
const imports=/((?:^|[\n;])\s*(?:import|export)\s*(?:[^'";]*?\s+from\s*)?['"])([^'"]+)(['"])/g;
async function bundleModules(){const modules={},seen=new Set();async function visit(url){if(seen.has(url))return;seen.add(url);const r=await fetch(url);if(!r.ok)throw Error('演讲者模块加载失败');const deps=[];modules[url]=(await r.text()).replace(imports,(all,a,b,c)=>{const dep=new URL(b,url).href;deps.push(dep);return a+dep+c;});await Promise.all(deps.map(visit));}await visit(consoleModuleURL);return {entry:consoleModuleURL,modules};}
function consoleBootstrap(nonce,origin){
 const ready=()=>window.opener?.postMessage({type:'laymesh-console-ready',version:2,nonce},origin),timer=setInterval(ready,250);ready();
 const receive=async event=>{if(event.source!==window.opener||event.data?.nonce!==nonce||event.data?.version!==2||!event.ports[0]||(origin!=='*'&&event.origin!==origin))return;window.removeEventListener('message',receive);clearInterval(timer);const port=event.ports[0],{bundle,config}=event.data,urls=new Map();
  function url(id){if(urls.has(id))return urls.get(id);const code=bundle.modules[id].replace(/((?:^|[\n;])\s*(?:import|export)\s*(?:[^'";]*?\s+from\s*)?['"])([^'"]+)(['"])/g,(all,a,b,c)=>a+url(b)+c);const blob=URL.createObjectURL(new Blob([code],{type:'text/javascript'}));urls.set(id,blob);return blob;}
  try{await(await import(url(bundle.entry))).bootPresenterConsole({port,config});}catch(error){document.body.textContent='演讲者控制台准备失败：'+error.message;port.postMessage({type:'error',error:error.message});}
  window.addEventListener('pagehide',()=>{for(const u of urls.values())URL.revokeObjectURL(u);},{once:true});
 };window.addEventListener('message',receive);
}
const token=()=>Array.from(crypto.getRandomValues(new Uint8Array(16)),b=>b.toString(16).padStart(2,'0')).join('');
// `auto` remains a compatibility alias for slides; only an explicit request
// opens the presenter console. Screen arrangement belongs to the user.
export async function startPresentation(view,{mode='slides',fullscreen,startPageId=view.getStartPageId()}={}){
 if(view.opened||view.closing||view.starting||view.presenter?.opened)return;
 if(!['auto','slides','presenter'].includes(mode))throw Error('Unknown presentation mode');
 view.starting=true;
 try{
  if(mode==='presenter')return await (view.presenter??=new PresenterView(view)).open({fullscreen:fullscreen??false,startPageId});
  return await view.open({fullscreen:fullscreen??true,startPageId});
 }finally{view.starting=false;}
}
// The original document owns playback. The popup contains controls and bitmaps only.
export class PresenterView {
 constructor(view,{runtime=()=>loadRendererRuntime(view.base),title=()=>document.title,css=()=>fetch(new URL('presentation.css',view.base)).then(r=>r.text())}={}){Object.assign(this,{view,runtime,title,css});view.presenter=this;}
 async open({fullscreen=false,startPageId=this.view.getStartPageId()}={}){
  if(this.opened){this.connect();return;}if(this.view.opened||this.view.closing)return;
  this.opened=true;this.started=false;this.launching=false;this.beginPromise=null;this.lastPresented=null;this.notices=new Map();this.sequence=0;this.lastStateKey=null;this.previewQueue=[];this.session=token();this.lastSent=0;this.elapsed=0;this.timerPaused=false;this.timerAt=null;this.noticeText='';
  const v=this.view;this.saved={adapter:v.runtimeAdapter,audience:v.audience,onState:v.onState};v.audience=true;v.onState=view=>{this.saved?.onState(view);this.sendState();};window.laymeshPresentation=v;
  const adapter=async(view,generation)=>{this.runtimeData??=this.runtime();const data=await this.runtimeData;this.skia??=initializeSkia(data.skiaBytes||data.skiaURL);const ck=await this.skia;if(!this.opened||generation!==view.generation)return;
   this.staticRenderer??=new PagePreviewRenderer({runtime:()=>this.runtimeData,skiaInstance:ck,page:async i=>{await view.deck.wait?.(i);return view.deck.store.page(view.deck.pages[i].id);}});
   // Preserve just the audience output buffer, so a paused/hidden console can
   // request the existing image without another scene render or GPU readPixels.
   const runtime=await createPresentationRuntime({...data,skiaInstance:ck,canvas:view.canvas,preserveDrawingBuffer:true,onPresent:meta=>{if(!this.opened||generation!==view.generation)return;this.lastPresented=meta;this.mirror?.present(meta);}},event=>{
    if(!this.opened||generation!==view.generation)return;
    if(event.type==='navigation-visible'){view.navigationVisible(event);return;}
    if(event.type!=='context')return;
    this.lastPresented=null;this.mirror?.reset();if(event.ready){this.notice('','context');view.resident.clear();view.readyResident?.clear();view.go(view.index,{time:view.clock.time,preserve:true});}else{view.clock.running=false;this.notice('显示资源正在恢复，放映已暂停。','context');view.sync();}
   });if(!this.opened||generation!==view.generation){await runtime({type:'dispose'});return;}view.mainRuntime=runtime;
  };adapter.stylesheet=this.saved.adapter?.stylesheet;v.runtimeAdapter=adapter;
  await v.open({fullscreen:false,startPageId,deferPreparation:true});if(!this.opened)return;
  this.buildRecovery();
  this.fullscreenListener=()=>{if(document.fullscreenElement===v.root){this.notice('','fullscreen');this.notice('','confirmation');}this.updateRecovery();this.sendState(true);};document.addEventListener('fullscreenchange',this.fullscreenListener);
  await this.launch(fullscreen);
 }
 async launch(fullscreen=false){
  if(!this.opened||!this.view.opened||this.launching)return;this.launching=true;const generation=this.view.generation;
  try{
   if(fullscreen){const result=await this.view.enterFullscreen();if(!this.opened||!this.view.opened||generation!==this.view.generation)return;
    if(!result?.ok){this.notice(result?.reason==='gesture'?'请点击按钮进入全屏。':`全屏未成功：${result?.message||'浏览器没有进入全屏'}。请点击按钮重试。`,'fullscreen');this.startupPrompt(result);this.updateRecovery();return;}
   }
   this.notice('','fullscreen');this.notice('','confirmation');
   // Open the console before asynchronous preparation while the click is active.
   this.started=true;this.recovery.append(this.recoveryFullscreen,this.recoveryWindow);this.view.progressBar.hidden=false;this.connect();this.updateRecovery();
   this.beginPromise??=this.view.begin();await this.beginPromise;
   if(this.opened){this.sendState(true);this.mirror?.force();}
  }finally{this.launching=false;}
 }
 startupPrompt(result){
  const v=this.view;v.overlay.hidden=false;v.progressHeading.textContent=result?.reason==='gesture'?'确认全屏':'全屏未成功';v.progressCount.textContent=this.noticeText;v.progressPercent.textContent='';v.progressBar.hidden=true;
  v.message.textContent='请点击下方按钮继续；也可先用窗口演讲，手动调整窗口后再全屏。';
  const actions=v.overlay.querySelector('.presentation-progress-actions');actions.prepend(this.recoveryFullscreen,this.recoveryWindow);this.recoveryFullscreen.classList.add('presentation-primary');this.recoveryFullscreen.focus();
 }
 async enterAudienceFullscreen(){
  if(!this.started)return this.launch(true);
  const generation=this.view.generation,result=await this.view.enterFullscreen();if(!this.opened||!this.view.opened||generation!==this.view.generation)return;
  if(!result?.ok)this.notice(`全屏未成功：${result?.message||'请在观众窗口点击按钮重试'}。`,'fullscreen');
  else{this.notice('','fullscreen');this.notice('','confirmation');}
  this.updateRecovery();this.sendState(true);
 }
 updateRecovery(){
  if(!this.recovery)return;this.recovery.hidden=!this.started;const active=document.fullscreenElement===this.view.root;
  this.recoveryFullscreen.hidden=this.started&&active;
  this.recoveryFullscreen.textContent='进入全屏';
  this.recoveryOpen.disabled=!this.started;this.recoveryWindow.hidden=this.started;
  this.recovery.classList.toggle('windowed',!active);
 }
 buildRecovery(){const v=this.view,bar=document.createElement('aside');bar.className='presenter-recovery';v.root.append(bar);this.recovery=bar;
  this.recoveryMessage=document.createElement('span');this.recoveryMessage.setAttribute('role','status');this.recoveryMessage.textContent='可将两个窗口拖到所需屏幕，再在观众窗口点击全屏。';bar.append(this.recoveryMessage);
  const button=(text,action)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.onclick=action;bar.append(b);return b;};
  this.recoveryOpen=button('打开演讲者控制台',()=>this.connect());this.recoveryOpen.dataset.action='presenter-reconnect';
  this.recoveryFullscreen=button('进入全屏',()=>this.enterAudienceFullscreen());this.recoveryFullscreen.dataset.action='audience-fullscreen';
  this.recoveryWindow=button('改用窗口演讲',()=>this.launch(false));this.recoveryWindow.dataset.action='presenter-windowed';
  this.recoveryEnd=button('结束放映',()=>this.close());this.recoveryEnd.dataset.action='presenter-end';this.updateRecovery();
 }
 confirmFullscreen(){
  this.view.clock.running=false;this.notice('请先将观众窗口拖到目标屏幕，再在该窗口点击“进入全屏”。','confirmation');this.updateRecovery();window.focus();this.recoveryFullscreen.focus();this.sendState(true);
 }
 notice(message,kind='runtime'){
  if(message)this.notices.set(kind,message);else this.notices.delete(kind);
  this.noticeText=[...this.notices.values()].join(' ');if(this.recoveryMessage)this.recoveryMessage.textContent=this.noticeText||'可将两个窗口拖到所需屏幕，再在观众窗口点击全屏。';this.recovery?.classList.toggle('attention',!!this.noticeText);this.send({type:'notice',message:this.noticeText});
 }
 async connect(){
  if(!this.opened||!this.started)return;if(this.popup&&!this.popup.closed){this.popup.focus();return;}
  const generation=this.connectionGeneration=(this.connectionGeneration||0)+1;this.connected=false;this.mirrorOptions={visible:false};this.port?.close();this.mirror?.dispose();this.previewQueue=[];this.nonce=token();const nonce=this.nonce;
  const features='popup,width=1280,height=800';
  this.popup=window.open('','laymesh-console-'+this.session,features);
  if(!this.popup){this.notice('演讲者控制台被拦截，请允许弹窗后点击“打开演讲者控制台”。','connection');return;}
  const popup=this.popup,origin=location.protocol==='file:'||location.origin==='null'?'*':location.origin;this.notice('正在连接演讲者控制台…','connection');
  window.removeEventListener('message',this.handshake);clearTimeout(this.handshakeTimer);
  const ready=new Promise((resolve,reject)=>{this.handshake=event=>{if(event.source===popup&&event.data?.nonce===nonce&&event.data?.version===2&&(origin==='*'||event.origin===origin)){window.removeEventListener('message',this.handshake);resolve();}};window.addEventListener('message',this.handshake);this.handshakeTimer=setTimeout(()=>reject(Error('演讲者控制台连接超时，请关闭控制台后重试。')),15000);});ready.catch(()=>{});
  popup.document.open();popup.document.write('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>LayMesh · 演讲者视图</title><body style="background:#15191f;color:#edf2f7;font:16px system-ui">正在准备控制台…<script>('+consoleBootstrap.toString()+')('+JSON.stringify(nonce)+','+JSON.stringify(origin)+')<\/script>');popup.document.close();
  clearInterval(this.watch);this.watch=setInterval(()=>{if(popup.closed){this.connected=false;this.mirror?.dispose();this.port?.close();this.previewQueue=[];this.notice('演讲者控制台已关闭；观众放映继续。','connection');clearInterval(this.watch);}},500);
  try{
   const [deck,bundle,css]=await Promise.all([this.view.prepare(this.view.startPageId),this.bundle??=bundleModules().catch(error=>{this.bundle=null;throw error;}),this.css(),ready]);clearTimeout(this.handshakeTimer);if(!this.opened||generation!==this.connectionGeneration)return;
   const channel=new MessageChannel();this.port=channel.port1;this.port.onmessage=({data})=>{if(this.opened&&generation===this.connectionGeneration)this.receive(data);else data.bitmap?.close();};this.port.start();
   this.mirror=new PresenterMirror({canvas:()=>this.view.canvas,send:(data,transfer)=>this.send(data,transfer),rect:(meta,c)=>{const p=deck.pages.find(p=>p.id===meta.pageId)?.inspection?.page;if(!p?.width||!p?.height)return;const k=Math.min(c.width/p.width,c.height/p.height),w=Math.round(p.width*k),h=Math.round(p.height*k);return [Math.round((c.width-w)/2),Math.round((c.height-h)/2),w,h];},onError:message=>this.notice(message)});
   // The first audience frame can finish before the console connects. Retain
   // its metadata while hidden, then copy the preserved buffer on readiness.
   this.mirror.configure(this.mirrorOptions);if(this.lastPresented)this.mirror.present(this.lastPresented);
   popup.postMessage({version:2,nonce,bundle,config:{session:this.session,pages:deck.pages.map(p=>({id:p.id,name:p.name,inspection:p.inspection||{}})),title:this.title(),css}},origin,[channel.port2]);
  }catch(error){if(this.opened&&generation===this.connectionGeneration)this.notice(error.message,'connection');}
 }
 send(data,transfer=[]){if(this.port&&this.connected)this.port.postMessage({...data,session:this.session},transfer);else for(const item of transfer)item.close?.();}
 sendState(force=false){const v=this.view;if(!this.opened||!v.deck)return;const stamp=performance.now();if(v.ready){if(this.timerAt!==null&&!this.timerPaused)this.elapsed+=(stamp-this.timerAt)/1000;this.timerAt=stamp;}if(!this.connected)return;const status=v.clock.status(),key=[v.index,v.serial,v.clock.active,v.clock.completed,v.clock.running,status.animationStatus,v.mask.hidden,v.mask.dataset.color,v.ready,v.ended,document.fullscreenElement===v.root].join('/'),now=performance.now();if(!force&&key===this.lastStateKey&&now-this.lastSent<65)return;const changed=key!==this.lastStateKey;this.lastStateKey=key;this.lastSent=now;
  const data={type:'state',notice:this.noticeText,error:!v.ready&&v.retry?.hidden===false?v.message?.textContent:null,elapsed:this.elapsed,timerPaused:this.timerPaused,sequence:++this.sequence,index:v.index,pageId:v.deck.pages[v.index]?.id,serial:v.serial,time:v.clock.time,active:v.clock.active,completed:v.clock.completed,running:v.clock.running,mode:v.clock.mode,speed:v.clock.speed,blank:v.mask.hidden?null:v.mask.dataset.color,ready:v.ready,ended:!!v.ended,reduced:v.reduced.matches,fullscreen:document.fullscreenElement===v.root,...status};
  if(changed||force)data.inspection=v.deck.pages[v.index]?.inspection;this.state=data;this.send(data);if(changed&&!v.clock.running)this.mirror?.force();this.pumpPreviews();
 }
 receive(data){
  if(data.type==='ready'){this.connected=true;this.mirror?.configure(this.mirrorOptions);this.notice('','connection');this.sendState(true);this.view.paint().then(()=>this.mirror?.force());}
  else if(data.type==='mirror-options'){this.mirrorOptions=data;if(this.connected)this.mirror?.configure(data);}
  else if(data.type==='frame-ack')this.mirror?.acknowledge(data.sequence);
  else if(data.type==='preview'){if(Number.isInteger(data.index)&&data.index>=0&&data.index<(this.view.deck?.pages.length||0)){this.previewQueue.push({...data,generation:this.connectionGeneration});this.pumpPreviews();}}
  else if(data.type==='console-closed'){this.connected=false;this.mirror?.dispose();this.previewQueue=[];this.notice('演讲者控制台已关闭；观众放映继续。','connection');}
  else if(data.type==='error')this.notice(data.error);
  else if(data.type==='command'){
   const v=this.view;if(data.action==='timer-pause'||data.action==='timer-reset'){this.sendState(true);if(data.action==='timer-pause')this.timerPaused=!this.timerPaused;else this.elapsed=0;this.timerAt=performance.now();this.sendState(true);return;}if(data.action==='end')this.close();else if(data.action==='retry')v.retryPage();else if(data.action==='go')v.go(data.index);else if(data.action==='focus-audience')window.focus();
   else if(data.action==='fullscreen')this.confirmFullscreen();
   else if(['next','previous','pause','replay','black','white'].includes(data.action))v.action(data.action);
  }
 }
 async pumpPreviews(){if(this.previewBusy||!this.previewQueue?.length||!this.opened||!this.connected||!this.staticRenderer)return;const v=this.view;if(v.clock.running||v.switching||v.painting){clearTimeout(this.previewTimer);this.previewTimer=setTimeout(()=>this.pumpPreviews(),150);return;}
  this.previewBusy=true;const request=this.previewQueue.shift();try{const canvas=await this.staticRenderer.draw(request.index,Math.min(1280,Math.max(100,request.width||240)));if(!this.opened||request.generation!==this.connectionGeneration)return;const bitmap=await createImageBitmap(canvas);if(!this.opened||request.generation!==this.connectionGeneration){bitmap.close();return;}this.send({type:'preview-result',id:request.id,bitmap},[bitmap]);}catch(error){if(request.generation===this.connectionGeneration)this.send({type:'preview-result',id:request.id,error:error.message});}finally{this.previewBusy=false;if(this.opened)this.previewTimer=setTimeout(()=>this.pumpPreviews(),30);}
 }
 close(){return this.view.close();}
 async audienceClosed(){if(!this.opened)return;this.opened=false;this.connectionGeneration++;this.send({type:'ended'});this.port?.close();this.popup?.close();this.mirror?.dispose();clearInterval(this.watch);clearTimeout(this.handshakeTimer);clearTimeout(this.previewTimer);window.removeEventListener('message',this.handshake);document.removeEventListener('fullscreenchange',this.fullscreenListener);await this.staticRenderer?.dispose();this.staticRenderer=null;this.previewQueue=[];this.connected=false;const saved=this.saved;this.saved=null;Object.assign(this.view,{runtimeAdapter:saved.adapter,audience:saved.audience,onState:saved.onState});}
}
