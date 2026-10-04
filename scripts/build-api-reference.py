#!/usr/bin/env python3
"""Generate bilingual parameter/reference/example coverage from shared API metadata."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args()
registry=json.loads((ROOT/'crates/laymesh-core/api.json').read_text());api=registry['api'];coverage=json.loads((ROOT/'docs/feature-coverage.json').read_text())['features']
def write(file,text):
 if a.check:assert file.read_text()==text,f'Stale API reference: {file}'
 else:file.write_text(text)
def typ(p):
 t=p.get('type','value')
 if p.get('values'):
  t=' | '.join('"'+v+'"' for v in p['values'])
  if 'boolean' in p.get('type',''):t+=' | boolean'
  if p['name']=='anchor' and 'top_left' in p['values']:t+=' | self selector'
  if p['name']=='position':t+=' | (length,length)'
 return t
def table_row(values):return '| '+' | '.join(str(v).replace('|','\\|').replace('\n',' ') for v in values)+' |'
for en in (False,True):
 lang='en' if en else 'zh-CN';key='descriptionEn' if en else 'description';lines=['# '+('API reference' if en else 'API 参考'),'','<!-- Generated from crates/laymesh-core/api.json and docs/feature-coverage.json. -->','']
 variable_lines=['## '+('Predefined string variables' if en else '预定义字符串变量'),'','Names below are ordinary string variables, available without declarations. User bindings take priority; the equivalent quoted value remains legal. Contextual meaning belongs to parameter documentation, not variable hover.' if en else '下列名称是无需声明的普通字符串变量。用户绑定优先，对应字符串写法继续合法。场景含义放在参数说明中，而不是变量悬停中。','','| '+('Variable | Type | String value' if en else '变量 | 类型 | 字符串值')+' |','| --- | --- | --- |']
 for v in registry['predefinedVariables']:variable_lines.append(table_row(['`'+v['name']+'`','`'+v['type']+'`','`'+json.dumps(v['value'],ensure_ascii=False)+'`']))
 lines+=variable_lines+['']
 detailed=[]
 for entry in api:
  name=entry['name'];f=coverage['api:'+name]
  block=['## '+name,'',entry[key],'',('Returns: ' if en else '返回：')+entry['returns']['en' if en else 'zh'],'']
  for field,caption in [('positional','Positional parameters: ' if en else '位置参数：'),('required','Required: ' if en else '必需：')]:
   if entry.get(field):block += [caption+', '.join('`'+n+'`' for n in entry[field])+'.','']
  block += [('| Parameter | Allowed type / unit | Meaning and choices | Default / inheritance |' if en else '| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |'),'| --- | --- | --- | --- |']
  for param in entry['parameters']:
   t=typ(param);unit=param.get('unit','none')
   if unit!='none':t+=' / '+(('canvas unit' if en else '画布单位') if unit=='geometry' else unit)
   meaning=param[key]
   for value in param.get('values',[]):meaning+='<br>`'+value+'`: '+param['valueDescriptions'][value]['en' if en else 'zh']
   default=param.get('default','—')
   if param.get('defaultFrom'):default=('inherits ' if en else '继承 ')+param['defaultFrom']
   block.append(table_row(['`'+param['name']+'`',t,meaning,default]))
  block += ['','### '+('Minimal complete example' if en else '最小完整示例'),'','```lay',(ROOT/f['minimal']).read_text().strip(),'```','',
   ('[Concepts and common errors]('+f['topic']+'.'+lang+'.md#'+f['anchor']+') · [Composition source](../../'+f['composition']+')' if en else '[概念与常见错误]('+f['topic']+'.'+lang+'.md#'+f['anchor']+') · [组合源码](../../'+f['composition']+')'),'']
  detailed+=block
 # Root reference links have one less directory level.
 root_blocks='\n'.join(detailed).replace('](../../examples/','](../examples/')
 for feature in coverage.values():root_blocks=root_blocks.replace(']('+feature['topic']+'.'+lang+'.md','](topics/'+feature['topic']+'.'+lang+'.md')
 write(ROOT/f'docs/api-reference.{lang}.md','\n'.join(lines)+'\n'+root_blocks.rstrip()+'\n')
 file=ROOT/f'docs/topics/interface-reference.{lang}.md'
 intro='# '+('Public parameters and examples' if en else '公开参数与示例')+'\n\n'+('The shared registry supplies runtime options, parameter completion, hover, signature help and this reference. Every function below has a runnable minimal source and a composition source. See the [feature map](feature-map.en.md) for complete coverage.' if en else '共享登记表统一提供运行时选项、参数补全、悬停、签名帮助与本参考。下列每项功能都有可运行的最小源码和组合源码，完整范围见[功能清单](feature-map.zh-CN.md)。')+'\n'
 lines=[intro,'']+variable_lines+['']+detailed
 lines+=['## '+('Geometry view types' if en else '几何视图类型'),'','[Anchors and path queries](anchors.en.md)' if en else '[锚点与路径查询](anchors.zh-CN.md)','']
 for name,t in registry['geometryTypes'].items():
  lines+=['### '+name,'',t.get(key,''),'']
  notes={
   'axes':('Index with an axis name, such as `chart.axes["x"]`; numeric indices are not axis names. The axis must have been declared before placement.','按轴名索引，例如 `chart.axes["x"]`；数字索引不是轴名。轴必须在图表放置前声明。'),
   'paths':('Index original subpaths in source order. Select one before continuous traversal, including `start`, `end`, `nodes`, `segments`, `at` and `between`.','按源码顺序索引原始子路径。连续遍历前先选一条，包括 `start`、`end`、`nodes`、`segments`、`at` 和 `between`。'),
   'segments':('Index original source segments, independently of render subdivision. On a selected segment, `at(t=...)` uses its raw parameter and `controls` exposes its control points.','索引原始源码段，与渲染细分数量无关。选中段后，`at(t=...)` 使用原始参数，`controls` 提供其控制点。'),
   'components':('Index tick label components in the axis component’s tick order, for example `chart.axes["x"].ticks[0].bounds`. Each component provides its own nine layout anchors.','按轴组件的刻度顺序索引刻度标签，例如 `chart.axes["x"].ticks[0].bounds`。每个部件具有自身布局框的九点锚点。'),
   'path_nodes':('Index the original connecting nodes; Bézier control points belong to `controls`. A corner node needs `with_side(...)` before a direction query.','索引原始连接节点；贝塞尔控制点属于 `controls`。尖角节点查询方向前须使用 `with_side(...)`。'),
   'control_nodes':('Index the selected segment’s original control points. These are positional anchors and need not lie on the curve; they do not provide a path tangent.','索引所选曲线段的原始控制点。这些位置锚点通常不在曲线上，不提供路径切线。'),
   'anchor':('`x` and `y` are physical positions in the current container. Read them for numeric coordinates or offsets; `target` takes the anchor object itself.','`x` 与 `y` 是当前容器中的物理位置。读取它们用于数值坐标或偏移；`target` 接受锚点对象本身。'),
   'vector':('Two dimensionless components: `[0]` is the horizontal component and `[1]` the vertical component. Tangent and normal directions are normalized after the instance transform. Multiply by a length to obtain a physical offset.','两个无量纲分量：`[0]` 为水平分量、`[1]` 为竖直分量。切线与法线方向在实例变换后归一化，乘以长度可得到物理偏移。'),
   'geometry_path':('The nine short names refer to `bounds`. Use geometric queries to get a point on the curve. `controls` and `at(t=...)` require a selected source segment.','九点简写对应 `bounds`。要取得曲线上的点，应使用几何查询。`controls` 与 `at(t=...)` 要求先选择一个源码段。')}
  if name in notes:lines += [notes[name][0 if en else 1],'']
  members=list(t.get('members',{}).items())+[(n+'(...)',v) for n,v in t.get('methods',{}).items()]
  if t.get('index'):members.append(('[i]',t['index']))
  if members:lines+=['| '+('Selection | Type' if en else '选取 | 类型')+' |','| --- | --- |']+[table_row(['`'+n+'`','`'+v+'`']) for n,v in members]+['']
 lines += ['## '+('Value types' if en else '值类型'),'']
 for name,t in registry.get('valueTypes',{}).items():
  lines += ['### '+name,'',t.get(key,''),'']
  members=list(t.get('members',{}).items())+[(n+'(...)',v) for n,v in t.get('methods',{}).items()]
  if t.get('index'):members.append(('[key]',t['index']))
  lines += ['| '+('Selection | Type' if en else '选取 | 类型')+' |','| --- | --- |']+[table_row(['`'+n+'`','`'+v+'`']) for n,v in members]+['']
 write(file,'\n'.join(lines).rstrip()+'\n')
 # A browsable manifest, including integrations rather than silently omitting them.
 lines=['# '+('Feature coverage map' if en else '功能覆盖清单'),'','<!-- Generated from docs/feature-coverage.json. -->','',
  ('This map covers public functions, geometry members, LCSS properties and integration workflows. Each row links an independent explanation, parameter reference where applicable, and two source examples. `python scripts/check-feature-docs.py` reports missing entries; `--render` validates every minimal drawing example and exports SVG/PDF/PNG.' if en else '本清单覆盖公开函数、几何成员、LCSS 属性和集成流程。每行关联独立说明、适用的参数参考及两份源码。`python scripts/check-feature-docs.py` 报告缺项，`--render` 验证每个最小绘图示例并导出 SVG/PDF/PNG。'),'','| '+('Feature | Explanation | Minimal source | Composition source' if en else '功能 | 说明 | 最小源码 | 组合源码')+' |','| --- | --- | --- | --- |']
 for name,f in coverage.items():lines.append(table_row(['`'+name+'`','[→]('+f['topic']+'.'+lang+'.md#'+f['anchor']+')','[→](../../'+f['minimal']+')','[→](../../'+f['composition']+')']))
 write(ROOT/f'docs/topics/feature-map.{lang}.md','\n'.join(lines).rstrip()+'\n')
print('Generated API references, variable registry and feature maps' if not a.check else 'Checked API references, variable registry and feature maps')
