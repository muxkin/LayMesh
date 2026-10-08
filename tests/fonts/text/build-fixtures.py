#!/usr/bin/env python3
"""Make renamed, test-only Noto subsets; never bundle these with LayMesh."""
import argparse
import hashlib
import json
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--font-root',type=Path,default=Path('/usr/share/fonts'))
args=p.parse_args()
sources=[]
for style in ['Regular','Bold','Italic','BoldItalic']:
    sources.append((f'truetype/noto/NotoSerif-{style}.ttf',0,'Latin',style,'https://github.com/notofonts/latin-greek-cyrillic'))
for style in ['Regular','Bold']:
    sources.append((f'opentype/noto/NotoSerifCJK-{style}.ttc',2,'CJK',style,'https://github.com/notofonts/noto-cjk'))
sources.extend([
    ('truetype/noto/NotoSansArabic-Regular.ttf',0,'Arabic','Regular','https://github.com/notofonts/arabic'),
    ('truetype/noto/NotoSansDevanagari-Regular.ttf',0,'Indic','Regular','https://github.com/notofonts/devanagari'),
])
inputs=sorted((ROOT/'tests/math/ratex-0.1.14').glob('*.txt'))+sorted((ROOT/'tests/math/ratex-0.1.14').glob('*.json'))
inputs+=[ROOT/'experiments/opentype-math/domain-cases.json',ROOT/'experiments/opentype-math/text-fallback-cases.json']
chars=set(''.join(f.read_text() for f in inputs))
codes={ord(c) for c in chars}|set(range(0x20,0x250))|set(range(0x300,0x370))|set(range(0x600,0x700))|set(range(0x900,0x980))
manifest=[]
for relative,index,script,style,url in sources:
    source=args.font_root/relative
    font=TTFont(source,fontNumber=index,recalcTimestamp=False)
    version=font['name'].getDebugName(5)
    options=subset.Options()
    options.layout_features=['*']
    options.name_IDs=[0,1,2,3,4,5,6,13,14,16,17]
    options.name_legacy=True
    options.name_languages=['*']
    reducer=subset.Subsetter(options=options);reducer.populate(unicodes=codes);reducer.subset(font)
    family=f'LayMesh Formula Text {script}'
    postscript=f'LayMeshFormulaText{script}-{style}'
    for entry in font['name'].names:
        value={1:family,4:f'{family} {style}',6:postscript,16:family}.get(entry.nameID)
        if value is not None:entry.string=value.encode(entry.getEncoding())
    if 'CFF ' in font:
        cff=font['CFF '].cff;cff.fontNames=[postscript]
        cff.topDictIndex[0].FamilyName=family;cff.topDictIndex[0].FullName=f'{family} {style}'
    output=HERE/f'{script}-{style}.{"otf" if "CFF " in font else "ttf"}'
    font.save(output)
    manifest.append({'file':output.name,'family':family,'style':style,'source':url,'source_file':relative,
        'source_index':index,'source_version':version,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
        'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'derivation':'renamed Unicode subset with shaping tables retained'})
    print(output.name,output.stat().st_size)
(HERE/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
