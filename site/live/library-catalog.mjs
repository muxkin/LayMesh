// One real compiler/renderer workspace for the entire pinned library inventory.
export function setupLibraryCatalog(editor,module){
 const catalog=module.closest('.style-library-catalog');if(!catalog)return;
 const en=document.documentElement.lang==='en',t=(zh,eng)=>en?eng:zh;
 const library=JSON.parse(catalog.querySelector('.library-data').textContent),entries=library.entries;
 const items=catalog.querySelector('.library-items'),search=catalog.querySelector('.library-search'),kind=catalog.querySelector('.library-kind'),status=catalog.querySelector('.library-status'),detail=catalog.querySelector('.library-detail'),count=catalog.querySelector('.library-count');
 const tuning=catalog.querySelector('.library-tuning'),play=catalog.querySelector('.library-play'),staticButton=catalog.querySelector('.library-static');
 const kinds={class:t('类','Class'),variable:t('变量','Variable'),keyframe:t('关键帧','Keyframe')};
 const localizedReason=reason=>reason.split('; ').map(part=>{const pair=part.split(' / ');return pair[en?0:1]||pair[0];}).join(en?'; ':'；');
 const labels={supported:t('可用','Supported'),partial:t('部分支持','Partial'),resources:t('需要额外资源','Needs resources'),unsupported:t('不支持 / 未验证','Unsupported / unverified')};
 const requested=new URLSearchParams(location.search).get('entry');
 let selected=entries.find(e=>e.id===requested||e.name===requested)||entries.find(e=>e.name===catalog.dataset.default),revision=0,preparing=false;
 const words=e=>[e.name,e.kind,e.family.join(' '),e.consumer,e.reason,...e.declarations.map(d=>d.property)].join(' ').toLowerCase();
 const searchable=new Map(entries.map(e=>[e.id,words(e)]));
 function filter(){
  const term=search.value.trim().toLowerCase();const matches=entries.filter(e=>(!kind.value||e.kind===kind.value)&&(!status.value||e.status===status.value)&&(!term||searchable.get(e.id).includes(term)));
  items.replaceChildren(...matches.map(e=>{const o=document.createElement('option');o.value=e.id;o.textContent=`${e.name} · ${labels[e.status]}`;o.selected=e.id===selected?.id;return o;}));
  if(!matches.some(e=>e.id===selected?.id))items.selectedIndex=-1;
  count.textContent=t(`${matches.length} / ${entries.length} 项 · ${library.name} ${library.version}`,`${matches.length} / ${entries.length} entries · ${library.name} ${library.version}`);
 }
 function describe(){
  const e=selected;catalog.dataset.selected=e.id;detail.replaceChildren();
  const heading=document.createElement('h3');heading.textContent=e.name;const state=document.createElement('p');state.className='library-support';state.dataset.status=e.status;state.textContent=labels[e.status]+' · '+kinds[e.kind]+' · '+e.family[en?1:0];
  const reason=document.createElement('p');reason.textContent=localizedReason(e.reason);
  const origin=document.createElement('a');origin.href=library.source;origin.textContent=`${library.name} ${library.version} · ${library.license} · `+t('上游资源','Upstream resource');
  const pre=document.createElement('pre'),code=document.createElement('code');code.textContent=e.kind==='class'?e.declarations.map(d=>`${d.property}: ${d.value}${d.important?' !important':''};`).join('\n'):e.value;pre.append(code);
  detail.append(heading,state,reason,origin,pre);
  const permalink=document.createElement('a'),url=new URL(location.href);url.searchParams.set('entry',e.id);permalink.href=url.href;permalink.textContent=t('此条目的完整案例链接','Link to this complete recipe');detail.append(permalink);
  if(e.kind==='variable'){
   const usage=document.createElement('p');usage.textContent=e.consumer?`${e.consumer}: var(${e.name})`:(t('变量可参与 var()；此项尚无经过验证的外观用法。','The token can be referenced with var(); no appearance recipe has been verified.'));detail.append(usage);
   if(e.defaultValue!==undefined&&e.defaultValue!==null){const active=document.createElement('p');active.textContent=t('默认 screen 条件的根值：','Root value under default screen conditions: ')+JSON.stringify(e.defaultValue);detail.append(active);}
  }
  if(e.definitions?.length>1){const variants=document.createElement('details'),summary=document.createElement('summary');summary.textContent=t('所有源码定义与作用域','All source definitions and scopes');variants.append(summary);for(const def of e.definitions){const p=document.createElement('pre');p.textContent=(def.selector||'@keyframes')+'\n'+def.value;variants.append(p);}detail.append(variants);}
  const usable=!!e.sample;module.hidden=!usable;tuning.hidden=!usable;play.disabled=!usable;staticButton.disabled=!usable;
 }
 async function setSource(source,token=++revision){
  preparing=true;editor.player.action('static');editor.playAfterPaint=false;module.dataset.playRequested='false';
  module.dispatchEvent(new CustomEvent('laymesh:select-source',{bubbles:true}));
  const panel=module.querySelector('[data-edit-file="'+editor.entry+'"]');await editor.editor(panel);
  if(token!==revision)return;
  const view=editor.views.get(editor.entry)?.view;if(!view){preparing=false;return;}
  view.dispatch({changes:{from:0,to:view.state.doc.length,insert:source}});editor.activate();editor.schedule(true);preparing=false;
 }
 items.addEventListener('change',()=>{
  const entry=entries.find(e=>e.id===items.value);if(!entry)return;
  ++revision;preparing=false;editor.player.action('static');editor.playAfterPaint=false;module.dataset.playRequested='false';selected=entry;describe();
  catalog.querySelector('.library-value').value='';catalog.querySelector('.library-dpi').value='';
  if(entry.kind==='variable'){const prop=catalog.querySelector('.library-property');if(![...prop.options].some(o=>o.value===entry.name)){const option=document.createElement('option');option.value=entry.name;option.textContent=entry.name;prop.append(option);}prop.value=entry.name;}
  if(entry.sample)setSource(entry.sample,revision);
 });
 for(const input of [search,kind,status])input.addEventListener('input',filter);
 catalog.querySelector('.library-apply').addEventListener('click',async()=>{
  if(!selected.sample)return;const panel=module.querySelector('[data-edit-file="'+editor.entry+'"]');await editor.editor(panel);
  const view=editor.views.get(editor.entry)?.view;if(!view)return;
  let source=view.state.doc.toString().replace(/\n# Catalog controls\nstyle \{[\s\S]*?\n\}\n# End catalog controls\n/g,'\n');
  const value=catalog.querySelector('.library-value').value.trim(),property=catalog.querySelector('.library-property').value,dpi=catalog.querySelector('.library-dpi').value;
  if(dpi){source=source.replace(/layout_dpi=[\d.]+,/g,'').replace('page=canvas(',`page=canvas(layout_dpi=${dpi},`);}
  if(value){source=source.replace(/class="([^"\n]+)"/g,(_,classes)=>`class="${classes.split(' ').filter(c=>c!=='catalog-custom').join(' ')} catalog-custom"`);source=source.replace('\nitem=',`\n# Catalog controls\nstyle {\n.catalog-custom { ${property}: ${value} !important; }\n}\n# End catalog controls\nitem=`);}
  await setSource(source);
 });
 play.addEventListener('click',()=>{if(!selected.sample)return;module.dataset.playRequested='true';editor.activate();if(!preparing&&module.dataset.liveState==='success'&&editor.presented&&editor.player.metadata){module.dataset.playRequested='false';editor.player.action('replay');}});
 staticButton.addEventListener('click',()=>{editor.playAfterPaint=false;module.dataset.playRequested='false';editor.player.action('static');});
 module.querySelector('.live-reset').addEventListener('click',()=>{++revision;preparing=false;selected=entries.find(e=>e.name===catalog.dataset.default);search.value='';kind.value='';status.value='';catalog.querySelector('.library-value').value='';catalog.querySelector('.library-dpi').value='';describe();filter();});
 describe();filter();if(selected.sample&&selected.name!==catalog.dataset.default)setSource(selected.sample);
}
