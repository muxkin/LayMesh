import {iconLabel,uiIcon} from './presentation-icons.mjs';
const t=(zh,en)=>/^zh/.test(document.documentElement.lang)?zh:en;
// One fullscreen entry; alternative modes stay in its dropdown.
export function presentationEntry(view){
 const group=document.createElement('div');group.className='presentation-entry-group';
 const main=view.fullscreenButton;main.dataset.action='present';main.setAttribute('aria-label',t('放映','Present'));iconLabel(main,'play',t('放映','Present'));
 const choices=document.createElement('details');choices.className='presentation-entry-menu';const trigger=document.createElement('summary');trigger.setAttribute('aria-label',t('放映选项','Presentation options'));trigger.title=trigger.getAttribute('aria-label');trigger.append(uiIcon('down'));
 const popover=document.createElement('div');popover.className='presentation-entry-popover';
 const windowed=view.button;windowed.dataset.action='present-window-entry';iconLabel(windowed,'window-play',t('窗口放映','Windowed presentation'));
 const fullscreen=document.createElement('button');fullscreen.type='button';fullscreen.dataset.action='present-fullscreen-entry';iconLabel(fullscreen,'expand',t('全屏放映','Fullscreen presentation'));fullscreen.disabled=main.disabled;
 main.onclick=()=>{choices.open=false;view.open({fullscreen:true});};windowed.onclick=()=>{choices.open=false;view.open({fullscreen:false});};fullscreen.onclick=main.onclick;
 popover.append(windowed,fullscreen);choices.append(trigger,popover);group.append(main,choices);
 choices.addEventListener('toggle',()=>{if(choices.open)group.closest('.example-module')?.querySelectorAll('details[open]:not(.dependencies)').forEach(other=>{if(other!==choices&&!other.contains(choices))other.open=false;});});
 document.addEventListener('pointerdown',e=>{if(!group.contains(e.target))choices.open=false;});
 choices.addEventListener('keydown',e=>{if(e.key==='Escape'){choices.open=false;trigger.focus();e.preventDefault();e.stopPropagation();}});
 return group;
}
