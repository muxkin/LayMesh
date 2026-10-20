#!/usr/bin/env python3
"""Report the follow-up against preserved v24 evidence without hiding misses."""
import argparse,json,html,csv,statistics,re,shutil
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('records',type=Path);p.add_argument('previous',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
def read(p):return json.loads(p.read_text())
def fmt(v,d=1):return '未测' if v is None else f'{v:,.{d}f}'
old={r['file']:r for r in read(a.previous/'summary.json')['runs']};new=read(a.records/'summary.json')['runs'];lines=[]
def emit(s=''):lines.append(s)
def table(headers,rows):
 emit('| '+' | '.join(headers)+' |');emit('| '+' | '.join('---' for _ in headers)+' |')
 for row in rows:emit('| '+' | '.join(str(x) for x in row)+' |')
 emit()
emit('# Open Props 动画优化后续验证（WebKit 暂缓）');emit()
emit('本轮只处理动画与资源使用。保留原作品、原附件和 v24 记录，不发布版本。WebKit 按用户要求不纳入本轮调查或通过率。')
preserved=read(a.records/'preservation.json');emit(f"新文件 SHA-256：`{preserved['outputSha256']}`。原附件 {preserved['preservedEntries']} 项源码、字体、资源及显示列表逐项相同；没有降低分辨率、删减效果、缩短动画，或新增页数／内存／显存配额。");emit()
emit('## 实现变化');emit()
emit('- CPU 保留 Skia 绘制的原密度内容，交给 Canvas2D 合成位置、旋转与透明度；删除已转移的 Skia 副本，避免每帧整页像素转换。透明转场采用相同的预乘透明度规则。')
emit('- CPU 阴影从实际轮廓直接模糊；仅对严格识别的矩形和圆角矩形用等价轮廓扩张代替整幅形态学处理，任意形状继续使用原路径。背景图案位于单个重复单元内时避免重复中间光栅化。')
emit('- 后台准备在效果、子对象、分块边界可恢复和取消；GPU 完成等待按测量成本合并，减少逐块串行等待。前后台仍为独立 Worker 与渲染器。软件 GPU 使用经对照的 CPU／Canvas2D 路径，硬件 GPU 保留 WebGL。')
emit('- 硬件首次进入先显示作品真实入口状态，再准备后续动画缓存；作品时钟与动画时长保留。');emit()
emit('## 自然播放和实际资源');emit()
emit('每个环境依次自然播放 26 页两轮，首遍立即开始，遵守原动画和停留时间；没有全篇预热或跳到终态。v24 是同一机器的历史批次，时间戳与后端见原始记录；不将跨批次样本当成同时实验。PSS 包含浏览器进程树；DRM 是驱动客户端驻留数据，含共享缓冲，集成显卡内存域不等同于独立显存。');emit()
historical=[]
for name,r in old.items():
 if name.startswith(('original-','baseline-')) and r['engine']!='webkit':historical.append([name.split('/')[0],r['traversalStatus'],str(r['visited'])+'/52',fmt(r.get('peakPssMiB'))])
table(['原附件／优化前运行时历史对照','结果','完成页次','观测 PSS 峰值 / MiB'],historical)
emit('未完成相同工作量的历史失败样本仅列观测值，不用于资源降幅计算。以下使用完整两轮的 v24 与新版比较，保留后续修复带来的增减。');emit()
rows=[]
for r in new:
 before=old.get(r['file'],{});label=r['file'].split('/')[0].replace('optimized-','')
 backend=r['backend']
 if 'Canvas2D' not in backend and any(x['stats'].get('nativeCompositions',0)>0 for x in r['lastObservedResourcesByRenderer'].values()):backend+=' + Canvas2D'
 rows.append([label,f"{r['visited']}/52 · {r['traversalStatus']}",backend,fmt(before.get('firstVisibleWarmP95Ms'))+' → '+fmt(r['firstVisibleWarmP95Ms']),fmt(before.get('peakPssMiB'))+' → '+fmt(r['peakPssMiB']),fmt(before.get('peakDrmVramMiB'))+' → '+fmt(r['peakDrmVramMiB'])])
table(['环境','自然遍历','实际后端','已准备切页 P95 / ms，v24 → 新版','PSS 峰值 / MiB，v24 → 新版','DRM VRAM / MiB，v24 → 新版'],rows)
increased=[r['file'].split('/')[0].replace('optimized-','') for r in new if r.get('peakPssMiB') is not None and old.get(r['file'],{}).get('peakPssMiB') is not None and r['peakPssMiB']>old[r['file']]['peakPssMiB']]
if increased:emit('实际 PSS 峰值没有全面下降；以下环境高于 v24：'+ '、'.join(increased)+'。这些结果不计为内存优化达标，不能用逻辑缓存下降覆盖它们。');emit()
residency=[]
for r in new:
 before=old.get(r['file'],{});previous={x['round']:x for x in before.get('roundEnds',[])};current={x['round']:x for x in r['roundEnds']}
 residency.append([r['file'].split('/')[0].replace('optimized-','')]+[fmt(previous.get(i,{}).get('pssMiB'))+' → '+fmt(current.get(i,{}).get('pssMiB')) for i in [0,1]])
table(['环境','第一轮末 PSS / MiB，v24 → 新版','第二轮末 PSS / MiB，v24 → 新版'],residency)
emit('轮末是完成原动画后的同一操作检查点，没有主动 GC 或清空内容缓存；它与采样峰值、退出后的驻留范围分别报告。');emit()
emit('以下单列逻辑缓存与 WASM 数据，不能与上表的进程或驱动占用相加。原生位图已替代原 Skia 缓存像素，是存活内容字节的子集；CanvasKit 只公开堆容量，不能将其标成已用内存。');emit()
rows=[]
for r in new:
 resources=[x['stats'] for x in r['lastObservedResourcesByRenderer'].values()]
 rows.append([r['file'].split('/')[0].replace('optimized-',''),fmt(sum(x.get('liveTextureBytes',0) for x in resources)/2**20),fmt(sum(x.get('nativeBitmapBytes',0) for x in resources)/2**20),fmt(sum(x.get('releasedTextureBytes',0) for x in resources)/2**20),fmt(r['maxRustLiveAllocationMiB']),fmt(r['maxRustHeapCapacityMiB']),fmt(r['maxSkiaHeapCapacityMiB'])])
table(['环境','末次存活内容 / MiB','其中原生位图 / MiB','累计释放内容 / MiB','单实例 Rust 请求分配峰值 / MiB','Rust 堆容量 / MiB','CanvasKit 堆容量 / MiB'],rows)
emit('## 帧预算与缓存');emit()
rows=[]
for r in new:
 s=r['stableAnimation'];c=r['secondTraversalCache'];rows.append([r['file'].split('/')[0].replace('optimized-',''),fmt(s['rasterP95Ms']),fmt(s['gpuP95Ms']),fmt(s['worstPageActiveReplyP95Ms']),len(r['holdRedrawViolations']),f"{c.get('layerCacheBuilds')}/{c.get('staticCacheBuilds')}/{c.get('effectDraws')}",r['activeForegroundFramesDuringOtherWorkerPreparation']])
table(['环境','稳定帧提交 P95 / ms','GPU 查询 P95 / ms','最慢页回复间隔 P95 / ms','静止重复绘制','第二轮层／静态／效果重建','另一 Worker 准备时的前台活动帧'],rows)
emit('提交耗时、GPU 查询、Worker 回复间隔和物理显示刷新率不同；低提交耗时不能证明实际 60 fps。后台与前台任务重叠也不等于驱动保证两组 GPU 指令同时执行。');emit()
emit('各页的提交、GPU 与回复间隔另见 [per-page.csv](per-page.csv)，避免整篇平均掩盖局部慢页。4K 软件路径若超过 16.67 ms，会在末尾明确判为未达到帧预算，不能将“不再卡死”当成全部达标。');emit()
emit('## 首次进入与冷阴影页');emit()
rows=[]
for name in ['v24','v26']:
 f=a.records/'first-entry'/name/'result.json'
 if f.exists():
  r=read(f);rows.append([name,r['samples'],r['status'],fmt(r['firstVisibleP95Ms']),fmt(r['completeP95Ms'])])
table(['首次进入版本','全新浏览器样本','结果','打开调用到可见检查点 P95 / ms','切页开始到准备完成 P95 / ms'],rows)
emit('首次可见通过提交通知后的 rAF／任务检查点独立观察，包含打开时的运行时初始化；准备完成字段从切页开始计时。两列起点不同，不相减推断光栅化时间。旧采集器的空值记录保留在 invalidated，不参与统计。');emit()
rows=[]
for f in sorted((a.records/'cold-navigation').glob('*/result.json')):
 r=read(f);before_file=a.previous/'cold-navigation'/('optimized-'+str(r['width']))/'result.json'
 before=read(before_file) if f.parent.name.startswith('chromium-') and f.parent.name.endswith('-hardware') and before_file.exists() else None
 reduction=(1-r['firstVisibleP95Ms']/before['firstVisibleP95Ms'])*100 if before and r['firstVisibleP95Ms'] is not None and before['firstVisibleP95Ms'] else None
 rows.append([f.parent.name,f"{r['visible']}/{r['samples']}",fmt(before['firstVisibleP95Ms'] if before else None)+' → '+fmt(r['firstVisibleP95Ms']),fmt(reduction)+'%' if reduction is not None else '无同条件冷页对照',r['status']])
table(['从渐变页输入阴影页','成功样本','开始变化 P95 / ms，v24 → 新版','相对 v24 降幅','结果'],rows)
baseline_rows=[]
for width in [1920,3840]:
 for name,label in [('original','原附件'),('baseline','优化前运行时')]:
  f=a.previous/'cold-navigation'/f'{name}-{width}'/'result.json'
  if f.exists():
   r=read(f);baseline_rows.append([f'{label} / Chromium 硬件 / {width}',f"{r['visible']}/{r['samples']}",fmt(r['firstVisibleP95Ms']),r['status']])
table(['历史同机冷阴影页对照','成功样本','P95 / ms','结果'],baseline_rows)
emit('优化前 4K 冷页样本全部超时，不能为这组数据计算有效 P95 或百分比降幅。原附件与优化前运行时的完整自然播放记录也保留在 historical-comparison。');emit()
emit('冷页输入测试使用独立新浏览器：真实进入 P15，观测后台 P16 请求，等待 150 ms 后通过键盘 16 + Enter 切页；与完整自然播放单独报告。');emit()
emit('## 功能、画质与生命周期');emit()
for folder in ['interaction','reopen-50']:
 for f in sorted((a.records/folder).glob('*.json')):
  if f.stem not in ['chromium','firefox']:continue
  r=read(f);s=r.get('samples',[])
  if not s:continue
  q=lambda field:sorted(x[field] for x in s)[max(0,__import__('math').ceil(len(s)*.95)-1)]
  last=[x['afterClose']['pss']/2**20 for x in s[-10:]]
  first=[x['afterClose']['pss']/2**20 for x in s[:10]]
  renderers=sorted({x['runtime']['renderers'] for x in s})
  window=min(10,len(s))
  emit(f"- {folder} / {f.stem}：{r['status']}，{len(s)} 轮；菜单 P95 {q('menuMs')} ms，退出 P95 {q('exitMs')} ms；前／末 {window} 轮 PSS 中位数 {fmt(statistics.median(first))} → {fmt(statistics.median(last))} MiB，末 {window} 轮范围 {fmt(min(last))}–{fmt(max(last))} MiB；存活渲染器数量 {renderers}；离线编辑、保存及重新打开 {'通过' if r.get('offlineEditSaved') else '未通过'}。")
emit('- Chromium、Firefox 的 26 页逐对象和 64×64 局部补丁画面对照；Firefox 4K 阴影／半透明重点页；透明淡入、旋转、延迟提交、缓存单一所有权与释放检查。各项结果见对应 JSON。')
disposition=read(a.records/'software-backend-quality-disposition.json') if (a.records/'software-backend-quality-disposition.json').exists() else None
if disposition:
 emit(f"- 额外的跨后端严格像素阈值检查未通过，原失败记录完整保留。复核全部 {disposition['checkedObjectTimes']} 个对象／时刻条目的最差局部后，新版相对 8 倍直接矢量采样参考均不劣于旧软件 GPU；其中 {disposition['pixelExactOldCpuPatches']} 个局部与旧 CPU 输出逐像素相同，其余局部相对旧 CPU 的最大 8 位通道平均差为 {disposition['maxCpuControlPatchMean8bit']:.3f}。因此接受为后端抗锯齿差异，不宣称 CPU／GPU 像素相同，也没有放宽原失败阈值。见 [局部画面对照](backend-aa.html)。")
emit('- 硬件 GPU 130 组缓存／直接绘制对照，以及实际 VS Code 预览、转场、禁用 Worker 资源请求的回归。原生 CLI 和内嵌编辑运行时已重新构建。')
emit('- 所有测试按顺序执行；源文件快照、候选包 SHA、资源字节估算、WASM 请求分配与容量、PSS 和可获取的 DRM 字段分别保留。缺少实测数据不填零。');emit()
emit('## 性能目标检查');emit()
misses=[]
for r in new:
 label=r['file'].split('/')[0]
 if r['traversalStatus']!='passed' or r['visited']!=52 or r['taskErrors']:misses.append(label+'：未完成两轮或有任务错误')
 if r['firstVisibleWarmP95Ms'] is None or r['firstVisibleWarmP95Ms']>200:misses.append(label+'：已准备切页 P95 未达到 200 ms')
 if r['stableAnimation']['rasterP95Ms'] is None or r['stableAnimation']['rasterP95Ms']>1000/60:misses.append(label+'：稳定帧提交 P95 超过 60 Hz 的 16.67 ms 预算')
 if r['holdRedrawViolations']:misses.append(label+'：静止阶段发生重复绘制')
 if any((r['secondTraversalCache'].get(k) or 0)>0 for k in ['layerCacheBuilds','staticCacheBuilds','effectDraws']):misses.append(label+'：第二轮有缓存或效果重建')
for folder in ['interaction','reopen-50']:
 for f in sorted((a.records/folder).glob('*.json')):
  if f.stem not in ['chromium','firefox']:continue
  r=read(f);samples=r.get('samples',[])
  for field,label in [('menuMs','菜单'),('exitMs','退出')]:
   values=sorted(x[field] for x in samples)
   if not values or values[max(0,__import__('math').ceil(len(values)*.95)-1)]>100:misses.append(f'{folder}/{f.stem}：{label} P95 未达到 100 ms')
  if r.get('status')!='passed' or not r.get('offlineEditSaved'):misses.append(f'{folder}/{f.stem}：交互或离线保存重新打开未通过')
for f in sorted((a.records/'cold-navigation').glob('*/result.json')):
 r=read(f)
 if r['status']!='passed':misses.append(f'{f.parent.name}：冷阴影页输入存在失败样本')
 if f.parent.name.startswith('chromium-') and f.parent.name.endswith('-hardware'):
  # The 50% goal is against the pre-optimization baseline, not an already
  # optimized v24. Keep censored baseline failures out of percentages.
  before_file=a.previous/'cold-navigation'/('baseline-'+str(r['width']))/'result.json'
  if before_file.exists():
   before=read(before_file)
   if before['status']=='passed' and (r['firstVisibleP95Ms'] is None or r['firstVisibleP95Ms']>before['firstVisibleP95Ms']*.5):misses.append(f'{f.parent.name}：相对优化前冷阴影页 P95 降幅未达到 50%')
if misses:
 for item in misses:emit('- '+item)
else:emit('本次自然播放的已准备切页、稳定提交帧预算、静止保持和第二轮缓存检查达到上述阈值；物理呈现帧率未由这些指标证明。')
emit();emit('未测边界：WebKit 本轮暂缓；其他 GPU、操作系统和实体屏幕帧呈现不外推为通过。反复往返和退出重开的有限记录不等于无限时长泄漏证明。')
emit();emit('诊断中还尝试了合并缩放／位移、背景整数区域拆分和局部重绘。由于跨浏览器收益不稳定或个别页面回退，均未进入交付运行时；相关记录单独保留，不计入优化成果。')
a.output.mkdir(parents=True,exist_ok=True);text='\n'.join(lines)+'\n';(a.output/'results.zh-CN.md').write_text(text)
try:
 import markdown
 body=markdown.markdown(text,extensions=['tables','fenced_code'])
except ImportError:
 # This report emits only headings, paragraphs, simple lists and tables.
 # Keep its standalone HTML readable without adding a project dependency.
 def inline(value):
  value=html.escape(value)
  value=re.sub(r'`([^`]+)`',r'<code>\1</code>',value)
  return re.sub(r'\[([^\]]+)\]\(([^\s)]+)\)',r'<a href="\2">\1</a>',value)
 blocks=[]
 for block in text.strip().split('\n\n'):
  rows=block.splitlines()
  if rows[0].startswith('| '):
   cells=lambda row:[inline(v.strip()) for v in row.strip('|').split('|')]
   header='<tr>'+''.join('<th>'+v+'</th>' for v in cells(rows[0]))+'</tr>'
   contents=''.join('<tr>'+''.join('<td>'+v+'</td>' for v in cells(row))+'</tr>' for row in rows[2:])
   blocks.append('<table><thead>'+header+'</thead><tbody>'+contents+'</tbody></table>')
  elif rows[0].startswith('#'):
   level=len(rows[0])-len(rows[0].lstrip('#'));blocks.append(f'<h{level}>'+inline(rows[0][level:].strip())+f'</h{level}>')
  elif all(row.startswith('- ') for row in rows):blocks.append('<ul>'+''.join('<li>'+inline(row[2:])+'</li>' for row in rows)+'</ul>')
  else:blocks.append('<p>'+'<br>'.join(inline(row) for row in rows)+'</p>')
 body='\n'.join(blocks)
(a.output/'results.html').write_text('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Open Props 动画优化后续验证</title><style>body{max-width:1200px;margin:40px auto;padding:0 24px;font:16px/1.8 system-ui;color:#1c2a35;background:#fafbf9}table{border-collapse:collapse;display:block;overflow:auto;font-size:14px}th,td{border:1px solid #cad4d6;padding:8px 12px;text-align:left}code{overflow-wrap:anywhere}h1,h2{line-height:1.4}a{color:#17644f}</style>'+body+'</html>')
(a.output/'acceptance.json').write_text(json.dumps({'misses':misses,'pssPeakIncreasesVsV24':increased,'crossBackendPixelEquality':'failed' if disposition else 'not reviewed','crossBackendQualityDisposition':disposition['status'] if disposition else None,'webkit':'deferred by user','physicalScanout':'not measured'},ensure_ascii=False,indent=2)+'\n')
if disposition:
 raw=read(a.records/'backend-aa-review/result.json');panels=[];(a.output/'aa').mkdir(exist_ok=True)
 for page in raw['results']:
  if not page['checks']:continue
  c=page['checks'][0];cells=[]
  for name,label in [('oldGPU','旧软件 GPU'),('oldCPU','旧 CPU'),('newCPU','新版 CPU'),('reference8x','8 倍采样参考')]:
   filename=page['page']+'-0-'+name+'.png';shutil.copy2(a.records/'backend-aa-review'/filename,a.output/'aa'/filename)
   metric='' if name=='reference8x' else f" · 平均误差 {c['metrics'][name]['mae']:.3f}"
   cells.append('<td>'+label+metric+'<br><img src="aa/'+filename+'" alt="'+label+'"></td>')
  panels.append('<h2>'+page['page']+' · '+html.escape(c['pick']['id'])+' · '+str(c['pick']['time'])+' s</h2><table><tr>'+''.join(cells)+'</tr></table>')
 (a.output/'backend-aa.html').write_text('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>软件后端抗锯齿复核</title><style>body{max-width:1200px;margin:40px auto;padding:0 20px;font:16px/1.8 system-ui;background:#fafbf9;color:#1c2a35}table{width:100%;table-layout:fixed;border-collapse:collapse}td{vertical-align:top;border:1px solid #ccd6d5;padding:8px;font-size:13px}img{image-rendering:pixelated;max-width:100%;width:256px}a{color:#17644f}</style><h1>软件后端抗锯齿复核</h1><p>原严格像素比较失败记录没有改为通过。这里对每页差异最大的条目展示其最差局部，最近邻放大用于看清像素；生产分辨率未改变。所有 803 个失败条目的局部均已检查，不以页面平均掩盖差异。8 倍参考使用未修改的旧 CPU 直接矢量绘制后平均取样。</p><p><a href="results.html">返回完整验证记录</a></p>'+''.join(panels)+'</html>')
with (a.output/'per-page.csv').open('w',newline='') as f:
 fields=['case','version','page','frames','rasterP95Ms','gpuP95Ms','activeReplyIntervalP95Ms','firstVisibleP95Ms'];writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader()
 for r in new:
  for version,record in [('v24',old.get(r['file'],{})),('new',r)]:
   for page in record.get('pages',[]):writer.writerow({'case':r['file'].split('/')[0],'version':version,**{k:page.get(k) for k in fields[2:]}})
print(json.dumps({'report':str(a.output),'misses':misses},ensure_ascii=False))
