import {autocompletion,completionKeymap,acceptCompletion,closeCompletion,insertCompletionText,pickedCompletion} from '@codemirror/autocomplete';
import {hoverTooltip,keymap,showTooltip,EditorView,ViewPlugin} from '@codemirror/view';
import {linter} from '@codemirror/lint';
import {StateField,StateEffect,Prec} from '@codemirror/state';
import {indentMore,indentLess} from '@codemirror/commands';
import {helpDOM,signatureDOM,positionInfo,helpTheme} from './help.mjs';
import {css} from '@codemirror/lang-css';
import {colorDecorations} from './color-decorations.mjs';
import {StreamLanguage} from '@codemirror/language';
const en=document.documentElement.lang==='en';
let worker,serial=0,workspaceSerial=0;const workspaces=new WeakMap();const requests=new Map();
function query(editor,file,method,offset=0,params={}){
 if(!workspaces.has(editor))workspaces.set(editor,++workspaceSerial);
 if(!worker){worker=new Worker(new URL('language-worker.js',import.meta.url),{type:'module'});worker.onmessage=({data})=>{const pending=requests.get(data.id);if(pending){clearTimeout(pending.timer);requests.delete(data.id);pending.resolve(data.error?undefined:data.result);}};worker.onerror=()=>{for(const p of requests.values()){clearTimeout(p.timer);p.resolve(undefined);}requests.clear();worker.terminate();worker=null;};}
 return new Promise(resolve=>{const id=++serial,timer=setTimeout(()=>{requests.delete(id);resolve(undefined);},5000);requests.set(id,{resolve,timer});worker.postMessage({id,workspace:workspaces.get(editor),file,offset,method,params,files:{...editor.files},locale:en?'en':'zh'});});
}
// Stateful highlighting preserves multi-line strings and embedded LCSS regions.
export const layLanguage=StreamLanguage.define({
 startState:()=>({quote:null,raw:false,math:false,css:0,comment:false}),
 token(s,state){
  if(state.comment){if(s.skipTo('*/')){s.match('*/');state.comment=false;}else s.skipToEnd();return 'comment';}
  if(state.quote){
   if(s.match(state.quote)){state.quote=null;state.math=false;return 'string';}
   if(!state.raw&&s.match(/^\$\$?/)){state.math=!state.math;return 'operator';}
   if(!state.raw&&s.match(/^\{[^{}]*\}/))return 'variableName';
   if(s.match(/^\\./))return state.math?'keyword':'escape';
   s.next();while(!s.eol()&&!s.match(state.quote,false)&&!['$','\\','{'].includes(s.peek()))s.next();return state.math?'keyword':'string';
  }
  if(s.eatSpace())return null;
  if(s.match(/^\/\*/)){state.comment=true;return 'comment';}
  if(!state.css&&s.match(/^#.*/))return 'comment';
  const quote=s.match(/^(rf|fr|r|f)?("""|'''|"|')/);if(quote){state.quote=quote[2];state.raw=(quote[1]||'').includes('r');return 'string';}
  if(s.match(/^style\s*(?=\{)/)){state.css=1;return 'keyword';}
  if(s.peek()==='{'){s.next();if(state.css)state.css++;return 'bracket';}
  if(s.peek()==='}'){s.next();if(state.css&&--state.css===1)state.css=0;return 'bracket';}
  if(state.css&&s.match(/^#[\da-f]{3,8}\b/i))return 'color';
  if(s.match(/^(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?/i))return 'number';
  if(s.match(/^(?:for|in|not|and|or|while|break|continue|if|else|return|function|import|export|from|true|false|null|auto|mm|cm|inch|pt|px|deg|rad|s|ms)\b/))return 'keyword';
  if(s.match(/^[\w-]+(?=\s*(?:=|:))/))return 'propertyName';
  if(s.match(/^[a-z_]\w*(?=\s*\()/i))return 'function';
  if(s.match(/^[a-z_]\w*/i))return 'variableName';
  s.next();return null;
 }
});
const signatureEffect=StateEffect.define();
const signatureField=StateField.define({create:()=>null,update:(value,tr)=>{for(const effect of tr.effects)if(effect.is(signatureEffect))return effect.value;return tr.docChanged||tr.selection?null:value;},provide:f=>showTooltip.from(f)});
export function languageSupport(editor,file){
 if(file.endsWith(".css"))return [css(),autocompletion({defaultKeymap:false}),Prec.highest(keymap.of([
  {key:'Tab',run:view=>!view.composing&&(acceptCompletion(view)||indentMore(view)),shift:view=>!view.composing&&indentLess(view)},
  {key:'Enter',run:view=>{if(!view.composing)closeCompletion(view);return false;}},
  ...completionKeymap.filter(binding=>binding.key!=='Enter').map(binding=>({...binding,run:view=>!view.composing&&binding.run(view)}))
 ]))];
 const current=()=>editor.views.get(file)?.view;
 const active=()=>editor.module.dataset.mode!=='summary'&&editor.module.querySelector('.source-panel:not([hidden])')?.dataset.editFile===file;
 const signature=view=>{const pos=view.state.selection.main.head,doc=view.state.doc;query(editor,file,'signature',pos).then(result=>{if(result&&active()&&current()===view&&view.state.doc===doc&&view.state.selection.main.head===pos)view.dispatch({effects:signatureEffect.of({pos,above:true,create:()=>({dom:signatureDOM(result,view)})})});});};
 const jump=async view=>{
  const result=await query(editor,file,'definition',view.state.selection.main.head);if(!result)return;
  const locations=Array.isArray(result)?result:[result];
  let target=locations[0];
  if(locations.length>1){
   const picker=document.createElement('dialog');picker.className='definition-picker';
   const title=document.createElement('p');title.textContent=en?'Choose a definition':'选择定义';picker.append(title);
   target=await new Promise(resolve=>{for(const d of locations){const b=document.createElement('button');b.textContent=d.uri+' · '+d.from;b.onclick=()=>{picker.close();resolve(d);};picker.append(b);}picker.addEventListener('cancel',()=>resolve(null),{once:true});document.body.append(picker);picker.showModal();});picker.remove();if(!target)return;
  }
  if(target.uri.startsWith('laymesh-library:/')){
   const content=await query(editor,target.uri,'stylesheetContent');if(!content)return;
   const dialog=document.createElement('dialog');dialog.className='library-source-dialog';const title=document.createElement('p');title.textContent=target.uri+' · '+(en?'Read only':'只读');
   const close=document.createElement('button');close.textContent=en?'Close':'关闭';close.onclick=()=>dialog.close();const pre=document.createElement('pre');pre.tabIndex=0;const code=document.createElement('code');
   code.append(document.createTextNode(content.text.slice(0,target.from)));const mark=document.createElement('mark');mark.textContent=content.text.slice(target.from,target.to)||' ';code.append(mark,document.createTextNode(content.text.slice(target.to)));pre.append(code);dialog.append(title,close,pre);document.body.append(dialog);dialog.addEventListener('close',()=>dialog.remove(),{once:true});dialog.showModal();mark.scrollIntoView({block:'center'});return;
  }
  const tab=[...editor.module.querySelectorAll('[role=tab]')].find(tab=>editor.module.querySelector('#'+CSS.escape(tab.getAttribute('aria-controls')))?.dataset.editFile===target.uri);tab?.click();const panel=editor.module.querySelector('.source-panel:not([hidden])');if(panel?.dataset.editFile===target.uri)await editor.editor(panel);const destination=editor.views.get(target.uri)?.view;if(destination){destination.dispatch({selection:{anchor:target.from}});destination.focus();}
 };
 const dependencyRefresh=ViewPlugin.fromClass(class {
  constructor(view){this.refresh=()=>{closeCompletion(view);view.dispatch({effects:signatureEffect.of(null)});};editor.module.addEventListener('laymesh:tab',this.refresh);}
  destroy(){editor.module.removeEventListener('laymesh:tab',this.refresh);}
 });
 return [colorDecorations(editor,file,query),helpTheme,dependencyRefresh,/\.(lcss|css)$/.test(file)?css():layLanguage,autocompletion({defaultKeymap:false,interactionDelay:0,positionInfo,override:[async context=>{
  const options=await query(editor,file,'completions',context.pos);if(!options?.length)return null;
  // CodeMirror filters the entire result range. Keep it at the cursor so an
  // unfinished suffix ("ro|xx") does not hide "round", while each accepted
  // option still replaces the complete range supplied by the language service.
  return {from:Math.min(...options.map(o=>o.from)),to:context.pos,options:options.map(({from,to,info,...o})=>{
   const tail=Math.max(0,(to??context.pos)-context.pos),text=o.apply??o.label;
   return {...o,apply:tail?(view,completion,start,end)=>view.dispatch({...insertCompletionText(view.state,text,start,end+tail),annotations:pickedCompletion.of(completion)}):o.apply,info:info?()=>helpDOM(info):undefined};
  }),validFor:/^[\w@./-]*$/};
 }]}),hoverTooltip(async(view,pos)=>{const doc=view.state.doc,result=await query(editor,file,'hover',pos);if(!result||view.state.doc!==doc)return null;return{pos:result.from,end:result.to,above:true,create:()=>({dom:helpDOM(result.contents)})};}),
 linter(async view=>{const doc=view.state.doc,result=await query(editor,file,'diagnostics');if(view.state.doc!==doc)return [];return(result||[]).map(d=>({...d,from:Math.min(doc.length,d.from),to:Math.min(doc.length,d.to),actions:d.replacement?[{name:en?'Rename':'改名',apply:(v,from,to)=>v.dispatch({changes:{from,to,insert:d.replacement}})}]:undefined}));},{delay:350}),signatureField,
 Prec.highest(keymap.of([
  {key:'Tab',run:view=>!view.composing&&(acceptCompletion(view)||indentMore(view)),shift:view=>!view.composing&&indentLess(view)},
  {key:'Enter',run:view=>{if(!view.composing)closeCompletion(view);return false;}},
  {key:'Escape',run:view=>{const signature=!!view.state.field(signatureField,false);view.dispatch({effects:signatureEffect.of(null)});return closeCompletion(view)||signature;}},
  ...completionKeymap.filter(binding=>!['Enter','Escape'].includes(binding.key)).map(binding=>({...binding,run:view=>!view.composing&&binding.run(view)})),
  {key:'F12',run:view=>{jump(view);return true;}},
  {key:'Mod-Shift-Space',run:view=>{signature(view);return true;}}
 ]))];
}
