// The bar never reserves canvas space. Hidden mode remains keyboard accessible.
export class PresentationToolbar {
 constructor(view){
  this.view=view;this.abort=new AbortController();const {signal}=this.abort,root=view.root;
  root.addEventListener('pointermove',event=>this.pointer(event),{signal});
  root.addEventListener('pointerleave',()=>{this.hover=false;this.sync();},{signal});
  root.addEventListener('focusin',()=>this.sync(),{signal});
  root.addEventListener('focusout',event=>{
   // During a native pointer focus transfer, activeElement can briefly be the
   // body. Making the bar inert then would cancel the destination button click.
   if(view.bar.contains(event.relatedTarget))return;
   queueMicrotask(()=>{if(!signal.aborted)this.sync();});
  },{signal});
  this.sync();
 }
 setMode(mode){if(!['always','hover','hidden'].includes(mode))return;this.view.toolbarMode=mode;this.hover=false;this.sync();}
 pointer(event){
  if(this.view.toolbarMode!=='hover')return;
  const bar=this.view.bar.getBoundingClientRect(),root=this.view.root.getBoundingClientRect();
  this.hover=event.clientX>=bar.left-16&&event.clientX<=bar.right+16&&event.clientY>=bar.top-16&&event.clientY<=root.bottom;
  this.sync();
 }
 sync(){
  const v=this.view,mode=v.toolbarMode||'hidden';
  const visible=mode==='always'||mode==='hover'&&this.hover||v.bar.contains(document.activeElement);
  v.bar.classList.toggle('idle',!visible);v.bar.inert=!visible;v.bar.setAttribute('aria-hidden',String(!visible));v.bar.dataset.mode=mode;
  v.menuPanels.toolbar.querySelectorAll('[data-toolbar-mode]').forEach(b=>b.setAttribute('aria-checked',String(b.dataset.toolbarMode===mode)));
 }
 tab(event){
  const bar=this.view.bar,items=[...bar.querySelectorAll('button')].filter(b=>!b.disabled&&!b.hidden),at=items.indexOf(document.activeElement);
  if(!items.length)return;
  event.preventDefault();event.stopPropagation();bar.inert=false;bar.classList.remove('idle');bar.setAttribute('aria-hidden','false');
  items[at<0?(event.shiftKey?items.length-1:0):(at+(event.shiftKey?items.length-1:1))%items.length].focus({preventScroll:true});
 }
 dispose(){this.abort.abort();}
}
