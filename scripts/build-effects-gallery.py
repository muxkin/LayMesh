#!/usr/bin/env python3
"""Render the editable static-effects gallery without remote assets or a server."""
import argparse, base64, hashlib, html, json, shutil, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,default=ROOT/'target/debug/laymesh')
p.add_argument('--output',type=Path,default=ROOT/'examples/output/effects3d/gallery')
p.add_argument('--dpi',type=float,default=120)
a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
entries=json.loads((ROOT/'examples/effects/presets.json').read_text())
assert len(entries)>=30
families=['Shadow & glow','Reflection','Soft edges','Bevel','Materials','Views']
files={};cards=[]
def download(source,label):
    # Embedded source works under file:// as well as HTTP; browsers can block
    # download attributes on links to other local files.
    encoded=base64.b64encode((ROOT/source).read_bytes()).decode('ascii')
    return f'<a href="data:text/plain;base64,{encoded}" download="{html.escape(Path(source).name)}">{html.escape(label)}</a>'
def render(source,name,dpi):
    image=out/(name+'.png')
    subprocess.run([str(a.binary.resolve()),'render',str(ROOT/source),'-o',str(image),'--dpi',str(dpi),'--warnings','hide'],check=True,capture_output=True)
    data=image.read_bytes();files[image.name]={'sha256':hashlib.sha256(data).hexdigest(),'source':source,'dpi':dpi}
    dest=out/'source'/source;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/source,dest)
    return image.name
for i,e in enumerate(entries):
    image=render(e['source'],e['name'],a.dpi);family=families[i//5]
    source=(ROOT/e['source']).read_text();class_name=html.escape(e['className'])
    cards.append(f'<article data-family="{family}"><img loading="lazy" width="340" height="283" src="{image}" alt="{html.escape(e["title"])}"><div class="caption"><h2>{html.escape(e["title"])}</h2><code>{class_name}</code><p>{download(e["source"],"Download .lay")}</p><details><summary>Editable source</summary><pre>{html.escape(source)}</pre></details></div></article>')
showcase=render('examples/effects/object-3d.lay','object-3d',a.dpi)
render('examples/effects/presets.lay','all-presets',a.dpi)
for source in ['tests/fonts/DejaVuSans.ttf','tests/fonts/text/CJK-Regular.otf','examples/gallery/assets/sample.png','crates/laymesh-core/assets/styles/effects.css']:
    dest=out/'source'/source;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/source,dest)
# Keep original license notices alongside the copied example assets.
for source in ['LICENSE','tests/fonts/text/Noto-CJK-copyright.txt','tests/fonts/DejaVuSans.LICENSE']:
    if (ROOT/source).is_file():
        dest=out/'source'/source;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/source,dest)
buttons='<button aria-pressed="true" data-filter="all">All 30</button>'+''.join(f'<button aria-pressed="false" data-filter="{f}">{f}</button>' for f in families)
page='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LayMesh — Surface, depth & light</title><style>
*{box-sizing:border-box}body{margin:0;background:#f5f7fb;color:#18324d;font:16px/1.5 system-ui,sans-serif}main{max-width:1240px;margin:auto;padding:40px 24px 70px}header{max-width:780px}h1{font-size:clamp(32px,5vw,56px);letter-spacing:-.04em;line-height:1.1;margin:.3em 0}header p{color:#61738c}.eyebrow{text-transform:uppercase;letter-spacing:.12em;font-size:12px}.showcase{margin:28px 0;max-width:850px}.showcase img{width:100%;height:auto;border-radius:14px}.showcase p{font-size:14px}a{color:#156f8c}nav{display:flex;flex-wrap:wrap;gap:8px;margin:30px 0 20px}button{border:1px solid #cbd5e0;border-radius:24px;background:transparent;color:inherit;padding:9px 15px;cursor:pointer;font:inherit}button[aria-pressed=true]{background:#18324d;color:white;border-color:#18324d}button:focus-visible,a:focus-visible,summary:focus-visible{outline:3px solid #24a8b8;outline-offset:3px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,280px),1fr));gap:18px}article{border:1px solid #dbe2eb;border-radius:14px;overflow:hidden;background:white}article[hidden]{display:none}article img{display:block;width:100%;height:auto;background:#f5f7fb}.caption{padding:18px}h2{font-size:19px;margin:0 0 5px}code{font-size:13px;color:#61738c}details{font-size:13px}summary{cursor:pointer}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f5f7fb;padding:12px;border-radius:8px}footer{margin-top:32px;font-size:14px;color:#61738c}
</style><main><header><p class="eyebrow">LayMesh / Static effects</p><h1>Surface, depth & light.</h1><p>Thirty editable presets, from subtle shadows to beveled solids. Choose a family, inspect the result, then edit its source.</p></header>'''
page+=f'<section class="showcase"><img src="{showcase}" alt="Beveled shape, extruded lettering, Chinese, formula and rounded image"><p>{download("examples/effects/object-3d.lay","Composition source")} · <a href="all-presets.png">Comparison sheet</a> · {download("crates/laymesh-core/assets/styles/effects.css","Preset stylesheet")}</p></section>'
page+='<nav aria-label="Effect family">'+buttons+'</nav><div class="grid">'+''.join(cards)+'</div><footer>Import @laymesh/effects.css. Override parameters in .lay or LCSS to customize each preset.</footer></main><script>document.querySelectorAll("[data-filter]").forEach(button=>button.addEventListener("click",()=>{document.querySelectorAll("[data-filter]").forEach(b=>b.setAttribute("aria-pressed",String(b===button)));document.querySelectorAll("article").forEach(card=>card.hidden=button.dataset.filter!=="all"&&card.dataset.family!==button.dataset.filter)}));</script></html>'
(out/'index.html').write_text(page)
(out/'manifest.json').write_text(json.dumps({'presets':len(entries),'images':files},indent=2)+'\n')
print(json.dumps({'gallery':str(out/'index.html'),'images':len(files)},ensure_ascii=False))
