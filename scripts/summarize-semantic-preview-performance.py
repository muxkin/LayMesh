#!/usr/bin/env python3
"""Validate and summarize painted preview benchmarks; never include smoke runs."""
import argparse
import json
import math
import statistics
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--input', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=True)


def read(relative):
    return json.loads((a.input / relative).read_text())


def save(relative, value):
    destination = a.output / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    value['summaryEstimator'] = 'Median: mean of middle two; P95: nearest rank; recomputed from raw samples'
    destination.write_text(json.dumps(value, indent=2) + '\n')


def distribution(values):
    values = sorted(values)
    assert values and all(math.isfinite(v) and v >= 0 for v in values)
    return dict(median=statistics.median(values), p95=values[math.ceil(len(values) * .95) - 1])


def document_summaries(rows, host):
    directory = 'directoryReceivedMs' if host == 'VS Code' else 'directoryMs'
    summaries = []
    for kind in ['lcss-heading', 'lcss-curves', 'laydoc-name', 'laydoc-reorder', 'laydoc-add', 'laydoc-remove']:
        if kind in ['laydoc-add', 'laydoc-remove']:
            operation = kind.removeprefix('laydoc-')
            def matches(row):
                pages = row['directory']['pages'] if host == 'VS Code' else row['ids']
                actual = row.get('operation', 'add' if len(pages) > 6 else 'remove')
                return row['kind'] == 'laydoc-add-remove' and actual == operation
            selected = [r for r in rows if matches(r)]
        else:
            selected = [r for r in rows if r['kind'] == kind]
        assert len(selected) == 50, (host, kind, len(selected))
        summaries.append(dict(host=host, kind=kind, samples=len(selected),
                              **{key: distribution([r[key] for r in selected])
                                 for key in [directory, 'currentMs', 'documentMs']}))
    return summaries


timings, documents, memory, hashes, phases = [], [], [], set(), []
for engine in ['chromium', 'firefox', 'webkit']:
    result = read(f'browser/{engine}.json')
    assert len(result['summaries']) == 8 and len(result['rows']) == 400
    assert not result['errors'] and not result['network']
    assert all(r['presentationConfirmed'] for r in result['rows'])
    hashes.add(result['sha256'])
    if '; software GPU;' in result['definition']:
        result['definition'] = result['definition'].replace(
            '; software GPU;', '; headless browser (automatically selected renderer);')
        result['definitionCorrection'] = 'Original harness labeled software GPU; per-sample renderer was not captured. Numeric records are unchanged.'
    for r in result['summaries']:
        values = [x['elapsedMs'] for x in result['rows'] if x['page'] == r['page'] and x['kind'] == r['kind']]
        assert len(values) == 50
        d = distribution(values)
        r.update(medianMs=d['median'], p95Ms=d['p95'], targetPassed=d['median'] <= 100 and d['p95'] <= 200)
    timings.extend(dict(r, host=engine) for r in result['summaries'])
    for page in ['02-seed', '06-editable']:
        selected = [r for r in result['rows'] if r['page'] == page and r['kind'] == 'color']
        phases.append(dict(host=engine, page=page, samples=len(selected),
                           **{key: distribution([r[key] for r in selected])
                              for key in ['elapsedMs', 'compileMs', 'prepareMs', 'rasterMs']}))
    retained = [r['wasmRetainedBytes'] for r in result['rows']]
    memory.append(dict(
        host=engine, peakWasmRetainedBytes=max(retained),
        peakDerivationCacheBytes=max(r['metrics']['cacheBytes'] for r in result['rows']),
        last100CapacityConstant=len(set(retained[-100:])) == 1,
    ))
    save(f'browser/{engine}.json', result)
    result = read(f'browser-document/{engine}.json')
    # A previous summary pass already split add/remove into separate rows.
    assert not result['smoke'] and len(result['summaries']) in (5, 6)
    assert not result['errors'] and not result['network']
    if len(result['rows']) == 250:
        # Replace the initial mixed 25-add/25-remove run with a separate
        # 50-add/50-remove run. Preserve every accepted raw sample unchanged.
        extra = read(f'browser-document-add-remove/{engine}.json')
        assert not extra['smoke'] and len(extra['rows']) == 100
        assert not extra['errors'] and not extra['network']
        assert extra['version'] == result['version'] and extra['htmlSha256'] == result['htmlSha256']
        assert all(r['kind'] == 'laydoc-add-remove' for r in extra['rows'])
        result['rows'] = [r for r in result['rows'] if r['kind'] != 'laydoc-add-remove'] + extra['rows']
        result['supplementalSource'] = f'browser-document-add-remove/{engine}.json'
        result['supersededMixedSamples'] = 50
    assert len(result['rows']) == 300
    assert all(r['presentationConfirmed'] for r in result['rows'])
    hashes.add(result['htmlSha256'])
    result['summaries'] = document_summaries(result['rows'], engine)
    documents.extend(result['summaries'])
    save(f'browser-document/{engine}.json', result)

native_hashes = set()
for page in ['02-seed', '06-editable']:
    for kind in ['color', 'position', 'text', 'size']:
        result = read(f'vscode/{page}-{kind}.json')
        assert len(result['hot']) == 50
        assert all(r['presentation_confirmed'] and r['incremental'] for r in result['hot'])
        assert result['periodic_cdp_diagnostics'] is False
        native_hashes.add(result['artifact_sha256']['native'])
        timing = result['edit_to_visible']
        d = distribution([r['edit_to_visible_ms'] for r in result['hot']])
        timing.update(median_ms=d['median'], p95_ms=d['p95'])
        result['hot_median_ms'] = statistics.median(r['elapsed_ms'] for r in result['hot'])
        timings.append(dict(host='VS Code', page=page, kind=kind, samples=timing['samples'],
                            medianMs=timing['median_ms'], p95Ms=timing['p95_ms'],
                            targetPassed=timing['median_ms'] <= 100 and timing['p95_ms'] <= 200))
        save(f'vscode/{page}-{kind}.json', result)
native_document_rows = []
for kind in ['lcss-heading', 'lcss-curves', 'laydoc-name', 'laydoc-reorder', 'laydoc-add-remove']:
    result = read(f'vscode-document/{kind}.json')
    assert not result['smoke'] and len(result['rows']) == (100 if kind == 'laydoc-add-remove' else 50)
    assert result['periodic_cdp_diagnostics'] is False
    assert all(r['presentationConfirmed'] for r in result['rows'])
    native_hashes.add(result['nativeSha256'])
    for key in ['directoryReceivedMs', 'currentMs', 'documentMs']:
        result['summary'][key] = distribution([r[key] for r in result['rows']])
    native_document_rows.extend(dict(r, kind=kind) for r in result['rows'])
    save(f'vscode-document/{kind}.json', result)
documents.extend(document_summaries(native_document_rows, 'VS Code'))

assert len(hashes) == len(native_hashes) == 1, 'Mixed build revisions in measurements'
assert all(r['samples'] == 50 for r in timings + documents)
assert all(math.isfinite(r[k]) and r[k] > 0 for r in timings for k in ['medianMs', 'p95Ms'])
for r in documents:
    directory = 'directoryReceivedMs' if r['host'] == 'VS Code' else 'directoryMs'
    assert all(math.isfinite(r[k][q]) and r[k][q] >= 0
               for k in [directory, 'currentMs', 'documentMs'] for q in ['median', 'p95'])
summary = dict(
    method='50 measured edits per operation after warmup; confirmed presentation; sequential runs; median is mean of middle two; P95 is nearest rank',
    htmlSha256=next(iter(hashes)), nativeSha256=next(iter(native_hashes)),
    totalSamples=sum(r['samples'] for r in timings + documents),
    localEditSamples=sum(r['samples'] for r in timings),
    documentEditSamples=sum(r['samples'] for r in documents),
    passedLocalCases=sum(r['targetPassed'] for r in timings), totalLocalCases=len(timings),
    timings=timings, documents=documents, memory=memory, browserColorPhases=phases,
    documentEndpoints={
        'browser': 'directory DOM update / current page presentation / all page compilation complete',
        'VS Code': 'directory received by host / current page presentation / deck and resources acknowledged',
    },
)
(a.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')

# Keep the report tables reproducible from the same checked raw records.
hosts = {'chromium': 'Chromium', 'firefox': 'Firefox', 'webkit': 'WebKit', 'VS Code': 'VS Code'}
kinds = {'color': '颜色', 'position': '位置', 'text': '文字', 'size': '尺寸'}
document_kinds = {'lcss-heading': 'LCSS 标题样式', 'lcss-curves': 'LCSS 全部曲线样式',
                  'laydoc-name': 'laydoc 改名', 'laydoc-reorder': 'laydoc 重排',
                  'laydoc-add': 'laydoc 新增页面', 'laydoc-remove': 'laydoc 删除页面'}
pair = lambda median, p95: f'{median:.1f} / {p95:.1f}'
lines = [
    f"共 {summary['totalSamples']:,} 次正式编辑样本；每个场景 50 次，已排除预热。新增与删除分别统计，各 50 次。中位数取中间两值的平均，P95 使用最近秩。",
    '', '### 局部 `.lay` 修改', '',
    '单位为毫秒，表中数值为 **中位数 / P95**。目标为中位数 ≤100 毫秒且 P95 ≤200 毫秒。', '',
    '| 环境 | 页面 | 颜色 | 位置 | 文字 | 尺寸 |',
    '|---|---|---:|---:|---:|---:|',
]
for host, label in hosts.items():
    for page, page_label in [('02-seed', '第 2 页'), ('06-editable', '第 6 页，1,536 条曲线')]:
        selected = {r['kind']: r for r in timings if r['host'] == host and r['page'] == page}
        values = [pair(selected[k]['medianMs'], selected[k]['p95Ms']) for k in kinds]
        lines.append('| ' + ' | '.join([label, page_label, *values]) + ' |')
lines += ['', f"{summary['totalLocalCases']} 个场景中，{summary['passedLocalCases']} 个同时达到两项延迟目标。", '',
          '### LCSS 与 laydoc 修改', '',
          '单位为毫秒，均为 **中位数 / P95**。当前页固定为第 6 页。浏览器目录终点是 DOM 更新，VS Code 目录终点是宿主收到新目录；整份同步分别是全部页面编译完成、整份放映及资源确认。', '',
          '| 环境 | 修改 | 当前页显示 | 目录更新或接收 | 整份同步 |',
          '|---|---|---:|---:|---:|']
for host, label in hosts.items():
    for kind, kind_label in document_kinds.items():
        r = next(r for r in documents if r['host'] == host and r['kind'] == kind)
        directory = 'directoryReceivedMs' if host == 'VS Code' else 'directoryMs'
        values = [pair(r[k]['median'], r[k]['p95']) for k in ['currentMs', directory, 'documentMs']]
        lines.append('| ' + ' | '.join([label, kind_label, *values]) + ' |')
lines += ['', '### 浏览器内存记录', '',
          '这些是连续 400 次局部编辑中的峰值。存活派生缓存估计值与编译器 WASM 保留容量含义不同，均不是浏览器进程总内存。', '',
          '| 环境 | 存活派生缓存峰值 MiB | 编译器 WASM 保留容量峰值 MiB | 最后 100 次容量稳定 |',
          '|---|---:|---:|---|']
for r in memory:
    lines.append(f"| {hosts[r['host']]} | {r['peakDerivationCacheBytes'] / 2**20:.1f} | "
                 f"{r['peakWasmRetainedBytes'] / 2**20:.1f} | {'是' if r['last100CapacityConstant'] else '否'} |")
lines += ['', '原始样本与汇总见本报告同名数据目录中的 `summary.json`、`browser`、`browser-document`、`vscode` 和 `vscode-document`。', '']
lines += ['### 浏览器改色阶段计时', '',
          '单位 ms，均为中位数 / P95，每行 50 次。准备与显示包含绘制、GPU 完成及画面确认，绘制列是它的子集，不能将所有列直接相加；输入合并窗口计入端到端时间。', '',
          '| 环境 | 页面 | 端到端 | 增量编译 | 准备与显示（含绘制） | 其中：绘制 |',
          '|---|---|---:|---:|---:|---:|']
for r in phases:
    lines.append('| ' + ' | '.join([hosts[r['host']], '第 2 页' if r['page'] == '02-seed' else '第 6 页',
                                   *[pair(r[k]['median'], r[k]['p95']) for k in ['elapsedMs', 'compileMs', 'prepareMs', 'rasterMs']]]) + ' |')
lines += ['']
(a.output / 'measurements.md').write_text('\n'.join(lines))
print(json.dumps({k: v for k, v in summary.items() if k not in ['timings', 'documents', 'memory']}, indent=2))
