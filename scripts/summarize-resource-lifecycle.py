#!/usr/bin/env python3
"""Summarize trusted-key navigation separately from authored transitions."""
import argparse
import json
import math
import statistics
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('directory', type=Path)
a = p.parse_args()
hosts = {'chromium': 'Chromium', 'firefox': 'Firefox', 'webkit': 'WebKit', 'vscode': 'VS Code'}

def distribution(values):
    assert values and all(isinstance(v, (int, float)) and math.isfinite(v) for v in values)
    return {'median': statistics.median(values), 'p95': sorted(values)[math.ceil(len(values) * .95) - 1]}

results = []
for host in hosts:
    for version in ['before', 'after']:
        source = a.directory / ('native-keyboard/' + version + '.json' if host == 'vscode' else f'keyboard-{version}/{host}.json')
        record = json.loads(source.read_text())
        rows = record['rows']
        assert len(rows) == 50 and all(r['trusted'] for r in rows)
        # Calls completing during a navigation can include a previous page's
        # paint or background preparation. Report the single activation's queue
        # wait instead of adding overlapping calls and treating the sum as one
        # interval on the navigation critical path. Preserve raw records.
        for row in rows:
            activate = [c for c in row['calls'] if c['type'] == 'activate']
            assert len(activate) == 1
            row['activationQueueMs'] = activate[0].get('queueWaitMs', 0)
        summary = {}
        for kind, chosen in [('all', rows), ('firstTraversal', [r for r in rows if r['round'] == 0]),
                             ('repeat', [r for r in rows if r['round'] > 0]), ('prepared', [r for r in rows if r['ready']])]:
            summary[kind] = {'samples': len(chosen)}
            if chosen:
                summary[kind].update({key: distribution([r[key] for r in chosen])
                                     for key in ['firstVisibleMs', 'completedMs', 'transitionMs', 'activationQueueMs']})
        results.append({'host': host, 'version': version, 'source': str(source.relative_to(a.directory)),
                        'artifactSha256': record.get('htmlSha256', record.get('nativeSha256')),
                        'wasmSha256': record.get('wasmSha256'),
                        'backend': record.get('backend'), 'dimensions': record.get('dimensions'),
                        'transitionRasterSize': record.get('memory', {}).get('transitionRasterSize'),
                        'device': record.get('memory', {}).get('device'), 'usesWorker': record.get('usesWorker'),
                        'usesMain': record.get('usesMain'), 'rendererFallback': record.get('rendererFallback'), 'openMs': record['openedMs'],
                        'trustedInputs': sum(r['trusted'] for r in record['inputs']),
                        'inputsWhileSwitching': sum(r['switching'] for r in record['inputs']),
                        'residentPagesAtEnd': len(record['resident']),
                        'frameCacheHits': sum(r['frameCacheHit'] for r in rows),
                        'preparedTargetPassed': (summary['prepared']['firstVisibleMs']['median'] <= 100 and
                                                 summary['prepared']['firstVisibleMs']['p95'] <= 200)
                                                if summary['prepared']['samples'] else None,
                        'destinations': [{'page': page + 1, 'samples': len(chosen),
                                          **{key: distribution([r[key] for r in chosen])
                                             for key in ['firstVisibleMs', 'transitionMs', 'activationQueueMs']}}
                                         for page in sorted({r['to'] for r in rows})
                                         if (chosen := [r for r in rows if r['to'] == page])],
                        'queueMeasured': version == 'after', 'summary': summary})

(a.directory / 'navigation-summary.json').write_text(json.dumps(results, indent=2) + '\n')
pair = lambda v: f"{v['median']:.1f} / {v['p95']:.1f}"
lines = ['### 连续切页', '', '单位 ms，均为中位数 / P95。每个版本每个环境 50 次实际页面切换；首次遍历仅含其中第一次经过各页的样本。', '',
         '| 环境 | 版本 | 全部：开始变化 | 首遍：开始变化 | 重复：开始变化 | 已准备：开始变化 | 切页完成 | 转场执行 | 激活请求排队 |',
         '|---|---|---:|---:|---:|---:|---:|---:|---:|']
for row in results:
    s = row['summary']
    cells = [hosts[row['host']], 'v004 对照' if row['version'] == 'before' else 'v005']
    cells += [pair(s[k]['firstVisibleMs']) if s[k]['samples'] else '—' for k in ['all', 'firstTraversal', 'repeat', 'prepared']]
    cells += [pair(s['all'][k]) for k in ['completedMs', 'transitionMs']]
    cells += [pair(s['all']['activationQueueMs']) if row['queueMeasured'] else '未埋点']
    lines.append('| ' + ' | '.join(cells) + ' |')
lines += ['', '首次遍历每组 5 次，重复遍历 45 次，合计 50 次；首次遍历的 P95 因样本少只能作描述。v004 尚无独立入口画面就绪标记，故“已准备”列不构造对应样本；队列耗时也未单独埋点。', '',
          '| 环境 | 版本 | 打开放映至首屏 ms（单次） | 测得切页 | 已准备样本 | 入口画面命中 | 结束时驻留页数 | 转场期间按键数 |',
          '|---|---|---:|---:|---:|---:|---:|---:|']
for row in results:
    lines.append('| ' + ' | '.join([hosts[row['host']], 'v004' if row['version'] == 'before' else 'v005', f"{row['openMs']:.1f}", '50',
                                   str(row['summary']['prepared']['samples']), str(row['frameCacheHits']),
                                   str(row['residentPagesAtEnd']), str(row['inputsWhileSwitching'])]) + ' |')
lines += ['', '启动是每次测试会话的一次观测，不足以计算分位数；没有为提高切页成绩新增整份预热等待。', '']
lines += ['| 环境 | 版本 | 后端 | Worker | 画布尺寸 | 转场实际像素 |', '|---|---|---|---|---|---|']
for row in results:
    dimensions = row['dimensions']
    raster = row['transitionRasterSize']
    lines.append('| ' + ' | '.join([hosts[row['host']], 'v004' if row['version'] == 'before' else 'v005',
                                   str(row['backend']), '是' if row['usesWorker'] else '否',
                                   f"{dimensions['width']} × {dimensions['height']}",
                                   ' × '.join(map(str, raster)) if raster else '未记录']) + ' |')
lines += ['']
(a.directory / 'navigation-measurements.md').write_text('\n'.join(lines))
print(json.dumps({'environments': len(hosts), 'acceptedPageChanges': 400, 'output': str(a.directory / 'navigation-summary.json')}))
