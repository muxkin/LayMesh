// Shared 16px outline icons for preview chrome and presentation controls.
const paths={
 image:'M2 2h12v12H2Zm0 9 4-4 3 3 2-2 3 3M10 5h.01',font:'M3 13 8 3l5 10M5 9h6',folder:'M1.5 4V2h5l2 2h6v10h-13Z',fit:'M2 5V2h3M11 2h3v3M2 11v3h3M11 14h3v-3M5 5h6v6H5Z',add:'M8 3v10M3 8h10',
 previous:'M10 3 5 8l5 5',next:'m6 3 5 5-5 5',down:'m4 6 4 4 4-4',
 play:'m5 2.5 8 5.5-8 5.5Z',pause:'M5 3v10M11 3v10',close:'m3.5 3.5 9 9m0-9-9 9',
 more:'M3 8h.01M8 8h.01M13 8h.01',replay:'M3 6a5.5 5.5 0 1 1-.3 4M3 2v4h4',
 page:'M4 1.5h5l3 3v10H4Zm5 0v3h3M6 8h4M6 11h4',
 black:'M2 3h12v9H2ZM5 14h6M8 12v2',white:'M3 2h10v12H3Z',
 speed:'M2 12a6 6 0 1 1 12 0M8 9l3-4',time:'M8 4v4l3 2M14 8a6 6 0 1 1-12 0 6 6 0 0 1 12 0',
 export:'M8 10V1m-3 3 3-3 3 3M3 7H2v7h12V7h-1',
 expand:'M6 2H2v4M10 2h4v4M2 10v4h4M14 10v4h-4',collapse:'M2 6h4V2M14 6h-4V2M6 14v-4H2M10 14v-4h4',
 'window-play':'M1.5 2.5h13v11h-13ZM6 5l5 3-5 3Z',
 warning:'M8 4v5m0 3h.01M14.5 8a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0',
 check:'m3 8 3.5 3.5L13 4.5',help:'M6 5a2 2 0 1 1 3 1.8C8 7.4 8 8 8 9m0 3h.01M14.5 8a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0'
};
export function uiIcon(name){const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 16 16');svg.setAttribute('width','16');svg.setAttribute('height','16');svg.setAttribute('fill','none');svg.setAttribute('stroke','currentColor');svg.setAttribute('stroke-width','1.5');svg.setAttribute('stroke-linecap','round');svg.setAttribute('stroke-linejoin','round');svg.setAttribute('aria-hidden','true');svg.classList.add('presentation-icon');const path=document.createElementNS(svg.namespaceURI,'path');path.setAttribute('d',paths[name]||paths.more);svg.append(path);return svg;}
export function iconLabel(node,name,label){node.replaceChildren(uiIcon(name));if(label){const span=document.createElement('span');span.textContent=label;node.append(span);}}
