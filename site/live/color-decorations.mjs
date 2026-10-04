import {Decoration,WidgetType,EditorView,ViewPlugin} from '@codemirror/view';
import {StateEffect,StateField,Transaction} from '@codemirror/state';
import {isolateHistory} from '@codemirror/commands';
import {colorPopover} from './color-popover.mjs';
let activeSession=null,opening=Promise.resolve();
const colorsEffect=StateEffect.define();
class Swatch extends WidgetType {
 constructor(info,source,edit){super();this.info=info;this.source=source;this.edit=edit;}
 eq(other){return other.source===this.source&&other.info.from===this.info.from&&other.info.to===this.info.to;}
 toDOM(view){const button=document.createElement('button');button.className='laymesh-color-swatch';button.type='button';button.setAttribute('aria-label','Edit color / 编辑颜色');button.title='Edit color / 编辑颜色';const [r,g,b,a]=this.info.rgba;button.style.background=`rgba(${r*255},${g*255},${b*255},${a})`;button.addEventListener('mousedown',e=>e.preventDefault());button.addEventListener('click',()=>this.edit(view,this.info,this.source,button));return button;}
 ignoreEvent(){return true;}
}
export function colorDecorations(editor,file,query){
 const field=StateField.define({create:()=>Decoration.none,update:(value,tr)=>{for(const effect of tr.effects)if(effect.is(colorsEffect))return effect.value;return value.map(tr.changes);},provide:f=>EditorView.decorations.from(f)});
 const edit=(view,clickedInfo,source,button)=>{
  // Serialize swatch switches; the previous close may have an async Rust query.
  const requestedDoc=view.state.doc;
  opening=opening.then(async()=>{
   let from=clickedInfo.from;
   if(activeSession){
    const previous=activeSession;if(previous.button===button)return;
    if(!await previous.control.commit())return;
    if(previous.view===view&&previous.doc===requestedDoc&&previous.change)from=previous.change.mapPos(from,1);
   }
   if(view.state.doc===requestedDoc&&view.state.doc.toString()!==source)return;
   const doc=view.state.doc,colors=await query(editor,file,'colors');if(view.state.doc!==doc)return;
   const info=colors?.find(c=>c.from===from);if(!info)return;
   // A source edit replaces swatch widgets. Resolve the current DOM anchor too.
   let anchor=button;if(!anchor.isConnected){let index=0;const widget=view.state.field(field).iter();while(widget.value){if(widget.from===from){anchor=view.dom.querySelectorAll('.laymesh-color-swatch')[index];break;}index++;widget.next();}}
   // Coordinates are also recoverable from the newly identified source position.
   const anchorRect=()=>anchor?.isConnected?anchor.getBoundingClientRect():view.coordsAtPos(info.from);
   const root=document.createElement('div');root.className='laymesh-color-popover';root.setAttribute('role','dialog');root.setAttribute('aria-modal','false');root.setAttribute('aria-label','Edit color / 编辑颜色');root.setAttribute('popover','manual');document.body.append(root);root.showPopover?.();
   const session={view,doc,button:anchor,change:null,control:null};activeSession=session;let positionFrame=0,disposed=false;const listeners=[];
   const listen=(target,type,fn,capture=false)=>{target.addEventListener(type,fn,capture);listeners.push(()=>target.removeEventListener(type,fn,capture));};
   const close=()=>{if(disposed)return;disposed=true;if(positionFrame)cancelAnimationFrame(positionFrame);observer.disconnect();listeners.forEach(fn=>fn());root.hidePopover?.();root.remove();if(activeSession===session)activeSession=null;view.focus();};
   session.control=colorPopover(root,info,async p=>{
    const checkDocument=()=>{if(view.state.doc!==doc){const error=Error('文档已变化，未写回。再次点击外部或按 Esc 关闭后重开 / Document changed; no edit applied. Close again and reopen.');error.code='STALE_COLOR';throw error;}};
    checkDocument();
    if(p.unchanged)return;
    const choices=await query(editor,file,'colorPresentations',0,{from:info.from,to:info.to,rgba:p.rgba});const choice=choices?.find(c=>c.space===p.space);if(!choice)throw Error('Color format unavailable');let text=choice.text;
    // Rust validates the color and chooses source syntax. Preserve floating
    // channels rather than its display-oriented six-digit presentations.
    if(p.space!=='hex'){const body=info.format==='constructor'?`${p.space}(${p.channels.join(', ')}, alpha=${p.alpha})`:`${p.space}(${p.channels.join(' ')} / ${p.alpha})`;text=info.format==='string'?`${info.quote}${body}${info.quote}`:body;}
    checkDocument();
    if(doc.sliceString(info.from,info.to)===text)return;
    const changes=view.state.changes({from:info.from,to:info.to,insert:text});session.change=changes;
    view.dispatch({changes,annotations:[Transaction.userEvent.of('input.color'),isolateHistory.of('full')]});
   },close,document.documentElement.lang);
   function position(){
    positionFrame=0;if(disposed)return;const r=anchorRect(),viewport=window.visualViewport;const left=viewport?.offsetLeft||0,top=viewport?.offsetTop||0,width=viewport?.width||innerWidth,height=viewport?.height||innerHeight;
    const visible=view.scrollDOM.getBoundingClientRect();
    if(!r||r.bottom<=Math.max(top,visible.top)||r.top>=Math.min(top+height,visible.bottom)||r.right<=Math.max(left,visible.left)||r.left>=Math.min(left+width,visible.right)){void session.control.commit();return;}
    const panel=session.control.element,w=panel.offsetWidth,h=panel.offsetHeight;
    const x=Math.max(left+8,Math.min(r.left,left+width-w-8));let y=r.bottom+6;
    if(y+h>top+height-8)y=r.top-h-6;
    y=Math.max(top+8,Math.min(y,top+height-h-8));
    const px=`${x}px`,py=`${y}px`;if(root.style.left!==px)root.style.left=px;if(root.style.top!==py)root.style.top=py;session.control.invalidateRects();
   }
   const schedulePosition=()=>{if(!positionFrame)positionFrame=requestAnimationFrame(position);};
   const observer=new ResizeObserver(schedulePosition);observer.observe(session.control.element);
   listen(window,'scroll',schedulePosition,true);listen(window,'resize',schedulePosition);if(window.visualViewport){listen(window.visualViewport,'scroll',schedulePosition);listen(window.visualViewport,'resize',schedulePosition);}
   listen(document,'pointerdown',e=>{if(!root.contains(e.target)&&!e.target.closest('.laymesh-color-swatch'))void session.control.commit();},true);
   listen(document,'keydown',e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();void session.control.commit();}},true);
   position();
  }).catch(error=>console.error('LayMesh color picker:',error));
 };
 const plugin=ViewPlugin.fromClass(class {
  constructor(view){this.view=view;this.dead=false;this.schedule();}
  schedule(){clearTimeout(this.timer);this.timer=setTimeout(async()=>{const source=this.view.state.doc.toString();const colors=await query(editor,file,'colors');if(this.dead||this.view.state.doc.toString()!==source)return;const widgets=(colors||[]).filter(c=>c.from>=0&&c.to<=source.length).map(info=>Decoration.widget({widget:new Swatch(info,source,edit),side:-1}).range(info.from));this.view.dispatch({effects:colorsEffect.of(Decoration.set(widgets,true))});},200);}
  update(update){if(update.docChanged)this.schedule();}
  destroy(){this.dead=true;clearTimeout(this.timer);if(activeSession?.view===this.view)activeSession.control.dispose();}
 });
 return [field,plugin,EditorView.theme({'.laymesh-color-swatch':{display:'inline-block',width:'12px',height:'12px',border:'1px solid #adc9ba',borderRadius:'2px',marginRight:'4px',padding:'0',cursor:'pointer',verticalAlign:'middle'}})];
}
