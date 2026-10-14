import {uiIcon,iconLabel} from './presentation-icons.mjs';
const t=(zh,en)=>/^zh/.test(document.documentElement.lang)?zh:en;
const el=(tag,parent,text,className)=>{const node=document.createElement(tag);if(text)node.textContent=text;if(className)node.className=className;parent.append(node);return node;};
const separator=parent=>{const node=el('span',parent,null,'presentation-separator');node.setAttribute('role','separator');};
const button=(parent,label,icon,action)=>{const b=el('button',parent);b.type='button';b.title=label;b.setAttribute('aria-label',label);iconLabel(b,icon);b.onclick=action;return b;};
export function buildControls(v){
 v.overlay=el('div',v.root,null,'presentation-progress');v.overlay.setAttribute('role','status');
 const heading=el('div',v.overlay,null,'presentation-progress-heading');v.progressHeading=el('h2',heading,t('正在准备演示','Preparing presentation'));
 button(heading,t('取消并退出','Cancel and exit'),'close',()=>v.close());
 const row=el('div',v.overlay,null,'presentation-progress-count');v.progressCount=el('span',row);v.progressPercent=el('span',row);
 v.progressBar=el('progress',v.overlay);v.progressBar.max=1;v.message=el('p',v.overlay,null,'presentation-progress-stage');
 const actions=el('div',v.overlay,null,'presentation-progress-actions');v.retry=el('button',actions,t('重试','Retry'));v.retry.type='button';v.retry.className='presentation-primary';v.retry.hidden=true;v.retry.onclick=async()=>{v.retry.hidden=true;await v.retryPage();};
 const cancel=el('button',actions,t('取消','Cancel'));cancel.type='button';cancel.onclick=()=>v.close();
 v.bar=el('nav',v.root,null,'presentation-bar idle');v.bar.inert=true;v.bar.setAttribute('aria-hidden','true');v.bar.setAttribute('aria-label',t('放映控制','Presentation controls'));v.buttons={};
 const control=(id,label,icon)=>{const b=button(v.bar,label,icon,()=>v.action(id));b.dataset.action='present-'+id;v.buttons[id]=b;};
 control('previous',t('上一步','Previous step'),'previous');control('pause',t('播放','Play'),'play');control('next',t('下一步','Next step'),'next');separator(v.bar);
 control('pages',t('页面列表','Page list'),'page');v.pageCounter=el('span',v.buttons.pages,'1 / 1');v.buttons.pages.replaceChildren(v.pageCounter,uiIcon('down'));v.buttons.pages.className='presentation-page-button';v.buttons.pages.setAttribute('aria-haspopup','menu');
 control('menu',t('更多','More'),'more');v.buttons.menu.setAttribute('aria-haspopup','menu');control('fullscreen',t('全屏','Fullscreen'),'expand');separator(v.bar);control('exit',t('退出放映','End presentation'),'close');
 v.endNotice=el('button',v.root,t('放映结束 · 退出','Presentation ended · Exit'),'presentation-ended');v.endNotice.hidden=true;v.endNotice.onclick=()=>v.close();
 v.menu=el('div',v.root,null,'presentation-menu');v.menu.setAttribute('role','menu');v.menu.hidden=true;v.menuPanels={};
 for(const id of ['main','pages','speed','time','toolbar','help']){const panel=el('div',v.menu,null,'presentation-menu-panel');v.menuPanels[id]=panel;panel.hidden=id!=='main';if(id!=='main'){const back=button(panel,t('返回','Back'),'previous',()=>v.showPanel('main'));back.className='presentation-menu-back';el('span',back,({pages:t('跳到页面','Go to page'),speed:t('播放速度','Playback speed'),time:t('时间定位','Timeline'),toolbar:t('控制栏显示','Show controls'),help:t('快捷键','Keyboard shortcuts')})[id]);separator(panel);}}
 const menu=v.menuPanels.main;
 const item=(id,label,icon,hint='')=>{const b=button(menu,label,icon,()=>{const running=v.menuResume;v.hideMenu(id==='fullscreen');if(['pause','black','white'].includes(id))v.clock.running=running;v.action(id);});b.setAttribute('role','menuitem');el('span',b,label);if(hint)el('kbd',b,hint);return b;};
 item('previous',t('上一步','Previous step'),'previous','←');item('next',t('下一步','Next step'),'next','→');separator(menu);
 v.menuPause=item('pause',t('暂停','Pause'),'pause','S');item('replay',t('重播当前页','Replay page'),'replay');
 const sub=(id,label,icon)=>{const b=button(menu,label,icon,()=>v.showPanel(id));b.setAttribute('role','menuitem');el('span',b,label);b.append(uiIcon('next'));return b;};
 sub('pages',t('跳到页面…','Go to page…'),'page');separator(menu);
 v.mode=button(menu,t('自动播放','Automatic playback'),'play',()=>{const auto=v.clock.mode!=='auto';v.clock.mode=auto?'auto':'manual';v.menuResume=auto;v.sync();});v.mode.setAttribute('role','menuitemcheckbox');el('span',v.mode,t('自动播放','Automatic playback'));el('i',v.mode,null,'presentation-switch');
 v.speedRow=sub('speed',t('播放速度','Playback speed'),'speed');v.speedValue=el('output',v.speedRow);v.speedRow.insertBefore(v.speedValue,v.speedRow.lastElementChild.previousElementSibling);
 sub('time',t('时间定位','Timeline'),'time');separator(menu);item('black',t('黑屏','Black screen'),'black','B');item('white',t('白屏','White screen'),'white','W');separator(menu);
 v.menuFullscreen=item('fullscreen',t('全屏','Fullscreen'),'expand','F');sub('toolbar',t('控制栏显示…','Show controls…'),'more');sub('help',t('快捷键','Keyboard shortcuts'),'help');item('exit',t('结束放映','End presentation'),'close','Esc');
 for(const [mode,label]of [['always',t('一直显示','Always visible')],['hover',t('鼠标到位置显示','Show on hover')],['hidden',t('不显示','Hidden')]]){const b=button(v.menuPanels.toolbar,label,'check',()=>{v.toolbar.setMode(mode);v.hideMenu();});b.dataset.toolbarMode=mode;b.setAttribute('role','menuitemradio');el('span',b,label);}
 el('p',v.menuPanels.toolbar,t('Tab 随时呼出；点击画面收起。','Press Tab to show controls; click the slide to dismiss.'),'presentation-menu-note');
 v.pages=el('div',v.menuPanels.pages,null,'presentation-pages');
 for(const n of [.25,.5,1,1.5,2]){const b=button(v.menuPanels.speed,n+'×','check',()=>{v.clock.speed=n;v.hideMenu();v.sync();});b.dataset.speed=n;b.setAttribute('role','menuitemradio');el('span',b,n+'×');}
 v.label=el('output',v.menuPanels.time,null,'presentation-time');v.scrub=el('input',v.menuPanels.time);v.scrub.type='range';v.scrub.min=0;v.scrub.step=.001;v.scrub.setAttribute('aria-label',t('动画时间','Animation time'));v.scrub.oninput=()=>{v.clock.seek(Number(v.scrub.value));v.media?.seek(v.clock.time,v.deck.pages.slice(0,v.index).reduce((sum,p)=>sum+(p.inspection?.animation?.duration||0),0));v.menuMediaSnapshot=v.media?{...v.media.snapshot(),locked:false}:null;v.menuResume=false;v.paint();v.sync();};
 const help=el('dl',v.menuPanels.help);for(const [key,label]of [['Space / Enter / → / ↓ / N',t('推进；播放中完成当前步','Advance; finish current step while playing')],['← / ↑ / P / Backspace',t('回到上一步边界','Previous step boundary')],['Wheel ↓ / ↑',t('推进 / 回退；惯性只触发一次','Advance / Back; one action per gesture')],['PageUp / PageDown',t('回退 / 推进','Back / Forward')],['Home / End',t('首页初态 / 末页完成态','First page start / Last page end')],['0–9 + Enter',t('跳到指定页','Go to page number')],['S',t('暂停 / 继续','Pause / Resume')],['F',t('全屏 / 退出全屏','Fullscreen / Exit fullscreen')],['B / W',t('黑屏 / 白屏；再按恢复','Black / White; press again to restore')],['Tab',t('呼出控制栏 / 切换按钮','Show controls / Focus next button')],['Esc',t('关闭菜单 / 退出全屏 / 结束放映','Close menu / Exit fullscreen / End presentation')]]){el('dt',help,key);el('dd',help,label);}
}
export function populatePages(v){v.pages.replaceChildren();v.deck.pages.forEach((p,i)=>{const b=button(v.pages,p.name||p.id,'page',()=>{v.hideMenu(false);v.go(i);});b.dataset.pageIndex=i;b.setAttribute('role','menuitem');b.replaceChildren();el('span',b,String(i+1),'presentation-page-number');el('span',b,p.name||p.id);});}
export function syncControls(v){
 const running=v.clock.running||v.media?.running||(!v.menu.hidden&&(v.menuResume||v.menuMediaResume)),pause=t(running?'暂停':'播放',running?'Pause':'Play');iconLabel(v.buttons.pause,running?'pause':'play');v.buttons.pause.title=pause;v.buttons.pause.setAttribute('aria-label',pause);
 const menuPause=t(running?'暂停':'继续',running?'Pause':'Resume');v.menuPause.replaceChildren(uiIcon(running?'pause':'play'));el('span',v.menuPause,menuPause);el('kbd',v.menuPause,'S');v.menuPause.setAttribute('aria-label',menuPause);
 v.pageCounter.textContent=`${v.index+1} / ${v.deck.pages.length}`;v.endNotice.hidden=!v.ended;v.mode.setAttribute('aria-checked',String(v.clock.mode==='auto'));v.speedValue.textContent=v.clock.speed+'×';
 v.pages.querySelectorAll('button').forEach(b=>b.setAttribute('aria-current',String(Number(b.dataset.pageIndex)===v.index)));v.menuPanels.speed.querySelectorAll('[data-speed]').forEach(b=>b.setAttribute('aria-checked',String(Number(b.dataset.speed)===v.clock.speed)));
 v.label.textContent=`${v.clock.time.toFixed(1)} / ${v.clock.duration.toFixed(1)} s`;v.scrub.max=v.clock.duration;v.scrub.value=v.clock.time;
}
