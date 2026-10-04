"""Run from the repository root: PYTHONPATH=python python examples/plot/complete-data.py.
Python supplies observations and grids; all coordinates and decoration placement stay editable in .lay.
"""
from pathlib import Path
import numpy as np
from laymesh import render_source

HERE = Path(__file__).resolve().parent
rng = np.random.default_rng(42)
observations = rng.normal(2, 0.7, 400)
weights = np.ones_like(observations)
x = np.linspace(-2, 2, 41)
y = np.linspace(-2, 2, 31)
z = np.sin(x[None, :]) * np.cos(y[:, None])
# Missing samples remain transparent; no interpolation or imputation is requested.
z[5:8, 6:9] = np.nan
source = 'page=canvas(size=(190 mm,105 mm),background="#ffffff")\ns=plot_style(font_family="DejaVu Sans",font_size=8 pt)\np=plot(size=(85 mm,90 mm),plot_area=box(offset=(16 mm, 15 mm), size=(58 mm, 52 mm)),style=s,\n x=axis(label="Observation",range=(0,4)),y=axis(label="Density",range=(0,0.8)))\na=p.hist(values={{observations}},weights={{weights}},bins=[0,0.5,1,1.5,2,2.5,3,3.5,4],stat="density",fill="#dce6ee",border_color="#264b69",label="Weighted sample")\nchart=page.add(p,offset=(4 mm,3 mm))\npage.add(legend(layers=[a],background="none"),target=chart.plot_bottom_left,offset=(0 mm,21 mm))\ncolors=color_scale(vmin=-1,vmax=1,cmap="rdbu")\nq=plot(size=(96 mm,90 mm),plot_area=box(offset=(16 mm, 15 mm), size=(58 mm, 52 mm)),style=s,\n x=axis(label="x",range=(-2,2)),y=axis(label="y",range=(-2,2)))\nq.contourf(z={{z}},x={{x}},y={{y}},levels=[-1,-0.75,-0.5,-0.25,0,0.25,0.5,0.75],color_scale=colors)\nchart2=page.add(q,offset=(90 mm,3 mm))\npage.add(colorbar(scale=colors,orientation="vertical",length=52 mm,ticks=[-1,0,1],label="Value",style=s),target=chart2.plot_top_right,offset=(4 mm,0 mm))\n'
result = render_source(source, namespace=locals(), base_dir=HERE,
                       output=HERE / "complete-data.pdf", save_source="complete-data.lay")
print(result.saved_source)
