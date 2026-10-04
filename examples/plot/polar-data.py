"""Portable native polar fields and raw-unit radar data; no Matplotlib dependency."""
from pathlib import Path
import numpy as np
from laymesh import render_source

HERE = Path(__file__).resolve().parent
theta = np.linspace(0, 360, 32, endpoint=False)
radius = np.linspace(0, 3, 25)
field = radius[:, None] * (1 + 0.22 * np.sin(np.deg2rad(theta))[None, :])
# Keep a measured region absent; its neighboring cells must remain transparent.
field[12:15, 12:15] = np.nan
values = np.array([800, 6, 40, 70, 80])
source = '# BEGIN DEMO\npage=canvas(size=(228 mm,132 mm),background="#ffffff")\ns=plot_style(font_family="DejaVu Sans",font_size=7 pt)\ncolors=color_scale(vmin=0,vmax=4,cmap="viridis")\np=plot(projection="polar",size=(105 mm,110 mm),plot_area=box(offset=(20 mm, 22 mm), size=(66 mm, 66 mm)),style=s,\n       theta=axis(ticks=[0,90,180,270]),r=axis(range=(0,3),ticks=[1,2,3],tick_color="#ffffff",tick_font_size=6 pt))\np.contourf(z={{field}},theta={{theta}},r={{radius}},levels=[0,0.5,1,1.5,2,2.5,3,3.5],periodic=true,color_scale=colors)\na=page.add(p,offset=(3 mm,0 mm))\npage.add(text(content="(a) Missing polar samples",font_family="DejaVu Sans",font_size=9 pt),offset=(13 mm,5 mm))\nq=plot(projection="radar",size=(118 mm,110 mm),plot_area=box(offset=(25 mm, 22 mm), size=(66 mm, 66 mm)),style=s,\n       categories=["Strength","Density","Cost","Temperature","Life"],ranges=[(0,1000),(0,10),(0,100),(-50,150),(0,100)],r=axis(ticks=[0.5,1],grid="major",tick_font_size=5 pt))\nq.area(values={{values}},fill="#264b69",opacity=0.2)\nq.line(values={{values}},color="#264b69",marker="diamond",marker_size=1.6 mm)\nb=page.add(q,offset=(109 mm,0 mm))\npage.add(text(content="(b) Raw units from Python",font_family="DejaVu Sans",font_size=9 pt),offset=(126 mm,5 mm))\npage.add(colorbar(scale=colors,orientation="horizontal",length=72 mm,thickness=3 mm,ticks=[0,1,2,3,4],label="Field",style=s),offset=(27 mm,107 mm))\n# END DEMO\n'
if __name__ == '__main__':
    result = render_source(source, namespace=globals(), base_dir=HERE,
                           output=HERE / 'polar-data.pdf', save_source='polar-data.lay',
                           show_warnings=False)
    (HERE / 'polar-data.svg').write_text(result.preview_svg, encoding='utf-8')
    print(result.saved_source)
