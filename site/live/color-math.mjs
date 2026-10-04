// Frontend picker math. Keep these rules aligned with core/src/color.rs;
// conformance tests compare this module against the actual Rust/WASM exports.
const clamp=v=>Math.max(0,Math.min(1,v));
const hue=v=>((v%360)+360)%360;
const linear=v=>v<=0.04045?v/12.92:((v+0.055)/1.055)**2.4;
const encoded=v=>v<=0.0031308?12.92*v:1.055*v**(1/2.4)-0.055;
function lab([r,g,b]) {
 [r,g,b]=[r,g,b].map(linear);
 const l=Math.cbrt(.4122214708*r+.5363325363*g+.0514459929*b);
 const m=Math.cbrt(.2119034982*r+.6806995451*g+.1073969566*b);
 const s=Math.cbrt(.0883024619*r+.2817188376*g+.6299787005*b);
 return [.2104542553*l+.793617785*m-.0040720468*s,1.9779984951*l-2.428592205*m+.4505937099*s,.0259040371*l+.7827717662*m-.808675766*s];
}
function fromLch([l,c,h],direction) {
 const a=c*(direction?.[0]??Math.cos(h*Math.PI/180)),b=c*(direction?.[1]??Math.sin(h*Math.PI/180));
 const cube=v=>v*v*v;
 const x=cube(l+.3963377774*a+.2158037573*b),y=cube(l-.1055613458*a-.0638541728*b),z=cube(l-.0894841775*a-1.291485548*b);
 return [4.0767416621*x-3.3077115913*y+.2309699292*z,-1.2684380046*x+2.6097574011*y-.3413193965*z,-.0041960863*x-.7034186147*y+1.707614701*z].map(encoded);
}
const byte=v=>Math.round(clamp(v)*255).toString(16).padStart(2,'0');
export function toHex([r,g,b,a]) {return '#'+[r,g,b].map(byte).join('')+(a<1?byte(a):'');}
export function parseHex(value) {
 let text=value.trim();if(!/^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i.test(text))throw Error('Invalid HEX color / 十六进制颜色需要 3、4、6 或 8 位');
 text=text.slice(1);if(text.length<5)text=[...text].map(v=>v+v).join('');
 return {space:'rgb',channels:[0,2,4].map(i=>parseInt(text.slice(i,i+2),16)),alpha:text.length===8?parseInt(text.slice(6),16)/255:1};
}
function resolveColor({space='rgb',channels,alpha=1},direction) {
 if(!Array.isArray(channels)||channels.length!==3||[...channels,alpha].some(v=>typeof v!=='number'||!Number.isFinite(v))||alpha<0||alpha>1)throw Error('Finite channels required; alpha 0–1 / 通道必须为有限数，透明度为 0–1');
 const source=[...channels];let rgb,mapped=false;
 if(space==='rgb') {
  if(source.some(v=>v<0||v>255))throw Error('RGB channels 0–255 / RGB 通道必须在 0–255 之间');
  rgb=source.map(v=>v/255);
 } else if(space==='hsv') {
  if(source.slice(1).some(v=>v<0||v>1))throw Error('HSV S/V 0–1 / HSV 饱和度和明度必须在 0–1 之间');
  source[0]=hue(source[0]);const [h,s,v]=source,c=v*s,x=c*(1-Math.abs((h/60)%2-1)),m=v-c;
  rgb=([[c,x,0],[x,c,0],[0,c,x],[0,x,c],[x,0,c],[c,0,x]][Math.floor(h/60)]).map(n=>n+m);
 } else if(space==='oklch') {
  if(source[0]<0||source[0]>1||source[1]<0)throw Error('OKLCH L 0–1, C ≥ 0 / OKLCH 明度为 0–1，色度非负');
  source[2]=hue(source[2]);direction??=[Math.cos(source[2]*Math.PI/180),Math.sin(source[2]*Math.PI/180)];rgb=fromLch(source,direction);mapped=rgb.some(v=>!Number.isFinite(v)||v< -1e-7||v>1+1e-7);
  if(mapped) {
   let lo=0,hi=Math.min(source[1],1);
   for(let i=0;i<48;i++){const c=(lo+hi)/2,q=fromLch([source[0],c,source[2]],direction);if(q.every(v=>v>=-1e-9&&v<=1+1e-9))lo=c;else hi=c;}
   rgb=fromLch([source[0],lo,source[2]],direction);
  }
 } else throw Error('Supported spaces: RGB, HSV, OKLCH / 支持 RGB、HSV、OKLCH');
 return {rgba:[...rgb.map(clamp),alpha],mapped,source};
}
function hsvChannels(rgb) {
 const [r,g,b]=rgb,max=Math.max(...rgb),min=Math.min(...rgb),d=max-min;
 const h=d<1e-12?0:max===r?60*((g-b)/d%6+6)%360:max===g?60*((b-r)/d+2):60*((r-g)/d+4);
 return [h,max===0?0:d/max,max];
}
function lchChannels(rgb) {
 const [l,a,z]=lab(rgb);return [l,Math.hypot(a,z),hue(Math.atan2(z,a)*180/Math.PI)];
}
// The web picker requests only HSV (its plane) and the visible channel space.
// Getter caches are per state; alpha-only edits retain all chromatic results.
export function pickerColor(input,previous) {
 const space=input.space||'rgb';
 const alpha=input.alpha===undefined?1:input.alpha;
 const same=previous&&previous.input.space===space&&input.channels?.length===3&&input.channels.every((v,i)=>v===previous.input.channels[i]);
 let resolved;
 if(same){if(!Number.isFinite(alpha)||alpha<0||alpha>1)throw Error('Alpha 0–1 / 透明度为 0–1');resolved={rgba:[...previous.rgba.slice(0,3),alpha],mapped:previous.mapped,source:previous.source};}
 else resolved=resolveColor(input);
 const {rgba,mapped,source}=resolved,cache=same?previous.cache:{};
 return {rgba,mapped,source,cache,input:{space,channels:[...input.channels]},get hex(){return toHex(rgba);},
  get rgb(){return cache.rgb??=(space==='rgb'?source:rgba.slice(0,3).map(v=>v*255));},
  get hsv(){return cache.hsv??=(space==='hsv'?source:hsvChannels(rgba.slice(0,3)));},
  get oklch(){return cache.oklch??=(space==='oklch'?source:lchChannels(rgba.slice(0,3)));}};
}
export function convertColor(input) {
 const c=pickerColor(input);return {rgba:c.rgba,mapped:c.mapped,hex:c.hex,rgb:c.rgb,hsv:c.hsv,oklch:c.oklch};
}
// Ramps need sRGB samples only, never reverse conversions or HEX formatting.
// RGB and HSV S/V interpolate exactly in sRGB; hue has six linear pieces.
export function colorRamp(space,channels,index,max) {
 const count=space==='rgb'||space==='hsv'&&index>0?2:space==='hsv'?7:17;
 const direction=space==='oklch'&&index!==2?[Math.cos(hue(channels[2])*Math.PI/180),Math.sin(hue(channels[2])*Math.PI/180)]:undefined;
 return Array.from({length:count},(_,i)=>{const c=[...channels];c[index]=max*i/(count-1);return resolveColor({space,channels:c,alpha:1},direction).rgba;});
}
