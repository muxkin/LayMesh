import { jsPDF } from "pdf-export";
import { webCrc32 } from "./web-archive.mjs";

const binary = bytes => {
  const chunks=[];
  for(let i=0;i<bytes.length;i+=32768) chunks.push(String.fromCharCode(...bytes.subarray(i,i+32768)));
  return chunks.join("");
};
const equal = (a,b) => a.length===b.length && a.every((v,i)=>v===b[i]);

// One page DOM and at most one worker message are live. PDF resources retain
// only compressed streams; no Image, PNG decoder or RGBA buffer on this thread.
export function createPDFExport({ cancelled, progress }) {
  let pdf, pageCount=0;
  const resources=new Map(), buckets=new Map(), layers=new Map();
  const host=document.createElement("div");
  Object.assign(host.style,{position:"fixed",left:"-100000px",top:"0",opacity:"0",pointerEvents:"none"});
  document.body.append(host);
  const check=()=>{if(cancelled())throw new DOMException("已取消导出","AbortError");};
  const writeStream=(attrs,bytes)=>{
    const id=pdf.internal.newObject();
    pdf.internal.write(`<< ${attrs} /Length ${bytes.length} >>`);
    pdf.internal.write("stream");pdf.internal.write(binary(bytes));pdf.internal.write("endstream");pdf.internal.write("endobj");
    return id;
  };
  const installResources=()=>{
    pdf.__private__.setPdfVersion("1.4");
    pdf.internal.events.subscribe("putResources",()=>{
      for(const r of resources.values()) {
        r.mask=writeStream(`/Type /XObject /Subtype /Image /Width ${r.width} /Height ${r.height} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode`,r.alpha);
        if(!r.solid)r.object=writeStream(`/Type /XObject /Subtype /Image /Width ${r.width} /Height ${r.height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode`,r.data);
      }
      for(const layer of layers.values()) {
        const [x,y,x1,y1]=layer.rect,w=x1-x,h=y1-y;
        let maskContent="0 g 0 0 1 1 re f\n",colorContent="",maskRefs="",colorRefs="";
        for(let i=0;i<layer.tiles.length;i++) {
          const tile=layer.tiles[i],r=resources.get(tile.id),b=tile.rect;
          const matrix=`${(b[2]-b[0])/w} 0 0 ${(b[3]-b[1])/h} ${(b[0]-x)/w} ${(y1-b[3])/h}`;
          maskContent+=`q ${matrix} cm /T${i} Do Q\n`;maskRefs+=`/T${i} ${r.mask} 0 R `;
          if(!r.solid){colorContent+=`q ${matrix} cm /T${i} Do Q\n`;colorRefs+=`/T${i} ${r.object} 0 R `;}
        }
        const mask=writeStream(`/Type /XObject /Subtype /Form /BBox [0 0 1 1] /Group << /S /Transparency /CS /DeviceGray /I true >> /Resources << /XObject << ${maskRefs} >> >>`,new TextEncoder().encode(maskContent));
        if(layer.color)colorContent=`${layer.color.slice(0,3).join(" ")} rg 0 0 1 1 re f`;
        layer.object=writeStream(`/Type /XObject /Subtype /Form /BBox [0 0 1 1] /Group << /S /Transparency /CS /DeviceRGB /I true >> /Resources << /XObject << ${colorRefs} >> /ExtGState << /Alpha << /Type /ExtGState /SMask << /S /Luminosity /G ${mask} 0 R >> >> >> >>`,new TextEncoder().encode(`/Alpha gs\n${colorContent}`));
      }
    });
    pdf.internal.events.subscribe("putXobjectDict",()=>{
      for(const [id,r] of layers)pdf.internal.write(`/${id} ${r.object} 0 R`);
    });
  };
  return {
    resource(r) {
      check();
      const key=`${r.width}:${r.height}:${r.solid}:${webCrc32(r.data)}:${webCrc32(r.alpha)}`;
      for(const id of buckets.get(key)||[]) {
        const old=resources.get(id);
        if(equal(old.data,r.data)&&equal(old.alpha,r.alpha))return id;
      }
      const id=`LMImage${resources.size+1}`;
      resources.set(id,r);buckets.set(key,[...(buckets.get(key)||[]),id]);return id;
    },
    layer(layer) {
      check();
      if(!layer.tiles.length)throw Error("PDF effect layer has no tiles");
      for(const tile of layer.tiles)if(!resources.has(tile.id))throw Error("Missing PDF effect tile");
      const id=`LMLayer${layers.size+1}`;layers.set(id,layer);return id;
    },
    async page(page) {
      check();progress(`正在写入 PDF 第 ${++pageCount} 页…`);
      await new Promise(resolve=>setTimeout(resolve,0));check();
      const width=page.width*72/25.4,height=page.height*72/25.4;
      if(![width,height].every(v=>Number.isFinite(v)&&v>0&&v<=14400))throw Error("放映版 PDF 页面宽高须大于 0 且不超过 5080mm");
      const orientation=width>height?"landscape":"portrait";
      if(!pdf){pdf=new jsPDF({unit:"pt",format:[width,height],orientation,compress:true,putOnlyUsedFonts:true,floatPrecision:8});installResources();}
      else pdf.addPage([width,height],orientation);
      const parsed=new DOMParser().parseFromString(new TextDecoder().decode(page.bytes),"image/svg+xml");
      if(parsed.querySelector("parsererror"))throw Error("无法解析导出的 SVG 页面");
      const svg=document.importNode(parsed.documentElement,true);
      // svg2pdf/jsPDF retains named gradients/patterns across pages. A new
      // standalone SVG starts its identifiers at lm1 again, so scope all IDs
      // and local references to this page before adding it to the document.
      const ids=new Map();
      for(const element of svg.querySelectorAll('[id]')) {
        const old=element.id,next=`lmPage${pageCount}_${old}`;
        ids.set(old,next);element.id=next;
      }
      for(const element of [svg,...svg.querySelectorAll('*')])for(const attribute of [...element.attributes]) {
        let value=attribute.value.replace(/url\(["']?#([^)'"\s]+)["']?\)/g,(all,id)=>ids.has(id)?`url(#${ids.get(id)})`:all);
        if(value.startsWith('#')&&ids.has(value.slice(1)))value='#'+ids.get(value.slice(1));
        if(value!==attribute.value)element.setAttributeNS(attribute.namespaceURI,attribute.name,value);
      }
      for(const element of svg.querySelectorAll("image")) {
        const url=element.getAttribute("href")||element.getAttribute("xlink:href");
        if(url?.startsWith("laymesh-pdf:")) {
          element.setAttribute("data-laymesh-image",url);
          element.removeAttribute("href");element.removeAttribute("xlink:href");
        }
      }
      host.replaceChildren(svg);
      try {
        await pdf.svg(svg,{x:0,y:0,width,height,loadExternalStyleSheets:false,
          laymeshImage(doc,url,element,transform) {
            if(!url?.startsWith("laymesh-pdf:"))return false;
            check();const id=url.slice(12);
            if(!layers.has(id))throw Error(`缺少 PDF 特效资源：${id}`);
            const [x,y,w,h]=["x","y","width","height"].map(n=>Number(element.getAttribute(n)||0));
            if(![x,y,w,h].every(Number.isFinite)||w<=0||h<=0)throw Error("无效 PDF 特效尺寸");
            doc.setCurrentTransformationMatrix(transform);
            doc.internal.write(`q ${w} 0 0 ${-h} ${x} ${y+h} cm /${id} Do Q`);
            return true;
          }
        });
      } finally {host.replaceChildren();}
      check();
    },
    finish(){check();if(!pdf)throw Error("PDF 没有页面");return new Uint8Array(pdf.output("arraybuffer"));},
    dispose(){host.remove();resources.clear();buckets.clear();layers.clear();pdf=null;}
  };
}

export async function exportVectorPDF(pages, options) {
  const writer=createPDFExport(options);
  try{for(const page of pages)await writer.page(page);return writer.finish();}finally{writer.dispose();}
}
