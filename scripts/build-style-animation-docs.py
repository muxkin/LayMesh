#!/usr/bin/env python3
"""Generate property reference and effect catalog from the shipped engine metadata."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
api = json.loads((ROOT / 'crates/laymesh-core/api.json').read_text())
css = (ROOT / 'crates/laymesh-core/assets/styles/animate.css').read_text()
groups = {
    'Attention seekers': ('强调', 'Emphasis'), 'Back entrances': ('缩放回弹入场', 'Back entrances'),
    'Back exits': ('缩放回弹退场', 'Back exits'), 'Bouncing entrances': ('弹跳入场', 'Bouncing entrances'),
    'Bouncing exits': ('弹跳退场', 'Bouncing exits'), 'Fading entrances': ('淡入', 'Fading entrances'),
    'Fading exits': ('淡出', 'Fading exits'), 'Flippers': ('三维翻转', 'Flips'),
    'Lightspeed': ('快速滑动', 'Lightspeed'), 'Rotating entrances': ('旋转入场', 'Rotating entrances'),
    'Rotating exits': ('旋转退场', 'Rotating exits'), 'Specials': ('特殊效果', 'Specials'),
    'Zooming entrances': ('缩放入场', 'Zooming entrances'), 'Zooming exits': ('缩放退场', 'Zooming exits'),
    'Sliding entrances': ('滑入', 'Sliding entrances'), 'Sliding exits': ('滑出', 'Sliding exits'),
}
special = {
    'bounce': ('上下弹跳', 'Vertical bounce'), 'flash': ('反复闪烁', 'Repeated flashing'),
    'pulse': ('轻微缩放脉冲', 'Gentle scale pulse'), 'rubberBand': ('橡皮筋拉伸回弹', 'Elastic stretch and recoil'),
    'shakeX': ('水平摇动', 'Horizontal shake'), 'shakeY': ('竖直摇动', 'Vertical shake'),
    'headShake': ('水平位移与三维摇头', 'Horizontal movement with a 3D head shake'),
    'swing': ('绕上边缘摆动', 'Swing around the top edge'), 'tada': ('缩放与旋转强调', 'Scale and rotation emphasis'),
    'wobble': ('左右摇摆', 'Side-to-side wobble'), 'jello': ('斜切变形回弹', 'Elastic skew'),
    'heartBeat': ('两次心跳式缩放', 'Double heartbeat scaling'), 'flip': ('透视三维翻转', 'Perspective 3D flip'),
    'hinge': ('从铰链处摆动后掉落', 'Swing on a hinge, then drop'),
    'jackInTheBox': ('缩放旋转弹出', 'Scale and rotate into view'),
    'rollIn': ('边滚动边入场', 'Roll into view'), 'rollOut': ('边滚动边退场', 'Roll out of view'),
}
directions = {'Down': ('向下', 'downward'), 'Up': ('向上', 'upward'), 'Left': ('向左', 'leftward'),
              'Right': ('向右', 'rightward'), 'TopLeft': ('左上角', 'top left'), 'TopRight': ('右上角', 'top right'),
              'BottomLeft': ('左下角', 'bottom left'), 'BottomRight': ('右下角', 'bottom right'),
              'X': ('绕 x 轴', 'around the x axis'), 'Y': ('绕 y 轴', 'around the y axis')}
effects = []
group = None
for match in re.finditer(r'/\*\s*([^*]+?)\s*\*/|^(?:\.animate__animated)?\.animate__(\w+)\s*\{([^}]+)\}', css, re.M):
    if match[1]:
        if match[1].strip() in groups:
            group = match[1].strip()
        continue
    name, body = match[2], match[3]
    if not re.search(r'(?<!-)animation-name\s*:', body):
        continue
    if name in special:
        description = special[name]
    else:
        family = next((p for p in ['lightSpeed', 'bounce', 'rotate', 'slide', 'zoom', 'fade', 'back', 'flip'] if name.startswith(p)), '')
        family_zh = {'lightSpeed': '快速倾斜滑动', 'bounce': '弹跳', 'rotate': '旋转', 'slide': '滑动', 'zoom': '缩放', 'fade': '透明度渐变', 'back': '缩放回弹', 'flip': '三维翻转'}[family]
        phase = 'In' if 'In' in name else 'Out'
        direction = name.split(phase, 1)[1].replace('Big', '')
        zh, en = directions.get(direction, ('', ''))
        extra = ('，大幅位移', ', large displacement') if name.endswith('Big') else ('', '')
        description = (family_zh + ('入场' if phase == 'In' else '退场') + ('，' + zh if zh else '') + extra[0],
                       groups[group][1] + ('; ' + en if en else '') + extra[1])
    effects.append({'name': name, 'class': 'animate__' + name, 'group': list(groups[group]), 'description': list(description)})
assert len(effects) == len({e['name'] for e in effects}) == 97

def write(path, text):
    path = ROOT / path
    if args.check:
        assert path.read_text() == text, f'Stale manual: {path}'
    else:
        path.write_text(text)

write('site/animation-effects.json', json.dumps(effects, ensure_ascii=False, indent=2) + '\n')
inherited = {'font-family','math-font','math-text-fallback','font-size','font-weight','font-style','color','line-height','text-shadow'}
samples = {
    'font-family':'"DejaVu Sans", sans-serif', 'math-font':'ratex-katex', 'math-text-fallback':'true',
    'font-size':'4mm', 'font-weight':'700', 'font-style':'italic', 'color':'#243447', 'line-height':'1.4',
    'background':'linear-gradient(120deg, #60a5fa, #075985)', 'padding':'1mm 2mm 1mm 2mm',
    'border-radius':'2mm', 'border-color':'#94a3b8', 'border-width':'0.5pt', 'border-style':'solid',
    'border-dash':'2mm 1mm', 'border-dash-offset':'0.5mm', 'border-cap':'round', 'border-join':'round',
    'border-miter-limit':'4', 'border-opacity':'0.7', 'line-color':'#0d6efd', 'line-width':'0.7pt',
    'line-style':'solid', 'line-dash':'2mm 1mm', 'line-dash-offset':'0.5mm', 'line-cap':'round',
    'line-join':'round', 'line-miter-limit':'4', 'line-opacity':'0.8', 'fill':'#dbeafe', 'opacity':'0.8',
    'tick-font-size':'8pt', 'label-font-size':'10pt', 'tick-color':'#243447', 'grid-color':'#cbd5e1',
    'grid-line-width':'0.3pt', 'grid-line-dash':'1mm 1mm', 'start-head':'head(shape="open",size=(3mm,2mm))',
    'end-head':'head(shape="triangle",size=(3mm,2mm))', 'start-cap':'round', 'end-cap':'square',
    'text-fill':'#0d6efd', 'text-stroke-color':'#ffffff', 'text-stroke-width':'0.3pt',
    'effects':'[shadow(blur=1mm,offset=(1mm,2mm))]', 'path':'path(commands=[move_to(0,0),line_to(20mm,10mm)])',
    'warp':'text_warp(kind="wave",amplitude=1mm,wavelength=10mm)',
    'extrude':'text_extrude(depth=1mm,angle=45deg,color="#334155")', 'shaft-width':'2mm',
    'heads':'end', 'head-size':'(4mm,3mm)', 'start-head-size':'(4mm,3mm)', 'end-head-size':'(4mm,3mm)',
    'line':'0.6pt solid #243447', 'border':'0.5pt solid #94a3b8', 'background-color':'#eff6ff',
    'background-size':'cover, 20mm 15mm', 'background-position':'center, right 2mm bottom 3mm', 'background-repeat':'no-repeat, space round',
    'background-image':'linear-gradient(120deg,#60a5fa,#075985)', 'font':'italic 700 4mm "DejaVu Sans"',
    'box-shadow':'1mm 2mm 3mm 0mm #0004', 'text-shadow':'0.5mm 0.5mm 1mm #0006',
    'filter':'drop-shadow(1mm 1mm 1mm #0005)', 'box-shadow-blur':'2mm', 'box-shadow-spread':'-0.5mm',
    'box-shadow-offset':'1mm 2mm', 'box-shadow-color':'#0d6efd66', 'box-shadow-position':'inset',
}
static = [p for p in api['cssProperties'] if p['name'] in samples]
assert len(static) == len(samples), set(samples) - {p['name'] for p in static}
for en in (False, True):
    lang = 'en' if en else 'zh-CN'
    idx = int(en)
    rows = ['<!-- properties:start -->']
    for p in static:
        name = p['name']
        target = ('Arrow shapes and endpoints' if en else '箭头形状及端部') if name in {'path','shaft-width','heads','head-size','start-head-size','end-head-size'} else ('Open strokes' if en else '开放线条') if name.startswith('line-') or name in {'line','start-head','end-head','start-cap','end-cap'} else ('Plot axes and their labels/grid' if en else '图表坐标轴、标签及网格') if name.startswith(('tick-','label-','grid-')) else ('Text and formulas' if en else '文字与公式') if name.startswith(('font','math','text-')) or name in {'color','line-height','warp','extrude'} else ('Drawable object regions; target support depends on material' if en else '对象区域；具体适用范围随素材而定')
        accepted = p.get('type','value')
        if p.get('values'): accepted += ': ' + ', '.join('`'+v+'`' for v in p['values'])
        default = p.get('default', p.get('defaultFrom', '按素材默认值' if not en else 'Material default'))
        unit = p.get('unit','none')
        if not en:
            unit = {'none':'无',
                    'explicit mm/cm/in/inch/pt/px or CSS relative units; px uses layout_dpi':'项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算',
                    'CSS lengths; px uses layout_dpi':'CSS 长度；px 按 layout_dpi 换算'}.get(unit, unit)
        rows += ['### '+name, '', p['descriptionEn' if en else 'description'], '',
                 ('Accepted values: ' if en else '合法值：') + accepted + '.', '',
                 ('Units: ' if en else '单位：') + unit + '.', '',
                 ('Default: ' if en else '默认：') + str(default) + '. ' + ('Inherited.' if en else '继承。') if name in inherited else ('Default: ' if en else '默认：') + str(default) + '. ' + ('Not inherited.' if en else '不继承。'), '',
                 ('Applies to: ' if en else '适用对象：') + target + '.', '',
                 '```lcss', '.sample { '+name+': '+samples[name]+'; }', '```', '']
        if name.startswith('box-shadow-'):
            rows += [('[Existing layers, lists and shorthand reset](style-shadows.'+lang+'.md#partial-overrides)' if en else '[已有层、列表与简写重置规则](style-shadows.'+lang+'.md#partial-overrides)'), '']
        if name == 'path':
            rows += [('`path` is a LayMesh constructor extension, not CSS path geometry or a DSL variable reference.' if en else '`path` 是 LayMesh 构造器扩展，不是网页 CSS 几何属性，也不能读取 `.lay` 变量。'), '']
    rows += ['<!-- properties:end -->']
    path = ROOT / f'docs/topics/style-properties.{lang}.md'
    text = path.read_text()
    text = re.sub(r'<!-- properties:start -->[\s\S]*?<!-- properties:end -->', '\n'.join(rows), text)
    write(path.relative_to(ROOT), text)
    path = ROOT / f'docs/topics/animation-catalog.{lang}.md'
    rows = ['<!-- effects:start -->']
    for key, titles in groups.items():
        rows += ['## '+titles[idx], '', '| '+('Class | Effect' if en else '类名 | 效果')+' |', '| --- | --- |']
        for effect in effects:
            if effect['group'] == list(titles): rows.append('| `'+effect['class']+'` | '+effect['description'][idx]+' |')
        rows += ['']
    rows += ['<!-- effects:end -->']
    text = re.sub(r'<!-- effects:start -->[\s\S]*?<!-- effects:end -->', '\n'.join(rows), path.read_text())
    write(path.relative_to(ROOT), text)
print(f'{"Checked" if args.check else "Generated"} {len(static)} static properties and {len(effects)} effects in both languages')
