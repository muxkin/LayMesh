// Recreate source-scheduled posters for videos compiled in the browser (no FFmpeg).
// Independent preview transport must never change an exported poster.
export async function mediaPosters(list,resources,cancelled=()=>false){
 if(!Object.keys(list.media?.assets||{}).length)return {list,resources};
 list=structuredClone(list);resources={...resources};const decoders=new Map();
 const check=()=>{if(cancelled())throw new DOMException('Export cancelled','AbortError');};
 const wait=(video,event)=>new Promise((resolve,reject)=>{
  let settled=false;const polling=setInterval(()=>{if(cancelled())done(new DOMException('Export cancelled','AbortError'));},100);
  const done=error=>{if(settled)return;settled=true;clearInterval(polling);clearTimeout(timer);video.removeEventListener(event,ready);video.removeEventListener('error',failed);error?reject(error):resolve();};
  const ready=()=>done(),failed=()=>done(Error('Cannot decode video poster'));const timer=setTimeout(()=>done(Error('Timed out decoding video poster')),30000);
  video.addEventListener(event,ready,{once:true});video.addEventListener('error',failed,{once:true});
 });
 const image=n=>n.kind.kind==='image'?n:n.kind.kind==='group'?n.kind.children.map(image).find(Boolean):null;
 const walk=async nodes=>{for(const n of nodes){check();if(n.hint?.mediaId&&n.name){
   if(n.hint.mediaPosterExplicit)continue;
   const asset=n.hint.mediaId,meta=list.media.assets[asset],picture=image(n);if(!picture)continue;
   let decoder=decoders.get(asset);if(!decoder){const video=document.createElement('video'),url=URL.createObjectURL(new Blob([resources[asset]],{type:meta.mime}));decoder={video,url};decoders.set(asset,decoder);video.muted=true;video.playsInline=true;video.preload='auto';const loaded=wait(video,'loadeddata');video.src=url;await loaded;}
   const video=decoder.video,time=list.media.cues.find(c=>c.target===n.name)?.trimStart||0;
   if(Math.abs(video.currentTime-time)>.0001){const sought=wait(video,'seeked');video.currentTime=time;await sought;}check();
   const c=n.hint.mediaPosterCropped?n.hint.mediaCrop:null,x=c?Math.round(c.offset[0]*video.videoWidth):0,y=c?Math.round(c.offset[1]*video.videoHeight):0,w=c?Math.max(1,Math.round(c.size[0]*video.videoWidth)):video.videoWidth,h=c?Math.max(1,Math.round(c.size[1]*video.videoHeight)):video.videoHeight;
   const canvas=new OffscreenCanvas(w,h);canvas.getContext('2d').drawImage(video,x,y,w,h,0,0,w,h);const bytes=new Uint8Array(await(await canvas.convertToBlob({type:'image/png'})).arrayBuffer());
   resources[picture.kind.resource]=bytes;list.resources[picture.kind.resource]={...list.resources[picture.kind.resource],mime:'image/png'};
   if(c)n.hint.mediaPosterSize=[video.videoWidth,video.videoHeight];
  }else if(n.kind.kind==='group')await walk(n.kind.children);}};
 try{await walk(list.nodes);return {list,resources};}finally{for(const{video,url}of decoders.values()){video.removeAttribute('src');video.load();URL.revokeObjectURL(url);}}
}
