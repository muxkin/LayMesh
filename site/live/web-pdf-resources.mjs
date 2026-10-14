import { deflatePDFBytes } from "./web-archive.mjs";

// PDF Flate streams are prepared in the worker, once. A solid shadow needs
// one RGB sample and an 8-bit soft mask, not a full RGB image plus PNG copies.
export function preparePDFTile({ pixels, width, height, alphaOnly=false }, color) {
  const stride=alphaOnly?1:4, channel=alphaOnly?0:3;
  let x0=width, y0=height, x1=0, y1=0;
  for (let y=0, i=channel; y<height; y++) for (let x=0; x<width; x++, i+=stride) if (pixels[i]) {
    x0=Math.min(x0,x); y0=Math.min(y0,y); x1=Math.max(x1,x+1); y1=Math.max(y1,y+1);
  }
  if (x1<=x0) return null;
  const w=x1-x0, h=y1-y0, alpha=new Uint8Array(w*h), rgb=color ? new Uint8Array(color.slice(0,3).map(v=>Math.round(v*255))) : new Uint8Array(w*h*3);
  for (let y=0, a=0, c=0; y<h; y++) for (let x=0; x<w; x++, a++) {
    const i=((y+y0)*width+x+x0)*stride;
    alpha[a]=pixels[i+channel];
    if (!color) { rgb[c++]=pixels[i]; rgb[c++]=pixels[i+1]; rgb[c++]=pixels[i+2]; }
  }
  return { width: w, height: h, solid: Boolean(color), data: deflatePDFBytes(rgb), alpha: deflatePDFBytes(alpha), crop: [x0,y0,x1,y1] };
}
