"""Self-contained comparison gallery, shared by local verification and Pages."""
import base64
import html
from pathlib import Path

NOTES = {
    'hint': '浏览器用有限色标近似中点提示；LayMesh 使用标准幂函数。此例另外通过独立公式验证，不计作与浏览器逐像素一致。',
    'hint-auto': '省略色标位置与中点提示共同参与定位；浏览器的有限色标近似与标准幂函数不同，已另作独立公式验证。',
    'round': '此浏览器使用 round 调整前的尺寸计算百分比位置；LayMesh 按最终平铺尺寸定位。层序与重复周期另作几何检查。',
    'radial-zero-width': '零宽椭圆的标准退化结果为镜像水平渐变；此 Chromium 显示末色。各 LayMesh 输出一致，但与这个浏览器参考不同。',
    'zero-repeat': '重复周期为零。CSS 标准明确规定这个红／白／蓝例子显示平均色 rgb(75%,50%,75%)，即浅紫色；此 Chromium 显示最后的蓝色。这是实质性参考差异。',
    'calc-size': '此 Chromium 拒绝半径中百分比与长度混合的 calc()，CSS.supports=false、background-image=none，白图是页面底色。240×160 下等价半径为 110px、69px；换成这两个数后浏览器与原生最大通道误差为 1/255。',
}


def write_gallery(root: Path, report: dict):
    cards = []
    for row in report['cases']:
        name = row['name']
        dark = name in {f'open-{space}-{number}' for space in ['default', 'srgb'] for number in [8, 16]}
        exception = bool(row.get('referenceDifference'))
        slight = name in ['open-default-18', 'open-srgb-18']
        status = 'exception' if exception or slight else 'dark' if dark else 'match'
        badge = {'exception': '参考差异 · 单独说明', 'dark': '正常暗色配方', 'match': '参考对照通过'}[status]
        note = NOTES.get(name, '')
        if dark:
            note = ('原配方是 #111 → #222 → #111 的黑灰色圆锥渐变，画面本来就很暗；不是空图或导出失败。' if name.endswith('-8') else
                    '原配方中心在 50% -250%，即当前画布上方 400px；画面只截取到接近黑色的末段。OKLab 与 sRGB 暗部亮度不同，各自与参考一致。')
        if slight:
            note = '与浏览器参考有 1 个平滑像素达到 4/255；其余平滑像素不超过 3/255。保留此差异，不计作严格的 3/255 达标。'
        if not note:
            note = '原始配方按对象直接绘制；各端输出及浏览器参考的指标见下方。'
        figures = []
        labels = {'css': '浏览器 CSS 参考', 'native': '原生 Skia', 'canvaskit': 'CanvasKit', 'svg': 'SVG 实际输出', 'pdf': 'PDF · Cairo', 'pptx': 'PPTX · LibreOffice → Cairo'}
        for kind, label in labels.items():
            path = root / f'{name}-{kind}.png'
            if not path.exists():
                continue
            data = base64.b64encode(path.read_bytes()).decode()
            if name == 'calc-size' and kind == 'css':
                label += ' · 原式不被接受'
            figures.append(f'<figure><figcaption>{label}</figcaption><button class="image" aria-label="放大 {html.escape(name)} {label}"><img width="240" height="160" loading="lazy" src="data:image/png;base64,{data}" alt="{html.escape(name)} {label}"></button></figure>')
        if name == 'calc-size' and (root / 'calc-size-resolved-css.png').exists():
            data = base64.b64encode((root / 'calc-size-resolved-css.png').read_bytes()).decode()
            figures.append(f'<figure><figcaption>浏览器等价半径 · 110px / 69px</figcaption><button class="image" aria-label="放大等价半径参考"><img width="240" height="160" loading="lazy" src="data:image/png;base64,{data}" alt="calc-size 等价半径参考"></button></figure>')
        metric = row['comparisons']['native']['css']['smooth']
        cross = [row['comparisons'][k]['native']['smooth']['maxChannel'] for k in ['canvaskit', 'svg', 'pdf', 'pptx'] if k in row['comparisons']]
        computed = row.get('computedCSS', {}).get('background', '')
        group = 'open' if name.startswith('open-') else 'semantic'
        cards.append(f'''<article id="{name}" data-group="{group}" data-status="{status}">
<header><h2>{name}</h2><span class="badge {status}">{badge}</span></header>
<code>{html.escape(row['css'])}</code><p class="note {status}">{note}</p>
<p class="metrics">原生 / 浏览器参考：平滑区域最大 {metric['maxChannel']}/255，超限像素 {metric['pixelsOver3']}；各 LayMesh 输出 / 原生：最大 {max(cross, default=0)}/255。</p>
<div class="figures">{''.join(figures)}</div><details><summary>查看浏览器解析结果与判定条件</summary><code>{html.escape(computed)}</code><p>比较尺寸固定为 240×160，sRGB、96 DPI。平滑区域排除两像素页边与硬边附近两像素；完整边缘指标保存在验证数据中。参考差异单独列出，未作为普通通过项隐藏。</p></details></article>''')
    count = len(report['cases'])
    body = '''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LayMesh 渐变对照画廊</title>
<style>
*{box-sizing:border-box}body{margin:0;background:#f1f5f9;color:#172033;font:15px/1.65 system-ui,sans-serif}main{max-width:1680px;margin:auto;padding:28px}h1{font-size:clamp(25px,4vw,36px);margin:0 0 12px}h2{font-size:21px;margin:0}a{color:#175dad}nav{display:flex;flex-wrap:wrap;gap:20px;margin:15px 0}code{display:block;overflow-wrap:anywhere;font:13px/1.7 ui-monospace,monospace;color:#42526b}article{background:white;border:1px solid #dce4ed;padding:22px;margin:22px 0;border-radius:14px;scroll-margin-top:20px}article header{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap;margin-bottom:10px}.badge{border-radius:6px;padding:3px 10px;font-size:12px;white-space:nowrap}.badge.match{background:#eaf5ef;color:#256145}.badge.exception{background:#fff0d6;color:#805000}.badge.dark{background:#e8edf5;color:#36465d}.note{padding:10px 14px;border-left:3px solid #c5d5e8;background:#f7f9fc}.note.exception{border-color:#c88e24;background:#fff9ee}.metrics{font-size:13px;color:#50617a}.figures{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:20px}figure{margin:0}figcaption{font-size:13px;margin:0 0 6px}.image{padding:0;border:1px solid #d9e1eb;display:block;background:none;cursor:zoom-in;width:100%;max-width:480px}.image img{display:block;width:100%;height:auto}details{margin-top:18px;color:#52617a;font-size:13px}details code{margin-top:10px}.filters{display:flex;flex-wrap:wrap;gap:12px;padding:16px;background:white;border:1px solid #dce4ed;border-radius:12px}input,select{font:inherit;padding:9px;border:1px solid #bcc9d9;border-radius:7px;background:white;color:inherit}input{flex:1;min-width:200px}.intro{max-width:1000px}#count{font-size:13px;color:#52617a}dialog{border:1px solid #bcc9d9;border-radius:12px;max-width:95vw;padding:18px}dialog::backdrop{background:#152033aa}dialog img{display:block;width:720px;max-width:85vw;height:auto}dialog button{display:block;margin:0 0 12px auto;padding:7px 15px;font:inherit}article[hidden]{display:none}@media(max-width:550px){main{padding:14px}article{padding:14px}.figures{grid-template-columns:1fr}}
</style><main><h1>LayMesh 渐变对照画廊</h1>
<p class="intro">原始 Open Props 30 组 × 默认 OKLab / 显式 sRGB，以及通用语义用例。每一行比较同一输入的浏览器 CSS、原生、CanvasKit 和实际导出画面。点击图片可放大。</p>
<p class="intro"><strong>两种指标分开看：</strong>“各端 / 原生”检查 LayMesh 输出一致性；“原生 / 浏览器参考”检查与 Chromium 的差异。暗色配方、标准退化行为及浏览器不接受的表达式均有单独说明，不能仅凭全黑、全白判断导出失败。</p>
<nav><a href="https://muxkin.github.io/LayMesh/docs/topics/gradients.zh-CN.html">中文手册</a><a class="lang-switch" href="https://muxkin.github.io/LayMesh/en/docs/topics/gradients.html">English guide</a><a href="verification.json">完整像素指标 JSON</a><a href="https://muxkin.github.io/LayMesh/docs/benchmarks/gradients-2026-10-19/README.zh-CN.html">验证报告</a></nav>
<div class="filters"><input id="search" type="search" aria-label="搜索用例" placeholder="搜索编号、CSS 或说明，例如 16、calc-size"><select id="filter" aria-label="筛选用例"><option value="all">全部用例</option><option value="open">Open Props 原始配方</option><option value="semantic">通用语义</option><option value="exception">浏览器参考差异</option><option value="dark">正常暗色配方</option></select></div><p id="count" aria-live="polite"></p>
'''+''.join(cards)+'''</main><dialog id="zoom"><button type="button" id="close">关闭</button><img alt="放大对照图"></dialog><script>
const search=document.querySelector('#search'),filter=document.querySelector('#filter'),cards=[...document.querySelectorAll('article')];
function update(){const q=search.value.toLowerCase(),f=filter.value;let n=0;for(const a of cards){a.hidden=!(a.textContent.toLowerCase().includes(q)&&(f==='all'||a.dataset.group===f||a.dataset.status===f));if(!a.hidden)n++;}document.querySelector('#count').textContent=`显示 ${n} / ${cards.length} 例 · Chromium 151.0.7922.34 · 240×160 · sRGB · 96 DPI`;}
search.addEventListener('input',update);filter.addEventListener('change',update);update();
const dialog=document.querySelector('#zoom');document.querySelector('#close').onclick=()=>dialog.close();for(const button of document.querySelectorAll('.image'))button.onclick=()=>{const source=button.querySelector('img'),target=dialog.querySelector('img');target.src=source.src;target.alt=source.alt;dialog.showModal();};
</script></html>'''
    assert body.count('<article ') == count
    (root / 'gallery.html').write_text(body)
