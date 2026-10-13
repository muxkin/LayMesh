import {PresentationWheel} from './presentation-wheel.mjs';
// Shared navigation, menus and shortcuts for preview and detached presentation.
export class PresentationInteraction {
 bindInput({signal,slideTarget=e=>e.target===this.root||e.target===this.canvas||e.target===this.mask}={}){
  this.wheelInput=new PresentationWheel();this.root.addEventListener('wheel',e=>this.wheel(e),{signal,passive:false});
  this.root.addEventListener('click',e=>{if(!slideTarget(e)||e.button!==0||e.ctrlKey||e.metaKey||e.altKey)return;const dismiss=this.bar.contains(document.activeElement);this.root.focus({preventScroll:true});if(!this.menu.hidden){this.hideMenu();return;}if(!dismiss&&!this.mediaClick?.(e))this.action('next');},{signal});
  this.root.addEventListener('contextmenu',e=>{if(e.target.closest('input,select'))return;e.preventDefault();e.stopPropagation();this.root.focus({preventScroll:true});this.showMenu(e.clientX,e.clientY);},{signal});
  this.root.addEventListener('keydown',e=>this.key(e),{signal});
 }
 blank(color){if(color){if(this.mask.hidden)this.blankResume=this.clock.running;this.clock.running=false;this.mask.dataset.color=color;this.mask.style.background=color;this.mask.hidden=false;}else{this.mask.hidden=true;this.clock.running=!!this.blankResume;this.last=performance.now();}}
 showPanel(id){for(const [name,panel]of Object.entries(this.menuPanels))panel.hidden=name!==id;this.panel=id;this.positionMenu();this.menuPanels[id].querySelector('button,input')?.focus();}
 positionMenu(){const r=this.root.getBoundingClientRect(),{x,y}=this.menuAnchor;this.menu.style.left=Math.max(8,Math.min(r.width-this.menu.offsetWidth-8,(x??r.left+r.width/2+160-this.menu.offsetWidth)-r.left))+'px';this.menu.style.top=Math.max(8,Math.min(r.height-this.menu.offsetHeight-8,(y??r.bottom-80-this.menu.offsetHeight)-r.top))+'px';}
 showMenu(x,y,panel='main'){if(!this.opened||!this.ready)return;if(this.menu.hidden){this.menuResume=this.clock.running;this.menuMediaResume=this.media?.enabled&&!this.media.paused;this.media?.pause();this.clock.running=false;}this.menu.hidden=false;this.menuAnchor={x,y};this.showPanel(panel);this.buttons[panel==='pages'?'pages':'menu'].setAttribute('aria-expanded','true');this.sync();}
 hideMenu(resume=true){this.menu.hidden=true;this.buttons.menu.setAttribute('aria-expanded','false');this.buttons.pages.setAttribute('aria-expanded','false');if(resume&&this.menuResume){this.clock.running=true;this.last=performance.now();}if(resume&&this.menuMediaResume)this.media?.resume();this.menuMediaResume=false;this.menuResume=false;this.root.focus({preventScroll:true});this.sync();}
 key(e){if(e.ctrlKey||e.metaKey||e.altKey||e.repeat)return;if(e.key==='Tab'){if(this.menu.hidden&&this.overlay.hidden){this.toolbar.tab(e);return;}const scope=this.overlay.hidden?this.menu:this.overlay,items=[...scope.querySelectorAll('button,select,input')].filter(x=>x.getClientRects().length&&!x.disabled),at=items.indexOf(document.activeElement);if(items.length&&(at<0||e.shiftKey&&at===0||!e.shiftKey&&at===items.length-1)){e.preventDefault();items[e.shiftKey?items.length-1:0].focus();}return;}if(!this.menu.hidden&&e.target.tagName!=='INPUT'&&['ArrowDown','ArrowUp'].includes(e.key)){const items=[...this.menu.querySelectorAll('button,select,input')].filter(x=>x.getClientRects().length),at=items.indexOf(document.activeElement);items[(at+(e.key==='ArrowDown'?1:items.length-1))%items.length].focus();e.preventDefault();return;}if(e.key==='Escape'){e.preventDefault();e.stopPropagation();if(!this.menu.hidden){if(this.panel==='main')this.hideMenu();else this.showPanel('main');}else this.close();return;}if(e.target.closest('input,select,button')||!this.deck)return;const key=e.key.toLowerCase();let action;if(/^\d$/.test(key)){this.digits=(this.digits||'')+key;clearTimeout(this.digitTimer);this.digitTimer=setTimeout(()=>this.digits='',2000);e.preventDefault();return;}
  if(key==='enter'&&this.digits){const n=Number(this.digits);this.digits='';if(n>=1&&n<=this.deck.pages.length)this.go(n-1);e.preventDefault();return;}
  if([' ','enter','arrowright','arrowdown','pagedown','n'].includes(key))action='next';else if(['arrowleft','arrowup','pageup','backspace','p'].includes(key))action='previous';else if(key==='s')action='pause';else if(key==='f')action='fullscreen';else if(key==='b'||key==='.')action='black';else if(key==='w'||key===',')action='white';else if(key==='home'){this.go(0);action='handled';}else if(key==='end'){this.go(this.deck.pages.length-1,{end:true});action='handled';}
  if(action){e.preventDefault();e.stopPropagation();if(action!=='handled')this.action(action);}
 }
 wheel(e){
  if(!this.ready||!this.root.contains(document.activeElement)||e.ctrlKey||e.metaKey||e.altKey||e.shiftKey||Math.abs(e.deltaX)>=Math.abs(e.deltaY))return;
  if(e.target.closest('.context-menu-list,.presentation-menu,.presentation-bar,.presentation-progress,.preview-menu,.preview-bar,.preview-progress,input,select,button'))return;
  e.preventDefault();e.stopPropagation();
  // Consume the gesture even while switching, preventing its inertial tail
  // from becoming an action immediately after the target page appears.
  const direction=this.wheelInput.consume(e,performance.now(),this.root.clientHeight);
  if(this.externalMenu||!this.menu.hidden||this.switching||!direction)return;
  this.root.focus({preventScroll:true});this.action(direction>0?'next':'previous');
 }

}
