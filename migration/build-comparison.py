#!/usr/bin/env python3
"""Build a vector-first comparison with immutable input/output provenance."""
import hashlib
import html
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ['basic.lay', 'typography.lay', 'outlines.lay', 'plot/multi-axes-breaks.lay',
           'plot/statistics.lay', 'plot/polar-field.lay']
NOTES = {
    'basic.lay': '已修复回归：实例身份、单位和样式适配另有契约测试。',
    'typography.lay': '已修复回归：RaTeX 横线中心坐标。允许的库差异：KaTeX 字形、字面大小和笔画粗细；没有人为放大公式。SVG/PDF 保持矢量，缩略图的抗锯齿不能用于判定分辨率。',
    'outlines.lay': '已修复回归：路径不再额外绘制矩形边框，融合默认不增加黑色描边。允许的库差异：真实圆弧的圆帽边界，不复刻旧 PathKit 的近似曲线外凸。',
    'plot/multi-axes-breaks.lay': '已修复回归：轴文字颜色继承、图例 marker、组合样式和默认间距。',
    'plot/statistics.lay': '已修复回归：图例纹理、轴文字颜色、阶跃方向及统计数据规则。',
    'plot/polar-field.lay': '已修复回归：使用 contour 0.13.1 的 D3 系算法，保留非均匀网格、周期接缝、孔洞和不重叠填色色带。',
}

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    destination = ROOT / 'release/comparison'
    destination.mkdir(parents=True, exist_ok=True)
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    provenance = {'baseline_commit': '78db22d', 'rust_commit': revision, 'dpi': 144, 'examples': []}
    content = ['''<!doctype html><html lang="zh-CN"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>LayMesh 语义修复对照</title><style>
body{font:16px/1.6 system-ui,sans-serif;margin:0;background:#f3f5f8;color:#172536}
main{max-width:1600px;margin:auto;padding:32px}h1{font-size:30px}section{margin:30px 0;background:white;padding:20px;border-radius:12px}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:16px}figure{margin:0}img{display:block;width:100%;height:auto;border:1px solid #dde3eb;box-sizing:border-box}
figcaption{font-weight:600;margin:0 0 8px}p{max-width:1250px}a{color:#195fa5}code{overflow-wrap:anywhere;font-size:.85em}details{margin:12px 0}
@media(max-width:800px){.pair{grid-template-columns:1fr}main{padding:16px}}</style>
<main><h1>LayMesh：Rust + RaTeX 语义修复对照</h1>
<p>页面优先显示 SVG；打开原文件可放大检查。两侧输入逐字节核对，源码和输出 SHA-256 见每项记录。
正文使用本机或用户字体，应用包仅内置公式字体。PNG 为 144 DPI，PDF 保留正文和公式 LaTeX 提取。</p>''',
        f'<p>Rust 构建提交：<code>{revision}</code>。<a href="provenance.json">完整来源记录</a></p>',
        '<p>验证范围和未验证平台见仓库 migration/README.md。旧版失败产物保留在每项的折叠区。</p>']
    for source in SOURCES:
        name = source.replace('/', '-').removesuffix('.lay')
        rel = 'examples/' + source
        current = (ROOT / rel).read_bytes()
        baseline = subprocess.check_output(['git', 'show', '78db22d:' + rel], cwd=ROOT)
        assert current == baseline, f'Comparison inputs changed: {rel}'
        record = {'source': rel, 'same_input_bytes': True, 'sha256': digest(current), 'outputs': {}}
        (destination / 'sources').mkdir(exist_ok=True)
        (destination / 'sources' / (name + '.lay')).write_bytes(current)
        content.append(f'<section><h2>{html.escape(name)}</h2><p>{html.escape(NOTES[source])}</p>')
        content.append(f'<p>相同输入：<a href="sources/{name}.lay">{html.escape(rel)}</a> · SHA-256 <code>{record["sha256"]}</code></p><div class="pair">')
        for backend, label in [('node', 'Node 基线'), ('rust', 'Rust + RaTeX 修复版')]:
            for suffix in ('svg', 'pdf', 'png'):
                path = destination / backend / f'{name}.{suffix}'
                record['outputs'][f'{backend}/{path.name}'] = {'bytes': path.stat().st_size, 'sha256': digest(path.read_bytes())}
            content.append(f'<figure><figcaption>{label} · <a href="{backend}/{name}.svg">SVG</a> · <a href="{backend}/{name}.pdf">PDF</a> · <a href="{backend}/{name}.png">PNG</a></figcaption><img loading="lazy" src="{backend}/{name}.svg" alt="{html.escape(label + " " + name)}"></figure>')
        content.append('</div>')
        old = ROOT / 'release/history/df2d5b1/comparison/rust'
        if (old / f'{name}.svg').is_file():
            (destination / 'before-fix').mkdir(exist_ok=True)
            for suffix in ('svg', 'png', 'pdf'):
                shutil.copy2(old / f'{name}.{suffix}', destination / 'before-fix' / f'{name}.{suffix}')
            content.append(f'<details><summary>查看修复前 Rust 失败产物（df2d5b1）</summary><a href="before-fix/{name}.svg">原 SVG</a> · <a href="before-fix/{name}.pdf">原 PDF</a><img loading="lazy" src="before-fix/{name}.svg" alt="修复前 {name}"></details>')
        content.append('</section>')
        provenance['examples'].append(record)
    content.append('</main></html>')
    (destination / 'index.html').write_text('\n'.join(content))
    (destination / 'provenance.json').write_text(json.dumps(provenance, ensure_ascii=False, indent=2) + '\n')
    web = ROOT / 'site/dist/migration-comparison'
    web.mkdir(parents=True, exist_ok=True)
    for name in ('index.html', 'provenance.json'):
        shutil.copy2(destination / name, web / name)
    for name in ('node', 'rust', 'before-fix', 'sources'):
        if (destination / name).is_dir():
            shutil.copytree(destination / name, web / name, dirs_exist_ok=True)
    print(destination / 'index.html')

if __name__ == '__main__':
    main()
