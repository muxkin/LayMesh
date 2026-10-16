#!/usr/bin/env python3
"""Compare the two accepted painted-preview runs and audit deck reuse."""
import argparse,json,statistics,math
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--baseline',type=Path,required=True)
p.add_argument('--input',type=Path,required=True)
a=p.parse_args();old=json.loads((a.baseline/'summary.json').read_text());new=json.loads((a.input/'summary.json').read_text())
assert old['totalSamples']==new['totalSamples']==2800
changes=[]
for row in new['timings']:
 before=next(r for r in old['timings'] if all(r[k]==row[k] for k in ['host','page','kind']))
 changes.append({**{k:row[k] for k in ['host','page','kind']},'endpoint':'current','oldMedianMs':before['medianMs'],'newMedianMs':row['medianMs'],'oldP95Ms':before['p95Ms'],'newP95Ms':row['p95Ms'],'medianReductionPercent':100*(1-row['medianMs']/before['medianMs'])})
for row in new['documents']:
 before=next(r for r in old['documents'] if all(r[k]==row[k] for k in ['host','kind']))
 for key in ['currentMs','documentMs']:
  changes.append({**{k:row[k] for k in ['host','kind']},'endpoint':key,'oldMedianMs':before[key]['median'],'newMedianMs':row[key]['median'],'oldP95Ms':before[key]['p95'],'newP95Ms':row[key]['p95'],'medianReductionPercent':100*(1-row[key]['median']/before[key]['median'])})
def dist(values):
 v=sorted(values);return {'median':statistics.median(v),'p95':v[math.ceil(len(v)*.95)-1]}
decks=[]
for kind in ['lcss-heading','lcss-curves','laydoc-name','laydoc-reorder','laydoc-add-remove']:
 result=json.loads((a.input/'vscode-document'/f'{kind}.json').read_text());rows=result['rows'];assert all(r['documentReady'] for r in rows)
 if kind.startswith('laydoc-'):assert all(r['preparation']['compiledPages']==0 for r in rows),'Directory-only changes must reuse prepared source pages'
 groups=[(kind,rows)] if kind!='laydoc-add-remove' else [('laydoc-'+op,[r for r in rows if r['operation']==op]) for op in ['add','remove']]
 for label,selected in groups:
  assert len(selected)==50
  decks.append({'kind':label,'samples':len(selected),'preparationMs':dist([r['presentationPreparationMs'] for r in selected]),'compiledPages':sorted({r['preparation']['compiledPages'] for r in selected}),'reusedPages':sorted({r['preparation']['reusedPages'] for r in selected})})
result={'baselineHtmlSha256':old['htmlSha256'],'htmlSha256':new['htmlSha256'],'nativeSha256':new['nativeSha256'],'changes':changes,'nativeDeckReuse':decks,'comparison':'Same machine, scenarios, 50 accepted samples, warmup and painted endpoint; original measured run retained, not simultaneous A/B'}
(a.input/'comparison.json').write_text(json.dumps(result,indent=2)+'\n')
labels={'chromium':'Chromium','firefox':'Firefox','webkit':'WebKit','VS Code':'VS Code'}
lines=['| 环境 | 场景与终点 | v002 中位数 / P95 | v003 中位数 / P95 | 中位数降低 |','|---|---|---:|---:|---:|']
for r in changes:
 if r['kind']!='lcss-curves' and not(r['host']=='VS Code' and r['kind'] in ['laydoc-name','laydoc-reorder'] and r['endpoint']=='documentMs'):continue
 endpoint={'currentMs':'当前页','documentMs':'整份同步'}[r['endpoint']]
 kind={'lcss-curves':'LCSS 曲线改色','laydoc-name':'laydoc 改名','laydoc-reorder':'laydoc 重排'}[r['kind']]
 lines.append(f"| {labels[r['host']]} | {kind}，{endpoint} | {r['oldMedianMs']:.1f} / {r['oldP95Ms']:.1f} | {r['newMedianMs']:.1f} / {r['newP95Ms']:.1f} | {r['medianReductionPercent']:.1f}% |")
(a.input/'comparison.md').write_text('\n'.join(lines)+'\n')
print(json.dumps({'comparisons':len(changes),'nativeDeckReuse':decks},indent=2))
