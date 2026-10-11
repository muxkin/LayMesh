// Deterministic workloads compiled by the production LAY/WASM pipeline.
// Counts are input sizes; the worker separately reports the lowered scene size.
export const stressCases = [
 {id:'01-baseline',label:'01 · 基础排版',patterns:0,charts:0,formulas:0,effects:false,motion:'title'},
 {id:'02-patterns-64',label:'02 · 64 组矢量图案',patterns:64,charts:0,formulas:0,effects:false,motion:'title'},
 {id:'03-patterns-256',label:'03 · 256 组矢量图案',patterns:256,charts:0,formulas:0,effects:false,motion:'title'},
 {id:'04-charts-6',label:'04 · 加入 6 张多序列图表',patterns:256,charts:6,formulas:0,effects:false,motion:'title'},
 {id:'05-charts-12',label:'05 · 增至 12 张图表',patterns:256,charts:12,formulas:0,effects:false,motion:'title'},
 {id:'06-formulas',label:'06 · 加入 24 个公式与嵌套布局',patterns:256,charts:12,formulas:24,effects:false,motion:'title'},
 {id:'07-effects',label:'07 · 加入透明、阴影与光晕',patterns:256,charts:12,formulas:24,effects:true,motion:'title'},
 {id:'08-motion-2d',label:'08 · 图表与公式同时做二维动画',patterns:256,charts:12,formulas:24,effects:true,motion:'2d'},
 {id:'09-motion-3d',label:'09 · 加入透视翻转与描线动画',patterns:256,charts:12,formulas:24,effects:true,motion:'3d'},
 {id:'10-dense',label:'10 · 24 张图、48 个公式、512 组图案',patterns:512,charts:24,formulas:48,effects:true,motion:'3d',dense:true},
];

const colors=['#2975b9','#d88735','#318b7d','#8867b1'];
const equations=[
 String.raw`\int_0^\infty e^{-x^2}\,dx=\frac{\sqrt{\pi}}{2}`,
 String.raw`E=\sum_{i=1}^{N}\frac{1}{2}m_i v_i^2`,
 String.raw`\frac{\partial u}{\partial t}=\alpha\nabla^2 u`,
 String.raw`\mathbf{A}=\begin{pmatrix}a&b\\c&d\end{pmatrix}`,
 String.raw`f(x)=\frac{1}{\sigma\sqrt{2\pi}}e^{-\frac{(x-\mu)^2}{2\sigma^2}}`,
 String.raw`\hat{f}(\omega)=\int_{-\infty}^{\infty}f(t)e^{-i\omega t}\,dt`,
 String.raw`\Delta G=\Delta H-T\Delta S`,
 String.raw`\lim_{n\to\infty}\left(1+\frac{1}{n}\right)^n=e`,
];
const motion=String.raw`
@keyframes title-motion {0%,100%{transform:translateX(0px)}50%{transform:translateX(24px)}}
@keyframes panel-motion {0%,100%{transform:translateY(0px) rotate(-1deg);opacity:.84}50%{transform:translateY(-5px) rotate(1deg);opacity:1}}
@keyframes panel-flip {0%,100%{transform:perspective(900px) rotateY(-32deg) rotateX(8deg)}50%{transform:perspective(900px) rotateY(32deg) rotateX(-8deg)}}
@keyframes motif-motion {0%,100%{transform:rotate(-20deg)}50%{transform:rotate(20deg)}}
`;
const n=value=>Number(value.toFixed(5));
const numberList=values=>'['+values.map(n).join(',')+']';

export function stressFiles(spec) {
 const targets=[],motifs=[];
 const lines=[`page=canvas(size=(320mm,180mm),stylesheet="motion.lcss",background="#f4f6fa")`];
 // Every motif contains a gradient, an ellipse and crossed strokes. They remain
 // visible through translucent cards instead of being hidden under an opaque page.
 const cols=spec.patterns>256?32:spec.patterns>64?32:16,rows=Math.ceil(spec.patterns/cols);
 for(let i=0;i<spec.patterns;i++){
  const x=5+(i%cols)*310/cols,y=25+Math.floor(i/cols)*145/rows,w=310/cols-1,h=145/rows-1;
  lines.push(`g=group()`,
   `g.add(rect(size=(${n(w)}mm,${n(h)}mm),border_color="none",border_radius=1mm,fill=linear_gradient(start=(0,0),end=(1,1),stops=[(0,"#edf1f7"),(1,"#d4e2ef")])))`,
   `g.add(ellipse(size=(${n(w*.5)}mm,${n(h*.5)}mm),fill="none",border_color="${colors[i%4]}",border_width=0.15mm),offset=(${n(w*.25)}mm,${n(h*.25)}mm))`,
   `g.add(line(length=${n(w*.8)}mm,line_color="#a8bdd2",line_width=0.12mm),offset=(${n(w*.1)}mm,${n(h*.5)}mm),rotation=18deg)`,
   `motif_${i}=page.add(g,offset=(${n(x)}mm,${n(y)}mm),opacity=0.6)`);
  motifs.push(`motif_${i}`);
 }
 lines.push(`title=page.add(text("Retained scene / ${spec.id}",font_family="DejaVu Sans",font_size=16pt,color="#20354b"),offset=(7mm,5mm))`,
  `page.add(text("${spec.patterns} patterns | ${spec.charts} charts | ${spec.formulas} equations | ${spec.motion}",font_family="DejaVu Sans",font_size=8pt,color="#526b82"),offset=(7mm,16mm))`);
 if(!spec.charts)lines.push(`page.add(text("LAYMESH / SKIA",font_family="DejaVu Sans",font_size=36pt,color="#20354b"),anchor=center,offset=(160mm,98mm))`);
 const chartCols=spec.charts>12?6:spec.charts>6?4:3,chartRows=Math.ceil(spec.charts/chartCols),chartH=spec.formulas?90:140;
 const cw=304/chartCols-3,ch=chartRows?chartH/chartRows-3:0,samples=spec.dense?384:192;
 for(let i=0;i<spec.charts;i++){
  const x=8+(i%chartCols)*304/chartCols,y=27+Math.floor(i/chartCols)*chartH/chartRows;
  const filter=spec.effects?',effects=[shadow(blur=0.75mm,offset=(0.65mm,0.8mm),opacity=0.35),glow(color="#6d9dd0",blur=0.35mm,opacity=0.16)]':'';
  lines.push(`card=group(${filter.slice(1)})`,
   `plate=card.add(rect(size=(${n(cw)}mm,${n(ch)}mm),border_radius=1mm,fill="#ffffff",border_color="#9badbf",border_width=0.12mm),opacity=0.86)`,
   `card.add(text("Panel ${i+1} / ${i%3===2?'Heat map':'Signal'}",font_family="DejaVu Sans",font_size=${spec.dense?4:5.5}pt,color="#20354b"),offset=(2mm,1mm))`,
   `q=plot(size=(${n(cw-3)}mm,${n(ch-5)}mm),plot_area=box(offset=(7mm,1mm),size=(${n(cw-12)}mm,${n(ch-12)}mm)),frame=box,style=plot_style(font_family="DejaVu Sans",font_size=${spec.dense?3:4}pt,line_width=0.18mm),x=axis(range=(0,10),ticks=[0,5,10]),y=axis(range=(-1.5,1.5),ticks=[-1,0,1]))`);
  if(i%3===2){
   const side=spec.dense?24:16,z=Array.from({length:side},(_,y)=>Array.from({length:side},(_,x)=>n(Math.sin(x*.3+i)*Math.cos(y*.25)+.3*Math.sin(x+y))));
   lines.push(`q.heatmap(z=${JSON.stringify(z)},extent=(0,10,-1.5,1.5),origin=lower,cmap="viridis")`);
  }else{
   const xs=Array.from({length:samples},(_,j)=>j*10/(samples-1));
   for(let k=0;k<3;k++){
    const ys=xs.map(x=>Math.sin(x*(1+k*.31)+i*.28)*Math.exp(-x*.035)+.13*Math.cos(x*7+i+k));
    lines.push(`q.line(x=${numberList(xs)},y=${numberList(ys)},color="${colors[k]}")`);
   }
   const xsSmall=xs.filter((_,j)=>j%16===0),ysSmall=xsSmall.map(x=>Math.sin(x+i*.28)*Math.exp(-x*.035));
   lines.push(`q.scatter(x=${numberList(xsSmall)},y=${numberList(ysSmall)},color="#20354b",marker_size=0.45mm)`);
  }
  lines.push(`card.add(q,offset=(0mm,4mm))`,`chart_${i}=page.add(card,offset=(${n(x)}mm,${n(y)}mm))`);
  targets.push(`chart_${i}`);
 }
 const fcols=spec.dense?8:6,frows=Math.ceil(spec.formulas/fcols),fw=304/fcols-2,fh=frows?52/frows-2:0;
 for(let i=0;i<spec.formulas;i++){
  const x=8+(i%fcols)*304/fcols,y=122+Math.floor(i/fcols)*52/frows;
  const effect=spec.effects?'effects=[shadow(blur=0.5mm,offset=(0.5mm,0.6mm),opacity=0.3)]':'';
  lines.push(`box=group(${effect})`,
   `plate=box.add(rect(size=(${n(fw)}mm,${n(fh)}mm),fill="#ffffff",border_color="#b5c5d3",border_width=0.12mm,border_radius=0.8mm),opacity=0.88)`,
   `box.add(formula(source=r"${equations[i%equations.length]}",font_size=${spec.dense?4.8:7}pt,color="#243b53"),anchor=center,target=plate.center)`,
   `formula_${i}=page.add(box,offset=(${n(x)}mm,${n(y)}mm))`);
  targets.push(`formula_${i}`);
 }
 // Infinite animations have an explicit finite LAY window. The benchmark wraps
 // only that active window and never counts a still hold as animated throughput.
 lines.push(`s=page.step(name="Stress",duration=12s,hold=0s)`,
  `s.animate(title,name="title-motion",duration=3s,iterations="infinite",easing="linear",fill="both")`);
 if(spec.motion!=='title'){
  targets.forEach((name,i)=>lines.push(`s.animate(${name},name="${spec.motion==='3d'?'panel-flip':'panel-motion'}",duration=3s,delay=-${n(.071*i)}s,iterations="infinite",easing="ease-in-out",fill="both")`));
 }
 if(spec.motion==='3d'){
  // Rotate every fourth motif; animate vector strokes independently.
  motifs.forEach((name,i)=>{if(i%4===0)lines.push(`s.animate(${name},name="motif-motion",duration=4s,delay=-${n(.013*i)}s,iterations="infinite",easing="linear",fill="both")`);});
  for(let i=0;i<12;i++)lines.push(`trace=page.add(line(length=20mm,line_color="${colors[i%4]}",line_width=0.22mm),offset=(${8+i*25}mm,176mm))`,
   `s.animate(trace,name="draw",duration=2s,delay=-${n(i*.09)}s,iterations="infinite",direction="alternate",easing="linear",fill="both")`);
 }
 const entry=`stress/${spec.id}.lay`;
 return {entry,files:{[entry]:lines.join('\n')+'\n','stress/motion.lcss':motion},parameters:{...spec,samplesPerLine:samples,seriesPerLineChart:3,heatmapSide:spec.dense?24:16}};
}
