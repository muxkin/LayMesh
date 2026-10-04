"""Shared documentation inputs for Python-only build tools."""
import hashlib,json,re,posixpath
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def examples():
 catalog=json.loads((ROOT/'examples/gallery/catalog.json').read_text());english=json.loads((ROOT/'examples/gallery/catalog.en.json').read_text());out=[]
 for category in catalog['categories']:
  for item in category['items']:
   source=item.get('source',f'examples/gallery/{category["slug"]}/{item["slug"]}.lay')
   out.append({'id':source.removeprefix('examples/').removesuffix('.lay').replace('/','-'),'source':source,'category':'plots' if category['slug']=='plots' else 'layout','group':category['slug'],'title':[item['title'],english[category['slug']]['items'][item['slug']][0]],'description':[item['description'],english[category['slug']]['items'][item['slug']][1]]})
 seen={e['source'] for e in out}
 for p in sorted((ROOT/'examples').rglob('*.lay')):
  source=p.relative_to(ROOT).as_posix()
  if 'output' in p.relative_to(ROOT).parts:continue
  if source in seen or 'canvas(' not in p.read_text():continue
  title=p.stem.replace('-',' ').title();out.append({'id':source.removeprefix('examples/').removesuffix('.lay').replace('/','-'),'source':source,'category':'plots' if '/plot/' in source else 'notebook' if '/notebook/' in source else 'applications','group':'plot' if '/plot/' in source else 'applications','title':[title,title],'description':['完整可复现的源码与输出。','Complete reproducible source and output.']})
 return out

def dependencies(source,seen=None):
 if seen is None:seen=set()
 if source in seen:return sorted(seen)
 seen.add(source);p=ROOT/source
 if p.suffix.lower() not in ('.lay','.lcss','.py','.ipynb','.svg'):return sorted(seen)
 text=p.read_text();text=re.sub(r'url\(\s*([^\'"()\s]+)\s*\)',r'url("\1")',text)
 for value in re.findall(r'''["']([^"'\n]+\.(?:lay|lcss|png|jpg|jpeg|tif|tiff|svg|json|csv|ttf|otf|ttc|otc)(?:#[^"'\n]*)?)["']''',text):
  value=value.split('#')[0];target=posixpath.normpath(posixpath.join(posixpath.dirname(source),value))
  if target.startswith('../') or value.startswith('/'):continue
  if (ROOT/target).is_file():dependencies(target,seen)
 return sorted(seen)
def fingerprint(files):
 h=hashlib.sha256()
 for f in sorted(set(files)):
  h.update(str(f).encode());h.update((ROOT/f).read_bytes())
 return h.hexdigest()
def engine_files():
 # Renderer provenance covers build settings and crate sources; standalone test
 # fixtures do not change rendered media when their platform paths are corrected.
 return ['Cargo.lock','Cargo.toml','rust-toolchain.toml']+[p.relative_to(ROOT).as_posix() for p in sorted((ROOT/'crates').rglob('*')) if p.is_file() and p.suffix in ('.rs','.toml','.json') and 'laymesh-wasm' not in p.parts and 'tests' not in p.relative_to(ROOT/'crates').parts]
def summary(source):
 text=(ROOT/source).read_text();m=re.search(r'# BEGIN DEMO\r?\n([\s\S]*?)\r?\n# END DEMO',text)
 if m:return m[1],m.start(1),text[:m.start(1)].count('\n')+1
 return '\n'.join(text.splitlines()[:14]),0,1
