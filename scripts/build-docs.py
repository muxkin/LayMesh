#!/usr/bin/env python3
"""Build bilingual static docs and live Rust/WASM examples without Node.js."""
from __future__ import annotations
import argparse,hashlib,html,importlib.util,json,os,posixpath,re,shutil,subprocess,sys,tempfile
from pathlib import Path
from urllib.parse import unquote
from jinja2 import Environment,FileSystemLoader,select_autoescape
from markdown_it import MarkdownIt
from site_support import ROOT,examples,dependencies
import page_fonts
OUT=ROOT/'site/dist';ENV=Environment(loader=FileSystemLoader(ROOT/'site/templates'),autoescape=select_autoescape());NAV=json.loads((ROOT/'site/navigation.json').read_text());EXAMPLES=examples();EX={e['source']:e for e in EXAMPLES};PAGES=[];BY_KEY={};BY_SOURCE={};MEDIA=json.loads((ROOT/'site/media/manifest.json').read_text())
STYLESHEET_VERSIONS={name:hashlib.sha256((ROOT/'site'/name).read_bytes()).hexdigest()[:16] for name in ['styles.css','live/presentation.css']}
def relative(page,target):return posixpath.relpath(str(target),posixpath.dirname(page['output']) or '.')
def asset(page,name):
 url=relative(page,'site/'+name)
 return url+'?v='+STYLESHEET_VERSIONS[name] if name in STYLESHEET_VERSIONS else url
def output_name(source,en):
 if source=='README.md':return 'index.html'
 if source=='README.en.md':return 'en/index.html'
 if source=='docs/README.zh-CN.md':return 'docs/index.html'
 if source=='docs/gallery/README.zh-CN.md':return 'docs/gallery/index.html'
 name=source.replace('.en.md','.html').replace('.md','.html').replace('README.html','index.html');return ('en/' if en else '')+name

def register(page):PAGES.append(page);BY_KEY[(page['key'],page['lang'])]=page;BY_SOURCE[page.get('source',page['key']+page['lang'])]=page

def paired(key,stem,titles,**extra):
 sources=stem if isinstance(stem,list) else [stem+'.zh-CN.md',stem+'.en.md']
 for i,lang in enumerate(['zh-CN','en']):
  source=sources[i]
  if not (ROOT/source).is_file():continue
  register(dict(key=key,source=source,lang=lang,title=titles[i],output=output_name(source,i==1),**extra))
def discover():
 paired('home',['README.md','README.en.md'],['首页','Home'],kind='home')
 for area in NAV['areas']:
  paired(area['key'],'docs/sections/'+area['key'],area['title'],area=area['key'])
  for group in area['groups']:
   for key in group['pages']:
    topic=next((t for t in NAV['topics'] if t['key']==key),None)
    if topic:paired(key,'docs/topics/'+key,topic['title'],area=area['key'],group=group['key'])
 paired('site',['site/README.md','site/README.en.md'],['文档构建','Building documentation'],area='reference')
 for doc in sorted((ROOT/'docs').rglob('*.md')):
  source=doc.relative_to(ROOT).as_posix()
  if source in BY_SOURCE:continue
  en=source.endswith('.en.md');key=source.removesuffix('.en.md').removesuffix('.zh-CN.md').removesuffix('.md');title=next((line.lstrip('# ').strip() for line in doc.read_text().splitlines() if line.startswith('# ')),doc.stem)
  register(dict(key=key,source=source,lang='en' if en else 'zh-CN',title=title,output=output_name(source,en),area='reference'))
 for e in EXAMPLES:
  for i,lang in enumerate(['zh-CN','en']):register(dict(key='example-'+e['id'],lang=lang,title=e['title'][i],output=('en/' if i else '')+'examples/'+e['id']+'.html',example=e['source'],kind='example',area='examples'))
 for area in NAV['areas']:
  if area['key']=='examples':
   for group in area['groups']:
    category={'layout-examples':'layout','plot-examples':'plots','notebook-examples':'notebook','applications':'applications'}[group['key']]
    group['pages']=['example-'+e['id'] for e in EXAMPLES if e['category']==category]
    for key in group['pages']:
     for lang in ['zh-CN','en']:BY_KEY[(key,lang)]['group']=group['key']
 for page in list(PAGES):
  if 'source' not in page:continue
  text=(ROOT/page['source']).read_text()
  for link in re.findall(r'(?<!!)\[[^]]*\]\(([^)]+)\)',text):
   target=posixpath.normpath(posixpath.join(posixpath.dirname(page['source']),link.split('#')[0]))
   if (ROOT/target).is_file() and target not in BY_SOURCE and target not in EX and Path(target).suffix.lower() in ('.py','.rs','.toml','.json','.csv','.lay','.laydoc','.lcss','.css','.txt','.yml','.yaml','.mjs','.ts','.md'):
    for lang in ['zh-CN','en']:
     key='source-'+target
     if (key,lang) not in BY_KEY:register(dict(key=key,file=target,lang=lang,title=Path(target).name,output=('en/' if lang=='en' else '')+'source/'+target+'.html',kind='source',area='reference'))

def code(text,language='lay',name='',line=0):
 rows=''.join(f'<span class="code-line"><span class="line-number" aria-hidden="true">{i+1+line}</span><span class="line-text">{html.escape(s) or " "}</span></span>' for i,s in enumerate(text.rstrip('\n').split('\n')))
 return f'<div class="code-block wrap"><div class="code-topbar"><span>{html.escape(name or language)}</span><div><button type="button" class="wrap-code" aria-pressed="true">Wrap</button><button type="button" class="copy-code">Copy</button></div></div><pre><code>{rows}</code></pre><textarea class="code-raw" hidden>{html.escape(text)}</textarea></div>'
def picture(page,source,alt=''):
 item=MEDIA['items'].get(source) or MEDIA['items'].get(re.sub(r'(?:-preview)?\.(png|jpe?g|webp)$','.lay',source))
 if item:
  variants=item['variants'];full=variants[-1];src=relative(page,variants[min(1,len(variants)-1)]['file']);srcset=', '.join(relative(page,v['file'])+' '+str(v['width'])+'w' for v in variants)
  return f'<img class="preview-image" src="{src}" srcset="{srcset}" sizes="(min-width:900px) 45vw, 90vw" width="{full["width"]}" height="{full["height"]}" data-full="{relative(page,full["file"])}" alt="{html.escape(alt)}" loading="lazy" decoding="async">'
 if (ROOT/source).is_file():return f'<img class="preview-image" src="{relative(page,source)}" alt="{html.escape(alt)}">'
 return '<div class="preview-image">'+html.escape(alt or source)+'</div>'
def context(page):
 en=page['lang']=='en'
 def get(key):return BY_KEY.get((key,page['lang'])) or BY_KEY.get(('home',page['lang']))
 return dict(page=page,navigation=NAV,tr=lambda zh,en_text:en_text if en else zh,translated=lambda values:values[int(en)],href=lambda key:relative(page,get(key)['output']),known=lambda key:(key,page['lang']) in BY_KEY,title=lambda key:get(key)['title'],asset=lambda f:asset(page,f),counterpart=relative(page,BY_KEY.get((page['key'],'zh-CN' if en else 'en'),BY_KEY[('home','zh-CN' if en else 'en')])['output']),search_base=json.dumps({'base':relative(page,'index.html')}))
def example_module(page,source):
 e=EX.get(source)
 if not e:return ''
 deps=[f for f in dependencies(source) if Path(f).suffix.lower() not in ('.ttf','.otf','.ttc','.otc','.woff','.woff2')];files=sorted([f for f in deps if Path(f).suffix in ('.lay','.laydoc','.lcss','.css','.csv','.json')],key=lambda f:(f!=source,f));python=source.removesuffix('.lay')+'.py'
 if (ROOT/python).is_file() and python not in files:files.append(python)
 panels=[]
 for i,file in enumerate(files):panels.append(dict(file=file,editable=Path(file).suffix in ('.lay','.laydoc','.lcss','.css','.csv','.json'),name=Path(file).name,id='example-'+e['id']+('-source' if i==0 else '-'+re.sub(r'[^\w-]','-',Path(file).name)),code=code((ROOT/file).read_text(),Path(file).suffix[1:],Path(file).name)))
 has_animation=any(re.search(r'\.animate\s*\(', (ROOT/f).read_text()) for f in deps if Path(f).suffix=='.lay')
 return ENV.get_template('example.html').render(**context(page),example=e,has_animation=has_animation,panels=panels,preview=picture(page,source,e['title'][page['lang']=='en']),dependencies=deps)
def gallery(page):
 return '<div class="example-grid gallery-grid">'+''.join(f'<a class="example-card" data-category="{e["category"]}" href="{relative(page,BY_KEY[("example-"+e["id"],page["lang"])]["output"])}">{picture(page,e["source"],e["title"][page["lang"]=="en"])}<strong>{html.escape(e["title"][page["lang"]=="en"])}</strong></a>' for e in EXAMPLES)+'</div>'
def rewrite(page,url):
 if not url or re.match(r'(?:#|[a-z]+:|//)',url,re.I):return url
 parts=url.split('#',1);target=posixpath.normpath(posixpath.join(posixpath.dirname(page.get('source',page.get('file','README.md'))),parts[0]));dest=BY_SOURCE.get(target,{}).get('output')
 if target in EX:dest=BY_KEY[('example-'+EX[target]['id'],page['lang'])]['output']
 if not dest and ('source-'+target,page['lang']) in BY_KEY:dest=BY_KEY[('source-'+target,page['lang'])]['output']
 return relative(page,dest or target)+('#'+parts[1] if len(parts)>1 else '')
def markdown(page,text):
 placeholders={}
 def catalog_embed(_):
  effects=json.loads((ROOT/'site/animation-effects.json').read_text());en=page['lang']=='en';index=int(en)
  options=[]
  for group in dict.fromkeys(e['group'][index] for e in effects):
   options.append('<optgroup label="'+html.escape(group,quote=True)+'">')
   for e in effects:
    if e['group'][index]==group:options.append('<option value="'+e['name']+'"'+(' selected' if e['name']=='fadeInUp' else '')+'>'+e['class']+'</option>')
   options.append('</optgroup>')
  key=f'LAYMESHBLOCK{len(placeholders)}'
  controls='<div class="effect-controls"><label>'+('效果 ' if not en else 'Effect ')+'<select class="effect-select" aria-label="'+('动画效果' if not en else 'Animation effect')+'">'+''.join(options)+'</select></label><button type="button" class="effect-play">'+('播放 / 重播' if not en else 'Play / Replay')+'</button><button type="button" class="effect-static">'+('完整排版' if not en else 'Static')+'</button></div><p class="effect-description" aria-live="polite"></p>'
  metadata='<script type="application/json" class="effect-data">'+json.dumps(effects,ensure_ascii=False).replace('<','\\u003c')+'</script>'
  placeholders[key]='<section class="animation-catalog">'+controls+metadata+example_module(page,'examples/animation/catalog.lay')+'</section>'
  return '\n\n'+key+'\n\n'
 text=re.sub(r'<!-- animation-catalog -->',catalog_embed,text)

 def library_embed(match):
  lib=next(l for l in json.loads((ROOT/'site/style-libraries.json').read_text())['libraries'] if l['name']==match[1]);en=page['lang']=='en'
  tr=lambda zh,eng:eng if en else zh
  default={'bootstrap':'bg-primary','open-props':'--gradient-7','animate':'animate__fadeInUp','effects':'fx-bevel-button'}[lib['name']]
  status=[('',tr('所有状态','All statuses')),('supported',tr('可用','Supported')),('partial',tr('部分支持','Partial')),('resources',tr('需要字体等资源','Needs resources')),('unsupported',tr('不支持 / 未验证','Unsupported / unverified'))]
  kinds=[('',tr('所有类型','All kinds')),('class',tr('类','Classes')),('variable',tr('变量','Variables')),('keyframe',tr('关键帧','Keyframes'))]
  select=lambda cls,values:'<select class="'+cls+'" aria-label="'+cls+'">'+''.join('<option value="'+v+'">'+label+'</option>' for v,label in values)+'</select>'
  controls='<div class="library-filter"><label>'+tr('检索','Search')+'<input class="library-search" type="search" placeholder="'+tr('类名、变量、属性或类别','Class, variable, property or category')+'"></label>'+select('library-kind',kinds)+select('library-status',status)+'</div><p class="library-count" aria-live="polite"></p><div class="library-browser"><select class="library-items" size="9" aria-label="'+tr('库条目','Library entries')+'"></select><div class="library-detail" aria-live="polite"></div></div>'
  properties=['background-color','border-radius','box-shadow-blur','box-shadow-spread','box-shadow-offset','box-shadow-color','color','font-size','opacity','padding','animation-duration','animation-delay','animation-iteration-count']
  if lib['name']=='effects':properties=['three-d','effects',*properties]
  controls+='<details class="library-tuning"><summary>'+tr('调参：选择属性并输入项目单位','Adjust a property using project units')+'</summary><div class="library-parameters"><label>layout_dpi<input class="library-dpi" type="number" min="1" placeholder="96"></label><label>'+tr('覆盖属性','Override property')+select('library-property',[(v,v) for v in properties])+'</label><label>'+tr('值','Value')+'<input class="library-value" placeholder="'+tr('例如 2mm、#e8590c、0.8s','e.g. 2mm, #e8590c, 0.8s')+'"></label><button type="button" class="library-apply">'+tr('应用覆盖','Apply override')+'</button></div><p>'+tr('完整源码可继续编辑。切换条目会载入该项的原始案例；恢复清除修改。动画时间定位位于预览的“更多 → 播放控制”。','Edit the full source too. Selecting an entry loads its original recipe; Restore clears edits. Seek animation time under More → Playback controls.')+'</p></details>'
  controls+='<div class="effect-controls"><button type="button" class="library-play">'+tr('播放 / 重播','Play / Replay')+'</button><button type="button" class="library-static">'+tr('完整排版','Static')+'</button></div>'
  metadata='<script type="application/json" class="library-data">'+json.dumps(lib,ensure_ascii=False,separators=(',',':')).replace('<','\\u003c')+'</script>'
  key=f'LAYMESHBLOCK{len(placeholders)}';placeholders[key]='<section class="style-library-catalog" data-default="'+default+'">'+controls+metadata+example_module(page,'examples/libraries/catalog-'+lib['name']+'.lay')+'</section>'
  return '\n\n'+key+'\n\n'
 text=re.sub(r'<!-- style-catalog:(bootstrap|open-props|animate|effects) -->',library_embed,text)

 def embed(m):
  key=f'LAYMESHBLOCK{len(placeholders)}';placeholders[key]=example_module(page,m[2]) if m[1]=='example' else code((ROOT/m[2]).read_text(),Path(m[2]).suffix[1:],Path(m[2]).name) if m[1]=='source' else gallery(page) if m[2]=='examples' else overview(page,m[2]);return '\n\n'+key+'\n\n'
 text=re.sub(r'<!-- (example|source|overview):([^ ]+) -->',embed,text)
 md=MarkdownIt('commonmark',{'html':True,'linkify':False}).enable('table')
 def fence(tokens,i,options,env):return code(tokens[i].content,tokens[i].info)
 md.renderer.rules['fence']=fence
 rendered=md.render(text)
 rendered=re.sub(r'<a href="([^"]+)"',lambda m:'<a href="'+html.escape(rewrite(page,html.unescape(m[1])),quote=True)+'"',rendered)
 def image(m):
  src=html.unescape(m[1]);alt=html.unescape(m[2] or '')
  if re.match(r'(?:https?:|data:)',src):return m[0]
  source=posixpath.normpath(posixpath.join(posixpath.dirname(page.get('source','README.md')),src));return picture(page,source,alt)
 rendered=re.sub(r'<img src="([^"]+)" alt="([^"]*)"[^>]*>',image,rendered)
 used=set(re.findall(r'\bid="([^"]+)"',rendered))
 def heading(m):
  raw=re.sub(r'<[^>]+>','',m[2]);raw=raw.replace('.', '-') if re.fullmatch(r'[A-Za-z_]\w*\.\w+',raw) else raw;slug=re.sub(r'[^\w\s-]','',html.unescape(raw)).strip().lower();slug=re.sub(r'\s+','-',slug);base=slug;i=1
  while slug in used:i+=1;slug=f'{base}-{i}'
  used.add(slug);return f'<h{m[1]} id="{slug}">{m[2]}<a class="heading-anchor" href="#{slug}">#</a></h{m[1]}>'
 rendered=re.sub(r'<h([1-6])>(.*?)</h\1>',heading,rendered,flags=re.S)
 for key,value in placeholders.items():rendered=rendered.replace('<p>'+key+'</p>',value)
 return rendered
def overview(page,key):
 area=next((a for a in NAV['areas'] if a['key']==key),None)
 if not area:return ''
 ctx=context(page)
 return '<div class="directory">'+''.join('<section class="directory-row"><h2>'+html.escape(ctx['translated'](g['title']))+'</h2><div>'+''.join(f'<a href="{ctx["href"](k)}">{html.escape(ctx["title"](k))} →</a>' for k in g['pages'] if ctx['known'](k))+'</div></section>' for g in area['groups'])+'</div>'
def body(page):
 kind=page.get('kind')
 if kind=='example':return '<h1>'+html.escape(page['title'])+'</h1>'+example_module(page,page['example'])+'<h2 id="reproduce">'+('Reproduce' if page['lang']=='en' else '复现')+'</h2>'+code('laymesh validate '+page['example']+'\nlaymesh render '+page['example']+' -o figure.svg'+(' --page 1' if page['example'].endswith('.laydoc') else '')+'\nlaymesh render '+page['example']+' -o figure.pdf\nlaymesh render '+page['example']+' -o figure.png --dpi 300'+(' --page 1' if page['example'].endswith('.laydoc') else ''),'sh')
 if kind=='source':return '<h1>'+html.escape(page['file'])+'</h1>'+code((ROOT/page['file']).read_text(),Path(page['file']).suffix[1:],page['file'])
 return markdown(page,(ROOT/page['source']).read_text())
def check():
 subprocess.run([sys.executable,str(ROOT/'scripts/build-docs-media.py'),'--check'],cwd=ROOT,check=True)
 count=0
 for p in OUT.rglob('*.html'):
  text=p.read_text()
  if 'class="example-module"' in text:
   head=text.split('</head>',1)[0]
   for name,version in STYLESHEET_VERSIONS.items():
    assert f'href="{relative({"output":p.relative_to(OUT).as_posix()},"site/"+name)}?v={version}"' in head,f'Missing or stale preview stylesheet: {p} -> {name}'
  for workspace in re.findall(r'<section class="example-module".*?(?=<div class="workspace-splitter")',text,re.S):
   assert 'data-mode="source"' in workspace and 'data-source-mode=' not in workspace and 'class="summary-panel"' not in workspace, f'Tutorial must show full source: {p}'
   entry=re.search(r'data-entry="([^"]+)"',workspace)[1]
   panels=list(re.finditer(r'<div class="source-panel"([^>]*)>.*?<textarea class="code-raw" hidden>(.*?)</textarea>',workspace,re.S))
   assert panels and ' hidden' not in panels[0][1], f'Initial source is hidden: {p}'
   for panel in panels:
    label=re.search(r'aria-labelledby="([^"]+)"',panel[1])[1]
    tab=re.search(r'<button\b[^>]*id="'+re.escape(label)+r'"[^>]*title="([^"]+)"',workspace)
    source=html.unescape(tab[1])
    assert html.unescape(panel[2])==(ROOT/source).read_text(), f'Truncated or stale source: {p} -> {source}'
   assert html.unescape(panels[0][2])==(ROOT/html.unescape(entry)).read_text(), f'Wrong initial file: {p}'
  if p.relative_to(OUT).as_posix() in ('skia-benchmark.html','tests/skia/benchmark.html'):
   assert text==(ROOT/'tests/skia/benchmark.html').read_text(),f'Stale standalone benchmark: {p}'
  elif p.relative_to(OUT).as_posix() in ('presentation-test.html','tests/skia/presentation.html'):
   assert text==(ROOT/'tests/skia/presentation.html').read_text(),f'Stale presentation test: {p}'
  elif p.relative_to(OUT).as_posix()=='docs/benchmarks/gradients-2026-10-19/gallery.html':
   assert text==(ROOT/'docs/benchmarks/gradients-2026-10-19/gallery.html').read_text(), 'Stale gradient gallery'
   assert text.count('<article ')==119 and '原式不被接受' in text and '正常暗色配方' in text
  elif p.relative_to(OUT).as_posix()=='gradient-gallery.html':
   assert 'docs/benchmarks/gradients-2026-10-19/gallery.html' in text
  elif p.relative_to(OUT).as_posix().startswith('docs/benchmarks/') and (ROOT/p.relative_to(OUT)).suffix=='.html' and (ROOT/p.relative_to(OUT)).is_file():
   assert text==(ROOT/p.relative_to(OUT)).read_text(),f'Stale standalone validation report: {p}'
  else:assert 'class="lang-switch"' in text,p
  # Inline scripts may construct markup with template expressions; validate
  # actual HTML attributes, including script src, rather than JS string data.
  text=re.sub(r'(<script\b[^>]*>).*?</script>',r'\1</script>',text,flags=re.S|re.I)
  for value in re.findall(r'(?:src|href|data-full)="([^"]+)"',text):
   value=html.unescape(value).split('#')[0].split('?')[0]
   if not value or re.match(r'(?:[a-z]+:|//)',value,re.I):continue
   target=(p.parent/value).resolve()
   if not target.is_file():raise RuntimeError(f'Broken link {p.relative_to(OUT)} → {value}')
  for href in re.findall(r'href="([^"]+)"',text):
   href=html.unescape(href)
   if '#' not in href or re.match(r'(?:[a-z]+:|//)',href,re.I):continue
   link,fragment=href.split('#',1);target=(p.parent/link).resolve() if link else p
   if fragment and target.suffix=='.html':assert f'id="{unquote(fragment)}"' in target.read_text(),f'Broken fragment: {p.relative_to(OUT)} → {href}'
  count+=1
 for en in (False,True):
  interface=(OUT/('en/docs/topics/interface-reference.html' if en else 'docs/topics/interface-reference.zh-CN.html')).read_text()
  for entry in json.loads((ROOT/'crates/laymesh-core/api.json').read_text())['api']:
   assert f'id="{entry["name"].replace(".","-")}"' in interface,f'Missing language-service documentation anchor: {entry["name"]}'

 for e in EXAMPLES:
  sibling=e['source'].removesuffix('.lay')+'.py'
  if (ROOT/sibling).is_file():
   for prefix in ('','en/'):
    page=(OUT/(prefix+'examples/'+e['id']+'.html')).read_text();assert f'title="{sibling}"' in page and f'data-edit-file="{sibling}"' not in page,f'Missing read-only Python tab: {sibling}'

 inventory=json.loads((ROOT/'site/style-libraries.json').read_text())
 for lib in inventory['libraries']:
  topic={'bootstrap':'style-bootstrap','open-props':'style-open-props','animate':'style-animate-library','effects':'art-effects'}[lib['name']]
  for prefix,suffix in [('','.zh-CN'),('en/','')]:
   text=(OUT/(prefix+'docs/topics/'+topic+suffix+'.html')).read_text()
   assert text.count('class="style-library-catalog"')==1
   assert text.count(f'data-entry="examples/libraries/catalog-{lib["name"]}.lay"')==1, 'Catalog must have one live workspace'
   embedded=re.search(r'<script type="application/json" class="library-data">(.*?)</script>',text,re.S)
   assert embedded and json.loads(embedded[1])==lib, 'Stale catalog inventory'
 live=OUT/'site/live';manifest=json.loads((live/'manifest.json').read_text())
 page_fonts.validate(live,manifest)
 assert (live/'wasm'/json.loads((live/'wasm/manifest.json').read_text())['wasm']).is_file();print(f'Checked {count} documentation pages, startup fonts and full editing fonts')
 spec=importlib.util.spec_from_file_location('check_live',ROOT/'scripts/build-docs-live.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.validate_runtime(live)
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--skip-wasm',action='store_true');a=parser.parse_args()
 if a.check:check();return
 discover()
 with tempfile.TemporaryDirectory(prefix='laymesh-wasm-preserve-') as cache:
  cached=Path(cache)/'wasm'
  if a.skip_wasm and (OUT/'site/live/wasm').is_dir():shutil.copytree(OUT/'site/live/wasm',cached)
  if OUT.exists():shutil.rmtree(OUT)
  (OUT/'site').mkdir(parents=True)
  if cached.is_dir():shutil.copytree(cached,OUT/'site/live/wasm')
 excluded=shutil.ignore_patterns('node_modules','dist','__pycache__','*.ttf','*.otf','*.ttc','*.otc','*.woff','*.woff2','*.pyc','_vendor','.venv','target','*.egg-info','output')
 for folder in ['examples','python','scripts','tests','.github','crates','extensions/vscode/src','release/licenses']:
  if (ROOT/folder).is_dir():shutil.copytree(ROOT/folder,OUT/folder,dirs_exist_ok=True,ignore=excluded)
 for name in ['LICENSE','Cargo.toml','Cargo.lock','rust-toolchain.toml']:
  if (ROOT/name).is_file():shutil.copy2(ROOT/name,OUT/name)
 for name in ['styles.css','app.js','logo.svg']:shutil.copy2(ROOT/'site'/name,OUT/'site'/name)
 shutil.copytree(ROOT/'site/media',OUT/'site/media',dirs_exist_ok=True)
 # Keep the measured gallery and its downloadable evidence as real Pages
 # assets; Markdown rendering alone does not copy standalone HTML/JSON/PNG.
 gallery_dir=Path('docs/benchmarks/gradients-2026-10-19')
 for source in (ROOT/gallery_dir).iterdir():
  if source.suffix in ('.html','.json','.png'):
   target=OUT/gallery_dir/source.name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)
 (OUT/'gradient-gallery.html').write_text('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta http-equiv="refresh" content="0;url=docs/benchmarks/gradients-2026-10-19/gallery.html"><title>LayMesh 渐变对照画廊</title><a href="docs/benchmarks/gradients-2026-10-19/gallery.html">打开渐变对照画廊</a></html>')
 for folder in ['object-effects-2026-10-19','animation-resources-2026-10-19']:
  source_dir=ROOT/'docs/benchmarks'/folder
  for source in source_dir.rglob('*'):
   if source.is_file() and source.suffix in ('.html','.json','.png','.csv') and not source.name.endswith('.zip.html'):
    target=OUT/source.relative_to(ROOT);target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)
 for page in PAGES:
  page['body']=body(page);page['sections']=[{'depth':int(m[1]),'id':m[2],'title':html.unescape(re.sub(r'<[^>]+>','',re.sub(r'<a class="heading-anchor".*?</a>','',m[3])))} for m in re.finditer(r'<h([23]) id="([^"]+)">(.*?)</h\1>',page['body'],re.S)]
 for lang in ['zh-CN','en']:
  search=[]
  for page in PAGES:
   if page['lang']!=lang:continue
   prose=re.sub(r'<script\b[^>]*>[\s\S]*?</script>','',page['body'])
   headings=' '.join(section['title'] for section in page['sections'])
   identifiers=' '.join(dict.fromkeys(html.unescape(re.sub(r'<[^>]+>',' ',item)) for item in re.findall(r'<code>(.*?)</code>',prose,re.S) if len(item)<120))
   search.append(dict(title=page['title'],group=page.get('area',''),url=page['output'],terms=headings+' '+identifiers+' '+re.sub(r'<[^>]+>',' ',prose)[:5000]))
  (OUT/f'site/search.{lang}.js').write_text('window.LAYMESH_SEARCH='+json.dumps(search,ensure_ascii=False).replace('<','\\u003c')+';\n')
 for page in PAGES:
  target=OUT/page['output'];target.parent.mkdir(parents=True,exist_ok=True);target.write_text(ENV.get_template('page.html').render(**context(page),body=page['body'],sections=page['sections'],has_preview='class="example-module"' in page['body']))
 spec=importlib.util.spec_from_file_location('build_live',ROOT/'scripts/build-docs-live.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.build(OUT/'site/live',a.skip_wasm)
 (OUT/'.nojekyll').write_text('');print(f'Built {len(PAGES)} documentation pages')
 check()
if __name__=='__main__':main()
