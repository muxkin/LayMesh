let thumbnailId=0;
export class PageThumbnails {
 constructor({parent,pages,renderer,onSelect,paused=()=>false,horizontal=false}){
  Object.assign(this,{pages,renderer,onSelect,paused,horizontal});this.current=0;this.focused=0;this.cache=new Map();this.cacheBytes=0;this.nodes=new Map();this.generation=0;
  this.root=document.createElement('div');this.root.className='page-thumbnails'+(horizontal?' horizontal':'');this.root.tabIndex=0;this.root.setAttribute('role','listbox');this.root.setAttribute('aria-label','页面预览');parent.append(this.root);
  this.content=document.createElement('div');this.content.className='thumbnail-content';this.root.append(this.content);
  this.root.onscroll=()=>this.schedule();this.root.onkeydown=e=>{const step=['ArrowDown','ArrowRight'].includes(e.key)?1:['ArrowUp','ArrowLeft'].includes(e.key)?-1:0;
   if(step||e.key==='Home'||e.key==='End'){e.preventDefault();this.focused=e.key==='Home'?0:e.key==='End'?this.pages.length-1:Math.max(0,Math.min(this.pages.length-1,this.focused+step));this.scrollTo(this.focused);this.update();}
   else if(e.key==='Enter'||e.key===' '){e.preventDefault();this.onSelect(this.focused);}
  };
  this.resize=new ResizeObserver(()=>{this.update();clearTimeout(this.resizeTimer);this.resizing=true;this.resizeTimer=setTimeout(()=>{this.resizing=false;this.pump();},180);});this.resize.observe(this.root);this.update();
 }
 schedule(){cancelAnimationFrame(this.raf);this.raf=requestAnimationFrame(()=>this.update());}
 layout(){this.itemWidth=this.horizontal?Math.max(160,Math.floor((this.root.clientWidth-60)/6)):Math.max(100,this.root.clientWidth-24);this.offsets=[0];for(const p of this.pages){const size=p.inspection?.page||p.list||{width:16,height:9};this.offsets.push(this.offsets.at(-1)+(this.horizontal?this.itemWidth+12:Math.round(this.itemWidth*size.height/size.width)+48));}}
 update(){if(this.closed)return;this.layout();const pos=this.horizontal?this.root.scrollLeft:this.root.scrollTop,size=this.horizontal?this.root.clientWidth:this.root.clientHeight;
  this.content.style[this.horizontal?'width':'height']=this.offsets.at(-1)+'px';const visible=new Set();this.wanted=[];
  for(let i=0;i<this.pages.length;i++){if(this.offsets[i+1]<pos-200||this.offsets[i]>pos+size+200)continue;visible.add(i);let b=this.nodes.get(i);const p=this.pages[i];
   if(!b){b=document.createElement('button');b.type='button';b.className='page-thumbnail';b.tabIndex=-1;b.setAttribute('role','option');b.id='thumb-'+(++thumbnailId);b.dataset.pageIndex=i;const img=document.createElement('img');img.alt='';img.draggable=false;const label=document.createElement('span');label.textContent=`${i+1}  ${p.name||p.id}`;b.append(img,label);b.onclick=()=>{this.focused=i;this.onSelect(i);};this.content.append(b);this.nodes.set(i,b);}
   b.style[this.horizontal?'left':'top']=this.offsets[i]+'px';b.style.width=this.itemWidth+'px';b.setAttribute('aria-selected',String(i===this.current));b.classList.toggle('keyboard-focus',i===this.focused&&document.activeElement===this.root);
   if(i===this.focused)this.root.setAttribute('aria-activedescendant',b.id);
   const cached=this.cache.get(i);if(cached){b.firstChild.src=cached.url;cached.used=performance.now();}this.wanted.push(i);
  }
  for(const [i,b]of this.nodes)if(!visible.has(i)){b.remove();this.nodes.delete(i);}this.pump();
 }
 async pump(){if(this.closed||this.busy)return;if(this.paused()||this.resizing){clearTimeout(this.timer);this.timer=setTimeout(()=>this.pump(),150);return;}
  const width=Math.min(1024,Math.ceil(this.itemWidth*Math.min(devicePixelRatio||1,2))),index=this.wanted.find(i=>(this.cache.get(i)?.width||0)<width*.9&&!this.failed?.has(i));if(index===undefined)return;
  this.busy=true;const generation=this.generation;try{const canvas=await this.renderer.draw(index,width);if(!canvas||this.closed||generation!==this.generation)return;const blob=await new Promise(r=>canvas.toBlob(r,'image/png'));if(!blob||this.closed||generation!==this.generation)return;
   const old=this.cache.get(index);if(old){URL.revokeObjectURL(old.url);this.cacheBytes-=old.bytes;}const item={url:URL.createObjectURL(blob),width,bytes:canvas.width*canvas.height*4,used:performance.now()};this.cache.set(index,item);this.cacheBytes+=item.bytes;
   if(this.nodes.has(index))this.nodes.get(index).firstChild.src=item.url;
   for(const [i,c]of [...this.cache].sort((a,b)=>a[1].used-b[1].used)){if(this.cacheBytes<=32*1048576)break;if(this.nodes.has(i))continue;URL.revokeObjectURL(c.url);this.cache.delete(i);this.cacheBytes-=c.bytes;}
  }catch(error){(this.failed??=new Set()).add(index);const b=this.nodes.get(index);if(b){b.title='预览暂不可用：'+error.message;b.onclick=()=>{this.failed.delete(index);this.onSelect(index);this.pump();};}}
  finally{this.busy=false;this.timer=setTimeout(()=>this.pump(),30);}
 }
 select(index){if(index===this.current)return;this.current=this.focused=index;this.scrollTo(index);this.update();}
 scrollTo(index){this.layout();const start=this.offsets[index],end=this.offsets[index+1],key=this.horizontal?'scrollLeft':'scrollTop',size=this.horizontal?this.root.clientWidth:this.root.clientHeight;if(start<this.root[key])this.root[key]=start;else if(end>this.root[key]+size)this.root[key]=end-size;}
 reset(pages){this.generation++;this.pages=pages;this.failed?.clear();for(const c of this.cache.values())URL.revokeObjectURL(c.url);this.cache.clear();this.cacheBytes=0;this.nodes.forEach(n=>n.remove());this.nodes.clear();this.update();}
 dispose(){this.closed=true;this.generation++;this.resize.disconnect();clearTimeout(this.resizeTimer);clearTimeout(this.timer);cancelAnimationFrame(this.raf);for(const c of this.cache.values())URL.revokeObjectURL(c.url);this.cache.clear();this.nodes.clear();this.renderer.dispose();this.root.remove();}
}
