import {colorPanel} from './color-panel.mjs';
const api=acquireVsCodeApi();let seq=0;const pending=new Map();
const request=(method,params)=>new Promise((resolve,reject)=>{const id=++seq;pending.set(id,{resolve,reject});api.postMessage({id,method,params});});
window.addEventListener('message',({data})=>{if(data.type==='initial'){colorPanel(document.body,data.color,p=>request('apply',p),()=>api.postMessage({method:'close'}),data.locale);}else{const p=pending.get(data.id);if(p){pending.delete(data.id);data.error?p.reject(Error(data.error)):p.resolve(data.result);}}});
api.postMessage({method:'ready'});
