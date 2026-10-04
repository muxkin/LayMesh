#!/usr/bin/env python3
"""Refresh curated bilingual walkthroughs while retaining detailed topic notes."""
import argparse,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args()
data=json.loads((ROOT/'docs/topic-walkthroughs.json').read_text())
registry=json.loads((ROOT/'crates/laymesh-core/api.json').read_text())
coverage=json.loads((ROOT/'docs/feature-coverage.json').read_text())['features']
nav=json.loads((ROOT/'site/navigation.json').read_text())
BEGIN='<!-- walkthrough:start -->';END='<!-- walkthrough:end -->'
for topic,info in data.items():
 for en in (False,True):
  lang='en' if en else 'zh-CN';suffix='En' if en else 'Zh'
  file=ROOT/f'docs/topics/{topic}.{lang}.md';old=file.read_text()
  title,body=old.split('\n',1)
  if BEGIN in body:
   body=body.split(END,1)[1]
   while re.match(r'^\s*## (Detailed behavior and further examples|详细行为与补充示例)\s*',body):
    body=re.sub(r'^\s*## (Detailed behavior and further examples|详细行为与补充示例)\s*','',body,count=1)
  else:
   body=re.sub(r'^\s*[^\n]+\n\n','',body,count=1)
   body=re.sub(r'<!-- example:[^>]+ -->\s*','',body)
   body=re.sub(r'^Learn how to use .*?constraints\.\n\n','',body)
   body=re.sub(r'^学习.*?约束。\n\n','',body)
   body=re.sub(r'^(The image corresponds to the complete source;.*|图片对应完整源码；.*)\n?','',body,flags=re.M)
   # Preserve useful existing tables and detailed rules, with their subheadings.
   body=re.sub(r'^## ', '### ', body,flags=re.M)
  minimal=coverage['api:'+info['minimalApi']]['minimal']
  composition=next((t.get('example') for t in nav['topics'] if t['key']==topic),None)
  if not composition:composition=coverage['api:'+info['minimalApi']]['composition']
  lines=[title,'',BEGIN,'## '+('Purpose and concepts' if en else '用途与概念'),'',info['concept'+suffix],'',
   '## '+('Minimal complete example' if en else '最小完整示例'),'',
   ('Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.' if en else '此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。'),'',
   '<!-- example:'+minimal+' -->','',
   '## '+('Parameters and default behavior' if en else '参数与默认行为'),'',
   ('Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.' if en else '裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。'),'',
   '## '+('Composition' if en else '组合用法'),'',info['composition'+suffix],'','<!-- example:'+composition+' -->','',
   '## '+('Common errors and limits' if en else '常见错误与限制'),'',info['errors'+suffix],'',
   '## '+('Individual functions' if en else '逐项功能说明'),'']
  for entry in registry['api']:
   key='api:'+entry['name'];feature=coverage[key]
   if feature['topic']!=topic:continue
   name=entry['name'];anchor=feature['anchor']
   lines+=['### '+anchor,'',entry['descriptionEn' if en else 'description'],'',
    ('Returns: ' if en else '返回：')+entry.get('returns',{}).get('en' if en else 'zh',''),'']
   if entry.get('required'):lines += [('Required inputs: ' if en else '必需输入：')+', '.join('`'+n+'`' for n in entry['required'])+'.','']
   lines += [('[Minimal complete source](../../'+feature['minimal']+') · [Composition source](../../'+feature['composition']+') · [All parameters](interface-reference.'+lang+'.md#'+anchor+')' if en else '[最小完整源码](../../'+feature['minimal']+') · [组合源码](../../'+feature['composition']+') · [全部参数](interface-reference.'+lang+'.md#'+anchor+')'),'']
  lines += [END,'']
  if body.strip():lines+=['## '+('Detailed behavior and further examples' if en else '详细行为与补充示例'),'',body.strip(),'']
  text='\n'.join(lines).rstrip()+'\n'
  if a.check:assert file.read_text()==text,f'Stale walkthrough: {file}'
  else:file.write_text(text)
print(f'{"Checked" if a.check else "Updated"} {len(data)*2} curated topic walkthroughs')
