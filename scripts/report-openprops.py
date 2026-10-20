#!/usr/bin/env python3
"""Build a small reviewable report from completed Open Props measurements."""
import argparse, csv, html, json, shutil
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('records',type=Path)
p.add_argument('output',type=Path)
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
for name in ['summary.json','environment.json','preservation.json','functional-summary.json','source-manifest.json','native.json']:
    source=a.records/name;target=a.output/name
    if source.exists() and source.resolve()!=target.resolve():shutil.copy2(source,target)
summary=json.loads((a.records/'summary.json').read_text())
functional=json.loads((a.records/'functional-summary.json').read_text())
environment=json.loads((a.records/'environment.json').read_text())
preserved=json.loads((a.records/'preservation.json').read_text())
runs=summary['runs']
optimized=[r for r in runs if r['file'].startswith('optimized-')]
fmt=lambda n:'未测' if n is None else f'{n:,.1f}'
table=lambda heads,rows:'\n'.join(['| '+' | '.join(heads)+' |','|'+'|'.join(['---']*len(heads))+'|',*['| '+' | '.join(map(str,row))+' |' for row in rows]])
lines=['# Open Props 动画与实际资源占用对照',
       '自然播放优化版已记录 %d / 8 个环境组合，其中 %d 个完成全部两轮且无任务错误。报告保留未通过及未测项目，不以界面没有卡死代替验收。'%(len(optimized),sum(r['traversalStatus']=='passed' and r['visited']==52 for r in optimized)),
       '作品为原附件的 26 页；源码、字体、图片和显示列表共 %d 项逐项 SHA-256 相同，改变 %d 项。没有缩短动画或降低输出分辨率。'%(preserved['checkedEntries'],len(preserved['changedEntries'])),
       '测试环境：%s，%d 个逻辑 CPU，Linux %s。浏览器依次测试，版本及实际后端见下表。'%(environment['cpu'],environment['logicalCpus'],environment['kernel']),
       '## 自然首遍与重复遍历',
       '每个完整记录应完成 52 次访问（26 页 × 2 轮）。失败记录保留；未完成记录的内存峰值不能与完整遍历直接计算降幅。测试遵循作品原有停留时间，没有提前等待全篇准备、跳到终态或在计时期间截图。',
       table(['记录','完成访问','状态','实际后端','PSS 采样峰值 / MiB','DRM VRAM 采样峰值 / MiB','已准备切页 P95 / ms','保持阶段重复绘制'],[
           [r['file'].split('/')[0],r['visited'],r['traversalStatus'],r['backend'] or '未测',fmt(r['peakPssMiB']),fmt(r['peakDrmVramMiB']),fmt(r['firstVisibleWarmP95Ms']),len(r['holdRedrawViolations'])] for r in runs]),
       'PSS 按 /proc 中完整 PPid 进程树采集，包含非主线程创建的子进程。早期仅遍历主线程 children 的记录已作废，不用于资源降幅；DRM 为 Linux 驱动客户端驻留数据，按设备与客户端 ID 去重，每 2 秒采样。驻留量包含共享缓冲，不等于显卡全局独占物理用量。共享缓冲字段另存 JSON，并保留原始客户端记录；不从客户端求和推断显卡全局独占物理用量。集成 GPU 的 VRAM 是驱动内存域，不等于独立显卡板载显存；软件驱动或缺少 DRM 数据时记为未测。',
       '## 两轮结束时的驻留量',
       '以下为相同操作位置的实际采样，属于本次有限遍历的证据，并不代表任意长时间运行的泄漏证明。缓存未因为轮次而清空。',
       table(['记录','轮次','PSS / MiB','DRM VRAM / MiB'],[[r['file'].split('/')[0],e['round']+1,fmt(e['pssMiB']),fmt(e['drmVramMiB'])]for r in runs for e in r['roundEnds']]),
       '## 运行时、分配与重建',
       '本表为放映 Worker 的最大单实例数据；各渲染器末次观测分项另存 JSON，整个浏览器占用以 PSS 为准。Rust 活跃分配是分配器请求字节，不含静态区、栈及分配器元数据。WASM 堆容量另外记录；CanvasKit 未公开活跃分配器数据，因此只列其容量。逻辑 RGBA 缓存字节与驱动显存分开保存在 summary.json。',
       table(['记录','单实例 Rust 活跃分配峰值 / MiB','单实例 Rust 堆容量 / MiB','单实例 CanvasKit 堆容量 / MiB','动画期间层重建','动画期间效果重绘'],[
           [r['file'].split('/')[0],fmt(r['maxRustLiveAllocationMiB']),fmt(r['maxRustHeapCapacityMiB']),fmt(r['maxSkiaHeapCapacityMiB']),sum(x['layerBuildsDuringPlayback']or 0 for x in r['pages']) if any(x['layerBuildsDuringPlayback']is not None for x in r['pages'])else '未测',sum(x['effectPictureReplaysDuringPlayback']or 0 for x in r['pages']) if any(x['effectPictureReplaysDuringPlayback']is not None for x in r['pages'])else '未测']for r in runs]),
       '前台与后台使用独立 Worker 和渲染上下文。后台分块准备、等待 GPU fence 并接受取消；前台耗时反馈控制后台提交间隔。任务重叠说明软件队列不再相互等待，不代表已测得 GPU 硬件同时执行。',
       '## 冷切到阴影页',
       '独立压力测试：每次新开浏览器与原文件，打开 P15，观察后台 P16 请求实际提交，150 ms 后使用正常的 16 + Enter 快捷键。该测试刻意提前切页；自然播放另行测量。失败按失败记录，不作为零延迟计入 P95。']
resume_environment=a.records/'environment-resume.json'
if resume_environment.exists():
    cohort=json.loads(resume_environment.read_text())
    lines.insert(4,'测量期间主机重启，启动时间约为 '+cohort['currentBootApproximateUtc']+'（UTC）。已完成记录保留原时间戳；中断的软件 4K 记录不参与汇总，恢复后从第一页重跑。硬件 1920 配对结果来自重启前；其余配对的时间与运行环境边界见 environment-resume.json，不将跨启动批次当成同一次连续实验。')
    if (a.output/'environment-resume.json').resolve()!=resume_environment.resolve():shutil.copy2(resume_environment,a.output/'environment-resume.json')
failures=[r for r in runs if r['traversalStatus']!='passed']
if failures:
    lines[-2:-2]=['失败记录：',table(['记录','观测到的失败'],[[r['file'].split('/')[0],r['failure'].splitlines()[0] if r.get('failure') else '; '.join(e['error'].splitlines()[0] for e in r['taskErrors'][:3])]for r in failures])]
lines[lines.index('## 冷切到阴影页'):lines.index('## 冷切到阴影页')]=[
    '## 稳定缓存后的 60 Hz 帧预算',
    '这些页面的动画属性为 transform / opacity。下面仅统计没有静态或对象缓存重建、也没有 CPU 像素表示转换的活动帧；16.67 ms 是单帧预算。提交耗时、异步 GPU 时间和逐页回复间隔分别保存。表中最慢页面的活动回复间隔可揭示驱动或调度造成的低帧率；仅提交耗时低于预算不代表达到 60 Hz。这些指标都不能直接替代物理显示器帧率。',
    table(['记录','稳定帧数','光栅提交 P95 / ms','GPU P95 / ms','最慢页面活动回复间隔 P95 / ms','提交超过 16.67 ms 的帧数','活动期间跳过请求数'],[[r['file'].split('/')[0],r['stableAnimation']['frames'],fmt(r['stableAnimation']['rasterP95Ms']),fmt(r['stableAnimation']['gpuP95Ms']),fmt(r['stableAnimation']['worstPageActiveReplyP95Ms']),r['stableAnimation']['framesWithRasterOver60HzBudget'],r['activeSkippedPaintRequests']]for r in optimized]),
    '## 第二轮实际重建次数',
    '单独检查第二轮全部实际绘制请求，不预先剔除发生重建的帧。首次像素表示转换单列；缺少旧版计数时记为未测。',
    table(['记录','实际绘制请求','对象缓存重建','静态缓存重建','直接效果绘制','效果内容重放','CPU 像素表示转换'],[[r['file'].split('/')[0],r.get('secondTraversalCache',{}).get('observedPaints',0),*[fmt(r.get('secondTraversalCache',{}).get(k))for k in ['layerCacheBuilds','staticCacheBuilds','effectDraws','effectPictureReplays','cpuTileConversions']]]for r in optimized]),
    '## 前台与后台任务重叠',
    table(['记录','后台准备期间前台完成活动帧数','这些前台帧的光栅提交 P95 / ms'],[[r['file'].split('/')[0],r['activeForegroundFramesDuringOtherWorkerPreparation'],fmt(r['activeForegroundRasterP95DuringOtherWorkerPreparationMs'])]for r in optimized]),
    '此处只统计视觉属性正在变化的动画区间，并检查不同 Worker 的请求时间区间重叠，验证前台无需等待整页后台准备；不把它称为 GPU 物理并行度测量。'
]
cold=[]
for f in sorted((a.records/'cold-navigation').glob('*/result.json')):
    if '-pilot' in f.parent.name:continue
    d=json.loads(f.read_text());cold.append((f.parent.name,d))
lines.append(table(['记录','可见 / 尝试','状态','首次变化 P95 / ms'],[[name,f"{d['visible']} / {d['samples']}",d['status'],fmt(d['firstVisibleP95Ms'])] for name,d in cold])if cold else '测量尚未结束。')
cold_by_name=dict(cold)
for width in [1920,3840]:
    old=cold_by_name.get('baseline-'+str(width));new=cold_by_name.get('optimized-'+str(width))
    if old and new and old['visible']==old['samples'] and new['visible']==new['samples'] and old['firstVisibleP95Ms'] and new['firstVisibleP95Ms'] is not None:
        improvement=100*(1-new['firstVisibleP95Ms']/old['firstVisibleP95Ms'])
        lines.append(f'{width} 像素宽冷切页 P95 降幅：{improvement:.1f}%；50% 目标'+('达到。'if improvement>=50 else '未达到。'))
retry_summary=a.records/'retries-summary.json'
if retry_summary.exists():
    retry_runs=json.loads(retry_summary.read_text())['runs']
    lines.extend(['## WebKit 崩溃复核','首次 WebKit 1920 × 920 测试在第二轮第 22 页前发生 WebProcess 崩溃，完成 47 次访问。该失败保留在主矩阵，没有用复跑覆盖。首次失败未保存完整调用轨迹；已修正测试收集器，使页面崩溃后仍保留外部已收到的调用和导航记录。',table(['复跑记录','完成访问','状态','已准备首次变化 P95 / ms','活动缓存帧绘制 P95 / ms','PSS 采样峰值 / MiB'],[[r['file'].split('/')[0],r['visited'],r['traversalStatus'],fmt(r['firstVisibleWarmP95Ms']),fmt(r['stableAnimation']['rasterP95Ms']),fmt(r['peakPssMiB'])]for r in retry_runs]),'一次相同候选、尺寸和操作序列的完整复跑未再次崩溃。原因尚未确认，不能将未复现写成故障已消除。初次崩溃和复跑的原始结果，以及可用日志边界，见 webkit-crash-review.json 和验证压缩包。'])
    for name in ['retries-summary.json','webkit-crash-review.json']:
        source=a.records/name
        if source.exists() and source.resolve()!=(a.output/name).resolve():shutil.copy2(source,a.output/name)
lines.extend(['## 目标检查',table(['项目','本次结果'],[
    ['优化版完整遍历且无任务错误（不代表性能达标）',f"{sum(r['traversalStatus']=='passed' and r['visited']==52 for r in optimized)} / 8 通过"],
    ['已准备页首次变化 P95 ≤ 200 ms',f"{sum(r['traversalStatus']=='passed' and r['visited']==52 and r['firstVisibleWarmP95Ms'] is not None and r['firstVisibleWarmP95Ms']<=200 for r in optimized)} / {len(optimized)} 个已记录组合满足"],
    ['保持阶段不持续重绘',f"{sum(r['traversalStatus']=='passed' and r['visited']==52 and not r['holdRedrawViolations'] for r in optimized)} / {len(optimized)} 个已记录组合满足"],
    ['菜单及退出 P95 ≤ 100 ms','通过（每浏览器 5 次）'if len(functional['interaction'])==3 and all(r['menuP95Ms']<=100 and r['exitP95Ms']<=100 for r in functional['interaction'])else '未通过'],
    ['其他页面不得出现明显回退','未通过：本机硬件 P01 首次进入观察到延迟增加，见下节' if (a.records/'first-entry-regression.json').exists() else '未单独判定'],
    ['其他 GPU / 操作系统','未测']])])
first_entry=a.records/'first-entry-regression.json'
if first_entry.exists():
    issue=json.loads(first_entry.read_text());evidence=issue['evidence']
    before=evidence['baseline']['visits'][0]['transition']['firstVisibleMs'];after=evidence['optimized']['visits'][0]['transition']['firstVisibleMs']
    lines.extend(['## 首次进入第 1 页的回退',f'本机硬件 Chromium、1920 × 920 的完整两轮对照中，P01 初次进入的首次变化由 {before:.1f} ms 增至 {after:.1f} ms。这里各版本只有一次冷进入观察，不称为多次冷启动 P95，也不与下方 20 次阴影页压力测试混用。该回退尚未修复，因此不能判定其他页面均无性能回退。', '新路径在首屏可见前准备了 53 块内容，入口准备约 374 ms，其中 GPU 完成等待约 224 ms。等待包含驱动／浏览器确认延迟，不等于 GPU 实际执行时长。逐次访问、阶段数据和文件身份见 first-entry-regression.json。'])
    if (a.output/first_entry.name).resolve()!=first_entry.resolve():shutil.copy2(first_entry,a.output/first_entry.name)
bottleneck=a.records/'remaining-bottlenecks.json'
if bottleneck.exists():
    issue=json.loads(bottleneck.read_text())
    lines.extend(['## 尚未解决的 CPU 工作块延迟',f"Firefox CPU 的 {issue['target']} 激活请求，与同一 Worker 中后台 {issue['backgroundPage']} 的同步阴影块重叠。该块耗时 {issue['backgroundPreparation']['maxTaskMs']:.0f} ms，目标请求排队 {issue['queueWaitMs']:.0f} ms。取消在工作块边界生效；此例仍超过 200 ms 切页目标。独立前台能够继续绘制，不等于后台每个 CPU 工作块都已满足响应预算。原始时间区间与文件身份保存在 remaining-bottlenecks.json。"])
    if (a.output/'remaining-bottlenecks.json').resolve()!=bottleneck.resolve():shutil.copy2(bottleneck,a.output/'remaining-bottlenecks.json')
if functional.get('cpuBlit'):
    lines.extend(['## CPU 缓存像素合成校验','三个浏览器使用相同缓存网格对比原 Skia 合成与新路径，覆盖真实 P12/P17 的 4K 中间态和终态、透明画布上的半透明叠加、旋转与嵌套裁剪回退，并断言每块只长期保留一种像素表示。',table(['浏览器','状态','比较数','最大通道差 / 255'],[[r['engine'],r['status'],r['checks'],r['maxChannelDifference']]for r in functional['cpuBlit']]),'不透明页面比较 RGBA，透明叠加比较预乘 RGBA。首次转换计入首帧时间和实际资源分配；稳定缓存帧预算单独排除这些一次性转换，并在逐页 CSV 中列出转换次数。'])
probe=a.records/'backend-probe'
if all((probe/(name+'-summary.json')).exists() for name in ['cpu','software-gpu']):
    diagnostics=[]
    for name in ['cpu','software-gpu']:
        d=json.loads((probe/(name+'-summary.json')).read_text())['runs'][0]
        diagnostics.append([name,d['visited'],fmt(d['firstVisibleWarmP95Ms']),d['stableAnimation']['frames'],fmt(d['stableAnimation']['worstPageActiveReplyP95Ms']),fmt(d['peakPssMiB'])])
    lines.extend(['## 同浏览器后端诊断','同一 Chromium、同一候选文件、3840 × 1840、P01/P02 两轮自然播放。CPU 使用测试注入的 software:true，交付源码没有因此统一切换后端。该小样本只用于解释取舍，不替代 26 页验收。',table(['后端','访问次数','已准备首次变化 P95 / ms','活动缓存帧数','最慢页回复间隔 P95 / ms','PSS 采样峰值 / MiB'],diagnostics),'这个样本中 CPU 的切页更快，但完成的活动动画帧更少；单一指标的收益不足以证明全局切换会更好。'])
density_probe=a.records/'diagnostics/layer-density.json'
if density_probe.exists():
    d=json.loads(density_probe.read_text());rows=[]
    for variant,name in enumerate(d['variants']):
        timings=[r for r in d['timings'] if r['variant']==variant and r['round']>0]
        checks=[c for c in d['checks'] if c['variant']==variant]
        average=lambda key:sum(r[key] for r in timings)/len(timings)
        phases=lambda key:sum(r['phases'][key] for r in timings)/len(timings)
        rows.append([name,fmt(average('rasterMs')),fmt(phases('static')),fmt(phases('layers')),fmt(phases('flush')),sum(r['layerBuilds'] for r in timings),max((c['max'] for c in checks),default=0)])
    lines.extend(['## 缓存合成诊断','Firefox CPU，3840 × 1840，真实 P12，固定分块规则，中间态与终态。候选包保持原有密度安全余量；另两个试验只修改 QA 夹具，分别移除数值余量及替换图像绘制调用。这是定位测试，不混入自然播放验收。',table(['试验','平均绘制 / ms','静态合成 / ms','对象合成 / ms','显示转换 / ms','重复层构建','最大通道差'],rows)])
resource_pairs=[]
for new in optimized:
    old=next((r for r in runs if r['file'].split('/')[0]==new['file'].split('/')[0].replace('optimized-','baseline-',1)),None)
    if not old or any(r['traversalStatus']!='passed' or r['visited']!=52 or r.get('memoryProtocol')!='proc-ppid-v2' for r in [old,new]):continue
    def change(key):
        a,b=old.get(key),new.get(key)
        return '未测' if a is None or b is None or not a else f'{100*(b/a-1):+.1f}%'
    resource_pairs.append([new['file'].split('/')[0],fmt(old['peakPssMiB']),fmt(new['peakPssMiB']),change('peakPssMiB'),fmt(old['peakDrmVramMiB']),fmt(new['peakDrmVramMiB']),change('peakDrmVramMiB')])
lines.extend(['## 可直接配对的资源变化','仅比较同环境、同分辨率、同采样协议且完成 52 次访问的记录。负值为下降，正值为上升；内存与显存分别判断，不以其中一项下降代表两项都下降。',table(['环境','之前 PSS / MiB','之后 PSS / MiB','PSS 变化','之前 VRAM / MiB','之后 VRAM / MiB','VRAM 变化'],resource_pairs)if resource_pairs else '暂无完整可配对记录。',
    table(['记录','完整进程树采样数','进程数量范围','采样 P95 / ms','包含已退出／不可读进程的样本数'],[[r['file'].split('/')[0],r.get('memorySamples',0),r.get('memoryProcessRange'),fmt(r.get('memorySamplingP95Ms')),r.get('memoryUnreadableSamples')]for r in runs])])
lines.extend(['## CPU 缓存表示与工作缓冲','CPU 像素数组替换对应的原生图像，是逻辑缓存字节的子集；不把它漏算，也不与逻辑缓存再次相加。转换时临时双持按资源分配计数，表中工作字节是按像素缓冲请求大小记录的上界；整体驻留仍以浏览器 PSS 为准，不能用 CanvasKit 堆容量下降代替实际内存下降。',table(['记录','单渲染器 CPU 缓存像素峰值 / MiB','转换工作缓冲上界 / MiB'],[[r['file'].split('/')[0],fmt(r.get('maxCpuTilePixelBytes')/2**20 if r.get('maxCpuTilePixelBytes')is not None else None),fmt(r.get('maxCpuTileConversionWorkingBytes')/2**20 if r.get('maxCpuTileConversionWorkingBytes')is not None else None)]for r in optimized])])
reopen=functional.get('reopen')
if reopen:
    lines.extend(['## 连续退出与重开',f"WebKit 连续 {reopen['cycles']} 次退出／重开：{reopen['status']}。首次退出后 PSS {reopen['firstPssMiB']:.1f} MiB；最后十次 {reopen['lastTenPssMinMiB']:.1f}–{reopen['lastTenPssMaxMiB']:.1f} MiB。后续准备可能保留新增有效内容，首末差值不能独立证明泄漏或无泄漏；资源与缓存复用分项在 functional-summary.json。"])
lines.extend(['## 操作、离线编辑与画质',
              table(['浏览器','版本','菜单 P95 / ms','退出 P95 / ms','样本数','离线编辑、保存、重新打开'],[[r['engine'],r['version'],r['menuP95Ms'],r['exitP95Ms'],r['samples'],'通过'if r['offlineEditSaved']else '失败']for r in functional['interaction']]),
              'CPU 回退共检查 %d 页、%d 个对象区域；最差对象平均通道差 %.4f / 255。真实 VS Code 回归：%s。原生渲染、增量更新、透明叠加、中间态、终态、转场及缓存生命周期的记录随验证文件提供。'%(functional['cpuPages'],functional['cpuObjectChecks'],functional['cpuWorstMeanObject']['mean'],functional['vscode']),
              '## 解释边界',
              '- 帧回复间隔不等于物理显示器扫描输出时间；GPU timer、fence 等待和主线程请求排队分别记录。CanvasKit 内部上传尚未独立暴露，不能把缺失值写成零。',
              '- 硬件测试以设备字符串为准；本机 AMD 的结果不能代替其他显卡或驱动。所有未测场景不标记通过。',
              '- 完整逐页数据和准备阶段记录见 summary.json 与 per-page.csv；原始测试记录保留失败、异常及后端信息。',
              '- 不发布版本。旧离线 HTML 不会自动获得运行时修复。',
              '优化包 SHA-256：`'+preserved['optimizedSha256']+'`。'])
markdown='\n\n'.join(lines)+'\n';(a.output/'results.zh-CN.md').write_text(markdown)
with (a.output/'per-page.csv').open('w',newline='')as f:
    cols=['run','page','frames','activeReplyIntervalP95Ms','rasterP95Ms','gpuP95Ms','requestP95Ms','firstVisibleP95Ms','layerBuildsDuringPlayback','effectPictureReplaysDuringPlayback','cpuTileConversionsDuringPlayback']
    writer=csv.DictWriter(f,fieldnames=cols);writer.writeheader()
    for r in runs:
        for row in r['pages']:writer.writerow({'run':r['file'].split('/')[0],**row})
body=[]
for block in lines:
    if block.startswith('# '):body.append('<h1>'+html.escape(block[2:])+'</h1>')
    elif block.startswith('## '):body.append('<h2>'+html.escape(block[3:])+'</h2>')
    elif block.startswith('|'):
        rows=block.splitlines();cells=lambda row:[html.escape(x.strip())for x in row.strip('|').split('|')]
        body.append('<div class="table"><table><thead><tr>'+''.join('<th>'+x+'</th>'for x in cells(rows[0]))+'</tr></thead><tbody>'+''.join('<tr>'+''.join('<td>'+x+'</td>'for x in cells(row))+'</tr>'for row in rows[2:])+'</tbody></table></div>')
    else:body.append('<p>'+html.escape(block.removeprefix('- '))+'</p>')
(a.output/'results.html').write_text('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Open Props 动画性能验证</title><style>body{font:16px/1.7 system-ui,sans-serif;max-width:1250px;margin:40px auto;padding:0 24px;color:#172a37;background:#f5f7fa}h1{font-size:30px}h2{margin-top:38px}.table{overflow:auto;background:white;border:1px solid #d9e0e7;border-radius:8px}table{border-collapse:collapse;width:100%;font-size:13px}th,td{text-align:left;border-bottom:1px solid #e1e6ec;padding:10px;white-space:nowrap}th{background:#eaf0f5}p{max-width:1000px}a{color:#1659b7}</style><body><p><a href="index.html">下载与对照入口</a> · <a href="summary.json">详细 JSON</a> · <a href="per-page.csv">逐页 CSV</a></p>'+''.join(body)+'</body></html>')
print(a.output/'results.html')
links=[('06_OpenProps_可编辑_优化版.zip.html','优化版'),('06_OpenProps_当前运行时_对照.zip.html','优化前运行时'),('06_OpenProps_原附件.zip.html','原附件')]
complete=len(runs)==22 and len(optimized)==8 and len(cold)==6 and len(functional['interaction'])==3
archive_link='<p><a href="verification.zip">可复现验证记录与脚本</a></p>' if (a.output/'verification.zip').exists() else '<p>原始验证记录仍在整理。</p>'
comparison='<article><h2>相同时间的画面对照</h2><p>P17，0.65 s，1920 × 920，CPU 后端。用于核对阴影、透明叠加和缓存绘制，不用于性能计时。</p><div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:18px">'+''.join('<div><p>'+label+'</p><a href="shadow-cpu-'+name+'.png"><img alt="'+label+'" src="shadow-cpu-'+name+'.png"></a></div>'for name,label in [('before','原直接绘制'),('after','新缓存绘制')])+'</div></article>' if (a.output/'shadow-cpu-before.png').exists() else ''
cards=''.join('<article><h2>'+label+'</h2><p><a href="'+html.escape(file)+'">打开作品</a> · <a download href="'+html.escape(file)+'">下载可编辑文件</a></p></article>'for file,label in links)
(a.output/'index.html').write_text('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LayMesh · Open Props 动画优化</title><style>body{font:17px/1.7 system-ui,sans-serif;color:#183244;background:#f5f7fa;max-width:1000px;margin:64px auto;padding:0 24px}h1{font-size:34px}h2{font-size:21px}article{background:white;border:1px solid #d8e2e9;border-radius:12px;padding:8px 24px;margin:16px 0}a{color:#125dc2}code{font-size:12px;overflow-wrap:anywhere}img{max-width:100%}</style><body><p>LayMesh / Open Props</p><h1>动画渲染与实际资源占用优化</h1><p>'+('测试记录已汇总，逐项结论与限制见报告。'if complete else '验证正在进行。以下为当前候选包；未完成项目不标记通过。')+'</p><p>26 页作品、全部源码、字体和素材保留。前后台独立 Worker，后台分块提交，复用内容和工作缓冲。没有新增固定页数或内存限额，没有删减效果或缩短动画。</p>'+cards+'<article><h2>验证记录</h2><p><a href="results.html">性能、资源与验收记录</a> · <a href="summary.json">完整指标 JSON</a> · <a href="per-page.csv">逐页 CSV</a> · <a href="preservation.json">源码与素材哈希核对</a></p>'+archive_link+'</article>'+comparison+'<p>旧 HTML 不会自动获得修复。需要离线使用时，请下载新版文件；本轮没有发布版本。</p><p>优化包 SHA-256：<code>'+preserved['optimizedSha256']+'</code></p></body></html>')
