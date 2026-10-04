const $=s=>document.querySelector(s);const $$=s=>[...document.querySelectorAll(s)];
const en=document.documentElement.lang==='en';const root=document.documentElement;
const get=(key,fallback)=>{try{return localStorage.getItem(key)||fallback;}catch{return fallback;}};
const save=(key,value)=>{try{localStorage.setItem(key,value);}catch{}};
$('.theme-toggle')?.addEventListener('click',()=>{root.dataset.theme=root.dataset.theme==='dark'?'light':'dark';save('laymesh-theme',root.dataset.theme);});
const desktopNav=matchMedia('(min-width:1280px)');const desktopToc=matchMedia('(min-width:1024px)');
const menu=$('.menu-toggle');const scrim=$('.mobile-scrim');
function syncNav(){const open=desktopNav.matches?root.dataset.nav!=='closed':document.body.classList.contains('menu-open');menu?.setAttribute('aria-expanded',String(open));scrim.hidden=desktopNav.matches||!open;}
function closeMenu(){document.body.classList.remove('menu-open');syncNav();}
menu?.addEventListener('click',()=>{if(desktopNav.matches){root.dataset.nav=root.dataset.nav==='closed'?'open':'closed';save('laymesh-nav',root.dataset.nav);}else document.body.classList.toggle('menu-open');syncNav();});
scrim?.addEventListener('click',closeMenu);desktopNav.addEventListener('change',closeMenu);$$('.sidebar a').forEach(a=>a.addEventListener('click',closeMenu));syncNav();
const toc=$('.floating-toc');const tocButton=$('.toc-toggle');const tocKey=()=>`laymesh-toc-${desktopToc.matches?'desktop':'small'}`;
function setToc(open,persist=false){root.dataset.toc=open?'open':'closed';tocButton?.setAttribute('aria-expanded',String(open));if(persist)save(tocKey(),root.dataset.toc);}
function initToc(){setToc(get(tocKey(),desktopToc.matches?'open':'closed')==='open');}
tocButton?.addEventListener('click',()=>setToc(root.dataset.toc==='closed',true));desktopToc.addEventListener('change',initToc);initToc();
$$('#page-toc a').forEach(a=>a.addEventListener('click',()=>{if(!desktopToc.matches)setToc(false);}));
const sections=$$('#page-toc a').map(a=>({link:a,heading:document.getElementById(decodeURIComponent(a.hash.slice(1)))})).filter(x=>x.heading);
let scrollPending=false;function trackSection(){scrollPending=false;const active=[...sections].reverse().find(x=>x.heading.getBoundingClientRect().top<=150)||sections[0];for(const item of sections)item.link.setAttribute('aria-current',String(item===active));}
addEventListener('scroll',()=>{if(!scrollPending){scrollPending=true;requestAnimationFrame(trackSection);}},{passive:true});trackSection();
function selectSource(module,mode,tab,updateUrl=true){
 const tabs=[...module.querySelectorAll('.source-files [role=tab]')];
 tab=tab||tabs.find(t=>t.getAttribute('aria-selected')==='true')||tabs[0];
 module.dataset.mode=mode;
 module.querySelectorAll('[data-source-mode]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.sourceMode===mode)));
 module.querySelector('.summary-panel').hidden=mode!=='summary';
 module.querySelector('.source-files').hidden=mode==='summary'||tabs.length<2;
 for(const t of tabs){const active=t===tab;t.setAttribute('aria-selected',String(active));t.tabIndex=active?0:-1;document.getElementById(t.getAttribute('aria-controls')).hidden=mode==='summary'||!active;}
 module.dispatchEvent(new CustomEvent('laymesh:tab'));
 if(updateUrl){const id=mode==='summary'?module.querySelector('.summary-panel').id:tab.getAttribute('aria-controls');history.replaceState(null,'','#'+id);if(langLink)langLink.href=languageBase+location.hash;}
}
$$('[data-source-mode]').forEach(button=>button.addEventListener('click',()=>selectSource(button.closest('.example-module'),button.dataset.sourceMode)));
$$('.source-files [role=tab]').forEach(tab=>{
 tab.addEventListener('click',()=>selectSource(tab.closest('.example-module'),'source',tab));
 tab.addEventListener('keydown',e=>{const tabs=[...tab.parentElement.querySelectorAll('[role=tab]')];let index=tabs.indexOf(tab);if(e.key==='ArrowRight')index=(index+1)%tabs.length;else if(e.key==='ArrowLeft')index=(index-1+tabs.length)%tabs.length;else if(e.key==='Home')index=0;else if(e.key==='End')index=tabs.length-1;else return;e.preventDefault();selectSource(tab.closest('.example-module'),'source',tabs[index]);tabs[index].focus();});
});
document.addEventListener('laymesh:select-source',e=>selectSource(e.target.closest('.example-module'),'source',e.target.closest('.example-module').querySelector('.source-files [role=tab]')));
const langLink=$('.lang-switch');const languageBase=langLink?.getAttribute('href');
function hashState(){let id;try{id=decodeURIComponent(location.hash.slice(1));}catch{return;}const panel=document.getElementById(id);if(panel?.matches('.summary-panel,[role=tabpanel]')){const module=panel.closest('.example-module');selectSource(module,panel.matches('.summary-panel')?'summary':'source',panel.matches('.summary-panel')?null:document.getElementById(panel.getAttribute('aria-labelledby')),false);module.scrollIntoView();}if(langLink)langLink.href=languageBase+(/^example-|^L\d+$/.test(id)?'#'+encodeURIComponent(id):'');}
addEventListener('hashchange',hashState);hashState();
$$('.wrap-code').forEach(button=>button.addEventListener('click',()=>{const wrapped=button.closest('.code-block').classList.toggle('wrap');button.setAttribute('aria-pressed',String(wrapped));}));
$$('.copy-code').forEach(button=>button.addEventListener('click',async()=>{const block=button.closest('.code-block');const code=block.getSource?.()??block.querySelector('.code-raw').value;try{if(navigator.clipboard?.writeText)await navigator.clipboard.writeText(code);else{const field=document.createElement('textarea');field.value=code;field.style.cssText='position:fixed;opacity:0';document.body.append(field);field.select();if(!document.execCommand('copy'))throw new Error('Copy failed');field.remove();button.focus();}button.textContent=en?'Copied':'已复制';}catch{button.textContent=en?'Copy failed':'复制失败';}setTimeout(()=>button.textContent=en?'Copy':'复制',1500);}));
const search=$('.search-dialog');const input=search?.querySelector('input');const results=$('.search-results');const searchBase=new URL(JSON.parse($('#search-data').textContent).base,location.href);const pages=(window.LAYMESH_SEARCH||[]).map(p=>({...p,url:new URL(p.url,searchBase).href}));
function updateSearch(){const query=input.value.trim().toLocaleLowerCase();const words=query.split(/\s+/).filter(Boolean);const matches=pages.map(p=>({...p,score:p.title.toLocaleLowerCase().includes(query)?2:1})).filter(p=>words.every(w=>`${p.title} ${p.group} ${p.terms}`.toLocaleLowerCase().includes(w))).sort((a,b)=>b.score-a.score).slice(0,80);results.replaceChildren();for(const p of matches){const a=document.createElement('a');a.href=p.url;a.className='search-result';for(const text of [p.title,p.group]){const span=document.createElement('span');span.textContent=text;a.append(span);}results.append(a);}if(!matches.length){const p=document.createElement('p');p.className='search-hint';p.textContent=en?'No matching topics':'没有找到匹配的主题';results.append(p);}}
function openSearch(){search.showModal();input.value='';updateSearch();input.focus();}$('.search-trigger')?.addEventListener('click',openSearch);input?.addEventListener('input',updateSearch);
input?.addEventListener('keydown',e=>{if(e.key==='ArrowDown'){e.preventDefault();results.querySelector('a')?.focus();}});
results?.addEventListener('keydown',e=>{const links=[...results.querySelectorAll('a')];const i=links.indexOf(document.activeElement);if(e.key==='ArrowDown'){e.preventDefault();links[Math.min(i+1,links.length-1)]?.focus();}if(e.key==='ArrowUp'){e.preventDefault();if(i<=0)input.focus();else links[i-1].focus();}});
$$('[data-filter]').forEach(button=>button.addEventListener('click',()=>{$$('[data-filter]').forEach(b=>b.setAttribute('aria-pressed',String(b===button)));$$('.gallery-grid [data-category]').forEach(card=>card.hidden=button.dataset.filter!=='all'&&card.dataset.category!==button.dataset.filter);}));
const lightbox=$('.image-dialog');const stage=$('.image-stage');const large=lightbox?.querySelector('img');let zoom=1,fitWidth=0,lastImage=null;
function zoomTo(value){zoom=Math.max(.25,Math.min(8,value));large.style.width=fitWidth*zoom+'px';large.style.height='auto';$('.image-reset').textContent=Math.round(zoom*100)+'%';}
$$('.preview-image').filter(img=>!img.closest('a,.example-module')).forEach(img=>{img.tabIndex=0;img.setAttribute('role','button');img.setAttribute('aria-label',(en?'Enlarge: ':'放大：')+img.alt);const open=()=>{lastImage=img;large.src=img.dataset.full||img.currentSrc;large.alt=img.alt;lightbox.querySelector('p').textContent=img.alt;lightbox.showModal();const ratio=Number(img.dataset.aspectRatio)||(Number(img.getAttribute('width'))||img.naturalWidth)/(Number(img.getAttribute('height'))||img.naturalHeight)||1;fitWidth=Math.min(stage.clientWidth-40,(stage.clientHeight-40)*ratio);zoomTo(1);stage.scrollTo(0,0);};img.addEventListener('click',open);img.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();open();}});});
$('.image-zoom-in')?.addEventListener('click',()=>zoomTo(zoom*1.5));$('.image-zoom-out')?.addEventListener('click',()=>zoomTo(zoom/1.5));$('.image-reset')?.addEventListener('click',()=>zoomTo(1));$('.image-close')?.addEventListener('click',()=>lightbox.close());lightbox?.addEventListener('close',()=>lastImage?.focus());
let drag;stage?.addEventListener('pointerdown',e=>{if(e.button!==0)return;drag={x:e.clientX,y:e.clientY,left:stage.scrollLeft,top:stage.scrollTop};stage.setPointerCapture(e.pointerId);e.preventDefault();});stage?.addEventListener('pointermove',e=>{if(drag){stage.scrollLeft=drag.left-(e.clientX-drag.x);stage.scrollTop=drag.top-(e.clientY-drag.y);}});stage?.addEventListener('pointerup',()=>drag=null);stage?.addEventListener('pointercancel',()=>drag=null);
document.addEventListener('keydown',e=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'){e.preventDefault();openSearch();}if(e.key==='/'&&!search.open&&!['INPUT','TEXTAREA'].includes(document.activeElement?.tagName)&&!document.activeElement?.closest('.cm-editor')){e.preventDefault();openSearch();}if(e.key==='Escape'&&!document.querySelector('dialog[open]:not(.search-dialog):not(.image-dialog)')){if(document.body.classList.contains('menu-open')){closeMenu();menu.focus();}else if(!search.open&&!lightbox.open&&root.dataset.toc==='open'){setToc(false);tocButton?.focus();}}});
try{const legacyTarget=document.getElementById(decodeURIComponent(location.hash.slice(1)));if(legacyTarget?.classList.contains('legacy-redirect'))location.replace(legacyTarget.href);}catch{}

if(document.querySelector('.example-module[data-entry]'))import(new URL('./live/editor.js',import.meta.url));
document.addEventListener('laymesh:lightbox',e=>{const img=e.detail.image;lastImage=img;const ratio=Number(img.dataset.aspectRatio)||(Number(img.getAttribute('width'))||img.naturalWidth)/(Number(img.getAttribute('height'))||img.naturalHeight)||1;fitWidth=Math.min(stage.clientWidth-40,(stage.clientHeight-40)*ratio);zoomTo(1);});
