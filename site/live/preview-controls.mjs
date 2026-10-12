import {PresentationInteraction} from './presentation-interaction.mjs';
import {PresentationToolbar} from './presentation-toolbar.mjs';
import {buildControls,populatePages,syncControls} from './presentation-controls.mjs';

export class PreviewControls extends PresentationInteraction {
 constructor(editor){
  super();this.editor=editor;this.player=editor.player;this.root=editor.workspace.viewport;this.root.classList.add('preview-presentation');this.opened=true;
  this.abort=new AbortController();this.mask=document.createElement('div');this.mask.className='presentation-mask';this.mask.hidden=true;this.root.append(this.mask);buildControls(this);this.overlay.hidden=true;
  for(const [node,kind]of [[this.mask,'mask'],[this.bar,'bar'],[this.menu,'menu'],[this.overlay,'progress'],[this.pages,'pages'],[this.endNotice,'ended']])node.className=node.className.replace('presentation-'+kind,'preview-'+kind);
  for(const [id,button]of Object.entries(this.buttons))button.dataset.action='preview-'+id;
  this.scrub.setAttribute('aria-label',/^zh/.test(document.documentElement.lang)?'预览动画时间':'Preview animation time');
  this.buttons.fullscreen.hidden=true;this.menuFullscreen.hidden=true;this.toolbar=new PresentationToolbar(this);
  this.mode.onclick=()=>{this.clock.mode=this.clock.mode==='auto'?'manual':'auto';this.menuResume=this.clock.mode==='auto'&&this.player.time!==null;this.sync();};
  this.bindInput({signal:this.abort.signal,slideTarget:e=>{if(e.target.closest('button,input,select,.preview-menu,.preview-bar,.preview-progress'))return false;if(editor.workspace.suppressPreviewClick){editor.workspace.suppressPreviewClick=false;return false;}return true;}});
  editor.workspace.previewControls=this;
  document.addEventListener('visibilitychange',()=>{if(document.hidden){this.clock.running=false;this.player.refresh();}},{signal:this.abort.signal});
  document.addEventListener('pointerdown',e=>{if(!this.menu.hidden&&!this.root.contains(e.target))this.hideMenu();},{signal:this.abort.signal});
  this.resize=new ResizeObserver(()=>{if(!this.menu.hidden)this.positionMenu();});this.resize.observe(this.root);
 }
 get clock(){return this.player.clock;}
 update(){
  const pages=this.editor.pageSelect?[...this.editor.pageSelect.options].map(p=>({id:p.value,name:p.textContent.replace(/^\d+\.\s*/, '')})):[{id:this.editor.pageId||'preview',name:''}];
  this.deck={pages};this.index=Math.max(0,this.editor.pageSelect?.selectedIndex||0);this.ready=!!this.editor.last;populatePages(this);this.ended=false;this.mask.hidden=true;this.menu.hidden=true;this.menuResume=false;this.sync();
 }
 action(id){
  if(id==='exit'){this.close();return;}if(id==='fullscreen'){this.editor.presentation.open({fullscreen:true});return;}
  if(id==='menu'||id==='pages'){this.showMenu(undefined,undefined,id==='pages'?'pages':'main');return;}
  if(!this.ready)return;
  if(id==='next'){if(!this.mask.hidden){this.blank(null);return;}this.player.navigate('next');}
  else if(id==='previous')this.player.navigate('previous');else if(id==='pause')this.player.action('play');else if(id==='replay')this.player.action('replay');
  else if(id==='black'||id==='white')this.blank(this.mask.hidden||this.mask.dataset.color!==id?id:null);
  this.last=performance.now();this.sync();
 }
 paint(){if(this.player.time===null)this.player.time=this.clock.time;this.player.refresh();}
 blank(color){super.blank(color);this.sync();}
 sync(){
  if(!this.deck||this.syncing)return;this.syncing=true;
  try{if(this.clock.running!==this.player.running||this.player.time!==null&&this.clock.time!==this.player.time){this.player.last=performance.now();this.player.refresh();}syncControls(this);this.root.dataset.running=String(this.player.running);this.root.dataset.time=this.player.time===null?'static':String(this.clock.time);this.root.dataset.completed=String(this.clock.completed);this.player.mode.value=this.clock.mode;this.player.speed=this.clock.speed;if(this.player.speedSelect)this.player.speedSelect.value=String(this.clock.speed);}finally{this.syncing=false;}
 }
 go(index,{end=false}={}){
  const select=this.editor.pageSelect;
  if(select&&index!==select.selectedIndex){this.player.resume={end,automatic:this.clock.mode==='auto'};select.selectedIndex=index;select.onchange();return;}
  const start=this.player.restorePage({end,automatic:false});this.paint();if(start)this.player.start();this.sync();
 }
 close(){this.clock.running=false;this.mask.hidden=true;if(!this.menu.hidden)this.hideMenu(false);this.player.action('static');this.ended=false;this.sync();}
 dispose(){this.abort.abort();this.toolbar.dispose();this.resize.disconnect();clearTimeout(this.digitTimer);}
}
