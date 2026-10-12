// Maintain synthetic plot data only. Layout and animation are authored in Lay.
import {readFile, writeFile} from 'node:fs/promises';

const output = new URL('../examples/animation/dashboard-data.json', import.meta.url);
const rounded = value => Number(value.toFixed(5));
const xs = Array.from({length:192}, (_,j) => j * 10 / 191);
const scatterX = xs.filter((_,j) => j % 16 === 0);
const panels = Array.from({length:12}, (_,i) => {
  if (i % 3 === 2) return {z:Array.from({length:16}, (_,y) =>
    Array.from({length:16}, (_,x) => rounded(Math.sin(x*.3+i)*Math.cos(y*.25)+.3*Math.sin(x+y))))};
  return {
    series:Array.from({length:3}, (_,k) => xs.map(x =>
      rounded(Math.sin(x*(1+k*.31)+i*.28)*Math.exp(-x*.035)+.13*Math.cos(x*7+i+k)))),
    scatter_y:scatterX.map(x => rounded(Math.sin(x+i*.28)*Math.exp(-x*.035)))
  };
});
// Keep vectors on individual lines so the data tab has a readable structure.
const lines = ['{', '  "x":'+JSON.stringify(xs.map(rounded))+',',
  '  "scatter_x":'+JSON.stringify(scatterX.map(rounded))+',', '  "panels":['];
panels.forEach((panel,i) => {
  lines.push('    {');
  if (panel.z) lines.push('      "z":[',
    ...panel.z.map((row,j) => '        '+JSON.stringify(row)+(j<15?',':'')), '      ]');
  else lines.push('      "series":[',
    ...panel.series.map((row,j) => '        '+JSON.stringify(row)+(j<2?',':'')),
    '      ],', '      "scatter_y":'+JSON.stringify(panel.scatter_y));
  lines.push('    }'+(i<11?',':''));
});
lines.push('  ]', '}');
const content = lines.join('\n')+'\n';
if (process.argv.includes('--check')) {
  if (await readFile(output,'utf8') !== content) throw Error('Stale dashboard data; run scripts/build-dashboard-data.mjs');
} else await writeFile(output,content);
console.log(`${process.argv.includes('--check')?'Checked':'Wrote'} synthetic data for 8 signal plots and 4 heatmaps`);
