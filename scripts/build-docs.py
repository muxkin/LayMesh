#!/usr/bin/env python3
"""Build bilingual static docs and live Rust/WASM examples without Node.js."""
from __future__ import annotations
import argparse,hashlib,html,importlib.util,json,os,posixpath,re,shutil,subprocess,sys,tempfile
from pathlib import Path
from urllib.parse import unquote
from jinja2 import Environment,FileSystemLoader,select_autoescape
from markdown_it import MarkdownIt
from site_support import ROOT,examples,dependencies,summary
OUT=ROOT/'site/dist';ENV=Environment(loader=FileSystemLoader(ROOT/'site/templates'),autoescape=select_autoescape());NAV=json.loads((ROOT/'site/navigation.json').read_text());EXAMPLES=examples();EX={e['source']:e for e in EXAMPLES};PAGES=[];BY_KEY={};BY_SOURCE={};MEDIA=json.loads((ROOT/'site/media/manifest.json').read_text())
def relative(page,target):return posixpath.relpath(str(target),posixpath.dirname(page['output']) or '.')
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
   if (ROOT/target).is_file() and target not in BY_SOURCE and target not in EX and Path(target).suffix.lower() in ('.py','.rs','.toml','.json','.csv','.lay','.lcss','.txt','.yml','.yaml','.mjs','.ts','.md'):
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
 return dict(page=page,navigation=NAV,tr=lambda zh,en_text:en_text if en else zh,translated=lambda values:values[int(en)],href=lambda key:relative(page,get(key)['output']),known=lambda key:(key,page['lang']) in BY_KEY,title=lambda key:get(key)['title'],asset=lambda f:relative(page,'site/'+f),counterpart=relative(page,BY_KEY.get((page['key'],'zh-CN' if en else 'en'),BY_KEY[('home','zh-CN' if en else 'en')])['output']),search_base=json.dumps({'base':relative(page,'index.html')}))
def example_module(page,source):
 e=EX.get(source)
 if not e:return ''
 deps=[f for f in dependencies(source) if Path(f).suffix.lower() not in ('.ttf','.otf','.ttc','.otc','.woff','.woff2')];files=sorted([f for f in deps if Path(f).suffix in ('.lay','.lcss','.csv','.json')],key=lambda f:(f!=source,f));python=source.removesuffix('.lay')+'.py'
 if (ROOT/python).is_file() and python not in files:files.append(python)
 panels=[]
 for i,file in enumerate(files):panels.append(dict(file=file,editable=Path(file).suffix in ('.lay','.lcss','.csv','.json'),name=Path(file).name,id='example-'+e['id']+('-source' if i==0 else '-'+re.sub(r'[^\w-]','-',Path(file).name)),code=code((ROOT/file).read_text(),Path(file).suffix[1:],Path(file).name)))
 text,start,line=summary(source)
 return ENV.get_template('example.html').render(**context(page),example=e,panels=panels,summary_from=start,summary_code=code(text,'lay',Path(source).name,line-1),preview=picture(page,source,e['title'][page['lang']=='en']),dependencies=deps)
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
 def embed(m):
  key=f'LAYMESHBLOCK{len(placeholders)}';placeholders[key]=example_module(page,m[2]) if m[1]=='example' else gallery(page) if m[2]=='examples' else overview(page,m[2]);return '\n\n'+key+'\n\n'
 text=re.sub(r'<!-- (example|overview):([^ ]+) -->',embed,text)
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
 if kind=='example':return '<h1>'+html.escape(page['title'])+'</h1>'+example_module(page,page['example'])+'<h2 id="reproduce">'+('Reproduce' if page['lang']=='en' else '复现')+'</h2>'+code('laymesh validate '+page['example']+'\nlaymesh render '+page['example']+' -o figure.svg\nlaymesh render '+page['example']+' -o figure.pdf\nlaymesh render '+page['example']+' -o figure.png --dpi 300','sh')
 if kind=='source':return '<h1>'+html.escape(page['file'])+'</h1>'+code((ROOT/page['file']).read_text(),Path(page['file']).suffix[1:],page['file'])
 return markdown(page,(ROOT/page['source']).read_text())
def check():
 subprocess.run([sys.executable,str(ROOT/'scripts/build-docs-media.py'),'--check'],cwd=ROOT,check=True)
 count=0
 for p in OUT.rglob('*.html'):
  text=p.read_text();assert 'class="lang-switch"' in text,p
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
 live=OUT/'site/live';manifest=json.loads((live/'manifest.json').read_text())
 expected={'fonts/DejaVuSans.ttf','fonts/NotoSansCJK-Regular.ttc'}
 assert {f['file'] for f in manifest['fonts']}==expected
 for font in manifest['fonts']:
  source=ROOT/'site/page-fonts'/Path(font['file']).name;target=live/font['file']
  assert target.read_bytes()==source.read_bytes() and hashlib.sha256(target.read_bytes()).hexdigest()==font['sha256']
  assert target.stat().st_size<20_000_000
  assert font['cdn']=='https://cdn.jsdelivr.net/gh/muxkin/LayMesh@main/site/page-fonts/'+target.name
 for license_name in ['DejaVu-LICENSE','Noto-CJK-LICENSE']:
  assert (live/'fonts'/license_name).read_bytes()==(ROOT/'site/page-fonts'/license_name).read_bytes()
 for p in live.rglob('*'):
  if p.suffix.lower() in ('.ttf','.otf','.ttc','.otc','.woff','.woff2'):assert p.relative_to(live).as_posix() in expected,f'Unexpected Page font: {p}'
 assert (live/'wasm/laymesh_wasm_bg.wasm').is_file();print(f'Checked {count} documentation pages and two Page-only body fonts')
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
 for page in PAGES:
  page['body']=body(page);page['sections']=[{'depth':int(m[1]),'id':m[2],'title':html.unescape(re.sub(r'<[^>]+>','',re.sub(r'<a class="heading-anchor".*?</a>','',m[3])))} for m in re.finditer(r'<h([23]) id="([^"]+)">(.*?)</h\1>',page['body'],re.S)]
 for lang in ['zh-CN','en']:
  search=[dict(title=p['title'],group=p.get('area',''),url=p['output'],terms=re.sub(r'<[^>]+>',' ',p['body'])[:5000]) for p in PAGES if p['lang']==lang];(OUT/f'site/search.{lang}.js').write_text('window.LAYMESH_SEARCH='+json.dumps(search,ensure_ascii=False).replace('<','\\u003c')+';\n')
 for page in PAGES:
  target=OUT/page['output'];target.parent.mkdir(parents=True,exist_ok=True);target.write_text(ENV.get_template('page.html').render(**context(page),body=page['body'],sections=page['sections']))
 spec=importlib.util.spec_from_file_location('build_live',ROOT/'scripts/build-docs-live.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.build(OUT/'site/live',a.skip_wasm)
 (OUT/'.nojekyll').write_text('');print(f'Built {len(PAGES)} documentation pages')
 check()
if __name__=='__main__':main()
